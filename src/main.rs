#[macro_use]
extern crate rocket;

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use password_hash::{PasswordHash, SaltString};
use rand_core::OsRng;
use rocket::fairing::AdHoc;
use rocket::form::Form;
use rocket::fs::{FileServer, relative};
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket_db_pools::{sqlx, Database};
use rocket_db_pools::sqlx::Row;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::env;

#[derive(Database)]
#[database("kinetic_db")]
struct Db(sqlx::SqlitePool);

#[derive(FromForm)]
struct RegisterForm {
    tenant_slug: String,
    tenant_name: Option<String>,
    email: String,
    password: String,
}

#[derive(FromForm)]
struct LoginForm {
    tenant_slug: String,
    email: String,
    password: String,
}

#[derive(FromForm)]
struct AdminLoginForm {
    email: String,
    password: String,
}

#[derive(FromForm)]
struct WorkspaceForm {
    slug: String,
    name: String,
}

struct User {
    id: i64,
    tenant_id: i64,
    tenant_slug: String,
    email: String,
}

struct AdminUser {
    id: i64,
    email: String,
}

#[derive(Serialize)]
struct WorkspaceView {
    id: i64,
    slug: String,
    name: String,
}

#[derive(Serialize, Clone)]
struct AdminLoginView {
    email: String,
}

#[derive(Serialize, Clone)]
struct WorkspaceFormView {
    slug: String,
    name: String,
}
#[derive(Serialize, Clone)]
struct RegisterView {
    tenant_slug: String,
    tenant_name: String,
    email: String,
}

#[derive(Serialize, Clone)]
struct LoginView {
    tenant_slug: String,
    email: String,
}

#[derive(Serialize)]
struct CurrentUserView {
    tenant_slug: String,
    email: String,
}

fn user_view(user: &User) -> CurrentUserView {
    CurrentUserView {
        tenant_slug: user.tenant_slug.clone(),
        email: user.email.clone(),
    }
}

fn register_view(
    tenant_slug: impl Into<String>,
    tenant_name: impl Into<String>,
    email: impl Into<String>,
) -> RegisterView {
    RegisterView {
        tenant_slug: tenant_slug.into(),
        tenant_name: tenant_name.into(),
        email: email.into(),
    }
}

fn login_view(tenant_slug: impl Into<String>, email: impl Into<String>) -> LoginView {
    LoginView {
        tenant_slug: tenant_slug.into(),
        email: email.into(),
    }
}

fn admin_login_view(email: impl Into<String>) -> AdminLoginView {
    AdminLoginView { email: email.into() }
}

fn workspace_form_view(
    slug: impl Into<String>,
    name: impl Into<String>,
) -> WorkspaceFormView {
    WorkspaceFormView {
        slug: slug.into(),
        name: name.into(),
    }
}

fn normalize_slug(input: &str) -> Option<String> {
    let slug = input.trim().to_lowercase().replace(' ', "-");
    if slug.is_empty() {
        return None;
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return None;
    }
    Some(slug)
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| "Failed to hash password.".to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<(), String> {
    let parsed = PasswordHash::new(hash).map_err(|_| "Invalid password hash.".to_string())?;
    let argon2 = Argon2::default();
    argon2
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| "Invalid credentials.".to_string())
}

async fn load_current_user(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Result<Option<User>, sqlx::Error> {
    let user_id = match cookies.get_private("user_id") {
        Some(cookie) => cookie.value().parse::<i64>().ok(),
        None => None,
    };
    let tenant_id = match cookies.get_private("tenant_id") {
        Some(cookie) => cookie.value().parse::<i64>().ok(),
        None => None,
    };
    let (user_id, tenant_id) = match (user_id, tenant_id) {
        (Some(user_id), Some(tenant_id)) => (user_id, tenant_id),
        _ => return Ok(None),
    };

    let row = sqlx::query(
        r#"
        SELECT users.id, users.email, tenants.id as tenant_id, tenants.slug
        FROM users
        JOIN tenants ON tenants.id = users.tenant_id
        WHERE users.id = ? AND tenants.id = ?
        "#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| User {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        tenant_slug: row.get("slug"),
        email: row.get("email"),
    }))
}

async fn load_admin_user(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Result<Option<AdminUser>, sqlx::Error> {
    let admin_id = match cookies.get_private("admin_id") {
        Some(cookie) => cookie.value().parse::<i64>().ok(),
        None => None,
    };
    let admin_id = match admin_id {
        Some(id) => id,
        None => return Ok(None),
    };

    let row = sqlx::query("SELECT id, email FROM admins WHERE id = ?")
        .bind(admin_id)
        .fetch_optional(&db.0)
        .await?;

    Ok(row.map(|row| AdminUser {
        id: row.get("id"),
        email: row.get("email"),
    }))
}

async fn ensure_schema(pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tenants (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            slug TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            email TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(tenant_id, email),
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS admins (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    seed_admin(pool).await?;

    Ok(())
}

async fn seed_admin(pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    let existing = sqlx::query("SELECT id FROM admins LIMIT 1")
        .fetch_optional(pool)
        .await?;
    if existing.is_some() {
        return Ok(());
    }

    let email = env::var("KINETIC_ADMIN_EMAIL").unwrap_or_else(|_| "admin@kinetic.local".to_string());
    let password = env::var("KINETIC_ADMIN_PASSWORD").unwrap_or_else(|_| "ChangeMe123!".to_string());
    let hash = hash_password(&password).map_err(|_| sqlx::Error::RowNotFound)?;

    sqlx::query("INSERT INTO admins (email, password_hash) VALUES (?, ?)")
        .bind(email.trim().to_lowercase())
        .bind(hash)
        .execute(pool)
        .await?;

    Ok(())
}

#[get("/")]
async fn index(cookies: &CookieJar<'_>, db: &Db) -> Result<Redirect, Template> {
    match load_current_user(cookies, db).await {
        Ok(Some(_)) => Ok(Redirect::to(uri!(dashboard))),
        Ok(None) => Err(Template::render(
            "index",
            context! {
                title: "Kinetic",
                current_user: Option::<CurrentUserView>::None,
            },
        )),
        Err(_) => Err(Template::render(
            "index",
            context! {
                title: "Kinetic",
                current_user: Option::<CurrentUserView>::None,
                error: "Unable to load account data.".to_string(),
            },
        )),
    }
}

#[get("/register")]
async fn register_form(cookies: &CookieJar<'_>, db: &Db) -> Template {
    let current_user = load_current_user(cookies, db)
        .await
        .ok()
        .flatten()
        .map(|user| user_view(&user));
    Template::render(
        "register",
        context! {
            title: "Create your workspace",
            current_user: current_user,
            error: Option::<String>::None,
            form: register_view("", "", ""),
        },
    )
}

#[post("/register", data = "<form>")]
async fn register_submit(
    cookies: &CookieJar<'_>,
    db: &Db,
    form: Form<RegisterForm>,
) -> Result<Redirect, Template> {
    let form = form.into_inner();
    let slug = match normalize_slug(&form.tenant_slug) {
        Some(slug) => slug,
        None => {
            return Err(Template::render(
                "register",
                context! {
                    title: "Create your workspace",
                    current_user: Option::<CurrentUserView>::None,
                    error: "Workspace slug must be lowercase letters, numbers, or dashes.".to_string(),
                    form: register_view(
                        form.tenant_slug,
                        form.tenant_name.unwrap_or_default(),
                        form.email,
                    ),
                },
            ))
        }
    };

    if form.password.trim().len() < 8 {
        return Err(Template::render(
            "register",
            context! {
                title: "Create your workspace",
                current_user: Option::<CurrentUserView>::None,
                error: "Password must be at least 8 characters.".to_string(),
                form: register_view(slug, form.tenant_name.unwrap_or_default(), form.email),
            },
        ));
    }

    let tenant_name = form
        .tenant_name
        .clone()
        .unwrap_or_else(|| slug.replace('-', " "));

    let tenant_id: i64 = match sqlx::query("SELECT id FROM tenants WHERE slug = ?")
        .bind(&slug)
        .fetch_optional(&db.0)
        .await
    {
        Ok(Some(row)) => row.get("id"),
        Ok(None) => {
            if let Err(err) = sqlx::query("INSERT INTO tenants (slug, name) VALUES (?, ?)")
                .bind(&slug)
                .bind(&tenant_name)
                .execute(&db.0)
                .await
            {
                return Err(Template::render(
                    "register",
                    context! {
                        title: "Create your workspace",
                        current_user: Option::<CurrentUserView>::None,
                        error: format!("Unable to create workspace: {err}"),
                        form: register_view(slug, tenant_name, form.email),
                    },
                ));
            }

            let row = match sqlx::query("SELECT id FROM tenants WHERE slug = ?")
                .bind(&slug)
                .fetch_one(&db.0)
                .await
            {
                Ok(row) => row,
                Err(_) => {
                    return Err(Template::render(
                        "register",
                        context! {
                            title: "Create your workspace",
                            current_user: Option::<CurrentUserView>::None,
                            error: "Workspace created, but could not load it.".to_string(),
                            form: register_view(slug.clone(), tenant_name.clone(), form.email),
                        },
                    ))
                }
            };
            row.get("id")
        }
        Err(_) => {
            return Err(Template::render(
                "register",
                context! {
                    title: "Create your workspace",
                    current_user: Option::<CurrentUserView>::None,
                    error: "Unable to check workspace.".to_string(),
                    form: register_view(slug, tenant_name, form.email),
                },
            ))
        }
    };

    let password_hash = match hash_password(&form.password) {
        Ok(hash) => hash,
        Err(message) => {
            return Err(Template::render(
                "register",
                context! {
                    title: "Create your workspace",
                    current_user: Option::<CurrentUserView>::None,
                    error: message,
                    form: register_view(slug, tenant_name, form.email),
                },
            ))
        }
    };

    if let Err(err) = sqlx::query(
        "INSERT INTO users (tenant_id, email, password_hash) VALUES (?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(form.email.trim().to_lowercase())
    .bind(password_hash)
    .execute(&db.0)
    .await
    {
        return Err(Template::render(
            "register",
            context! {
                title: "Create your workspace",
                current_user: Option::<CurrentUserView>::None,
                error: format!("Unable to create user: {err}"),
                form: register_view(slug, tenant_name, form.email),
            },
        ));
    }

    let user_row = match sqlx::query(
        r#"
        SELECT users.id, users.email, tenants.id as tenant_id, tenants.slug
        FROM users
        JOIN tenants ON tenants.id = users.tenant_id
        WHERE users.email = ? AND tenants.id = ?
        "#,
    )
    .bind(form.email.trim().to_lowercase())
    .bind(tenant_id)
    .fetch_one(&db.0)
    .await
    {
        Ok(row) => row,
        Err(_) => {
            return Err(Template::render(
                "register",
                context! {
                    title: "Create your workspace",
                    current_user: Option::<CurrentUserView>::None,
                    error: "User created, but could not load profile.".to_string(),
                    form: register_view(slug.clone(), tenant_name.clone(), form.email),
                },
            ))
        }
    };

    let user_id: i64 = user_row.get("id");
    cookies.add_private(
        Cookie::build(("user_id", user_id.to_string()))
            .path("/")
            .build(),
    );
    cookies.add_private(
        Cookie::build(("tenant_id", tenant_id.to_string()))
            .path("/")
            .build(),
    );

    Ok(Redirect::to(uri!(dashboard)))
}

#[get("/login")]
async fn login_form(cookies: &CookieJar<'_>, db: &Db) -> Template {
    let current_user = load_current_user(cookies, db)
        .await
        .ok()
        .flatten()
        .map(|user| user_view(&user));
    Template::render(
        "login",
        context! {
            title: "Welcome back",
            current_user: current_user,
            error: Option::<String>::None,
            form: login_view("", ""),
        },
    )
}

#[post("/login", data = "<form>")]
async fn login_submit(
    cookies: &CookieJar<'_>,
    db: &Db,
    form: Form<LoginForm>,
) -> Result<Redirect, Template> {
    let form = form.into_inner();
    let slug = match normalize_slug(&form.tenant_slug) {
        Some(slug) => slug,
        None => {
            return Err(Template::render(
                "login",
                context! {
                    title: "Welcome back",
                    current_user: Option::<CurrentUserView>::None,
                    error: "Workspace slug must be lowercase letters, numbers, or dashes.".to_string(),
                    form: login_view(form.tenant_slug, form.email),
                },
            ))
        }
    };

    let row = match sqlx::query(
        r#"
        SELECT users.id, users.email, users.password_hash, tenants.id as tenant_id, tenants.slug
        FROM users
        JOIN tenants ON tenants.id = users.tenant_id
        WHERE users.email = ? AND tenants.slug = ?
        "#,
    )
    .bind(form.email.trim().to_lowercase())
    .bind(&slug)
    .fetch_optional(&db.0)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            return Err(Template::render(
                "login",
                context! {
                    title: "Welcome back",
                    current_user: Option::<CurrentUserView>::None,
                    error: "Invalid credentials.".to_string(),
                    form: login_view(slug, form.email),
                },
            ))
        }
        Err(_) => {
            return Err(Template::render(
                "login",
                context! {
                    title: "Welcome back",
                    current_user: Option::<CurrentUserView>::None,
                    error: "Unable to verify credentials.".to_string(),
                    form: login_view(slug, form.email),
                },
            ))
        }
    };

    let password_hash: String = row.get("password_hash");
    if let Err(message) = verify_password(&form.password, &password_hash) {
        return Err(Template::render(
            "login",
            context! {
                title: "Welcome back",
                current_user: Option::<CurrentUserView>::None,
                error: message,
                form: login_view(slug, form.email),
            },
        ));
    }

    let user_id: i64 = row.get("id");
    let tenant_id: i64 = row.get("tenant_id");
    cookies.add_private(
        Cookie::build(("user_id", user_id.to_string()))
            .path("/")
            .build(),
    );
    cookies.add_private(
        Cookie::build(("tenant_id", tenant_id.to_string()))
            .path("/")
            .build(),
    );

    Ok(Redirect::to(uri!(dashboard)))
}

#[get("/logout")]
fn logout(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove_private(Cookie::build("user_id").build());
    cookies.remove_private(Cookie::build("tenant_id").build());
    Redirect::to(uri!(index))
}

#[get("/admin/login")]
fn admin_login_form() -> Template {
    Template::render(
        "admin/login",
        context! {
            title: "Admin access",
            error: Option::<String>::None,
            form: admin_login_view(""),
        },
    )
}

#[post("/admin/login", data = "<form>")]
async fn admin_login_submit(
    cookies: &CookieJar<'_>,
    db: &Db,
    form: Form<AdminLoginForm>,
) -> Result<Redirect, Template> {
    let form = form.into_inner();
    let row = match sqlx::query("SELECT id, email, password_hash FROM admins WHERE email = ?")
        .bind(form.email.trim().to_lowercase())
        .fetch_optional(&db.0)
        .await
    {
        Ok(Some(row)) => row,
        _ => {
            return Err(Template::render(
                "admin/login",
                context! {
                    title: "Admin access",
                    error: "Invalid credentials.".to_string(),
                    form: admin_login_view(form.email),
                },
            ))
        }
    };

    let password_hash: String = row.get("password_hash");
    if let Err(message) = verify_password(&form.password, &password_hash) {
        return Err(Template::render(
            "admin/login",
            context! {
                title: "Admin access",
                error: message,
                form: admin_login_view(form.email),
            },
        ));
    }

    let admin_id: i64 = row.get("id");
    cookies.add_private(
        Cookie::build(("admin_id", admin_id.to_string()))
            .path("/")
            .build(),
    );

    Ok(Redirect::to(uri!(admin_workspaces)))
}

#[get("/admin/logout")]
fn admin_logout(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove_private(Cookie::build("admin_id").build());
    Redirect::to(uri!(admin_login_form))
}

#[get("/admin/workspaces")]
async fn admin_workspaces(cookies: &CookieJar<'_>, db: &Db) -> Result<Template, Redirect> {
    let admin = match load_admin_user(cookies, db).await {
        Ok(Some(admin)) => admin,
        _ => return Err(Redirect::to(uri!(admin_login_form))),
    };

    let rows = sqlx::query("SELECT id, slug, name FROM tenants ORDER BY id DESC")
        .fetch_all(&db.0)
        .await
        .unwrap_or_default();

    let workspaces: Vec<WorkspaceView> = rows
        .into_iter()
        .map(|row| {
            WorkspaceView {
                id: row.get::<i64, _>("id"),
                slug: row.get::<String, _>("slug"),
                name: row.get::<String, _>("name"),
            }
        })
        .collect();

    Ok(Template::render(
        "admin/workspaces",
        context! {
            title: "Workspaces",
            admin_email: admin.email,
            workspaces: workspaces,
        },
    ))
}

#[get("/admin/workspaces/new")]
async fn admin_workspace_new_form(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Result<Template, Redirect> {
    let admin = match load_admin_user(cookies, db).await {
        Ok(Some(admin)) => admin,
        _ => return Err(Redirect::to(uri!(admin_login_form))),
    };

    Ok(Template::render(
        "admin/workspace_new",
        context! {
            title: "New workspace",
            admin_email: admin.email,
            error: Option::<String>::None,
            form: workspace_form_view("", ""),
        },
    ))
}

#[post("/admin/workspaces", data = "<form>")]
async fn admin_workspace_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    form: Form<WorkspaceForm>,
) -> Result<Redirect, Template> {
    let admin = match load_admin_user(cookies, db).await {
        Ok(Some(admin)) => admin,
        _ => return Ok(Redirect::to(uri!(admin_login_form))),
    };
    let form = form.into_inner();
    let slug = match normalize_slug(&form.slug) {
        Some(slug) => slug,
        None => {
            return Err(Template::render(
                "admin/workspace_new",
                context! {
                    title: "New workspace",
                    admin_email: admin.email,
                    error: "Slug must be lowercase letters, numbers, or dashes.".to_string(),
                    form: workspace_form_view(form.slug, form.name),
                },
            ))
        }
    };

    if form.name.trim().is_empty() {
        return Err(Template::render(
            "admin/workspace_new",
            context! {
                title: "New workspace",
                admin_email: admin.email,
                error: "Workspace name is required.".to_string(),
                form: workspace_form_view(slug, form.name),
            },
        ));
    }

    if let Err(err) = sqlx::query("INSERT INTO tenants (slug, name) VALUES (?, ?)")
        .bind(&slug)
        .bind(form.name.trim())
        .execute(&db.0)
        .await
    {
        return Err(Template::render(
            "admin/workspace_new",
            context! {
                title: "New workspace",
                admin_email: admin.email,
                error: format!("Unable to create workspace: {err}"),
                form: workspace_form_view(slug, form.name),
            },
        ));
    }

    Ok(Redirect::to(uri!(admin_workspaces)))
}

#[get("/admin/workspaces/<id>/edit")]
async fn admin_workspace_edit_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
) -> Result<Template, Redirect> {
    let admin = match load_admin_user(cookies, db).await {
        Ok(Some(admin)) => admin,
        _ => return Err(Redirect::to(uri!(admin_login_form))),
    };

    let row = match sqlx::query("SELECT id, slug, name FROM tenants WHERE id = ?")
        .bind(id)
        .fetch_optional(&db.0)
        .await
    {
        Ok(Some(row)) => row,
        _ => return Ok(Template::render(
            "admin/workspaces",
            context! {
                title: "Workspaces",
                admin_email: admin.email,
                workspaces: Vec::<WorkspaceView>::new(),
                error: "Workspace not found.".to_string(),
            },
        )),
    };

    Ok(Template::render(
        "admin/workspace_edit",
        context! {
            title: "Edit workspace",
            admin_email: admin.email,
            error: Option::<String>::None,
            workspace_id: row.get::<i64, _>("id"),
            form: workspace_form_view(
                row.get::<String, _>("slug"),
                row.get::<String, _>("name"),
            ),
        },
    ))
}

#[post("/admin/workspaces/<id>", data = "<form>")]
async fn admin_workspace_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
    form: Form<WorkspaceForm>,
) -> Result<Redirect, Template> {
    let admin = match load_admin_user(cookies, db).await {
        Ok(Some(admin)) => admin,
        _ => return Ok(Redirect::to(uri!(admin_login_form))),
    };
    let form = form.into_inner();
    let slug = match normalize_slug(&form.slug) {
        Some(slug) => slug,
        None => {
            return Err(Template::render(
                "admin/workspace_edit",
                context! {
                    title: "Edit workspace",
                    admin_email: admin.email,
                    error: "Slug must be lowercase letters, numbers, or dashes.".to_string(),
                    workspace_id: id,
                    form: workspace_form_view(form.slug, form.name),
                },
            ))
        }
    };

    if form.name.trim().is_empty() {
        return Err(Template::render(
            "admin/workspace_edit",
            context! {
                title: "Edit workspace",
                admin_email: admin.email,
                error: "Workspace name is required.".to_string(),
                workspace_id: id,
                form: workspace_form_view(slug, form.name),
            },
        ));
    }

    if let Err(err) = sqlx::query("UPDATE tenants SET slug = ?, name = ? WHERE id = ?")
        .bind(&slug)
        .bind(form.name.trim())
        .bind(id)
        .execute(&db.0)
        .await
    {
        return Err(Template::render(
            "admin/workspace_edit",
            context! {
                title: "Edit workspace",
                admin_email: admin.email,
                error: format!("Unable to update workspace: {err}"),
                workspace_id: id,
                form: workspace_form_view(slug, form.name),
            },
        ));
    }

    Ok(Redirect::to(uri!(admin_workspaces)))
}

#[post("/admin/workspaces/<id>/delete")]
async fn admin_workspace_delete(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
) -> Result<Redirect, Template> {
    let admin = match load_admin_user(cookies, db).await {
        Ok(Some(admin)) => admin,
        _ => return Ok(Redirect::to(uri!(admin_login_form))),
    };

    if let Err(err) = sqlx::query("DELETE FROM users WHERE tenant_id = ?")
        .bind(id)
        .execute(&db.0)
        .await
    {
        return Err(Template::render(
            "admin/workspaces",
            context! {
                title: "Workspaces",
                admin_email: admin.email,
                workspaces: Vec::<WorkspaceView>::new(),
                error: format!("Unable to delete workspace users: {err}"),
            },
        ));
    }

    if let Err(err) = sqlx::query("DELETE FROM tenants WHERE id = ?")
        .bind(id)
        .execute(&db.0)
        .await
    {
        return Err(Template::render(
            "admin/workspaces",
            context! {
                title: "Workspaces",
                admin_email: admin.email,
                workspaces: Vec::<WorkspaceView>::new(),
                error: format!("Unable to delete workspace: {err}"),
            },
        ));
    }

    Ok(Redirect::to(uri!(admin_workspaces)))
}

#[get("/dashboard")]
async fn dashboard(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Result<Template, Redirect> {
    let user = match load_current_user(cookies, db).await {
        Ok(Some(user)) => user,
        _ => return Err(Redirect::to(uri!(login_form))),
    };

    Ok(Template::render(
        "dashboard",
        context! {
            title: "Dashboard",
            current_user: Some(user_view(&user)),
            tenant_slug: user.tenant_slug,
            email: user.email,
        },
    ))
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Db::init())
        .attach(Template::fairing())
        .attach(AdHoc::try_on_ignite("Init DB", |rocket| async {
            let db = Db::fetch(&rocket).expect("database pool");
            if let Err(err) = ensure_schema(&db.0).await {
                eprintln!("Failed to init schema: {err}");
                return Err(rocket);
            }
            Ok(rocket)
        }))
        .mount(
            "/",
            routes![
                index,
                register_form,
                register_submit,
                login_form,
                login_submit,
                logout,
                admin_login_form,
                admin_login_submit,
                admin_logout,
                admin_workspaces,
                admin_workspace_new_form,
                admin_workspace_create,
                admin_workspace_edit_form,
                admin_workspace_update,
                admin_workspace_delete,
                dashboard
            ],
        )
        .mount("/static", FileServer::from(relative!("static")))
}

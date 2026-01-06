use rocket::form::Form;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket_dyn_templates::{context, Template};

use crate::models::{
    AdminUserForm,
    AdminUserFormView,
    PaginationView,
    WorkspaceForm,
    WorkspaceFormView,
    User,
};
use crate::repositories::{user_permission_repo, user_repo};
use crate::services::access_service;
use crate::services::{auth_service, workspace_service};
use crate::Db;
use crate::services::utils::hash_password;

const PER_PAGE: usize = 10;

fn normalize_page(page: Option<usize>) -> usize {
    page.unwrap_or(1).max(1)
}

fn pagination_view(
    page: usize,
    total_count: i64,
    build_url: impl Fn(usize) -> String,
) -> PaginationView {
    let per_page = PER_PAGE as i64;
    let total_pages = ((total_count + per_page - 1) / per_page).max(1) as usize;
    let page = page.min(total_pages).max(1);
    let has_prev = page > 1;
    let has_next = page < total_pages;
    let prev_url = if has_prev {
        build_url(page - 1)
    } else {
        build_url(1)
    };
    let next_url = if has_next {
        build_url(page + 1)
    } else {
        build_url(total_pages)
    };

    PaginationView {
        page,
        total_pages,
        has_prev,
        has_next,
        prev_url,
        next_url,
    }
}

async fn admin_from_cookies(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Option<User> {
    let user_id = cookies
        .get_private("user_id")
        .and_then(|c| c.value().parse::<i64>().ok());
    let tenant_id = cookies
        .get_private("tenant_id")
        .and_then(|c| c.value().parse::<i64>().ok());
    match (user_id, tenant_id) {
        (Some(user_id), Some(tenant_id)) => auth_service::get_user_by_ids(db, user_id, tenant_id)
            .await
            .ok()
            .flatten()
            .filter(|user| user.is_super_admin),
        _ => None,
    }
}

#[get("/admin/login")]
pub fn admin_login_form() -> Redirect {
    Redirect::to("/login")
}

#[post("/admin/login")]
pub fn admin_login_submit() -> Redirect {
    Redirect::to("/login")
}

#[get("/admin/logout")]
pub fn admin_logout(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove_private(Cookie::build("user_id").build());
    cookies.remove_private(Cookie::build("tenant_id").build());
    Redirect::to("/login")
}

#[get("/admin/workspaces?<page>")]
pub async fn admin_workspaces(
    cookies: &CookieJar<'_>,
    db: &Db,
    page: Option<usize>,
) -> Result<Template, Redirect> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Err(Redirect::to(uri!(admin_login_form))),
    };

    let page = normalize_page(page);
    let offset = ((page - 1) * PER_PAGE) as i64;
    let workspaces = workspace_service::list_workspaces_paged(db, PER_PAGE as i64, offset)
        .await
        .unwrap_or_default();
    let total_workspaces = workspace_service::count_workspaces(db).await.unwrap_or(0);
    let pagination = pagination_view(page, total_workspaces, |target_page| {
        format!("/admin/workspaces?page={}", target_page)
    });

    Ok(Template::render(
        "admin/workspaces",
        context! {
            title: "Workspaces",
            admin_email: admin.email,
            workspaces: workspaces,
            pagination: pagination,
        },
    ))
}

#[get("/admin/users")]
pub async fn admin_users(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Result<Template, Redirect> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Err(Redirect::to(uri!(admin_login_form))),
    };

    let users = user_repo::list_users_all(db).await.unwrap_or_default();
    Ok(Template::render(
        "admin/users",
        context! {
            title: "Users",
            admin_email: admin.email,
            users: users,
            error: Option::<String>::None,
        },
    ))
}

#[get("/admin/users/new")]
pub async fn admin_user_new_form(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Result<Template, Redirect> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Err(Redirect::to(uri!(admin_login_form))),
    };
    let workspaces = workspace_service::list_workspaces(db)
        .await
        .unwrap_or_default();
    let default_tenant_id = workspaces.first().map(|workspace| workspace.id).unwrap_or(0);

    Ok(Template::render(
        "admin/user_new",
        context! {
            title: "New user",
            admin_email: admin.email,
            error: Option::<String>::None,
            form: AdminUserFormView::new(default_tenant_id, "", access_service::role_options()[0], ""),
            workspaces: workspaces,
            role_options: access_service::role_options(),
        },
    ))
}

#[post("/admin/users", data = "<form>")]
pub async fn admin_user_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    form: Form<AdminUserForm>,
) -> Result<Redirect, Template> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Ok(Redirect::to(uri!(admin_login_form))),
    };
    let form = form.into_inner();
    let workspaces = workspace_service::list_workspaces(db)
        .await
        .unwrap_or_default();
    let role = form.role.trim().to_string();
    let email = form.email.trim().to_lowercase();
    let password = form.password.unwrap_or_default();

    if form.tenant_id <= 0
        || workspace_service::find_workspace_by_id(db, form.tenant_id)
            .await
            .ok()
            .flatten()
            .is_none()
    {
        return Err(Template::render(
            "admin/user_new",
            context! {
                title: "New user",
                admin_email: admin.email,
                error: "Workspace is required.".to_string(),
                form: AdminUserFormView::new(form.tenant_id, email, role, ""),
                workspaces: workspaces,
                role_options: access_service::role_options(),
            },
        ));
    }

    if email.is_empty() {
        return Err(Template::render(
            "admin/user_new",
            context! {
                title: "New user",
                admin_email: admin.email,
                error: "Email is required.".to_string(),
                form: AdminUserFormView::new(form.tenant_id, email, role, ""),
                workspaces: workspaces,
                role_options: access_service::role_options(),
            },
        ));
    }

    let role = match access_service::role_options()
        .iter()
        .find(|option| option.eq_ignore_ascii_case(&role))
    {
        Some(value) => value.to_string(),
        None => {
            return Err(Template::render(
                "admin/user_new",
                context! {
                    title: "New user",
                    admin_email: admin.email,
                    error: "Role is invalid.".to_string(),
                    form: AdminUserFormView::new(form.tenant_id, email, role, ""),
                    workspaces: workspaces,
                    role_options: access_service::role_options(),
                },
            ))
        }
    };

    if password.trim().len() < 8 {
        return Err(Template::render(
            "admin/user_new",
            context! {
                title: "New user",
                admin_email: admin.email,
                error: "Password must be at least 8 characters.".to_string(),
                form: AdminUserFormView::new(form.tenant_id, email, role, ""),
                workspaces: workspaces,
                role_options: access_service::role_options(),
            },
        ));
    }

    let hash = match hash_password(password.trim()) {
        Ok(hash) => hash,
        Err(message) => {
            return Err(Template::render(
                "admin/user_new",
                context! {
                    title: "New user",
                    admin_email: admin.email,
                    error: message,
                    form: AdminUserFormView::new(form.tenant_id, email, role, ""),
                    workspaces: workspaces,
                    role_options: access_service::role_options(),
                },
            ))
        }
    };

    if let Err(err) = user_repo::create_user(db, form.tenant_id, &email, &hash, &role).await {
        return Err(Template::render(
            "admin/user_new",
            context! {
                title: "New user",
                admin_email: admin.email,
                error: format!("Unable to create user: {err}"),
                form: AdminUserFormView::new(form.tenant_id, email, role, ""),
                workspaces: workspaces,
                role_options: access_service::role_options(),
            },
        ));
    }

    Ok(Redirect::to(uri!(admin_users)))
}

#[get("/admin/users/<id>/edit")]
pub async fn admin_user_edit_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
) -> Result<Template, Redirect> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Err(Redirect::to(uri!(admin_login_form))),
    };

    let user = match user_repo::find_user_by_id_any(db, id).await {
        Ok(Some(user)) => user,
        _ => {
            let users = user_repo::list_users_all(db).await.unwrap_or_default();
            return Ok(Template::render(
                "admin/users",
                context! {
                    title: "Users",
                    admin_email: admin.email,
                    users: users,
                    error: "User not found.".to_string(),
                },
            ));
        }
    };
    let workspaces = workspace_service::list_workspaces(db)
        .await
        .unwrap_or_default();

    Ok(Template::render(
        "admin/user_edit",
        context! {
            title: "Edit user",
            admin_email: admin.email,
            error: Option::<String>::None,
            user_id: user.id,
            form: AdminUserFormView::new(user.tenant_id, user.email, user.role, ""),
            workspaces: workspaces,
            role_options: access_service::role_options(),
            is_super_admin: user.is_super_admin,
        },
    ))
}

#[post("/admin/users/<id>", data = "<form>")]
pub async fn admin_user_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
    form: Form<AdminUserForm>,
) -> Result<Redirect, Template> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Ok(Redirect::to(uri!(admin_login_form))),
    };
    let form = form.into_inner();
    let mut user = match user_repo::find_user_by_id_any(db, id).await {
        Ok(Some(user)) => user,
        _ => {
            let users = user_repo::list_users_all(db).await.unwrap_or_default();
            return Err(Template::render(
                "admin/users",
                context! {
                    title: "Users",
                    admin_email: admin.email,
                    users: users,
                    error: "User not found.".to_string(),
                },
            ));
        }
    };
    let workspaces = workspace_service::list_workspaces(db)
        .await
        .unwrap_or_default();
    let role_input = form.role.trim().to_string();
    let email = form.email.trim().to_lowercase();
    let password = form.password.unwrap_or_default();

    if email.is_empty() {
        return Err(Template::render(
            "admin/user_edit",
            context! {
                title: "Edit user",
                admin_email: admin.email,
                error: "Email is required.".to_string(),
                user_id: id,
                form: AdminUserFormView::new(form.tenant_id, email, role_input, ""),
                workspaces: workspaces,
                role_options: access_service::role_options(),
                is_super_admin: user.is_super_admin,
            },
        ));
    }

    let role = match access_service::role_options()
        .iter()
        .find(|option| option.eq_ignore_ascii_case(&role_input))
    {
        Some(value) => value.to_string(),
        None => {
            return Err(Template::render(
                "admin/user_edit",
                context! {
                    title: "Edit user",
                    admin_email: admin.email,
                    error: "Role is invalid.".to_string(),
                    user_id: id,
                    form: AdminUserFormView::new(form.tenant_id, email, role_input, ""),
                    workspaces: workspaces,
                    role_options: access_service::role_options(),
                    is_super_admin: user.is_super_admin,
                },
            ))
        }
    };

    let target_tenant_id = if user.is_super_admin {
        user.tenant_id
    } else {
        form.tenant_id
    };

    if target_tenant_id <= 0
        || workspace_service::find_workspace_by_id(db, target_tenant_id)
            .await
            .ok()
            .flatten()
            .is_none()
    {
        return Err(Template::render(
            "admin/user_edit",
            context! {
                title: "Edit user",
                admin_email: admin.email,
                error: "Workspace is required.".to_string(),
                user_id: id,
                form: AdminUserFormView::new(target_tenant_id, email, role, ""),
                workspaces: workspaces,
                role_options: access_service::role_options(),
                is_super_admin: user.is_super_admin,
            },
        ));
    }

    if !password.trim().is_empty() && password.trim().len() < 8 {
        return Err(Template::render(
            "admin/user_edit",
            context! {
                title: "Edit user",
                admin_email: admin.email,
                error: "Password must be at least 8 characters.".to_string(),
                user_id: id,
                form: AdminUserFormView::new(target_tenant_id, email, role, ""),
                workspaces: workspaces,
                role_options: access_service::role_options(),
                is_super_admin: user.is_super_admin,
            },
        ));
    }

    if target_tenant_id != user.tenant_id {
        let _ = user_permission_repo::replace_permissions_for_user(
            db,
            user.tenant_id,
            user.id,
            &Vec::<crate::models::UserPermission>::new(),
        )
        .await;
    }

    if let Err(err) = user_repo::update_user_admin(db, id, target_tenant_id, &email, &role).await {
        return Err(Template::render(
            "admin/user_edit",
            context! {
                title: "Edit user",
                admin_email: admin.email,
                error: format!("Unable to update user: {err}"),
                user_id: id,
                form: AdminUserFormView::new(target_tenant_id, email, role, ""),
                workspaces: workspaces,
                role_options: access_service::role_options(),
                is_super_admin: user.is_super_admin,
            },
        ));
    }

    if !password.trim().is_empty() {
        let hash = match hash_password(password.trim()) {
            Ok(hash) => hash,
            Err(message) => {
                return Err(Template::render(
                    "admin/user_edit",
                    context! {
                        title: "Edit user",
                        admin_email: admin.email,
                        error: message,
                        user_id: id,
                        form: AdminUserFormView::new(target_tenant_id, email, role, ""),
                        workspaces: workspaces,
                        role_options: access_service::role_options(),
                        is_super_admin: user.is_super_admin,
                    },
                ))
            }
        };
        if let Err(err) = user_repo::update_user_password(db, id, &hash).await {
            return Err(Template::render(
                "admin/user_edit",
                context! {
                    title: "Edit user",
                    admin_email: admin.email,
                    error: format!("Unable to update password: {err}"),
                    user_id: id,
                    form: AdminUserFormView::new(target_tenant_id, email, role, ""),
                    workspaces: workspaces,
                    role_options: access_service::role_options(),
                    is_super_admin: user.is_super_admin,
                },
            ));
        }
    }

    Ok(Redirect::to(uri!(admin_users)))
}

#[post("/admin/users/<id>/delete")]
pub async fn admin_user_delete(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
) -> Result<Redirect, Template> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Ok(Redirect::to(uri!(admin_login_form))),
    };
    let user = user_repo::find_user_by_id_any(db, id).await.ok().flatten();
    if let Some(user) = user {
        if user.is_super_admin {
            let users = user_repo::list_users_all(db).await.unwrap_or_default();
            return Err(Template::render(
                "admin/users",
                context! {
                    title: "Users",
                    admin_email: admin.email,
                    users: users,
                    error: "Super admin user cannot be deleted.".to_string(),
                },
            ));
        }
    }

    if let Err(err) = user_repo::delete_user_by_id(db, id).await {
        let users = user_repo::list_users_all(db).await.unwrap_or_default();
        return Err(Template::render(
            "admin/users",
            context! {
                title: "Users",
                admin_email: admin.email,
                users: users,
                error: format!("Unable to delete user: {err}"),
            },
        ));
    }

    Ok(Redirect::to(uri!(admin_users)))
}

#[get("/admin/workspaces/new")]
pub async fn admin_workspace_new_form(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Result<Template, Redirect> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Err(Redirect::to(uri!(admin_login_form))),
    };

    Ok(Template::render(
        "admin/workspace_new",
        context! {
            title: "New workspace",
            admin_email: admin.email,
            error: Option::<String>::None,
            form: WorkspaceFormView::new("", "", "free"),
            plan_options: workspace_service::plan_definitions(),
        },
    ))
}

#[post("/admin/workspaces", data = "<form>")]
pub async fn admin_workspace_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    form: Form<WorkspaceForm>,
) -> Result<Redirect, Template> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Ok(Redirect::to(uri!(admin_login_form))),
    };
    let form = form.into_inner();
    match workspace_service::create_workspace(db, form.slug, form.name, form.plan_key).await {
        Ok(_) => Ok(Redirect::to(uri!(admin_workspaces(
            page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
            "admin/workspace_new",
            context! {
                title: "New workspace",
                admin_email: admin.email,
                error: err.message,
                form: err.form,
                plan_options: workspace_service::plan_definitions(),
            },
        )),
    }
}

#[get("/admin/workspaces/<id>/edit")]
pub async fn admin_workspace_edit_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
) -> Result<Template, Redirect> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Err(Redirect::to(uri!(admin_login_form))),
    };

    let workspace = match workspace_service::find_workspace_by_id(db, id).await {
        Ok(Some(workspace)) => workspace,
        _ => {
            return Ok(Template::render(
                "admin/workspaces",
                context! {
                    title: "Workspaces",
                    admin_email: admin.email,
                    workspaces: Vec::<crate::models::Workspace>::new(),
                    pagination: pagination_view(1, 0, |target_page| {
                        format!("/admin/workspaces?page={}", target_page)
                    }),
                    error: "Workspace not found.".to_string(),
                },
            ))
        }
    };

    Ok(Template::render(
        "admin/workspace_edit",
        context! {
            title: "Edit workspace",
            admin_email: admin.email,
            error: Option::<String>::None,
            workspace_id: workspace.id,
            form: WorkspaceFormView::new(workspace.slug, workspace.name, workspace.plan_key),
            plan_options: workspace_service::plan_definitions(),
        },
    ))
}

#[post("/admin/workspaces/<id>", data = "<form>")]
pub async fn admin_workspace_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
    form: Form<WorkspaceForm>,
) -> Result<Redirect, Template> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Ok(Redirect::to(uri!(admin_login_form))),
    };
    let form = form.into_inner();
    match workspace_service::update_workspace(db, id, form.slug, form.name, form.plan_key).await {
        Ok(_) => Ok(Redirect::to(uri!(admin_workspaces(
            page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
            "admin/workspace_edit",
            context! {
                title: "Edit workspace",
                admin_email: admin.email,
                error: err.message,
                workspace_id: id,
                form: err.form,
                plan_options: workspace_service::plan_definitions(),
            },
        )),
    }
}

#[post("/admin/workspaces/<id>/delete")]
pub async fn admin_workspace_delete(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
) -> Result<Redirect, Template> {
    let admin = match admin_from_cookies(cookies, db).await {
        Some(admin) => admin,
        None => return Ok(Redirect::to(uri!(admin_login_form))),
    };

    if let Err(message) = workspace_service::delete_workspace(db, id).await {
        return Err(Template::render(
            "admin/workspaces",
            context! {
                title: "Workspaces",
                admin_email: admin.email,
                workspaces: Vec::<crate::models::Workspace>::new(),
                pagination: pagination_view(1, 0, |target_page| {
                    format!("/admin/workspaces?page={}", target_page)
                }),
                error: message,
            },
        ));
    }

    Ok(Redirect::to(uri!(admin_workspaces(
        page = Option::<usize>::None
    ))))
}

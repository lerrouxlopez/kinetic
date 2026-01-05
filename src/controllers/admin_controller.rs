use rocket::form::Form;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket_dyn_templates::{context, Template};

use crate::models::{AdminLoginForm, AdminLoginView, PaginationView, WorkspaceForm, WorkspaceFormView};
use crate::services::{admin_service, workspace_service};
use crate::Db;

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
) -> Option<crate::models::AdminUser> {
    let admin_id = cookies
        .get_private("admin_id")
        .and_then(|c| c.value().parse::<i64>().ok());
    match admin_id {
        Some(admin_id) => admin_service::get_admin_by_id(db, admin_id)
            .await
            .ok()
            .flatten(),
        None => None,
    }
}

#[get("/admin/login")]
pub fn admin_login_form() -> Template {
    Template::render(
        "admin/login",
        context! {
            title: "Admin access",
            error: Option::<String>::None,
            form: AdminLoginView::new(""),
        },
    )
}

#[post("/admin/login", data = "<form>")]
pub async fn admin_login_submit(
    cookies: &CookieJar<'_>,
    db: &Db,
    form: Form<AdminLoginForm>,
) -> Result<Redirect, Template> {
    let form = form.into_inner();
    match admin_service::authenticate_admin(db, form.email, form.password).await {
        Ok(admin) => {
            cookies.add_private(
                Cookie::build(("admin_id", admin.id.to_string()))
                    .path("/")
                    .build(),
            );
            Ok(Redirect::to(uri!(admin_workspaces(
                page = Option::<usize>::None
            ))))
        }
        Err(err) => Err(Template::render(
            "admin/login",
            context! {
                title: "Admin access",
                error: err.message,
                form: err.form,
            },
        )),
    }
}

#[get("/admin/logout")]
pub fn admin_logout(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove_private(Cookie::build("admin_id").build());
    Redirect::to(uri!(admin_login_form))
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
            form: WorkspaceFormView::new("", ""),
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
    match workspace_service::create_workspace(db, form.slug, form.name).await {
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
            form: WorkspaceFormView::new(workspace.slug, workspace.name),
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
    match workspace_service::update_workspace(db, id, form.slug, form.name).await {
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

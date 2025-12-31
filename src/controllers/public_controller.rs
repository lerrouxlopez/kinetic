use rocket::form::Form;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket_dyn_templates::{context, Template};

use crate::models::{CurrentUserView, LoginForm, LoginView, RegisterForm, RegisterView};
use crate::services::auth_service;
use crate::Db;

async fn current_user_from_cookies(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Option<CurrentUserView> {
    let user_id = cookies.get_private("user_id").and_then(|c| c.value().parse().ok());
    let tenant_id = cookies.get_private("tenant_id").and_then(|c| c.value().parse().ok());
    match (user_id, tenant_id) {
        (Some(user_id), Some(tenant_id)) => auth_service::get_user_by_ids(db, user_id, tenant_id)
            .await
            .ok()
            .flatten()
            .map(|user| CurrentUserView::from(&user)),
        _ => None,
    }
}

#[get("/")]
pub async fn index(cookies: &CookieJar<'_>, db: &Db) -> Result<Redirect, Template> {
    match current_user_from_cookies(cookies, db).await {
        Some(_) => Ok(Redirect::to(uri!(dashboard))),
        None => Err(Template::render(
            "index",
            context! {
                title: "Kinetic",
                current_user: Option::<CurrentUserView>::None,
            },
        )),
    }
}

#[get("/register")]
pub async fn register_form(cookies: &CookieJar<'_>, db: &Db) -> Template {
    let current_user = current_user_from_cookies(cookies, db).await;
    Template::render(
        "register",
        context! {
            title: "Create your workspace",
            current_user: current_user,
            error: Option::<String>::None,
            form: RegisterView::new("", "", ""),
        },
    )
}

#[post("/register", data = "<form>")]
pub async fn register_submit(
    cookies: &CookieJar<'_>,
    db: &Db,
    form: Form<RegisterForm>,
) -> Result<Redirect, Template> {
    let form = form.into_inner();
    match auth_service::register(db, form).await {
        Ok((user, tenant_id)) => {
            cookies.add_private(
                Cookie::build(("user_id", user.id.to_string()))
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
        Err(err) => Err(Template::render(
            "register",
            context! {
                title: "Create your workspace",
                current_user: Option::<CurrentUserView>::None,
                error: err.message,
                form: err.form,
            },
        )),
    }
}

#[get("/login")]
pub async fn login_form(cookies: &CookieJar<'_>, db: &Db) -> Template {
    let current_user = current_user_from_cookies(cookies, db).await;
    Template::render(
        "login",
        context! {
            title: "Welcome back",
            current_user: current_user,
            error: Option::<String>::None,
            form: LoginView::new("", ""),
        },
    )
}

#[post("/login", data = "<form>")]
pub async fn login_submit(
    cookies: &CookieJar<'_>,
    db: &Db,
    form: Form<LoginForm>,
) -> Result<Redirect, Template> {
    let form = form.into_inner();
    match auth_service::login(db, form).await {
        Ok(user) => {
            cookies.add_private(
                Cookie::build(("user_id", user.id.to_string()))
                    .path("/")
                    .build(),
            );
            cookies.add_private(
                Cookie::build(("tenant_id", user.tenant_id.to_string()))
                    .path("/")
                    .build(),
            );
            Ok(Redirect::to(uri!(dashboard)))
        }
        Err(err) => Err(Template::render(
            "login",
            context! {
                title: "Welcome back",
                current_user: Option::<CurrentUserView>::None,
                error: err.message,
                form: err.form,
            },
        )),
    }
}

#[get("/logout")]
pub fn logout(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove_private(Cookie::build("user_id").build());
    cookies.remove_private(Cookie::build("tenant_id").build());
    Redirect::to(uri!(index))
}

#[get("/dashboard")]
pub async fn dashboard(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Result<Template, Redirect> {
    let user_id = cookies.get_private("user_id").and_then(|c| c.value().parse().ok());
    let tenant_id = cookies.get_private("tenant_id").and_then(|c| c.value().parse().ok());
    let user = match (user_id, tenant_id) {
        (Some(user_id), Some(tenant_id)) => auth_service::get_user_by_ids(db, user_id, tenant_id)
            .await
            .ok()
            .flatten(),
        _ => None,
    };
    match user {
        Some(user) => Ok(Template::render(
            "dashboard",
            context! {
                title: "Dashboard",
                current_user: Some(CurrentUserView::from(&user)),
                tenant_slug: user.tenant_slug,
                email: user.email,
            },
        )),
        None => Err(Redirect::to(uri!(login_form))),
    }
}

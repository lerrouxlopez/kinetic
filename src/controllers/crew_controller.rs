use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket_dyn_templates::{context, Template};

use crate::models::{CrewForm, CrewFormView, CurrentUserView};
use crate::services::{auth_service, crew_service};
use crate::Db;

async fn tenant_from_cookies(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Option<(i64, CurrentUserView)> {
    let user_id = cookies.get_private("user_id").and_then(|c| c.value().parse().ok());
    let tenant_id = cookies.get_private("tenant_id").and_then(|c| c.value().parse().ok());
    match (user_id, tenant_id) {
        (Some(user_id), Some(tenant_id)) => auth_service::get_user_by_ids(db, user_id, tenant_id)
            .await
            .ok()
            .flatten()
            .map(|user| (tenant_id, CurrentUserView::from(&user))),
        _ => None,
    }
}

#[get("/crew")]
pub async fn crew_index(cookies: &CookieJar<'_>, db: &Db) -> Result<Template, Redirect> {
    let (tenant_id, current_user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };

    let crews = crew_service::list_crews(db, tenant_id)
        .await
        .unwrap_or_default();
    let stats = crew_service::stats_from_crews(&crews);

    Ok(Template::render(
        "crew/index",
        context! {
            title: "Crew roster",
            current_user: Some(current_user),
            crews: crews,
            stats: stats,
        },
    ))
}

#[get("/crew/new")]
pub async fn crew_new_form(cookies: &CookieJar<'_>, db: &Db) -> Result<Template, Redirect> {
    let (_, current_user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };

    Ok(Template::render(
        "crew/new",
        context! {
            title: "New crew",
            current_user: Some(current_user),
            error: Option::<String>::None,
            form: CrewFormView::new("", 0, "Active"),
            status_options: crew_service::status_options(),
        },
    ))
}

#[post("/crew", data = "<form>")]
pub async fn crew_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    form: Form<CrewForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, current_user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let form = form.into_inner();
    match crew_service::create_crew(db, tenant_id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(crew_index))),
        Err(err) => Err(Template::render(
            "crew/new",
            context! {
                title: "New crew",
                current_user: Some(current_user),
                error: err.message,
                form: err.form,
                status_options: crew_service::status_options(),
            },
        )),
    }
}

#[get("/crew/<id>/edit")]
pub async fn crew_edit_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
) -> Result<Template, Redirect> {
    let (tenant_id, current_user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };

    let crew = match crew_service::find_crew_by_id(db, tenant_id, id).await {
        Ok(Some(crew)) => crew,
        _ => {
            return Ok(Template::render(
                "crew/index",
                context! {
                    title: "Crew roster",
                    current_user: Some(current_user),
                    crews: Vec::<crate::models::Crew>::new(),
                    stats: crew_service::stats_from_crews(&[]),
                    error: "Crew not found.".to_string(),
                },
            ))
        }
    };

    Ok(Template::render(
        "crew/edit",
        context! {
            title: "Edit crew",
            current_user: Some(current_user),
            error: Option::<String>::None,
            crew_id: crew.id,
            form: CrewFormView::new(crew.name, crew.members_count, crew.status),
            status_options: crew_service::status_options(),
        },
    ))
}

#[post("/crew/<id>", data = "<form>")]
pub async fn crew_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
    form: Form<CrewForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, current_user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let form = form.into_inner();
    match crew_service::update_crew(db, tenant_id, id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(crew_index))),
        Err(err) => Err(Template::render(
            "crew/edit",
            context! {
                title: "Edit crew",
                current_user: Some(current_user),
                error: err.message,
                crew_id: id,
                form: err.form,
                status_options: crew_service::status_options(),
            },
        )),
    }
}

#[post("/crew/<id>/delete")]
pub async fn crew_delete(
    cookies: &CookieJar<'_>,
    db: &Db,
    id: i64,
) -> Result<Redirect, Template> {
    let (tenant_id, current_user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };

    if let Err(message) = crew_service::delete_crew(db, tenant_id, id).await {
        return Err(Template::render(
            "crew/index",
            context! {
                title: "Crew roster",
                current_user: Some(current_user),
                crews: Vec::<crate::models::Crew>::new(),
                stats: crew_service::stats_from_crews(&[]),
                error: message,
            },
        ));
    }

    Ok(Redirect::to(uri!(crew_index)))
}

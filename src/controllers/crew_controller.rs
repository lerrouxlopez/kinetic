use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket_dyn_templates::{context, Template};

use crate::models::{
    CrewForm,
    CrewFormView,
    CrewMemberForm,
    CrewMemberFormView,
    CurrentUserView,
    PaginationView,
};
use crate::services::{access_service, auth_service, crew_service, workspace_service};
use crate::repositories::user_repo;
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

fn empty_members_pagination(slug: &str, crew_id: i64) -> PaginationView {
    pagination_view(1, 0, |target_page| {
        format!("/{}/crew/{}/profile?members_page={}", slug, crew_id, target_page)
    })
}

async fn tenant_from_cookies(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Option<(i64, crate::models::User)> {
    let user_id = cookies.get_private("user_id").and_then(|c| c.value().parse().ok());
    let tenant_id = cookies.get_private("tenant_id").and_then(|c| c.value().parse().ok());
    match (user_id, tenant_id) {
        (Some(user_id), Some(tenant_id)) => auth_service::get_user_by_ids(db, user_id, tenant_id)
            .await
            .ok()
            .flatten()
            .map(|user| (tenant_id, user)),
        _ => None,
    }
}

async fn workspace_brand(db: &Db, tenant_id: i64) -> crate::models::WorkspaceBrandView {
    workspace_service::find_workspace_by_id(db, tenant_id)
        .await
        .ok()
        .flatten()
        .map(|workspace| workspace_service::workspace_brand_view(&workspace))
        .unwrap_or_else(workspace_service::default_workspace_brand_view)
}

#[get("/<slug>/crew?<page>")]
pub async fn crew_index(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    page: Option<usize>,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(crew_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }
    if !access_service::can_view(db, &user, "crew").await {
        return Err(Redirect::to(uri!(crate::controllers::public_controller::dashboard(
            slug = current_user.tenant_slug
        ))));
    }

    let all_crews = crew_service::list_crews(db, tenant_id)
        .await
        .unwrap_or_default();
    let stats = crew_service::stats_from_crews(&all_crews);
    let is_free_plan = user.plan_key.eq_ignore_ascii_case("free");
    let free_limits = workspace_service::free_plan_limits(db).await;
    let crew_limit = free_limits.crews.unwrap_or(2) as usize;
    let crew_limit_reached = is_free_plan && stats.total_crews >= crew_limit;
    let page = normalize_page(page);
    let offset = ((page - 1) * PER_PAGE) as i64;
    let crews = crew_service::list_crews_paged(db, tenant_id, PER_PAGE as i64, offset)
        .await
        .unwrap_or_default();
    let total_crews = crew_service::count_crews(db, tenant_id).await.unwrap_or(0);
    let pagination = pagination_view(page, total_crews, |target_page| {
        format!("/{}/crew?page={}", current_user.tenant_slug, target_page)
    });

    Ok(Template::render(
        "crew/index",
        context! {
            title: "Crew roster",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            crews: crews,
            stats: stats,
            crew_limit_reached: crew_limit_reached,
            crew_limit: crew_limit,
            pagination: pagination,
        },
    ))
}

#[get("/<slug>/crew/<id>/profile?<members_page>")]
pub async fn crew_show(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    members_page: Option<usize>,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_view(db, &user, "crew").await {
        return Err(Redirect::to(uri!(crate::controllers::public_controller::dashboard(
            slug = current_user.tenant_slug
        ))));
    }
    let tenant_slug = current_user.tenant_slug.clone();

    let crew = match crew_service::find_crew_by_id(db, tenant_id, id).await {
        Ok(Some(crew)) => crew,
        _ => {
            return Ok(Template::render(
                "crew/index",
                context! {
                    title: "Crew roster",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    crews: Vec::<crate::models::Crew>::new(),
                    stats: crew_service::stats_from_crews(&[]),
                    error: "Crew not found.".to_string(),
                    crew_limit_reached: false,
                },
            ))
        }
    };

    let members_page = normalize_page(members_page);
    let offset = ((members_page - 1) * PER_PAGE) as i64;
    let members = crew_service::list_members_paged(db, tenant_id, id, PER_PAGE as i64, offset)
        .await
        .unwrap_or_default();
    let total_members = crew_service::count_members(db, tenant_id, id).await.unwrap_or(0);
    let members_pagination = pagination_view(members_page, total_members, |target_page| {
        format!(
            "/{}/crew/{}/profile?members_page={}",
            tenant_slug, id, target_page
        )
    });
    let is_free_plan = user.plan_key.eq_ignore_ascii_case("free");
    let free_limits = workspace_service::free_plan_limits(db).await;
    let member_limit = free_limits.members_per_crew.unwrap_or(5);
    let member_limit_reached = is_free_plan && total_members >= member_limit;

    Ok(Template::render(
        "crew/show",
        context! {
            title: "Crew details",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            crew: crew,
            members: members,
            members_count: total_members as usize,
            member_limit_reached: member_limit_reached,
            member_limit: member_limit,
            members_pagination: members_pagination,
        },
    ))
}

#[get("/<slug>/crew/new")]
pub async fn crew_new_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
) -> Result<Template, Redirect> {
    let (_, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(crew_new_form(slug = current_user.tenant_slug))));
    }
    if !access_service::can_edit(db, &user, "crew").await {
        return Err(Redirect::to(uri!(crew_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }

    Ok(Template::render(
        "crew/new",
        context! {
            title: "New crew",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            error: Option::<String>::None,
            form: CrewFormView::new("", "Active"),
            status_options: crew_service::status_options(),
        },
    ))
}

#[post("/<slug>/crew", data = "<form>")]
pub async fn crew_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    form: Form<CrewForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(crew_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "crew").await {
        return Ok(Redirect::to(uri!(crew_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    match crew_service::create_crew(db, tenant_id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(crew_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
            "crew/new",
            context! {
                title: "New crew",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: err.message,
                form: err.form,
                status_options: crew_service::status_options(),
            },
        )),
    }
}

#[get("/<slug>/crew/<id>/edit")]
pub async fn crew_edit_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(crew_edit_form(slug = current_user.tenant_slug, id = id))));
    }
    if !access_service::can_edit(db, &user, "crew").await {
        return Err(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        ))));
    }

    let crew = match crew_service::find_crew_by_id(db, tenant_id, id).await {
        Ok(Some(crew)) => crew,
        _ => {
            return Ok(Template::render(
                "crew/index",
                context! {
                    title: "Crew roster",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    crews: Vec::<crate::models::Crew>::new(),
                    stats: crew_service::stats_from_crews(&[]),
                    error: "Crew not found.".to_string(),
                    crew_limit_reached: false,
                },
            ))
        }
    };

    Ok(Template::render(
        "crew/edit",
        context! {
            title: "Edit crew",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            error: Option::<String>::None,
            crew_id: crew.id,
            form: CrewFormView::new(crew.name, crew.status),
            status_options: crew_service::status_options(),
        },
    ))
}

#[get("/<slug>/crew/<id>/members/new")]
pub async fn crew_member_new_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(crew_member_new_form(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }
    if !access_service::can_edit(db, &user, "crew").await {
        return Err(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        ))));
    }

    let crew = match crew_service::find_crew_by_id(db, tenant_id, id).await {
        Ok(Some(crew)) => crew,
        _ => {
            return Ok(Template::render(
                "crew/index",
                context! {
                    title: "Crew roster",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    crews: Vec::<crate::models::Crew>::new(),
                    stats: crew_service::stats_from_crews(&[]),
                    error: "Crew not found.".to_string(),
                    crew_limit_reached: false,
                },
            ))
        }
    };
    let users = user_repo::list_users_by_tenant(db, tenant_id)
        .await
        .unwrap_or_default();

    Ok(Template::render(
        "crew/member_new",
        context! {
            title: "New member",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            error: Option::<String>::None,
            crew: crew,
            form: CrewMemberFormView::new(0, "", "", ""),
            users: users,
        },
    ))
}

#[post("/<slug>/crew/<id>/members", data = "<form>")]
pub async fn crew_member_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    form: Form<CrewMemberForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "crew").await {
        return Ok(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    let crew = match crew_service::find_crew_by_id(db, tenant_id, id).await {
        Ok(Some(crew)) => crew,
        _ => {
            return Err(Template::render(
                "crew/index",
                context! {
                    title: "Crew roster",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    crews: Vec::<crate::models::Crew>::new(),
                    stats: crew_service::stats_from_crews(&[]),
                    error: "Crew not found.".to_string(),
                    crew_limit_reached: false,
                },
            ))
        }
    };
    let users = user_repo::list_users_by_tenant(db, tenant_id)
        .await
        .unwrap_or_default();

    match crew_service::create_member(db, tenant_id, id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
            "crew/member_new",
            context! {
                title: "New member",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: err.message,
                crew: crew,
                form: err.form,
                users: users,
            },
        )),
    }
}

#[get("/<slug>/crew/<id>/members/<member_id>/edit")]
pub async fn crew_member_edit_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    member_id: i64,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(crew_member_edit_form(
            slug = current_user.tenant_slug,
            id = id,
            member_id = member_id
        ))));
    }
    if !access_service::can_edit(db, &user, "crew").await {
        return Err(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        ))));
    }

    let crew = match crew_service::find_crew_by_id(db, tenant_id, id).await {
        Ok(Some(crew)) => crew,
        _ => {
            return Ok(Template::render(
                "crew/index",
                context! {
                    title: "Crew roster",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    crews: Vec::<crate::models::Crew>::new(),
                    stats: crew_service::stats_from_crews(&[]),
                    error: "Crew not found.".to_string(),
                    crew_limit_reached: false,
                },
            ))
        }
    };

    let member = match crew_service::find_member_by_id(db, tenant_id, id, member_id).await {
        Ok(Some(member)) => member,
        _ => {
            let tenant_slug = current_user.tenant_slug.clone();
            let members_pagination = empty_members_pagination(&tenant_slug, id);
            return Ok(Template::render(
                "crew/show",
                context! {
                    title: "Crew details",
                    current_user: Some(current_user),
                    workspace_brand: workspace_brand(db, user.tenant_id).await,
                    crew: crew,
                    members: Vec::<crate::models::CrewMember>::new(),
                    members_count: 0,
                    members_pagination: members_pagination,
                    error: "Member not found.".to_string(),
                    member_limit_reached: false,
                },
            ))
        }
    };
    let users = user_repo::list_users_by_tenant(db, tenant_id)
        .await
        .unwrap_or_default();
    let selected_user_id = if let Some(user_id) = member.user_id {
        user_id
    } else {
        user_repo::find_user_by_email_and_tenant(db, &member.email, tenant_id)
            .await
            .ok()
            .flatten()
            .map(|user| user.id)
            .unwrap_or(0)
    };

    Ok(Template::render(
        "crew/member_edit",
        context! {
            title: "Edit member",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            crew: crew,
            error: Option::<String>::None,
            member_id: member.id,
            form: CrewMemberFormView::new(
                selected_user_id,
                member.name,
                member.phone,
                member.position,
            ),
            users: users,
        },
    ))
}

#[post("/<slug>/crew/<id>/members/<member_id>", data = "<form>")]
pub async fn crew_member_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    member_id: i64,
    form: Form<CrewMemberForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "crew").await {
        return Ok(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    let crew = match crew_service::find_crew_by_id(db, tenant_id, id).await {
        Ok(Some(crew)) => crew,
        _ => {
            return Err(Template::render(
                "crew/index",
                context! {
                    title: "Crew roster",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    crews: Vec::<crate::models::Crew>::new(),
                    stats: crew_service::stats_from_crews(&[]),
                    error: "Crew not found.".to_string(),
                    crew_limit_reached: false,
                },
            ))
        }
    };
    let users = user_repo::list_users_by_tenant(db, tenant_id)
        .await
        .unwrap_or_default();

    match crew_service::update_member(db, tenant_id, id, member_id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
            "crew/member_edit",
            context! {
                title: "Edit member",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                crew: crew,
                error: err.message,
                member_id: member_id,
                form: err.form,
                users: users,
            },
        )),
    }
}

#[post("/<slug>/crew/<id>/members/<member_id>/delete")]
pub async fn crew_member_delete(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    member_id: i64,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_delete(db, &user, "crew").await {
        return Ok(Redirect::to(uri!(crew_show(
            slug = current_user.tenant_slug,
            id = id,
            members_page = Option::<usize>::None
        ))));
    }

    let crew = match crew_service::find_crew_by_id(db, tenant_id, id).await {
        Ok(Some(crew)) => crew,
        _ => {
            return Err(Template::render(
                "crew/index",
                context! {
                    title: "Crew roster",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    crews: Vec::<crate::models::Crew>::new(),
                    stats: crew_service::stats_from_crews(&[]),
                    error: "Crew not found.".to_string(),
                    crew_limit_reached: false,
                },
            ))
        }
    };

    if let Err(message) = crew_service::delete_member(db, tenant_id, id, member_id).await {
        let tenant_slug = current_user.tenant_slug.clone();
        let members_pagination = empty_members_pagination(&tenant_slug, id);
        return Err(Template::render(
            "crew/show",
            context! {
                title: "Crew details",
                current_user: Some(current_user),
                workspace_brand: workspace_brand(db, user.tenant_id).await,
                crew: crew,
                members: Vec::<crate::models::CrewMember>::new(),
                members_count: 0,
                members_pagination: members_pagination,
                error: message,
                member_limit_reached: false,
                crew_limit_reached: false,
            },
        ));
    }

    Ok(Redirect::to(uri!(crew_show(
        slug = current_user.tenant_slug,
        id = id,
        members_page = Option::<usize>::None
    ))))
}

#[post("/<slug>/crew/<id>", data = "<form>")]
pub async fn crew_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    form: Form<CrewForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(crew_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "crew").await {
        return Ok(Redirect::to(uri!(crew_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    match crew_service::update_crew(db, tenant_id, id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(crew_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
            "crew/edit",
            context! {
                title: "Edit crew",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: err.message,
                crew_id: id,
                form: err.form,
                status_options: crew_service::status_options(),
            },
        )),
    }
}

#[post("/<slug>/crew/<id>/delete")]
pub async fn crew_delete(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(crew_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }
    if !access_service::can_delete(db, &user, "crew").await {
        return Ok(Redirect::to(uri!(crew_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }

    if let Err(message) = crew_service::delete_crew(db, tenant_id, id).await {
        return Err(Template::render(
            "crew/index",
            context! {
                title: "Crew roster",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                crews: Vec::<crate::models::Crew>::new(),
                stats: crew_service::stats_from_crews(&[]),
                error: message,
                crew_limit_reached: false,
            },
        ));
    }

    Ok(Redirect::to(uri!(crew_index(
        slug = current_user.tenant_slug,
        page = Option::<usize>::None
    ))))
}





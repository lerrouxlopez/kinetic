use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket_dyn_templates::{context, Template};

use crate::models::{
    AppointmentForm,
    AppointmentFormView,
    ClientContactForm,
    ClientContactFormView,
    ClientForm,
    ClientFormView,
    CurrentUserView,
    EmailForm,
    EmailFormView,
    PaginationView,
};
use crate::services::{access_service, appointment_service, auth_service, client_service, email_service, workspace_service};
use crate::Db;

const PER_PAGE: usize = 10;

fn split_scheduled_for(value: &str) -> (String, String) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return ("".to_string(), "".to_string());
    }
    let separator = if trimmed.contains('T') { 'T' } else { ' ' };
    let mut parts = trimmed.splitn(2, separator);
    let date_part = parts.next().unwrap_or("").trim().to_string();
    let time_part = parts
        .next()
        .unwrap_or("")
        .trim()
        .trim_end_matches('Z')
        .to_string();
    let time_part = if time_part.len() > 5 {
        time_part[..5].to_string()
    } else {
        time_part
    };
    (date_part, time_part)
}

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

fn empty_clients_pagination(slug: &str) -> PaginationView {
    pagination_view(1, 0, |target_page| format!("/{}/clients?page={}", slug, target_page))
}

fn empty_client_show_pagination(slug: &str, id: i64) -> (PaginationView, PaginationView) {
    let contacts_pagination = pagination_view(1, 0, |target_page| {
        format!(
            "/{}/clients/{}/profile?contacts_page={}&appointments_page=1",
            slug, id, target_page
        )
    });
    let appointments_pagination = pagination_view(1, 0, |target_page| {
        format!(
            "/{}/clients/{}/profile?contacts_page=1&appointments_page={}",
            slug, id, target_page
        )
    });
    (contacts_pagination, appointments_pagination)
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

#[get("/<slug>/clients?<page>")]
pub async fn clients_index(
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
        return Err(Redirect::to(uri!(clients_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }
    if !access_service::can_view(db, &user, "clients").await {
        return Err(Redirect::to(uri!(crate::controllers::public_controller::dashboard(
            slug = current_user.tenant_slug
        ))));
    }
    let tenant_slug = current_user.tenant_slug.clone();

    let page = normalize_page(page);
    let offset = ((page - 1) * PER_PAGE) as i64;
    let clients = client_service::list_clients_paged(db, tenant_id, PER_PAGE as i64, offset)
        .await
        .unwrap_or_default();
    let total_clients = client_service::count_clients(db, tenant_id).await.unwrap_or(0);
    let pagination = pagination_view(page, total_clients, |target_page| {
        format!("/{}/clients?page={}", tenant_slug, target_page)
    });
    let (_plan_key, limits) = workspace_service::plan_limits_for_tenant(db, tenant_id).await;
    let client_limit = limits.clients.unwrap_or(0);
    let client_limit_reached = limits
        .clients
        .map(|limit| total_clients >= limit)
        .unwrap_or(false);

    Ok(Template::render(
        "clients/index",
        context! {
            title: "Clients",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            clients: clients,
            pagination: pagination,
            client_limit: client_limit,
            client_limit_reached: client_limit_reached,
        },
    ))
}

#[get("/<slug>/clients/new")]
pub async fn client_new_form(
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
        return Err(Redirect::to(uri!(client_new_form(slug = current_user.tenant_slug))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Err(Redirect::to(uri!(clients_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }

    Ok(Template::render(
        "clients/new",
        context! {
            title: "New client",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            error: Option::<String>::None,
            form: ClientFormView::new("", "", "", "", "", "", "Proposal", "USD"),
            stage_options: client_service::client_stage_options(),
            currency_options: client_service::currency_options(),
        },
    ))
}

#[post("/<slug>/clients", data = "<form>")]
pub async fn client_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    form: Form<ClientForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(clients_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Ok(Redirect::to(uri!(clients_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    match client_service::create_client(db, tenant_id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(clients_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
            "clients/new",
            context! {
                title: "New client",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: err.message,
                form: err.form,
                stage_options: client_service::client_stage_options(),
                currency_options: client_service::currency_options(),
            },
        )),
    }
}

#[get("/<slug>/clients/<id>/profile?<contacts_page>&<appointments_page>")]
pub async fn client_show(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    contacts_page: Option<usize>,
    appointments_page: Option<usize>,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_view(db, &user, "clients").await {
        return Err(Redirect::to(uri!(crate::controllers::public_controller::dashboard(
            slug = current_user.tenant_slug
        ))));
    }
    let tenant_slug = current_user.tenant_slug.clone();

    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Ok(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };

    let contacts_page = normalize_page(contacts_page);
    let appointments_page = normalize_page(appointments_page);
    let contacts_offset = ((contacts_page - 1) * PER_PAGE) as i64;
    let appointments_offset = ((appointments_page - 1) * PER_PAGE) as i64;

    let contacts = client_service::list_contacts_paged(
        db,
        tenant_id,
        id,
        PER_PAGE as i64,
        contacts_offset,
    )
    .await
    .unwrap_or_default();
    let total_contacts = client_service::count_contacts(db, tenant_id, id).await.unwrap_or(0);
    let contacts_count = total_contacts as usize;
    let contacts_pagination = pagination_view(contacts_page, total_contacts, |target_page| {
        format!(
            "/{}/clients/{}/profile?contacts_page={}&appointments_page={}",
            tenant_slug, id, target_page, appointments_page
        )
    });

    let appointments = appointment_service::list_appointments_paged(
        db,
        tenant_id,
        id,
        PER_PAGE as i64,
        appointments_offset,
    )
    .await
    .unwrap_or_default();
    let total_appointments = appointment_service::count_appointments(db, tenant_id, id)
        .await
        .unwrap_or(0);
    let appointments_pagination =
        pagination_view(appointments_page, total_appointments, |target_page| {
            format!(
                "/{}/clients/{}/profile?contacts_page={}&appointments_page={}",
                tenant_slug, id, contacts_page, target_page
            )
        });
    let appointments_count = total_appointments as usize;
    let deployments_count = 0;
    let (_plan_key, limits) = workspace_service::plan_limits_for_tenant(db, tenant_id).await;
    let contacts_limit = limits.contacts_per_client.unwrap_or(0);
    let contacts_limit_reached = limits
        .contacts_per_client
        .map(|limit| total_contacts >= limit)
        .unwrap_or(false);
    let appointments_limit = limits.appointments_per_client.unwrap_or(0);
    let appointments_limit_reached = limits
        .appointments_per_client
        .map(|limit| total_appointments >= limit)
        .unwrap_or(false);

    Ok(Template::render(
        "clients/show",
        context! {
            title: "Client details",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            client: client,
            contacts: contacts,
            contacts_count: contacts_count,
            appointments: appointments,
            appointments_count: appointments_count,
            deployments_count: deployments_count,
            contacts_pagination: contacts_pagination,
            appointments_pagination: appointments_pagination,
            contacts_limit: contacts_limit,
            contacts_limit_reached: contacts_limit_reached,
            appointments_limit: appointments_limit,
            appointments_limit_reached: appointments_limit_reached,
        },
    ))
}

#[get("/<slug>/clients/<id>/edit")]
pub async fn client_edit_form(
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
        return Err(Redirect::to(uri!(client_edit_form(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Err(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }

    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Ok(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };

    Ok(Template::render(
        "clients/edit",
        context! {
            title: "Edit client",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            error: Option::<String>::None,
            client_id: client.id,
            form: ClientFormView::new(
                client.company_name,
                client.address,
                client.phone,
                client.email,
                client.latitude,
                client.longitude,
                client.stage,
                client.currency,
            ),
            stage_options: client_service::client_stage_options(),
            currency_options: client_service::currency_options(),
        },
    ))
}

#[post("/<slug>/clients/<id>", data = "<form>")]
pub async fn client_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    form: Form<ClientForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(clients_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    match client_service::update_client(db, tenant_id, id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(clients_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
            "clients/edit",
            context! {
                title: "Edit client",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: err.message,
                client_id: id,
                form: err.form,
                stage_options: client_service::client_stage_options(),
                currency_options: client_service::currency_options(),
            },
        )),
    }
}

#[post("/<slug>/clients/<id>/delete")]
pub async fn client_delete(
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
        return Ok(Redirect::to(uri!(clients_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }
    if !access_service::can_delete(db, &user, "clients").await {
        return Ok(Redirect::to(uri!(clients_index(
            slug = current_user.tenant_slug,
            page = Option::<usize>::None
        ))));
    }

    if let Err(message) = client_service::delete_client(db, tenant_id, id).await {
        return Err(Template::render(
            "clients/index",
            context! {
                title: "Clients",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                clients: Vec::<crate::models::Client>::new(),
                error: message,
            },
        ));
    }

    Ok(Redirect::to(uri!(clients_index(
        slug = current_user.tenant_slug,
        page = Option::<usize>::None
    ))))
}

#[get("/<slug>/clients/<id>/contacts/new")]
pub async fn contact_new_form(
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
        return Err(Redirect::to(uri!(contact_new_form(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Err(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }

    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Ok(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };

    Ok(Template::render(
        "clients/contact_new",
        context! {
            title: "New contact",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            error: Option::<String>::None,
            client: client,
            form: ClientContactFormView::new("", "", "", "", "", ""),
        },
    ))
}

#[post("/<slug>/clients/<id>/contacts", data = "<form>")]
pub async fn contact_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    form: Form<ClientContactForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Err(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };

    match client_service::create_contact(db, tenant_id, id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
        "clients/contact_new",
        context! {
            title: "New contact",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            error: err.message,
            client: client,
            form: err.form,
        },
    )),
    }
}

#[get("/<slug>/clients/<id>/contacts/<contact_id>/edit")]
pub async fn contact_edit_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    contact_id: i64,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(contact_edit_form(
            slug = current_user.tenant_slug,
            id = id,
            contact_id = contact_id
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Err(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }

    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Ok(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };

    let contact = match client_service::find_contact_by_id(db, tenant_id, id, contact_id).await {
        Ok(Some(contact)) => contact,
        _ => {
            let tenant_slug = current_user.tenant_slug.clone();
            let (contacts_pagination, appointments_pagination) =
                empty_client_show_pagination(&tenant_slug, id);
            return Ok(Template::render(
                "clients/show",
                context! {
                    title: "Client details",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    client: client,
                    contacts: Vec::<crate::models::ClientContact>::new(),
                    contacts_count: 0,
                    appointments: Vec::<crate::models::Appointment>::new(),
                    appointments_count: 0,
                    deployments_count: 0,
                    contacts_pagination: contacts_pagination,
                    appointments_pagination: appointments_pagination,
                    error: "Contact not found.".to_string(),
                },
            ))
        }
    };

    Ok(Template::render(
        "clients/contact_edit",
        context! {
            title: "Edit contact",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            client: client,
            error: Option::<String>::None,
            contact_id: contact.id,
            form: ClientContactFormView::new(
                contact.name,
                contact.address,
                contact.email,
                contact.phone,
                contact.department,
                contact.position,
            ),
        },
    ))
}

#[post("/<slug>/clients/<id>/contacts/<contact_id>", data = "<form>")]
pub async fn contact_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    contact_id: i64,
    form: Form<ClientContactForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Err(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };

    match client_service::update_contact(db, tenant_id, id, contact_id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
        "clients/contact_edit",
        context! {
            title: "Edit contact",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            client: client,
            error: err.message,
            contact_id: contact_id,
            form: err.form,
        },
    )),
    }
}

#[post("/<slug>/clients/<id>/contacts/<contact_id>/delete")]
pub async fn contact_delete(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    contact_id: i64,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_delete(db, &user, "clients").await {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }

    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Err(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };

    if let Err(message) = client_service::delete_contact(db, tenant_id, id, contact_id).await {
        let tenant_slug = current_user.tenant_slug.clone();
        let (contacts_pagination, appointments_pagination) =
            empty_client_show_pagination(&tenant_slug, id);
        return Err(Template::render(
            "clients/show",
            context! {
                title: "Client details",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                client: client,
                contacts: Vec::<crate::models::ClientContact>::new(),
                contacts_count: 0,
                appointments: Vec::<crate::models::Appointment>::new(),
                appointments_count: 0,
                deployments_count: 0,
                contacts_pagination: contacts_pagination,
                appointments_pagination: appointments_pagination,
                error: message,
            },
        ));
    }

    Ok(Redirect::to(uri!(client_show(
        slug = current_user.tenant_slug,
        id = id,
        contacts_page = Option::<usize>::None,
        appointments_page = Option::<usize>::None
    ))))
}

#[get("/<slug>/clients/<id>/contacts/<contact_id>/appointments/new")]
pub async fn appointment_new_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    contact_id: i64,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(appointment_new_form(
            slug = current_user.tenant_slug,
            id = id,
            contact_id = contact_id
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Err(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }

    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Ok(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };
    let contact = match client_service::find_contact_by_id(db, tenant_id, id, contact_id).await {
        Ok(Some(contact)) => contact,
        _ => {
            let tenant_slug = current_user.tenant_slug.clone();
            let (contacts_pagination, appointments_pagination) =
                empty_client_show_pagination(&tenant_slug, id);
            return Ok(Template::render(
                "clients/show",
                context! {
                    title: "Client details",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    client: client,
                    contacts: Vec::<crate::models::ClientContact>::new(),
                    contacts_count: 0,
                    appointments: Vec::<crate::models::Appointment>::new(),
                    appointments_count: 0,
                    deployments_count: 0,
                    contacts_pagination: contacts_pagination,
                    appointments_pagination: appointments_pagination,
                    error: "Contact not found.".to_string(),
                },
            ))
        }
    };
    let (scheduled_date, scheduled_time) = split_scheduled_for("");

    Ok(Template::render(
        "clients/appointment_new",
        context! {
            title: "New appointment",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            client: client,
            contact: contact,
            contact_id: contact_id,
            error: Option::<String>::None,
            form: AppointmentFormView::new("", "", "Scheduled", ""),
            scheduled_date: scheduled_date,
            scheduled_time: scheduled_time,
            status_options: appointment_service::status_options(),
        },
    ))
}

#[post("/<slug>/clients/<id>/contacts/<contact_id>/appointments", data = "<form>")]
pub async fn appointment_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    contact_id: i64,
    form: Form<AppointmentForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Err(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };
    let contact = match client_service::find_contact_by_id(db, tenant_id, id, contact_id).await {
        Ok(Some(contact)) => contact,
        _ => {
            let tenant_slug = current_user.tenant_slug.clone();
            let (contacts_pagination, appointments_pagination) =
                empty_client_show_pagination(&tenant_slug, id);
            return Err(Template::render(
                "clients/show",
                context! {
                    title: "Client details",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    client: client,
                    contacts: Vec::<crate::models::ClientContact>::new(),
                    contacts_count: 0,
                    appointments: Vec::<crate::models::Appointment>::new(),
                    appointments_count: 0,
                    deployments_count: 0,
                    contacts_pagination: contacts_pagination,
                    appointments_pagination: appointments_pagination,
                    error: "Contact not found.".to_string(),
                },
            ))
        }
    };

    match appointment_service::create_appointment(db, tenant_id, id, contact_id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        )))),
        Err(err) => {
            let (scheduled_date, scheduled_time) = split_scheduled_for(&err.form.scheduled_for);
            Err(Template::render(
                "clients/appointment_new",
                context! {
                    title: "New appointment",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    client: client,
                    contact: contact,
                    contact_id: contact_id,
                    error: err.message,
                    form: err.form,
                    scheduled_date: scheduled_date,
                    scheduled_time: scheduled_time,
                    status_options: appointment_service::status_options(),
                },
            ))
        }
    }
}

#[get("/<slug>/clients/<id>/appointments/<appointment_id>/edit")]
pub async fn appointment_edit_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    appointment_id: i64,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(appointment_edit_form(
            slug = current_user.tenant_slug,
            id = id,
            appointment_id = appointment_id
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Err(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }

    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Ok(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };

    let appointment = match appointment_service::find_appointment(db, tenant_id, id, appointment_id).await {
        Ok(Some(appointment)) => appointment,
        _ => {
            let tenant_slug = current_user.tenant_slug.clone();
            let (contacts_pagination, appointments_pagination) =
                empty_client_show_pagination(&tenant_slug, id);
            return Ok(Template::render(
                "clients/show",
                context! {
                    title: "Client details",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    client: client,
                    contacts: Vec::<crate::models::ClientContact>::new(),
                    appointments: Vec::<crate::models::Appointment>::new(),
                    contacts_count: 0,
                    appointments_count: 0,
                    deployments_count: 0,
                    contacts_pagination: contacts_pagination,
                    appointments_pagination: appointments_pagination,
                    error: "Appointment not found.".to_string(),
                },
            ))
        }
    };

    let (scheduled_date, scheduled_time) = split_scheduled_for(&appointment.scheduled_for);

    Ok(Template::render(
        "clients/appointment_edit",
        context! {
            title: "Edit appointment",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            client: client,
            appointment_id: appointment.id,
            contact_name: appointment.contact_name,
            error: Option::<String>::None,
            form: AppointmentFormView::new(
                appointment.title,
                appointment.scheduled_for,
                appointment.status,
                appointment.notes,
            ),
            scheduled_date: scheduled_date,
            scheduled_time: scheduled_time,
            status_options: appointment_service::status_options(),
        },
    ))
}

#[post("/<slug>/clients/<id>/appointments/<appointment_id>", data = "<form>")]
pub async fn appointment_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    appointment_id: i64,
    form: Form<AppointmentForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Err(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };

    let appointment = appointment_service::find_appointment(db, tenant_id, id, appointment_id)
        .await
        .ok()
        .flatten();
    let contact_name = appointment
        .as_ref()
        .map(|appointment| appointment.contact_name.clone())
        .unwrap_or_default();

    match appointment_service::update_appointment(db, tenant_id, id, appointment_id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        )))),
        Err(err) => {
            let (scheduled_date, scheduled_time) = split_scheduled_for(&err.form.scheduled_for);
            Err(Template::render(
                "clients/appointment_edit",
                context! {
                    title: "Edit appointment",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    client: client,
                    appointment_id: appointment_id,
                    contact_name: contact_name,
                    error: err.message,
                    form: err.form,
                    scheduled_date: scheduled_date,
                    scheduled_time: scheduled_time,
                    status_options: appointment_service::status_options(),
                },
            ))
        }
    }
}

#[post("/<slug>/clients/<id>/appointments/<appointment_id>/delete")]
pub async fn appointment_delete(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    appointment_id: i64,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_delete(db, &user, "clients").await {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }

    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Err(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };

    if let Err(message) = appointment_service::delete_appointment(db, tenant_id, id, appointment_id).await {
        let tenant_slug = current_user.tenant_slug.clone();
        let (contacts_pagination, appointments_pagination) =
            empty_client_show_pagination(&tenant_slug, id);
        return Err(Template::render(
            "clients/show",
            context! {
                title: "Client details",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                client: client,
                contacts: Vec::<crate::models::ClientContact>::new(),
                appointments: Vec::<crate::models::Appointment>::new(),
                contacts_count: 0,
                appointments_count: 0,
                deployments_count: 0,
                contacts_pagination: contacts_pagination,
                appointments_pagination: appointments_pagination,
                error: message,
            },
        ));
    }

    Ok(Redirect::to(uri!(client_show(
        slug = current_user.tenant_slug,
        id = id,
        contacts_page = Option::<usize>::None,
        appointments_page = Option::<usize>::None
    ))))
}

#[get("/<slug>/clients/<id>/contacts/<contact_id>/email")]
pub async fn contact_email_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    contact_id: i64,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(contact_email_form(
            slug = current_user.tenant_slug,
            id = id,
            contact_id = contact_id
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Err(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }

    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Ok(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };
    let contact = match client_service::find_contact_by_id(db, tenant_id, id, contact_id).await {
        Ok(Some(contact)) => contact,
        _ => {
            let tenant_slug = current_user.tenant_slug.clone();
            let (contacts_pagination, appointments_pagination) =
                empty_client_show_pagination(&tenant_slug, id);
            return Ok(Template::render(
                "clients/show",
                context! {
                    title: "Client details",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    client: client,
                    contacts: Vec::<crate::models::ClientContact>::new(),
                    contacts_count: 0,
                    appointments: Vec::<crate::models::Appointment>::new(),
                    appointments_count: 0,
                    deployments_count: 0,
                    contacts_pagination: contacts_pagination,
                    appointments_pagination: appointments_pagination,
                    error: "Contact not found.".to_string(),
                },
            ))
        }
    };

    Ok(Template::render(
        "clients/email_compose",
        context! {
            title: "Email contact",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            client: client,
            contact: contact.clone(),
            to_email: contact.email.clone(),
            cc_emails: "None",
            action_url: format!("/{}/clients/{}/contacts/{}/email", slug, id, contact_id),
            cancel_url: format!("/{}/clients/{}/profile", slug, id),
            error: Option::<String>::None,
            form: EmailFormView::new("", ""),
        },
    ))
}

#[post("/<slug>/clients/<id>/contacts/<contact_id>/email", data = "<form>")]
pub async fn contact_email_send(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    contact_id: i64,
    form: Form<EmailForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Err(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };
    let contact = match client_service::find_contact_by_id(db, tenant_id, id, contact_id).await {
        Ok(Some(contact)) => contact,
        _ => {
            let tenant_slug = current_user.tenant_slug.clone();
            let (contacts_pagination, appointments_pagination) =
                empty_client_show_pagination(&tenant_slug, id);
            return Err(Template::render(
                "clients/show",
                context! {
                    title: "Client details",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    client: client,
                    contacts: Vec::<crate::models::ClientContact>::new(),
                    contacts_count: 0,
                    appointments: Vec::<crate::models::Appointment>::new(),
                    appointments_count: 0,
                    deployments_count: 0,
                    contacts_pagination: contacts_pagination,
                    appointments_pagination: appointments_pagination,
                    error: "Contact not found.".to_string(),
                },
            ))
        }
    };

    match email_service::queue_email(
        db,
        tenant_id,
        Some(id),
        Some(contact_id),
        contact.email.clone(),
        Vec::new(),
        form.subject,
        form.body,
    )
    .await
    {
        Ok(_) => Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
            "clients/email_compose",
            context! {
                title: "Email contact",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                client: client,
                contact: contact.clone(),
                to_email: contact.email.clone(),
                cc_emails: "None",
                action_url: format!("/{}/clients/{}/contacts/{}/email", slug, id, contact_id),
                cancel_url: format!("/{}/clients/{}/profile", slug, id),
                error: err.message,
                form: err.form,
            },
        )),
    }
}

#[get("/<slug>/clients/<id>/email-blast")]
pub async fn client_email_blast_form(
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
        return Err(Redirect::to(uri!(client_email_blast_form(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Err(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }

    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Ok(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };
    let contacts = client_service::list_contacts(db, tenant_id, id)
        .await
        .unwrap_or_default();
    let mut cc_emails: Vec<String> = contacts
        .iter()
        .map(|contact| contact.email.trim().to_string())
        .filter(|email| !email.is_empty() && email != client.email.trim())
        .collect();
    cc_emails.sort();
    cc_emails.dedup();
    let cc_display = if cc_emails.is_empty() {
        "None".to_string()
    } else {
        cc_emails.join(", ")
    };

    Ok(Template::render(
        "clients/email_compose",
        context! {
            title: "Email blast",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            client: client.clone(),
            contact: Option::<crate::models::ClientContact>::None,
            to_email: client.email.clone(),
            cc_emails: cc_display,
            action_url: format!("/{}/clients/{}/email-blast", slug, id),
            cancel_url: format!("/{}/clients/{}/profile", slug, id),
            error: Option::<String>::None,
            form: EmailFormView::new("", ""),
        },
    ))
}

#[post("/<slug>/clients/<id>/email-blast", data = "<form>")]
pub async fn client_email_blast_send(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    form: Form<EmailForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    if !access_service::can_edit(db, &user, "clients").await {
        return Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        ))));
    }
    let form = form.into_inner();
    let client = match client_service::find_client_by_id(db, tenant_id, id).await {
        Ok(Some(client)) => client,
        _ => {
            return Err(Template::render(
                "clients/index",
                context! {
                    title: "Clients",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    clients: Vec::<crate::models::Client>::new(),
                    error: "Client not found.".to_string(),
                },
            ))
        }
    };
    let contacts = client_service::list_contacts(db, tenant_id, id)
        .await
        .unwrap_or_default();
    let mut cc_emails: Vec<String> = contacts
        .iter()
        .map(|contact| contact.email.trim().to_string())
        .filter(|email| !email.is_empty() && email != client.email.trim())
        .collect();
    cc_emails.sort();
    cc_emails.dedup();
    let cc_display = if cc_emails.is_empty() {
        "None".to_string()
    } else {
        cc_emails.join(", ")
    };

    match email_service::queue_email(
        db,
        tenant_id,
        Some(id),
        None,
        client.email.clone(),
        cc_emails,
        form.subject,
        form.body,
    )
    .await
    {
        Ok(_) => Ok(Redirect::to(uri!(client_show(
            slug = current_user.tenant_slug,
            id = id,
            contacts_page = Option::<usize>::None,
            appointments_page = Option::<usize>::None
        )))),
        Err(err) => Err(Template::render(
            "clients/email_compose",
            context! {
            title: "Email blast",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            client: client.clone(),
            contact: Option::<crate::models::ClientContact>::None,
            to_email: client.email.clone(),
            cc_emails: cc_display,
            action_url: format!("/{}/clients/{}/email-blast", slug, id),
            cancel_url: format!("/{}/clients/{}/profile", slug, id),
            error: err.message,
            form: err.form,
            },
        )),
    }
}



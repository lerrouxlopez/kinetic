use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket_dyn_templates::{context, Template};

use crate::models::{
    CurrentUserView,
    DeploymentUpdate,
    EmailForm,
    EmailFormView,
    InvoiceForm,
    InvoiceFormView,
};
use crate::services::{
    access_service,
    auth_service,
    email_service,
    invoice_service,
    tracking_service,
    workspace_service,
};
use crate::Db;

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

fn invoice_number(id: i64) -> String {
    format!("INV-{:05}", id)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn build_invoice_email_body(
    invoice: &crate::models::InvoiceSummary,
    invoice_number: &str,
    total_hours: f64,
    total_amount: f64,
    updates: &[DeploymentUpdate],
) -> String {
    let client_name = escape_html(&invoice.client_name);
    let crew_name = escape_html(&invoice.crew_name);
    let notes = escape_html(&invoice.notes);
    let start_at = escape_html(&invoice.start_at);
    let end_at = escape_html(&invoice.end_at);
    let currency = escape_html(&invoice.client_currency);

    let mut body = String::new();
    body.push_str(&format!("<h2>Invoice {}</h2>", escape_html(invoice_number)));
    body.push_str(&format!(
        "<p><strong>Client:</strong> {}<br><strong>Crew:</strong> {}<br><strong>Deployment:</strong> {} to {}</p>",
        client_name, crew_name, start_at, end_at
    ));
    body.push_str(&format!(
        "<p><strong>Total hours:</strong> {:.2}<br><strong>Rate:</strong> {:.2} {}<br><strong>Total:</strong> {:.2} {}</p>",
        total_hours, invoice.fee_per_hour, currency, total_amount, currency
    ));
    if !invoice.notes.trim().is_empty() {
        body.push_str(&format!("<p><strong>Notes:</strong> {}</p>", notes));
    }
    body.push_str("<h3>Deployment reports</h3>");
    if updates.is_empty() {
        body.push_str("<p>No deployment updates were recorded.</p>");
    } else {
        body.push_str("<table cellpadding=\"6\" cellspacing=\"0\" border=\"1\" style=\"border-collapse: collapse;\">");
        body.push_str("<thead><tr><th>Date</th><th>Start</th><th>Finish</th><th>Hours</th><th>Notes</th></tr></thead><tbody>");
        for update in updates {
            let work_date = escape_html(&update.work_date);
            let start_time = escape_html(&update.start_time);
            let end_time = escape_html(&update.end_time);
            let report = escape_html(&update.notes);
            body.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td><td>{}</td></tr>",
                work_date, start_time, end_time, update.hours_worked, report
            ));
        }
        body.push_str("</tbody></table>");
    }

    body
}

#[get("/<slug>/invoices")]
pub async fn invoices_index(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(invoices_index(
            slug = current_user.tenant_slug
        ))));
    }
    if !access_service::can_view(db, &user, "invoices").await {
        return Err(Redirect::to(uri!(crate::controllers::public_controller::dashboard(
            slug = current_user.tenant_slug
        ))));
    }

    let invoices = invoice_service::list_invoices_with_details(db, tenant_id)
        .await
        .unwrap_or_default();
    let invoice_items = invoices
        .into_iter()
        .map(|invoice| {
            let total_amount = invoice.total_hours * invoice.fee_per_hour;
            context! {
                id: invoice.id,
                deployment_id: invoice.deployment_id,
                invoice_number: invoice_number(invoice.id),
                client_name: invoice.client_name,
                crew_name: invoice.crew_name,
                created_at: invoice.created_at,
                status: invoice.status,
                total_hours: invoice.total_hours,
                total_amount: total_amount,
                currency: invoice.client_currency,
            }
        })
        .collect::<Vec<_>>();

    let pending = invoice_service::list_invoice_candidates(db, tenant_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|candidate| {
            let total_amount = candidate.total_hours * candidate.fee_per_hour;
            context! {
                deployment_id: candidate.deployment_id,
                client_name: candidate.client_name,
                crew_name: candidate.crew_name,
                start_at: candidate.start_at,
                end_at: candidate.end_at,
                fee_per_hour: candidate.fee_per_hour,
                total_hours: candidate.total_hours,
                total_amount: total_amount,
                currency: candidate.client_currency,
            }
        })
        .collect::<Vec<_>>();

    Ok(Template::render(
        "invoices/index",
        context! {
            title: "Invoices",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
            invoices: invoice_items,
            pending_invoices: pending,
            error: Option::<String>::None,
        },
    ))
}

#[get("/<slug>/invoices/new?<deployment_id>")]
pub async fn invoice_new_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    deployment_id: Option<i64>,
) -> Result<Template, Redirect> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Err(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Err(Redirect::to(uri!(invoice_new_form(
            slug = current_user.tenant_slug,
            deployment_id = deployment_id
        ))));
    }
    if !access_service::can_edit(db, &user, "invoices").await {
        return Err(Redirect::to(uri!(invoices_index(
            slug = current_user.tenant_slug
        ))));
    }

    let deployment_options = invoice_service::list_invoice_candidates_for_select(db, tenant_id)
        .await
        .unwrap_or_default();
    let selected = deployment_id.unwrap_or(0);

    Ok(Template::render(
        "invoices/new",
        context! {
            title: "New invoice",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
            error: Option::<String>::None,
            form: InvoiceFormView::new(selected, "Draft", ""),
            deployment_options: deployment_options,
            status_options: invoice_service::status_options(),
        },
    ))
}

#[post("/<slug>/invoices", data = "<form>")]
pub async fn invoice_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    form: Form<InvoiceForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(invoices_index(
            slug = current_user.tenant_slug
        ))));
    }
    if !access_service::can_edit(db, &user, "invoices").await {
        return Ok(Redirect::to(uri!(invoices_index(
            slug = current_user.tenant_slug
        ))));
    }
    let form = form.into_inner();
    let deployment_options = invoice_service::list_invoice_candidates_for_select(db, tenant_id)
        .await
        .unwrap_or_default();

    match invoice_service::create_invoice(db, tenant_id, form).await {
        Ok(invoice_id) => Ok(Redirect::to(uri!(invoice_show(
            slug = current_user.tenant_slug,
            id = invoice_id
        )))),
        Err(err) => Err(Template::render(
            "invoices/new",
            context! {
                title: "New invoice",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
                error: err.message,
                form: err.form,
                deployment_options: deployment_options,
                status_options: invoice_service::status_options(),
            },
        )),
    }
}

#[get("/<slug>/invoices/<id>")]
pub async fn invoice_show(
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
        return Err(Redirect::to(uri!(invoice_show(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }
    if !access_service::can_view(db, &user, "invoices").await {
        return Err(Redirect::to(uri!(invoices_index(
            slug = current_user.tenant_slug
        ))));
    }

    let invoice = match invoice_service::find_invoice_with_details(db, tenant_id, id).await {
        Ok(Some(invoice)) => invoice,
        _ => {
            let invoices = invoice_service::list_invoices_with_details(db, tenant_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|invoice| {
                    let total_amount = invoice.total_hours * invoice.fee_per_hour;
                    context! {
                        id: invoice.id,
                        deployment_id: invoice.deployment_id,
                        invoice_number: invoice_number(invoice.id),
                        client_name: invoice.client_name,
                        crew_name: invoice.crew_name,
                        created_at: invoice.created_at,
                        status: invoice.status,
                        total_hours: invoice.total_hours,
                        total_amount: total_amount,
                        currency: invoice.client_currency,
                    }
                })
                .collect::<Vec<_>>();
            return Ok(Template::render(
                "invoices/index",
                context! {
                    title: "Invoices",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
                    invoices: invoices,
                    pending_invoices: Vec::<serde_json::Value>::new(),
                    error: "Invoice not found.".to_string(),
                },
            ));
        }
    };

    let updates = tracking_service::list_updates(db, tenant_id, invoice.deployment_id)
        .await
        .unwrap_or_default();
    let total_hours: f64 = updates.iter().map(|update| update.hours_worked).sum();
    let total_amount = total_hours * invoice.fee_per_hour;

    Ok(Template::render(
        "invoices/show",
        context! {
            title: "Invoice",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
            invoice: invoice,
            invoice_number: invoice_number(id),
            updates: updates,
            total_hours: total_hours,
            total_amount: total_amount,
            error: Option::<String>::None,
        },
    ))
}

#[get("/<slug>/invoices/<id>/edit")]
pub async fn invoice_edit_form(
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
        return Err(Redirect::to(uri!(invoice_edit_form(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }
    if !access_service::can_edit(db, &user, "invoices").await {
        return Err(Redirect::to(uri!(invoice_show(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }

    let invoice = match invoice_service::find_invoice_with_details(db, tenant_id, id).await {
        Ok(Some(invoice)) => invoice,
        _ => {
            let invoices = invoice_service::list_invoices_with_details(db, tenant_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|invoice| {
                    let total_amount = invoice.total_hours * invoice.fee_per_hour;
                    context! {
                        id: invoice.id,
                        deployment_id: invoice.deployment_id,
                        invoice_number: invoice_number(invoice.id),
                        client_name: invoice.client_name,
                        crew_name: invoice.crew_name,
                        created_at: invoice.created_at,
                        status: invoice.status,
                        total_hours: invoice.total_hours,
                        total_amount: total_amount,
                        currency: invoice.client_currency,
                    }
                })
                .collect::<Vec<_>>();
            return Ok(Template::render(
                "invoices/index",
                context! {
                    title: "Invoices",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
                    invoices: invoices,
                    pending_invoices: Vec::<serde_json::Value>::new(),
                    error: "Invoice not found.".to_string(),
                },
            ));
        }
    };

    let form = InvoiceFormView::new(
        invoice.deployment_id,
        invoice.status.clone(),
        invoice.notes.clone(),
    );

    Ok(Template::render(
        "invoices/edit",
        context! {
            title: "Edit invoice",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
            error: Option::<String>::None,
            invoice: invoice,
            invoice_number: invoice_number(id),
            form: form,
            status_options: invoice_service::status_options(),
        },
    ))
}

#[post("/<slug>/invoices/<id>", data = "<form>")]
pub async fn invoice_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    form: Form<InvoiceForm>,
) -> Result<Redirect, Template> {
    let (tenant_id, user) = match tenant_from_cookies(cookies, db).await {
        Some(data) => data,
        None => return Ok(Redirect::to(uri!(crate::controllers::public_controller::login_form))),
    };
    let current_user = CurrentUserView::from(&user);
    if current_user.tenant_slug != slug {
        return Ok(Redirect::to(uri!(invoices_index(
            slug = current_user.tenant_slug
        ))));
    }
    if !access_service::can_edit(db, &user, "invoices").await {
        return Ok(Redirect::to(uri!(invoice_show(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }
    let form = form.into_inner();
    let invoice = match invoice_service::find_invoice_with_details(db, tenant_id, id).await {
        Ok(Some(invoice)) => invoice,
        _ => {
            let invoices = invoice_service::list_invoices_with_details(db, tenant_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|invoice| {
                    let total_amount = invoice.total_hours * invoice.fee_per_hour;
                    context! {
                        id: invoice.id,
                        deployment_id: invoice.deployment_id,
                        invoice_number: invoice_number(invoice.id),
                        client_name: invoice.client_name,
                        crew_name: invoice.crew_name,
                        created_at: invoice.created_at,
                        status: invoice.status,
                        total_hours: invoice.total_hours,
                        total_amount: total_amount,
                        currency: invoice.client_currency,
                    }
                })
                .collect::<Vec<_>>();
            return Err(Template::render(
                "invoices/index",
                context! {
                    title: "Invoices",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
                    invoices: invoices,
                    pending_invoices: Vec::<serde_json::Value>::new(),
                    error: "Invoice not found.".to_string(),
                },
            ));
        }
    };

    match invoice_service::update_invoice(db, tenant_id, id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(invoice_show(
            slug = current_user.tenant_slug,
            id = id
        )))),
        Err(err) => Err(Template::render(
            "invoices/edit",
            context! {
                title: "Edit invoice",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
                error: err.message,
                invoice: invoice,
                invoice_number: invoice_number(id),
                form: err.form,
                status_options: invoice_service::status_options(),
            },
        )),
    }
}

#[post("/<slug>/invoices/<id>/delete")]
pub async fn invoice_delete(
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
        return Ok(Redirect::to(uri!(invoices_index(
            slug = current_user.tenant_slug
        ))));
    }
    if !access_service::can_delete(db, &user, "invoices").await {
        return Ok(Redirect::to(uri!(invoice_show(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }

    if let Err(message) = invoice_service::delete_invoice(db, tenant_id, id).await {
        let invoices = invoice_service::list_invoices_with_details(db, tenant_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|invoice| {
                let total_amount = invoice.total_hours * invoice.fee_per_hour;
                context! {
                    id: invoice.id,
                    deployment_id: invoice.deployment_id,
                    invoice_number: invoice_number(invoice.id),
                    client_name: invoice.client_name,
                    crew_name: invoice.crew_name,
                    created_at: invoice.created_at,
                    status: invoice.status,
                    total_hours: invoice.total_hours,
                    total_amount: total_amount,
                    currency: invoice.client_currency,
                }
            })
            .collect::<Vec<_>>();
        return Err(Template::render(
            "invoices/index",
            context! {
                title: "Invoices",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
                invoices: invoices,
                pending_invoices: Vec::<serde_json::Value>::new(),
                error: message,
            },
        ));
    }

    Ok(Redirect::to(uri!(invoices_index(
        slug = current_user.tenant_slug
    ))))
}

#[get("/<slug>/invoices/<id>/email")]
pub async fn invoice_email_form(
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
        return Err(Redirect::to(uri!(invoice_email_form(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }
    if !access_service::can_view(db, &user, "invoices").await {
        return Err(Redirect::to(uri!(invoice_show(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }

    let invoice = match invoice_service::find_invoice_with_details(db, tenant_id, id).await {
        Ok(Some(invoice)) => invoice,
        _ => {
            let invoices = invoice_service::list_invoices_with_details(db, tenant_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|invoice| {
                    let total_amount = invoice.total_hours * invoice.fee_per_hour;
                    context! {
                        id: invoice.id,
                        deployment_id: invoice.deployment_id,
                        invoice_number: invoice_number(invoice.id),
                        client_name: invoice.client_name,
                        crew_name: invoice.crew_name,
                        created_at: invoice.created_at,
                        status: invoice.status,
                        total_hours: invoice.total_hours,
                        total_amount: total_amount,
                        currency: invoice.client_currency,
                    }
                })
                .collect::<Vec<_>>();
            return Ok(Template::render(
                "invoices/index",
                context! {
                    title: "Invoices",
                    current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
                    invoices: invoices,
                    pending_invoices: Vec::<serde_json::Value>::new(),
                    error: "Invoice not found.".to_string(),
                },
            ));
        }
    };

    let updates = tracking_service::list_updates(db, tenant_id, invoice.deployment_id)
        .await
        .unwrap_or_default();
    let total_hours: f64 = updates.iter().map(|update| update.hours_worked).sum();
    let total_amount = total_hours * invoice.fee_per_hour;
    let invoice_no = invoice_number(invoice.id);
    let subject = format!("Invoice {} for {}", invoice_no, &invoice.client_name);
    let body = build_invoice_email_body(&invoice, &invoice_no, total_hours, total_amount, &updates);
    let error = if invoice.client_email.trim().is_empty() {
        Some("Client email is required to send invoices.".to_string())
    } else {
        None
    };

    Ok(Template::render(
        "clients/email_compose",
        context! {
            title: "Email invoice",
            current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
            client: context! {
                company_name: invoice.client_name.clone(),
                email: invoice.client_email.clone()
            },
            contact: Option::<crate::models::ClientContact>::None,
            to_email: invoice.client_email.clone(),
            cc_emails: "None",
            action_url: format!("/{}/invoices/{}/email", slug, id),
            cancel_url: format!("/{}/invoices/{}", slug, id),
            error: error,
            form: EmailFormView::new(subject, body),
        },
    ))
}

#[post("/<slug>/invoices/<id>/email", data = "<form>")]
pub async fn invoice_email_send(
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
        return Ok(Redirect::to(uri!(invoice_show(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }
    if !access_service::can_edit(db, &user, "invoices").await {
        return Ok(Redirect::to(uri!(invoice_show(
            slug = current_user.tenant_slug,
            id = id
        ))));
    }
    let form = form.into_inner();

    let invoice = match invoice_service::find_invoice_with_details(db, tenant_id, id).await {
        Ok(Some(invoice)) => invoice,
        _ => {
            return Ok(Redirect::to(uri!(invoices_index(
                slug = current_user.tenant_slug
            ))))
        }
    };

    let to_email = invoice.client_email.trim().to_string();
    if to_email.is_empty() {
        return Err(Template::render(
            "clients/email_compose",
            context! {
                title: "Email invoice",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
                client: context! {
                    company_name: invoice.client_name.clone(),
                    email: invoice.client_email.clone()
                },
                contact: Option::<crate::models::ClientContact>::None,
                to_email: invoice.client_email.clone(),
                cc_emails: "None",
                action_url: format!("/{}/invoices/{}/email", slug, id),
                cancel_url: format!("/{}/invoices/{}", slug, id),
                error: "Client email is required to send invoices.".to_string(),
                form: EmailFormView::new(form.subject, form.body),
            },
        ));
    }

    match email_service::queue_email(
        db,
        tenant_id,
        Some(invoice.client_id),
        None,
        to_email,
        Vec::new(),
        form.subject,
        form.body,
    )
    .await
    {
        Ok(_) => Ok(Redirect::to(uri!(invoice_show(
            slug = current_user.tenant_slug,
            id = id
        )))),
        Err(err) => Err(Template::render(
            "clients/email_compose",
            context! {
                title: "Email invoice",
                current_user: Some(current_user),
            workspace_brand: workspace_brand(db, tenant_id).await,
                client: context! {
                    company_name: invoice.client_name.clone(),
                    email: invoice.client_email.clone()
                },
                contact: Option::<crate::models::ClientContact>::None,
                to_email: invoice.client_email.clone(),
                cc_emails: "None",
                action_url: format!("/{}/invoices/{}/email", slug, id),
                cancel_url: format!("/{}/invoices/{}", slug, id),
                error: err.message,
                form: err.form,
            },
        )),
    }
}


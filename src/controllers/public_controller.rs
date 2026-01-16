use rocket::form::Form;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket_dyn_templates::{context, Template};

use crate::models::{
    CurrentUserView,
    DeploymentForm,
    DeploymentFormView,
    DeploymentUpdateForm,
    DeploymentUpdateFormView,
    DiscussionForm,
    DiscussionFormView,
    WorkTimerForm,
    LoginForm,
    LoginView,
    RegisterForm,
    RegisterView,
    UserPermissionForm,
    WorkspaceThemeForm,
    WorkspaceRegisterForm,
    WorkspaceRegisterView,
    WorkspaceEmailSettingsForm,
};
use crate::repositories::{
    client_repo,
    crew_member_repo,
    deployment_repo,
    deployment_update_repo,
    email_repo,
    invoice_repo,
    tenant_repo,
    user_repo,
};
use crate::services::{
    access_service,
    auth_service,
    appointment_service,
    client_service,
    crew_service,
    deployment_discussion_service,
    deployment_service,
    invoice_service,
    email_service,
    tracking_service,
    workspace_service,
};
use crate::Db;
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn to_datetime_local(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "".to_string();
    }
    if trimmed.contains('T') {
        return trimmed.to_string();
    }
    trimmed.replace(' ', "T")
}

fn is_theme_locked(plan_key: &str) -> bool {
    plan_key.eq_ignore_ascii_case("free")
}

fn portal_progress(status: &str, update_count: i64) -> i64 {
    let base = if status.eq_ignore_ascii_case("completed") {
        100
    } else if status.eq_ignore_ascii_case("active") {
        60
    } else if status.eq_ignore_ascii_case("scheduled") {
        20
    } else if status.eq_ignore_ascii_case("cancelled") {
        0
    } else {
        10
    };
    let boost = if status.eq_ignore_ascii_case("completed") {
        0
    } else {
        (update_count * 5).min(30)
    };
    (base + boost).min(100)
}

fn portal_window(start_at: &str, end_at: &str) -> String {
    let start_trimmed = start_at.trim();
    let end_trimmed = end_at.trim();
    if start_trimmed.is_empty() && end_trimmed.is_empty() {
        "TBD".to_string()
    } else if end_trimmed.is_empty() {
        format!("Starts {start_trimmed}")
    } else if start_trimmed.is_empty() {
        format!("Ends {end_trimmed}")
    } else {
        format!("{start_trimmed} - {end_trimmed}")
    }
}

fn portal_note_preview(note: &str) -> String {
    let trimmed = note.trim();
    if trimmed.len() <= 120 {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..120])
    }
}

fn is_truthy(value: Option<&str>) -> bool {
    matches!(value, Some("1" | "true" | "yes" | "on"))
}

fn recommended_crews_view(
    crews: &[crate::models::Crew],
    required_skills: &str,
    compatibility_pref: &str,
) -> Vec<serde_json::Value> {
    if required_skills.trim().is_empty() && compatibility_pref.trim().is_empty() {
        return Vec::new();
    }
    crew_service::recommend_crews(crews, required_skills, compatibility_pref)
        .into_iter()
        .filter(|rec| rec.score > 0)
        .take(3)
        .map(|rec| {
            serde_json::json!({
                "id": rec.id,
                "name": rec.name,
                "status": rec.status,
                "score": rec.score,
                "skill_matches": rec.skill_matches,
                "compatibility_matches": rec.compatibility_matches
            })
        })
        .collect()
}

async fn current_user_from_cookies(
    cookies: &CookieJar<'_>,
    db: &Db,
) -> Option<crate::models::User> {
    let user_id = cookies.get_private("user_id").and_then(|c| c.value().parse().ok());
    let tenant_id = cookies.get_private("tenant_id").and_then(|c| c.value().parse().ok());
    match (user_id, tenant_id) {
        (Some(user_id), Some(tenant_id)) => auth_service::get_user_by_ids(db, user_id, tenant_id)
            .await
            .ok()
            .flatten(),
        _ => None,
    }
}

async fn workspace_user(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
) -> Result<crate::models::User, Redirect> {
    let user = match current_user_from_cookies(cookies, db).await {
        Some(user) => user,
        None => return Err(Redirect::to(uri!(login_form))),
    };
    if user.tenant_slug != slug {
        return Err(Redirect::to(uri!(dashboard(slug = user.tenant_slug))));
    }
    Ok(user)
}

async fn workspace_brand(db: &Db, tenant_id: i64) -> crate::models::WorkspaceBrandView {
    workspace_service::find_workspace_by_id(db, tenant_id)
        .await
        .ok()
        .flatten()
        .map(|workspace| workspace_service::workspace_brand_view(&workspace))
        .unwrap_or_else(workspace_service::default_workspace_brand_view)
}

#[get("/")]
pub async fn index(cookies: &CookieJar<'_>, db: &Db) -> Result<Redirect, Template> {
    match current_user_from_cookies(cookies, db).await {
        Some(user) => Ok(Redirect::to(uri!(dashboard(slug = user.tenant_slug)))),
        None => {
            let appointments_total = appointment_service::count_appointments_all(db)
                .await
                .unwrap_or(0);
            let workspaces_total = workspace_service::count_workspaces(db).await.unwrap_or(0);
            let active_crews_total = crew_service::count_active_crews_all(db)
                .await
                .unwrap_or(0);
            let deployments_total = deployment_service::count_deployments_all(db)
                .await
                .unwrap_or(0);
            Err(Template::render(
                "index",
                context! {
                    title: "Kinetic",
                    current_user: Option::<CurrentUserView>::None,
                    appointments_total: appointments_total,
                    workspaces_total: workspaces_total,
                    active_crews_total: active_crews_total,
                    deployments_total: deployments_total,
                },
            ))
        }
    }
}

#[get("/register")]
pub async fn register_form(cookies: &CookieJar<'_>, db: &Db) -> Template {
    let current_user = current_user_from_cookies(cookies, db)
        .await
        .map(|user| CurrentUserView::from(&user));
    Template::render(
        "register",
        context! {
            title: "Create your workspace",
            current_user: current_user,
            error: Option::<String>::None,
            form: RegisterView::new("", "", "free"),
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
            Ok(Redirect::to(uri!(dashboard(slug = user.tenant_slug))))
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

#[get("/<slug>/register")]
pub async fn workspace_register_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
) -> Result<Template, Redirect> {
    if current_user_from_cookies(cookies, db).await.is_some() {
        return Err(Redirect::to(uri!(dashboard(slug = slug))));
    }
    Ok(Template::render(
        "workspace/register",
        context! {
            title: "Join workspace",
            current_user: Option::<CurrentUserView>::None,
            tenant_slug: slug,
            error: Option::<String>::None,
            form: WorkspaceRegisterView::new(""),
        },
    ))
}

#[post("/<slug>/register", data = "<form>")]
pub async fn workspace_register_submit(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    form: Form<WorkspaceRegisterForm>,
) -> Result<Redirect, Template> {
    if current_user_from_cookies(cookies, db).await.is_some() {
        return Ok(Redirect::to(uri!(dashboard(slug = slug))));
    }
    let form = form.into_inner();
    match auth_service::register_workspace_user(db, slug, form).await {
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
            Ok(Redirect::to(uri!(dashboard(slug = user.tenant_slug))))
        }
        Err(err) => Err(Template::render(
            "workspace/register",
            context! {
                title: "Join workspace",
                current_user: Option::<CurrentUserView>::None,
                tenant_slug: slug,
                error: err.message,
                form: WorkspaceRegisterView::new(err.form.email),
            },
        )),
    }
}

#[get("/login")]
pub async fn login_form(cookies: &CookieJar<'_>, db: &Db) -> Template {
    let current_user = current_user_from_cookies(cookies, db)
        .await
        .map(|user| CurrentUserView::from(&user));
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
            Ok(Redirect::to(uri!(dashboard(slug = user.tenant_slug))))
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

#[get("/portal/view/<slug>/<token>?<hide_completed>")]
pub async fn client_portal(
    db: &Db,
    slug: &str,
    token: &str,
    hide_completed: Option<String>,
) -> Template {
    let hide_completed_flag = is_truthy(hide_completed.as_deref());
    let tenant_id = match tenant_repo::find_tenant_id_by_slug(db, slug).await {
        Ok(Some(id)) => id,
        _ => {
            return Template::render(
                "clients/portal",
                context! {
                    title: "Client portal",
                    current_user: Option::<CurrentUserView>::None,
                    workspace_brand: workspace_service::default_workspace_brand_view(),
                    client_name: "Client portal".to_string(),
                    portal_error: "Invalid portal link.".to_string(),
                    portal_slug: slug,
                    portal_token: token,
                    is_portal: true,
                    hide_completed: hide_completed_flag,
                    deployments: Vec::<serde_json::Value>::new(),
                    active_deployments: 0,
                    completed_deployments: 0,
                    overall_progress: 0,
                },
            );
        }
    };

    let workspace_brand = workspace_brand(db, tenant_id).await;
    let client = match client_service::find_client_by_portal_token(db, tenant_id, token).await {
        Ok(Some(client)) => client,
        _ => {
            return Template::render(
                "clients/portal",
                context! {
                    title: "Client portal",
                    current_user: Option::<CurrentUserView>::None,
                    workspace_brand: workspace_brand,
                    client_name: "Client portal".to_string(),
                    portal_error: "Portal link not found.".to_string(),
                    portal_slug: slug,
                    portal_token: token,
                    is_portal: true,
                    hide_completed: hide_completed_flag,
                    deployments: Vec::<serde_json::Value>::new(),
                    active_deployments: 0,
                    completed_deployments: 0,
                    overall_progress: 0,
                },
            );
        }
    };

    let deployments = deployment_repo::list_deployments_with_names_by_client(
        db,
        tenant_id,
        client.id,
    )
    .await
    .unwrap_or_default();
    let deployment_ids = deployments.iter().map(|deployment| deployment.id).collect::<Vec<_>>();
    let update_counts = deployment_update_repo::count_updates_for_deployments(
        db,
        tenant_id,
        &deployment_ids,
    )
    .await
    .unwrap_or_default();
    let latest_updates = deployment_update_repo::list_latest_updates_for_deployments(
        db,
        tenant_id,
        &deployment_ids,
    )
    .await
    .unwrap_or_default();
    let update_map = update_counts
        .into_iter()
        .collect::<HashMap<i64, i64>>();
    let latest_map = latest_updates
        .into_iter()
        .map(|(deployment_id, work_date, start_time, notes)| {
            (deployment_id, (work_date, start_time, notes))
        })
        .collect::<HashMap<i64, (String, String, String)>>();

    let mut progress_total = 0i64;
    let mut deployments_view = Vec::new();
    for deployment in deployments.iter() {
        if hide_completed_flag && deployment.status.eq_ignore_ascii_case("completed") {
            continue;
        }
        let update_count = *update_map.get(&deployment.id).unwrap_or(&0);
        let progress = portal_progress(&deployment.status, update_count);
        progress_total += progress;
        let latest_update = match latest_map.get(&deployment.id) {
            Some((work_date, start_time, notes)) => format!(
                "{} {} - {}",
                work_date,
                start_time,
                portal_note_preview(notes)
            ),
            None => "No updates yet.".to_string(),
        };
        let expected_window = portal_window(&deployment.start_at, &deployment.end_at);
        deployments_view.push(context! {
            id: deployment.id,
            crew_name: deployment.crew_name.clone(),
            status: deployment.status.clone(),
            deployment_type: deployment.deployment_type.clone(),
            info: deployment.info.clone(),
            expected_window: expected_window,
            progress: progress,
            updates_total: update_count,
            latest_update: latest_update,
        });
    }
    let active_deployments = deployments
        .iter()
        .filter(|deployment| deployment.status.eq_ignore_ascii_case("active"))
        .count() as i64;
    let completed_deployments = deployments
        .iter()
        .filter(|deployment| deployment.status.eq_ignore_ascii_case("completed"))
        .count() as i64;
    let overall_progress = if deployments.is_empty() {
        0
    } else {
        (progress_total / deployments.len() as i64).min(100)
    };

    Template::render(
        "clients/portal",
        context! {
            title: format!("{} portal", client.company_name),
            current_user: Option::<CurrentUserView>::None,
            workspace_brand: workspace_brand,
            client_name: client.company_name.clone(),
            portal_error: "".to_string(),
            portal_slug: slug,
            portal_token: token,
            is_portal: true,
            hide_completed: hide_completed_flag,
            client: client,
            deployments: deployments_view,
            active_deployments: active_deployments,
            completed_deployments: completed_deployments,
            overall_progress: overall_progress,
        },
    )
}

#[post("/portal/view/<slug>/<token>/deployments/<deployment_id>/complete?<hide_completed>")]
pub async fn client_portal_mark_complete(
    db: &Db,
    slug: &str,
    token: &str,
    deployment_id: i64,
    hide_completed: Option<String>,
) -> Redirect {
    let hide_completed_flag = is_truthy(hide_completed.as_deref());
    let portal_redirect = if hide_completed_flag {
        Redirect::to(format!("/portal/view/{}/{}?hide_completed=1", slug, token))
    } else {
        Redirect::to(uri!(client_portal(slug = slug, token = token, hide_completed = Option::<String>::None)))
    };
    let tenant_id = match tenant_repo::find_tenant_id_by_slug(db, slug).await {
        Ok(Some(id)) => id,
        _ => return portal_redirect,
    };
    let client = match client_service::find_client_by_portal_token(db, tenant_id, token).await {
        Ok(Some(client)) => client,
        _ => return portal_redirect,
    };
    let deployment = match deployment_repo::find_deployment_by_id(db, tenant_id, deployment_id).await {
        Ok(Some(deployment)) => deployment,
        _ => return portal_redirect,
    };
    if deployment.client_id != client.id {
        return portal_redirect;
    }
    let _ = deployment_repo::update_deployment_status(
        db,
        tenant_id,
        deployment_id,
        "Completed",
    )
    .await;
    portal_redirect
}

#[get("/<slug>/dashboard")]
pub async fn dashboard(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
) -> Result<Template, Redirect> {
    let user = workspace_user(cookies, db, slug).await?;
    if !access_service::can_view(db, &user, "dashboard").await {
        return Err(Redirect::to(uri!(login_form)));
    }
    let can_view_clients = access_service::can_view(db, &user, "clients").await;
    let can_view_crew = access_service::can_view(db, &user, "crew").await;
    let can_view_deployments = access_service::can_view(db, &user, "deployments").await;
    let can_view_tracking = access_service::can_view(db, &user, "tracking").await;
    let can_view_invoices = access_service::can_view(db, &user, "invoices").await;
    let can_view_settings = access_service::can_view(db, &user, "settings").await;
    let is_owner = access_service::is_owner(&user.role);
    let is_admin = access_service::is_admin(&user.role);
    let is_sales = access_service::is_sales(&user.role);
    let is_accounting = access_service::is_accounting(&user.role);
    let is_employee = access_service::is_employee(&user.role);
    let is_operations = access_service::is_operations(&user.role);
    let is_admin_workspace = user.tenant_slug == "admin" || user.is_super_admin;
    let active_timer = tracking_service::active_timer(db, user.tenant_id, user.id)
        .await
        .ok()
        .flatten();
    let active_timer_label = match active_timer.as_ref() {
        Some(timer) => {
            deployment_service::find_deployment_label(db, user.tenant_id, timer.deployment_id)
                .await
                .ok()
                .flatten()
        }
        None => None,
    };
    let clients_total = if can_view_clients {
        if is_admin_workspace {
            client_service::count_clients_all(db).await.unwrap_or(0)
        } else {
            client_service::count_clients(db, user.tenant_id)
                .await
                .unwrap_or(0)
        }
    } else {
        0
    };
    let crew_ids = if can_view_deployments && is_employee {
        crew_member_repo::list_crew_ids_for_user(db, user.tenant_id, user.id, &user.email)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let deployments_total = if can_view_deployments {
        if is_admin_workspace {
            deployment_service::count_deployments_all(db)
                .await
                .unwrap_or(0)
        } else if is_employee {
            deployment_service::count_deployments_for_crews(db, user.tenant_id, &crew_ids)
                .await
                .unwrap_or(0)
        } else {
            deployment_service::count_deployments(db, user.tenant_id)
                .await
                .unwrap_or(0)
        }
    } else {
        0
    };
    let crews_total = if can_view_crew {
        if is_admin_workspace {
            crew_service::count_crews_all(db).await.unwrap_or(0)
        } else {
            crew_service::count_crews(db, user.tenant_id).await.unwrap_or(0)
        }
    } else {
        0
    };
    let deployment_status_counts = if can_view_deployments {
        if is_admin_workspace {
            deployment_service::count_deployments_by_status_all(db)
                .await
                .unwrap_or_default()
        } else if is_employee {
            deployment_service::count_deployments_by_status_for_crews(
                db,
                user.tenant_id,
                &crew_ids,
            )
            .await
            .unwrap_or_default()
        } else {
            deployment_service::count_deployments_by_status(db, user.tenant_id)
                .await
                .unwrap_or_default()
        }
    } else {
        Vec::new()
    };
    let deployment_statuses = ["Scheduled", "Active", "Completed", "Cancelled"];
    let deployment_status_chart = deployment_statuses
        .iter()
        .map(|status| {
            let count = deployment_status_counts
                .iter()
                .find(|(label, _)| label.eq_ignore_ascii_case(status))
                .map(|(_, count)| *count)
                .unwrap_or(0);
            let percent = if deployments_total > 0 {
                ((count as f64 / deployments_total as f64) * 100.0).round() as i64
            } else {
                0
            };
            context! {
                label: *status,
                count: count,
                percent: percent,
            }
        })
        .collect::<Vec<_>>();
    let appointment_total = if can_view_clients {
        if is_admin_workspace {
            appointment_service::count_appointments_all(db)
                .await
                .unwrap_or(0)
        } else {
            appointment_service::count_appointments_total(db, user.tenant_id)
                .await
                .unwrap_or(0)
        }
    } else {
        0
    };
    let appointment_status_counts = if can_view_clients {
        if is_admin_workspace {
            appointment_service::count_appointments_by_status_all(db)
                .await
                .unwrap_or_default()
        } else {
            appointment_service::count_appointments_by_status(db, user.tenant_id)
                .await
                .unwrap_or_default()
        }
    } else {
        Vec::new()
    };
    let appointment_statuses = ["Scheduled", "Confirmed", "Attended", "Cancelled", "No-Show"];
    let appointment_status_chart = appointment_statuses
        .iter()
        .map(|status| {
            let count = appointment_status_counts
                .iter()
                .find(|(label, _)| label.eq_ignore_ascii_case(status))
                .map(|(_, count)| *count)
                .unwrap_or(0);
            let percent = if appointment_total > 0 {
                ((count as f64 / appointment_total as f64) * 100.0).round() as i64
            } else {
                0
            };
            context! {
                label: *status,
                count: count,
                percent: percent,
            }
        })
        .collect::<Vec<_>>();
    let email_total = if can_view_settings {
        if is_admin_workspace {
            email_service::count_outbound_emails_all(db)
                .await
                .unwrap_or(0)
        } else {
            email_service::count_outbound_emails(db, user.tenant_id)
                .await
                .unwrap_or(0)
        }
    } else {
        0
    };
    let email_status_counts = if can_view_settings {
        if is_admin_workspace {
            email_service::count_outbound_emails_by_status_all(db)
                .await
                .unwrap_or_default()
        } else {
            email_service::count_outbound_emails_by_status(db, user.tenant_id)
                .await
                .unwrap_or_default()
        }
    } else {
        Vec::new()
    };
    let email_statuses = ["Sent", "Queued", "Failed"];
    let email_status_chart = email_statuses
        .iter()
        .map(|status| {
            let count = email_status_counts
                .iter()
                .find(|(label, _)| label.eq_ignore_ascii_case(status))
                .map(|(_, count)| *count)
                .unwrap_or(0);
            let percent = if email_total > 0 {
                ((count as f64 / email_total as f64) * 100.0).round() as i64
            } else {
                0
            };
            context! {
                label: *status,
                count: count,
                percent: percent,
            }
        })
        .collect::<Vec<_>>();
    let invoice_total = if can_view_invoices {
        if is_admin_workspace {
            invoice_service::count_invoices_all(db)
                .await
                .unwrap_or(0)
        } else {
            invoice_service::count_invoices(db, user.tenant_id)
                .await
                .unwrap_or(0)
        }
    } else {
        0
    };
    let invoice_status_counts = if can_view_invoices {
        if is_admin_workspace {
            invoice_service::count_invoices_by_status_all(db)
                .await
                .unwrap_or_default()
        } else {
            invoice_service::count_invoices_by_status(db, user.tenant_id)
                .await
                .unwrap_or_default()
        }
    } else {
        Vec::new()
    };
    let invoice_draft_total = invoice_status_counts
        .iter()
        .find(|(label, _)| label.eq_ignore_ascii_case("Draft"))
        .map(|(_, count)| *count)
        .unwrap_or(0);
    let invoice_sent_total = invoice_status_counts
        .iter()
        .find(|(label, _)| label.eq_ignore_ascii_case("Sent"))
        .map(|(_, count)| *count)
        .unwrap_or(0);
    let invoice_paid_total = invoice_status_counts
        .iter()
        .find(|(label, _)| label.eq_ignore_ascii_case("Paid"))
        .map(|(_, count)| *count)
        .unwrap_or(0);
    let invoice_status_chart = invoice_service::status_options()
        .iter()
        .map(|status| {
            let count = invoice_status_counts
                .iter()
                .find(|(label, _)| label.eq_ignore_ascii_case(status))
                .map(|(_, count)| *count)
                .unwrap_or(0);
            let percent = if invoice_total > 0 {
                ((count as f64 / invoice_total as f64) * 100.0).round() as i64
            } else {
                0
            };
            context! {
                label: *status,
                count: count,
                percent: percent,
            }
        })
        .collect::<Vec<_>>();
    let tracking_reports_total = if is_employee {
        tracking_service::count_updates_for_crews(db, user.tenant_id, &crew_ids)
            .await
            .unwrap_or(0)
    } else {
        0
    };
    let new_deployments_today = if can_view_deployments {
        if is_admin_workspace {
            deployment_repo::count_new_deployments_today_all(db)
                .await
                .unwrap_or(0)
        } else {
            deployment_repo::count_new_deployments_today(db, user.tenant_id)
                .await
                .unwrap_or(0)
        }
    } else {
        0
    };
    let overdue_updates = if can_view_tracking {
        deployment_update_repo::list_overdue_active_deployments(db, user.tenant_id, 24)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let idle_crews = if can_view_crew {
        crew_service::list_idle_crews(db, user.tenant_id, 5)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let outcome_kpis = if can_view_deployments {
        let avg_hours_to_ready = deployment_update_repo::average_hours_to_first_update(
            db,
            user.tenant_id,
        )
        .await
        .unwrap_or(0.0);
        let completed_total =
            deployment_update_repo::count_completed_deployments(db, user.tenant_id)
                .await
                .unwrap_or(0);
        let issue_completed =
            deployment_update_repo::count_completed_with_issue_keywords(db, user.tenant_id)
                .await
                .unwrap_or(0);
        let rework_completed =
            deployment_update_repo::count_completed_with_rework_keywords(db, user.tenant_id)
                .await
                .unwrap_or(0);
        let first_time_fix = if completed_total > 0 {
            (((completed_total - issue_completed) as f64 / completed_total as f64) * 100.0)
                .round() as i64
        } else {
            0
        };
        let rework_rate = if completed_total > 0 {
            ((rework_completed as f64 / completed_total as f64) * 100.0).round() as i64
        } else {
            0
        };
        let active_deployments = deployment_status_counts
            .iter()
            .find(|(label, _)| label.eq_ignore_ascii_case("Active"))
            .map(|(_, count)| *count)
            .unwrap_or(0);
        let utilization = if crews_total > 0 {
            ((active_deployments as f64 / crews_total as f64) * 100.0).round() as i64
        } else {
            0
        };
        Some(context! {
            avg_hours_to_ready: avg_hours_to_ready,
            first_time_fix: first_time_fix,
            rework_rate: rework_rate,
            utilization: utilization,
            quality: first_time_fix,
        })
    } else {
        None
    };
    let crew_stats = if can_view_crew {
        let crews = if is_admin_workspace {
            crew_service::list_crews_all(db).await.unwrap_or_default()
        } else {
            crew_service::list_crews(db, user.tenant_id)
                .await
                .unwrap_or_default()
        };
        crew_service::stats_from_crews(&crews)
    } else {
        crate::models::CrewStats {
            total_crews: 0,
            active_crews: 0,
            idle_crews: 0,
            on_leave_crews: 0,
            total_members: 0,
        }
    };
    let crew_status_chart = [
        ("Active", crew_stats.active_crews),
        ("Idle", crew_stats.idle_crews),
        ("On Leave", crew_stats.on_leave_crews),
    ]
    .iter()
    .map(|(label, count)| {
        let percent = if crew_stats.total_crews > 0 {
            ((*count as f64 / crew_stats.total_crews as f64) * 100.0).round() as i64
        } else {
            0
        };
        context! {
            label: *label,
            count: *count,
            percent: percent,
        }
    })
    .collect::<Vec<_>>();
    Ok(Template::render(
        "dashboard",
        context! {
            title: "Dashboard",
            current_user: Some(CurrentUserView::from(&user)),
            current_user_id: user.id,
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            tenant_slug: user.tenant_slug,
            email: user.email,
            can_view_clients: can_view_clients,
            can_view_crew: can_view_crew,
            can_view_deployments: can_view_deployments,
            can_view_tracking: can_view_tracking,
            can_view_invoices: can_view_invoices,
            can_view_settings: can_view_settings,
            is_owner: is_owner,
            is_admin: is_admin,
            is_sales: is_sales,
            is_accounting: is_accounting,
            is_employee: is_employee,
            is_operations: is_operations,
            clients_total: clients_total,
            deployments_total: deployments_total,
            crews_total: crews_total,
            deployment_status_chart: deployment_status_chart,
            crew_status_chart: crew_status_chart,
            total_members: crew_stats.total_members,
            appointment_total: appointment_total,
            appointment_status_chart: appointment_status_chart,
            email_total: email_total,
            email_status_chart: email_status_chart,
            invoice_total: invoice_total,
            invoice_status_chart: invoice_status_chart,
            invoice_draft_total: invoice_draft_total,
            invoice_sent_total: invoice_sent_total,
            invoice_paid_total: invoice_paid_total,
            tracking_reports_total: tracking_reports_total,
            new_deployments_today: new_deployments_today,
            overdue_updates: overdue_updates,
            idle_crews: idle_crews,
            outcome_kpis: outcome_kpis,
            active_timer: active_timer,
            active_timer_label: active_timer_label,
        },
    ))
}

#[get("/<slug>/plans")]
pub async fn plans(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
) -> Result<Template, Redirect> {
    let user = workspace_user(cookies, db, slug).await?;
    let workspace = workspace_service::find_workspace_by_id(db, user.tenant_id)
        .await
        .ok()
        .flatten();
    let current_plan_key = workspace
        .as_ref()
        .map(|workspace| workspace.plan_key.clone())
        .unwrap_or_else(|| user.plan_key.clone());
    let current_plan = workspace_service::find_plan(&current_plan_key);
    let upgrade_options = workspace_service::upgrade_options(&current_plan_key);
    let free_plan_limits = workspace_service::free_plan_limits(db).await;
    let free_plan_expiry_days = free_plan_limits.expires_after_days.unwrap_or(0);

    Ok(Template::render(
        "workspace/plans",
        context! {
            title: "Upgrade plan",
            current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            current_plan_key: current_plan_key,
            current_plan: current_plan,
            upgrade_options: upgrade_options,
            free_plan_expiry_days: free_plan_expiry_days,
        },
    ))
}

#[get("/<slug>/tracking?<deployment_id>")]
pub async fn tracking(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    deployment_id: Option<i64>,
) -> Result<Template, Redirect> {
    let user = workspace_user(cookies, db, slug).await?;
    if !access_service::can_view(db, &user, "tracking").await {
        return Err(Redirect::to(uri!(dashboard(slug = user.tenant_slug))));
    }
    let _ = tracking_service::close_stale_timers(db, user.tenant_id).await;
    let deployments = if access_service::is_employee(&user.role) {
        let crew_ids =
            crew_member_repo::list_crew_ids_for_user(db, user.tenant_id, user.id, &user.email)
            .await
            .unwrap_or_default();
        deployment_service::list_deployments_for_select_for_crews(db, user.tenant_id, &crew_ids)
            .await
            .unwrap_or_default()
    } else {
        deployment_service::list_deployments_for_select(db, user.tenant_id)
            .await
            .unwrap_or_default()
    };
    let allowed_ids = deployments.iter().map(|item| item.id).collect::<Vec<_>>();
    let active_timer = tracking_service::active_timer(db, user.tenant_id, user.id)
        .await
        .ok()
        .flatten();
    let selected_deployment = deployment_id
        .filter(|id| allowed_ids.contains(id))
        .or_else(|| {
            active_timer
                .as_ref()
                .map(|timer| timer.deployment_id)
                .filter(|id| allowed_ids.contains(id))
        })
        .or_else(|| deployments.first().map(|item| item.id))
        .filter(|id| *id > 0);
    let is_admin_viewer =
        access_service::is_owner(&user.role) || access_service::is_admin(&user.role);
    let is_owner = access_service::is_owner(&user.role);
    let updates = match selected_deployment {
        Some(deployment_id) if access_service::is_employee(&user.role) => {
            tracking_service::list_updates_for_user(
                db,
                user.tenant_id,
                deployment_id,
                user.id,
            )
            .await
            .unwrap_or_default()
        }
        Some(deployment_id) => tracking_service::list_updates(db, user.tenant_id, deployment_id)
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let missing_user_ids = if is_admin_viewer {
        if let Some(deployment_id) = selected_deployment {
            tracking_service::count_updates_missing_user_id(
                db,
                user.tenant_id,
                deployment_id,
            )
            .await
            .unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };
    let chart_updates = updates
        .iter()
        .filter(|update| !update.is_placeholder)
        .cloned()
        .collect::<Vec<_>>();
    let chart_data = build_hours_chart(&chart_updates);
    let coverage_map = build_live_coverage_map(db, &user).await;
    let coverage_map_json =
        serde_json::to_string(&coverage_map).unwrap_or_else(|_| "{}".to_string());
    let discussions = match selected_deployment {
        Some(deployment_id) if deployment_id > 0 => {
            deployment_discussion_service::list_discussions_by_deployment(
                db,
                user.tenant_id,
                deployment_id,
            )
            .await
            .unwrap_or_default()
        }
        _ => Vec::new(),
    };
    let can_edit_tracking = access_service::can_edit(db, &user, "tracking").await;
    let can_delete_tracking = access_service::can_delete(db, &user, "tracking").await;
    let can_edit_updates = is_owner
        || updates
            .iter()
            .any(|update| update.is_placeholder && update.user_id == Some(user.id));
    Ok(Template::render(
        "tracking/index",
        context! {
            title: "Tracking",
            current_user: Some(CurrentUserView::from(&user)),
            current_user_id: user.id,
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            deployments: deployments,
            selected_deployment: selected_deployment,
            updates: updates,
            chart_data: chart_data,
            coverage_map_json: coverage_map_json,
            include_map: true,
            error: Option::<String>::None,
            is_employee: access_service::is_employee(&user.role),
            is_admin_viewer: is_admin_viewer,
            missing_user_ids: missing_user_ids,
            is_owner: is_owner,
            can_edit_updates: can_edit_updates,
            can_edit_tracking: can_edit_tracking,
            can_delete_tracking: can_delete_tracking,
            discussions: discussions,
            active_timer: active_timer,
            form: DeploymentUpdateFormView::new(
                selected_deployment.unwrap_or(0),
                "",
                "",
                "",
                "",
            ),
        },
    ))
}

#[post("/<slug>/tracking", data = "<form>")]
pub async fn tracking_update_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    form: Form<DeploymentUpdateForm>,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::can_edit(db, &user, "tracking").await {
        return Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Option::<i64>::None
        ))));
    }
    let form = form.into_inner();
    let deployments = if access_service::is_employee(&user.role) {
        let crew_ids =
            crew_member_repo::list_crew_ids_for_user(db, user.tenant_id, user.id, &user.email)
            .await
            .unwrap_or_default();
        deployment_service::list_deployments_for_select_for_crews(db, user.tenant_id, &crew_ids)
            .await
            .unwrap_or_default()
    } else {
        deployment_service::list_deployments_for_select(db, user.tenant_id)
            .await
            .unwrap_or_default()
    };
    let selected_deployment = if form.deployment_id > 0 {
        Some(form.deployment_id)
    } else {
        deployments.first().map(|item| item.id)
    };

    if access_service::is_employee(&user.role) {
        let allowed_ids = deployments.iter().map(|item| item.id).collect::<Vec<_>>();
        if let Some(deployment_id) = selected_deployment {
            if !allowed_ids.contains(&deployment_id) {
                let active_timer = tracking_service::active_timer(db, user.tenant_id, user.id)
                    .await
                    .ok()
                    .flatten();
                let coverage_map = build_live_coverage_map(db, &user).await;
                let coverage_map_json =
                    serde_json::to_string(&coverage_map).unwrap_or_else(|_| "{}".to_string());
                return Err(Template::render(
                    "tracking/index",
                    context! {
                        title: "Tracking",
                        current_user: Some(CurrentUserView::from(&user)),
                        current_user_id: user.id,
                        workspace_brand: workspace_brand(db, user.tenant_id).await,
                        deployments: deployments,
                        selected_deployment: Option::<i64>::None,
                        updates: Vec::<crate::models::DeploymentUpdate>::new(),
                        chart_data: build_hours_chart(&[]),
                        coverage_map_json: coverage_map_json,
                        include_map: true,
                        error: "You do not have access to that deployment.".to_string(),
                        is_employee: access_service::is_employee(&user.role),
                        is_admin_viewer: access_service::is_owner(&user.role) || access_service::is_admin(&user.role),
                        missing_user_ids: 0,
                        is_owner: access_service::is_owner(&user.role),
                        can_edit_updates: access_service::is_owner(&user.role),
                        can_edit_tracking: access_service::can_edit(db, &user, "tracking").await,
                        can_delete_tracking: access_service::can_delete(db, &user, "tracking").await,
                        discussions: Vec::<crate::models::DeploymentDiscussion>::new(),
                        active_timer: active_timer,
                        form: DeploymentUpdateFormView::new(
                            0,
                            "",
                            "",
                            "",
                            "",
                        ),
                    },
                ));
            }
        }
    }

    match tracking_service::create_update(db, user.tenant_id, user.id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = selected_deployment
        )))),
        Err(err) => {
            let deployment_id = selected_deployment.unwrap_or(0);
            let is_admin_viewer =
                access_service::is_owner(&user.role) || access_service::is_admin(&user.role);
            let is_owner = access_service::is_owner(&user.role);
            let active_timer = tracking_service::active_timer(db, user.tenant_id, user.id)
                .await
                .ok()
                .flatten();
            let updates = if deployment_id > 0 {
                if access_service::is_employee(&user.role) {
                    tracking_service::list_updates_for_user(
                        db,
                        user.tenant_id,
                        deployment_id,
                        user.id,
                    )
                    .await
                    .unwrap_or_default()
                } else {
                    tracking_service::list_updates(db, user.tenant_id, deployment_id)
                        .await
                        .unwrap_or_default()
                }
            } else {
                Vec::new()
            };
            let missing_user_ids = if is_admin_viewer && deployment_id > 0 {
                tracking_service::count_updates_missing_user_id(
                    db,
                    user.tenant_id,
                    deployment_id,
                )
                .await
                .unwrap_or(0)
            } else {
                0
            };
            let chart_updates = updates
                .iter()
                .filter(|update| !update.is_placeholder)
                .cloned()
                .collect::<Vec<_>>();
            let chart_data = build_hours_chart(&chart_updates);
            let coverage_map = build_live_coverage_map(db, &user).await;
            let coverage_map_json =
                serde_json::to_string(&coverage_map).unwrap_or_else(|_| "{}".to_string());
            let discussions = if deployment_id > 0 {
                deployment_discussion_service::list_discussions_by_deployment(
                    db,
                    user.tenant_id,
                    deployment_id,
                )
                .await
                .unwrap_or_default()
            } else {
                Vec::new()
            };
            let can_edit_tracking = access_service::can_edit(db, &user, "tracking").await;
            let can_delete_tracking = access_service::can_delete(db, &user, "tracking").await;
            let can_edit_updates = is_owner
                || updates
                    .iter()
                    .any(|update| update.is_placeholder && update.user_id == Some(user.id));
            Err(Template::render(
                "tracking/index",
                context! {
                    title: "Tracking",
                    current_user: Some(CurrentUserView::from(&user)),
                    current_user_id: user.id,
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    deployments: deployments,
                    selected_deployment: selected_deployment,
                    updates: updates,
                    chart_data: chart_data,
                    coverage_map_json: coverage_map_json,
                    include_map: true,
                    error: err.message,
                    is_employee: access_service::is_employee(&user.role),
                    is_admin_viewer: is_admin_viewer,
                    missing_user_ids: missing_user_ids,
                    is_owner: is_owner,
                    can_edit_updates: can_edit_updates,
                    can_edit_tracking: can_edit_tracking,
                    can_delete_tracking: can_delete_tracking,
                    discussions: discussions,
                    active_timer: active_timer,
                    form: err.form,
                },
            ))
        }
    }
}

#[post("/<slug>/tracking/start", data = "<form>")]
pub async fn tracking_timer_start(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    form: Form<WorkTimerForm>,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::can_edit(db, &user, "tracking").await {
        return Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Option::<i64>::None
        ))));
    }
    let form = form.into_inner();
    let deployments = if access_service::is_employee(&user.role) {
        let crew_ids =
            crew_member_repo::list_crew_ids_for_user(db, user.tenant_id, user.id, &user.email)
                .await
                .unwrap_or_default();
        deployment_service::list_deployments_for_select_for_crews(db, user.tenant_id, &crew_ids)
            .await
            .unwrap_or_default()
    } else {
        deployment_service::list_deployments_for_select(db, user.tenant_id)
            .await
            .unwrap_or_default()
    };
    let selected_deployment = if form.deployment_id > 0 {
        Some(form.deployment_id)
    } else {
        None
    };

    if access_service::is_employee(&user.role) {
        let allowed_ids = deployments.iter().map(|item| item.id).collect::<Vec<_>>();
        if let Some(deployment_id) = selected_deployment {
            if !allowed_ids.contains(&deployment_id) {
                return Err(render_tracking_error(
                    db,
                    &user,
                    deployments,
                    None,
                    "You do not have access to that deployment.",
                )
                .await);
            }
        }
    }

    let error_message = if form.deployment_id <= 0 {
        Some("Select a deployment before starting work.".to_string())
    } else {
        None
    };
    let start_result = if error_message.is_none() {
        tracking_service::start_timer(db, user.tenant_id, form.deployment_id, user.id).await
    } else {
        Err(error_message.unwrap())
    };

    match start_result {
        Ok(_) => Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = selected_deployment
        )))),
        Err(message) => Err(
            render_tracking_error(db, &user, deployments, selected_deployment, &message).await,
        ),
    }
}

#[post("/<slug>/tracking/stop", data = "<form>")]
pub async fn tracking_timer_stop(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    form: Form<WorkTimerForm>,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::can_edit(db, &user, "tracking").await {
        return Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Option::<i64>::None
        ))));
    }
    let form = form.into_inner();
    let deployments = if access_service::is_employee(&user.role) {
        let crew_ids =
            crew_member_repo::list_crew_ids_for_user(db, user.tenant_id, user.id, &user.email)
                .await
                .unwrap_or_default();
        deployment_service::list_deployments_for_select_for_crews(db, user.tenant_id, &crew_ids)
            .await
            .unwrap_or_default()
    } else {
        deployment_service::list_deployments_for_select(db, user.tenant_id)
            .await
            .unwrap_or_default()
    };
    let selected_deployment = if form.deployment_id > 0 {
        Some(form.deployment_id)
    } else {
        None
    };

    let stop_result = tracking_service::stop_timer(
        db,
        user.tenant_id,
        form.deployment_id,
        user.id,
    )
    .await;
    match stop_result {
        Ok(_) => Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = selected_deployment
        )))),
        Err(message) => Err(
            render_tracking_error(db, &user, deployments, selected_deployment, &message).await,
        ),
    }
}

#[get("/<slug>/tracking/updates/<id>/edit")]
pub async fn tracking_update_edit_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
) -> Result<Template, Redirect> {
    let user = workspace_user(cookies, db, slug).await?;
    let update = match tracking_service::find_update_by_id(db, user.tenant_id, id).await {
        Ok(Some(update)) => update,
        _ => {
            return Err(Redirect::to(uri!(tracking(
                slug = user.tenant_slug,
                deployment_id = Option::<i64>::None
            ))))
        }
    };
    let is_owner = access_service::is_owner(&user.role);
    let can_edit_placeholder =
        update.is_placeholder && update.user_id == Some(user.id);
    if !is_owner && !can_edit_placeholder {
        return Err(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Option::<i64>::None
        ))));
    }

    Ok(Template::render(
        "tracking/edit",
        context! {
            title: "Edit update",
            current_user: Some(CurrentUserView::from(&user)),
            current_user_id: user.id,
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            error: Option::<String>::None,
            update_id: update.id,
            deployment_id: update.deployment_id,
            can_edit_times: is_owner,
            form: DeploymentUpdateFormView::new(
                update.deployment_id,
                update.work_date,
                update.start_time,
                update.end_time,
                update.notes,
            ),
        },
    ))
}

#[post("/<slug>/tracking/updates/<id>", data = "<form>")]
pub async fn tracking_update_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    form: Form<DeploymentUpdateForm>,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    let existing = match tracking_service::find_update_by_id(db, user.tenant_id, id).await {
        Ok(Some(update)) => update,
        _ => {
            return Ok(Redirect::to(uri!(tracking(
                slug = user.tenant_slug,
                deployment_id = Option::<i64>::None
            ))))
        }
    };
    let is_owner = access_service::is_owner(&user.role);
    let can_edit_placeholder =
        existing.is_placeholder && existing.user_id == Some(user.id);
    if !is_owner && !can_edit_placeholder {
        return Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Option::<i64>::None
        ))));
    }
    let mut form = form.into_inner();
    if !is_owner {
        form.work_date = existing.work_date.clone();
        form.start_time = existing.start_time.clone();
        form.end_time = existing.end_time.clone();
    }
    match tracking_service::update_update(db, user.tenant_id, id, form).await {
        Ok(deployment_id) => Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Some(deployment_id)
        )))),
        Err(err) => Err(Template::render(
            "tracking/edit",
            context! {
                title: "Edit update",
                current_user: Some(CurrentUserView::from(&user)),
                current_user_id: user.id,
                workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: err.message,
                update_id: id,
                deployment_id: err.form.deployment_id,
                can_edit_times: is_owner,
                form: err.form,
            },
        )),
    }
}

#[post("/<slug>/tracking/updates/<id>/delete")]
pub async fn tracking_update_delete(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::is_owner(&user.role) {
        return Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Option::<i64>::None
        ))));
    }
    match tracking_service::delete_update(db, user.tenant_id, id).await {
        Ok(deployment_id) => Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Some(deployment_id)
        )))),
        Err(message) => {
            let deployments = deployment_service::list_deployments_for_select(db, user.tenant_id)
                .await
                .unwrap_or_default();
            let selected_deployment = deployments.first().map(|item| item.id);
            let updates = match selected_deployment {
                Some(deployment_id) => tracking_service::list_updates(
                    db,
                    user.tenant_id,
                    deployment_id,
                )
                .await
                .unwrap_or_default(),
                None => Vec::new(),
            };
            let chart_updates = updates
                .iter()
                .filter(|update| !update.is_placeholder)
                .cloned()
                .collect::<Vec<_>>();
            let chart_data = build_hours_chart(&chart_updates);
            let coverage_map = build_live_coverage_map(db, &user).await;
            let coverage_map_json =
                serde_json::to_string(&coverage_map).unwrap_or_else(|_| "{}".to_string());
            let discussions = if let Some(deployment_id) = selected_deployment {
                deployment_discussion_service::list_discussions_by_deployment(
                    db,
                    user.tenant_id,
                    deployment_id,
                )
                .await
                .unwrap_or_default()
            } else {
                Vec::new()
            };
            let can_edit_updates = true;
            let can_edit_tracking = access_service::can_edit(db, &user, "tracking").await;
            let can_delete_tracking = access_service::can_delete(db, &user, "tracking").await;
            let active_timer = tracking_service::active_timer(db, user.tenant_id, user.id)
                .await
                .ok()
                .flatten();
            Err(Template::render(
                "tracking/index",
                context! {
                    title: "Tracking",
                    current_user: Some(CurrentUserView::from(&user)),
                    current_user_id: user.id,
                    workspace_brand: workspace_brand(db, user.tenant_id).await,
                    deployments: deployments,
                    selected_deployment: selected_deployment,
                    updates: updates,
                    chart_data: chart_data,
                    coverage_map_json: coverage_map_json,
                    include_map: true,
                    error: message,
                    is_employee: access_service::is_employee(&user.role),
                    is_admin_viewer: true,
                    missing_user_ids: 0,
                    is_owner: true,
                    can_edit_updates: can_edit_updates,
                    can_edit_tracking: can_edit_tracking,
                    can_delete_tracking: can_delete_tracking,
                    discussions: discussions,
                    active_timer: active_timer,
                    form: DeploymentUpdateFormView::new(
                        selected_deployment.unwrap_or(0),
                        "",
                        "",
                        "",
                        "",
                    ),
                },
            ))
        }
    }
}

#[get("/<slug>/tracking/deployments/<deployment_id>/discussions/new")]
pub async fn tracking_discussion_new_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    deployment_id: i64,
) -> Result<Template, Redirect> {
    let user = workspace_user(cookies, db, slug).await?;
    if !access_service::can_edit(db, &user, "tracking").await {
        return Err(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Option::<i64>::None
        ))));
    }
    let deployment = match deployment_repo::find_deployment_by_id(db, user.tenant_id, deployment_id).await {
        Ok(Some(deployment)) => deployment,
        _ => {
            return Err(Redirect::to(uri!(tracking(
                slug = user.tenant_slug,
                deployment_id = Option::<i64>::None
            ))))
        }
    };
    let users = user_repo::list_users_by_tenant(db, user.tenant_id)
        .await
        .unwrap_or_default();

    Ok(Template::render(
        "tracking/discussion_new",
        context! {
            title: "New discussion",
            current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            deployment: deployment,
            users: users,
            error: Option::<String>::None,
            form: DiscussionFormView::new("", None),
        },
    ))
}

#[post("/<slug>/tracking/deployments/<deployment_id>/discussions", data = "<form>")]
pub async fn tracking_discussion_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    deployment_id: i64,
    form: Form<DiscussionForm>,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::can_edit(db, &user, "tracking").await {
        return Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Option::<i64>::None
        ))));
    }
    let deployments = if access_service::is_employee(&user.role) {
        let crew_ids =
            crew_member_repo::list_crew_ids_for_user(db, user.tenant_id, user.id, &user.email)
                .await
                .unwrap_or_default();
        deployment_service::list_deployments_for_select_for_crews(db, user.tenant_id, &crew_ids)
            .await
            .unwrap_or_default()
    } else {
        deployment_service::list_deployments_for_select(db, user.tenant_id)
            .await
            .unwrap_or_default()
    };
    let form = form.into_inner();
    let deployment = match deployment_repo::find_deployment_by_id(db, user.tenant_id, deployment_id).await {
        Ok(Some(deployment)) => deployment,
        _ => {
            return Err(
                render_tracking_error(db, &user, deployments, None, "Deployment not found.").await,
            )
        }
    };
    let users = user_repo::list_users_by_tenant(db, user.tenant_id)
        .await
        .unwrap_or_default();

    match deployment_discussion_service::create_discussion(
        db,
        user.tenant_id,
        deployment_id,
        user.id,
        form,
    )
    .await
    {
        Ok(_) => Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Some(deployment_id)
        )))),
        Err(err) => Err(Template::render(
            "tracking/discussion_new",
            context! {
                title: "New discussion",
                current_user: Some(CurrentUserView::from(&user)),
                workspace_brand: workspace_brand(db, user.tenant_id).await,
                deployment: deployment,
                users: users,
                error: err.message,
                form: err.form,
            },
        )),
    }
}

#[get("/<slug>/tracking/deployments/<deployment_id>/discussions/<discussion_id>/edit")]
pub async fn tracking_discussion_edit_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    deployment_id: i64,
    discussion_id: i64,
) -> Result<Template, Redirect> {
    let user = workspace_user(cookies, db, slug).await?;
    if !access_service::can_edit(db, &user, "tracking").await {
        return Err(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Option::<i64>::None
        ))));
    }
    let deployment = match deployment_repo::find_deployment_by_id(db, user.tenant_id, deployment_id).await {
        Ok(Some(deployment)) => deployment,
        _ => {
            return Err(Redirect::to(uri!(tracking(
                slug = user.tenant_slug,
                deployment_id = Option::<i64>::None
            ))))
        }
    };
    let discussion = match deployment_discussion_service::find_discussion_by_id(
        db,
        user.tenant_id,
        deployment_id,
        discussion_id,
    )
    .await
    .ok()
    .flatten()
    {
        Some(discussion) => discussion,
        None => {
            return Err(Redirect::to(uri!(tracking(
                slug = user.tenant_slug,
                deployment_id = Some(deployment_id)
            ))))
        }
    };
    let users = user_repo::list_users_by_tenant(db, user.tenant_id)
        .await
        .unwrap_or_default();

    Ok(Template::render(
        "tracking/discussion_edit",
        context! {
            title: "Edit discussion",
            current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            deployment: deployment,
            users: users,
            discussion_id: discussion_id,
            error: Option::<String>::None,
            form: DiscussionFormView::new(discussion.message, discussion.tagged_user_id),
        },
    ))
}

#[post("/<slug>/tracking/deployments/<deployment_id>/discussions/<discussion_id>", data = "<form>")]
pub async fn tracking_discussion_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    deployment_id: i64,
    discussion_id: i64,
    form: Form<DiscussionForm>,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::can_edit(db, &user, "tracking").await {
        return Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Option::<i64>::None
        ))));
    }
    let form = form.into_inner();
    let deployments = if access_service::is_employee(&user.role) {
        let crew_ids =
            crew_member_repo::list_crew_ids_for_user(db, user.tenant_id, user.id, &user.email)
                .await
                .unwrap_or_default();
        deployment_service::list_deployments_for_select_for_crews(db, user.tenant_id, &crew_ids)
            .await
            .unwrap_or_default()
    } else {
        deployment_service::list_deployments_for_select(db, user.tenant_id)
            .await
            .unwrap_or_default()
    };
    let deployment = match deployment_repo::find_deployment_by_id(db, user.tenant_id, deployment_id).await {
        Ok(Some(deployment)) => deployment,
        _ => {
            return Err(
                render_tracking_error(db, &user, deployments, None, "Deployment not found.").await,
            )
        }
    };
    let users = user_repo::list_users_by_tenant(db, user.tenant_id)
        .await
        .unwrap_or_default();

    match deployment_discussion_service::update_discussion(
        db,
        user.tenant_id,
        deployment_id,
        discussion_id,
        form,
    )
    .await
    {
        Ok(_) => Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Some(deployment_id)
        )))),
        Err(err) => Err(Template::render(
            "tracking/discussion_edit",
            context! {
                title: "Edit discussion",
                current_user: Some(CurrentUserView::from(&user)),
                workspace_brand: workspace_brand(db, user.tenant_id).await,
                deployment: deployment,
                users: users,
                discussion_id: discussion_id,
                error: err.message,
                form: err.form,
            },
        )),
    }
}

#[post("/<slug>/tracking/deployments/<deployment_id>/discussions/<discussion_id>/delete")]
pub async fn tracking_discussion_delete(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    deployment_id: i64,
    discussion_id: i64,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::can_delete(db, &user, "tracking").await {
        return Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = Option::<i64>::None
        ))));
    }

    if let Err(message) = deployment_discussion_service::delete_discussion(
        db,
        user.tenant_id,
        deployment_id,
        discussion_id,
    )
    .await
    .map_err(|err| format!("Unable to delete discussion: {err}"))
    {
        let deployments = if access_service::is_employee(&user.role) {
            let crew_ids =
                crew_member_repo::list_crew_ids_for_user(db, user.tenant_id, user.id, &user.email)
                    .await
                    .unwrap_or_default();
            deployment_service::list_deployments_for_select_for_crews(
                db,
                user.tenant_id,
                &crew_ids,
            )
            .await
            .unwrap_or_default()
        } else {
            deployment_service::list_deployments_for_select(db, user.tenant_id)
                .await
                .unwrap_or_default()
        };
        return Err(
            render_tracking_error(
                db,
                &user,
                deployments,
                Some(deployment_id),
                &message,
            )
            .await,
        );
    }

    Ok(Redirect::to(uri!(tracking(
        slug = user.tenant_slug,
        deployment_id = Some(deployment_id)
    ))))
}

async fn render_tracking_error(
    db: &Db,
    user: &crate::models::User,
    deployments: Vec<crate::models::DeploymentSelect>,
    selected_deployment: Option<i64>,
    message: &str,
) -> Template {
    let updates = match selected_deployment {
        Some(deployment_id) if access_service::is_employee(&user.role) => {
            tracking_service::list_updates_for_user(db, user.tenant_id, deployment_id, user.id)
                .await
                .unwrap_or_default()
        }
        Some(deployment_id) => tracking_service::list_updates(db, user.tenant_id, deployment_id)
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let chart_updates = updates
        .iter()
        .filter(|update| !update.is_placeholder)
        .cloned()
        .collect::<Vec<_>>();
    let chart_data = build_hours_chart(&chart_updates);
    let is_admin_viewer =
        access_service::is_owner(&user.role) || access_service::is_admin(&user.role);
    let is_owner = access_service::is_owner(&user.role);
    let missing_user_ids = if is_admin_viewer {
        if let Some(deployment_id) = selected_deployment {
            tracking_service::count_updates_missing_user_id(
                db,
                user.tenant_id,
                deployment_id,
            )
            .await
            .unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };
    let can_edit_updates = is_owner
        || updates
            .iter()
            .any(|update| update.is_placeholder && update.user_id == Some(user.id));
    let active_timer = tracking_service::active_timer(db, user.tenant_id, user.id)
        .await
        .ok()
        .flatten();
    let coverage_map = build_live_coverage_map(db, user).await;
    let coverage_map_json =
        serde_json::to_string(&coverage_map).unwrap_or_else(|_| "{}".to_string());
    let discussions = match selected_deployment {
        Some(deployment_id) if deployment_id > 0 => {
            deployment_discussion_service::list_discussions_by_deployment(
                db,
                user.tenant_id,
                deployment_id,
            )
            .await
            .unwrap_or_default()
        }
        _ => Vec::new(),
    };
    let can_edit_tracking = access_service::can_edit(db, &user, "tracking").await;
    let can_delete_tracking = access_service::can_delete(db, &user, "tracking").await;
    Template::render(
        "tracking/index",
        context! {
            title: "Tracking",
            current_user: Some(CurrentUserView::from(user)),
            current_user_id: user.id,
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            deployments: deployments,
            selected_deployment: selected_deployment,
            updates: updates,
            chart_data: chart_data,
            coverage_map_json: coverage_map_json,
            include_map: true,
            error: message.to_string(),
            is_employee: access_service::is_employee(&user.role),
            is_admin_viewer: is_admin_viewer,
            missing_user_ids: missing_user_ids,
            is_owner: is_owner,
            can_edit_updates: can_edit_updates,
            can_edit_tracking: can_edit_tracking,
            can_delete_tracking: can_delete_tracking,
            discussions: discussions,
            active_timer: active_timer,
            form: DeploymentUpdateFormView::new(
                selected_deployment.unwrap_or(0),
                "",
                "",
                "",
                "",
            ),
        },
    )
}

async fn build_live_coverage_map(db: &Db, user: &crate::models::User) -> serde_json::Value {
    let deployment_ids = deployment_repo::list_active_deployment_ids(db, user.tenant_id)
        .await
        .unwrap_or_default();
    let locations =
        deployment_repo::list_deployment_locations_by_ids(db, user.tenant_id, &deployment_ids)
            .await
            .unwrap_or_default();

    let mut active_by_client: HashMap<i64, i64> = HashMap::new();
    for row in locations {
        *active_by_client.entry(row.client_id).or_insert(0) += 1;
    }

    let clients = client_repo::list_clients(db, user.tenant_id)
        .await
        .unwrap_or_default();
    let mut points = Vec::new();
    let mut total_active = 0;
    for client in clients {
        let lat = client.latitude.parse::<f64>().ok();
        let lng = client.longitude.parse::<f64>().ok();
        if lat.is_none() || lng.is_none() {
            continue;
        }
        let active_count = *active_by_client.get(&client.id).unwrap_or(&0);
        if active_count > 0 {
            total_active += active_count;
        }
        points.push(serde_json::json!({
            "lat": lat.unwrap(),
            "lng": lng.unwrap(),
            "client_name": client.company_name,
            "active_count": active_count
        }));
    }

    serde_json::json!({
        "points": points,
        "total_active": total_active
    })
}

fn build_hours_chart(updates: &[crate::models::DeploymentUpdate]) -> serde_json::Value {
    if updates.is_empty() {
        return serde_json::json!({
            "points": [],
            "path": "",
            "area_path": "",
            "width": 720,
            "height": 240,
            "padding": 32,
            "max_hours": 0.0
        });
    }
    let max_hours = updates
        .iter()
        .map(|update| update.hours_worked)
        .fold(0.0, f64::max)
        .max(1.0);
    let width = 720.0;
    let height = 240.0;
    let padding = 32.0;
    let inner_width = width - padding * 2.0;
    let inner_height = height - padding * 2.0;
    let ordered = updates.iter().rev().collect::<Vec<_>>();
    let count = ordered.len().max(1) as f64;
    let step = if count > 1.0 {
        inner_width / (count - 1.0)
    } else {
        0.0
    };
    let mut points = Vec::new();
    for (index, update) in ordered.iter().enumerate() {
        let ratio = (update.hours_worked / max_hours).min(1.0);
        let x = padding + step * index as f64;
        let y = padding + (1.0 - ratio) * inner_height;
        points.push(serde_json::json!({
            "x": x,
            "y": y,
            "work_date": update.work_date,
            "hours_worked": update.hours_worked
        }));
    }

    let mut path = String::new();
    let mut area_path = String::new();
    if let Some(first) = points.first() {
        let first_x = first["x"].as_f64().unwrap_or(padding);
        let first_y = first["y"].as_f64().unwrap_or(height - padding);
        path.push_str(&format!("M {:.1} {:.1}", first_x, first_y));
        for point in points.iter().skip(1) {
            let x = point["x"].as_f64().unwrap_or(first_x);
            let y = point["y"].as_f64().unwrap_or(first_y);
            path.push_str(&format!(" L {:.1} {:.1}", x, y));
        }
        let baseline = height - padding;
        let last_x = points
            .last()
            .and_then(|point| point["x"].as_f64())
            .unwrap_or(first_x);
        area_path.push_str(&path);
        area_path.push_str(&format!(" L {:.1} {:.1}", last_x, baseline));
        area_path.push_str(&format!(" L {:.1} {:.1} Z", first_x, baseline));
    }

    serde_json::json!({
        "points": points,
        "path": path,
        "area_path": area_path,
        "width": width,
        "height": height,
        "padding": padding,
        "max_hours": max_hours
    })
}

#[get("/<slug>/settings?<tab>")]
pub async fn settings(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    tab: Option<String>,
) -> Result<Template, Redirect> {
    let user = workspace_user(cookies, db, slug).await?;
    if !access_service::can_view(db, &user, "settings").await {
        return Err(Redirect::to(uri!(dashboard(slug = user.tenant_slug))));
    }
    let workspace = workspace_service::find_workspace_by_id(db, user.tenant_id)
        .await
        .ok()
        .flatten();
    let email_form = workspace
        .as_ref()
        .map(workspace_service::workspace_email_settings_view)
        .unwrap_or_else(workspace_service::default_email_settings_view);
    let theme_form = workspace
        .as_ref()
        .map(workspace_service::workspace_theme_view)
        .unwrap_or_else(workspace_service::default_theme_view);
    let theme_options = workspace_service::theme_options();
    let workspace_brand = workspace_brand(db, user.tenant_id).await;
    let is_owner = access_service::is_owner(&user.role);
    let theme_locked = is_theme_locked(&user.plan_key);
    let requested_tab = tab.unwrap_or_else(|| "email".to_string());
    let active_tab = if !is_owner && (requested_tab == "users" || requested_tab == "theme") {
        "email".to_string()
    } else {
        requested_tab
    };
    let mut users_context = Vec::new();
    if is_owner {
        let users = user_repo::list_users_by_tenant(db, user.tenant_id)
            .await
            .unwrap_or_default();
        for user_entry in users {
            let permissions = access_service::list_permissions_for_user(
                db,
                user.tenant_id,
                user_entry.id,
                &user_entry.role,
            )
            .await
            .unwrap_or_default();
            let permission_rows = access_service::RESOURCES
                .iter()
                .map(|(key, label)| {
                    let match_perm = permissions.iter().find(|perm| perm.resource == *key);
                    context! {
                        key: *key,
                        label: *label,
                        can_view: match_perm.map(|perm| perm.can_view).unwrap_or(false),
                        can_edit: match_perm.map(|perm| perm.can_edit).unwrap_or(false),
                        can_delete: match_perm.map(|perm| perm.can_delete).unwrap_or(false),
                    }
                })
                .collect::<Vec<_>>();
            let is_owner_entry = user_entry.role.eq_ignore_ascii_case("Owner");
            let role_name = if user_entry.role.trim().is_empty() {
                "Owner".to_string()
            } else {
                user_entry.role.clone()
            };
            users_context.push(context! {
                id: user_entry.id,
                email: user_entry.email,
                role: role_name,
                is_owner: is_owner_entry,
                permissions: permission_rows,
            });
        }
    }
    Ok(Template::render(
        "placeholders/settings",
        context! {
            title: "Settings",
            current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand,
            error: Option::<String>::None,
            email_form: email_form,
            email_provider_options: workspace_service::email_provider_options(),
            theme_form: theme_form,
            theme_options: theme_options,
            font_options: workspace_service::font_options(),
            active_tab: active_tab,
            is_owner: is_owner,
            is_theme_locked: theme_locked,
            users: users_context,
            role_options: access_service::role_options(),
        },
    ))
}

#[get("/<slug>/email-log")]
pub async fn email_log(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
) -> Result<Template, Redirect> {
    let user = workspace_user(cookies, db, slug).await?;
    if !access_service::can_view(db, &user, "settings").await {
        return Err(Redirect::to(uri!(dashboard(slug = user.tenant_slug))));
    }

    let logs = email_repo::list_outbound_emails_for_tenant(db, user.tenant_id, 100)
        .await
        .unwrap_or_default();
    let status_counts = email_service::count_outbound_emails_by_status(db, user.tenant_id)
        .await
        .unwrap_or_default();
    let queued_total = status_counts
        .iter()
        .find(|(status, _)| status.eq_ignore_ascii_case("Queued"))
        .map(|(_, count)| *count)
        .unwrap_or(0);
    let sent_total = status_counts
        .iter()
        .find(|(status, _)| status.eq_ignore_ascii_case("Sent"))
        .map(|(_, count)| *count)
        .unwrap_or(0);
    let failed_total = status_counts
        .iter()
        .find(|(status, _)| status.eq_ignore_ascii_case("Failed"))
        .map(|(_, count)| *count)
        .unwrap_or(0);

    Ok(Template::render(
        "emails/log",
        context! {
            title: "Email log",
            current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            logs: logs,
            queued_total: queued_total,
            sent_total: sent_total,
            failed_total: failed_total,
        },
    ))
}

#[post("/<slug>/settings/email", data = "<form>")]
pub async fn settings_email_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    form: Form<WorkspaceEmailSettingsForm>,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::can_edit(db, &user, "settings").await {
        return Ok(Redirect::to(uri!(settings(
            slug = user.tenant_slug,
            tab = Option::<String>::None
        ))));
    }
    let form = form.into_inner();
    match workspace_service::update_email_settings(db, user.tenant_id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(settings(slug = user.tenant_slug, tab = Option::<String>::None)))),
        Err(err) => Err(Template::render(
            "placeholders/settings",
            context! {
                title: "Settings",
                current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: err.message,
                email_form: err.form,
                email_provider_options: workspace_service::email_provider_options(),
                theme_form: workspace_service::default_theme_view(),
                theme_options: workspace_service::theme_options(),
                font_options: workspace_service::font_options(),
                active_tab: "email",
                is_owner: access_service::is_owner(&user.role),
                is_theme_locked: is_theme_locked(&user.plan_key),
                users: Vec::<serde_json::Value>::new(),
                role_options: access_service::role_options(),
            },
        )),
    }
}

#[post("/<slug>/settings/theme", data = "<form>")]
pub async fn settings_theme_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    form: Form<WorkspaceThemeForm<'_>>,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if access_service::is_plan_expired(db, &user).await {
        return Ok(Redirect::to(uri!(plans(slug = user.tenant_slug))));
    }
    if !access_service::is_owner(&user.role) {
        return Ok(Redirect::to(uri!(settings(
            slug = user.tenant_slug,
            tab = Some("theme".to_string())
        ))));
    }

    let existing_theme_form = workspace_service::find_workspace_by_id(db, user.tenant_id)
        .await
        .ok()
        .flatten()
        .as_ref()
        .map(workspace_service::workspace_theme_view)
        .unwrap_or_else(workspace_service::default_theme_view);
    let theme_locked = is_theme_locked(&user.plan_key);
    if theme_locked {
        return Err(Template::render(
            "placeholders/settings",
            context! {
                title: "Settings",
                current_user: Some(CurrentUserView::from(&user)),
                workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: "Theme settings are available on Professional and Enterprise plans.".to_string(),
                email_form: workspace_service::default_email_settings_view(),
                email_provider_options: workspace_service::email_provider_options(),
                theme_form: existing_theme_form,
                theme_options: workspace_service::theme_options(),
                font_options: workspace_service::font_options(),
                active_tab: "theme",
                is_owner: true,
                is_theme_locked: theme_locked,
                users: Vec::<serde_json::Value>::new(),
                role_options: access_service::role_options(),
            },
        ));
    }

    let mut form = form.into_inner();
    let mut logo_path = None;
    if let Some(mut logo) = form.logo.take() {
        if logo.len() > 0 {
            let content_type = logo.content_type().map(|value| value.media_type());
            let extension = match content_type {
                Some(media_type) if media_type.top().eq("image") => match media_type.sub().as_str() {
                    "png" => Some("png"),
                    "jpeg" | "jpg" => Some("jpg"),
                    "svg+xml" => Some("svg"),
                    "webp" => Some("webp"),
                    _ => None,
                },
                _ => None,
            };

            let extension = match extension {
                Some(value) => value,
                None => {
                    return Err(Template::render(
                        "placeholders/settings",
                        context! {
                            title: "Settings",
                            current_user: Some(CurrentUserView::from(&user)),
                            workspace_brand: workspace_brand(db, user.tenant_id).await,
                            error: "Logo must be a PNG, JPG, SVG, or WebP image.".to_string(),
                            email_form: workspace_service::default_email_settings_view(),
                            email_provider_options: workspace_service::email_provider_options(),
                            theme_form: existing_theme_form,
                            theme_options: workspace_service::theme_options(),
                            font_options: workspace_service::font_options(),
                            active_tab: "theme",
                            is_owner: true,
                            is_theme_locked: theme_locked,
                            users: Vec::<serde_json::Value>::new(),
                            role_options: access_service::role_options(),
                        },
                    ));
                }
            };

            let upload_dir = Path::new("static/uploads");
            if let Err(err) = fs::create_dir_all(upload_dir) {
                return Err(Template::render(
                    "placeholders/settings",
                    context! {
                        title: "Settings",
                        current_user: Some(CurrentUserView::from(&user)),
                        workspace_brand: workspace_brand(db, user.tenant_id).await,
                        error: format!("Unable to create upload folder: {err}"),
                        email_form: workspace_service::default_email_settings_view(),
                        email_provider_options: workspace_service::email_provider_options(),
                        theme_form: existing_theme_form,
                        theme_options: workspace_service::theme_options(),
                        font_options: workspace_service::font_options(),
                        active_tab: "theme",
                        is_owner: true,
                        is_theme_locked: theme_locked,
                        users: Vec::<serde_json::Value>::new(),
                        role_options: access_service::role_options(),
                    },
                ));
            }

            let filename = format!(
                "tenant-{}-{}.{}",
                user.tenant_id,
                Utc::now().timestamp(),
                extension
            );
            let target_path = upload_dir.join(&filename);
            if let Err(err) = logo.persist_to(&target_path).await {
                return Err(Template::render(
                    "placeholders/settings",
                    context! {
                        title: "Settings",
                        current_user: Some(CurrentUserView::from(&user)),
                        workspace_brand: workspace_brand(db, user.tenant_id).await,
                        error: format!("Unable to save logo: {err}"),
                        email_form: workspace_service::default_email_settings_view(),
                        email_provider_options: workspace_service::email_provider_options(),
                        theme_form: existing_theme_form,
                        theme_options: workspace_service::theme_options(),
                        font_options: workspace_service::font_options(),
                        active_tab: "theme",
                        is_owner: true,
                        is_theme_locked: theme_locked,
                        users: Vec::<serde_json::Value>::new(),
                        role_options: access_service::role_options(),
                    },
                ));
            }

            logo_path = Some(format!("/static/uploads/{}", filename));
        }
    }

    match workspace_service::update_theme_settings(db, user.tenant_id, form, logo_path).await {
        Ok(_) => Ok(Redirect::to(uri!(settings(
            slug = user.tenant_slug,
            tab = Some("theme".to_string())
        )))),
        Err(err) => Err(Template::render(
            "placeholders/settings",
            context! {
                title: "Settings",
                current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: err.message,
                email_form: workspace_service::default_email_settings_view(),
                email_provider_options: workspace_service::email_provider_options(),
                theme_form: err.form,
                theme_options: workspace_service::theme_options(),
                font_options: workspace_service::font_options(),
                active_tab: "theme",
                is_owner: true,
                is_theme_locked: theme_locked,
                users: Vec::<serde_json::Value>::new(),
                role_options: access_service::role_options(),
            },
        )),
    }
}

#[post("/<slug>/settings/seed-demo")]
pub async fn settings_seed_demo(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::can_edit(db, &user, "settings").await {
        return Ok(Redirect::to(uri!(settings(
            slug = user.tenant_slug,
            tab = Option::<String>::None
        ))));
    }

    match workspace_service::seed_demo_data(db, user.tenant_id).await {
        Ok(_) => Ok(Redirect::to(uri!(settings(slug = user.tenant_slug, tab = Option::<String>::None)))),
        Err(message) => {
            let workspace = workspace_service::find_workspace_by_id(db, user.tenant_id)
                .await
                .ok()
                .flatten();
            let email_form = workspace
                .as_ref()
                .map(workspace_service::workspace_email_settings_view)
                .unwrap_or_else(workspace_service::default_email_settings_view);
            let theme_form = workspace
                .as_ref()
                .map(workspace_service::workspace_theme_view)
                .unwrap_or_else(workspace_service::default_theme_view);
            Err(Template::render(
                "placeholders/settings",
                context! {
                    title: "Settings",
                    current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    error: message,
                    email_form: email_form,
                    email_provider_options: workspace_service::email_provider_options(),
                    theme_form: theme_form,
                    theme_options: workspace_service::theme_options(),
                    font_options: workspace_service::font_options(),
                    active_tab: "email",
                    is_owner: access_service::is_owner(&user.role),
                    is_theme_locked: is_theme_locked(&user.plan_key),
                    users: Vec::<serde_json::Value>::new(),
                    role_options: access_service::role_options(),
                },
            ))
        }
    }
}

#[post("/<slug>/settings/users/<user_id>", data = "<form>")]
pub async fn settings_users_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    user_id: i64,
    form: Form<UserPermissionForm>,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if access_service::is_plan_expired(db, &user).await {
        return Ok(Redirect::to(uri!(plans(slug = user.tenant_slug))));
    }
    if !access_service::is_owner(&user.role) {
        return Ok(Redirect::to(uri!(settings(
            slug = user.tenant_slug,
            tab = Option::<String>::None
        ))));
    }
    let form = form.into_inner();
    let role = form.role.trim().to_string();
    let role = match access_service::role_options()
        .iter()
        .find(|option| option.eq_ignore_ascii_case(&role))
    {
        Some(value) => value.to_string(),
        None => {
        return Err(Template::render(
            "placeholders/settings",
            context! {
                title: "Settings",
                current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: "Invalid role selection.".to_string(),
                email_form: workspace_service::default_email_settings_view(),
                email_provider_options: workspace_service::email_provider_options(),
                theme_form: workspace_service::default_theme_view(),
                theme_options: workspace_service::theme_options(),
                font_options: workspace_service::font_options(),
                active_tab: "users",
                is_owner: true,
                is_theme_locked: is_theme_locked(&user.plan_key),
                users: Vec::<serde_json::Value>::new(),
                role_options: access_service::role_options(),
            },
        ));
        }
    };
    let selected = form.permissions.unwrap_or_default();
    let permissions = access_service::RESOURCES
        .iter()
        .map(|(key, _)| {
            let view_key = format!("{}:view", key);
            let edit_key = format!("{}:edit", key);
            let delete_key = format!("{}:delete", key);
            crate::models::UserPermission {
                resource: (*key).to_string(),
                can_view: selected.contains(&view_key),
                can_edit: selected.contains(&edit_key),
                can_delete: selected.contains(&delete_key),
            }
        })
        .collect::<Vec<_>>();

    if let Err(err) = access_service::refresh_user_role(db, user.tenant_id, user_id, &role).await {
        return Err(Template::render(
            "placeholders/settings",
            context! {
                title: "Settings",
                current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: format!("Unable to update user role: {err}"),
                email_form: workspace_service::default_email_settings_view(),
                email_provider_options: workspace_service::email_provider_options(),
                theme_form: workspace_service::default_theme_view(),
                theme_options: workspace_service::theme_options(),
                font_options: workspace_service::font_options(),
                active_tab: "users",
                is_owner: true,
                is_theme_locked: is_theme_locked(&user.plan_key),
                users: Vec::<serde_json::Value>::new(),
                role_options: access_service::role_options(),
            },
        ));
    }

    if let Err(err) = access_service::replace_permissions_for_user(
        db,
        user.tenant_id,
        user_id,
        &permissions,
    )
    .await
    {
        return Err(Template::render(
            "placeholders/settings",
            context! {
                title: "Settings",
                current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                error: format!("Unable to update permissions: {err}"),
                email_form: workspace_service::default_email_settings_view(),
                email_provider_options: workspace_service::email_provider_options(),
                theme_form: workspace_service::default_theme_view(),
                theme_options: workspace_service::theme_options(),
                font_options: workspace_service::font_options(),
                active_tab: "users",
                is_owner: true,
                is_theme_locked: is_theme_locked(&user.plan_key),
                users: Vec::<serde_json::Value>::new(),
                role_options: access_service::role_options(),
            },
        ));
    }

    Ok(Redirect::to(uri!(settings(
        slug = user.tenant_slug,
        tab = Some("users".to_string())
    ))))
}

#[get("/<slug>/deployments")]
pub async fn deployments(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
) -> Result<Template, Redirect> {
    let user = workspace_user(cookies, db, slug).await?;
    if !access_service::can_view(db, &user, "deployments").await {
        return Err(Redirect::to(uri!(dashboard(slug = user.tenant_slug))));
    }
    let groups = if access_service::is_employee(&user.role) {
        let crew_ids =
            crew_member_repo::list_crew_ids_for_user(db, user.tenant_id, user.id, &user.email)
            .await
            .unwrap_or_default();
        deployment_service::list_deployments_grouped_for_crews(db, user.tenant_id, &crew_ids)
            .await
            .unwrap_or_default()
    } else {
        deployment_service::list_deployments_grouped(db, user.tenant_id)
            .await
            .unwrap_or_default()
    };
    let (_plan_key, limits) = workspace_service::plan_limits_for_tenant(db, user.tenant_id).await;
    let deployment_limit = limits.deployments_per_client;
    let deployment_limit_reached = deployment_limit
        .map(|limit| {
            !groups.is_empty()
                && groups
                    .iter()
                    .all(|group| group.deployments.len() as i64 >= limit)
        })
        .unwrap_or(false);
    let deployment_ids = groups
        .iter()
        .flat_map(|group| group.deployments.iter().map(|deployment| deployment.id))
        .collect::<Vec<_>>();
    let issue_flags = deployment_update_repo::list_issue_flags_for_deployments(
        db,
        user.tenant_id,
        &deployment_ids,
    )
    .await
    .unwrap_or_default();
    let mut issue_map = HashMap::new();
    for (deployment_id, issue_count, resolved_count) in issue_flags {
        issue_map.insert(deployment_id, (issue_count, resolved_count));
    }
    let invoice_statuses = invoice_repo::list_invoice_statuses_for_deployments(
        db,
        user.tenant_id,
        &deployment_ids,
    )
    .await
    .unwrap_or_default();
    let invoice_map = invoice_statuses
        .into_iter()
        .collect::<HashMap<i64, String>>();
    let deployments = groups
        .into_iter()
        .map(|group| {
            let deployment_count = group.deployments.len() as i64;
            let items = group
                .deployments
                .into_iter()
                .map(|deployment| {
                    let calculated_fee = deployment_service::calculated_fee(
                        &deployment.start_at,
                        &deployment.end_at,
                        deployment.fee_per_hour,
                    );
                    let (issue_count, resolved_count) = issue_map
                        .get(&deployment.id)
                        .copied()
                        .unwrap_or((0, 0));
                    let invoice_status =
                        invoice_map.get(&deployment.id).map(|status| status.as_str());
                    let timeline = deployment_service::deployment_timeline(
                        &deployment.status,
                        &deployment.deployment_type,
                        issue_count,
                        resolved_count,
                        invoice_status,
                    );
                    context! {
                        id: deployment.id,
                        crew_id: deployment.crew_id,
                        crew_name: deployment.crew_name,
                        start_at: deployment.start_at,
                        end_at: deployment.end_at,
                        fee_per_hour: deployment.fee_per_hour,
                        calculated_fee: calculated_fee,
                        info: deployment.info,
                        status: deployment.status,
                        timeline: timeline,
                    }
                })
                .collect::<Vec<_>>();
            context! {
                client_id: group.client_id,
                client_name: group.client_name,
                client_currency: group.client_currency,
                limit_reached: deployment_limit
                    .map(|limit| deployment_count >= limit)
                    .unwrap_or(false),
                deployments: items,
            }
        })
        .collect::<Vec<_>>();
    Ok(Template::render(
        "deployments/index",
        context! {
            title: "Deployments",
            current_user: Some(CurrentUserView::from(&user)),
            current_user_id: user.id,
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            deployments: deployments,
            deployment_limit: deployment_limit.unwrap_or(0),
            deployment_limit_reached: deployment_limit_reached,
        },
    ))
}

#[get("/<slug>/deployments/new")]
pub async fn deployment_new_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
) -> Result<Template, Redirect> {
    let user = workspace_user(cookies, db, slug).await?;
    if !access_service::can_edit(db, &user, "deployments").await {
        return Err(Redirect::to(uri!(deployments(slug = user.tenant_slug))));
    }
    let clients = client_service::list_clients(db, user.tenant_id)
        .await
        .unwrap_or_default();
    let crews = crew_service::list_crews(db, user.tenant_id)
        .await
        .unwrap_or_default();
    let recommended_crews = recommended_crews_view(&crews, "", "");
    Ok(Template::render(
        "deployments/new",
        context! {
            title: "New deployment",
                current_user: Some(CurrentUserView::from(&user)),
                current_user_id: user.id,
                workspace_brand: workspace_brand(db, user.tenant_id).await,
            error: Option::<String>::None,
            form: DeploymentFormView::new(0, 0, "", "", 0.0, "", "Scheduled", "Onsite", "", ""),
            clients: clients,
            crews: crews,
            recommended_crews: recommended_crews,
            status_options: deployment_service::status_options(),
            deployment_type_options: deployment_service::deployment_type_options(),
        },
    ))
}

#[post("/<slug>/deployments", data = "<form>")]
pub async fn deployment_create(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    form: Form<DeploymentForm>,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::can_edit(db, &user, "deployments").await {
        return Ok(Redirect::to(uri!(deployments(slug = user.tenant_slug))));
    }
    let form = form.into_inner();
    let clients = client_service::list_clients(db, user.tenant_id)
        .await
        .unwrap_or_default();
    let crews = crew_service::list_crews(db, user.tenant_id)
        .await
        .unwrap_or_default();

    match deployment_service::create_deployment(db, user.tenant_id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(deployments(slug = user.tenant_slug)))),
        Err(err) => {
            let recommended_crews = recommended_crews_view(
                &crews,
                &err.form.required_skills,
                &err.form.compatibility_pref,
            );
            Err(Template::render(
                "deployments/new",
                context! {
                    title: "New deployment",
                        current_user: Some(CurrentUserView::from(&user)),
                        current_user_id: user.id,
                        workspace_brand: workspace_brand(db, user.tenant_id).await,
                    error: err.message,
                    form: err.form,
                    clients: clients,
                    crews: crews,
                    recommended_crews: recommended_crews,
                    status_options: deployment_service::status_options(),
                    deployment_type_options: deployment_service::deployment_type_options(),
                },
            ))
        }
    }
}

#[get("/<slug>/deployments/<id>/edit")]
pub async fn deployment_edit_form(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
) -> Result<Template, Redirect> {
    let user = workspace_user(cookies, db, slug).await?;
    if !access_service::can_edit(db, &user, "deployments").await {
        return Err(Redirect::to(uri!(deployments(slug = user.tenant_slug))));
    }
    let deployment = match deployment_service::find_deployment_by_id(db, user.tenant_id, id).await {
        Ok(Some(deployment)) => deployment,
        _ => {
            return Ok(Template::render(
                "deployments/index",
                context! {
                    title: "Deployments",
                    current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                    deployments: Vec::<DeploymentFormView>::new(),
                    error: "Deployment not found.".to_string(),
                },
            ))
        }
    };
    let clients = client_service::list_clients(db, user.tenant_id)
        .await
        .unwrap_or_default();
    let crews = crew_service::list_crews(db, user.tenant_id)
        .await
        .unwrap_or_default();
    let recommended_crews = recommended_crews_view(
        &crews,
        &deployment.required_skills,
        &deployment.compatibility_pref,
    );

    Ok(Template::render(
        "deployments/edit",
        context! {
            title: "Edit deployment",
            current_user: Some(CurrentUserView::from(&user)),
            current_user_id: user.id,
            workspace_brand: workspace_brand(db, user.tenant_id).await,
            error: Option::<String>::None,
            deployment_id: deployment.id,
            form: DeploymentFormView::new(
                deployment.client_id,
                deployment.crew_id,
                to_datetime_local(&deployment.start_at),
                to_datetime_local(&deployment.end_at),
                deployment.fee_per_hour,
                deployment.info,
                deployment.status,
                deployment.deployment_type,
                deployment.required_skills,
                deployment.compatibility_pref,
            ),
            clients: clients,
            crews: crews,
            recommended_crews: recommended_crews,
            status_options: deployment_service::status_options(),
            deployment_type_options: deployment_service::deployment_type_options(),
        },
    ))
}

#[post("/<slug>/deployments/<id>", data = "<form>")]
pub async fn deployment_update(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
    form: Form<DeploymentForm>,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::can_edit(db, &user, "deployments").await {
        return Ok(Redirect::to(uri!(deployments(slug = user.tenant_slug))));
    }
    let form = form.into_inner();
    let clients = client_service::list_clients(db, user.tenant_id)
        .await
        .unwrap_or_default();
    let crews = crew_service::list_crews(db, user.tenant_id)
        .await
        .unwrap_or_default();

    match deployment_service::update_deployment(db, user.tenant_id, id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(deployments(slug = user.tenant_slug)))),
        Err(err) => {
            let recommended_crews = recommended_crews_view(
                &crews,
                &err.form.required_skills,
                &err.form.compatibility_pref,
            );
            Err(Template::render(
                "deployments/edit",
                context! {
                    title: "Edit deployment",
                    current_user: Some(CurrentUserView::from(&user)),
                    current_user_id: user.id,
                    workspace_brand: workspace_brand(db, user.tenant_id).await,
                    error: err.message,
                    deployment_id: id,
                    form: err.form,
                    clients: clients,
                    crews: crews,
                    recommended_crews: recommended_crews,
                    status_options: deployment_service::status_options(),
                    deployment_type_options: deployment_service::deployment_type_options(),
                },
            ))
        }
    }
}

#[post("/<slug>/deployments/<id>/delete")]
pub async fn deployment_delete(
    cookies: &CookieJar<'_>,
    db: &Db,
    slug: &str,
    id: i64,
) -> Result<Redirect, Template> {
    let user = match workspace_user(cookies, db, slug).await {
        Ok(user) => user,
        Err(redirect) => return Ok(redirect),
    };
    if !access_service::can_delete(db, &user, "deployments").await {
        return Ok(Redirect::to(uri!(deployments(slug = user.tenant_slug))));
    }

    if let Err(message) = deployment_service::delete_deployment(db, user.tenant_id, id).await {
        let groups = deployment_service::list_deployments_grouped(db, user.tenant_id)
            .await
            .unwrap_or_default();
        let deployment_ids = groups
            .iter()
            .flat_map(|group| group.deployments.iter().map(|deployment| deployment.id))
            .collect::<Vec<_>>();
        let issue_flags = deployment_update_repo::list_issue_flags_for_deployments(
            db,
            user.tenant_id,
            &deployment_ids,
        )
        .await
        .unwrap_or_default();
        let mut issue_map = HashMap::new();
        for (deployment_id, issue_count, resolved_count) in issue_flags {
            issue_map.insert(deployment_id, (issue_count, resolved_count));
        }
        let invoice_statuses = invoice_repo::list_invoice_statuses_for_deployments(
            db,
            user.tenant_id,
            &deployment_ids,
        )
        .await
        .unwrap_or_default();
        let invoice_map = invoice_statuses
            .into_iter()
            .collect::<HashMap<i64, String>>();
        let deployments = groups
            .into_iter()
            .map(|group| {
                let items = group
                    .deployments
                    .into_iter()
                    .map(|deployment| {
                        let calculated_fee = deployment_service::calculated_fee(
                            &deployment.start_at,
                            &deployment.end_at,
                            deployment.fee_per_hour,
                        );
                        let (issue_count, resolved_count) = issue_map
                            .get(&deployment.id)
                            .copied()
                            .unwrap_or((0, 0));
                        let invoice_status =
                            invoice_map.get(&deployment.id).map(|status| status.as_str());
                        let timeline = deployment_service::deployment_timeline(
                            &deployment.status,
                            &deployment.deployment_type,
                            issue_count,
                            resolved_count,
                            invoice_status,
                        );
                        context! {
                            id: deployment.id,
                            crew_id: deployment.crew_id,
                            crew_name: deployment.crew_name,
                            start_at: deployment.start_at,
                            end_at: deployment.end_at,
                            fee_per_hour: deployment.fee_per_hour,
                            calculated_fee: calculated_fee,
                            info: deployment.info,
                            status: deployment.status,
                            timeline: timeline,
                        }
                    })
                    .collect::<Vec<_>>();
                context! {
                    client_id: group.client_id,
                    client_name: group.client_name,
                    client_currency: group.client_currency,
                    deployments: items,
                }
            })
            .collect::<Vec<_>>();
        return Err(Template::render(
            "deployments/index",
            context! {
                title: "Deployments",
                current_user: Some(CurrentUserView::from(&user)),
            workspace_brand: workspace_brand(db, user.tenant_id).await,
                deployments: deployments,
                error: message,
            },
        ));
    }

    Ok(Redirect::to(uri!(deployments(slug = user.tenant_slug))))
}



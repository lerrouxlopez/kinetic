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
    LoginForm,
    LoginView,
    RegisterForm,
    RegisterView,
    UserPermissionForm,
    WorkspaceRegisterForm,
    WorkspaceRegisterView,
    WorkspaceEmailSettingsForm,
};
use crate::repositories::{crew_member_repo, user_repo};
use crate::services::{
    access_service,
    auth_service,
    appointment_service,
    client_service,
    crew_service,
    deployment_service,
    email_service,
    tracking_service,
    workspace_service,
};
use crate::Db;

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

#[get("/")]
pub async fn index(cookies: &CookieJar<'_>, db: &Db) -> Result<Redirect, Template> {
    match current_user_from_cookies(cookies, db).await {
        Some(user) => Ok(Redirect::to(uri!(dashboard(slug = user.tenant_slug)))),
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
    let current_user = current_user_from_cookies(cookies, db)
        .await
        .map(|user| CurrentUserView::from(&user));
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
    let clients_total = if can_view_clients {
        client_service::count_clients(db, user.tenant_id)
            .await
            .unwrap_or(0)
    } else {
        0
    };
    let crew_ids = if can_view_deployments && access_service::is_employee(&user.role) {
        crew_member_repo::list_crew_ids_for_user(db, user.tenant_id, user.id, &user.email)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let deployments_total = if can_view_deployments {
        if access_service::is_employee(&user.role) {
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
        crew_service::count_crews(db, user.tenant_id).await.unwrap_or(0)
    } else {
        0
    };
    let deployment_status_counts = if can_view_deployments {
        if access_service::is_employee(&user.role) {
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
        appointment_service::count_appointments_total(db, user.tenant_id)
            .await
            .unwrap_or(0)
    } else {
        0
    };
    let appointment_status_counts = if can_view_clients {
        appointment_service::count_appointments_by_status(db, user.tenant_id)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let appointment_statuses = ["Scheduled", "On Going", "Cancelled"];
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
        email_service::count_outbound_emails(db, user.tenant_id)
            .await
            .unwrap_or(0)
    } else {
        0
    };
    let email_status_counts = if can_view_settings {
        email_service::count_outbound_emails_by_status(db, user.tenant_id)
            .await
            .unwrap_or_default()
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
    let crew_stats = if can_view_crew {
        let crews = crew_service::list_crews(db, user.tenant_id)
            .await
            .unwrap_or_default();
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
            tenant_slug: user.tenant_slug,
            email: user.email,
            can_view_clients: can_view_clients,
            can_view_crew: can_view_crew,
            can_view_deployments: can_view_deployments,
            can_view_tracking: can_view_tracking,
            can_view_invoices: can_view_invoices,
            can_view_settings: can_view_settings,
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
    let selected_deployment = deployment_id
        .filter(|id| allowed_ids.contains(id))
        .or_else(|| deployments.first().map(|item| item.id))
        .filter(|id| *id > 0);
    let updates = match selected_deployment {
        Some(deployment_id) => tracking_service::list_updates(db, user.tenant_id, deployment_id)
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let chart_data = build_hours_chart(&updates);
    Ok(Template::render(
        "tracking/index",
        context! {
            title: "Tracking",
            current_user: Some(CurrentUserView::from(&user)),
            deployments: deployments,
            selected_deployment: selected_deployment,
            updates: updates,
            chart_data: chart_data,
            error: Option::<String>::None,
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
                return Err(Template::render(
                    "tracking/index",
                    context! {
                        title: "Tracking",
                        current_user: Some(CurrentUserView::from(&user)),
                        deployments: deployments,
                        selected_deployment: Option::<i64>::None,
                        updates: Vec::<crate::models::DeploymentUpdate>::new(),
                        chart_data: build_hours_chart(&[]),
                        error: "You do not have access to that deployment.".to_string(),
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

    match tracking_service::create_update(db, user.tenant_id, form).await {
        Ok(_) => Ok(Redirect::to(uri!(tracking(
            slug = user.tenant_slug,
            deployment_id = selected_deployment
        )))),
        Err(err) => {
            let deployment_id = selected_deployment.unwrap_or(0);
            let updates = if deployment_id > 0 {
                tracking_service::list_updates(db, user.tenant_id, deployment_id)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            let chart_data = build_hours_chart(&updates);
            Err(Template::render(
                "tracking/index",
                context! {
                    title: "Tracking",
                    current_user: Some(CurrentUserView::from(&user)),
                    deployments: deployments,
                    selected_deployment: selected_deployment,
                    updates: updates,
                    chart_data: chart_data,
                    error: err.message,
                    form: err.form,
                },
            ))
        }
    }
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
    let is_owner = access_service::is_owner(&user.role);
    let requested_tab = tab.unwrap_or_else(|| "email".to_string());
    let active_tab = if !is_owner && requested_tab == "users" {
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
            error: Option::<String>::None,
            email_form: email_form,
            email_provider_options: workspace_service::email_provider_options(),
            active_tab: active_tab,
            is_owner: is_owner,
            users: users_context,
            role_options: access_service::role_options(),
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
                error: err.message,
                email_form: err.form,
                email_provider_options: workspace_service::email_provider_options(),
                active_tab: "email",
                is_owner: access_service::is_owner(&user.role),
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
            Err(Template::render(
                "placeholders/settings",
                context! {
                    title: "Settings",
                    current_user: Some(CurrentUserView::from(&user)),
                    error: message,
                    email_form: email_form,
                    email_provider_options: workspace_service::email_provider_options(),
                    active_tab: "email",
                    is_owner: access_service::is_owner(&user.role),
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
                error: "Invalid role selection.".to_string(),
                email_form: workspace_service::default_email_settings_view(),
                email_provider_options: workspace_service::email_provider_options(),
                active_tab: "users",
                is_owner: true,
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
                error: format!("Unable to update user role: {err}"),
                email_form: workspace_service::default_email_settings_view(),
                email_provider_options: workspace_service::email_provider_options(),
                active_tab: "users",
                is_owner: true,
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
                error: format!("Unable to update permissions: {err}"),
                email_form: workspace_service::default_email_settings_view(),
                email_provider_options: workspace_service::email_provider_options(),
                active_tab: "users",
                is_owner: true,
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
    Ok(Template::render(
        "deployments/index",
        context! {
            title: "Deployments",
            current_user: Some(CurrentUserView::from(&user)),
            deployments: deployments,
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
    Ok(Template::render(
        "deployments/new",
        context! {
            title: "New deployment",
            current_user: Some(CurrentUserView::from(&user)),
            error: Option::<String>::None,
            form: DeploymentFormView::new(0, 0, "", "", 0.0, "", "Scheduled"),
            clients: clients,
            crews: crews,
            status_options: deployment_service::status_options(),
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
        Err(err) => Err(Template::render(
            "deployments/new",
            context! {
                title: "New deployment",
                current_user: Some(CurrentUserView::from(&user)),
                error: err.message,
                form: err.form,
                clients: clients,
                crews: crews,
                status_options: deployment_service::status_options(),
            },
        )),
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

    Ok(Template::render(
        "deployments/edit",
        context! {
            title: "Edit deployment",
            current_user: Some(CurrentUserView::from(&user)),
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
            ),
            clients: clients,
            crews: crews,
            status_options: deployment_service::status_options(),
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
        Err(err) => Err(Template::render(
            "deployments/edit",
            context! {
                title: "Edit deployment",
                current_user: Some(CurrentUserView::from(&user)),
                error: err.message,
                deployment_id: id,
                form: err.form,
                clients: clients,
                crews: crews,
                status_options: deployment_service::status_options(),
            },
        )),
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
                deployments: deployments,
                error: message,
            },
        ));
    }

    Ok(Redirect::to(uri!(deployments(slug = user.tenant_slug))))
}

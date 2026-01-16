use rocket_db_pools::sqlx::{self, Row};
use serde::Serialize;

use crate::models::{
    ThemeOption,
    Workspace,
    WorkspaceBrandView,
    WorkspaceEmailSettingsForm,
    WorkspaceEmailSettingsView,
    WorkspaceFormView,
    WorkspaceThemeForm,
    WorkspaceThemeView,
};
use crate::repositories::tenant_repo;
use crate::services::utils::normalize_slug;
use crate::Db;
use chrono::{Duration, NaiveDateTime, Utc};

pub struct WorkspaceError {
    pub message: String,
    pub form: WorkspaceFormView,
}

pub struct WorkspaceEmailSettingsError {
    pub message: String,
    pub form: WorkspaceEmailSettingsView,
}

pub struct WorkspaceThemeError {
    pub message: String,
    pub form: WorkspaceThemeView,
}

#[derive(Serialize, Clone, Copy)]
pub struct PlanLimits {
    pub clients: Option<i64>,
    pub contacts_per_client: Option<i64>,
    pub appointments_per_client: Option<i64>,
    pub deployments_per_client: Option<i64>,
    pub crews: Option<i64>,
    pub members_per_crew: Option<i64>,
    pub users: Option<i64>,
    pub expires_after_days: Option<i64>,
}

#[derive(Serialize, Clone, Copy)]
pub struct PlanDefinition {
    pub key: &'static str,
    pub name: &'static str,
    pub price: &'static str,
    pub min_term_months: i64,
    pub whitelabel: bool,
    pub limits: PlanLimits,
    pub payment_url: &'static str,
    pub payment_qr_url: &'static str,
}

pub fn email_provider_options() -> Vec<&'static str> {
    vec![
        "Mailtrap",
        "Amazon SES (Simple Email Service)",
        "Mailgun",
        "Postmark",
        "Resend",
        "Sendmail",
        "SMTP",
    ]
}

pub fn theme_options() -> Vec<ThemeOption> {
    vec![
        ThemeOption {
            key: "kinetic".to_string(),
            name: "Kinetic Ember".to_string(),
            primary: "#f59e0b".to_string(),
            secondary: "#ef4444".to_string(),
            on_primary: "#111827".to_string(),
        },
        ThemeOption {
            key: "cobalt".to_string(),
            name: "Cobalt Tide".to_string(),
            primary: "#2563eb".to_string(),
            secondary: "#22d3ee".to_string(),
            on_primary: "#f8fafc".to_string(),
        },
        ThemeOption {
            key: "emerald".to_string(),
            name: "Emerald Pulse".to_string(),
            primary: "#10b981".to_string(),
            secondary: "#34d399".to_string(),
            on_primary: "#0f172a".to_string(),
        },
        ThemeOption {
            key: "forest".to_string(),
            name: "Forest Trail".to_string(),
            primary: "#15803d".to_string(),
            secondary: "#16a34a".to_string(),
            on_primary: "#f8fafc".to_string(),
        },
        ThemeOption {
            key: "ocean".to_string(),
            name: "Ocean Drift".to_string(),
            primary: "#0ea5e9".to_string(),
            secondary: "#38bdf8".to_string(),
            on_primary: "#0f172a".to_string(),
        },
        ThemeOption {
            key: "plum".to_string(),
            name: "Plum Neon".to_string(),
            primary: "#a855f7".to_string(),
            secondary: "#ec4899".to_string(),
            on_primary: "#f8fafc".to_string(),
        },
        ThemeOption {
            key: "canyon".to_string(),
            name: "Canyon Heat".to_string(),
            primary: "#b45309".to_string(),
            secondary: "#f59e0b".to_string(),
            on_primary: "#f8fafc".to_string(),
        },
        ThemeOption {
            key: "slate".to_string(),
            name: "Slate Signal".to_string(),
            primary: "#475569".to_string(),
            secondary: "#94a3b8".to_string(),
            on_primary: "#f8fafc".to_string(),
        },
        ThemeOption {
            key: "sunset".to_string(),
            name: "Sunset Drive".to_string(),
            primary: "#f97316".to_string(),
            secondary: "#f43f5e".to_string(),
            on_primary: "#111827".to_string(),
        },
        ThemeOption {
            key: "aurora".to_string(),
            name: "Aurora Mint".to_string(),
            primary: "#14b8a6".to_string(),
            secondary: "#22c55e".to_string(),
            on_primary: "#0f172a".to_string(),
        },
        ThemeOption {
            key: "midnight".to_string(),
            name: "Midnight Steel".to_string(),
            primary: "#0f172a".to_string(),
            secondary: "#64748b".to_string(),
            on_primary: "#f8fafc".to_string(),
        },
    ]
}

pub fn font_options() -> Vec<&'static str> {
    vec![
        "Space Grotesk",
        "Sora",
        "DM Sans",
        "IBM Plex Sans",
        "Work Sans",
        "Space Mono",
    ]
}

pub fn plan_definitions() -> Vec<PlanDefinition> {
    vec![
        PlanDefinition {
            key: "free",
            name: "Free",
            price: "Free",
            min_term_months: 1,
            whitelabel: false,
            limits: PlanLimits {
                clients: Some(5),
                contacts_per_client: Some(5),
                appointments_per_client: Some(20),
                deployments_per_client: Some(1),
                crews: Some(2),
                members_per_crew: Some(5),
                users: Some(11),
                expires_after_days: Some(30),
            },
            payment_url: "https://wise.com/pay/kinetic/free",
            payment_qr_url: "/static/qr/wise-free.png",
        },
        PlanDefinition {
            key: "pro",
            name: "Professional",
            price: "$19.99 / month",
            min_term_months: 6,
            whitelabel: true,
            limits: PlanLimits {
                clients: Some(20),
                contacts_per_client: Some(5),
                appointments_per_client: Some(40),
                deployments_per_client: Some(5),
                crews: Some(5),
                members_per_crew: Some(10),
                users: Some(51),
                expires_after_days: None,
            },
            payment_url: "https://wise.com/pay/kinetic/pro",
            payment_qr_url: "/static/qr/wise-pro.png",
        },
        PlanDefinition {
            key: "enterprise",
            name: "Enterprise",
            price: "$49.99 / month",
            min_term_months: 12,
            whitelabel: true,
            limits: PlanLimits {
                clients: None,
                contacts_per_client: None,
                appointments_per_client: None,
                deployments_per_client: None,
                crews: None,
                members_per_crew: None,
                users: None,
                expires_after_days: None,
            },
            payment_url: "https://wise.com/pay/kinetic/enterprise",
            payment_qr_url: "/static/qr/wise-enterprise.png",
        },
    ]
}

pub fn find_plan(key: &str) -> Option<PlanDefinition> {
    plan_definitions()
        .into_iter()
        .find(|plan| plan.key.eq_ignore_ascii_case(key))
}

pub fn plan_name(plan_key: &str) -> &'static str {
    find_plan(plan_key)
        .map(|plan| plan.name)
        .unwrap_or("Current")
}

fn default_plan_limits(plan_key: &str) -> PlanLimits {
    find_plan(plan_key)
        .map(|plan| plan.limits)
        .unwrap_or(PlanLimits {
            clients: None,
            contacts_per_client: None,
            appointments_per_client: None,
            deployments_per_client: None,
            crews: None,
            members_per_crew: None,
            users: None,
            expires_after_days: None,
        })
}

pub async fn plan_limits_for(db: &Db, plan_key: &str) -> PlanLimits {
    let row = sqlx::query(
        r#"
        SELECT clients,
               contacts_per_client,
               appointments_per_client,
               deployments_per_client,
               crews,
               members_per_crew,
               users,
               expires_after_days
        FROM plan_limits
        WHERE plan_key = ?
        "#,
    )
    .bind(plan_key)
    .fetch_optional(&db.0)
    .await;

    match row {
        Ok(Some(row)) if plan_key.eq_ignore_ascii_case("enterprise") => PlanLimits {
            clients: None,
            contacts_per_client: None,
            appointments_per_client: None,
            deployments_per_client: None,
            crews: None,
            members_per_crew: None,
            users: None,
            expires_after_days: Some(row.get("expires_after_days")),
        },
        Ok(Some(row)) => PlanLimits {
            clients: Some(row.get("clients")),
            contacts_per_client: Some(row.get("contacts_per_client")),
            appointments_per_client: Some(row.get("appointments_per_client")),
            deployments_per_client: Some(row.get("deployments_per_client")),
            crews: Some(row.get("crews")),
            members_per_crew: Some(row.get("members_per_crew")),
            users: Some(row.get("users")),
            expires_after_days: Some(row.get("expires_after_days")),
        },
        _ => default_plan_limits(plan_key),
    }
}

pub async fn free_plan_limits(db: &Db) -> PlanLimits {
    plan_limits_for(db, "free").await
}

pub async fn pro_plan_limits(db: &Db) -> PlanLimits {
    plan_limits_for(db, "pro").await
}

pub async fn enterprise_plan_limits(db: &Db) -> PlanLimits {
    plan_limits_for(db, "enterprise").await
}

pub async fn plan_limits_for_tenant(db: &Db, tenant_id: i64) -> (String, PlanLimits) {
    let plan_key = find_workspace_by_id(db, tenant_id)
        .await
        .ok()
        .flatten()
        .map(|workspace| workspace.plan_key)
        .unwrap_or_else(|| "free".to_string());
    let limits = plan_limits_for(db, &plan_key).await;
    (plan_key, limits)
}

pub async fn update_plan_limits(
    db: &Db,
    plan_key: &str,
    limits: &crate::models::PlanLimitsForm,
) -> Result<(), String> {
    if limits.clients <= 0
        || limits.contacts_per_client <= 0
        || limits.appointments_per_client <= 0
        || limits.deployments_per_client <= 0
        || limits.crews <= 0
        || limits.members_per_crew <= 0
        || limits.users <= 0
        || limits.expires_after_days < 0
    {
        return Err("All limit values must be greater than 0.".to_string());
    }

    sqlx::query(
        r#"
        INSERT INTO plan_limits (
            plan_key,
            clients,
            contacts_per_client,
            appointments_per_client,
            deployments_per_client,
            crews,
            members_per_crew,
            users,
            expires_after_days
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(plan_key) DO UPDATE SET
            clients = excluded.clients,
            contacts_per_client = excluded.contacts_per_client,
            appointments_per_client = excluded.appointments_per_client,
            deployments_per_client = excluded.deployments_per_client,
            crews = excluded.crews,
            members_per_crew = excluded.members_per_crew,
            users = excluded.users,
            expires_after_days = excluded.expires_after_days
        "#,
    )
    .bind(plan_key)
    .bind(limits.clients)
    .bind(limits.contacts_per_client)
    .bind(limits.appointments_per_client)
    .bind(limits.deployments_per_client)
    .bind(limits.crews)
    .bind(limits.members_per_crew)
    .bind(limits.users)
    .bind(limits.expires_after_days)
    .execute(&db.0)
    .await
    .map_err(|err| format!("Unable to update {plan_key} plan limits: {err}"))?;

    Ok(())
}

pub async fn update_free_plan_limits(
    db: &Db,
    limits: &crate::models::PlanLimitsForm,
) -> Result<(), String> {
    if !matches!(limits.expires_after_days, 0 | 30 | 60 | 90) {
        return Err("Free plan expiry must be 0, 30, 60, or 90 days.".to_string());
    }

    update_plan_limits(db, "free", limits).await
}

pub async fn update_pro_plan_limits(
    db: &Db,
    limits: &crate::models::PlanLimitsForm,
) -> Result<(), String> {
    if !matches!(limits.expires_after_days, 180 | 210 | 240) {
        return Err("Professional plan expiry must be 6, 7, or 8 months.".to_string());
    }
    update_plan_limits(db, "pro", limits).await
}

pub async fn update_enterprise_plan_expiry(
    db: &Db,
    expires_after_days: i64,
) -> Result<(), String> {
    if !matches!(expires_after_days, 365 | 395 | 425) {
        return Err("Enterprise plan expiry must be 1 year, 1 year + 1 month, or 1 year + 2 months.".to_string());
    }

    sqlx::query(
        r#"
        INSERT INTO plan_limits (
            plan_key,
            clients,
            contacts_per_client,
            appointments_per_client,
            deployments_per_client,
            crews,
            members_per_crew,
            users,
            expires_after_days
        ) VALUES (?, 0, 0, 0, 0, 0, 0, 0, ?)
        ON CONFLICT(plan_key) DO UPDATE SET
            expires_after_days = excluded.expires_after_days
        "#,
    )
    .bind("enterprise")
    .bind(expires_after_days)
    .execute(&db.0)
    .await
    .map_err(|err| format!("Unable to update enterprise plan expiry: {err}"))?;

    Ok(())
}

pub async fn is_free_plan(db: &Db, tenant_id: i64) -> bool {
    find_workspace_by_id(db, tenant_id)
        .await
        .ok()
        .flatten()
        .map(|workspace| workspace.plan_key.eq_ignore_ascii_case("free"))
        .unwrap_or(false)
}

pub fn upgrade_options(current_plan: &str) -> Vec<PlanDefinition> {
    let order = ["free", "pro", "enterprise"];
    let current_index = order
        .iter()
        .position(|key| key.eq_ignore_ascii_case(current_plan));
    let plans = plan_definitions();
    match current_index {
        Some(index) => plans
            .into_iter()
            .filter(|plan| {
                order
                    .iter()
                    .position(|key| key.eq_ignore_ascii_case(&plan.key))
                    .map(|plan_index| plan_index > index)
                    .unwrap_or(false)
            })
            .collect(),
        None => plans,
    }
}

pub fn is_whitelabel_enabled(plan_key: &str) -> bool {
    find_plan(plan_key).map(|plan| plan.whitelabel).unwrap_or(false)
}

pub fn is_plan_expired(plan_key: &str, plan_started_at: &str, expires_after_days: i64) -> bool {
    if !plan_key.eq_ignore_ascii_case("free") {
        return false;
    }
    if expires_after_days <= 0 {
        return false;
    }
    let parsed = NaiveDateTime::parse_from_str(plan_started_at, "%Y-%m-%d %H:%M:%S");
    let started_at = match parsed {
        Ok(value) => value,
        Err(_) => return false,
    };
    let expires_at = started_at + Duration::days(expires_after_days);
    Utc::now().naive_utc() > expires_at
}

pub async fn is_workspace_plan_expired(db: &Db, tenant_id: i64) -> bool {
    let workspace = match find_workspace_by_id(db, tenant_id).await {
        Ok(Some(workspace)) => workspace,
        _ => return false,
    };
    if workspace.plan_expired {
        return true;
    }
    let limits = plan_limits_for(db, "free").await;
    let expires_after_days = limits.expires_after_days.unwrap_or(0);
    is_plan_expired(
        &workspace.plan_key,
        &workspace.plan_started_at,
        expires_after_days,
    )
}

pub async fn list_workspaces(db: &Db) -> Result<Vec<Workspace>, sqlx::Error> {
    tenant_repo::list_workspaces(db).await
}

pub async fn list_workspaces_paged(
    db: &Db,
    limit: i64,
    offset: i64,
) -> Result<Vec<Workspace>, sqlx::Error> {
    tenant_repo::list_workspaces_paged(db, limit, offset).await
}

pub async fn count_workspaces(db: &Db) -> Result<i64, sqlx::Error> {
    tenant_repo::count_workspaces(db).await
}

pub async fn find_workspace_by_id(
    db: &Db,
    id: i64,
) -> Result<Option<Workspace>, sqlx::Error> {
    tenant_repo::find_workspace_by_id(db, id).await
}

pub async fn create_workspace(
    db: &Db,
    slug_input: String,
    name_input: String,
    _plan_key: String,
) -> Result<(), WorkspaceError> {
    let plan_key = "free".to_string();
    let slug = match normalize_slug(&slug_input) {
        Some(slug) => slug,
        None => {
            return Err(WorkspaceError {
                message: "Slug must be lowercase letters, numbers, or dashes.".to_string(),
                form: WorkspaceFormView::new(slug_input, name_input, plan_key),
            })
        }
    };

    if name_input.trim().is_empty() {
        return Err(WorkspaceError {
            message: "Workspace name is required.".to_string(),
            form: WorkspaceFormView::new(slug, name_input, plan_key.clone()),
        });
    }

    if find_plan(&plan_key).is_none() {
        return Err(WorkspaceError {
            message: "Plan selection is invalid.".to_string(),
            form: WorkspaceFormView::new(slug, name_input, plan_key.clone()),
        });
    }

    if let Err(err) = tenant_repo::create_tenant(db, &slug, name_input.trim(), &plan_key).await {
        return Err(WorkspaceError {
            message: format!("Unable to create workspace: {err}"),
            form: WorkspaceFormView::new(slug, name_input, plan_key.clone()),
        });
    }

    Ok(())
}

pub async fn update_workspace(
    db: &Db,
    id: i64,
    slug_input: String,
    name_input: String,
    plan_key: String,
) -> Result<(), WorkspaceError> {
    let existing = find_workspace_by_id(db, id).await.ok().flatten();
    let slug = match normalize_slug(&slug_input) {
        Some(slug) => slug,
        None => {
            return Err(WorkspaceError {
                message: "Slug must be lowercase letters, numbers, or dashes.".to_string(),
                form: WorkspaceFormView::new(slug_input, name_input, plan_key),
            })
        }
    };

    if name_input.trim().is_empty() {
        return Err(WorkspaceError {
            message: "Workspace name is required.".to_string(),
            form: WorkspaceFormView::new(slug, name_input, plan_key.clone()),
        });
    }

    if find_plan(&plan_key).is_none() {
        return Err(WorkspaceError {
            message: "Plan selection is invalid.".to_string(),
            form: WorkspaceFormView::new(slug, name_input, plan_key.clone()),
        });
    }

    if let Err(err) =
        tenant_repo::update_workspace(db, id, &slug, name_input.trim(), &plan_key).await
    {
        return Err(WorkspaceError {
            message: format!("Unable to update workspace: {err}"),
            form: WorkspaceFormView::new(slug, name_input, plan_key.clone()),
        });
    }
    if let Some(existing) = existing {
        if !existing.plan_key.eq_ignore_ascii_case(&plan_key) {
            let _ = tenant_repo::set_workspace_plan_expired(db, id, false).await;
        }
    }

    Ok(())
}

pub async fn expire_workspace_plan(db: &Db, id: i64) -> Result<(), String> {
    tenant_repo::set_workspace_plan_expired(db, id, true)
        .await
        .map_err(|err| format!("Unable to expire workspace plan: {err}"))?;
    Ok(())
}

pub async fn update_email_settings(
    db: &Db,
    id: i64,
    form: WorkspaceEmailSettingsForm,
) -> Result<(), WorkspaceEmailSettingsError> {
    let email_provider = form.email_provider.trim().to_string();
    if email_provider.is_empty() {
        return Err(WorkspaceEmailSettingsError {
            message: "Email provider is required.".to_string(),
            form: WorkspaceEmailSettingsView::new(
                "Mailtrap", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
            ),
        });
    }
    if !email_provider_options()
        .iter()
        .any(|option| option.eq(&email_provider))
    {
        return Err(WorkspaceEmailSettingsError {
            message: "Email provider is not supported.".to_string(),
            form: WorkspaceEmailSettingsView::new(
                email_provider, "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
            ),
        });
    }

    let from_address = form.from_address.trim().to_string();
    if from_address.is_empty() {
        return Err(WorkspaceEmailSettingsError {
            message: "From address is required.".to_string(),
            form: WorkspaceEmailSettingsView::new(
                email_provider,
                form.from_name,
                "",
                form.smtp_host.clone().unwrap_or_default(),
                form.smtp_port.clone().unwrap_or_default(),
                form.smtp_username.clone().unwrap_or_default(),
                form.smtp_password.clone().unwrap_or_default(),
                form.smtp_encryption.clone().unwrap_or_default(),
                form.mailgun_domain.clone().unwrap_or_default(),
                form.mailgun_api_key.clone().unwrap_or_default(),
                form.postmark_server_token.clone().unwrap_or_default(),
                form.resend_api_key.clone().unwrap_or_default(),
                form.ses_access_key.clone().unwrap_or_default(),
                form.ses_secret_key.clone().unwrap_or_default(),
                form.ses_region.clone().unwrap_or_default(),
                form.sendmail_path.clone().unwrap_or_default(),
            ),
        });
    }

    let smtp_host = form.smtp_host.unwrap_or_default();
    let smtp_port = form.smtp_port.unwrap_or_default();
    let smtp_username = form.smtp_username.unwrap_or_default();
    let smtp_password = form.smtp_password.unwrap_or_default();
    let smtp_encryption = form.smtp_encryption.unwrap_or_default();
    let mailgun_domain = form.mailgun_domain.unwrap_or_default();
    let mailgun_api_key = form.mailgun_api_key.unwrap_or_default();
    let postmark_server_token = form.postmark_server_token.unwrap_or_default();
    let resend_api_key = form.resend_api_key.unwrap_or_default();
    let ses_access_key = form.ses_access_key.unwrap_or_default();
    let ses_secret_key = form.ses_secret_key.unwrap_or_default();
    let ses_region = form.ses_region.unwrap_or_default();
    let sendmail_path = form.sendmail_path.unwrap_or_default();

    let required = match email_provider.as_str() {
        "Mailtrap" | "SMTP" => {
            let mut missing = Vec::new();
            if smtp_host.trim().is_empty() {
                missing.push("SMTP host");
            }
            if smtp_port.trim().is_empty() {
                missing.push("SMTP port");
            }
            if smtp_username.trim().is_empty() {
                missing.push("SMTP username");
            }
            if smtp_password.trim().is_empty() {
                missing.push("SMTP password");
            }
            if !missing.is_empty() {
                return Err(WorkspaceEmailSettingsError {
                    message: format!("Missing required fields: {}.", missing.join(", ")),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider,
                        form.from_name,
                        from_address,
                        smtp_host,
                        smtp_port,
                        smtp_username,
                        smtp_password,
                        smtp_encryption,
                        mailgun_domain,
                        mailgun_api_key,
                        postmark_server_token,
                        resend_api_key,
                        ses_access_key,
                        ses_secret_key,
                        ses_region,
                        sendmail_path,
                    ),
                });
            }
            true
        }
        "Amazon SES (Simple Email Service)" => {
            if ses_access_key.trim().is_empty()
                || ses_secret_key.trim().is_empty()
                || ses_region.trim().is_empty()
            {
                return Err(WorkspaceEmailSettingsError {
                    message: "SES access key, secret key, and region are required.".to_string(),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider,
                        form.from_name,
                        from_address,
                        smtp_host,
                        smtp_port,
                        smtp_username,
                        smtp_password,
                        smtp_encryption,
                        mailgun_domain,
                        mailgun_api_key,
                        postmark_server_token,
                        resend_api_key,
                        ses_access_key,
                        ses_secret_key,
                        ses_region,
                        sendmail_path,
                    ),
                });
            }
            true
        }
        "Mailgun" => {
            if mailgun_domain.trim().is_empty() || mailgun_api_key.trim().is_empty() {
                return Err(WorkspaceEmailSettingsError {
                    message: "Mailgun domain and API key are required.".to_string(),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider,
                        form.from_name,
                        from_address,
                        smtp_host,
                        smtp_port,
                        smtp_username,
                        smtp_password,
                        smtp_encryption,
                        mailgun_domain,
                        mailgun_api_key,
                        postmark_server_token,
                        resend_api_key,
                        ses_access_key,
                        ses_secret_key,
                        ses_region,
                        sendmail_path,
                    ),
                });
            }
            true
        }
        "Postmark" => {
            if postmark_server_token.trim().is_empty() {
                return Err(WorkspaceEmailSettingsError {
                    message: "Postmark server token is required.".to_string(),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider,
                        form.from_name,
                        from_address,
                        smtp_host,
                        smtp_port,
                        smtp_username,
                        smtp_password,
                        smtp_encryption,
                        mailgun_domain,
                        mailgun_api_key,
                        postmark_server_token,
                        resend_api_key,
                        ses_access_key,
                        ses_secret_key,
                        ses_region,
                        sendmail_path,
                    ),
                });
            }
            true
        }
        "Resend" => {
            if resend_api_key.trim().is_empty() {
                return Err(WorkspaceEmailSettingsError {
                    message: "Resend API key is required.".to_string(),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider,
                        form.from_name,
                        from_address,
                        smtp_host,
                        smtp_port,
                        smtp_username,
                        smtp_password,
                        smtp_encryption,
                        mailgun_domain,
                        mailgun_api_key,
                        postmark_server_token,
                        resend_api_key,
                        ses_access_key,
                        ses_secret_key,
                        ses_region,
                        sendmail_path,
                    ),
                });
            }
            true
        }
        "Sendmail" => {
            if sendmail_path.trim().is_empty() {
                return Err(WorkspaceEmailSettingsError {
                    message: "Sendmail path is required.".to_string(),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider,
                        form.from_name,
                        from_address,
                        smtp_host,
                        smtp_port,
                        smtp_username,
                        smtp_password,
                        smtp_encryption,
                        mailgun_domain,
                        mailgun_api_key,
                        postmark_server_token,
                        resend_api_key,
                        ses_access_key,
                        ses_secret_key,
                        ses_region,
                        sendmail_path,
                    ),
                });
            }
            true
        }
        _ => false,
    };

    if !required {
        return Err(WorkspaceEmailSettingsError {
            message: "Email provider is not supported.".to_string(),
            form: WorkspaceEmailSettingsView::new(
                email_provider,
                form.from_name,
                from_address,
                smtp_host,
                smtp_port,
                smtp_username,
                smtp_password,
                smtp_encryption,
                mailgun_domain,
                mailgun_api_key,
                postmark_server_token,
                resend_api_key,
                ses_access_key,
                ses_secret_key,
                ses_region,
                sendmail_path,
            ),
        });
    }

    if let Err(err) = tenant_repo::update_email_settings(
        db,
        id,
        &email_provider,
        form.from_name.trim(),
        &from_address,
        smtp_host.trim(),
        smtp_port.trim(),
        smtp_username.trim(),
        smtp_password.trim(),
        smtp_encryption.trim(),
        mailgun_domain.trim(),
        mailgun_api_key.trim(),
        postmark_server_token.trim(),
        resend_api_key.trim(),
        ses_access_key.trim(),
        ses_secret_key.trim(),
        ses_region.trim(),
        sendmail_path.trim(),
    )
    .await
    {
        return Err(WorkspaceEmailSettingsError {
            message: format!("Unable to update email settings: {err}"),
            form: WorkspaceEmailSettingsView::new(
                email_provider,
                form.from_name,
                from_address,
                smtp_host,
                smtp_port,
                smtp_username,
                smtp_password,
                smtp_encryption,
                mailgun_domain,
                mailgun_api_key,
                postmark_server_token,
                resend_api_key,
                ses_access_key,
                ses_secret_key,
                ses_region,
                sendmail_path,
            ),
        });
    }

    Ok(())
}

pub async fn update_theme_settings(
    db: &Db,
    id: i64,
    form: WorkspaceThemeForm<'_>,
    logo_path: Option<String>,
) -> Result<(), WorkspaceThemeError> {
    let existing = find_workspace_by_id(db, id)
        .await
        .ok()
        .flatten()
        .as_ref()
        .map(workspace_theme_view)
        .unwrap_or_else(default_theme_view);
    let app_name = form
        .app_name
        .unwrap_or_default()
        .trim()
        .to_string();
    let app_name = if app_name.is_empty() {
        existing.app_name.clone()
    } else {
        app_name
    };

    let theme_key = form
        .theme_key
        .unwrap_or_default()
        .trim()
        .to_string();
    let theme_key = if theme_key.is_empty() {
        existing.theme_key.clone()
    } else {
        theme_key
    };
    if !theme_options()
        .iter()
        .any(|option| option.key == theme_key)
    {
        return Err(WorkspaceThemeError {
            message: "Theme selection is invalid.".to_string(),
            form: WorkspaceThemeView {
                app_name,
                logo_url: logo_path.unwrap_or(existing.logo_url),
                theme_key,
                background_hue: existing.background_hue,
                body_font: existing.body_font.clone(),
                heading_font: existing.heading_font.clone(),
            },
        });
    }

    let background_hue = form
        .background_hue
        .unwrap_or(existing.background_hue)
        .clamp(0, 360);
    let body_font = form
        .body_font
        .unwrap_or_else(|| existing.body_font.clone())
        .trim()
        .to_string();
    let body_font = if body_font.is_empty() {
        existing.body_font.clone()
    } else {
        body_font
    };
    let heading_font = form
        .heading_font
        .unwrap_or_else(|| existing.heading_font.clone())
        .trim()
        .to_string();
    let heading_font = if heading_font.is_empty() {
        existing.heading_font.clone()
    } else {
        heading_font
    };
    if !font_options().iter().any(|option| option.eq(&body_font))
        || !font_options().iter().any(|option| option.eq(&heading_font))
    {
        return Err(WorkspaceThemeError {
            message: "Font selection is invalid.".to_string(),
            form: WorkspaceThemeView {
                app_name,
                logo_url: logo_path.unwrap_or(existing.logo_url),
                theme_key,
                background_hue,
                body_font,
                heading_font,
            },
        });
    }
    let current_logo = find_workspace_by_id(db, id)
        .await
        .ok()
        .flatten()
        .map(|workspace| workspace.logo_path)
        .unwrap_or_default();
    let resolved_logo = logo_path.unwrap_or(current_logo);

    if let Err(err) = tenant_repo::update_theme_settings(
        db,
        id,
        &app_name,
        &theme_key,
        background_hue,
        &body_font,
        &heading_font,
        &resolved_logo,
    )
    .await
    {
        return Err(WorkspaceThemeError {
            message: format!("Unable to update theme: {err}"),
            form: WorkspaceThemeView {
                app_name,
                logo_url: resolved_logo,
                theme_key,
                background_hue,
                body_font,
                heading_font,
            },
        });
    }

    Ok(())
}

pub fn workspace_email_settings_view(workspace: &Workspace) -> WorkspaceEmailSettingsView {
    WorkspaceEmailSettingsView::new(
        workspace.email_provider.clone(),
        workspace.email_from_name.clone(),
        workspace.email_from_address.clone(),
        workspace.smtp_host.clone(),
        workspace.smtp_port.clone(),
        workspace.smtp_username.clone(),
        workspace.smtp_password.clone(),
        workspace.smtp_encryption.clone(),
        workspace.mailgun_domain.clone(),
        workspace.mailgun_api_key.clone(),
        workspace.postmark_server_token.clone(),
        workspace.resend_api_key.clone(),
        workspace.ses_access_key.clone(),
        workspace.ses_secret_key.clone(),
        workspace.ses_region.clone(),
        workspace.sendmail_path.clone(),
    )
}

pub fn default_email_settings_view() -> WorkspaceEmailSettingsView {
    WorkspaceEmailSettingsView::new(
        "Mailtrap", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
    )
}

pub fn workspace_theme_view(workspace: &Workspace) -> WorkspaceThemeView {
    WorkspaceThemeView {
        app_name: workspace.app_name.clone(),
        logo_url: logo_url_from_path(&workspace.logo_path),
        theme_key: workspace.theme_key.clone(),
        background_hue: workspace.background_hue,
        body_font: workspace.body_font.clone(),
        heading_font: workspace.heading_font.clone(),
    }
}

pub fn default_theme_view() -> WorkspaceThemeView {
    WorkspaceThemeView {
        app_name: "Kinetic".to_string(),
        logo_url: "/static/logo.png".to_string(),
        theme_key: "kinetic".to_string(),
        background_hue: 32,
        body_font: "Space Grotesk".to_string(),
        heading_font: "Space Grotesk".to_string(),
    }
}

pub fn workspace_brand_view(workspace: &Workspace) -> WorkspaceBrandView {
    let (primary, secondary, on_primary) = theme_palette(&workspace.theme_key);
    WorkspaceBrandView {
        app_name: if workspace.app_name.trim().is_empty() {
            workspace.name.clone()
        } else {
            workspace.app_name.clone()
        },
        logo_url: logo_url_from_path(&workspace.logo_path),
        theme_key: workspace.theme_key.clone(),
        background_hue: workspace.background_hue,
        body_font: workspace.body_font.clone(),
        heading_font: workspace.heading_font.clone(),
        primary,
        secondary,
        on_primary,
    }
}

pub fn default_workspace_brand_view() -> WorkspaceBrandView {
    let (primary, secondary, on_primary) = theme_palette("kinetic");
    WorkspaceBrandView {
        app_name: "Kinetic".to_string(),
        logo_url: "/static/logo.png".to_string(),
        theme_key: "kinetic".to_string(),
        background_hue: 32,
        body_font: "Space Grotesk".to_string(),
        heading_font: "Space Grotesk".to_string(),
        primary,
        secondary,
        on_primary,
    }
}

fn theme_palette(theme_key: &str) -> (String, String, String) {
    theme_options()
        .into_iter()
        .find(|option| option.key == theme_key)
        .map(|option| (option.primary, option.secondary, option.on_primary))
        .unwrap_or_else(|| {
            (
                "#f59e0b".to_string(),
                "#ef4444".to_string(),
                "#111827".to_string(),
            )
        })
}

fn logo_url_from_path(path: &str) -> String {
    if path.trim().is_empty() {
        "/static/logo.png".to_string()
    } else {
        path.to_string()
    }
}

pub async fn delete_workspace(db: &Db, id: i64) -> Result<(), String> {
    let mut tx = db
        .0
        .begin()
        .await
        .map_err(|err| format!("Unable to start workspace delete: {err}"))?;

    sqlx::query("DELETE FROM outbound_emails WHERE tenant_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to delete workspace emails: {err}"))?;
    sqlx::query("DELETE FROM appointments WHERE tenant_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to delete workspace appointments: {err}"))?;
    sqlx::query("DELETE FROM deployment_updates WHERE tenant_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to delete workspace tracking updates: {err}"))?;
    sqlx::query("DELETE FROM invoices WHERE tenant_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to delete workspace invoices: {err}"))?;
    sqlx::query("DELETE FROM deployments WHERE tenant_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to delete workspace deployments: {err}"))?;
    sqlx::query("DELETE FROM client_contacts WHERE tenant_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to delete workspace contacts: {err}"))?;
    sqlx::query("DELETE FROM clients WHERE tenant_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to delete workspace clients: {err}"))?;
    sqlx::query("DELETE FROM crew_members WHERE tenant_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to delete workspace crew members: {err}"))?;
    sqlx::query("DELETE FROM crews WHERE tenant_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to delete workspace crews: {err}"))?;
    sqlx::query("DELETE FROM user_permissions WHERE tenant_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to delete workspace permissions: {err}"))?;
    sqlx::query("DELETE FROM users WHERE tenant_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to delete workspace users: {err}"))?;
    sqlx::query("DELETE FROM tenants WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            if is_foreign_key_violation(&err) {
                "Unable to delete workspace because it still has related data. Delete the workspace data before removing the workspace.".to_string()
            } else {
                format!("Unable to delete workspace: {err}")
            }
        })?;

    tx.commit()
        .await
        .map_err(|err| format!("Unable to finalize workspace delete: {err}"))?;
    Ok(())
}

fn is_foreign_key_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => {
            db_err.code().map(|code| code == "787").unwrap_or(false)
                || db_err.message().contains("FOREIGN KEY constraint failed")
        }
        _ => false,
    }
}

pub async fn seed_demo_data(db: &Db, tenant_id: i64) -> Result<(), String> {
    let client_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM clients WHERE tenant_id = ?")
        .bind(tenant_id)
        .fetch_one(&db.0)
        .await
        .map_err(|err| format!("Unable to check clients: {err}"))?
        .get("count");
    let contact_count: i64 =
        sqlx::query("SELECT COUNT(*) as count FROM client_contacts WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&db.0)
            .await
            .map_err(|err| format!("Unable to check contacts: {err}"))?
            .get("count");
    let appointment_count: i64 =
        sqlx::query("SELECT COUNT(*) as count FROM appointments WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&db.0)
            .await
            .map_err(|err| format!("Unable to check appointments: {err}"))?
            .get("count");

    if client_count > 0 || contact_count > 0 || appointment_count > 0 {
        return Err("Demo data is only available for brand-new workspaces.".to_string());
    }

    let mut tx = db
        .0
        .begin()
        .await
        .map_err(|err| format!("Unable to start seed transaction: {err}"))?;
    let mut client_ids = Vec::new();
    let mut contact_ids = Vec::new();
    let stages = ["Proposal", "Negotiation", "Closed"];

    for index in 1..=50 {
        let stage = stages[(index - 1) % stages.len()];
        sqlx::query(
            r#"
            INSERT INTO clients
                (tenant_id, company_name, address, phone, email, latitude, longitude, stage)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(tenant_id)
        .bind(format!("Client {index}"))
        .bind(format!("{index} Kinetic Way"))
        .bind(format!("555-010{:02}", index % 100))
        .bind(format!("client{index}@example.com"))
        .bind("37.7749")
        .bind("-122.4194")
        .bind(stage)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to seed clients: {err}"))?;
        let client_id: i64 = sqlx::query("SELECT last_insert_rowid() as id")
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| format!("Unable to read client id: {err}"))?
            .get("id");
        client_ids.push(client_id);
    }

    for index in 1..=50 {
        let client_id = client_ids[(index - 1) % client_ids.len()];
        sqlx::query(
            r#"
            INSERT INTO client_contacts
                (client_id, tenant_id, name, address, email, phone, department, position)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(client_id)
        .bind(tenant_id)
        .bind(format!("Contact {index}"))
        .bind(format!("{index} Kinetic Way"))
        .bind(format!("contact{index}@example.com"))
        .bind(format!("555-020{:02}", index % 100))
        .bind(format!("Department {}", (index - 1) % 5 + 1))
        .bind(format!("Position {}", (index - 1) % 4 + 1))
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to seed contacts: {err}"))?;
        let contact_id: i64 = sqlx::query("SELECT last_insert_rowid() as id")
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| format!("Unable to read contact id: {err}"))?
            .get("id");
        contact_ids.push(contact_id);
    }

    for index in 1..=50 {
        let contact_id = contact_ids[(index - 1) % contact_ids.len()];
        let client_id = client_ids[(index - 1) % client_ids.len()];
        let scheduled_for = format!(
            "2026-01-{:02} 09:{:02}",
            (index - 1) % 28 + 1,
            index % 60
        );
        let status = match index % 5 {
            0 => "No-Show",
            1 => "Scheduled",
            2 => "Confirmed",
            3 => "Attended",
            _ => "Cancelled",
        };
        sqlx::query(
            r#"
            INSERT INTO appointments
                (tenant_id, client_id, contact_id, title, scheduled_for, status, notes)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(tenant_id)
        .bind(client_id)
        .bind(contact_id)
        .bind(format!("Meeting {index}"))
        .bind(scheduled_for)
        .bind(status)
        .bind(format!("Notes for meeting {index}."))
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Unable to seed appointments: {err}"))?;
    }

    tx.commit()
        .await
        .map_err(|err| format!("Unable to commit demo seed: {err}"))?;

    Ok(())
}


use rocket_db_pools::sqlx::{self, Row};

use crate::models::{Workspace, WorkspaceEmailSettingsForm, WorkspaceEmailSettingsView, WorkspaceFormView};
use crate::repositories::tenant_repo;
use crate::services::utils::normalize_slug;
use crate::Db;

pub struct WorkspaceError {
    pub message: String,
    pub form: WorkspaceFormView,
}

pub struct WorkspaceEmailSettingsError {
    pub message: String,
    pub form: WorkspaceEmailSettingsView,
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
) -> Result<(), WorkspaceError> {
    let slug = match normalize_slug(&slug_input) {
        Some(slug) => slug,
        None => {
            return Err(WorkspaceError {
                message: "Slug must be lowercase letters, numbers, or dashes.".to_string(),
                form: WorkspaceFormView::new(slug_input, name_input),
            })
        }
    };

    if name_input.trim().is_empty() {
        return Err(WorkspaceError {
            message: "Workspace name is required.".to_string(),
            form: WorkspaceFormView::new(slug, name_input),
        });
    }

    if let Err(err) = tenant_repo::create_tenant(db, &slug, name_input.trim()).await {
        return Err(WorkspaceError {
            message: format!("Unable to create workspace: {err}"),
            form: WorkspaceFormView::new(slug, name_input),
        });
    }

    Ok(())
}

pub async fn update_workspace(
    db: &Db,
    id: i64,
    slug_input: String,
    name_input: String,
) -> Result<(), WorkspaceError> {
    let slug = match normalize_slug(&slug_input) {
        Some(slug) => slug,
        None => {
            return Err(WorkspaceError {
                message: "Slug must be lowercase letters, numbers, or dashes.".to_string(),
                form: WorkspaceFormView::new(slug_input, name_input),
            })
        }
    };

    if name_input.trim().is_empty() {
        return Err(WorkspaceError {
            message: "Workspace name is required.".to_string(),
            form: WorkspaceFormView::new(slug, name_input),
        });
    }

    if let Err(err) = tenant_repo::update_workspace(db, id, &slug, name_input.trim()).await {
        return Err(WorkspaceError {
            message: format!("Unable to update workspace: {err}"),
            form: WorkspaceFormView::new(slug, name_input),
        });
    }

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
                email_provider, form.from_name, "", form.smtp_host, form.smtp_port,
                form.smtp_username, form.smtp_password, form.smtp_encryption, form.mailgun_domain,
                form.mailgun_api_key, form.postmark_server_token, form.resend_api_key,
                form.ses_access_key, form.ses_secret_key, form.ses_region, form.sendmail_path,
            ),
        });
    }

    let required = match email_provider.as_str() {
        "Mailtrap" | "SMTP" => {
            let mut missing = Vec::new();
            if form.smtp_host.trim().is_empty() {
                missing.push("SMTP host");
            }
            if form.smtp_port.trim().is_empty() {
                missing.push("SMTP port");
            }
            if form.smtp_username.trim().is_empty() {
                missing.push("SMTP username");
            }
            if form.smtp_password.trim().is_empty() {
                missing.push("SMTP password");
            }
            if !missing.is_empty() {
                return Err(WorkspaceEmailSettingsError {
                    message: format!("Missing required fields: {}.", missing.join(", ")),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider, form.from_name, from_address, form.smtp_host, form.smtp_port,
                        form.smtp_username, form.smtp_password, form.smtp_encryption,
                        form.mailgun_domain, form.mailgun_api_key, form.postmark_server_token,
                        form.resend_api_key, form.ses_access_key, form.ses_secret_key,
                        form.ses_region, form.sendmail_path,
                    ),
                });
            }
            true
        }
        "Amazon SES (Simple Email Service)" => {
            if form.ses_access_key.trim().is_empty()
                || form.ses_secret_key.trim().is_empty()
                || form.ses_region.trim().is_empty()
            {
                return Err(WorkspaceEmailSettingsError {
                    message: "SES access key, secret key, and region are required.".to_string(),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider, form.from_name, from_address, form.smtp_host, form.smtp_port,
                        form.smtp_username, form.smtp_password, form.smtp_encryption,
                        form.mailgun_domain, form.mailgun_api_key, form.postmark_server_token,
                        form.resend_api_key, form.ses_access_key, form.ses_secret_key,
                        form.ses_region, form.sendmail_path,
                    ),
                });
            }
            true
        }
        "Mailgun" => {
            if form.mailgun_domain.trim().is_empty() || form.mailgun_api_key.trim().is_empty() {
                return Err(WorkspaceEmailSettingsError {
                    message: "Mailgun domain and API key are required.".to_string(),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider, form.from_name, from_address, form.smtp_host, form.smtp_port,
                        form.smtp_username, form.smtp_password, form.smtp_encryption,
                        form.mailgun_domain, form.mailgun_api_key, form.postmark_server_token,
                        form.resend_api_key, form.ses_access_key, form.ses_secret_key,
                        form.ses_region, form.sendmail_path,
                    ),
                });
            }
            true
        }
        "Postmark" => {
            if form.postmark_server_token.trim().is_empty() {
                return Err(WorkspaceEmailSettingsError {
                    message: "Postmark server token is required.".to_string(),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider, form.from_name, from_address, form.smtp_host, form.smtp_port,
                        form.smtp_username, form.smtp_password, form.smtp_encryption,
                        form.mailgun_domain, form.mailgun_api_key, form.postmark_server_token,
                        form.resend_api_key, form.ses_access_key, form.ses_secret_key,
                        form.ses_region, form.sendmail_path,
                    ),
                });
            }
            true
        }
        "Resend" => {
            if form.resend_api_key.trim().is_empty() {
                return Err(WorkspaceEmailSettingsError {
                    message: "Resend API key is required.".to_string(),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider, form.from_name, from_address, form.smtp_host, form.smtp_port,
                        form.smtp_username, form.smtp_password, form.smtp_encryption,
                        form.mailgun_domain, form.mailgun_api_key, form.postmark_server_token,
                        form.resend_api_key, form.ses_access_key, form.ses_secret_key,
                        form.ses_region, form.sendmail_path,
                    ),
                });
            }
            true
        }
        "Sendmail" => {
            if form.sendmail_path.trim().is_empty() {
                return Err(WorkspaceEmailSettingsError {
                    message: "Sendmail path is required.".to_string(),
                    form: WorkspaceEmailSettingsView::new(
                        email_provider, form.from_name, from_address, form.smtp_host, form.smtp_port,
                        form.smtp_username, form.smtp_password, form.smtp_encryption,
                        form.mailgun_domain, form.mailgun_api_key, form.postmark_server_token,
                        form.resend_api_key, form.ses_access_key, form.ses_secret_key,
                        form.ses_region, form.sendmail_path,
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
                email_provider, form.from_name, from_address, form.smtp_host, form.smtp_port,
                form.smtp_username, form.smtp_password, form.smtp_encryption, form.mailgun_domain,
                form.mailgun_api_key, form.postmark_server_token, form.resend_api_key,
                form.ses_access_key, form.ses_secret_key, form.ses_region, form.sendmail_path,
            ),
        });
    }

    if let Err(err) = tenant_repo::update_email_settings(
        db,
        id,
        &email_provider,
        form.from_name.trim(),
        &from_address,
        form.smtp_host.trim(),
        form.smtp_port.trim(),
        form.smtp_username.trim(),
        form.smtp_password.trim(),
        form.smtp_encryption.trim(),
        form.mailgun_domain.trim(),
        form.mailgun_api_key.trim(),
        form.postmark_server_token.trim(),
        form.resend_api_key.trim(),
        form.ses_access_key.trim(),
        form.ses_secret_key.trim(),
        form.ses_region.trim(),
        form.sendmail_path.trim(),
    )
    .await
    {
        return Err(WorkspaceEmailSettingsError {
            message: format!("Unable to update email settings: {err}"),
            form: WorkspaceEmailSettingsView::new(
                email_provider, form.from_name, from_address, form.smtp_host, form.smtp_port,
                form.smtp_username, form.smtp_password, form.smtp_encryption, form.mailgun_domain,
                form.mailgun_api_key, form.postmark_server_token, form.resend_api_key,
                form.ses_access_key, form.ses_secret_key, form.ses_region, form.sendmail_path,
            ),
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

pub async fn delete_workspace(db: &Db, id: i64) -> Result<(), String> {
    tenant_repo::delete_users_by_tenant(db, id)
        .await
        .map_err(|err| format!("Unable to delete workspace users: {err}"))?;
    tenant_repo::delete_workspace(db, id)
        .await
        .map_err(|err| format!("Unable to delete workspace: {err}"))?;
    Ok(())
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
        let status = if index % 3 == 0 {
            "On Going"
        } else if index % 5 == 0 {
            "Cancelled"
        } else {
            "Scheduled"
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

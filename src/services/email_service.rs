use lettre::message::{header::ContentType, Mailbox, Message, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use rocket_db_pools::sqlx;

use crate::models::EmailFormView;
use crate::repositories::{email_repo, tenant_repo};
use crate::Db;

pub struct EmailError {
    pub message: String,
    pub form: EmailFormView,
}

pub async fn count_outbound_emails(
    db: &Db,
    tenant_id: i64,
) -> Result<i64, sqlx::Error> {
    email_repo::count_outbound_emails(db, tenant_id).await
}

pub async fn count_outbound_emails_by_status(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    email_repo::count_outbound_emails_by_status(db, tenant_id).await
}

pub async fn queue_email(
    db: &Db,
    tenant_id: i64,
    client_id: Option<i64>,
    contact_id: Option<i64>,
    to_email: String,
    cc_emails: Vec<String>,
    subject: String,
    html_body: String,
) -> Result<(), EmailError> {
    let subject = subject.trim().to_string();
    if subject.is_empty() {
        return Err(EmailError {
            message: "Email subject is required.".to_string(),
            form: EmailFormView::new("", html_body),
        });
    }
    let html_body = html_body.trim().to_string();
    if html_body.is_empty() {
        return Err(EmailError {
            message: "Email body is required.".to_string(),
            form: EmailFormView::new(subject, ""),
        });
    }
    let to_email = to_email.trim().to_string();
    if to_email.is_empty() {
        return Err(EmailError {
            message: "Recipient email is required.".to_string(),
            form: EmailFormView::new(subject, html_body),
        });
    }

    let workspace = match tenant_repo::find_workspace_by_id(db, tenant_id).await {
        Ok(Some(workspace)) => workspace,
        _ => {
            return Err(EmailError {
                message: "Workspace not found.".to_string(),
                form: EmailFormView::new(subject, html_body),
            })
        }
    };

    let cc_list = cc_emails
        .iter()
        .map(|email| email.trim())
        .filter(|email| !email.is_empty())
        .collect::<Vec<&str>>()
        .join(", ");

    let outbound_id = match email_repo::create_outbound_email(
        db,
        tenant_id,
        client_id,
        contact_id,
        &to_email,
        &cc_list,
        &subject,
        &html_body,
        &workspace.email_provider,
    )
    .await {
        Ok(id) => id,
        Err(err) => {
            return Err(EmailError {
                message: format!("Unable to queue email: {err}"),
                form: EmailFormView::new(subject, html_body),
            });
        }
    };

    if workspace.email_provider == "Mailtrap" {
        if let Err(err) = send_via_mailtrap(&workspace, &to_email, &cc_list, &subject, &html_body).await
        {
            let _ = email_repo::update_outbound_email_status(db, outbound_id, "Failed").await;
            return Err(EmailError {
                message: format!("Unable to send email via Mailtrap: {err}"),
                form: EmailFormView::new(subject, html_body),
            });
        }
        let _ = email_repo::update_outbound_email_status(db, outbound_id, "Sent").await;
    }

    Ok(())
}

async fn send_via_mailtrap(
    workspace: &crate::models::Workspace,
    to_email: &str,
    cc_emails: &str,
    subject: &str,
    html_body: &str,
) -> Result<(), String> {
    let from_address = workspace.email_from_address.trim();
    if from_address.is_empty() {
        return Err("Mailtrap requires a from address.".to_string());
    }
    let from = Mailbox::new(
        if workspace.email_from_name.trim().is_empty() {
            None
        } else {
            Some(workspace.email_from_name.clone())
        },
        from_address
            .parse()
            .map_err(|_| "Invalid from address.".to_string())?,
    );

    let mut builder = Message::builder().from(from);
    builder = builder
        .to(to_email.parse().map_err(|_| "Invalid to address.".to_string())?)
        .subject(subject);

    if !cc_emails.trim().is_empty() {
        for email in cc_emails.split(',') {
            let trimmed = email.trim();
            if !trimmed.is_empty() {
                builder = builder
                    .cc(trimmed.parse().map_err(|_| "Invalid CC address.".to_string())?);
            }
        }
    }

    let message = builder
        .singlepart(
            SinglePart::builder()
                .header(ContentType::TEXT_HTML)
                .body(html_body.to_string()),
        )
        .map_err(|_| "Unable to build email.".to_string())?;

    let smtp_host = workspace.smtp_host.trim();
    let smtp_port: u16 = workspace
        .smtp_port
        .trim()
        .parse()
        .map_err(|_| "Invalid SMTP port.".to_string())?;
    let smtp_username = workspace.smtp_username.trim();
    let smtp_password = workspace.smtp_password.trim();

    let credentials = Credentials::new(smtp_username.to_string(), smtp_password.to_string());

    let encryption = workspace.smtp_encryption.trim();
    let transport = match encryption {
        "SSL/TLS" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp_host)
            .port(smtp_port)
            .tls(Tls::Wrapper(
                TlsParameters::new(smtp_host.to_string())
                    .map_err(|_| "Invalid TLS configuration.".to_string())?,
            ))
            .credentials(credentials)
            .build(),
        "" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp_host)
            .port(smtp_port)
            .tls(Tls::None)
            .credentials(credentials)
            .build(),
        _ => {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(smtp_host)
                .map_err(|_| "Unable to configure STARTTLS.".to_string())?
                .port(smtp_port)
                .credentials(credentials)
                .build()
        }
    };

    transport
        .send(message)
        .await
        .map_err(|_| "SMTP send failed.".to_string())?;

    Ok(())
}

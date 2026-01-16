use rocket_db_pools::sqlx;
use rocket_db_pools::sqlx::Row;
use serde::Serialize;

use crate::Db;

#[derive(Serialize)]
pub struct OutboundEmailRow {
    pub id: i64,
    pub tenant_id: i64,
    pub to_email: String,
    pub cc_emails: String,
    pub subject: String,
    pub html_body: String,
    pub provider: String,
}

#[derive(Serialize)]
pub struct OutboundEmailLogRow {
    pub id: i64,
    pub to_email: String,
    pub cc_emails: String,
    pub subject: String,
    pub status: String,
    pub provider: String,
    pub error_message: String,
    pub created_at: String,
}

pub async fn create_outbound_email(
    db: &Db,
    tenant_id: i64,
    client_id: Option<i64>,
    contact_id: Option<i64>,
    to_email: &str,
    cc_emails: &str,
    subject: &str,
    html_body: &str,
    provider: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO outbound_emails
            (tenant_id, client_id, contact_id, to_email, cc_emails, subject, html_body, provider)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(contact_id)
    .bind(to_email)
    .bind(cc_emails)
    .bind(subject)
    .bind(html_body)
    .bind(provider)
    .execute(&db.0)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_outbound_email_status(
    db: &Db,
    id: i64,
    status: &str,
) -> Result<(), sqlx::Error> {
    update_outbound_email_status_with_error(db, id, status, "")
        .await?;
    Ok(())
}

pub async fn update_outbound_email_status_with_error(
    db: &Db,
    id: i64,
    status: &str,
    error_message: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE outbound_emails SET status = ?, error_message = ? WHERE id = ?")
        .bind(status)
        .bind(error_message)
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}

pub async fn list_outbound_emails_by_status(
    db: &Db,
    status: &str,
    limit: i64,
) -> Result<Vec<OutboundEmailRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id,
               tenant_id,
               to_email,
               cc_emails,
               subject,
               html_body,
               provider
        FROM outbound_emails
        WHERE status = ?
        ORDER BY id ASC
        LIMIT ?
        "#,
    )
    .bind(status)
    .bind(limit)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| OutboundEmailRow {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            to_email: row.get("to_email"),
            cc_emails: row.get("cc_emails"),
            subject: row.get("subject"),
            html_body: row.get("html_body"),
            provider: row.get("provider"),
        })
        .collect())
}

pub async fn list_outbound_emails_for_tenant(
    db: &Db,
    tenant_id: i64,
    limit: i64,
) -> Result<Vec<OutboundEmailLogRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id,
               to_email,
               cc_emails,
               subject,
               status,
               provider,
               error_message,
               created_at
        FROM outbound_emails
        WHERE tenant_id = ?
        ORDER BY created_at DESC, id DESC
        LIMIT ?
        "#,
    )
    .bind(tenant_id)
    .bind(limit)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| OutboundEmailLogRow {
            id: row.get("id"),
            to_email: row.get("to_email"),
            cc_emails: row.get("cc_emails"),
            subject: row.get("subject"),
            status: row.get("status"),
            provider: row.get("provider"),
            error_message: row.get("error_message"),
            created_at: row.get("created_at"),
        })
        .collect())
}

pub async fn count_outbound_emails(
    db: &Db,
    tenant_id: i64,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM outbound_emails WHERE tenant_id = ?")
        .bind(tenant_id)
        .fetch_one(&db.0)
        .await?;
    Ok(row.get("count"))
}

pub async fn count_outbound_emails_all(db: &Db) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM outbound_emails")
        .fetch_one(&db.0)
        .await?;
    Ok(row.get("count"))
}

pub async fn count_outbound_emails_by_status(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT status, COUNT(*) as count FROM outbound_emails WHERE tenant_id = ? GROUP BY status",
    )
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.get("status"), row.get("count")))
        .collect())
}

pub async fn count_outbound_emails_by_status_all(
    db: &Db,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows =
        sqlx::query("SELECT status, COUNT(*) as count FROM outbound_emails GROUP BY status")
            .fetch_all(&db.0)
            .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.get("status"), row.get("count")))
        .collect())
}

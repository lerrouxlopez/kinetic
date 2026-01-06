use rocket_db_pools::sqlx;
use rocket_db_pools::sqlx::Row;

use crate::Db;

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
    sqlx::query("UPDATE outbound_emails SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
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

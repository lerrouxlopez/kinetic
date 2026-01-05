use rocket_db_pools::sqlx;

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

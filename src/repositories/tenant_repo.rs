use rocket_db_pools::sqlx::{self, Row};

use crate::models::Workspace;
use crate::Db;

pub async fn find_tenant_id_by_slug(
    db: &Db,
    slug: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let row = sqlx::query("SELECT id FROM tenants WHERE slug = ?")
        .bind(slug)
        .fetch_optional(&db.0)
        .await?;
    Ok(row.map(|row| row.get("id")))
}

pub async fn create_tenant(
    db: &Db,
    slug: &str,
    name: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query("INSERT INTO tenants (slug, name) VALUES (?, ?)")
        .bind(slug)
        .bind(name)
        .execute(&db.0)
        .await?;

    let row = sqlx::query("SELECT id FROM tenants WHERE slug = ?")
        .bind(slug)
        .fetch_one(&db.0)
        .await?;
    Ok(row.get("id"))
}

pub async fn list_workspaces(db: &Db) -> Result<Vec<Workspace>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id,
               slug,
               name,
               email_provider,
               email_from_name,
               email_from_address,
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
               sendmail_path
        FROM tenants
        ORDER BY id DESC
        "#,
    )
        .fetch_all(&db.0)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| Workspace {
            id: row.get("id"),
            slug: row.get("slug"),
            name: row.get("name"),
            email_provider: row.get("email_provider"),
            email_from_name: row.get("email_from_name"),
            email_from_address: row.get("email_from_address"),
            smtp_host: row.get("smtp_host"),
            smtp_port: row.get("smtp_port"),
            smtp_username: row.get("smtp_username"),
            smtp_password: row.get("smtp_password"),
            smtp_encryption: row.get("smtp_encryption"),
            mailgun_domain: row.get("mailgun_domain"),
            mailgun_api_key: row.get("mailgun_api_key"),
            postmark_server_token: row.get("postmark_server_token"),
            resend_api_key: row.get("resend_api_key"),
            ses_access_key: row.get("ses_access_key"),
            ses_secret_key: row.get("ses_secret_key"),
            ses_region: row.get("ses_region"),
            sendmail_path: row.get("sendmail_path"),
        })
        .collect())
}

pub async fn list_workspaces_paged(
    db: &Db,
    limit: i64,
    offset: i64,
) -> Result<Vec<Workspace>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id,
               slug,
               name,
               email_provider,
               email_from_name,
               email_from_address,
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
               sendmail_path
        FROM tenants
        ORDER BY id DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Workspace {
            id: row.get("id"),
            slug: row.get("slug"),
            name: row.get("name"),
            email_provider: row.get("email_provider"),
            email_from_name: row.get("email_from_name"),
            email_from_address: row.get("email_from_address"),
            smtp_host: row.get("smtp_host"),
            smtp_port: row.get("smtp_port"),
            smtp_username: row.get("smtp_username"),
            smtp_password: row.get("smtp_password"),
            smtp_encryption: row.get("smtp_encryption"),
            mailgun_domain: row.get("mailgun_domain"),
            mailgun_api_key: row.get("mailgun_api_key"),
            postmark_server_token: row.get("postmark_server_token"),
            resend_api_key: row.get("resend_api_key"),
            ses_access_key: row.get("ses_access_key"),
            ses_secret_key: row.get("ses_secret_key"),
            ses_region: row.get("ses_region"),
            sendmail_path: row.get("sendmail_path"),
        })
        .collect())
}

pub async fn count_workspaces(db: &Db) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM tenants")
        .fetch_one(&db.0)
        .await?;
    Ok(row.get("count"))
}

pub async fn find_workspace_by_id(
    db: &Db,
    id: i64,
) -> Result<Option<Workspace>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id,
               slug,
               name,
               email_provider,
               email_from_name,
               email_from_address,
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
               sendmail_path
        FROM tenants
        WHERE id = ?
        "#,
    )
        .bind(id)
        .fetch_optional(&db.0)
        .await?;

    Ok(row.map(|row| Workspace {
        id: row.get("id"),
        slug: row.get("slug"),
        name: row.get("name"),
        email_provider: row.get("email_provider"),
        email_from_name: row.get("email_from_name"),
        email_from_address: row.get("email_from_address"),
        smtp_host: row.get("smtp_host"),
        smtp_port: row.get("smtp_port"),
        smtp_username: row.get("smtp_username"),
        smtp_password: row.get("smtp_password"),
        smtp_encryption: row.get("smtp_encryption"),
        mailgun_domain: row.get("mailgun_domain"),
        mailgun_api_key: row.get("mailgun_api_key"),
        postmark_server_token: row.get("postmark_server_token"),
        resend_api_key: row.get("resend_api_key"),
        ses_access_key: row.get("ses_access_key"),
        ses_secret_key: row.get("ses_secret_key"),
        ses_region: row.get("ses_region"),
        sendmail_path: row.get("sendmail_path"),
    }))
}

pub async fn update_workspace(
    db: &Db,
    id: i64,
    slug: &str,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tenants SET slug = ?, name = ? WHERE id = ?")
        .bind(slug)
        .bind(name)
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}

pub async fn update_email_settings(
    db: &Db,
    id: i64,
    email_provider: &str,
    from_name: &str,
    from_address: &str,
    smtp_host: &str,
    smtp_port: &str,
    smtp_username: &str,
    smtp_password: &str,
    smtp_encryption: &str,
    mailgun_domain: &str,
    mailgun_api_key: &str,
    postmark_server_token: &str,
    resend_api_key: &str,
    ses_access_key: &str,
    ses_secret_key: &str,
    ses_region: &str,
    sendmail_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE tenants
        SET email_provider = ?,
            email_from_name = ?,
            email_from_address = ?,
            smtp_host = ?,
            smtp_port = ?,
            smtp_username = ?,
            smtp_password = ?,
            smtp_encryption = ?,
            mailgun_domain = ?,
            mailgun_api_key = ?,
            postmark_server_token = ?,
            resend_api_key = ?,
            ses_access_key = ?,
            ses_secret_key = ?,
            ses_region = ?,
            sendmail_path = ?
        WHERE id = ?
        "#,
    )
    .bind(email_provider)
    .bind(from_name)
    .bind(from_address)
    .bind(smtp_host)
    .bind(smtp_port)
    .bind(smtp_username)
    .bind(smtp_password)
    .bind(smtp_encryption)
    .bind(mailgun_domain)
    .bind(mailgun_api_key)
    .bind(postmark_server_token)
    .bind(resend_api_key)
    .bind(ses_access_key)
    .bind(ses_secret_key)
    .bind(ses_region)
    .bind(sendmail_path)
    .bind(id)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn delete_workspace(db: &Db, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM tenants WHERE id = ?")
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}

pub async fn delete_users_by_tenant(db: &Db, tenant_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE tenant_id = ?")
        .bind(tenant_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

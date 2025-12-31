use rocket_db_pools::sqlx::{self, Row};

use crate::models::{User, UserAuth};
use crate::Db;

pub async fn find_user_by_ids(
    db: &Db,
    user_id: i64,
    tenant_id: i64,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT users.id, users.email, tenants.id as tenant_id, tenants.slug
        FROM users
        JOIN tenants ON tenants.id = users.tenant_id
        WHERE users.id = ? AND tenants.id = ?
        "#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| User {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        tenant_slug: row.get("slug"),
        email: row.get("email"),
    }))
}

pub async fn find_user_by_email_and_tenant(
    db: &Db,
    email: &str,
    tenant_id: i64,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT users.id, users.email, tenants.id as tenant_id, tenants.slug
        FROM users
        JOIN tenants ON tenants.id = users.tenant_id
        WHERE users.email = ? AND tenants.id = ?
        "#,
    )
    .bind(email)
    .bind(tenant_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| User {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        tenant_slug: row.get("slug"),
        email: row.get("email"),
    }))
}

pub async fn find_user_auth_by_email_and_tenant_slug(
    db: &Db,
    email: &str,
    tenant_slug: &str,
) -> Result<Option<UserAuth>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT users.id, users.email, users.password_hash, tenants.id as tenant_id, tenants.slug
        FROM users
        JOIN tenants ON tenants.id = users.tenant_id
        WHERE users.email = ? AND tenants.slug = ?
        "#,
    )
    .bind(email)
    .bind(tenant_slug)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| UserAuth {
        user: User {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_slug: row.get("slug"),
            email: row.get("email"),
        },
        password_hash: row.get("password_hash"),
    }))
}

pub async fn create_user(
    db: &Db,
    tenant_id: i64,
    email: &str,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO users (tenant_id, email, password_hash) VALUES (?, ?, ?)")
        .bind(tenant_id)
        .bind(email)
        .bind(password_hash)
        .execute(&db.0)
        .await?;
    Ok(())
}

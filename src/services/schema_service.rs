use rocket_db_pools::sqlx;
use std::env;

use crate::repositories::admin_repo;
use crate::services::utils::hash_password;
use crate::Db;

pub async fn ensure_schema(db: &Db) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tenants (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            slug TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            email TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(tenant_id, email),
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS admins (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS crews (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            members_count INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'Active',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    seed_admin(db).await?;

    Ok(())
}

async fn seed_admin(db: &Db) -> Result<(), sqlx::Error> {
    if admin_repo::any_admin_exists(db).await? {
        return Ok(());
    }

    let email = env::var("KINETIC_ADMIN_EMAIL").unwrap_or_else(|_| "admin@kinetic.local".to_string());
    let password = env::var("KINETIC_ADMIN_PASSWORD").unwrap_or_else(|_| "ChangeMe123!".to_string());
    let hash = hash_password(&password).map_err(|_| sqlx::Error::RowNotFound)?;

    admin_repo::create_admin(db, &email.trim().to_lowercase(), &hash).await?;

    Ok(())
}

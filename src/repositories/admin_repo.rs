use rocket_db_pools::sqlx::{self, Row};

use crate::models::{AdminAuth, AdminUser};
use crate::Db;

pub async fn find_admin_by_id(
    db: &Db,
    admin_id: i64,
) -> Result<Option<AdminUser>, sqlx::Error> {
    let row = sqlx::query("SELECT id, email FROM admins WHERE id = ?")
        .bind(admin_id)
        .fetch_optional(&db.0)
        .await?;

    Ok(row.map(|row| AdminUser {
        id: row.get("id"),
        email: row.get("email"),
    }))
}

pub async fn find_admin_auth_by_email(
    db: &Db,
    email: &str,
) -> Result<Option<AdminAuth>, sqlx::Error> {
    let row = sqlx::query("SELECT id, email, password_hash FROM admins WHERE email = ?")
        .bind(email)
        .fetch_optional(&db.0)
        .await?;

    Ok(row.map(|row| AdminAuth {
        admin: AdminUser {
            id: row.get("id"),
            email: row.get("email"),
        },
        password_hash: row.get("password_hash"),
    }))
}

pub async fn any_admin_exists(db: &Db) -> Result<bool, sqlx::Error> {
    let row = sqlx::query("SELECT id FROM admins LIMIT 1")
        .fetch_optional(&db.0)
        .await?;
    Ok(row.is_some())
}

pub async fn create_admin(
    db: &Db,
    email: &str,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO admins (email, password_hash) VALUES (?, ?)")
        .bind(email)
        .bind(password_hash)
        .execute(&db.0)
        .await?;
    Ok(())
}

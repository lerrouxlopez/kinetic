use rocket_db_pools::sqlx::{self, Row};

use crate::models::{AdminUserSummary, User, UserAuth};
use crate::repositories::tenant_repo;
use crate::Db;

pub async fn find_user_by_ids(
    db: &Db,
    user_id: i64,
    tenant_id: i64,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT users.id, users.email, users.role, users.is_super_admin, tenants.id as tenant_id, tenants.slug, tenants.plan_key
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
        plan_key: row.get("plan_key"),
        email: row.get("email"),
        role: row.get("role"),
        is_super_admin: row.get::<i64, _>("is_super_admin") == 1,
    }))
}

pub async fn find_user_by_email_and_tenant(
    db: &Db,
    email: &str,
    tenant_id: i64,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT users.id, users.email, users.role, users.is_super_admin, tenants.id as tenant_id, tenants.slug, tenants.plan_key
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
        plan_key: row.get("plan_key"),
        email: row.get("email"),
        role: row.get("role"),
        is_super_admin: row.get::<i64, _>("is_super_admin") == 1,
    }))
}

pub async fn find_user_by_id(
    db: &Db,
    tenant_id: i64,
    user_id: i64,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT users.id, users.email, users.role, users.is_super_admin, tenants.id as tenant_id, tenants.slug, tenants.plan_key
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
        plan_key: row.get("plan_key"),
        email: row.get("email"),
        role: row.get("role"),
        is_super_admin: row.get::<i64, _>("is_super_admin") == 1,
    }))
}

pub async fn find_user_by_id_any(
    db: &Db,
    user_id: i64,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT users.id, users.email, users.role, users.is_super_admin, tenants.id as tenant_id, tenants.slug, tenants.plan_key
        FROM users
        JOIN tenants ON tenants.id = users.tenant_id
        WHERE users.id = ?
        "#,
    )
    .bind(user_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| User {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        tenant_slug: row.get("slug"),
        plan_key: row.get("plan_key"),
        email: row.get("email"),
        role: row.get("role"),
        is_super_admin: row.get::<i64, _>("is_super_admin") == 1,
    }))
}

pub async fn find_user_auth_by_email_and_tenant_slug(
    db: &Db,
    email: &str,
    tenant_slug: &str,
) -> Result<Option<UserAuth>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT users.id, users.email, users.role, users.is_super_admin, users.password_hash, tenants.id as tenant_id, tenants.slug, tenants.plan_key
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
            plan_key: row.get("plan_key"),
            email: row.get("email"),
            role: row.get("role"),
            is_super_admin: row.get::<i64, _>("is_super_admin") == 1,
        },
        password_hash: row.get("password_hash"),
    }))
}

pub async fn create_user(
    db: &Db,
    tenant_id: i64,
    email: &str,
    password_hash: &str,
    role: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO users (tenant_id, email, password_hash, role) VALUES (?, ?, ?, ?)")
        .bind(tenant_id)
        .bind(email)
        .bind(password_hash)
        .bind(role)
        .execute(&db.0)
        .await?;
    Ok(())
}

pub async fn create_super_admin(
    db: &Db,
    tenant_id: i64,
    email: &str,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (tenant_id, email, password_hash, role, is_super_admin) VALUES (?, ?, ?, ?, 1)",
    )
    .bind(tenant_id)
    .bind(email)
    .bind(password_hash)
    .bind("Owner")
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn list_users_by_tenant(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<crate::models::UserSummary>, sqlx::Error> {
    let admin_tenant_id = tenant_repo::find_tenant_id_by_slug(db, "admin")
        .await?
        .unwrap_or(-1);
    let rows = if tenant_id == admin_tenant_id {
        sqlx::query("SELECT id, email, role FROM users WHERE tenant_id = ? ORDER BY id ASC")
            .bind(tenant_id)
            .fetch_all(&db.0)
            .await?
    } else {
        sqlx::query(
            "SELECT id, email, role FROM users WHERE tenant_id = ? AND is_super_admin = 0 ORDER BY id ASC",
        )
        .bind(tenant_id)
        .fetch_all(&db.0)
        .await?
    };

    Ok(rows
        .into_iter()
        .map(|row| crate::models::UserSummary {
            id: row.get("id"),
            email: row.get("email"),
            role: row.get("role"),
        })
        .collect())
}

pub async fn list_users_all(db: &Db) -> Result<Vec<AdminUserSummary>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT users.id,
               users.email,
               users.role,
               users.is_super_admin,
               tenants.id as tenant_id,
               tenants.slug,
               tenants.name
        FROM users
        JOIN tenants ON tenants.id = users.tenant_id
        ORDER BY tenants.name ASC, users.email ASC
        "#,
    )
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| AdminUserSummary {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_slug: row.get("slug"),
            tenant_name: row.get("name"),
            email: row.get("email"),
            role: row.get("role"),
            is_super_admin: row.get::<i64, _>("is_super_admin") == 1,
        })
        .collect())
}

pub async fn count_users_by_tenant(db: &Db, tenant_id: i64) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE tenant_id = ?")
        .bind(tenant_id)
        .fetch_one(&db.0)
        .await?;
    Ok(row.get("count"))
}

pub async fn find_super_admin_auth_by_email(
    db: &Db,
    email: &str,
) -> Result<Option<UserAuth>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT users.id, users.email, users.role, users.is_super_admin, users.password_hash, tenants.id as tenant_id, tenants.slug, tenants.plan_key
        FROM users
        JOIN tenants ON tenants.id = users.tenant_id
        WHERE users.email = ? AND users.is_super_admin = 1
        "#,
    )
    .bind(email)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| UserAuth {
        user: User {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_slug: row.get("slug"),
            plan_key: row.get("plan_key"),
            email: row.get("email"),
            role: row.get("role"),
            is_super_admin: row.get::<i64, _>("is_super_admin") == 1,
        },
        password_hash: row.get("password_hash"),
    }))
}

pub async fn update_user_admin(
    db: &Db,
    user_id: i64,
    tenant_id: i64,
    email: &str,
    role: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET tenant_id = ?, email = ?, role = ? WHERE id = ?")
        .bind(tenant_id)
        .bind(email)
        .bind(role)
        .bind(user_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

pub async fn update_user_password(
    db: &Db,
    user_id: i64,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(password_hash)
        .bind(user_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

pub async fn delete_user_by_id(db: &Db, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

pub async fn update_user_role(
    db: &Db,
    tenant_id: i64,
    user_id: i64,
    role: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET role = ? WHERE id = ? AND tenant_id = ?")
        .bind(role)
        .bind(user_id)
        .bind(tenant_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

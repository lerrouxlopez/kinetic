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
    let rows = sqlx::query("SELECT id, slug, name FROM tenants ORDER BY id DESC")
        .fetch_all(&db.0)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| Workspace {
            id: row.get("id"),
            slug: row.get("slug"),
            name: row.get("name"),
        })
        .collect())
}

pub async fn find_workspace_by_id(
    db: &Db,
    id: i64,
) -> Result<Option<Workspace>, sqlx::Error> {
    let row = sqlx::query("SELECT id, slug, name FROM tenants WHERE id = ?")
        .bind(id)
        .fetch_optional(&db.0)
        .await?;

    Ok(row.map(|row| Workspace {
        id: row.get("id"),
        slug: row.get("slug"),
        name: row.get("name"),
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

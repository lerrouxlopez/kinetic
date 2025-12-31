use rocket_db_pools::sqlx::{self, Row};

use crate::models::Crew;
use crate::Db;

pub async fn list_crews(db: &Db, tenant_id: i64) -> Result<Vec<Crew>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, tenant_id, name, members_count, status FROM crews WHERE tenant_id = ? ORDER BY id DESC",
    )
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Crew {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            name: row.get("name"),
            members_count: row.get("members_count"),
            status: row.get("status"),
        })
        .collect())
}

pub async fn find_crew_by_id(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
) -> Result<Option<Crew>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, tenant_id, name, members_count, status FROM crews WHERE id = ? AND tenant_id = ?",
    )
    .bind(crew_id)
    .bind(tenant_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| Crew {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        name: row.get("name"),
        members_count: row.get("members_count"),
        status: row.get("status"),
    }))
}

pub async fn create_crew(
    db: &Db,
    tenant_id: i64,
    name: &str,
    members_count: i64,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO crews (tenant_id, name, members_count, status) VALUES (?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(name)
    .bind(members_count)
    .bind(status)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn update_crew(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    name: &str,
    members_count: i64,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE crews SET name = ?, members_count = ?, status = ? WHERE id = ? AND tenant_id = ?",
    )
    .bind(name)
    .bind(members_count)
    .bind(status)
    .bind(crew_id)
    .bind(tenant_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn delete_crew(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM crews WHERE id = ? AND tenant_id = ?")
        .bind(crew_id)
        .bind(tenant_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

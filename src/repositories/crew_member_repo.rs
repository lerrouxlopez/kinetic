use rocket_db_pools::sqlx::{self, Row};

use crate::models::CrewMember;
use crate::Db;

pub async fn list_members(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
) -> Result<Vec<CrewMember>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, crew_id, tenant_id, name, phone, email, position
        FROM crew_members
        WHERE crew_id = ? AND tenant_id = ?
        ORDER BY id DESC
        "#,
    )
    .bind(crew_id)
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| CrewMember {
            id: row.get("id"),
            crew_id: row.get("crew_id"),
            tenant_id: row.get("tenant_id"),
            name: row.get("name"),
            phone: row.get("phone"),
            email: row.get("email"),
            position: row.get("position"),
        })
        .collect())
}

pub async fn list_members_paged(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<CrewMember>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, crew_id, tenant_id, name, phone, email, position
        FROM crew_members
        WHERE crew_id = ? AND tenant_id = ?
        ORDER BY id DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(crew_id)
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| CrewMember {
            id: row.get("id"),
            crew_id: row.get("crew_id"),
            tenant_id: row.get("tenant_id"),
            name: row.get("name"),
            phone: row.get("phone"),
            email: row.get("email"),
            position: row.get("position"),
        })
        .collect())
}

pub async fn count_members(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM crew_members WHERE crew_id = ? AND tenant_id = ?",
    )
    .bind(crew_id)
    .bind(tenant_id)
    .fetch_one(&db.0)
    .await?;
    Ok(row.get("count"))
}

pub async fn find_member_by_id(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    member_id: i64,
) -> Result<Option<CrewMember>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, crew_id, tenant_id, name, phone, email, position
        FROM crew_members
        WHERE id = ? AND crew_id = ? AND tenant_id = ?
        "#,
    )
    .bind(member_id)
    .bind(crew_id)
    .bind(tenant_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| CrewMember {
        id: row.get("id"),
        crew_id: row.get("crew_id"),
        tenant_id: row.get("tenant_id"),
        name: row.get("name"),
        phone: row.get("phone"),
        email: row.get("email"),
        position: row.get("position"),
    }))
}

pub async fn create_member(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    name: &str,
    phone: &str,
    email: &str,
    position: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO crew_members (crew_id, tenant_id, name, phone, email, position)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(crew_id)
    .bind(tenant_id)
    .bind(name)
    .bind(phone)
    .bind(email)
    .bind(position)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn update_member(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    member_id: i64,
    name: &str,
    phone: &str,
    email: &str,
    position: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE crew_members
        SET name = ?, phone = ?, email = ?, position = ?
        WHERE id = ? AND crew_id = ? AND tenant_id = ?
        "#,
    )
    .bind(name)
    .bind(phone)
    .bind(email)
    .bind(position)
    .bind(member_id)
    .bind(crew_id)
    .bind(tenant_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn delete_member(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    member_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM crew_members WHERE id = ? AND crew_id = ? AND tenant_id = ?",
    )
    .bind(member_id)
    .bind(crew_id)
    .bind(tenant_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

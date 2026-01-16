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
        SELECT id, crew_id, tenant_id, user_id, name, phone, email, position, availability_status
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
            user_id: row.get("user_id"),
            name: row.get("name"),
            phone: row.get("phone"),
            email: row.get("email"),
            position: row.get("position"),
            availability_status: row.get("availability_status"),
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
        SELECT id, crew_id, tenant_id, user_id, name, phone, email, position, availability_status
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
            user_id: row.get("user_id"),
            name: row.get("name"),
            phone: row.get("phone"),
            email: row.get("email"),
            position: row.get("position"),
            availability_status: row.get("availability_status"),
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
        SELECT id, crew_id, tenant_id, user_id, name, phone, email, position, availability_status
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
        user_id: row.get("user_id"),
        name: row.get("name"),
        phone: row.get("phone"),
        email: row.get("email"),
        position: row.get("position"),
        availability_status: row.get("availability_status"),
    }))
}

pub async fn create_member(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    user_id: Option<i64>,
    name: &str,
    phone: &str,
    email: &str,
    position: &str,
    availability_status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO crew_members (crew_id, tenant_id, user_id, name, phone, email, position, availability_status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(crew_id)
    .bind(tenant_id)
    .bind(user_id)
    .bind(name)
    .bind(phone)
    .bind(email)
    .bind(position)
    .bind(availability_status)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn update_member(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    member_id: i64,
    user_id: Option<i64>,
    name: &str,
    phone: &str,
    email: &str,
    position: &str,
    availability_status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE crew_members
        SET user_id = ?, name = ?, phone = ?, email = ?, position = ?, availability_status = ?
        WHERE id = ? AND crew_id = ? AND tenant_id = ?
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(phone)
    .bind(email)
    .bind(position)
    .bind(availability_status)
    .bind(member_id)
    .bind(crew_id)
    .bind(tenant_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn list_crew_ids_for_user(
    db: &Db,
    tenant_id: i64,
    user_id: i64,
    email: &str,
) -> Result<Vec<i64>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT crew_id
        FROM crew_members
        WHERE tenant_id = ?
          AND (user_id = ? OR (user_id IS NULL AND lower(email) = lower(?)))
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(email)
    .fetch_all(&db.0)
    .await?;

    Ok(rows.into_iter().map(|row| row.get("crew_id")).collect())
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

pub async fn count_availability_by_crew(
    db: &Db,
    tenant_id: i64,
    crew_ids: &[i64],
) -> Result<Vec<(i64, String, i64)>, sqlx::Error> {
    if crew_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = crew_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
        SELECT crew_id, availability_status, COUNT(*) as count
        FROM crew_members
        WHERE tenant_id = ? AND crew_id IN ({})
        GROUP BY crew_id, availability_status
        "#,
        placeholders
    );
    let mut query = sqlx::query(&sql).bind(tenant_id);
    for crew_id in crew_ids {
        query = query.bind(crew_id);
    }
    let rows = query.fetch_all(&db.0).await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get("crew_id"), row.get("availability_status"), row.get("count")))
        .collect())
}

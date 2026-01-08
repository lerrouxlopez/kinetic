use rocket_db_pools::sqlx::{self, Row};

use crate::models::WorkTimer;
use crate::Db;

pub async fn find_active_timer(
    db: &Db,
    tenant_id: i64,
    user_id: i64,
) -> Result<Option<WorkTimer>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, tenant_id, deployment_id, user_id, start_at, end_at
        FROM work_timers
        WHERE tenant_id = ? AND user_id = ? AND end_at IS NULL
        ORDER BY id DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| WorkTimer {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        deployment_id: row.get("deployment_id"),
        user_id: row.get("user_id"),
        start_at: row.get("start_at"),
        end_at: row.get("end_at"),
    }))
}

pub async fn create_timer(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    user_id: i64,
    start_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO work_timers (tenant_id, deployment_id, user_id, start_at)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .bind(user_id)
    .bind(start_at)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn stop_timer(
    db: &Db,
    tenant_id: i64,
    timer_id: i64,
    end_at: &str,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE work_timers
        SET end_at = ?
        WHERE tenant_id = ? AND id = ? AND end_at IS NULL
        "#,
    )
    .bind(end_at)
    .bind(tenant_id)
    .bind(timer_id)
    .execute(&db.0)
    .await?;

    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

pub async fn list_stale_timers(
    db: &Db,
    tenant_id: i64,
    cutoff: &str,
) -> Result<Vec<WorkTimer>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, tenant_id, deployment_id, user_id, start_at, end_at
        FROM work_timers
        WHERE tenant_id = ? AND end_at IS NULL AND start_at <= ?
        "#,
    )
    .bind(tenant_id)
    .bind(cutoff)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| WorkTimer {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            deployment_id: row.get("deployment_id"),
            user_id: row.get("user_id"),
            start_at: row.get("start_at"),
            end_at: row.get("end_at"),
        })
        .collect())
}

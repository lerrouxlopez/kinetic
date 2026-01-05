use rocket_db_pools::sqlx::{self, Row};

use crate::models::DeploymentUpdate;
use crate::Db;

pub async fn list_updates(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
) -> Result<Vec<DeploymentUpdate>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, tenant_id, deployment_id, work_date, start_time, end_time, hours_worked, notes
        FROM deployment_updates
        WHERE tenant_id = ? AND deployment_id = ?
        ORDER BY work_date DESC, id DESC
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| DeploymentUpdate {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            deployment_id: row.get("deployment_id"),
            work_date: row.get("work_date"),
            start_time: row.get("start_time"),
            end_time: row.get("end_time"),
            hours_worked: row.get("hours_worked"),
            notes: row.get("notes"),
        })
        .collect())
}

pub async fn find_update_by_date(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    work_date: &str,
) -> Result<Option<DeploymentUpdate>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, tenant_id, deployment_id, work_date, start_time, end_time, hours_worked, notes
        FROM deployment_updates
        WHERE tenant_id = ? AND deployment_id = ? AND work_date = ?
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .bind(work_date)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| DeploymentUpdate {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        deployment_id: row.get("deployment_id"),
        work_date: row.get("work_date"),
        start_time: row.get("start_time"),
        end_time: row.get("end_time"),
        hours_worked: row.get("hours_worked"),
        notes: row.get("notes"),
    }))
}

pub async fn create_update(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    work_date: &str,
    start_time: &str,
    end_time: &str,
    hours_worked: f64,
    notes: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO deployment_updates
            (tenant_id, deployment_id, work_date, start_time, end_time, hours_worked, notes)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .bind(work_date)
    .bind(start_time)
    .bind(end_time)
    .bind(hours_worked)
    .bind(notes)
    .execute(&db.0)
    .await?;
    Ok(())
}

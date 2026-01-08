use rocket_db_pools::sqlx::{self, Row};

use crate::models::DeploymentUpdate;
use crate::Db;

fn is_missing_user_id(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => {
            let message = db_err.message();
            message.contains("no such column: deployment_updates.user_id")
                || message.contains("no such column: user_id")
                || message.contains("has no column named user_id")
        }
        _ => false,
    }
}

pub async fn list_updates(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
) -> Result<Vec<DeploymentUpdate>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT deployment_updates.id,
               deployment_updates.tenant_id,
               deployment_updates.deployment_id,
               deployment_updates.user_id,
               deployment_updates.work_date,
               deployment_updates.start_time,
               deployment_updates.end_time,
               deployment_updates.hours_worked,
               deployment_updates.notes,
               deployment_updates.is_placeholder,
               COALESCE(users.email, '') as user_email
        FROM deployment_updates
        LEFT JOIN users ON users.id = deployment_updates.user_id
        WHERE deployment_updates.tenant_id = ? AND deployment_updates.deployment_id = ?
        ORDER BY deployment_updates.work_date DESC, deployment_updates.id DESC
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .fetch_all(&db.0)
    .await;

    match rows {
        Ok(rows) => Ok(rows
            .into_iter()
            .map(|row| DeploymentUpdate {
                id: row.get("id"),
                tenant_id: row.get("tenant_id"),
                deployment_id: row.get("deployment_id"),
                user_id: row.get("user_id"),
                user_email: row.get("user_email"),
                work_date: row.get("work_date"),
                start_time: row.get("start_time"),
                end_time: row.get("end_time"),
                hours_worked: row.get("hours_worked"),
                notes: row.get("notes"),
                is_placeholder: row.get::<i64, _>("is_placeholder") != 0,
            })
            .collect()),
        Err(err) if is_missing_user_id(&err) => {
            let rows = sqlx::query(
                r#"
                SELECT id, tenant_id, deployment_id, work_date, start_time, end_time, hours_worked, notes
                FROM deployment_updates
                WHERE deployment_updates.tenant_id = ? AND deployment_updates.deployment_id = ?
                ORDER BY deployment_updates.work_date DESC, deployment_updates.id DESC
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
                    user_id: None,
                    user_email: "".to_string(),
                    work_date: row.get("work_date"),
                    start_time: row.get("start_time"),
                    end_time: row.get("end_time"),
                    hours_worked: row.get("hours_worked"),
                    notes: row.get("notes"),
                    is_placeholder: false,
                })
                .collect())
        }
        Err(err) => Err(err),
    }
}

pub async fn list_updates_for_user(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    user_id: i64,
) -> Result<Vec<DeploymentUpdate>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT deployment_updates.id,
               deployment_updates.tenant_id,
               deployment_updates.deployment_id,
               deployment_updates.user_id,
               deployment_updates.work_date,
               deployment_updates.start_time,
               deployment_updates.end_time,
               deployment_updates.hours_worked,
               deployment_updates.notes,
               deployment_updates.is_placeholder,
               COALESCE(users.email, '') as user_email
        FROM deployment_updates
        LEFT JOIN users ON users.id = deployment_updates.user_id
        WHERE deployment_updates.tenant_id = ? AND deployment_updates.deployment_id = ? AND deployment_updates.user_id = ?
        ORDER BY deployment_updates.work_date DESC, deployment_updates.id DESC
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .bind(user_id)
    .fetch_all(&db.0)
    .await;

    match rows {
        Ok(rows) => Ok(rows
            .into_iter()
            .map(|row| DeploymentUpdate {
                id: row.get("id"),
                tenant_id: row.get("tenant_id"),
                deployment_id: row.get("deployment_id"),
                user_id: row.get("user_id"),
                user_email: row.get("user_email"),
                work_date: row.get("work_date"),
                start_time: row.get("start_time"),
                end_time: row.get("end_time"),
                hours_worked: row.get("hours_worked"),
                notes: row.get("notes"),
                is_placeholder: row.get::<i64, _>("is_placeholder") != 0,
            })
            .collect()),
        Err(err) if is_missing_user_id(&err) => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

pub async fn find_update_by_id(
    db: &Db,
    tenant_id: i64,
    update_id: i64,
) -> Result<Option<DeploymentUpdate>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id,
               tenant_id,
               deployment_id,
               user_id,
               work_date,
               start_time,
               end_time,
               hours_worked,
               notes,
               is_placeholder
        FROM deployment_updates
        WHERE tenant_id = ? AND id = ?
        "#,
    )
    .bind(tenant_id)
    .bind(update_id)
    .fetch_optional(&db.0)
    .await;

    match row {
        Ok(row) => Ok(row.map(|row| DeploymentUpdate {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            deployment_id: row.get("deployment_id"),
            user_id: row.get("user_id"),
            user_email: "".to_string(),
            work_date: row.get("work_date"),
            start_time: row.get("start_time"),
            end_time: row.get("end_time"),
            hours_worked: row.get("hours_worked"),
            notes: row.get("notes"),
            is_placeholder: row.get::<i64, _>("is_placeholder") != 0,
        })),
        Err(err) if is_missing_user_id(&err) => {
            let row = sqlx::query(
                r#"
                SELECT id,
                       tenant_id,
                       deployment_id,
                       work_date,
                       start_time,
                       end_time,
                       hours_worked,
                       notes
                FROM deployment_updates
                WHERE tenant_id = ? AND id = ?
                "#,
            )
            .bind(tenant_id)
            .bind(update_id)
            .fetch_optional(&db.0)
            .await?;

            Ok(row.map(|row| DeploymentUpdate {
                id: row.get("id"),
                tenant_id: row.get("tenant_id"),
                deployment_id: row.get("deployment_id"),
                user_id: None,
                user_email: "".to_string(),
                work_date: row.get("work_date"),
                start_time: row.get("start_time"),
                end_time: row.get("end_time"),
                hours_worked: row.get("hours_worked"),
                notes: row.get("notes"),
                is_placeholder: false,
            }))
        }
        Err(err) => Err(err),
    }
}

pub async fn find_update_by_date(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    work_date: &str,
) -> Result<Option<DeploymentUpdate>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT deployment_updates.id,
               deployment_updates.tenant_id,
               deployment_updates.deployment_id,
               deployment_updates.user_id,
               deployment_updates.work_date,
               deployment_updates.start_time,
               deployment_updates.end_time,
               deployment_updates.hours_worked,
               deployment_updates.notes,
               deployment_updates.is_placeholder,
               COALESCE(users.email, '') as user_email
        FROM deployment_updates
        LEFT JOIN users ON users.id = deployment_updates.user_id
        WHERE deployment_updates.tenant_id = ? AND deployment_updates.deployment_id = ? AND deployment_updates.work_date = ?
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .bind(work_date)
    .fetch_optional(&db.0)
    .await;

    match row {
        Ok(row) => Ok(row.map(|row| DeploymentUpdate {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            deployment_id: row.get("deployment_id"),
            user_id: row.get("user_id"),
            user_email: row.get("user_email"),
            work_date: row.get("work_date"),
            start_time: row.get("start_time"),
            end_time: row.get("end_time"),
            hours_worked: row.get("hours_worked"),
            notes: row.get("notes"),
            is_placeholder: row.get::<i64, _>("is_placeholder") != 0,
        })),
        Err(err) if is_missing_user_id(&err) => {
            let row = sqlx::query(
                r#"
                SELECT id, tenant_id, deployment_id, work_date, start_time, end_time, hours_worked, notes
                FROM deployment_updates
                WHERE deployment_updates.tenant_id = ? AND deployment_updates.deployment_id = ? AND deployment_updates.work_date = ?
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
                user_id: None,
                user_email: "".to_string(),
                work_date: row.get("work_date"),
                start_time: row.get("start_time"),
                end_time: row.get("end_time"),
                hours_worked: row.get("hours_worked"),
                notes: row.get("notes"),
                is_placeholder: false,
            }))
        }
        Err(err) => Err(err),
    }
}

pub async fn find_update_by_date_for_user(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    user_id: i64,
    work_date: &str,
) -> Result<Option<DeploymentUpdate>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT deployment_updates.id,
               deployment_updates.tenant_id,
               deployment_updates.deployment_id,
               deployment_updates.user_id,
               deployment_updates.work_date,
               deployment_updates.start_time,
               deployment_updates.end_time,
               deployment_updates.hours_worked,
               deployment_updates.notes,
               deployment_updates.is_placeholder,
               COALESCE(users.email, '') as user_email
        FROM deployment_updates
        LEFT JOIN users ON users.id = deployment_updates.user_id
        WHERE deployment_updates.tenant_id = ? AND deployment_updates.deployment_id = ?
          AND deployment_updates.user_id = ? AND deployment_updates.work_date = ?
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .bind(user_id)
    .bind(work_date)
    .fetch_optional(&db.0)
    .await;

    match row {
        Ok(row) => Ok(row.map(|row| DeploymentUpdate {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            deployment_id: row.get("deployment_id"),
            user_id: row.get("user_id"),
            user_email: row.get("user_email"),
            work_date: row.get("work_date"),
            start_time: row.get("start_time"),
            end_time: row.get("end_time"),
            hours_worked: row.get("hours_worked"),
            notes: row.get("notes"),
            is_placeholder: row.get::<i64, _>("is_placeholder") != 0,
        })),
        Err(err) if is_missing_user_id(&err) => {
            let row = sqlx::query(
                r#"
                SELECT id, tenant_id, deployment_id, work_date, start_time, end_time, hours_worked, notes
                FROM deployment_updates
                WHERE deployment_updates.tenant_id = ? AND deployment_updates.deployment_id = ?
                  AND deployment_updates.user_id = ? AND deployment_updates.work_date = ?
                "#,
            )
            .bind(tenant_id)
            .bind(deployment_id)
            .bind(user_id)
            .bind(work_date)
            .fetch_optional(&db.0)
            .await?;

            Ok(row.map(|row| DeploymentUpdate {
                id: row.get("id"),
                tenant_id: row.get("tenant_id"),
                deployment_id: row.get("deployment_id"),
                user_id: Some(user_id),
                user_email: "".to_string(),
                work_date: row.get("work_date"),
                start_time: row.get("start_time"),
                end_time: row.get("end_time"),
                hours_worked: row.get("hours_worked"),
                notes: row.get("notes"),
                is_placeholder: false,
            }))
        }
        Err(err) => Err(err),
    }
}

pub async fn create_update(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    user_id: i64,
    work_date: &str,
    start_time: &str,
    end_time: &str,
    hours_worked: f64,
    notes: &str,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO deployment_updates
            (tenant_id, deployment_id, user_id, work_date, start_time, end_time, hours_worked, notes, is_placeholder)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0)
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .bind(user_id)
    .bind(work_date)
    .bind(start_time)
    .bind(end_time)
    .bind(hours_worked)
    .bind(notes)
    .execute(&db.0)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(err) if is_missing_user_id(&err) => {
            sqlx::query(
                r#"
                INSERT INTO deployment_updates
                    (tenant_id, deployment_id, work_date, start_time, end_time, hours_worked, notes, is_placeholder)
                VALUES (?, ?, ?, ?, ?, ?, ?, 0)
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
            .await
            .map(|_| ())
        }
        Err(err) => Err(err),
    }?;
    Ok(())
}

pub async fn count_updates_for_crews(
    db: &Db,
    tenant_id: i64,
    crew_ids: &[i64],
) -> Result<i64, sqlx::Error> {
    if crew_ids.is_empty() {
        return Ok(0);
    }
    let placeholders = crew_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
        SELECT COUNT(*) as count
        FROM deployment_updates
        JOIN deployments ON deployment_updates.deployment_id = deployments.id
        WHERE deployment_updates.tenant_id = ? AND deployments.crew_id IN ({})
        "#,
        placeholders
    );
    let mut query = sqlx::query(&sql).bind(tenant_id);
    for crew_id in crew_ids {
        query = query.bind(crew_id);
    }
    let row = query.fetch_one(&db.0).await?;
    Ok(row.get("count"))
}

pub async fn update_update(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    update_id: i64,
    work_date: &str,
    start_time: &str,
    end_time: &str,
    hours_worked: f64,
    notes: &str,
    is_placeholder: bool,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE deployment_updates
        SET work_date = ?, start_time = ?, end_time = ?, hours_worked = ?, notes = ?, is_placeholder = ?
        WHERE tenant_id = ? AND deployment_id = ? AND id = ?
        "#,
    )
    .bind(work_date)
    .bind(start_time)
    .bind(end_time)
    .bind(hours_worked)
    .bind(notes)
    .bind(if is_placeholder { 1 } else { 0 })
    .bind(tenant_id)
    .bind(deployment_id)
    .bind(update_id)
    .execute(&db.0)
    .await?;

    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

pub async fn delete_update(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    update_id: i64,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM deployment_updates WHERE tenant_id = ? AND deployment_id = ? AND id = ?",
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .bind(update_id)
    .execute(&db.0)
    .await?;

    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

pub async fn create_placeholder_update(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    user_id: i64,
    work_date: &str,
    start_time: &str,
    end_time: &str,
    hours_worked: f64,
    notes: &str,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO deployment_updates
            (tenant_id, deployment_id, user_id, work_date, start_time, end_time, hours_worked, notes, is_placeholder)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1)
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .bind(user_id)
    .bind(work_date)
    .bind(start_time)
    .bind(end_time)
    .bind(hours_worked)
    .bind(notes)
    .execute(&db.0)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(err) if is_missing_user_id(&err) => {
            sqlx::query(
                r#"
                INSERT INTO deployment_updates
                    (tenant_id, deployment_id, work_date, start_time, end_time, hours_worked, notes, is_placeholder)
                VALUES (?, ?, ?, ?, ?, ?, ?, 1)
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
            .await
            .map(|_| ())
        }
        Err(err) => Err(err),
    }?;
    Ok(())
}

pub async fn count_updates_missing_user_id(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM deployment_updates
        WHERE deployment_updates.tenant_id = ? AND deployment_updates.deployment_id = ? AND deployment_updates.user_id IS NULL
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .fetch_one(&db.0)
    .await;

    match row {
        Ok(row) => Ok(row.get("count")),
        Err(err) if is_missing_user_id(&err) => Ok(0),
        Err(err) => Err(err),
    }
}

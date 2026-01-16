use rocket_db_pools::sqlx::{self, Row};

use crate::models::Deployment;
use crate::Db;

pub struct DeploymentRow {
    pub id: i64,
    pub client_id: i64,
    pub client_name: String,
    pub client_currency: String,
    pub crew_id: i64,
    pub crew_name: String,
    pub start_at: String,
    pub end_at: String,
    pub fee_per_hour: f64,
    pub info: String,
    pub status: String,
    pub deployment_type: String,
}

pub struct DeploymentCrewRow {
    pub deployment_id: i64,
    pub crew_id: i64,
    pub crew_name: String,
    pub client_name: String,
}

pub struct DeploymentLocationRow {
    pub deployment_id: i64,
    pub crew_id: i64,
    pub client_id: i64,
    pub crew_name: String,
    pub client_name: String,
    pub latitude: String,
    pub longitude: String,
}

pub async fn list_deployments_with_names(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<DeploymentRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            deployments.id as deployment_id,
            deployments.client_id as client_id,
            clients.company_name as client_name,
            clients.currency as client_currency,
            deployments.crew_id as crew_id,
            crews.name as crew_name,
            deployments.start_at as start_at,
            deployments.end_at as end_at,
            deployments.fee_per_hour as fee_per_hour,
            deployments.info as info,
            deployments.status as status,
            deployments.deployment_type as deployment_type
        FROM deployments
        JOIN clients ON deployments.client_id = clients.id
        JOIN crews ON deployments.crew_id = crews.id
        WHERE deployments.tenant_id = ?
        ORDER BY clients.company_name ASC, deployments.start_at DESC, deployments.id DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| DeploymentRow {
            id: row.get("deployment_id"),
            client_id: row.get("client_id"),
            client_name: row.get("client_name"),
            client_currency: row.get("client_currency"),
            crew_id: row.get("crew_id"),
            crew_name: row.get("crew_name"),
            start_at: row.get("start_at"),
            end_at: row.get("end_at"),
            fee_per_hour: row.get("fee_per_hour"),
            info: row.get("info"),
            status: row.get("status"),
            deployment_type: row.get("deployment_type"),
        })
        .collect())
}

pub async fn list_deployments_with_names_by_client(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<Vec<DeploymentRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            deployments.id as deployment_id,
            deployments.client_id as client_id,
            clients.company_name as client_name,
            clients.currency as client_currency,
            deployments.crew_id as crew_id,
            crews.name as crew_name,
            deployments.start_at as start_at,
            deployments.end_at as end_at,
            deployments.fee_per_hour as fee_per_hour,
            deployments.info as info,
            deployments.status as status,
            deployments.deployment_type as deployment_type
        FROM deployments
        JOIN clients ON deployments.client_id = clients.id
        JOIN crews ON deployments.crew_id = crews.id
        WHERE deployments.tenant_id = ? AND deployments.client_id = ?
        ORDER BY deployments.start_at DESC, deployments.id DESC
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| DeploymentRow {
            id: row.get("deployment_id"),
            client_id: row.get("client_id"),
            client_name: row.get("client_name"),
            client_currency: row.get("client_currency"),
            crew_id: row.get("crew_id"),
            crew_name: row.get("crew_name"),
            start_at: row.get("start_at"),
            end_at: row.get("end_at"),
            fee_per_hour: row.get("fee_per_hour"),
            info: row.get("info"),
            status: row.get("status"),
            deployment_type: row.get("deployment_type"),
        })
        .collect())
}

pub async fn list_deployment_crews_by_ids(
    db: &Db,
    tenant_id: i64,
    deployment_ids: &[i64],
) -> Result<Vec<DeploymentCrewRow>, sqlx::Error> {
    if deployment_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = deployment_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
        SELECT deployments.id as deployment_id,
               deployments.crew_id as crew_id,
               crews.name as crew_name,
               clients.company_name as client_name
        FROM deployments
        JOIN crews ON deployments.crew_id = crews.id
        JOIN clients ON deployments.client_id = clients.id
        WHERE deployments.tenant_id = ? AND deployments.id IN ({})
        "#,
        placeholders
    );
    let mut query = sqlx::query(&sql).bind(tenant_id);
    for deployment_id in deployment_ids {
        query = query.bind(deployment_id);
    }
    let rows = query.fetch_all(&db.0).await?;
    Ok(rows
        .into_iter()
        .map(|row| DeploymentCrewRow {
            deployment_id: row.get("deployment_id"),
            crew_id: row.get("crew_id"),
            crew_name: row.get("crew_name"),
            client_name: row.get("client_name"),
        })
        .collect())
}

pub async fn list_deployment_locations_by_ids(
    db: &Db,
    tenant_id: i64,
    deployment_ids: &[i64],
) -> Result<Vec<DeploymentLocationRow>, sqlx::Error> {
    if deployment_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = deployment_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
        SELECT deployments.id as deployment_id,
               deployments.crew_id as crew_id,
               clients.id as client_id,
               crews.name as crew_name,
               clients.company_name as client_name,
               clients.latitude as latitude,
               clients.longitude as longitude
        FROM deployments
        JOIN crews ON deployments.crew_id = crews.id
        JOIN clients ON deployments.client_id = clients.id
        WHERE deployments.tenant_id = ? AND deployments.id IN ({})
        "#,
        placeholders
    );
    let mut query = sqlx::query(&sql).bind(tenant_id);
    for deployment_id in deployment_ids {
        query = query.bind(deployment_id);
    }
    let rows = query.fetch_all(&db.0).await?;
    Ok(rows
        .into_iter()
        .map(|row| DeploymentLocationRow {
            deployment_id: row.get("deployment_id"),
            crew_id: row.get("crew_id"),
            client_id: row.get("client_id"),
            crew_name: row.get("crew_name"),
            client_name: row.get("client_name"),
            latitude: row.get("latitude"),
            longitude: row.get("longitude"),
        })
        .collect())
}

pub async fn list_deployments_with_names_for_crews(
    db: &Db,
    tenant_id: i64,
    crew_ids: &[i64],
) -> Result<Vec<DeploymentRow>, sqlx::Error> {
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
        SELECT
            deployments.id as deployment_id,
            deployments.client_id as client_id,
            clients.company_name as client_name,
            clients.currency as client_currency,
            deployments.crew_id as crew_id,
            crews.name as crew_name,
            deployments.start_at as start_at,
            deployments.end_at as end_at,
            deployments.fee_per_hour as fee_per_hour,
            deployments.info as info,
            deployments.status as status,
            deployments.deployment_type as deployment_type
        FROM deployments
        JOIN clients ON deployments.client_id = clients.id
        JOIN crews ON deployments.crew_id = crews.id
        WHERE deployments.tenant_id = ? AND deployments.crew_id IN ({})
        ORDER BY clients.company_name ASC, deployments.start_at DESC, deployments.id DESC
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
        .map(|row| DeploymentRow {
            id: row.get("deployment_id"),
            client_id: row.get("client_id"),
            client_name: row.get("client_name"),
            crew_id: row.get("crew_id"),
            crew_name: row.get("crew_name"),
            start_at: row.get("start_at"),
            end_at: row.get("end_at"),
            fee_per_hour: row.get("fee_per_hour"),
            info: row.get("info"),
            status: row.get("status"),
            client_currency: row.get("client_currency"),
            deployment_type: row.get("deployment_type"),
        })
        .collect())
}

pub async fn create_deployment(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    crew_id: i64,
    start_at: &str,
    end_at: &str,
    fee_per_hour: f64,
    info: &str,
    status: &str,
    deployment_type: &str,
    required_skills: &str,
    compatibility_pref: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO deployments
            (tenant_id, client_id, crew_id, start_at, end_at, fee_per_hour, info, status, deployment_type, required_skills, compatibility_pref)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(crew_id)
    .bind(start_at)
    .bind(end_at)
    .bind(fee_per_hour)
    .bind(info)
    .bind(status)
    .bind(deployment_type)
    .bind(required_skills)
    .bind(compatibility_pref)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn find_deployment_by_id(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
) -> Result<Option<Deployment>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, tenant_id, client_id, crew_id, start_at, end_at, fee_per_hour, info, status, deployment_type, required_skills, compatibility_pref
        FROM deployments
        WHERE id = ? AND tenant_id = ?
        "#,
    )
    .bind(deployment_id)
    .bind(tenant_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| Deployment {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        client_id: row.get("client_id"),
        crew_id: row.get("crew_id"),
        start_at: row.get("start_at"),
        end_at: row.get("end_at"),
        fee_per_hour: row.get("fee_per_hour"),
        info: row.get("info"),
        status: row.get("status"),
        deployment_type: row.get("deployment_type"),
        required_skills: row.get("required_skills"),
        compatibility_pref: row.get("compatibility_pref"),
    }))
}

pub async fn find_deployment_label(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT clients.company_name as client_name, crews.name as crew_name
        FROM deployments
        JOIN clients ON deployments.client_id = clients.id
        JOIN crews ON deployments.crew_id = crews.id
        WHERE deployments.tenant_id = ? AND deployments.id = ?
        "#,
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| {
        let client_name: String = row.get("client_name");
        let crew_name: String = row.get("crew_name");
        format!("{client_name} - {crew_name}")
    }))
}

pub async fn update_deployment(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    client_id: i64,
    crew_id: i64,
    start_at: &str,
    end_at: &str,
    fee_per_hour: f64,
    info: &str,
    status: &str,
    deployment_type: &str,
    required_skills: &str,
    compatibility_pref: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE deployments
        SET client_id = ?, crew_id = ?, start_at = ?, end_at = ?, fee_per_hour = ?, info = ?, status = ?, deployment_type = ?, required_skills = ?, compatibility_pref = ?
        WHERE id = ? AND tenant_id = ?
        "#,
    )
    .bind(client_id)
    .bind(crew_id)
    .bind(start_at)
    .bind(end_at)
    .bind(fee_per_hour)
    .bind(info)
    .bind(status)
    .bind(deployment_type)
    .bind(required_skills)
    .bind(compatibility_pref)
    .bind(deployment_id)
    .bind(tenant_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn update_deployment_status(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE deployments
        SET status = ?
        WHERE id = ? AND tenant_id = ?
        "#,
    )
    .bind(status)
    .bind(deployment_id)
    .bind(tenant_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn delete_deployment(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM deployments WHERE id = ? AND tenant_id = ?")
        .bind(deployment_id)
        .bind(tenant_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

pub async fn count_deployments_by_client(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM deployments WHERE tenant_id = ? AND client_id = ?",
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_one(&db.0)
    .await?;
    Ok(row.get("count"))
}

pub async fn list_deployments(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<Deployment>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, tenant_id, client_id, crew_id, start_at, end_at, fee_per_hour, info, status, deployment_type, required_skills, compatibility_pref
        FROM deployments
        WHERE tenant_id = ?
        ORDER BY id DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Deployment {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            client_id: row.get("client_id"),
            crew_id: row.get("crew_id"),
        start_at: row.get("start_at"),
        end_at: row.get("end_at"),
        fee_per_hour: row.get("fee_per_hour"),
        info: row.get("info"),
        status: row.get("status"),
        deployment_type: row.get("deployment_type"),
        required_skills: row.get("required_skills"),
        compatibility_pref: row.get("compatibility_pref"),
    })
        .collect())
}

pub async fn list_active_deployment_ids(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<i64>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id
        FROM deployments
        WHERE tenant_id = ? AND status = 'Active'
        "#,
    )
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows.into_iter().map(|row| row.get("id")).collect())
}

pub async fn count_deployments(db: &Db, tenant_id: i64) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM deployments WHERE tenant_id = ?")
        .bind(tenant_id)
        .fetch_one(&db.0)
        .await?;
    Ok(row.get("count"))
}

pub async fn count_deployments_total(db: &Db) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM deployments")
        .fetch_one(&db.0)
        .await?;
    Ok(row.get("count"))
}

pub async fn count_deployments_for_crews(
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
        "SELECT COUNT(*) as count FROM deployments WHERE tenant_id = ? AND crew_id IN ({})",
        placeholders
    );
    let mut query = sqlx::query(&sql).bind(tenant_id);
    for crew_id in crew_ids {
        query = query.bind(crew_id);
    }
    let row = query.fetch_one(&db.0).await?;
    Ok(row.get("count"))
}

pub async fn count_deployments_by_status(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT status, COUNT(*) as count FROM deployments WHERE tenant_id = ? GROUP BY status",
    )
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.get("status"), row.get("count")))
        .collect())
}

pub async fn count_deployments_by_status_all(
    db: &Db,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT status, COUNT(*) as count FROM deployments GROUP BY status",
    )
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.get("status"), row.get("count")))
        .collect())
}

pub async fn count_deployments_by_status_for_crews(
    db: &Db,
    tenant_id: i64,
    crew_ids: &[i64],
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    if crew_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = crew_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT status, COUNT(*) as count FROM deployments WHERE tenant_id = ? AND crew_id IN ({}) GROUP BY status",
        placeholders
    );
    let mut query = sqlx::query(&sql).bind(tenant_id);
    for crew_id in crew_ids {
        query = query.bind(crew_id);
    }
    let rows = query.fetch_all(&db.0).await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.get("status"), row.get("count")))
        .collect())
}

pub async fn count_new_deployments_today(
    db: &Db,
    tenant_id: i64,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM deployments
        WHERE tenant_id = ? AND date(created_at) = date('now')
        "#,
    )
    .bind(tenant_id)
    .fetch_one(&db.0)
    .await?;
    Ok(row.get("count"))
}

pub async fn count_new_deployments_today_all(db: &Db) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM deployments WHERE date(created_at) = date('now')",
    )
    .fetch_one(&db.0)
    .await?;
    Ok(row.get("count"))
}

pub async fn list_recent_statuses_for_crew(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    limit: i64,
) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT status
        FROM deployments
        WHERE tenant_id = ? AND crew_id = ?
        ORDER BY id DESC
        LIMIT ?
        "#,
    )
    .bind(tenant_id)
    .bind(crew_id)
    .bind(limit)
    .fetch_all(&db.0)
    .await?;

    Ok(rows.into_iter().map(|row| row.get("status")).collect())
}

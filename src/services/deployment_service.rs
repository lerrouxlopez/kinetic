use rocket_db_pools::sqlx;

use crate::models::{
    Deployment,
    DeploymentClientGroup,
    DeploymentForm,
    DeploymentFormView,
    DeploymentSelect,
    DeploymentSummary,
};
use crate::repositories::deployment_repo;
use crate::Db;

pub struct DeploymentError {
    pub message: String,
    pub form: DeploymentFormView,
}

const STATUS_SCHEDULED: &str = "Scheduled";
const STATUS_ACTIVE: &str = "Active";
const STATUS_COMPLETED: &str = "Completed";
const STATUS_CANCELLED: &str = "Cancelled";

pub fn status_options() -> [&'static str; 4] {
    [STATUS_SCHEDULED, STATUS_ACTIVE, STATUS_COMPLETED, STATUS_CANCELLED]
}

pub async fn list_deployments_grouped(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<DeploymentClientGroup>, sqlx::Error> {
    let rows = deployment_repo::list_deployments_with_names(db, tenant_id).await?;
    Ok(group_deployments(rows))
}

pub async fn list_deployments_for_select(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<DeploymentSelect>, sqlx::Error> {
    let rows = deployment_repo::list_deployments_with_names(db, tenant_id).await?;
    Ok(rows
        .into_iter()
        .map(|row| DeploymentSelect {
            id: row.id,
            label: format!("{} - {}", row.client_name, row.crew_name),
        })
    .collect())
}

pub async fn list_deployments_grouped_for_crews(
    db: &Db,
    tenant_id: i64,
    crew_ids: &[i64],
) -> Result<Vec<DeploymentClientGroup>, sqlx::Error> {
    let rows = deployment_repo::list_deployments_with_names_for_crews(db, tenant_id, crew_ids)
        .await?;
    Ok(group_deployments(rows))
}

pub async fn list_deployments_for_select_for_crews(
    db: &Db,
    tenant_id: i64,
    crew_ids: &[i64],
) -> Result<Vec<DeploymentSelect>, sqlx::Error> {
    let rows = deployment_repo::list_deployments_with_names_for_crews(db, tenant_id, crew_ids)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| DeploymentSelect {
            id: row.id,
            label: format!("{} - {}", row.client_name, row.crew_name),
        })
        .collect())
}

pub async fn create_deployment(
    db: &Db,
    tenant_id: i64,
    form: DeploymentForm,
) -> Result<(), DeploymentError> {
    if form.client_id <= 0 {
        return Err(DeploymentError {
            message: "Client is required.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                form.start_at,
                form.end_at,
                form.fee_per_hour,
                form.info,
                form.status,
            ),
        });
    }
    if form.crew_id <= 0 {
        return Err(DeploymentError {
            message: "Crew is required.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                form.start_at,
                form.end_at,
                form.fee_per_hour,
                form.info,
                form.status,
            ),
        });
    }
    let start_input = form.start_at.trim().to_string();
    if start_input.is_empty() {
        return Err(DeploymentError {
            message: "Start time is required.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                "",
                form.end_at,
                form.fee_per_hour,
                form.info,
                form.status,
            ),
        });
    }
    let end_input = form.end_at.trim().to_string();
    if end_input.is_empty() {
        return Err(DeploymentError {
            message: "Finish time is required.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                start_input,
                "",
                form.fee_per_hour,
                form.info,
                form.status,
            ),
        });
    }

    let fee_per_hour = form.fee_per_hour;
    if fee_per_hour <= 0.0 {
        return Err(DeploymentError {
            message: "Fee per hour must be greater than 0.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                start_input,
                end_input,
                fee_per_hour,
                form.info,
                form.status,
            ),
        });
    }

    let info = form.info.trim().to_string();
    if info.is_empty() {
        return Err(DeploymentError {
            message: "Deployment information is required.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                start_input,
                end_input,
                fee_per_hour,
                "",
                form.status,
            ),
        });
    }

    let status = normalize_status(form.status);
    let start_at = normalize_datetime(&start_input);
    let end_at = normalize_datetime(&end_input);

    if let Err(err) = deployment_repo::create_deployment(
        db,
        tenant_id,
        form.client_id,
        form.crew_id,
        &start_at,
        &end_at,
        fee_per_hour,
        &info,
        &status,
    )
    .await
    {
        return Err(DeploymentError {
            message: format!("Unable to create deployment: {err}"),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                start_at,
                end_at,
                fee_per_hour,
                info,
                status,
            ),
        });
    }

    Ok(())
}

pub async fn update_deployment(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    form: DeploymentForm,
) -> Result<(), DeploymentError> {
    if form.client_id <= 0 {
        return Err(DeploymentError {
            message: "Client is required.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                form.start_at,
                form.end_at,
                form.fee_per_hour,
                form.info,
                form.status,
            ),
        });
    }
    if form.crew_id <= 0 {
        return Err(DeploymentError {
            message: "Crew is required.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                form.start_at,
                form.end_at,
                form.fee_per_hour,
                form.info,
                form.status,
            ),
        });
    }
    let start_input = form.start_at.trim().to_string();
    if start_input.is_empty() {
        return Err(DeploymentError {
            message: "Start time is required.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                "",
                form.end_at,
                form.fee_per_hour,
                form.info,
                form.status,
            ),
        });
    }
    let end_input = form.end_at.trim().to_string();
    if end_input.is_empty() {
        return Err(DeploymentError {
            message: "Finish time is required.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                start_input,
                "",
                form.fee_per_hour,
                form.info,
                form.status,
            ),
        });
    }

    let fee_per_hour = form.fee_per_hour;
    if fee_per_hour <= 0.0 {
        return Err(DeploymentError {
            message: "Fee per hour must be greater than 0.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                start_input,
                end_input,
                fee_per_hour,
                form.info,
                form.status,
            ),
        });
    }

    let info = form.info.trim().to_string();
    if info.is_empty() {
        return Err(DeploymentError {
            message: "Deployment information is required.".to_string(),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                start_input,
                end_input,
                fee_per_hour,
                "",
                form.status,
            ),
        });
    }

    let status = normalize_status(form.status);
    let start_at = normalize_datetime(&start_input);
    let end_at = normalize_datetime(&end_input);

    if let Err(err) = deployment_repo::update_deployment(
        db,
        tenant_id,
        deployment_id,
        form.client_id,
        form.crew_id,
        &start_at,
        &end_at,
        fee_per_hour,
        &info,
        &status,
    )
    .await
    {
        return Err(DeploymentError {
            message: format!("Unable to update deployment: {err}"),
            form: DeploymentFormView::new(
                form.client_id,
                form.crew_id,
                start_at,
                end_at,
                fee_per_hour,
                info,
                status,
            ),
        });
    }

    Ok(())
}

pub async fn find_deployment_by_id(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
) -> Result<Option<Deployment>, sqlx::Error> {
    deployment_repo::find_deployment_by_id(db, tenant_id, deployment_id).await
}

pub async fn delete_deployment(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
) -> Result<(), String> {
    deployment_repo::delete_deployment(db, tenant_id, deployment_id)
        .await
        .map_err(|err| format!("Unable to delete deployment: {err}"))
}

pub async fn count_deployments(db: &Db, tenant_id: i64) -> Result<i64, sqlx::Error> {
    deployment_repo::count_deployments(db, tenant_id).await
}

pub async fn count_deployments_all(db: &Db) -> Result<i64, sqlx::Error> {
    deployment_repo::count_deployments_total(db).await
}

pub async fn count_deployments_for_crews(
    db: &Db,
    tenant_id: i64,
    crew_ids: &[i64],
) -> Result<i64, sqlx::Error> {
    deployment_repo::count_deployments_for_crews(db, tenant_id, crew_ids).await
}

pub async fn count_deployments_by_status(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    deployment_repo::count_deployments_by_status(db, tenant_id).await
}

pub async fn count_deployments_by_status_all(
    db: &Db,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    deployment_repo::count_deployments_by_status_all(db).await
}

pub async fn count_deployments_by_status_for_crews(
    db: &Db,
    tenant_id: i64,
    crew_ids: &[i64],
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    deployment_repo::count_deployments_by_status_for_crews(db, tenant_id, crew_ids).await
}

fn normalize_status(input: String) -> String {
    let status = input.trim();
    for option in status_options() {
        if option.eq_ignore_ascii_case(status) {
            return option.to_string();
        }
    }
    STATUS_SCHEDULED.to_string()
}

fn normalize_datetime(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "".to_string();
    }
    if trimmed.contains('T') {
        return trimmed.replace('T', " ");
    }
    trimmed.to_string()
}

fn group_deployments(rows: Vec<deployment_repo::DeploymentRow>) -> Vec<DeploymentClientGroup> {
    let mut groups: Vec<DeploymentClientGroup> = Vec::new();

    for row in rows {
        if let Some(existing) = groups.iter_mut().find(|group| group.client_id == row.client_id)
        {
            existing.deployments.push(DeploymentSummary {
                id: row.id,
                crew_id: row.crew_id,
                crew_name: row.crew_name,
                start_at: row.start_at,
                end_at: row.end_at,
                fee_per_hour: row.fee_per_hour,
                info: row.info,
                status: row.status,
            });
            continue;
        }

        groups.push(DeploymentClientGroup {
            client_id: row.client_id,
            client_name: row.client_name,
            client_currency: row.client_currency,
            deployments: vec![DeploymentSummary {
                id: row.id,
                crew_id: row.crew_id,
                crew_name: row.crew_name,
                start_at: row.start_at,
                end_at: row.end_at,
                fee_per_hour: row.fee_per_hour,
                info: row.info,
                status: row.status,
            }],
        });
    }

    groups
}

pub fn calculated_fee(start_at: &str, end_at: &str, fee_per_hour: f64) -> f64 {
    if fee_per_hour <= 0.0 {
        return 0.0;
    }
    let start = parse_datetime(start_at);
    let end = parse_datetime(end_at);
    if start.is_none() || end.is_none() {
        return 0.0;
    }
    let start = start.unwrap();
    let end = end.unwrap();
    let mut minutes = (end - start).num_minutes();
    if minutes <= 0 {
        return 0.0;
    }

    let total_days = minutes / (60 * 24);
    minutes -= total_days * 60 * 24;
    let extra_minutes = minutes.min(8 * 60);
    let billable_minutes = (total_days * 8 * 60) + extra_minutes;
    let hours = billable_minutes as f64 / 60.0;
    (hours * fee_per_hour * 100.0).round() / 100.0
}

fn parse_datetime(value: &str) -> Option<chrono::NaiveDateTime> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M") {
        return Some(parsed);
    }
    chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S").ok()
}

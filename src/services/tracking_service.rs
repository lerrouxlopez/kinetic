use rocket_db_pools::sqlx;

use crate::models::{DeploymentUpdate, DeploymentUpdateForm, DeploymentUpdateFormView};
use crate::repositories::deployment_update_repo;
use crate::Db;

pub struct TrackingError {
    pub message: String,
    pub form: DeploymentUpdateFormView,
}

pub async fn list_updates(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
) -> Result<Vec<DeploymentUpdate>, sqlx::Error> {
    deployment_update_repo::list_updates(db, tenant_id, deployment_id).await
}

pub async fn count_updates_for_crews(
    db: &Db,
    tenant_id: i64,
    crew_ids: &[i64],
) -> Result<i64, sqlx::Error> {
    deployment_update_repo::count_updates_for_crews(db, tenant_id, crew_ids).await
}

pub async fn create_update(
    db: &Db,
    tenant_id: i64,
    form: DeploymentUpdateForm,
) -> Result<(), TrackingError> {
    if form.deployment_id <= 0 {
        return Err(TrackingError {
            message: "Deployment is required.".to_string(),
            form: DeploymentUpdateFormView::new(
                form.deployment_id,
                form.work_date,
                form.start_time,
                form.end_time,
                form.notes,
            ),
        });
    }

    let work_date = form.work_date.trim().to_string();
    if work_date.is_empty() {
        return Err(TrackingError {
            message: "Work date is required.".to_string(),
            form: DeploymentUpdateFormView::new(
                form.deployment_id,
                "",
                form.start_time,
                form.end_time,
                form.notes,
            ),
        });
    }

    let start_time = form.start_time.trim().to_string();
    let end_time = form.end_time.trim().to_string();
    if start_time.is_empty() || end_time.is_empty() {
        return Err(TrackingError {
            message: "Start and finish times are required.".to_string(),
            form: DeploymentUpdateFormView::new(
                form.deployment_id,
                work_date,
                start_time,
                end_time,
                form.notes,
            ),
        });
    }

    let start = parse_time(&start_time).ok_or_else(|| TrackingError {
        message: "Start time format is invalid.".to_string(),
        form: DeploymentUpdateFormView::new(
            form.deployment_id,
            work_date.clone(),
            start_time.clone(),
            end_time.clone(),
            form.notes.clone(),
        ),
    })?;
    let end = parse_time(&end_time).ok_or_else(|| TrackingError {
        message: "Finish time format is invalid.".to_string(),
        form: DeploymentUpdateFormView::new(
            form.deployment_id,
            work_date.clone(),
            start_time.clone(),
            end_time.clone(),
            form.notes.clone(),
        ),
    })?;

    let duration = end.signed_duration_since(start);
    let minutes = duration.num_minutes();
    if minutes <= 0 {
        return Err(TrackingError {
            message: "Finish time must be after start time.".to_string(),
            form: DeploymentUpdateFormView::new(
                form.deployment_id,
                work_date,
                start_time,
                end_time,
                form.notes,
            ),
        });
    }
    let hours_worked = (minutes as f64 / 60.0 * 100.0).round() / 100.0;

    if let Ok(Some(_)) = deployment_update_repo::find_update_by_date(
        db,
        tenant_id,
        form.deployment_id,
        &work_date,
    )
    .await
    {
        return Err(TrackingError {
            message: "An update for this work day already exists.".to_string(),
            form: DeploymentUpdateFormView::new(
                form.deployment_id,
                work_date,
                start_time,
                end_time,
                form.notes,
            ),
        });
    }

    let notes = form.notes.trim().to_string();
    if notes.is_empty() {
        return Err(TrackingError {
            message: "Update notes are required.".to_string(),
            form: DeploymentUpdateFormView::new(
                form.deployment_id,
                work_date,
                start_time,
                end_time,
                "",
            ),
        });
    }

    if let Err(err) = deployment_update_repo::create_update(
        db,
        tenant_id,
        form.deployment_id,
        &work_date,
        &start_time,
        &end_time,
        hours_worked,
        &notes,
    )
    .await
    {
        return Err(TrackingError {
            message: format!("Unable to save update: {err}"),
            form: DeploymentUpdateFormView::new(
                form.deployment_id,
                work_date,
                start_time,
                end_time,
                notes,
            ),
        });
    }

    Ok(())
}

fn parse_time(value: &str) -> Option<chrono::NaiveTime> {
    chrono::NaiveTime::parse_from_str(value, "%H:%M").ok()
}

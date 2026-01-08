use rocket_db_pools::sqlx;

use crate::models::{DeploymentUpdate, DeploymentUpdateForm, DeploymentUpdateFormView, WorkTimer};
use crate::repositories::{deployment_update_repo, work_timer_repo};
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

pub async fn list_updates_for_user(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    user_id: i64,
) -> Result<Vec<DeploymentUpdate>, sqlx::Error> {
    deployment_update_repo::list_updates_for_user(db, tenant_id, deployment_id, user_id).await
}

pub async fn find_update_by_id(
    db: &Db,
    tenant_id: i64,
    update_id: i64,
) -> Result<Option<DeploymentUpdate>, sqlx::Error> {
    deployment_update_repo::find_update_by_id(db, tenant_id, update_id).await
}

pub async fn count_updates_for_crews(
    db: &Db,
    tenant_id: i64,
    crew_ids: &[i64],
) -> Result<i64, sqlx::Error> {
    deployment_update_repo::count_updates_for_crews(db, tenant_id, crew_ids).await
}

pub async fn count_updates_missing_user_id(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
) -> Result<i64, sqlx::Error> {
    deployment_update_repo::count_updates_missing_user_id(db, tenant_id, deployment_id).await
}

pub async fn create_update(
    db: &Db,
    tenant_id: i64,
    user_id: i64,
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

    if let Ok(Some(existing)) = deployment_update_repo::find_update_by_date_for_user(
        db,
        tenant_id,
        form.deployment_id,
        user_id,
        &work_date,
    )
    .await
    {
        if existing.is_placeholder {
            if existing.user_id.is_some() && existing.user_id != Some(user_id) {
                return Err(TrackingError {
                    message: "This update belongs to another user.".to_string(),
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
            if let Err(err) = deployment_update_repo::update_update(
                db,
                tenant_id,
                form.deployment_id,
                existing.id,
                &work_date,
                &start_time,
                &end_time,
                hours_worked,
                &notes,
                false,
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
            return Ok(());
        }
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
    if notes == "NO REPORT SUBMITTED" {
        return Err(TrackingError {
            message: "Please enter a report before saving.".to_string(),
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
        user_id,
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

pub async fn update_update(
    db: &Db,
    tenant_id: i64,
    update_id: i64,
    form: DeploymentUpdateForm,
) -> Result<i64, TrackingError> {
    let existing = match deployment_update_repo::find_update_by_id(db, tenant_id, update_id).await {
        Ok(Some(update)) => update,
        Ok(None) => {
            return Err(TrackingError {
                message: "Update not found.".to_string(),
                form: DeploymentUpdateFormView::new(
                    form.deployment_id,
                    form.work_date,
                    form.start_time,
                    form.end_time,
                    form.notes,
                ),
            })
        }
        Err(err) => {
            return Err(TrackingError {
                message: format!("Unable to load update: {err}"),
                form: DeploymentUpdateFormView::new(
                    form.deployment_id,
                    form.work_date,
                    form.start_time,
                    form.end_time,
                    form.notes,
                ),
            })
        }
    };

    let deployment_id = existing.deployment_id;
    let work_date = form.work_date.trim().to_string();
    if work_date.is_empty() {
        return Err(TrackingError {
            message: "Work date is required.".to_string(),
            form: DeploymentUpdateFormView::new(
                deployment_id,
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
                deployment_id,
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
            deployment_id,
            work_date.clone(),
            start_time.clone(),
            end_time.clone(),
            form.notes.clone(),
        ),
    })?;
    let end = parse_time(&end_time).ok_or_else(|| TrackingError {
        message: "Finish time format is invalid.".to_string(),
        form: DeploymentUpdateFormView::new(
            deployment_id,
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
                deployment_id,
                work_date,
                start_time,
                end_time,
                form.notes,
            ),
        });
    }
    let hours_worked = (minutes as f64 / 60.0 * 100.0).round() / 100.0;

    if let Ok(Some(other)) = match existing.user_id {
        Some(user_id) => {
            deployment_update_repo::find_update_by_date_for_user(
                db,
                tenant_id,
                deployment_id,
                user_id,
                &work_date,
            )
            .await
        }
        None => deployment_update_repo::find_update_by_date(db, tenant_id, deployment_id, &work_date)
            .await,
    } {
        if other.id != update_id {
            return Err(TrackingError {
                message: "An update for this work day already exists.".to_string(),
                form: DeploymentUpdateFormView::new(
                    deployment_id,
                    work_date,
                    start_time,
                    end_time,
                    form.notes,
                ),
            });
        }
    }

    let notes = form.notes.trim().to_string();
    if notes.is_empty() {
        return Err(TrackingError {
            message: "Update notes are required.".to_string(),
            form: DeploymentUpdateFormView::new(
                deployment_id,
                work_date,
                start_time,
                end_time,
                "",
            ),
        });
    }

    if let Err(err) = deployment_update_repo::update_update(
        db,
        tenant_id,
        deployment_id,
        update_id,
        &work_date,
        &start_time,
        &end_time,
        hours_worked,
        &notes,
        if existing.is_placeholder && notes != "NO REPORT SUBMITTED" {
            false
        } else {
            existing.is_placeholder
        },
    )
    .await
    {
        return Err(TrackingError {
            message: format!("Unable to update: {err}"),
            form: DeploymentUpdateFormView::new(
                deployment_id,
                work_date,
                start_time,
                end_time,
                notes,
            ),
        });
    }

    Ok(deployment_id)
}

pub async fn delete_update(
    db: &Db,
    tenant_id: i64,
    update_id: i64,
) -> Result<i64, String> {
    let existing = deployment_update_repo::find_update_by_id(db, tenant_id, update_id)
        .await
        .map_err(|err| format!("Unable to load update: {err}"))?
        .ok_or_else(|| "Update not found.".to_string())?;
    deployment_update_repo::delete_update(db, tenant_id, existing.deployment_id, update_id)
        .await
        .map_err(|err| format!("Unable to delete update: {err}"))?;
    Ok(existing.deployment_id)
}

pub async fn start_timer(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    user_id: i64,
) -> Result<(), String> {
    if let Ok(Some(_)) = work_timer_repo::find_active_timer(db, tenant_id, user_id).await {
        return Err("You already have an active timer.".to_string());
    }
    let now = chrono::Local::now().naive_local();
    let start_at = now.format("%Y-%m-%d %H:%M").to_string();
    work_timer_repo::create_timer(db, tenant_id, deployment_id, user_id, &start_at)
        .await
        .map_err(|err| format!("Unable to start timer: {err}"))?;
    Ok(())
}

pub async fn stop_timer(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    user_id: i64,
) -> Result<(), String> {
    let timer = work_timer_repo::find_active_timer(db, tenant_id, user_id)
        .await
        .map_err(|err| format!("Unable to load timer: {err}"))?
        .ok_or_else(|| "No active timer found.".to_string())?;
    if timer.deployment_id != deployment_id {
        return Err("Active timer belongs to another deployment.".to_string());
    }
    let now = chrono::Local::now().naive_local();
    let mut end_at = now.format("%Y-%m-%d %H:%M").to_string();
    end_at = ensure_end_after_start(&timer.start_at, &end_at).unwrap_or(end_at);
    work_timer_repo::stop_timer(db, tenant_id, timer.id, &end_at)
        .await
        .map_err(|err| format!("Unable to stop timer: {err}"))?;
    maybe_create_placeholder_update(db, tenant_id, &timer.start_at, &end_at, timer.deployment_id, user_id)
        .await?;
    Ok(())
}

pub async fn active_timer(
    db: &Db,
    tenant_id: i64,
    user_id: i64,
) -> Result<Option<WorkTimer>, sqlx::Error> {
    work_timer_repo::find_active_timer(db, tenant_id, user_id).await
}

pub async fn close_stale_timers(db: &Db, tenant_id: i64) -> Result<(), sqlx::Error> {
    let cutoff = chrono::Local::now()
        .naive_local()
        .checked_sub_signed(chrono::Duration::hours(9))
        .unwrap_or_else(|| chrono::Local::now().naive_local())
        .format("%Y-%m-%d %H:%M")
        .to_string();
    let timers = work_timer_repo::list_stale_timers(db, tenant_id, &cutoff).await?;
    for timer in timers {
        if let Some(end_at) = add_hours(&timer.start_at, 9) {
            work_timer_repo::stop_timer(db, tenant_id, timer.id, &end_at).await?;
            let _ = maybe_create_placeholder_update(
                db,
                tenant_id,
                &timer.start_at,
                &end_at,
                timer.deployment_id,
                timer.user_id,
            )
            .await;
        }
    }
    Ok(())
}

async fn maybe_create_placeholder_update(
    db: &Db,
    tenant_id: i64,
    start_at: &str,
    end_at: &str,
    deployment_id: i64,
    user_id: i64,
) -> Result<(), String> {
    let (work_date, start_time) = split_datetime(start_at)
        .ok_or_else(|| "Invalid start time.".to_string())?;
    let (_, end_time) =
        split_datetime(end_at).ok_or_else(|| "Invalid finish time.".to_string())?;
    let start = parse_time(&start_time).ok_or_else(|| "Start time format is invalid.".to_string())?;
    let end = parse_time(&end_time).ok_or_else(|| "Finish time format is invalid.".to_string())?;
    let mut minutes = end.signed_duration_since(start).num_minutes();
    if minutes <= 0 {
        minutes = 1;
    }
    let hours_worked = (minutes as f64 / 60.0 * 100.0).round() / 100.0;
    if let Ok(Some(existing)) = deployment_update_repo::find_update_by_date_for_user(
        db,
        tenant_id,
        deployment_id,
        user_id,
        &work_date,
    )
    .await
    {
        if existing.is_placeholder {
            deployment_update_repo::update_update(
                db,
                tenant_id,
                deployment_id,
                existing.id,
                &work_date,
                &start_time,
                &end_time,
                hours_worked,
                "NO REPORT SUBMITTED",
                true,
            )
            .await
            .map_err(|err| format!("Unable to update placeholder: {err}"))?;
        }
        return Ok(());
    }
    deployment_update_repo::create_placeholder_update(
        db,
        tenant_id,
        deployment_id,
        user_id,
        &work_date,
        &start_time,
        &end_time,
        hours_worked,
        "NO REPORT SUBMITTED",
    )
    .await
    .map_err(|err| format!("Unable to create placeholder: {err}"))?;
    Ok(())
}

fn parse_time(value: &str) -> Option<chrono::NaiveTime> {
    chrono::NaiveTime::parse_from_str(value, "%H:%M").ok()
}

fn split_datetime(value: &str) -> Option<(String, String)> {
    let parts = value.trim().split_once(' ')?;
    Some((parts.0.to_string(), parts.1.to_string()))
}

fn add_hours(value: &str, hours: i64) -> Option<String> {
    let parsed = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").ok()?;
    let updated = parsed + chrono::Duration::hours(hours);
    Some(updated.format("%Y-%m-%d %H:%M").to_string())
}

fn ensure_end_after_start(start_at: &str, end_at: &str) -> Option<String> {
    let start = chrono::NaiveDateTime::parse_from_str(start_at, "%Y-%m-%d %H:%M").ok()?;
    let end = chrono::NaiveDateTime::parse_from_str(end_at, "%Y-%m-%d %H:%M").ok()?;
    if end <= start {
        let updated = start + chrono::Duration::minutes(1);
        return Some(updated.format("%Y-%m-%d %H:%M").to_string());
    }
    Some(end_at.to_string())
}

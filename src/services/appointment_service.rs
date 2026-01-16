use rocket_db_pools::sqlx;

use crate::models::{Appointment, AppointmentForm, AppointmentFormView};
use crate::repositories::appointment_repo;
use crate::services::workspace_service;
use crate::Db;

pub struct AppointmentError {
    pub message: String,
    pub form: AppointmentFormView,
}

const STATUS_SCHEDULED: &str = "Scheduled";
const STATUS_CONFIRMED: &str = "Confirmed";
const STATUS_ATTENDED: &str = "Attended";
const STATUS_CANCELLED: &str = "Cancelled";
const STATUS_NO_SHOW: &str = "No-Show";

pub fn status_options() -> [&'static str; 5] {
    [
        STATUS_SCHEDULED,
        STATUS_CONFIRMED,
        STATUS_ATTENDED,
        STATUS_CANCELLED,
        STATUS_NO_SHOW,
    ]
}

pub async fn list_appointments(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<Vec<Appointment>, sqlx::Error> {
    appointment_repo::list_appointments_by_client(db, tenant_id, client_id).await
}

pub async fn list_appointments_paged(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<Appointment>, sqlx::Error> {
    appointment_repo::list_appointments_by_client_paged(db, tenant_id, client_id, limit, offset).await
}

pub async fn count_appointments(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<i64, sqlx::Error> {
    appointment_repo::count_appointments_by_client(db, tenant_id, client_id).await
}

pub async fn count_appointments_total(
    db: &Db,
    tenant_id: i64,
) -> Result<i64, sqlx::Error> {
    appointment_repo::count_appointments(db, tenant_id).await
}

pub async fn count_appointments_all(db: &Db) -> Result<i64, sqlx::Error> {
    appointment_repo::count_appointments_total(db).await
}

pub async fn count_appointments_by_status(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    appointment_repo::count_appointments_by_status(db, tenant_id).await
}

pub async fn count_appointments_by_status_all(
    db: &Db,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    appointment_repo::count_appointments_by_status_all(db).await
}

pub async fn find_appointment(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    appointment_id: i64,
) -> Result<Option<Appointment>, sqlx::Error> {
    appointment_repo::find_appointment_by_id(db, tenant_id, client_id, appointment_id).await
}

pub async fn create_appointment(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    contact_id: i64,
    form: AppointmentForm,
) -> Result<(), AppointmentError> {
    let (plan_key, limits) = workspace_service::plan_limits_for_tenant(db, tenant_id).await;
    if let Some(limit) = limits.appointments_per_client {
        let existing = appointment_repo::count_appointments_by_client(db, tenant_id, client_id)
            .await
            .unwrap_or(0);
        if existing >= limit {
            let plan_name = workspace_service::plan_name(&plan_key);
            return Err(AppointmentError {
                message: format!(
                    "{plan_name} plan workspaces can have up to {limit} appointments per client. Upgrade to add more."
                ),
                form: AppointmentFormView::new(
                    form.title,
                    form.scheduled_for,
                    form.status,
                    form.notes,
                ),
            });
        }
    }
    let title = form.title.trim().to_string();
    let scheduled_for = form.scheduled_for.trim().to_string();
    let status = normalize_status(form.status);
    let notes = form.notes.trim().to_string();

    if title.is_empty() {
        return Err(AppointmentError {
            message: "Appointment title is required.".to_string(),
            form: AppointmentFormView::new("", scheduled_for, status, notes),
        });
    }

    if scheduled_for.is_empty() {
        return Err(AppointmentError {
            message: "Scheduled date/time is required.".to_string(),
            form: AppointmentFormView::new(title, "", status, notes),
        });
    }

    if let Err(err) = appointment_repo::create_appointment(
        db,
        tenant_id,
        client_id,
        contact_id,
        &title,
        &scheduled_for,
        &status,
        &notes,
    )
    .await
    {
        return Err(AppointmentError {
            message: format!("Unable to create appointment: {err}"),
            form: AppointmentFormView::new(title, scheduled_for, status, notes),
        });
    }

    Ok(())
}

pub async fn update_appointment(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    appointment_id: i64,
    form: AppointmentForm,
) -> Result<(), AppointmentError> {
    let title = form.title.trim().to_string();
    let scheduled_for = form.scheduled_for.trim().to_string();
    let status = normalize_status(form.status);
    let notes = form.notes.trim().to_string();

    if title.is_empty() {
        return Err(AppointmentError {
            message: "Appointment title is required.".to_string(),
            form: AppointmentFormView::new("", scheduled_for, status, notes),
        });
    }

    if scheduled_for.is_empty() {
        return Err(AppointmentError {
            message: "Scheduled date/time is required.".to_string(),
            form: AppointmentFormView::new(title, "", status, notes),
        });
    }

    if let Err(err) = appointment_repo::update_appointment(
        db,
        tenant_id,
        client_id,
        appointment_id,
        &title,
        &scheduled_for,
        &status,
        &notes,
    )
    .await
    {
        return Err(AppointmentError {
            message: format!("Unable to update appointment: {err}"),
            form: AppointmentFormView::new(title, scheduled_for, status, notes),
        });
    }

    Ok(())
}

pub async fn delete_appointment(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    appointment_id: i64,
) -> Result<(), String> {
    appointment_repo::delete_appointment(db, tenant_id, client_id, appointment_id)
        .await
        .map_err(|err| format!("Unable to delete appointment: {err}"))
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

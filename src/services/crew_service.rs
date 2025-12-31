use rocket_db_pools::sqlx;

use crate::models::{Crew, CrewForm, CrewFormView, CrewStats};
use crate::repositories::crew_repo;
use crate::Db;

pub struct CrewError {
    pub message: String,
    pub form: CrewFormView,
}

const STATUS_ACTIVE: &str = "Active";
const STATUS_IDLE: &str = "Idle";
const STATUS_ON_LEAVE: &str = "On Leave";

pub fn status_options() -> [&'static str; 3] {
    [STATUS_ACTIVE, STATUS_IDLE, STATUS_ON_LEAVE]
}

pub async fn list_crews(db: &Db, tenant_id: i64) -> Result<Vec<Crew>, sqlx::Error> {
    crew_repo::list_crews(db, tenant_id).await
}

pub async fn find_crew_by_id(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
) -> Result<Option<Crew>, sqlx::Error> {
    crew_repo::find_crew_by_id(db, tenant_id, crew_id).await
}

pub fn stats_from_crews(crews: &[Crew]) -> CrewStats {
    let mut active = 0;
    let mut idle = 0;
    let mut on_leave = 0;
    let mut members = 0;

    for crew in crews {
        members += crew.members_count;
        match crew.status.as_str() {
            STATUS_ACTIVE => active += 1,
            STATUS_IDLE => idle += 1,
            STATUS_ON_LEAVE => on_leave += 1,
            _ => {}
        }
    }

    CrewStats {
        total_crews: crews.len(),
        active_crews: active,
        idle_crews: idle,
        on_leave_crews: on_leave,
        total_members: members,
    }
}

pub async fn create_crew(
    db: &Db,
    tenant_id: i64,
    form: CrewForm,
) -> Result<(), CrewError> {
    let name = form.name.trim().to_string();
    let status = normalize_status(form.status);

    if name.is_empty() {
        return Err(CrewError {
            message: "Crew name is required.".to_string(),
            form: CrewFormView::new("", form.members_count, status),
        });
    }

    if form.members_count < 0 {
        return Err(CrewError {
            message: "Members count must be zero or more.".to_string(),
            form: CrewFormView::new(name, form.members_count, status),
        });
    }

    if let Err(err) = crew_repo::create_crew(db, tenant_id, &name, form.members_count, &status).await
    {
        return Err(CrewError {
            message: format!("Unable to create crew: {err}"),
            form: CrewFormView::new(name, form.members_count, status),
        });
    }

    Ok(())
}

pub async fn update_crew(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    form: CrewForm,
) -> Result<(), CrewError> {
    let name = form.name.trim().to_string();
    let status = normalize_status(form.status);

    if name.is_empty() {
        return Err(CrewError {
            message: "Crew name is required.".to_string(),
            form: CrewFormView::new("", form.members_count, status),
        });
    }

    if form.members_count < 0 {
        return Err(CrewError {
            message: "Members count must be zero or more.".to_string(),
            form: CrewFormView::new(name, form.members_count, status),
        });
    }

    if let Err(err) =
        crew_repo::update_crew(db, tenant_id, crew_id, &name, form.members_count, &status).await
    {
        return Err(CrewError {
            message: format!("Unable to update crew: {err}"),
            form: CrewFormView::new(name, form.members_count, status),
        });
    }

    Ok(())
}

pub async fn delete_crew(db: &Db, tenant_id: i64, crew_id: i64) -> Result<(), String> {
    crew_repo::delete_crew(db, tenant_id, crew_id)
        .await
        .map_err(|err| format!("Unable to delete crew: {err}"))
}

fn normalize_status(input: String) -> String {
    let status = input.trim();
    for option in status_options() {
        if option.eq_ignore_ascii_case(status) {
            return option.to_string();
        }
    }
    STATUS_ACTIVE.to_string()
}

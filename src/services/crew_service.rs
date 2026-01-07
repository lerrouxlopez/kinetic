use rocket_db_pools::sqlx;

use crate::models::{
    Crew,
    CrewForm,
    CrewFormView,
    CrewMember,
    CrewMemberForm,
    CrewMemberFormView,
    CrewStats,
};
use crate::repositories::{crew_member_repo, crew_repo, user_repo};
use crate::services::workspace_service;
use crate::Db;

pub struct CrewError {
    pub message: String,
    pub form: CrewFormView,
}

pub struct CrewMemberError {
    pub message: String,
    pub form: CrewMemberFormView,
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

pub async fn list_crews_all(db: &Db) -> Result<Vec<Crew>, sqlx::Error> {
    crew_repo::list_crews_all(db).await
}

pub async fn list_crews_paged(
    db: &Db,
    tenant_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<Crew>, sqlx::Error> {
    crew_repo::list_crews_paged(db, tenant_id, limit, offset).await
}

pub async fn count_crews(db: &Db, tenant_id: i64) -> Result<i64, sqlx::Error> {
    crew_repo::count_crews(db, tenant_id).await
}

pub async fn count_crews_all(db: &Db) -> Result<i64, sqlx::Error> {
    crew_repo::count_crews_all(db).await
}

pub async fn count_active_crews_all(db: &Db) -> Result<i64, sqlx::Error> {
    crew_repo::count_crews_by_status(db, STATUS_ACTIVE).await
}

pub async fn find_crew_by_id(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
) -> Result<Option<Crew>, sqlx::Error> {
    crew_repo::find_crew_by_id(db, tenant_id, crew_id).await
}

pub async fn list_members_paged(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<CrewMember>, sqlx::Error> {
    crew_member_repo::list_members_paged(db, tenant_id, crew_id, limit, offset).await
}

pub async fn count_members(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
) -> Result<i64, sqlx::Error> {
    crew_member_repo::count_members(db, tenant_id, crew_id).await
}

pub async fn find_member_by_id(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    member_id: i64,
) -> Result<Option<CrewMember>, sqlx::Error> {
    crew_member_repo::find_member_by_id(db, tenant_id, crew_id, member_id).await
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
    if workspace_service::is_free_plan(db, tenant_id).await {
        let limits = workspace_service::free_plan_limits(db).await;
        let limit = limits.crews.unwrap_or(2);
        let existing = crew_repo::count_crews(db, tenant_id).await.unwrap_or(0);
        if existing >= limit {
            return Err(CrewError {
                message: format!(
                    "Free plan workspaces can have up to {limit} crews. Upgrade to add more."
                ),
                form: CrewFormView::new(form.name, form.status),
            });
        }
    }
    let name = form.name.trim().to_string();
    let status = normalize_status(form.status);

    if name.is_empty() {
        return Err(CrewError {
            message: "Crew name is required.".to_string(),
            form: CrewFormView::new("", status),
        });
    }

    if let Err(err) = crew_repo::create_crew(db, tenant_id, &name, &status).await
    {
        return Err(CrewError {
            message: format!("Unable to create crew: {err}"),
            form: CrewFormView::new(name, status),
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
            form: CrewFormView::new("", status),
        });
    }

    if let Err(err) = crew_repo::update_crew(db, tenant_id, crew_id, &name, &status).await {
        return Err(CrewError {
            message: format!("Unable to update crew: {err}"),
            form: CrewFormView::new(name, status),
        });
    }

    Ok(())
}

pub async fn delete_crew(db: &Db, tenant_id: i64, crew_id: i64) -> Result<(), String> {
    crew_repo::delete_crew(db, tenant_id, crew_id)
        .await
        .map_err(|err| format!("Unable to delete crew: {err}"))
}

pub async fn create_member(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    form: CrewMemberForm,
) -> Result<(), CrewMemberError> {
    if workspace_service::is_free_plan(db, tenant_id).await {
        let limits = workspace_service::free_plan_limits(db).await;
        let limit = limits.members_per_crew.unwrap_or(5);
        let existing = crew_member_repo::count_members(db, tenant_id, crew_id)
            .await
            .unwrap_or(0);
        if existing >= limit {
            return Err(CrewMemberError {
                message: format!(
                    "Free plan workspaces can have up to {limit} members per crew. Upgrade to add more."
                ),
                form: CrewMemberFormView::new(form.user_id, form.name, form.phone, form.position),
            });
        }
    }
    let name = form.name.trim().to_string();
    if name.is_empty() {
        return Err(CrewMemberError {
            message: "Member name is required.".to_string(),
            form: CrewMemberFormView::new(form.user_id, "", form.phone, form.position),
        });
    }
    let phone = form.phone.trim().to_string();
    if phone.is_empty() {
        return Err(CrewMemberError {
            message: "Member phone is required.".to_string(),
            form: CrewMemberFormView::new(form.user_id, name, "", form.position),
        });
    }
    if form.user_id <= 0 {
        return Err(CrewMemberError {
            message: "User account is required.".to_string(),
            form: CrewMemberFormView::new(0, name, phone, form.position),
        });
    }
    let user = match user_repo::find_user_by_id(db, tenant_id, form.user_id).await {
        Ok(Some(user)) => user,
        _ => {
            return Err(CrewMemberError {
                message: "Selected user was not found.".to_string(),
                form: CrewMemberFormView::new(form.user_id, name, phone, form.position),
            })
        }
    };

    if let Err(err) = crew_member_repo::create_member(
        db,
        tenant_id,
        crew_id,
        Some(user.id),
        &name,
        &phone,
        &user.email,
        form.position.trim(),
    )
    .await
    {
        return Err(CrewMemberError {
            message: format!("Unable to create crew member: {err}"),
            form: CrewMemberFormView::new(form.user_id, name, phone, form.position),
        });
    }

    if let Err(err) = crew_repo::update_members_count(db, tenant_id, crew_id).await {
        return Err(CrewMemberError {
            message: format!("Unable to update crew members count: {err}"),
            form: CrewMemberFormView::new(form.user_id, name, phone, form.position),
        });
    }

    Ok(())
}

pub async fn update_member(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    member_id: i64,
    form: CrewMemberForm,
) -> Result<(), CrewMemberError> {
    let name = form.name.trim().to_string();
    if name.is_empty() {
        return Err(CrewMemberError {
            message: "Member name is required.".to_string(),
            form: CrewMemberFormView::new(form.user_id, "", form.phone, form.position),
        });
    }
    let phone = form.phone.trim().to_string();
    if phone.is_empty() {
        return Err(CrewMemberError {
            message: "Member phone is required.".to_string(),
            form: CrewMemberFormView::new(form.user_id, name, "", form.position),
        });
    }
    if form.user_id <= 0 {
        return Err(CrewMemberError {
            message: "User account is required.".to_string(),
            form: CrewMemberFormView::new(0, name, phone, form.position),
        });
    }
    let user = match user_repo::find_user_by_id(db, tenant_id, form.user_id).await {
        Ok(Some(user)) => user,
        _ => {
            return Err(CrewMemberError {
                message: "Selected user was not found.".to_string(),
                form: CrewMemberFormView::new(form.user_id, name, phone, form.position),
            })
        }
    };

    if let Err(err) = crew_member_repo::update_member(
        db,
        tenant_id,
        crew_id,
        member_id,
        Some(user.id),
        &name,
        &phone,
        &user.email,
        form.position.trim(),
    )
    .await
    {
        return Err(CrewMemberError {
            message: format!("Unable to update crew member: {err}"),
            form: CrewMemberFormView::new(form.user_id, name, phone, form.position),
        });
    }

    Ok(())
}

pub async fn delete_member(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    member_id: i64,
) -> Result<(), String> {
    crew_member_repo::delete_member(db, tenant_id, crew_id, member_id)
        .await
        .map_err(|err| format!("Unable to delete crew member: {err}"))?;
    crew_repo::update_members_count(db, tenant_id, crew_id)
        .await
        .map_err(|err| format!("Unable to update crew members count: {err}"))?;
    Ok(())
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

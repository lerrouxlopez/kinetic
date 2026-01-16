use rocket_db_pools::sqlx;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::models::{
    Crew,
    CrewForm,
    CrewFormView,
    CrewMember,
    CrewMemberForm,
    CrewMemberFormView,
    CrewStats,
    CrewRosterView,
};
use crate::repositories::{crew_member_repo, crew_repo, deployment_repo, user_repo};
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

pub struct CrewRecommendation {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub skill_matches: i64,
    pub compatibility_matches: i64,
    pub score: i64,
}

const STATUS_ACTIVE: &str = "Active";
const STATUS_IDLE: &str = "Idle";
const STATUS_ON_LEAVE: &str = "On Leave";
const AVAILABILITY_AVAILABLE: &str = "Available";
const AVAILABILITY_AWAY: &str = "Away";
const AVAILABILITY_UNAVAILABLE: &str = "Unavailable";
const RECENT_OUTCOME_LIMIT: i64 = 5;

pub fn status_options() -> [&'static str; 3] {
    [STATUS_ACTIVE, STATUS_IDLE, STATUS_ON_LEAVE]
}

pub fn availability_options() -> [&'static str; 3] {
    [
        AVAILABILITY_AVAILABLE,
        AVAILABILITY_AWAY,
        AVAILABILITY_UNAVAILABLE,
    ]
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

pub async fn list_idle_crews(
    db: &Db,
    tenant_id: i64,
    limit: i64,
) -> Result<Vec<Crew>, sqlx::Error> {
    crew_repo::list_idle_crews(db, tenant_id, limit).await
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
    let (plan_key, limits) = workspace_service::plan_limits_for_tenant(db, tenant_id).await;
    if let Some(limit) = limits.crews {
        let existing = crew_repo::count_crews(db, tenant_id).await.unwrap_or(0);
        if existing >= limit {
            let plan_name = workspace_service::plan_name(&plan_key);
            return Err(CrewError {
                message: format!(
                    "{plan_name} plan workspaces can have up to {limit} crews. Upgrade to add more."
                ),
                form: CrewFormView::new(
                    form.name,
                    form.status,
                    form.gear_score,
                    form.skill_tags,
                    form.compatibility_tags,
                ),
            });
        }
    }
    let name = form.name.trim().to_string();
    let status = normalize_status(form.status);
    let gear_score = normalize_gear_score(form.gear_score);
    let skill_tags = normalize_tags(form.skill_tags);
    let compatibility_tags = normalize_tags(form.compatibility_tags);

    if name.is_empty() {
        return Err(CrewError {
            message: "Crew name is required.".to_string(),
            form: CrewFormView::new("", status, gear_score, skill_tags, compatibility_tags),
        });
    }

    if let Err(err) = crew_repo::create_crew(
        db,
        tenant_id,
        &name,
        &status,
        gear_score,
        &skill_tags,
        &compatibility_tags,
    )
    .await
    {
        return Err(CrewError {
            message: format!("Unable to create crew: {err}"),
            form: CrewFormView::new(name, status, gear_score, skill_tags, compatibility_tags),
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
    let gear_score = normalize_gear_score(form.gear_score);
    let skill_tags = normalize_tags(form.skill_tags);
    let compatibility_tags = normalize_tags(form.compatibility_tags);

    if name.is_empty() {
        return Err(CrewError {
            message: "Crew name is required.".to_string(),
            form: CrewFormView::new("", status, gear_score, skill_tags, compatibility_tags),
        });
    }

    if let Err(err) = crew_repo::update_crew(
        db,
        tenant_id,
        crew_id,
        &name,
        &status,
        gear_score,
        &skill_tags,
        &compatibility_tags,
    )
    .await
    {
        return Err(CrewError {
            message: format!("Unable to update crew: {err}"),
            form: CrewFormView::new(name, status, gear_score, skill_tags, compatibility_tags),
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
    let (plan_key, limits) = workspace_service::plan_limits_for_tenant(db, tenant_id).await;
    if let Some(limit) = limits.members_per_crew {
        let existing = crew_member_repo::count_members(db, tenant_id, crew_id)
            .await
            .unwrap_or(0);
        if existing >= limit {
            let plan_name = workspace_service::plan_name(&plan_key);
            return Err(CrewMemberError {
                message: format!(
                    "{plan_name} plan workspaces can have up to {limit} members per crew. Upgrade to add more."
                ),
                form: CrewMemberFormView::new(
                    form.user_id,
                    form.name,
                    form.phone,
                    form.position,
                    form.availability_status.clone(),
                ),
            });
        }
    }
    let name = form.name.trim().to_string();
    let availability_status = normalize_availability(form.availability_status.clone());
    if name.is_empty() {
        return Err(CrewMemberError {
            message: "Member name is required.".to_string(),
            form: CrewMemberFormView::new(
                form.user_id,
                "",
                form.phone,
                form.position,
                availability_status,
            ),
        });
    }
    let phone = form.phone.trim().to_string();
    if phone.is_empty() {
        return Err(CrewMemberError {
            message: "Member phone is required.".to_string(),
            form: CrewMemberFormView::new(
                form.user_id,
                name,
                "",
                form.position,
                availability_status,
            ),
        });
    }
    if form.user_id <= 0 {
        return Err(CrewMemberError {
            message: "User account is required.".to_string(),
            form: CrewMemberFormView::new(0, name, phone, form.position, availability_status),
        });
    }
    let user = match user_repo::find_user_by_id(db, tenant_id, form.user_id).await {
        Ok(Some(user)) => user,
        _ => {
            return Err(CrewMemberError {
                message: "Selected user was not found.".to_string(),
                form: CrewMemberFormView::new(
                    form.user_id,
                    name,
                    phone,
                    form.position,
                    availability_status,
                ),
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
        &availability_status,
    )
    .await
    {
        return Err(CrewMemberError {
            message: format!("Unable to create crew member: {err}"),
            form: CrewMemberFormView::new(
                form.user_id,
                name,
                phone,
                form.position,
                availability_status,
            ),
        });
    }

    if let Err(err) = crew_repo::update_members_count(db, tenant_id, crew_id).await {
        return Err(CrewMemberError {
            message: format!("Unable to update crew members count: {err}"),
            form: CrewMemberFormView::new(
                form.user_id,
                name,
                phone,
                form.position,
                availability_status,
            ),
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
    let availability_status = normalize_availability(form.availability_status.clone());
    if name.is_empty() {
        return Err(CrewMemberError {
            message: "Member name is required.".to_string(),
            form: CrewMemberFormView::new(
                form.user_id,
                "",
                form.phone,
                form.position,
                availability_status,
            ),
        });
    }
    let phone = form.phone.trim().to_string();
    if phone.is_empty() {
        return Err(CrewMemberError {
            message: "Member phone is required.".to_string(),
            form: CrewMemberFormView::new(
                form.user_id,
                name,
                "",
                form.position,
                availability_status,
            ),
        });
    }
    if form.user_id <= 0 {
        return Err(CrewMemberError {
            message: "User account is required.".to_string(),
            form: CrewMemberFormView::new(0, name, phone, form.position, availability_status),
        });
    }
    let user = match user_repo::find_user_by_id(db, tenant_id, form.user_id).await {
        Ok(Some(user)) => user,
        _ => {
            return Err(CrewMemberError {
                message: "Selected user was not found.".to_string(),
                form: CrewMemberFormView::new(
                    form.user_id,
                    name,
                    phone,
                    form.position,
                    availability_status,
                ),
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
        &availability_status,
    )
    .await
    {
        return Err(CrewMemberError {
            message: format!("Unable to update crew member: {err}"),
            form: CrewMemberFormView::new(
                form.user_id,
                name,
                phone,
                form.position,
                availability_status,
            ),
        });
    }

    Ok(())
}

pub async fn roster_views(
    db: &Db,
    tenant_id: i64,
    crews: Vec<Crew>,
) -> Vec<CrewRosterView> {
    let crew_ids = crews.iter().map(|crew| crew.id).collect::<Vec<_>>();
    let availability_counts = crew_member_repo::count_availability_by_crew(db, tenant_id, &crew_ids)
        .await
        .unwrap_or_default();
    let availability_map = availability_counts
        .into_iter()
        .fold(HashMap::new(), |mut acc, (crew_id, status, count)| {
            let entry = acc.entry(crew_id).or_insert((0, 0, 0));
            match status.as_str() {
                AVAILABILITY_AVAILABLE => entry.0 += count,
                AVAILABILITY_AWAY => entry.1 += count,
                AVAILABILITY_UNAVAILABLE => entry.2 += count,
                _ => {}
            }
            acc
        });

    let mut roster = Vec::with_capacity(crews.len());
    for crew in crews {
        let (available, away, unavailable) =
            availability_map.get(&crew.id).copied().unwrap_or((0, 0, 0));
        let availability_score = availability_score(available, away, unavailable);
        let recent_statuses = deployment_repo::list_recent_statuses_for_crew(
            db,
            tenant_id,
            crew.id,
            RECENT_OUTCOME_LIMIT,
        )
        .await
        .unwrap_or_default();
        let outcome_score = outcome_score(&recent_statuses);
        let readiness_score =
            readiness_score(availability_score, crew.gear_score, outcome_score);

        roster.push(CrewRosterView {
            id: crew.id,
            name: crew.name,
            status: crew.status,
            members_count: crew.members_count,
            readiness_score,
            skill_tags: crew.skill_tags,
            compatibility_tags: crew.compatibility_tags,
        });
    }

    roster
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

pub fn recommend_crews(
    crews: &[Crew],
    required_skills: &str,
    compatibility_pref: &str,
) -> Vec<CrewRecommendation> {
    let required_set = tag_set(required_skills);
    let compatibility_set = tag_set(compatibility_pref);
    let mut recommendations = crews
        .iter()
        .map(|crew| {
            let crew_skills = tag_set(&crew.skill_tags);
            let crew_compat = tag_set(&crew.compatibility_tags);
            let skill_matches = count_matches(&crew_skills, &required_set);
            let compatibility_matches = count_matches(&crew_compat, &compatibility_set);
            let status_bonus = match crew.status.as_str() {
                STATUS_ACTIVE => 2,
                STATUS_IDLE => 1,
                STATUS_ON_LEAVE => -1,
                _ => 0,
            };
            let gear_bonus = (crew.gear_score / 20).clamp(0, 5);
            let score = skill_matches * 4 + compatibility_matches * 2 + status_bonus + gear_bonus;
            CrewRecommendation {
                id: crew.id,
                name: crew.name.clone(),
                status: crew.status.clone(),
                skill_matches,
                compatibility_matches,
                score,
            }
        })
        .collect::<Vec<_>>();
    recommendations.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
    recommendations
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

fn normalize_availability(input: String) -> String {
    let status = input.trim();
    for option in availability_options() {
        if option.eq_ignore_ascii_case(status) {
            return option.to_string();
        }
    }
    AVAILABILITY_AVAILABLE.to_string()
}

fn normalize_gear_score(input: i64) -> i64 {
    input.clamp(0, 100)
}

fn normalize_tags(input: String) -> String {
    let mut unique: Vec<String> = Vec::new();
    for raw in input.split(',') {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = trimmed.to_lowercase();
        if unique.iter().any(|tag| tag.eq_ignore_ascii_case(&normalized)) {
            continue;
        }
        unique.push(normalized);
    }
    unique.join(", ")
}

fn tag_set(input: &str) -> HashSet<String> {
    input
        .split(',')
        .map(|tag| tag.trim().to_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn count_matches(source: &HashSet<String>, target: &HashSet<String>) -> i64 {
    if target.is_empty() {
        return 0;
    }
    source.iter().filter(|tag| target.contains(*tag)).count() as i64
}

fn availability_score(available: i64, away: i64, unavailable: i64) -> i64 {
    let total = available + away + unavailable;
    if total == 0 {
        return 0;
    }
    let weighted = available * 100 + away * 50;
    (weighted / total).clamp(0, 100)
}

fn outcome_score(statuses: &[String]) -> i64 {
    if statuses.is_empty() {
        return 70;
    }
    let total = statuses.len() as i64;
    let score_sum: i64 = statuses
        .iter()
        .map(|status| match status.as_str() {
            "Completed" => 100,
            "Active" => 70,
            "Scheduled" => 60,
            "Cancelled" => 0,
            _ => 50,
        })
        .sum();
    (score_sum / total).clamp(0, 100)
}

fn readiness_score(availability_score: i64, gear_score: i64, outcome_score: i64) -> i64 {
    ((availability_score * 45 + gear_score * 25 + outcome_score * 30) / 100).clamp(0, 100)
}

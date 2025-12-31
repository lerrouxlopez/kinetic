use rocket_db_pools::sqlx;

use crate::models::{Workspace, WorkspaceFormView};
use crate::repositories::tenant_repo;
use crate::services::utils::normalize_slug;
use crate::Db;

pub struct WorkspaceError {
    pub message: String,
    pub form: WorkspaceFormView,
}

pub async fn list_workspaces(db: &Db) -> Result<Vec<Workspace>, sqlx::Error> {
    tenant_repo::list_workspaces(db).await
}

pub async fn find_workspace_by_id(
    db: &Db,
    id: i64,
) -> Result<Option<Workspace>, sqlx::Error> {
    tenant_repo::find_workspace_by_id(db, id).await
}

pub async fn create_workspace(
    db: &Db,
    slug_input: String,
    name_input: String,
) -> Result<(), WorkspaceError> {
    let slug = match normalize_slug(&slug_input) {
        Some(slug) => slug,
        None => {
            return Err(WorkspaceError {
                message: "Slug must be lowercase letters, numbers, or dashes.".to_string(),
                form: WorkspaceFormView::new(slug_input, name_input),
            })
        }
    };

    if name_input.trim().is_empty() {
        return Err(WorkspaceError {
            message: "Workspace name is required.".to_string(),
            form: WorkspaceFormView::new(slug, name_input),
        });
    }

    if let Err(err) = tenant_repo::create_tenant(db, &slug, name_input.trim()).await {
        return Err(WorkspaceError {
            message: format!("Unable to create workspace: {err}"),
            form: WorkspaceFormView::new(slug, name_input),
        });
    }

    Ok(())
}

pub async fn update_workspace(
    db: &Db,
    id: i64,
    slug_input: String,
    name_input: String,
) -> Result<(), WorkspaceError> {
    let slug = match normalize_slug(&slug_input) {
        Some(slug) => slug,
        None => {
            return Err(WorkspaceError {
                message: "Slug must be lowercase letters, numbers, or dashes.".to_string(),
                form: WorkspaceFormView::new(slug_input, name_input),
            })
        }
    };

    if name_input.trim().is_empty() {
        return Err(WorkspaceError {
            message: "Workspace name is required.".to_string(),
            form: WorkspaceFormView::new(slug, name_input),
        });
    }

    if let Err(err) = tenant_repo::update_workspace(db, id, &slug, name_input.trim()).await {
        return Err(WorkspaceError {
            message: format!("Unable to update workspace: {err}"),
            form: WorkspaceFormView::new(slug, name_input),
        });
    }

    Ok(())
}

pub async fn delete_workspace(db: &Db, id: i64) -> Result<(), String> {
    tenant_repo::delete_users_by_tenant(db, id)
        .await
        .map_err(|err| format!("Unable to delete workspace users: {err}"))?;
    tenant_repo::delete_workspace(db, id)
        .await
        .map_err(|err| format!("Unable to delete workspace: {err}"))?;
    Ok(())
}

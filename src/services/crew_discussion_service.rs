use rocket_db_pools::sqlx;

use crate::models::{CrewDiscussion, DiscussionForm, DiscussionFormView};
use crate::repositories::{crew_discussion_repo, user_repo};
use crate::Db;

pub struct DiscussionError {
    pub message: String,
    pub form: DiscussionFormView,
}

pub async fn list_discussions_by_crew(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
) -> Result<Vec<CrewDiscussion>, sqlx::Error> {
    crew_discussion_repo::list_discussions_by_crew(db, tenant_id, crew_id).await
}

pub async fn find_discussion_by_id(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    discussion_id: i64,
) -> Result<Option<CrewDiscussion>, sqlx::Error> {
    crew_discussion_repo::find_discussion_by_id(db, tenant_id, crew_id, discussion_id).await
}

pub async fn create_discussion(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    author_id: i64,
    form: DiscussionForm,
) -> Result<(), DiscussionError> {
    let message = form.message.trim().to_string();
    if message.is_empty() {
        return Err(DiscussionError {
            message: "Message is required.".to_string(),
            form: DiscussionFormView::new("", form.tagged_user_id),
        });
    }
    let tagged_user_id = normalize_tagged_user_id(form.tagged_user_id);
    if let Some(tagged_user_id) = tagged_user_id {
        let tagged_user = user_repo::find_user_by_id(db, tenant_id, tagged_user_id)
            .await
            .unwrap_or(None);
        if tagged_user.is_none() {
            return Err(DiscussionError {
                message: "Tagged user not found.".to_string(),
                form: DiscussionFormView::new(message, None),
            });
        }
    }

    if let Err(err) = crew_discussion_repo::create_discussion(
        db,
        tenant_id,
        crew_id,
        author_id,
        &message,
        tagged_user_id,
    )
    .await
    {
        return Err(DiscussionError {
            message: format!("Unable to add discussion: {err}"),
            form: DiscussionFormView::new(message, tagged_user_id),
        });
    }

    Ok(())
}

pub async fn update_discussion(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    discussion_id: i64,
    form: DiscussionForm,
) -> Result<(), DiscussionError> {
    let message = form.message.trim().to_string();
    if message.is_empty() {
        return Err(DiscussionError {
            message: "Message is required.".to_string(),
            form: DiscussionFormView::new("", form.tagged_user_id),
        });
    }
    let tagged_user_id = normalize_tagged_user_id(form.tagged_user_id);
    if let Some(tagged_user_id) = tagged_user_id {
        let tagged_user = user_repo::find_user_by_id(db, tenant_id, tagged_user_id)
            .await
            .unwrap_or(None);
        if tagged_user.is_none() {
            return Err(DiscussionError {
                message: "Tagged user not found.".to_string(),
                form: DiscussionFormView::new(message, None),
            });
        }
    }

    if let Err(err) = crew_discussion_repo::update_discussion(
        db,
        tenant_id,
        crew_id,
        discussion_id,
        &message,
        tagged_user_id,
    )
    .await
    {
        return Err(DiscussionError {
            message: format!("Unable to update discussion: {err}"),
            form: DiscussionFormView::new(message, tagged_user_id),
        });
    }

    Ok(())
}

pub async fn delete_discussion(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    discussion_id: i64,
) -> Result<(), sqlx::Error> {
    crew_discussion_repo::delete_discussion(db, tenant_id, crew_id, discussion_id).await
}

fn normalize_tagged_user_id(tagged_user_id: Option<i64>) -> Option<i64> {
    match tagged_user_id {
        Some(value) if value > 0 => Some(value),
        _ => None,
    }
}

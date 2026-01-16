use rocket_db_pools::sqlx::{self, Row};

use crate::models::CrewDiscussion;
use crate::Db;

pub async fn list_discussions_by_crew(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
) -> Result<Vec<CrewDiscussion>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT d.id,
               d.tenant_id,
               d.crew_id,
               d.author_id,
               d.tagged_user_id,
               d.message,
               d.created_at,
               author.email as author_email,
               tagged.email as tagged_user_email
        FROM crew_discussions d
        JOIN users author
          ON author.id = d.author_id
         AND author.tenant_id = d.tenant_id
        LEFT JOIN users tagged
          ON tagged.id = d.tagged_user_id
         AND tagged.tenant_id = d.tenant_id
        WHERE d.tenant_id = ? AND d.crew_id = ?
        ORDER BY d.created_at DESC, d.id DESC
        "#,
    )
    .bind(tenant_id)
    .bind(crew_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| CrewDiscussion {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            crew_id: row.get("crew_id"),
            author_id: row.get("author_id"),
            author_email: row.get("author_email"),
            tagged_user_id: row.get("tagged_user_id"),
            tagged_user_email: row.get("tagged_user_email"),
            message: row.get("message"),
            created_at: row.get("created_at"),
        })
        .collect())
}

pub async fn find_discussion_by_id(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    discussion_id: i64,
) -> Result<Option<CrewDiscussion>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT d.id,
               d.tenant_id,
               d.crew_id,
               d.author_id,
               d.tagged_user_id,
               d.message,
               d.created_at,
               author.email as author_email,
               tagged.email as tagged_user_email
        FROM crew_discussions d
        JOIN users author
          ON author.id = d.author_id
         AND author.tenant_id = d.tenant_id
        LEFT JOIN users tagged
          ON tagged.id = d.tagged_user_id
         AND tagged.tenant_id = d.tenant_id
        WHERE d.tenant_id = ? AND d.crew_id = ? AND d.id = ?
        "#,
    )
    .bind(tenant_id)
    .bind(crew_id)
    .bind(discussion_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| CrewDiscussion {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        crew_id: row.get("crew_id"),
        author_id: row.get("author_id"),
        author_email: row.get("author_email"),
        tagged_user_id: row.get("tagged_user_id"),
        tagged_user_email: row.get("tagged_user_email"),
        message: row.get("message"),
        created_at: row.get("created_at"),
    }))
}

pub async fn create_discussion(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    author_id: i64,
    message: &str,
    tagged_user_id: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO crew_discussions
            (tenant_id, crew_id, author_id, tagged_user_id, message)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(tenant_id)
    .bind(crew_id)
    .bind(author_id)
    .bind(tagged_user_id)
    .bind(message)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn update_discussion(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    discussion_id: i64,
    message: &str,
    tagged_user_id: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE crew_discussions
        SET message = ?, tagged_user_id = ?
        WHERE tenant_id = ? AND crew_id = ? AND id = ?
        "#,
    )
    .bind(message)
    .bind(tagged_user_id)
    .bind(tenant_id)
    .bind(crew_id)
    .bind(discussion_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn delete_discussion(
    db: &Db,
    tenant_id: i64,
    crew_id: i64,
    discussion_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM crew_discussions WHERE tenant_id = ? AND crew_id = ? AND id = ?",
    )
    .bind(tenant_id)
    .bind(crew_id)
    .bind(discussion_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

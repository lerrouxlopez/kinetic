use rocket_db_pools::sqlx::{self, Row};

use crate::models::UserPermission;
use crate::Db;

pub async fn list_permissions_for_user(
    db: &Db,
    tenant_id: i64,
    user_id: i64,
) -> Result<Vec<UserPermission>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT resource, can_view, can_edit, can_delete
        FROM user_permissions
        WHERE tenant_id = ? AND user_id = ?
        ORDER BY resource ASC
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| UserPermission {
            resource: row.get("resource"),
            can_view: row.get::<i64, _>("can_view") != 0,
            can_edit: row.get::<i64, _>("can_edit") != 0,
            can_delete: row.get::<i64, _>("can_delete") != 0,
        })
        .collect())
}

pub async fn replace_permissions_for_user(
    db: &Db,
    tenant_id: i64,
    user_id: i64,
    permissions: &[UserPermission],
) -> Result<(), sqlx::Error> {
    let mut tx = db.0.begin().await?;
    sqlx::query("DELETE FROM user_permissions WHERE tenant_id = ? AND user_id = ?")
        .bind(tenant_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    for permission in permissions {
        sqlx::query(
            r#"
            INSERT INTO user_permissions
                (tenant_id, user_id, resource, can_view, can_edit, can_delete)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(&permission.resource)
        .bind(if permission.can_view { 1 } else { 0 })
        .bind(if permission.can_edit { 1 } else { 0 })
        .bind(if permission.can_delete { 1 } else { 0 })
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

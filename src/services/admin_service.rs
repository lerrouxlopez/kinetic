use rocket_db_pools::sqlx;

use crate::models::{AdminLoginView, AdminUser};
use crate::repositories::admin_repo;
use crate::services::utils::verify_password;
use crate::Db;

pub struct AdminLoginError {
    pub message: String,
    pub form: AdminLoginView,
}

pub async fn get_admin_by_id(
    db: &Db,
    admin_id: i64,
) -> Result<Option<AdminUser>, sqlx::Error> {
    admin_repo::find_admin_by_id(db, admin_id).await
}

pub async fn authenticate_admin(
    db: &Db,
    email: String,
    password: String,
) -> Result<AdminUser, AdminLoginError> {
    let auth = match admin_repo::find_admin_auth_by_email(db, &email.trim().to_lowercase()).await {
        Ok(Some(auth)) => auth,
        _ => {
            return Err(AdminLoginError {
                message: "Invalid credentials.".to_string(),
                form: AdminLoginView::new(email),
            })
        }
    };

    if let Err(message) = verify_password(&password, &auth.password_hash) {
        return Err(AdminLoginError {
            message,
            form: AdminLoginView::new(email),
        });
    }

    Ok(auth.admin)
}

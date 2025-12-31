use rocket_db_pools::sqlx;

use crate::models::{LoginForm, LoginView, RegisterForm, RegisterView, User};
use crate::repositories::{tenant_repo, user_repo};
use crate::services::utils::{hash_password, normalize_slug, verify_password};
use crate::Db;

pub struct RegisterError {
    pub message: String,
    pub form: RegisterView,
}

pub struct LoginError {
    pub message: String,
    pub form: LoginView,
}

pub async fn get_user_by_ids(
    db: &Db,
    user_id: i64,
    tenant_id: i64,
) -> Result<Option<User>, sqlx::Error> {
    user_repo::find_user_by_ids(db, user_id, tenant_id).await
}

pub async fn register(
    db: &Db,
    form: RegisterForm,
) -> Result<(User, i64), RegisterError> {
    let slug = match normalize_slug(&form.tenant_slug) {
        Some(slug) => slug,
        None => {
            return Err(RegisterError {
                message: "Workspace slug must be lowercase letters, numbers, or dashes.".to_string(),
                form: RegisterView::new(
                    form.tenant_slug,
                    form.tenant_name.unwrap_or_default(),
                    form.email,
                ),
            })
        }
    };

    if form.password.trim().len() < 8 {
        return Err(RegisterError {
            message: "Password must be at least 8 characters.".to_string(),
            form: RegisterView::new(slug, form.tenant_name.unwrap_or_default(), form.email),
        });
    }

    let tenant_name = form
        .tenant_name
        .clone()
        .unwrap_or_else(|| slug.replace('-', " "));

    let tenant_id: i64 = match tenant_repo::find_tenant_id_by_slug(db, &slug).await {
        Ok(Some(id)) => id,
        Ok(None) => match tenant_repo::create_tenant(db, &slug, &tenant_name).await {
            Ok(id) => id,
            Err(err) => {
                return Err(RegisterError {
                    message: format!("Unable to create workspace: {err}"),
                    form: RegisterView::new(slug, tenant_name, form.email),
                })
            }
        },
        Err(_) => {
            return Err(RegisterError {
                message: "Unable to check workspace.".to_string(),
                form: RegisterView::new(slug, tenant_name, form.email),
            })
        }
    };

    let password_hash = match hash_password(&form.password) {
        Ok(hash) => hash,
        Err(message) => {
            return Err(RegisterError {
                message,
                form: RegisterView::new(slug, tenant_name, form.email),
            })
        }
    };

    if let Err(err) = user_repo::create_user(
        db,
        tenant_id,
        &form.email.trim().to_lowercase(),
        &password_hash,
    )
    .await
    {
        return Err(RegisterError {
            message: format!("Unable to create user: {err}"),
            form: RegisterView::new(slug, tenant_name, form.email),
        });
    }

    let user = match user_repo::find_user_by_email_and_tenant(
        db,
        &form.email.trim().to_lowercase(),
        tenant_id,
    )
    .await
    {
        Ok(Some(user)) => user,
        _ => {
            return Err(RegisterError {
                message: "User created, but could not load profile.".to_string(),
                form: RegisterView::new(slug, tenant_name, form.email),
            })
        }
    };

    Ok((user, tenant_id))
}

pub async fn login(db: &Db, form: LoginForm) -> Result<User, LoginError> {
    let slug = match normalize_slug(&form.tenant_slug) {
        Some(slug) => slug,
        None => {
            return Err(LoginError {
                message: "Workspace slug must be lowercase letters, numbers, or dashes.".to_string(),
                form: LoginView::new(form.tenant_slug, form.email),
            })
        }
    };

    let auth = match user_repo::find_user_auth_by_email_and_tenant_slug(
        db,
        &form.email.trim().to_lowercase(),
        &slug,
    )
    .await
    {
        Ok(Some(auth)) => auth,
        _ => {
            return Err(LoginError {
                message: "Invalid credentials.".to_string(),
                form: LoginView::new(slug, form.email),
            })
        }
    };

    if let Err(message) = verify_password(&form.password, &auth.password_hash) {
        return Err(LoginError {
            message,
            form: LoginView::new(slug, form.email),
        });
    }

    Ok(auth.user)
}

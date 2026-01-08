use rocket_db_pools::sqlx;

use crate::models::{
    LoginForm,
    LoginView,
    RegisterForm,
    RegisterView,
    User,
    WorkspaceRegisterForm,
};
use crate::repositories::{tenant_repo, user_repo};
use crate::services::utils::{hash_password, normalize_slug, verify_password};
use crate::services::workspace_service;
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
    if let Some(mut user) = user_repo::find_user_by_ids(db, user_id, tenant_id).await? {
        user.plan_expired = workspace_service::is_workspace_plan_expired(db, user.tenant_id).await;
        return Ok(Some(user));
    }

    let Some(user) = user_repo::find_user_by_id_any(db, user_id).await? else {
        return Ok(None);
    };
    if !user.is_super_admin {
        return Ok(None);
    }

    let workspace = match tenant_repo::find_workspace_by_id(db, tenant_id).await? {
        Some(workspace) => workspace,
        None => return Ok(None),
    };

    let mut user = User {
        tenant_id: workspace.id,
        tenant_slug: workspace.slug,
        plan_key: workspace.plan_key,
        ..user
    };
    user.plan_expired = workspace_service::is_workspace_plan_expired(db, user.tenant_id).await;
    Ok(Some(user))
}

pub async fn register(
    db: &Db,
    form: RegisterForm,
) -> Result<(User, i64), RegisterError> {
    let plan_key = "free".to_string();
    let tenant_name = form.tenant_name.trim().to_string();
    if tenant_name.is_empty() {
        return Err(RegisterError {
            message: "Company name is required.".to_string(),
            form: RegisterView::new(tenant_name, form.email, plan_key.clone()),
        });
    }

    let slug = match normalize_slug(&tenant_name) {
        Some(slug) => slug,
        None => {
            return Err(RegisterError {
                message: "Company name must be letters, numbers, or dashes.".to_string(),
                form: RegisterView::new(tenant_name, form.email, plan_key.clone()),
            })
        }
    };

    if form.password.trim().len() < 8 {
        return Err(RegisterError {
            message: "Password must be at least 8 characters.".to_string(),
            form: RegisterView::new(tenant_name, form.email, plan_key.clone()),
        });
    }

    let tenant_id: i64 = match tenant_repo::find_tenant_id_by_slug(db, &slug).await {
        Ok(Some(id)) => id,
        Ok(None) => match tenant_repo::create_tenant(db, &slug, &tenant_name, &plan_key).await {
            Ok(id) => id,
            Err(err) => {
                return Err(RegisterError {
                    message: format!("Unable to create workspace: {err}"),
                    form: RegisterView::new(tenant_name, form.email, plan_key.clone()),
                })
            }
        },
        Err(_) => {
            return Err(RegisterError {
                message: "Unable to check workspace.".to_string(),
                form: RegisterView::new(tenant_name, form.email, plan_key.clone()),
            })
        }
    };

    let password_hash = match hash_password(&form.password) {
        Ok(hash) => hash,
        Err(message) => {
            return Err(RegisterError {
                message,
                form: RegisterView::new(tenant_name, form.email, plan_key.clone()),
            })
        }
    };

    if let Err(err) = user_repo::create_user(
        db,
        tenant_id,
        &form.email.trim().to_lowercase(),
        &password_hash,
        "Owner",
    )
    .await
    {
        return Err(RegisterError {
            message: format!("Unable to create user: {err}"),
            form: RegisterView::new(tenant_name, form.email, plan_key.clone()),
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
                form: RegisterView::new(tenant_name, form.email, plan_key.clone()),
            })
        }
    };

    Ok((user, tenant_id))
}

pub async fn register_workspace_user(
    db: &Db,
    tenant_slug: &str,
    form: WorkspaceRegisterForm,
) -> Result<User, RegisterError> {
    let slug = match normalize_slug(tenant_slug) {
        Some(slug) => slug,
        None => {
            return Err(RegisterError {
                message: "Workspace slug must be lowercase letters, numbers, or dashes.".to_string(),
                form: RegisterView::new("", form.email, "free"),
            })
        }
    };

    if form.password.trim().len() < 8 {
        return Err(RegisterError {
            message: "Password must be at least 8 characters.".to_string(),
            form: RegisterView::new("", form.email, "free"),
        });
    }

    let tenant_id: i64 = match tenant_repo::find_tenant_id_by_slug(db, &slug).await {
        Ok(Some(id)) => id,
        _ => {
            return Err(RegisterError {
                message: "Workspace not found.".to_string(),
                form: RegisterView::new("", form.email, "free"),
            })
        }
    };

    let password_hash = match hash_password(&form.password) {
        Ok(hash) => hash,
        Err(message) => {
            return Err(RegisterError {
                message,
                form: RegisterView::new("", form.email, "free"),
            })
        }
    };

    if let Err(err) = user_repo::create_user(
        db,
        tenant_id,
        &form.email.trim().to_lowercase(),
        &password_hash,
        "Employee",
    )
    .await
    {
        return Err(RegisterError {
            message: format!("Unable to create user: {err}"),
            form: RegisterView::new("", form.email, "free"),
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
                form: RegisterView::new("", form.email, "free"),
            })
        }
    };

    Ok(user)
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
        _ => match user_repo::find_super_admin_auth_by_email(db, &form.email.trim().to_lowercase()).await {
            Ok(Some(auth)) => auth,
            _ => {
                return Err(LoginError {
                    message: "Invalid credentials.".to_string(),
                    form: LoginView::new(slug, form.email),
                })
            }
        },
    };

    if let Err(message) = verify_password(&form.password, &auth.password_hash) {
        return Err(LoginError {
            message,
            form: LoginView::new(slug, form.email),
        });
    }

    if auth.user.is_super_admin {
        let tenant_id = match tenant_repo::find_tenant_id_by_slug(db, &slug).await {
            Ok(Some(id)) => id,
            _ => {
                return Err(LoginError {
                    message: "Workspace not found.".to_string(),
                    form: LoginView::new(slug, form.email),
                })
            }
        };

        let mut user = auth.user;
        user.tenant_id = tenant_id;
        user.tenant_slug = slug;
        return Ok(user);
    }

    Ok(auth.user)
}


use rocket::form::FromForm;
use serde::Serialize;

#[derive(FromForm)]
pub struct RegisterForm {
    pub tenant_slug: String,
    pub tenant_name: Option<String>,
    pub email: String,
    pub password: String,
}

#[derive(FromForm)]
pub struct LoginForm {
    pub tenant_slug: String,
    pub email: String,
    pub password: String,
}

#[derive(FromForm)]
pub struct AdminLoginForm {
    pub email: String,
    pub password: String,
}

#[derive(FromForm)]
pub struct WorkspaceForm {
    pub slug: String,
    pub name: String,
}

#[derive(FromForm)]
pub struct CrewForm {
    pub name: String,
    pub members_count: i64,
    pub status: String,
}

pub struct User {
    pub id: i64,
    pub tenant_id: i64,
    pub tenant_slug: String,
    pub email: String,
}

pub struct AdminUser {
    pub id: i64,
    pub email: String,
}

#[derive(Serialize, Clone)]
pub struct Workspace {
    pub id: i64,
    pub slug: String,
    pub name: String,
}

pub struct UserAuth {
    pub user: User,
    pub password_hash: String,
}

#[derive(Serialize, Clone)]
pub struct Crew {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub members_count: i64,
    pub status: String,
}

pub struct AdminAuth {
    pub admin: AdminUser,
    pub password_hash: String,
}

#[derive(Serialize, Clone)]
pub struct RegisterView {
    pub tenant_slug: String,
    pub tenant_name: String,
    pub email: String,
}

#[derive(Serialize, Clone)]
pub struct LoginView {
    pub tenant_slug: String,
    pub email: String,
}

#[derive(Serialize, Clone)]
pub struct AdminLoginView {
    pub email: String,
}

#[derive(Serialize, Clone)]
pub struct WorkspaceFormView {
    pub slug: String,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct CrewFormView {
    pub name: String,
    pub members_count: i64,
    pub status: String,
}

#[derive(Serialize)]
pub struct CrewStats {
    pub total_crews: usize,
    pub active_crews: usize,
    pub idle_crews: usize,
    pub on_leave_crews: usize,
    pub total_members: i64,
}

#[derive(Serialize)]
pub struct CurrentUserView {
    pub tenant_slug: String,
    pub email: String,
}

impl From<&User> for CurrentUserView {
    fn from(user: &User) -> Self {
        CurrentUserView {
            tenant_slug: user.tenant_slug.clone(),
            email: user.email.clone(),
        }
    }
}

impl RegisterView {
    pub fn new(
        tenant_slug: impl Into<String>,
        tenant_name: impl Into<String>,
        email: impl Into<String>,
    ) -> Self {
        RegisterView {
            tenant_slug: tenant_slug.into(),
            tenant_name: tenant_name.into(),
            email: email.into(),
        }
    }
}

impl LoginView {
    pub fn new(tenant_slug: impl Into<String>, email: impl Into<String>) -> Self {
        LoginView {
            tenant_slug: tenant_slug.into(),
            email: email.into(),
        }
    }
}

impl AdminLoginView {
    pub fn new(email: impl Into<String>) -> Self {
        AdminLoginView { email: email.into() }
    }
}

impl WorkspaceFormView {
    pub fn new(slug: impl Into<String>, name: impl Into<String>) -> Self {
        WorkspaceFormView {
            slug: slug.into(),
            name: name.into(),
        }
    }
}

impl CrewFormView {
    pub fn new(name: impl Into<String>, members_count: i64, status: impl Into<String>) -> Self {
        CrewFormView {
            name: name.into(),
            members_count,
            status: status.into(),
        }
    }
}

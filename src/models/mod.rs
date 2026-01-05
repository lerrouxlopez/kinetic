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
pub struct WorkspaceEmailSettingsForm {
    pub email_provider: String,
    pub from_name: String,
    pub from_address: String,
    pub smtp_host: String,
    pub smtp_port: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_encryption: String,
    pub mailgun_domain: String,
    pub mailgun_api_key: String,
    pub postmark_server_token: String,
    pub resend_api_key: String,
    pub ses_access_key: String,
    pub ses_secret_key: String,
    pub ses_region: String,
    pub sendmail_path: String,
}

#[derive(FromForm)]
pub struct CrewForm {
    pub name: String,
    pub members_count: i64,
    pub status: String,
}

#[derive(FromForm)]
pub struct CrewMemberForm {
    pub name: String,
    pub phone: String,
    pub email: String,
    pub position: String,
}

#[derive(FromForm)]
pub struct ClientForm {
    pub company_name: String,
    pub address: String,
    pub phone: String,
    pub email: String,
    pub latitude: String,
    pub longitude: String,
    pub stage: String,
    pub currency: String,
}

#[derive(FromForm)]
pub struct ClientContactForm {
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone: String,
    pub department: String,
    pub position: String,
}

#[derive(FromForm)]
pub struct AppointmentForm {
    pub title: String,
    pub scheduled_for: String,
    pub status: String,
    pub notes: String,
}

#[derive(FromForm)]
pub struct DeploymentForm {
    pub client_id: i64,
    pub crew_id: i64,
    pub start_at: String,
    pub end_at: String,
    pub fee_per_hour: f64,
    pub info: String,
    pub status: String,
}

#[derive(FromForm)]
pub struct DeploymentUpdateForm {
    pub deployment_id: i64,
    pub work_date: String,
    pub start_time: String,
    pub end_time: String,
    pub notes: String,
}

#[derive(FromForm)]
pub struct EmailForm {
    pub subject: String,
    pub body: String,
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
    pub email_provider: String,
    pub email_from_name: String,
    pub email_from_address: String,
    pub smtp_host: String,
    pub smtp_port: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_encryption: String,
    pub mailgun_domain: String,
    pub mailgun_api_key: String,
    pub postmark_server_token: String,
    pub resend_api_key: String,
    pub ses_access_key: String,
    pub ses_secret_key: String,
    pub ses_region: String,
    pub sendmail_path: String,
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

#[derive(Serialize, Clone)]
pub struct CrewMember {
    pub id: i64,
    pub crew_id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub position: String,
}

#[derive(Serialize, Clone)]
pub struct Client {
    pub id: i64,
    pub tenant_id: i64,
    pub company_name: String,
    pub address: String,
    pub phone: String,
    pub email: String,
    pub latitude: String,
    pub longitude: String,
    pub stage: String,
    pub currency: String,
}

#[derive(Serialize, Clone)]
pub struct ClientContact {
    pub id: i64,
    pub client_id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone: String,
    pub department: String,
    pub position: String,
}

#[derive(Serialize, Clone)]
pub struct Appointment {
    pub id: i64,
    pub client_id: i64,
    pub contact_id: i64,
    pub tenant_id: i64,
    pub contact_name: String,
    pub title: String,
    pub scheduled_for: String,
    pub status: String,
    pub notes: String,
}

#[derive(Serialize, Clone)]
pub struct Deployment {
    pub id: i64,
    pub tenant_id: i64,
    pub client_id: i64,
    pub crew_id: i64,
    pub start_at: String,
    pub end_at: String,
    pub fee_per_hour: f64,
    pub info: String,
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct DeploymentSummary {
    pub id: i64,
    pub crew_id: i64,
    pub crew_name: String,
    pub start_at: String,
    pub end_at: String,
    pub fee_per_hour: f64,
    pub info: String,
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct DeploymentUpdate {
    pub id: i64,
    pub tenant_id: i64,
    pub deployment_id: i64,
    pub work_date: String,
    pub start_time: String,
    pub end_time: String,
    pub hours_worked: f64,
    pub notes: String,
}

#[derive(Serialize, Clone)]
pub struct DeploymentSelect {
    pub id: i64,
    pub label: String,
}

#[derive(Serialize, Clone)]
pub struct DeploymentClientGroup {
    pub client_id: i64,
    pub client_name: String,
    pub client_currency: String,
    pub deployments: Vec<DeploymentSummary>,
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

#[derive(Serialize, Clone)]
pub struct CrewMemberFormView {
    pub name: String,
    pub phone: String,
    pub email: String,
    pub position: String,
}

#[derive(Serialize, Clone)]
pub struct ClientFormView {
    pub company_name: String,
    pub address: String,
    pub phone: String,
    pub email: String,
    pub latitude: String,
    pub longitude: String,
    pub stage: String,
    pub currency: String,
}

#[derive(Serialize, Clone)]
pub struct ClientContactFormView {
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone: String,
    pub department: String,
    pub position: String,
}

#[derive(Serialize, Clone)]
pub struct AppointmentFormView {
    pub title: String,
    pub scheduled_for: String,
    pub status: String,
    pub notes: String,
}

#[derive(Serialize, Clone)]
pub struct DeploymentFormView {
    pub client_id: i64,
    pub crew_id: i64,
    pub start_at: String,
    pub end_at: String,
    pub fee_per_hour: f64,
    pub info: String,
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct DeploymentUpdateFormView {
    pub deployment_id: i64,
    pub work_date: String,
    pub start_time: String,
    pub end_time: String,
    pub notes: String,
}

#[derive(Serialize, Clone)]
pub struct WorkspaceEmailSettingsView {
    pub email_provider: String,
    pub from_name: String,
    pub from_address: String,
    pub smtp_host: String,
    pub smtp_port: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_encryption: String,
    pub mailgun_domain: String,
    pub mailgun_api_key: String,
    pub postmark_server_token: String,
    pub resend_api_key: String,
    pub ses_access_key: String,
    pub ses_secret_key: String,
    pub ses_region: String,
    pub sendmail_path: String,
}

#[derive(Serialize, Clone)]
pub struct EmailFormView {
    pub subject: String,
    pub body: String,
}

#[derive(Serialize)]
pub struct CrewStats {
    pub total_crews: usize,
    pub active_crews: usize,
    pub idle_crews: usize,
    pub on_leave_crews: usize,
    pub total_members: i64,
}

#[derive(Serialize, Clone)]
pub struct PaginationView {
    pub page: usize,
    pub total_pages: usize,
    pub has_prev: bool,
    pub has_next: bool,
    pub prev_url: String,
    pub next_url: String,
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

impl WorkspaceEmailSettingsView {
    pub fn new(
        email_provider: impl Into<String>,
        from_name: impl Into<String>,
        from_address: impl Into<String>,
        smtp_host: impl Into<String>,
        smtp_port: impl Into<String>,
        smtp_username: impl Into<String>,
        smtp_password: impl Into<String>,
        smtp_encryption: impl Into<String>,
        mailgun_domain: impl Into<String>,
        mailgun_api_key: impl Into<String>,
        postmark_server_token: impl Into<String>,
        resend_api_key: impl Into<String>,
        ses_access_key: impl Into<String>,
        ses_secret_key: impl Into<String>,
        ses_region: impl Into<String>,
        sendmail_path: impl Into<String>,
    ) -> Self {
        WorkspaceEmailSettingsView {
            email_provider: email_provider.into(),
            from_name: from_name.into(),
            from_address: from_address.into(),
            smtp_host: smtp_host.into(),
            smtp_port: smtp_port.into(),
            smtp_username: smtp_username.into(),
            smtp_password: smtp_password.into(),
            smtp_encryption: smtp_encryption.into(),
            mailgun_domain: mailgun_domain.into(),
            mailgun_api_key: mailgun_api_key.into(),
            postmark_server_token: postmark_server_token.into(),
            resend_api_key: resend_api_key.into(),
            ses_access_key: ses_access_key.into(),
            ses_secret_key: ses_secret_key.into(),
            ses_region: ses_region.into(),
            sendmail_path: sendmail_path.into(),
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

impl CrewMemberFormView {
    pub fn new(
        name: impl Into<String>,
        phone: impl Into<String>,
        email: impl Into<String>,
        position: impl Into<String>,
    ) -> Self {
        CrewMemberFormView {
            name: name.into(),
            phone: phone.into(),
            email: email.into(),
            position: position.into(),
        }
    }
}

impl ClientFormView {
    pub fn new(
        company_name: impl Into<String>,
        address: impl Into<String>,
        phone: impl Into<String>,
        email: impl Into<String>,
        latitude: impl Into<String>,
        longitude: impl Into<String>,
        stage: impl Into<String>,
        currency: impl Into<String>,
    ) -> Self {
        ClientFormView {
            company_name: company_name.into(),
            address: address.into(),
            phone: phone.into(),
            email: email.into(),
            latitude: latitude.into(),
            longitude: longitude.into(),
            stage: stage.into(),
            currency: currency.into(),
        }
    }
}

impl ClientContactFormView {
    pub fn new(
        name: impl Into<String>,
        address: impl Into<String>,
        email: impl Into<String>,
        phone: impl Into<String>,
        department: impl Into<String>,
        position: impl Into<String>,
    ) -> Self {
        ClientContactFormView {
            name: name.into(),
            address: address.into(),
            email: email.into(),
            phone: phone.into(),
            department: department.into(),
            position: position.into(),
        }
    }
}

impl AppointmentFormView {
    pub fn new(
        title: impl Into<String>,
        scheduled_for: impl Into<String>,
        status: impl Into<String>,
        notes: impl Into<String>,
    ) -> Self {
        AppointmentFormView {
            title: title.into(),
            scheduled_for: scheduled_for.into(),
            status: status.into(),
            notes: notes.into(),
        }
    }
}

impl DeploymentFormView {
    pub fn new(
        client_id: i64,
        crew_id: i64,
        start_at: impl Into<String>,
        end_at: impl Into<String>,
        fee_per_hour: f64,
        info: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        DeploymentFormView {
            client_id,
            crew_id,
            start_at: start_at.into(),
            end_at: end_at.into(),
            fee_per_hour,
            info: info.into(),
            status: status.into(),
        }
    }
}

impl DeploymentUpdateFormView {
    pub fn new(
        deployment_id: i64,
        work_date: impl Into<String>,
        start_time: impl Into<String>,
        end_time: impl Into<String>,
        notes: impl Into<String>,
    ) -> Self {
        DeploymentUpdateFormView {
            deployment_id,
            work_date: work_date.into(),
            start_time: start_time.into(),
            end_time: end_time.into(),
            notes: notes.into(),
        }
    }
}
impl EmailFormView {
    pub fn new(subject: impl Into<String>, body: impl Into<String>) -> Self {
        EmailFormView {
            subject: subject.into(),
            body: body.into(),
        }
    }
}

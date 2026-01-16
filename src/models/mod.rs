use rocket::form::FromForm;
use rocket::fs::TempFile;
use serde::Serialize;

#[derive(FromForm)]
pub struct RegisterForm {
    pub tenant_name: String,
    pub email: String,
    pub password: String,
    pub plan_key: String,
}

#[derive(FromForm)]
pub struct LoginForm {
    pub tenant_slug: String,
    pub email: String,
    pub password: String,
}

#[derive(FromForm)]
pub struct WorkspaceRegisterForm {
    pub email: String,
    pub password: String,
}

#[derive(FromForm)]
pub struct AdminLoginForm {
    pub email: String,
    pub password: String,
}

#[derive(FromForm)]
pub struct AdminUserForm {
    pub tenant_id: i64,
    pub email: String,
    pub role: String,
    pub password: Option<String>,
}

#[derive(FromForm, Clone)]
pub struct PlanLimitsForm {
    pub clients: i64,
    pub contacts_per_client: i64,
    pub appointments_per_client: i64,
    pub deployments_per_client: i64,
    pub crews: i64,
    pub members_per_crew: i64,
    pub users: i64,
    pub expires_after_days: i64,
}

#[derive(FromForm)]
pub struct WorkspaceForm {
    pub slug: String,
    pub name: String,
    pub plan_key: String,
}

#[derive(FromForm)]
pub struct WorkspaceEmailSettingsForm {
    pub email_provider: String,
    pub from_name: String,
    pub from_address: String,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_encryption: Option<String>,
    pub mailgun_domain: Option<String>,
    pub mailgun_api_key: Option<String>,
    pub postmark_server_token: Option<String>,
    pub resend_api_key: Option<String>,
    pub ses_access_key: Option<String>,
    pub ses_secret_key: Option<String>,
    pub ses_region: Option<String>,
    pub sendmail_path: Option<String>,
}

#[derive(FromForm)]
pub struct WorkspaceThemeForm<'r> {
    pub app_name: Option<String>,
    pub theme_key: Option<String>,
    pub background_hue: Option<i64>,
    pub body_font: Option<String>,
    pub heading_font: Option<String>,
    pub logo: Option<TempFile<'r>>,
}

#[derive(FromForm)]
pub struct CrewForm {
    pub name: String,
    pub status: String,
    pub gear_score: i64,
    pub skill_tags: String,
    pub compatibility_tags: String,
}

#[derive(FromForm)]
pub struct CrewMemberForm {
    pub user_id: i64,
    pub name: String,
    pub phone: String,
    pub position: String,
    pub availability_status: String,
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
pub struct DiscussionForm {
    pub message: String,
    pub tagged_user_id: Option<i64>,
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
    pub deployment_type: String,
    pub required_skills: String,
    pub compatibility_pref: String,
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
pub struct WorkTimerForm {
    pub deployment_id: i64,
}

#[derive(FromForm)]
pub struct InvoiceForm {
    pub deployment_id: i64,
    pub status: String,
    pub notes: String,
}

#[derive(FromForm)]
pub struct EmailForm {
    pub subject: String,
    pub body: String,
}

#[derive(FromForm)]
pub struct UserPermissionForm {
    pub role: String,
    pub permissions: Option<Vec<String>>,
}

pub struct User {
    pub id: i64,
    pub tenant_id: i64,
    pub tenant_slug: String,
    pub plan_key: String,
    pub plan_expired: bool,
    pub email: String,
    pub role: String,
    pub is_super_admin: bool,
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
    pub app_name: String,
    pub logo_path: String,
    pub theme_key: String,
    pub background_hue: i64,
    pub body_font: String,
    pub heading_font: String,
    pub plan_key: String,
    pub plan_started_at: String,
    pub plan_expired: bool,
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
    pub gear_score: i64,
    pub skill_tags: String,
    pub compatibility_tags: String,
}

#[derive(Serialize, Clone)]
pub struct CrewMember {
    pub id: i64,
    pub crew_id: i64,
    pub tenant_id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub position: String,
    pub availability_status: String,
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
    pub portal_token: String,
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
pub struct Discussion {
    pub id: i64,
    pub tenant_id: i64,
    pub client_id: i64,
    pub author_id: i64,
    pub author_email: String,
    pub tagged_user_id: Option<i64>,
    pub tagged_user_email: Option<String>,
    pub message: String,
    pub created_at: String,
}

#[derive(Serialize, Clone)]
pub struct CrewDiscussion {
    pub id: i64,
    pub tenant_id: i64,
    pub crew_id: i64,
    pub author_id: i64,
    pub author_email: String,
    pub tagged_user_id: Option<i64>,
    pub tagged_user_email: Option<String>,
    pub message: String,
    pub created_at: String,
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
    pub deployment_type: String,
    pub required_skills: String,
    pub compatibility_pref: String,
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
    pub deployment_type: String,
}

#[derive(Serialize, Clone)]
pub struct DeploymentTimelineStep {
    pub label: String,
    pub state: String,
    pub note: String,
}

#[derive(Serialize, Clone)]
pub struct DeploymentUpdate {
    pub id: i64,
    pub tenant_id: i64,
    pub deployment_id: i64,
    pub user_id: Option<i64>,
    pub user_email: String,
    pub work_date: String,
    pub start_time: String,
    pub end_time: String,
    pub hours_worked: f64,
    pub notes: String,
    pub is_placeholder: bool,
}

#[derive(Serialize, Clone)]
pub struct WorkTimer {
    pub id: i64,
    pub tenant_id: i64,
    pub deployment_id: i64,
    pub user_id: i64,
    pub start_at: String,
    pub end_at: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct Invoice {
    pub id: i64,
    pub tenant_id: i64,
    pub deployment_id: i64,
    pub status: String,
    pub notes: String,
    pub created_at: String,
}

#[derive(Serialize, Clone)]
pub struct InvoiceSummary {
    pub id: i64,
    pub deployment_id: i64,
    pub status: String,
    pub notes: String,
    pub created_at: String,
    pub client_id: i64,
    pub client_name: String,
    pub client_address: String,
    pub client_email: String,
    pub client_currency: String,
    pub crew_id: i64,
    pub crew_name: String,
    pub start_at: String,
    pub end_at: String,
    pub fee_per_hour: f64,
    pub total_hours: f64,
}

#[derive(Serialize, Clone)]
pub struct InvoiceCandidate {
    pub deployment_id: i64,
    pub client_name: String,
    pub crew_name: String,
    pub start_at: String,
    pub end_at: String,
    pub fee_per_hour: f64,
    pub client_currency: String,
    pub total_hours: f64,
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
    pub tenant_name: String,
    pub email: String,
    pub plan_key: String,
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
    pub plan_key: String,
}

#[derive(Serialize, Clone)]
pub struct CrewFormView {
    pub name: String,
    pub status: String,
    pub gear_score: i64,
    pub skill_tags: String,
    pub compatibility_tags: String,
}

#[derive(Serialize, Clone)]
pub struct CrewMemberFormView {
    pub user_id: i64,
    pub name: String,
    pub phone: String,
    pub position: String,
    pub availability_status: String,
}

#[derive(Serialize, Clone)]
pub struct CrewRosterView {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub members_count: i64,
    pub readiness_score: i64,
    pub skill_tags: String,
    pub compatibility_tags: String,
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
pub struct DiscussionFormView {
    pub message: String,
    pub tagged_user_id: Option<i64>,
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
    pub deployment_type: String,
    pub required_skills: String,
    pub compatibility_pref: String,
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
pub struct InvoiceFormView {
    pub deployment_id: i64,
    pub status: String,
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
pub struct WorkspaceThemeView {
    pub app_name: String,
    pub logo_url: String,
    pub theme_key: String,
    pub background_hue: i64,
    pub body_font: String,
    pub heading_font: String,
}

#[derive(Serialize, Clone)]
pub struct WorkspaceBrandView {
    pub app_name: String,
    pub logo_url: String,
    pub theme_key: String,
    pub background_hue: i64,
    pub body_font: String,
    pub heading_font: String,
    pub primary: String,
    pub secondary: String,
    pub on_primary: String,
}

#[derive(Serialize, Clone)]
pub struct ThemeOption {
    pub key: String,
    pub name: String,
    pub primary: String,
    pub secondary: String,
    pub on_primary: String,
}

#[derive(Serialize, Clone)]
pub struct EmailFormView {
    pub subject: String,
    pub body: String,
}

#[derive(Serialize, Clone)]
pub struct WorkspaceRegisterView {
    pub email: String,
}

#[derive(Serialize, Clone)]
pub struct UserSummary {
    pub id: i64,
    pub email: String,
    pub role: String,
}

#[derive(Serialize, Clone)]
pub struct AdminUserSummary {
    pub id: i64,
    pub tenant_id: i64,
    pub tenant_slug: String,
    pub tenant_name: String,
    pub email: String,
    pub role: String,
    pub is_super_admin: bool,
}

#[derive(Serialize, Clone)]
pub struct AdminUserFormView {
    pub tenant_id: i64,
    pub email: String,
    pub role: String,
    pub password: String,
}

#[derive(Serialize, Clone)]
pub struct UserPermission {
    pub resource: String,
    pub can_view: bool,
    pub can_edit: bool,
    pub can_delete: bool,
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
    pub plan_key: String,
    pub plan_expired: bool,
}

impl From<&User> for CurrentUserView {
    fn from(user: &User) -> Self {
        CurrentUserView {
            tenant_slug: user.tenant_slug.clone(),
            email: user.email.clone(),
            plan_key: user.plan_key.clone(),
            plan_expired: user.plan_expired,
        }
    }
}

impl RegisterView {
    pub fn new(
        tenant_name: impl Into<String>,
        email: impl Into<String>,
        plan_key: impl Into<String>,
    ) -> Self {
        RegisterView {
            tenant_name: tenant_name.into(),
            email: email.into(),
            plan_key: plan_key.into(),
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
    pub fn new(slug: impl Into<String>, name: impl Into<String>, plan_key: impl Into<String>) -> Self {
        WorkspaceFormView {
            slug: slug.into(),
            name: name.into(),
            plan_key: plan_key.into(),
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
    pub fn new(
        name: impl Into<String>,
        status: impl Into<String>,
        gear_score: i64,
        skill_tags: impl Into<String>,
        compatibility_tags: impl Into<String>,
    ) -> Self {
        CrewFormView {
            name: name.into(),
            status: status.into(),
            gear_score,
            skill_tags: skill_tags.into(),
            compatibility_tags: compatibility_tags.into(),
        }
    }
}

impl CrewMemberFormView {
    pub fn new(
        user_id: i64,
        name: impl Into<String>,
        phone: impl Into<String>,
        position: impl Into<String>,
        availability_status: impl Into<String>,
    ) -> Self {
        CrewMemberFormView {
            user_id,
            name: name.into(),
            phone: phone.into(),
            position: position.into(),
            availability_status: availability_status.into(),
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

impl DiscussionFormView {
    pub fn new(message: impl Into<String>, tagged_user_id: Option<i64>) -> Self {
        DiscussionFormView {
            message: message.into(),
            tagged_user_id,
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
        deployment_type: impl Into<String>,
        required_skills: impl Into<String>,
        compatibility_pref: impl Into<String>,
    ) -> Self {
        DeploymentFormView {
            client_id,
            crew_id,
            start_at: start_at.into(),
            end_at: end_at.into(),
            fee_per_hour,
            info: info.into(),
            status: status.into(),
            deployment_type: deployment_type.into(),
            required_skills: required_skills.into(),
            compatibility_pref: compatibility_pref.into(),
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

impl InvoiceFormView {
    pub fn new(
        deployment_id: i64,
        status: impl Into<String>,
        notes: impl Into<String>,
    ) -> Self {
        InvoiceFormView {
            deployment_id,
            status: status.into(),
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

impl WorkspaceRegisterView {
    pub fn new(email: impl Into<String>) -> Self {
        WorkspaceRegisterView {
            email: email.into(),
        }
    }
}

impl AdminUserFormView {
    pub fn new(
        tenant_id: i64,
        email: impl Into<String>,
        role: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        AdminUserFormView {
            tenant_id,
            email: email.into(),
            role: role.into(),
            password: password.into(),
        }
    }
}

use serde::{Deserialize, Serialize};

/// Login credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginArgs {
    #[serde(rename = "email")]
    pub username: String,
    pub password: String,
}

/// Login redirect response when regional redirect is needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRedirectResponse {
    pub status: i32,
    pub data: LoginRedirectData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRedirectData {
    pub redirect: bool,
    pub region: String,
}

/// Main login response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub status: i32,
    pub data: LoginResponseData,
}

/// Login response data - can be either complete user data or step data for MFA
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LoginResponseData {
    Redirect(LoginRedirectData),
    Complete(Data),
    Step(StepData),
    Locked(LockedData),
}

/// Locked account data for rate limiting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedData {
    pub code: i32,
    pub data: LockoutInfo,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutInfo {
    pub failures: i32,
    pub interval: i32,
    pub lockout: i32,
}

/// Step data for additional authentication requirements (MFA, email verification, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepData {
    pub step: Step,
    pub user: StepUser,
    pub auth_ticket: AuthTicket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    #[serde(rename = "type")]
    pub step_type: String,
    pub component_name: String,
    pub props: StepProps,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepProps {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepUser {
    pub id: String,
    pub account_type: String,
    pub country: String,
    pub ui_language: String,
}

/// Complete login data with full user information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Data {
    pub user: User,
    pub messages: DataMessages,
    pub notifications: Notifications,
    pub auth_ticket: AuthTicket,
    #[serde(default)]
    pub invitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthTicket {
    pub token: String,
    pub expires: i64,
    pub duration: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DataMessages {
    pub unread: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Notifications {
    pub unresolved: i32,
}

/// User profile information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct User {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub country: String,
    pub ui_language: String,
    pub communication_language: String,
    pub account_type: String,
    pub uom: String,
    pub date_format: String,
    pub time_format: String,
    #[serde(default)]
    pub email_day: Vec<i32>,
    #[serde(default)]
    pub system: System,
    #[serde(default)]
    pub details: Details,
    #[serde(default)]
    pub two_factor: Option<TwoFactor>,
    #[serde(default)]
    pub created: i64,
    #[serde(default)]
    pub last_login: i64,
    #[serde(default)]
    pub programs: Details,
    #[serde(default)]
    pub date_of_birth: i64,
    #[serde(default)]
    pub practices: Details,
    #[serde(default)]
    pub devices: Details,
    #[serde(default)]
    pub consents: Consents,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TwoFactor {
    pub primary_method: String,
    pub primary_value: String,
    pub secondary_method: String,
    pub secondary_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Consents {
    #[serde(default)]
    pub llu: Llu,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Llu {
    pub policy_accept: i64,
    pub tou_accept: i64,
}

/// Empty details object
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Details {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct System {
    pub messages: SystemMessages,
}

impl Default for System {
    fn default() -> Self {
        Self {
            messages: SystemMessages::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SystemMessages {
    pub first_use_phoenix: Option<i64>,
    pub first_use_phoenix_reports_data_merged: Option<i64>,
    pub llu_getting_started_banner: Option<i64>,
    pub llu_new_feature_modal: Option<i64>,
    pub llu_onboarding: Option<i64>,
    pub lv_web_post_release: Option<String>,
    pub app_review_banner: Option<i64>,
    pub streaming_tour_mandatory: Option<i64>,
}

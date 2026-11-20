//! HTTP client and configuration for the LibreLinkUp API.
//!
//! Entry points: [`LibreLinkUpClient`] (authenticated and unauthenticated requests).

use crate::{
    errors::{LibreLinkUpError, Result},
    models::{
        client::{LibreCgmData, ReadRawResponse, ReadResponse, TrendType},
        common::Connection,
        connections::ConnectionsResponse,
        countries::CountryConfigResponse,
        graph::GraphResponse,
        logbook::LogbookResponse,
        login::{AccountResponse, Data as LoginData, LoginArgs, UserResponse},
        notifications::NotificationSettingsResponse,
        region::Region,
    },
    utils::TREND_MAP,
};
use reqwest::{Client, header};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::sync::RwLock;

/// LibreLinkUp app version sent in the `version` header unless [`ClientConfig::api_version`] is set.
pub const DEFAULT_API_VERSION: &str = "4.16.0";

// API endpoint paths, relative to the region's base URL.
const LOGIN_ENDPOINT: &str = "/llu/auth/login";
const CONNECTIONS_ENDPOINT: &str = "/llu/connections";
const COUNTRY_CONFIG_ENDPOINT: &str = "/llu/config/country";
const USER_ENDPOINT: &str = "/user";
const ACCOUNT_ENDPOINT: &str = "/account";
const NOTIFICATIONS_SETTINGS_ENDPOINT: &str = "/llu/notifications/settings";

/// Selector for [`ConnectionIdentifier::ByFunction`]: given the followed patients, returns the
/// `patient_id` of the one to read, or `None` if none fits.
type ConnectionFn = Arc<dyn Fn(&[Connection]) -> Option<String> + Send + Sync>;

/// What a login response tells the client to do next.
#[derive(Debug, PartialEq, Eq)]
enum LoginOutcome {
    /// Logged in; use this token and account id for authenticated requests.
    Authenticated { token: String, account_id: String },
    /// The account lives in another region; log in again there.
    Redirect { region: String },
}

/// Interprets a login response body.
///
/// The body is inspected as untyped JSON first, because the shape of `data` depends on the
/// outcome and step responses (terms of use, privacy policy, ...) vary. Only a successful
/// login is deserialized into a typed struct.
///
/// Errors never contain values from the body (tokens, emails, ...): at most the login
/// status and the names of the keys under `data`.
fn classify_login_response(body: &str) -> Result<LoginOutcome> {
    let value: Value = serde_json::from_str(body).map_err(|e| {
        // serde_json's syntax errors report only the position, not the content.
        LibreLinkUpError::InvalidResponse(format!("login response is not valid JSON: {}", e))
    })?;
    let status = value.get("status").and_then(Value::as_i64);
    let data = value.get("data");
    let field = |path: &str| data.and_then(|d| d.pointer(path));

    if field("/redirect").and_then(Value::as_bool) == Some(true) {
        let region = field("/region").and_then(Value::as_str).ok_or_else(|| {
            unexpected_login_response("login redirect without a region", status, data)
        })?;
        return Ok(LoginOutcome::Redirect {
            region: region.to_string(),
        });
    }

    if let Some(lockout) = field("/data/lockout").and_then(Value::as_i64) {
        return Err(LibreLinkUpError::AccountLocked(
            i32::try_from(lockout).unwrap_or(i32::MAX),
        ));
    }

    match status {
        Some(2) => Err(LibreLinkUpError::BadCredentials),
        Some(4) => {
            let non_empty_str = |path: &str| {
                field(path)
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
            };
            let step = non_empty_str("/step/componentName")
                .or_else(|| non_empty_str("/step/type"))
                .unwrap_or("unknown");
            Err(LibreLinkUpError::AdditionalActionRequired(step.to_string()))
        }
        Some(0) => {
            // The serde error is not included: it can quote values from the body.
            let complete: LoginData = data
                .and_then(|d| serde_json::from_value(d.clone()).ok())
                .ok_or_else(|| {
                    unexpected_login_response("unexpected successful login body", status, data)
                })?;
            Ok(LoginOutcome::Authenticated {
                token: complete.auth_ticket.token,
                account_id: complete.user.id,
            })
        }
        _ => Err(unexpected_login_response(
            "unrecognized login status",
            status,
            data,
        )),
    }
}

/// Builds an `InvalidResponse` for a login body that could not be interpreted, naming only
/// the status and the keys under `data`, never their values.
fn unexpected_login_response(
    reason: &str,
    status: Option<i64>,
    data: Option<&Value>,
) -> LibreLinkUpError {
    let status = status.map_or_else(|| "missing".to_string(), |s| s.to_string());
    let keys = match data {
        Some(Value::Object(map)) => map.keys().cloned().collect::<Vec<_>>().join(", "),
        Some(_) => "<not an object>".to_string(),
        None => "<missing>".to_string(),
    };
    LibreLinkUpError::InvalidResponse(format!(
        "{} (login status: {}, data keys: [{}])",
        reason, status, keys
    ))
}

/// Client configuration. Build it with [`ClientConfig::new`] and the `with_*` methods.
///
/// # Examples
///
/// ```
/// use libre_link_up_api_client::{ClientConfig, Region};
///
/// let config = ClientConfig::new("email@example.com", "password")
///     .with_region(Region::US);
/// ```
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// LibreLinkUp (follower) account email.
    pub username: String,
    /// LibreLinkUp (follower) account password.
    pub password: String,
    /// LibreLinkUp app version to present to the API. `None` uses [`DEFAULT_API_VERSION`].
    /// Raise it if the API starts rejecting requests as coming from an outdated app.
    pub api_version: Option<String>,
    /// API region. `None` uses [`Region::Global`], which redirects to the account's region at login.
    pub region: Option<Region>,
    /// Which followed patient to read. `None` reads the first one the API returns.
    pub connection_identifier: Option<ConnectionIdentifier>,
}

impl ClientConfig {
    /// Create a new configuration with required credentials
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            api_version: None,
            region: None,
            connection_identifier: None,
        }
    }

    /// Set the API version
    #[must_use]
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = Some(version.into());
        self
    }

    /// Set the API region
    #[must_use]
    pub fn with_region(mut self, region: Region) -> Self {
        self.region = Some(region);
        self
    }

    /// Set the connection identifier
    #[must_use]
    pub fn with_connection_identifier(mut self, identifier: ConnectionIdentifier) -> Self {
        self.connection_identifier = Some(identifier);
        self
    }
}

/// Which followed patient to read, for accounts that follow more than one.
///
/// The choice is made on the first read and kept for the client's lifetime.
///
/// # Examples
///
/// ```
/// use libre_link_up_api_client::ConnectionIdentifier;
///
/// // By patient name
/// let by_name = ConnectionIdentifier::ByName("John Doe".to_string());
///
/// // By custom function
/// let by_fn = ConnectionIdentifier::ByFunction(
///     std::sync::Arc::new(|connections| {
///         connections.first().map(|c| c.patient_id.clone())
///     })
/// );
/// ```
#[derive(Clone)]
pub enum ConnectionIdentifier {
    /// Match the patient's full name, `"<first name> <last name>"`, ignoring case.
    ByName(String),
    /// Pick the patient with a function that receives every followed patient and returns
    /// the chosen one's `patient_id`, or `None` if none fits.
    ByFunction(ConnectionFn),
}

impl std::fmt::Debug for ConnectionIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ByName(name) => write!(f, "ByName({})", name),
            Self::ByFunction(_) => write!(f, "ByFunction(<closure>)"),
        }
    }
}

/// Main LibreLinkUp API client
///
/// Handles authentication, token management, and API requests. The same client supports both
/// **authenticated** and **unauthenticated** calls:
///
/// - **Authenticated** (require login; token and account-id are sent): [`read`](Self::read),
///   [`read_raw`](Self::read_raw), [`get_user`](Self::get_user), [`get_account`](Self::get_account),
///   [`get_logbook`](Self::get_logbook), [`get_notification_settings`](Self::get_notification_settings).
///   These use automatic login and token refresh.
/// - **Unauthenticated** (no credentials sent): [`get_country_config`](Self::get_country_config).
///
/// # Examples
///
/// ```no_run
/// use libre_link_up_api_client::{LibreLinkUpClient, ClientConfig, Region};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Simple creation
/// let client = LibreLinkUpClient::simple(
///     "email@example.com".to_string(),
///     "password".to_string(),
///     None,
/// )?;
///
/// // With configuration
/// let config = ClientConfig {
///     username: "email@example.com".to_string(),
///     password: "password".to_string(),
///     api_version: None,
///     region: Some(Region::EU),
///     connection_identifier: None,
/// };
/// let client = LibreLinkUpClient::new(config)?;
/// # Ok(())
/// # }
/// ```
pub struct LibreLinkUpClient {
    config: ClientConfig,
    client: Client,
    base_url: Arc<RwLock<String>>,
    jwt_token: Arc<RwLock<Option<String>>>,
    account_id: Arc<RwLock<Option<String>>>,
    connection_id: Arc<RwLock<Option<String>>>,
}

impl LibreLinkUpClient {
    /// Create a client from a full [`ClientConfig`]. No request is made until the first call;
    /// login happens then.
    ///
    /// # Errors
    ///
    /// - [`LibreLinkUpError::InvalidConfig`] if the username or password is empty, or
    ///   `api_version` is not a valid HTTP header value
    /// - [`LibreLinkUpError::Http`] if the HTTP client cannot be built
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libre_link_up_api_client::{LibreLinkUpClient, ClientConfig, Region};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ClientConfig::new("email@example.com", "password")
    ///     .with_region(Region::EU);
    ///
    /// let client = LibreLinkUpClient::new(config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(config: ClientConfig) -> Result<Self> {
        // Basic validation to avoid confusing HTTP-level errors later
        if config.username.trim().is_empty() {
            return Err(LibreLinkUpError::InvalidConfig(
                "username must not be empty".to_string(),
            ));
        }
        if config.password.is_empty() {
            return Err(LibreLinkUpError::InvalidConfig(
                "password must not be empty".to_string(),
            ));
        }

        let version = config.api_version.as_deref().unwrap_or(DEFAULT_API_VERSION);
        let version_header = header::HeaderValue::from_str(version).map_err(|_| {
            LibreLinkUpError::InvalidConfig(format!("invalid api_version '{}'", version))
        })?;

        let region = config.region.unwrap_or_default();
        let base_url_str = region.base_url().to_string();

        let mut headers = header::HeaderMap::new();
        headers.insert(header::USER_AGENT, "Mozilla/5.0 (iPhone; CPU OS 17_4.1 like Mac OS X) AppleWebKit/536.26 (KHTML, like Gecko) Version/17.4.1 Mobile/10A5355d Safari/8536.25".parse().unwrap());
        headers.insert(header::ACCEPT, "application/json".parse().unwrap());
        headers.insert("accept-encoding", "gzip".parse().unwrap());
        headers.insert("cache-control", "no-cache".parse().unwrap());
        headers.insert("connection", "Keep-Alive".parse().unwrap());
        headers.insert(
            header::CONTENT_TYPE,
            "application/json;charset=UTF-8".parse().unwrap(),
        );
        headers.insert("product", "llu.ios".parse().unwrap());
        headers.insert("version", version_header);
        headers.insert("accept-language", "en-US".parse().unwrap());

        let client: Client = Client::builder()
            .default_headers(headers)
            .gzip(true)
            .build()?;

        Ok(Self {
            config,
            client,
            base_url: Arc::new(RwLock::new(base_url_str)),
            jwt_token: Arc::new(RwLock::new(None)),
            account_id: Arc::new(RwLock::new(None)),
            connection_id: Arc::new(RwLock::new(None)),
        })
    }

    /// Create a client from credentials and an optional region code, with all other settings default.
    ///
    /// # Arguments
    ///
    /// * `username` - LibreLinkUp (follower) account email
    /// * `password` - LibreLinkUp (follower) account password
    /// * `region` - Region code such as `"us"` or `"eu"` (case-insensitive; see [`Region`]).
    ///   `None` or an unrecognized code uses [`Region::Global`], which redirects to the
    ///   account's region at login.
    ///
    /// # Errors
    ///
    /// Same as [`new`](Self::new).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libre_link_up_api_client::LibreLinkUpClient;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LibreLinkUpClient::simple(
    ///     "email@example.com",
    ///     "password",
    ///     None,
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn simple(
        username: impl Into<String>,
        password: impl Into<String>,
        region: Option<&str>,
    ) -> Result<Self> {
        let mut config = ClientConfig::new(username, password);
        if let Some(region) = region {
            config = config.with_region(region.parse().unwrap_or_default());
        }
        Self::new(config)
    }

    /// Log in and store the token and account id, following a regional redirect if needed.
    async fn login(&self) -> Result<()> {
        let base_url = self.base_url.read().await.clone();
        let url = format!("{}{}", base_url, LOGIN_ENDPOINT);

        let login_args = LoginArgs {
            username: self.config.username.clone(),
            password: self.config.password.clone(),
        };

        let response = self.client.post(&url).json(&login_args).send().await?;

        // Check if response is successful
        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read response".to_string());
            return Err(LibreLinkUpError::InvalidResponse(format!(
                "Login failed - HTTP {}: {}",
                status, text
            )));
        }

        let text = response.text().await?;

        match classify_login_response(&text)? {
            LoginOutcome::Redirect { region } => self.handle_redirect(&region).await,
            LoginOutcome::Authenticated { token, account_id } => {
                *self.jwt_token.write().await = Some(token);
                *self.account_id.write().await = Some(account_id);
                Ok(())
            }
        }
    }

    /// Handle regional redirect during login
    async fn handle_redirect(&self, region: &str) -> Result<()> {
        // Parse region string (FromStr never fails, defaults to Global)
        let region_enum = Region::from_str(region).unwrap();
        let region_url = region_enum.base_url().to_string();
        *self.base_url.write().await = region_url;

        // Retry login with new region (using Box::pin for recursion)
        Box::pin(self.login()).await
    }

    /// Make an authenticated request with automatic re-authentication
    async fn authenticated_request<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        // Ensure we're logged in
        if self.jwt_token.read().await.is_none() {
            self.login().await?;
        }

        match self.try_request(path).await {
            Ok(response) => Ok(response),
            Err(_) => {
                // Re-authenticate and retry
                self.login().await?;
                self.try_request(path).await
            }
        }
    }

    /// Try to make a request with current authentication
    async fn try_request<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let base_url = self.base_url.read().await.clone();
        let url = format!("{}{}", base_url, path);

        let jwt_token = self.jwt_token.read().await.clone();
        let account_id = self.account_id.read().await.clone();

        let mut request = self.client.get(&url);

        if let Some(token) = jwt_token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        // Add SHA-256 hashed account-id header if available
        if let Some(id) = account_id {
            let mut hasher = Sha256::new();
            hasher.update(id.as_bytes());
            let hashed_id = format!("{:x}", hasher.finalize());
            request = request.header("account-id", hashed_id);
        }

        let response = request.send().await?;

        // Check if response is successful
        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read response".to_string());
            return Err(LibreLinkUpError::InvalidResponse(format!(
                "request to '{}' failed - HTTP {}: {}",
                path, status, text
            )));
        }

        // Try to parse JSON, with better error message on failure
        let text = response.text().await?;
        serde_json::from_str(&text).map_err(|e| {
            LibreLinkUpError::InvalidResponse(format!("failed to parse JSON for '{}': {}", path, e))
        })
    }

    /// Make an unauthenticated GET request (no Bearer token or account-id).
    /// Use for endpoints that do not require login (e.g. country config).
    async fn unauthenticated_get<T: DeserializeOwned>(
        &self,
        url: &str,
        path_label: &str,
    ) -> Result<T> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read response".to_string());
            return Err(LibreLinkUpError::InvalidResponse(format!(
                "request to '{}' failed - HTTP {}: {}",
                path_label, status, body
            )));
        }
        let body: String = response.text().await?;
        serde_json::from_str(&body).map_err(|e| {
            LibreLinkUpError::InvalidResponse(format!(
                "failed to parse JSON for '{}': {}",
                path_label, e
            ))
        })
    }

    /// Get list of connections
    async fn get_connections(&self) -> Result<ConnectionsResponse> {
        self.authenticated_request(CONNECTIONS_ENDPOINT).await
    }

    /// Get current user profile (authenticated).
    ///
    /// Returns user info, messages, notifications, and auth ticket.
    ///
    /// # Errors
    ///
    /// Returns [`LibreLinkUpError::Http`] or [`LibreLinkUpError::InvalidResponse`] on failure.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libre_link_up_api_client::LibreLinkUpClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LibreLinkUpClient::simple(
    ///     "user@example.com".to_string(),
    ///     "password".to_string(),
    ///     None,
    /// )?;
    /// let user = client.get_user().await?;
    /// println!("{} {}", user.data.user.first_name, user.data.user.last_name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_user(&self) -> Result<UserResponse> {
        self.authenticated_request(USER_ENDPOINT).await
    }

    /// Get account info (authenticated).
    ///
    /// Returns user profile and current auth ticket.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libre_link_up_api_client::LibreLinkUpClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LibreLinkUpClient::simple(
    ///     "user@example.com".to_string(),
    ///     "password".to_string(),
    ///     None,
    /// )?;
    /// let account = client.get_account().await?;
    /// println!("{} {}", account.data.user.first_name, account.data.user.last_name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_account(&self) -> Result<AccountResponse> {
        self.authenticated_request(ACCOUNT_ENDPOINT).await
    }

    /// Get a patient's logbook: glucose events and alarms (authenticated).
    ///
    /// Entries are raw API data; `value` is in the account's display unit, so use
    /// `value_in_mg_per_dl` for a fixed unit.
    ///
    /// # Arguments
    ///
    /// * `patient_id` - [`Connection::patient_id`], e.g. from
    ///   [`read_raw`](Self::read_raw)`().await?.connection.patient_id`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libre_link_up_api_client::LibreLinkUpClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LibreLinkUpClient::simple("user@example.com", "password", None)?;
    /// let patient_id = client.read_raw().await?.connection.patient_id;
    /// let logbook = client.get_logbook(&patient_id).await?;
    /// for entry in &logbook.data {
    ///     println!("{} {} mg/dL", entry.timestamp, entry.value_in_mg_per_dl);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_logbook(&self, patient_id: &str) -> Result<LogbookResponse> {
        let path = format!("{}/{}/logbook", CONNECTIONS_ENDPOINT, patient_id);
        self.authenticated_request(&path).await
    }

    /// Get notification settings for a connection (authenticated).
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Connection/patient ID (same as used for [`read`](Self::read) / [`read_raw`](Self::read_raw)).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libre_link_up_api_client::LibreLinkUpClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LibreLinkUpClient::simple(
    ///     "user@example.com".to_string(),
    ///     "password".to_string(),
    ///     None,
    /// )?;
    /// let settings = client.get_notification_settings("connection-id").await?;
    /// println!("Alarms enabled: {}", settings.data.alarm_rules.c);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_notification_settings(
        &self,
        connection_id: &str,
    ) -> Result<NotificationSettingsResponse> {
        let path = format!("{}/{}", NOTIFICATIONS_SETTINGS_ENDPOINT, connection_id);
        self.authenticated_request(&path).await
    }

    /// Fetch country/region config (unauthenticated).
    ///
    /// Uses the global API endpoint. Does not require login.
    ///
    /// # Arguments
    ///
    /// * `country` - Country code (e.g. `"us"`, `"eu"`).
    /// * `version` - API version (e.g. `"4.16.0"`). If `None`, uses the client's configured version.
    ///
    /// # Errors
    ///
    /// Returns [`LibreLinkUpError::Http`] or [`LibreLinkUpError::InvalidResponse`] on failure.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libre_link_up_api_client::LibreLinkUpClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LibreLinkUpClient::simple(
    ///     "user@example.com".to_string(),
    ///     "password".to_string(),
    ///     None,
    /// )?;
    /// let config = client.get_country_config("us", None).await?;
    /// println!("Min version: {:?}", config.data.min_version);
    /// println!("Regional map: {:?}", config.data.regional_map);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_country_config(
        &self,
        country: &str,
        version: Option<&str>,
    ) -> Result<CountryConfigResponse> {
        let version =
            version.unwrap_or_else(|| self.config.api_version.as_deref().unwrap_or("4.16.0"));
        let url = format!(
            "{}{}?country={}&version={}",
            Region::Global.base_url(),
            COUNTRY_CONFIG_ENDPOINT,
            country,
            version,
        );
        self.unauthenticated_get(&url, COUNTRY_CONFIG_ENDPOINT)
            .await
    }

    /// Get connection ID by identifier
    fn get_connection_id(&self, connections: &[Connection]) -> Result<String> {
        match &self.config.connection_identifier {
            Some(ConnectionIdentifier::ByName(name)) => {
                let connection = connections
                    .iter()
                    .find(|c| {
                        format!("{} {}", c.first_name, c.last_name).to_lowercase()
                            == name.to_lowercase()
                    })
                    .ok_or_else(|| LibreLinkUpError::ConnectionNotFound(name.clone()))?;

                Ok(connection.patient_id.clone())
            }
            Some(ConnectionIdentifier::ByFunction(func)) => {
                func(connections).ok_or(LibreLinkUpError::ConnectionFunctionFailed)
            }
            None => {
                // Default to first connection
                connections
                    .first()
                    .map(|c| c.patient_id.clone())
                    .ok_or(LibreLinkUpError::NoConnections)
            }
        }
    }

    /// Fetch the selected patient's graph data as the API returns it.
    ///
    /// Use this when you need fields that [`read`](Self::read) drops (patient details, sensor,
    /// alarm rules, the display unit). Glucose values here are not normalized; see
    /// [`ReadRawResponse`].
    ///
    /// Logs in on first use and again if a request fails. The patient is chosen on the first
    /// call (see [`ClientConfig::connection_identifier`]) and reused afterwards.
    ///
    /// # Errors
    ///
    /// - Login errors: [`LibreLinkUpError::BadCredentials`], [`LibreLinkUpError::AccountLocked`],
    ///   [`LibreLinkUpError::AdditionalActionRequired`]
    /// - [`LibreLinkUpError::NoConnections`] if the account follows no patients
    /// - [`LibreLinkUpError::ConnectionNotFound`] / [`LibreLinkUpError::ConnectionFunctionFailed`]
    ///   if the configured [`ConnectionIdentifier`] matches no patient
    /// - [`LibreLinkUpError::Http`] for network errors
    /// - [`LibreLinkUpError::InvalidResponse`] for an error status or unexpected response body
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libre_link_up_api_client::LibreLinkUpClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LibreLinkUpClient::simple("email@example.com", "password", None)?;
    ///
    /// let raw = client.read_raw().await?;
    /// println!("Connection: {:?}", raw.connection);
    /// println!("Active sensors: {}", raw.active_sensors.len());
    /// println!("Graph data points: {}", raw.graph_data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read_raw(&self) -> Result<ReadRawResponse> {
        let connection_id = if let Some(id) = self.connection_id.read().await.clone() {
            id
        } else {
            let connections = self.get_connections().await?;

            if connections.data.is_empty() {
                return Err(LibreLinkUpError::NoConnections);
            }

            let id = self.get_connection_id(&connections.data)?;
            *self.connection_id.write().await = Some(id.clone());
            id
        };

        let path = format!("{}/{}/graph", CONNECTIONS_ENDPOINT, connection_id);
        let graph_response: GraphResponse = self.authenticated_request(&path).await?;

        Ok(ReadRawResponse {
            connection: graph_response.data.connection,
            active_sensors: graph_response.data.active_sensors,
            graph_data: graph_response.data.graph_data,
        })
    }

    /// Fetch the selected patient's latest reading and recent history, normalized to
    /// [`LibreCgmData`] (mg/dL, UTC timestamps).
    ///
    /// History points whose timestamp cannot be parsed are left out of
    /// [`ReadResponse::history`].
    ///
    /// # Errors
    ///
    /// Everything [`read_raw`](Self::read_raw) can return, plus
    /// [`LibreLinkUpError::InvalidResponse`] if the latest reading's timestamp cannot be parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libre_link_up_api_client::LibreLinkUpClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LibreLinkUpClient::simple("email@example.com", "password", None)?;
    ///
    /// let data = client.read().await?;
    /// println!("Current glucose: {:.1} mg/dL", data.current.value_mg_dl);
    /// println!("Trend: {:?}", data.current.trend);
    /// println!("Historical readings: {}", data.history.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read(&self) -> Result<ReadResponse> {
        let raw = self.read_raw().await?;
        let current = LibreCgmData::try_from(&raw.connection.glucose_measurement)?;
        // A history point with an unparseable timestamp cannot be placed in time; drop it
        // rather than failing the whole read or inventing a time.
        let history = raw
            .graph_data
            .iter()
            .filter_map(|item| LibreCgmData::try_from(item).ok())
            .collect();
        Ok(ReadResponse { current, history })
    }

    /// Start a background task that polls [`read`](Self::read) and reports an average every
    /// `sample_count` distinct readings.
    ///
    /// Every `interval` the task reads the latest reading and keeps it if its timestamp is new.
    /// Once `sample_count` readings are collected it calls
    /// `on_average(average, samples, history)` and starts collecting again:
    ///
    /// * `average` - mean `value_mg_dl` (rounded) and mean trend of `samples`, with the
    ///   timestamp, `is_high` and `is_low` of the newest sample. Samples without a computable
    ///   trend are left out of the trend mean; if none has one, the trend is `NotComputable`.
    /// * `samples` - the readings that were averaged, oldest first.
    /// * `history` - [`ReadResponse::history`] from the latest poll.
    ///
    /// Failed polls are skipped silently. The task uses its own client (and login session)
    /// built from this client's configuration, and runs until you call `abort()` on the
    /// returned handle.
    ///
    /// # Errors
    ///
    /// - [`LibreLinkUpError::InvalidConfig`] if `sample_count` or `interval` is zero
    /// - Anything [`new`](Self::new) can return
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libre_link_up_api_client::LibreLinkUpClient;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LibreLinkUpClient::simple("email@example.com", "password", None)?;
    ///
    /// let handle = client.poll_averaged(
    ///     5,
    ///     Duration::from_secs(60),
    ///     |average, samples, _history| {
    ///         println!(
    ///             "Average of {} readings: {:.0} mg/dL ({:?})",
    ///             samples.len(),
    ///             average.value_mg_dl,
    ///             average.trend
    ///         );
    ///     },
    /// ).await?;
    ///
    /// // Later, to stop polling:
    /// handle.abort();
    /// # Ok(())
    /// # }
    /// ```
    pub async fn poll_averaged<F>(
        &self,
        sample_count: usize,
        interval: Duration,
        mut on_average: F,
    ) -> Result<tokio::task::JoinHandle<()>>
    where
        F: FnMut(LibreCgmData, Vec<LibreCgmData>, Vec<LibreCgmData>) + Send + 'static,
    {
        if sample_count == 0 {
            return Err(LibreLinkUpError::InvalidConfig(
                "sample_count must be at least 1".to_string(),
            ));
        }
        if interval.is_zero() {
            return Err(LibreLinkUpError::InvalidConfig(
                "interval must be greater than zero".to_string(),
            ));
        }

        let client = Self::new(self.config.clone())?;

        let handle = tokio::spawn(async move {
            let mut memory: Vec<LibreCgmData> = Vec::new();
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                if let Ok(read_response) = client.read().await {
                    let current = read_response.current;
                    let history = read_response.history;

                    // Check if we already have this reading
                    if !memory.iter().any(|m| m.timestamp == current.timestamp) {
                        memory.push(current.clone());
                    }

                    if memory.len() >= sample_count {
                        let avg_value =
                            memory.iter().map(|m| m.value_mg_dl).sum::<f64>() / memory.len() as f64;

                        // NotComputable sits at index 0 of TREND_MAP; averaging it in would
                        // bias the result toward "falling", so only computable trends count.
                        let trend_indices: Vec<usize> = memory
                            .iter()
                            .filter(|m| m.trend != TrendType::NotComputable)
                            .filter_map(|m| TREND_MAP.iter().position(|&t| t == m.trend))
                            .collect();

                        let avg_trend = if trend_indices.is_empty() {
                            TrendType::NotComputable
                        } else {
                            let idx = (trend_indices.iter().sum::<usize>() as f64
                                / trend_indices.len() as f64)
                                .round() as usize;
                            TREND_MAP[idx]
                        };

                        let averaged = LibreCgmData {
                            value_mg_dl: avg_value.round(),
                            is_high: current.is_high,
                            is_low: current.is_low,
                            trend: avg_trend,
                            timestamp: current.timestamp,
                        };

                        // Move the collected readings into the callback without cloning
                        let collected = std::mem::take(&mut memory);
                        on_average(averaged, collected, history);
                    }
                }
            }
        });

        Ok(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_invalid_config<T>(result: Result<T>) -> bool {
        matches!(result, Err(LibreLinkUpError::InvalidConfig(_)))
    }

    #[test]
    fn empty_credentials_are_invalid_config() {
        assert!(is_invalid_config(LibreLinkUpClient::simple(
            " ", "pw", None
        )));
        assert!(is_invalid_config(LibreLinkUpClient::simple(
            "me@example.com",
            "",
            None
        )));
    }

    #[test]
    fn bad_api_version_is_invalid_config_not_a_panic() {
        let config = ClientConfig::new("me@example.com", "pw").with_api_version("4.16\n0");
        assert!(is_invalid_config(LibreLinkUpClient::new(config)));
    }

    #[tokio::test]
    async fn poll_averaged_rejects_zero_sample_count_and_interval() {
        let client = LibreLinkUpClient::simple("me@example.com", "pw", None).unwrap();
        let noop = |_, _, _| {};
        assert!(is_invalid_config(
            client.poll_averaged(0, Duration::from_secs(1), noop).await
        ));
        assert!(is_invalid_config(
            client.poll_averaged(1, Duration::ZERO, noop).await
        ));
    }

    mod login_response {
        use super::*;
        use crate::models::login::{LoginResponse, LoginResponseData};
        use serde_json::json;

        const TOKEN: &str = "secret-token-value";

        fn classify(body: Value) -> Result<LoginOutcome> {
            classify_login_response(&body.to_string())
        }

        fn tou_step() -> Value {
            json!({"status": 4, "data": {
                "step": {"type": "tou", "componentName": "tou", "props": {"reaccept": true, "type": "tou"}},
                "user": {"accountType": "pat", "country": "DE", "uiLanguage": "de-DE"},
                "authTicket": {"token": TOKEN, "expires": 1700000000, "duration": 3600000}
            }})
        }

        fn action_required(result: Result<LoginOutcome>) -> String {
            match result {
                Err(LibreLinkUpError::AdditionalActionRequired(step)) => step,
                other => panic!("expected AdditionalActionRequired, got {:?}", other),
            }
        }

        #[test]
        fn terms_of_use_step_is_action_required() {
            assert_eq!(action_required(classify(tou_step())), "tou");
        }

        #[test]
        fn privacy_policy_step_is_action_required() {
            let body = json!({"status": 4, "data": {
                "step": {"type": "pp", "componentName": "pp", "props": {"reaccept": true, "type": "pp"}},
                "user": {"accountType": "pat", "country": "DE", "uiLanguage": "de-DE"},
                "authTicket": {"token": TOKEN, "expires": 1700000000, "duration": 3600000}
            }});
            assert_eq!(action_required(classify(body)), "pp");
        }

        #[test]
        fn step_without_component_name_uses_type() {
            let body = json!({"status": 4, "data": {"step": {"type": "verifyEmail"}}});
            assert_eq!(action_required(classify(body)), "verifyEmail");
        }

        #[test]
        fn step_without_name_or_type_is_unknown() {
            let body = json!({"status": 4, "data": {}});
            assert_eq!(action_required(classify(body)), "unknown");
        }

        #[test]
        fn lockout_is_account_locked() {
            let body = json!({"status": 429, "data": {
                "code": 60, "data": {"failures": 5, "interval": 60, "lockout": 300}, "message": "locked"
            }});
            assert!(matches!(
                classify(body),
                Err(LibreLinkUpError::AccountLocked(300))
            ));
        }

        #[test]
        fn status_2_is_bad_credentials() {
            let body = json!({"status": 2, "error": {"message": "notAuthenticated"}});
            assert!(matches!(
                classify(body),
                Err(LibreLinkUpError::BadCredentials)
            ));
        }

        #[test]
        fn redirect_returns_region() {
            let body = json!({"status": 0, "data": {"redirect": true, "region": "eu"}});
            assert_eq!(
                classify(body).unwrap(),
                LoginOutcome::Redirect {
                    region: "eu".to_string()
                }
            );
        }

        #[test]
        fn minimal_successful_login_is_authenticated() {
            let body = json!({"status": 0, "data": {
                "user": {"id": "user-123"},
                "messages": {},
                "notifications": {},
                "authTicket": {"token": TOKEN, "expires": 1700000000, "duration": 3600000}
            }});
            assert_eq!(
                classify(body).unwrap(),
                LoginOutcome::Authenticated {
                    token: TOKEN.to_string(),
                    account_id: "user-123".to_string()
                }
            );
        }

        fn invalid_response_message(result: Result<LoginOutcome>) -> String {
            match result {
                Err(LibreLinkUpError::InvalidResponse(msg)) => msg,
                other => panic!("expected InvalidResponse, got {:?}", other),
            }
        }

        #[test]
        fn unknown_status_is_invalid_response_without_values() {
            let mut body = tou_step();
            body["status"] = json!(7);
            let msg = invalid_response_message(classify(body));
            assert!(msg.contains("login status: 7"), "{}", msg);
            assert!(msg.contains("authTicket"), "{}", msg);
            assert!(!msg.contains(TOKEN), "{}", msg);
            assert!(!msg.contains("DE"), "{}", msg);
        }

        #[test]
        fn malformed_successful_login_is_invalid_response_without_values() {
            let body = json!({"status": 0, "data": {"authTicket": {"token": TOKEN}}});
            let msg = invalid_response_message(classify(body));
            assert!(msg.contains("login status: 0"), "{}", msg);
            assert!(!msg.contains(TOKEN), "{}", msg);
        }

        #[test]
        fn public_login_response_type_accepts_terms_step() {
            let parsed: LoginResponse = serde_json::from_value(tou_step()).unwrap();
            match parsed.data {
                LoginResponseData::Step(step) => assert_eq!(step.step.component_name, "tou"),
                other => panic!("expected Step, got {:?}", other),
            }
        }
    }
}

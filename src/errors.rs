//! Error types for LibreLinkUp API operations

use thiserror::Error;

/// Error types that can occur when using the LibreLinkUp API client
///
/// # Examples
///
/// ```no_run
/// use libre_link_up_api_client::{LibreLinkUpClient, LibreLinkUpError};
///
/// # async fn example() {
/// let client = LibreLinkUpClient::simple(
///     "email@example.com".to_string(),
///     "password".to_string(),
///     None,
/// );
///
/// match client {
///     Ok(c) => println!("Client created"),
///     Err(LibreLinkUpError::Http(e)) => eprintln!("Network error: {}", e),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// # }
/// ```
#[derive(Debug, Error)]
pub enum LibreLinkUpError {
    /// The request could not be sent or the response could not be read (network, TLS,
    /// timeout, or failing to build the HTTP client).
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Login was rejected. Check that these are LibreLinkUp (follower) credentials,
    /// not LibreLink (sensor wearer) credentials.
    #[error(
        "Bad credentials. Please ensure that you have entered the credentials of your LibreLinkUp account (and not of your LibreLink account)."
    )]
    BadCredentials,

    /// Too many failed logins. Holds the number of seconds to wait before trying again.
    #[error(
        "Account temporarily locked due to multiple failed login attempts. Please wait {0} seconds and try again."
    )]
    AccountLocked(i32),

    /// The account must complete a step in the LibreLinkUp app first, such as accepting
    /// new terms. Holds the name of the step the API asked for.
    #[error(
        "Additional action required for your account: {0}. Please login via app and perform required steps and try again."
    )]
    AdditionalActionRequired(String),

    /// No followed patient matched [`ConnectionIdentifier::ByName`](crate::ConnectionIdentifier::ByName).
    /// Holds the name that was searched for.
    #[error("Unable to identify connection by name '{0}'")]
    ConnectionNotFound(String),

    /// The [`ConnectionIdentifier::ByFunction`](crate::ConnectionIdentifier::ByFunction)
    /// selector returned `None`.
    #[error("Unable to identify connection by the provided function")]
    ConnectionFunctionFailed,

    /// The account does not follow any patients yet.
    #[error("Your account does not follow any patients. Please start following and try again.")]
    NoConnections,

    /// The client configuration is unusable, e.g. an empty username or password.
    /// Returned when constructing the client or starting a poller, before any request.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// The API answered with a non-success HTTP status, a body that does not match the
    /// expected shape, or data that cannot be interpreted (such as an unparseable timestamp).
    #[error("Invalid API response: {0}")]
    InvalidResponse(String),
}

/// Result type for LibreLinkUp API operations.
pub type Result<T> = std::result::Result<T, LibreLinkUpError>;

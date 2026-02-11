# LibreLinkUp API Client (Rust)

Unofficial Rust client for the LibreLinkUp API - fetches CGM (Continuous Glucose Monitor) data from FreeStyle Libre 2/3 devices via Abbott's sharing service.

This is a Rust implementation inspired by [TypeScript libre-link-up-api-client](https://github.com/DiaKEM/libre-link-up-api-client) and [LibreLinkUp Status Bar Extension](https://github.com/borkod/librelinkup-vs-code-extension).

## Features

- ✅ Automatic authentication and token management
- ✅ Regional endpoint handling (US, EU, EU2, JP, DE, FR, AP, AU, AE, CA, LA, RU, CN)
- ✅ Read current and historical glucose data
- ✅ Raw API response access
- ✅ Averaged glucose readings over time
- ✅ Type-safe API with proper error handling
- ✅ Async/await support with Tokio
- ✅ Support for authenticated and unauthenticated endpoints

## Documentation

- **Libre APIs**: Libre APIs are documented in the [LibreView Unofficial API Documentation](https://libreview-unofficial.stoplight.io/docs/libreview-unofficial/8i2x0tc4qumh2-authentication) (external resource).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
libre_link_up_api_client = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

### Simple Usage

```rust
use libre_link_up_api_client::LibreLinkUpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with credentials
    let client = LibreLinkUpClient::simple(
        "your_email@example.com".to_string(),
        "your_password".to_string(),
        None,
    )?;

    // Read current glucose data
    let data = client.read().await?;

    println!("Current glucose: {:.1} mg/dL", data.current.value);
    println!("Trend: {:?}", data.current.trend);
    println!("Historical readings: {}", data.history.len());

    Ok(())
}
```

### Advanced Configuration

```rust
use libre_link_up_api_client::{ClientConfig, ConnectionIdentifier, LibreLinkUpClient, Region};

let config = ClientConfig {
    username: "your_email@example.com".to_string(),
    password: "your_password".to_string(),
    api_version: Some("4.16.0".to_string()),
    region: Some(Region::EU),
    connection_identifier: Some(ConnectionIdentifier::ByName("John Doe".to_string())),
};

let client = LibreLinkUpClient::new(config)?;
```

## API Methods

### Client Creation

#### `LibreLinkUpClient::simple(username, password, region)`
Create a basic client with default settings.

#### `LibreLinkUpClient::new(config)`
Create a client with custom configuration.

### Authenticated Endpoints

All authenticated endpoints automatically handle login and token refresh.

#### `client.read() -> Result<ReadResponse>`
Returns current glucose reading + historical data.

**Response:**
```rust
ReadResponse {
    current: LibreCgmData,      // Latest reading
    history: Vec<LibreCgmData>, // Historical readings
}
```

#### `client.read_raw() -> Result<ReadRawResponse>`
Returns raw API response with all details.

**Response:**
```rust
ReadRawResponse {
    connection: Connection,           // Patient connection info
    active_sensors: Vec<ActiveSensor>, // Sensor details
    graph_data: Vec<GlucoseItem>,    // Raw glucose readings
}
```

#### `client.get_user() -> Result<UserResponse>`
Get current user profile with messages, notifications, and auth ticket.

**Response:**
```rust
UserResponse {
    status: i32,
    data: Data {
        user: User,
        messages: DataMessages,
        notifications: Notifications,
        auth_ticket: AuthTicket,
        invitations: Option<Vec<String>>,
        trusted_device_token: String,
    },
}
```

#### `client.get_account() -> Result<AccountResponse>`
Get account information with user profile and current auth ticket.

**Response:**
```rust
AccountResponse {
    status: i32,
    data: AccountData {
        user: User,
    },
    ticket: AuthTicket,
}
```

#### `client.get_logbook(patient_id: &str) -> Result<LogbookResponse>`
Get logbook (glucose events/alarms) for a patient.

**Response:**
```rust
LogbookResponse {
    status: i32,
    data: Vec<LogbookEntry>,
    ticket: AuthTicket,
}
```

#### `client.get_notification_settings(connection_id: &str) -> Result<NotificationSettingsResponse>`
Get notification settings for a connection.

**Response:**
```rust
NotificationSettingsResponse {
    status: i32,
    data: NotificationSettingsData {
        connection_id: String,
        alarm_rules: NotificationSettingsAlarmRules,
        std: Std,
        patient_device: NotificationSettingsPatientDevice,
    },
    ticket: AuthTicket,
}
```

#### `client.read_averaged(amount, callback, interval_ms) -> Result<JoinHandle>`
Polls the API repeatedly and calculates averages.

**Parameters:**
- `amount`: Number of readings to collect before calculating average
- `callback`: Function called with `(average, readings, history)`
- `interval_ms`: Milliseconds between API calls

**Returns:** `JoinHandle` that can be aborted to stop polling.

### Unauthenticated Endpoints

#### `client.get_country_config(country: &str, version: Option<&str>) -> Result<CountryConfigResponse>`
Fetch country/region configuration (does not require login).

**Example:**
```rust
let config = client.get_country_config("us", None).await?;
println!("Min version: {:?}", config.data.min_version);
```

#### `get_country_config(country: &str, version: &str) -> Result<CountryConfigResponse>`
Standalone function to fetch country config without creating a client.

**Example:**
```rust
use libre_link_up_api_client::get_country_config;

let config = get_country_config("us", "4.16.0").await?;
```

## Data Types

### `LibreCgmData`
```rust
pub struct LibreCgmData {
    pub value: f64,           // Glucose in mg/dL
    pub is_high: bool,        // Above target range
    pub is_low: bool,         // Below target range
    pub trend: TrendType,     // Arrow direction
    pub date: DateTime<Utc>,  // Timestamp
}
```

### `TrendType`
```rust
pub enum TrendType {
    SingleDown,      // ↓↓ Falling fast
    FortyFiveDown,   // ↘ Falling
    Flat,            // → Stable
    FortyFiveUp,     // ↗ Rising
    SingleUp,        // ↑↑ Rising fast
    NotComputable,   // ? Unknown
}
```

### `Region`
```rust
pub enum Region {
    Global,  // Auto-redirects
    US, EU, EU2, FR, JP, DE, AP, AU, AE, CA, LA, RU, CN
}
```

## Connection Identifiers

### By Name
```rust
ConnectionIdentifier::ByName("John Doe".to_string())
```

### By Custom Function
```rust
use std::sync::Arc;

ConnectionIdentifier::ByFunction(Arc::new(|connections| {
    connections.iter()
        .find(|c| c.patient_id == "12345")
        .map(|c| c.patient_id.clone())
}))
```

### None (Default)
Uses the first available connection.

## Error Handling

```rust
use libre_link_up_api_client::LibreLinkUpError;

match client.read().await {
    Ok(data) => println!("Glucose: {}", data.current.value),
    Err(LibreLinkUpError::BadCredentials) => {
        eprintln!("Invalid username or password");
    }
    Err(LibreLinkUpError::NoConnections) => {
        eprintln!("No patients being followed");
    }
    Err(LibreLinkUpError::AccountLocked(lockout)) => {
        eprintln!("Account locked for {} seconds", lockout);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Examples

Run the examples:

```bash
# Basic usage
cargo run --example basic_usage

# Averaged readings
cargo run --example averaged_reading
```

## Regional Support

The client automatically handles regional redirects. Supported regions:
- 🇺🇸 US (api-us.libreview.io)
- 🇪🇺 EU (api-eu.libreview.io)
- 🇪🇺 EU2 (api-eu2.libreview.io)
- 🇫🇷 FR (api-fr.libreview.io)
- 🇯🇵 JP (api-jp.libreview.io)
- 🇩🇪 DE (api-de.libreview.io)
- 🇦🇺 AU (api-au.libreview.io)
- 🇦🇪 AE (api-ae.libreview.io)
- 🌏 AP (api-ap.libreview.io)
- 🇨🇦 CA (api-ca.libreview.io)
- 🌎 LA (api-la.libreview.io)
- 🇷🇺 RU (api.libreview.ru)
- 🇨🇳 CN (api-cn.myfreestyle.cn)

## License

MIT License

## Disclaimer

This is an unofficial client. Use at your own risk. The API is not officially documented and may change without notice.

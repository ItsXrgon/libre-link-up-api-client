//! Processed glucose and read-response types ([`read`](crate::LibreLinkUpClient::read), [`read_raw`](crate::LibreLinkUpClient::read_raw)).

use crate::models::common::{ActiveSensor, Connection, GlucoseItem};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Direction and speed of glucose change, as shown by the arrow in the LibreLink app.
///
/// Mapped from the API's `TrendArrow` field (1–5). A missing or unknown arrow maps to
/// [`NotComputable`](Self::NotComputable), never to [`Flat`](Self::Flat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendType {
    /// Falling quickly (↓)
    SingleDown,
    /// Falling (↘)
    FortyFiveDown,
    /// Changing slowly (→)
    Flat,
    /// Rising (↗)
    FortyFiveUp,
    /// Rising quickly (↑)
    SingleUp,
    /// The sensor could not compute a trend, or the API sent none.
    NotComputable,
}

/// One glucose reading, normalized for consumption.
///
/// Returned by [`read`](crate::LibreLinkUpClient::read). Raw API readings from
/// [`read_raw`](crate::LibreLinkUpClient::read_raw) can be converted with `TryFrom`:
/// `LibreCgmData::try_from(&raw.connection.glucose_measurement)`.
///
/// # Examples
///
/// ```
/// use libre_link_up_api_client::{LibreCgmData, TrendType};
/// use chrono::Utc;
///
/// let data = LibreCgmData {
///     value_mg_dl: 120.0,
///     is_high: false,
///     is_low: false,
///     trend: TrendType::Flat,
///     timestamp: Utc::now(),
/// };
/// let mmol_l = data.value_mg_dl / 18.0;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibreCgmData {
    /// Glucose value in mg/dL, whatever display unit the LibreLinkUp account uses.
    #[serde(rename = "valueMgDl")]
    pub value_mg_dl: f64,
    /// The API's `isHigh` flag for this reading.
    #[serde(rename = "isHigh")]
    pub is_high: bool,
    /// The API's `isLow` flag for this reading.
    #[serde(rename = "isLow")]
    pub is_low: bool,
    /// Trend arrow at the time of the reading.
    pub trend: TrendType,
    /// When the sensor took the reading (the API's `FactoryTimestamp`, which is UTC).
    pub timestamp: DateTime<Utc>,
}

/// Latest reading plus recent history for the selected patient, returned by
/// [`read`](crate::LibreLinkUpClient::read).
///
/// # Examples
///
/// ```no_run
/// use libre_link_up_api_client::LibreLinkUpClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = LibreLinkUpClient::simple("email@example.com", "password", None)?;
///
/// let response = client.read().await?;
/// println!("Current: {:.1} mg/dL", response.current.value_mg_dl);
/// println!("History: {} readings", response.history.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadResponse {
    /// Most recent reading.
    pub current: LibreCgmData,
    /// Earlier readings from the API's graph data, in the order the API returns them.
    ///
    /// The API keeps them at a coarser interval than live readings and may not include
    /// `current`. Points whose timestamp cannot be parsed are left out.
    pub history: Vec<LibreCgmData>,
}

/// The API's graph response for the selected patient, as deserialized and otherwise
/// unprocessed, returned by [`read_raw`](crate::LibreLinkUpClient::read_raw).
///
/// Glucose fields here are as the API sends them: `Value` is in the account's display unit,
/// so prefer `value_in_mg_per_dl`, or convert with `LibreCgmData::try_from`.
///
/// # Examples
///
/// ```no_run
/// use libre_link_up_api_client::{LibreCgmData, LibreLinkUpClient};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = LibreLinkUpClient::simple("email@example.com", "password", None)?;
///
/// let raw = client.read_raw().await?;
/// println!("Patient ID: {}", raw.connection.patient_id);
/// let current = LibreCgmData::try_from(&raw.connection.glucose_measurement)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadRawResponse {
    /// The followed patient, including the latest reading (`glucose_measurement`).
    pub connection: Connection,
    /// Sensors currently active for the patient.
    pub active_sensors: Vec<ActiveSensor>,
    /// Historical readings (the API's `graphData`).
    pub graph_data: Vec<GlucoseItem>,
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Trend direction for glucose readings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendType {
    SingleDown,
    FortyFiveDown,
    Flat,
    FortyFiveUp,
    SingleUp,
    NotComputable,
}

/// Processed glucose data for consumption
///
/// # Examples
///
/// ```
/// use libre_link_up_api_client::{LibreCgmData, TrendType};
/// use chrono::Utc;
///
/// let data = LibreCgmData {
///     value: 120.0,
///     is_high: false,
///     is_low: false,
///     trend: TrendType::Flat,
///     date: Utc::now(),
/// };
/// assert_eq!(data.value, 120.0);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibreCgmData {
    /// Glucose value in mg/dL
    pub value: f64,
    /// Whether the value is above the target high
    #[serde(rename = "isHigh")]
    pub is_high: bool,
    /// Whether the value is below the target low
    #[serde(rename = "isLow")]
    pub is_low: bool,
    /// Trend direction
    pub trend: TrendType,
    /// Timestamp of the reading
    pub date: DateTime<Utc>,
}

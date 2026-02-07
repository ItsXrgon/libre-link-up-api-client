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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

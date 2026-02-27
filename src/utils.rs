//! Utilities for mapping API glucose data to [`LibreCgmData`].
//!
//! Main entry: [`map_glucose_data`].

use crate::models::{
    LibreCgmData,
    client::TrendType,
    common::{GlucoseItem, GlucoseMeasurement},
};
use chrono::Utc;

/// Maps API trend arrow index (0–6) to [`TrendType`]. Used when converting raw readings.
pub const TREND_MAP: [TrendType; 7] = [
    TrendType::NotComputable,
    TrendType::SingleDown,
    TrendType::FortyFiveDown,
    TrendType::Flat,
    TrendType::FortyFiveUp,
    TrendType::SingleUp,
    TrendType::NotComputable,
];

/// Converts an API trend arrow index to [`TrendType`]. Returns [`TrendType::Flat`] if missing or out of range.
pub fn get_trend(trend_arrow: Option<i32>) -> TrendType {
    trend_arrow
        .and_then(|arrow| TREND_MAP.get(arrow as usize).copied())
        .unwrap_or(TrendType::Flat)
}

/// Implemented by API types that can be converted to [`LibreCgmData`] via [`map_glucose_data`].
pub trait GlucoseData {
    fn factory_timestamp(&self) -> &str;
    fn value(&self) -> f64;
    fn is_high(&self) -> bool;
    fn is_low(&self) -> bool;
    fn trend_arrow(&self) -> Option<i32>;
}

impl GlucoseData for GlucoseItem {
    fn factory_timestamp(&self) -> &str {
        &self.factory_timestamp
    }
    fn value(&self) -> f64 {
        self.value
    }
    fn is_high(&self) -> bool {
        self.is_high
    }
    fn is_low(&self) -> bool {
        self.is_low
    }
    fn trend_arrow(&self) -> Option<i32> {
        self.trend_arrow
    }
}

impl GlucoseData for GlucoseMeasurement {
    fn factory_timestamp(&self) -> &str {
        &self.factory_timestamp
    }
    fn value(&self) -> f64 {
        self.value
    }
    fn is_high(&self) -> bool {
        self.is_high
    }
    fn is_low(&self) -> bool {
        self.is_low
    }
    fn trend_arrow(&self) -> Option<i32> {
        Some(self.trend_arrow)
    }
}

/// Converts a [`GlucoseData`] item (e.g. [`GlucoseItem`], [`GlucoseMeasurement`]) into [`LibreCgmData`]. Uses [`get_trend`] for the trend; parses timestamp or falls back to now.
pub fn map_glucose_data<T: GlucoseData>(item: &T) -> LibreCgmData {
    let date = format!("{} UTC", item.factory_timestamp())
        .parse()
        .unwrap_or_else(|_| Utc::now());

    LibreCgmData {
        value: item.value(),
        is_high: item.is_high(),
        is_low: item.is_low(),
        trend: get_trend(item.trend_arrow()),
        date,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::GlucoseItem;

    #[test]
    fn test_get_trend() {
        assert_eq!(get_trend(Some(3)), TrendType::Flat);
        assert_eq!(get_trend(Some(5)), TrendType::SingleUp);
        assert_eq!(get_trend(Some(0)), TrendType::NotComputable);
        assert_eq!(get_trend(None), TrendType::Flat);
        assert_eq!(get_trend(Some(10)), TrendType::Flat);
    }

    #[test]
    fn test_map_glucose_data() {
        let item = GlucoseItem {
            factory_timestamp: "10/24/2024 10:00:00 AM".to_string(),
            timestamp: "10/24/2024 10:00:00 AM".to_string(),
            item_type: 0,
            value_in_mg_per_dl: 100.0,
            trend_arrow: Some(3),
            trend_message: None,
            measurement_color: 1,
            glucose_units: 0,
            value: 100.0,
            is_high: false,
            is_low: false,
        };

        let result = map_glucose_data(&item);
        assert_eq!(result.value, 100.0);
        assert_eq!(result.trend, TrendType::Flat);
        assert!(!result.is_high);
        assert!(!result.is_low);
    }
}

#[cfg(test)]
mod region_tests {
    use super::super::models::Region;
    use std::str::FromStr;

    #[test]
    fn test_region_parsing() {
        assert_eq!(Region::from_str("US").unwrap(), Region::US);
        assert_eq!(Region::from_str("eu").unwrap(), Region::EU);
        assert_eq!(Region::from_str("EU2").unwrap(), Region::EU2);
        assert_eq!(Region::from_str("invalid").unwrap(), Region::Global);
    }

    #[test]
    fn test_region_urls() {
        assert!(Region::US.base_url().contains("api-us"));
        assert!(Region::EU.base_url().contains("api-eu"));
        assert!(Region::Global.base_url().contains("api.libreview"));
    }
}

//! Conversion from raw API glucose readings to [`LibreCgmData`].
//!
//! Public entry points are the `TryFrom<&GlucoseItem>` and `TryFrom<&GlucoseMeasurement>`
//! impls for [`LibreCgmData`].

use crate::{
    errors::{LibreLinkUpError, Result},
    models::{
        LibreCgmData,
        client::TrendType,
        common::{GlucoseItem, GlucoseMeasurement},
    },
};
use chrono::{DateTime, NaiveDateTime, Utc};

/// API `TrendArrow` index (0–6) to [`TrendType`].
///
/// Indices 0 and 6 (and any index outside this table, see [`trend_from_arrow`]) mean the
/// sensor could not compute a trend. The order also runs from falling to rising, which
/// `poll_averaged` relies on to average trends.
pub(crate) const TREND_MAP: [TrendType; 7] = [
    TrendType::NotComputable,
    TrendType::SingleDown,
    TrendType::FortyFiveDown,
    TrendType::Flat,
    TrendType::FortyFiveUp,
    TrendType::SingleUp,
    TrendType::NotComputable,
];

/// Converts an API `TrendArrow` to [`TrendType`].
///
/// A missing, negative or out-of-range arrow is [`TrendType::NotComputable`]: an unknown
/// trend must not be reported as stable.
pub(crate) fn trend_from_arrow(trend_arrow: Option<i32>) -> TrendType {
    trend_arrow
        .and_then(|arrow| usize::try_from(arrow).ok())
        .and_then(|idx| TREND_MAP.get(idx).copied())
        .unwrap_or(TrendType::NotComputable)
}

/// Format of the API's `FactoryTimestamp` field (always UTC).
const FACTORY_TIMESTAMP_FORMAT: &str = "%m/%d/%Y %I:%M:%S %p";

/// The fields shared by [`GlucoseItem`] and [`GlucoseMeasurement`] that [`map_glucose_data`] needs.
trait GlucoseData {
    fn factory_timestamp(&self) -> &str;
    /// Glucose value in mg/dL, regardless of the account's display unit.
    fn value_mg_dl(&self) -> f64;
    fn is_high(&self) -> bool;
    fn is_low(&self) -> bool;
    fn trend_arrow(&self) -> Option<i32>;
}

impl GlucoseData for GlucoseItem {
    fn factory_timestamp(&self) -> &str {
        &self.factory_timestamp
    }
    fn value_mg_dl(&self) -> f64 {
        // `Value` is in the account's display unit (mg/dL or mmol/L); `ValueInMgPerDl` is always mg/dL.
        self.value_in_mg_per_dl
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
    fn value_mg_dl(&self) -> f64 {
        // `Value` is in the account's display unit (mg/dL or mmol/L); `ValueInMgPerDl` is always mg/dL.
        self.value_in_mg_per_dl
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

/// Builds a [`LibreCgmData`]. Fails with [`LibreLinkUpError::InvalidResponse`] if
/// `FactoryTimestamp` cannot be parsed: a reading with an unknown time is never stamped
/// with the current time.
fn map_glucose_data<T: GlucoseData>(item: &T) -> Result<LibreCgmData> {
    let timestamp =
        NaiveDateTime::parse_from_str(item.factory_timestamp(), FACTORY_TIMESTAMP_FORMAT)
            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
            .map_err(|e| {
                LibreLinkUpError::InvalidResponse(format!(
                    "unparseable FactoryTimestamp '{}': {}",
                    item.factory_timestamp(),
                    e
                ))
            })?;

    Ok(LibreCgmData {
        value_mg_dl: item.value_mg_dl(),
        is_high: item.is_high(),
        is_low: item.is_low(),
        trend: trend_from_arrow(item.trend_arrow()),
        timestamp,
    })
}

/// Converts a history point from [`read_raw`](crate::LibreLinkUpClient::read_raw).
///
/// # Errors
///
/// [`LibreLinkUpError::InvalidResponse`] if `FactoryTimestamp` cannot be parsed.
impl TryFrom<&GlucoseItem> for LibreCgmData {
    type Error = LibreLinkUpError;

    fn try_from(item: &GlucoseItem) -> Result<Self> {
        map_glucose_data(item)
    }
}

/// Converts the latest reading from [`read_raw`](crate::LibreLinkUpClient::read_raw).
///
/// # Errors
///
/// [`LibreLinkUpError::InvalidResponse`] if `FactoryTimestamp` cannot be parsed.
impl TryFrom<&GlucoseMeasurement> for LibreCgmData {
    type Error = LibreLinkUpError;

    fn try_from(item: &GlucoseMeasurement) -> Result<Self> {
        map_glucose_data(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    const TS: &str = "10/24/2024 10:00:00 PM";

    fn item(units: i32, value: f64, mg_dl: f64, ts: &str, arrow: Option<i32>) -> GlucoseItem {
        serde_json::from_value(json!({
            "FactoryTimestamp": ts,
            "Timestamp": ts,
            "type": 0,
            "ValueInMgPerDl": mg_dl,
            "TrendArrow": arrow,
            "TrendMessage": null,
            "MeasurementColor": 1,
            "GlucoseUnits": units,
            "Value": value,
            "isHigh": false,
            "isLow": false
        }))
        .unwrap()
    }

    fn measurement(units: i32, value: f64, mg_dl: f64, ts: &str, arrow: i32) -> GlucoseMeasurement {
        serde_json::from_value(json!({
            "FactoryTimestamp": ts,
            "Timestamp": ts,
            "type": 1,
            "ValueInMgPerDl": mg_dl,
            "TrendArrow": arrow,
            "TrendMessage": null,
            "MeasurementColor": 1,
            "GlucoseUnits": units,
            "Value": value,
            "isHigh": false,
            "isLow": false
        }))
        .unwrap()
    }

    #[test]
    fn test_trend_from_arrow() {
        assert_eq!(trend_from_arrow(Some(3)), TrendType::Flat);
        assert_eq!(trend_from_arrow(Some(5)), TrendType::SingleUp);
        assert_eq!(trend_from_arrow(Some(0)), TrendType::NotComputable);
        assert_eq!(trend_from_arrow(Some(6)), TrendType::NotComputable);
        assert_eq!(trend_from_arrow(None), TrendType::NotComputable);
        assert_eq!(trend_from_arrow(Some(10)), TrendType::NotComputable);
        assert_eq!(trend_from_arrow(Some(-1)), TrendType::NotComputable);
    }

    #[test]
    fn test_map_glucose_data() {
        let result = map_glucose_data(&item(0, 100.0, 100.0, TS, Some(3))).unwrap();
        assert_eq!(result.value_mg_dl, 100.0);
        assert_eq!(result.trend, TrendType::Flat);
        assert_eq!(
            result.timestamp,
            Utc.with_ymd_and_hms(2024, 10, 24, 22, 0, 0).unwrap()
        );
        assert!(!result.is_high);
        assert!(!result.is_low);
    }

    #[test]
    fn mmol_item_maps_to_mg_dl() {
        let result = LibreCgmData::try_from(&item(1, 6.2, 112.0, TS, Some(3))).unwrap();
        assert_eq!(result.value_mg_dl, 112.0);
    }

    #[test]
    fn mmol_measurement_maps_to_mg_dl() {
        let result = LibreCgmData::try_from(&measurement(1, 6.2, 112.0, TS, 3)).unwrap();
        assert_eq!(result.value_mg_dl, 112.0);
    }

    #[test]
    fn mg_dl_account_is_unchanged() {
        let from_item = LibreCgmData::try_from(&item(0, 112.0, 112.0, TS, Some(3))).unwrap();
        let from_measurement =
            LibreCgmData::try_from(&measurement(0, 112.0, 112.0, TS, 3)).unwrap();
        assert_eq!(from_item.value_mg_dl, 112.0);
        assert_eq!(from_measurement.value_mg_dl, 112.0);
    }

    #[test]
    fn bad_timestamp_is_an_error_not_now() {
        let result = map_glucose_data(&item(0, 100.0, 100.0, "2024-10-24T22:00:00Z", Some(3)));
        assert!(matches!(result, Err(LibreLinkUpError::InvalidResponse(_))));
        let result = map_glucose_data(&measurement(0, 100.0, 100.0, "", 3));
        assert!(matches!(result, Err(LibreLinkUpError::InvalidResponse(_))));
    }

    #[test]
    fn missing_trend_arrow_is_not_computable() {
        let result = map_glucose_data(&item(0, 100.0, 100.0, TS, None)).unwrap();
        assert_eq!(result.trend, TrendType::NotComputable);
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

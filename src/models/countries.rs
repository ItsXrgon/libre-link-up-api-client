use serde::{Deserialize, Serialize};

/// Regional endpoint configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AE {
    pub lsl_api: String,
    pub socket_hub: String,
}

/// Map of all regional endpoints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionalMap {
    pub us: AE,
    pub eu: AE,
    pub fr: AE,
    pub jp: AE,
    pub de: AE,
    pub ap: AE,
    pub au: AE,
    pub ae: AE,
    pub eu2: AE,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub regional_map: RegionalMap,
}

/// Response from the country/region configuration endpoint
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CountryResponse {
    pub status: i32,
    pub data: Data,
}

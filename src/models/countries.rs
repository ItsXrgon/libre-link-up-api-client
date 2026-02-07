use serde::{Deserialize, Serialize};

/// Regional endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AE {
    pub lsl_api: String,
    pub socket_hub: String,
}

/// Map of all regional endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub regional_map: RegionalMap,
}

/// Response from the country/region configuration endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryResponse {
    pub status: i32,
    pub data: Data,
}

impl RegionalMap {
    /// Get endpoint by region key
    pub fn get(&self, region: &str) -> Option<&AE> {
        match region {
            "us" => Some(&self.us),
            "eu" => Some(&self.eu),
            "eu2" => Some(&self.eu2),
            "fr" => Some(&self.fr),
            "jp" => Some(&self.jp),
            "de" => Some(&self.de),
            "ap" => Some(&self.ap),
            "au" => Some(&self.au),
            "ae" => Some(&self.ae),
            _ => None,
        }
    }
}

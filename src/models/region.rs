#[derive(Debug, Clone)]
pub enum Region {
    /// Global endpoint (auto-redirects to appropriate region)
    Global,
    /// United Arab Emirates
    AE,
    /// Asia-Pacific
    AP,
    /// Australia
    AU,
    /// Canada
    CA,
    /// Germany
    DE,
    /// Europe
    EU,
    /// Europe 2
    EU2,
    /// France
    FR,
    /// Japan
    JP,
    /// United States
    US,
    /// Latin America
    LA,
    /// Russia
    RU,
    /// China
    CN,
    /// Custom endpoint URL
    Custom(String),
}
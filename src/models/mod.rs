//! Data models for LibreLinkUp API
//! 
//! These models are direct translations from the TypeScript types in the original
//! libre-link-up-api-client repository.

pub mod client;
pub mod connection;
pub mod connections;
pub mod countries;
pub mod graph;
pub mod login;
pub mod region;

// Re-export commonly used types
pub use client::{LibreCgmData, TrendType};
pub use connection::{
    ActiveSensor, AlarmRules, Connection, ConnectionResponse, Device, GlucoseItem, Sensor, Ticket,
};
pub use connections::{ConnectionsResponse, Datum};
pub use countries::{CountryResponse, RegionalMap, AE};
pub use graph::GraphData;
pub use login::{
    AuthTicket, Data as LoginData, LoginArgs, LoginRedirectResponse, LoginResponse,
    LoginResponseData, StepData, User,
};
pub use region::Region;

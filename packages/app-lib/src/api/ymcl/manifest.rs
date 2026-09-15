//! YAP protocol types shared by the capabilities and manifest endpoints.
//!
//! All structs tolerate unknown fields so older launchers keep working
//! against newer adapters (YAP §6.1 forward compatibility).

use serde::{Deserialize, Serialize};

pub const YAP_PROTOCOL_VERSION: u32 = 1;
pub const YAP_API_BASE: &str = "/api/plugins/ymcl-adapter/v1";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct YmclDomainIdentity {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub logo_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct YmclAuthMethod {
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub authorize_url: Option<String>,
    #[serde(default)]
    pub token_url: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub providers_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct YmclAuthConfig {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub methods: Vec<YmclAuthMethod>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct YmclCapabilities {
    #[serde(default)]
    pub protocol_version: u32,
    #[serde(default)]
    pub adapter_version: Option<String>,
    #[serde(default)]
    pub domain: Option<YmclDomainIdentity>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub auth: Option<YmclAuthConfig>,
    #[serde(default)]
    pub mip: Option<MipEndpointConfig>,
}

/// MIP distribution face advertised by the adapter (YAP §7).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MipEndpointConfig {
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclNavigationItem {
    pub id: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub sort: i64,
    #[serde(default)]
    pub required_permission: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclPageDescriptor {
    pub id: String,
    #[serde(default)]
    pub renderer: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub data_source: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    #[serde(default)]
    pub bundle: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclManifest {
    #[serde(default = "default_protocol_version")]
    pub protocol_version: u32,
    #[serde(default)]
    pub domain: Option<YmclDomainIdentity>,
    #[serde(default)]
    pub navigation: Vec<YmclNavigationItem>,
    #[serde(default)]
    pub pages: Vec<YmclPageDescriptor>,
    #[serde(default)]
    pub data_sources: Vec<serde_json::Value>,
    #[serde(default)]
    pub actions: Option<serde_json::Value>,
    #[serde(default)]
    pub theme: Option<serde_json::Value>,
    #[serde(default)]
    pub home: Option<serde_json::Value>,
}

fn default_protocol_version() -> u32 {
    YAP_PROTOCOL_VERSION
}

pub fn capabilities_url(origin: &str) -> String {
    format!("{origin}{YAP_API_BASE}/capabilities")
}

pub fn manifest_url(origin: &str) -> String {
    format!("{origin}{YAP_API_BASE}/manifest")
}

/// Adapters must serve the major version the launcher understands; minor
/// drift is allowed and resolved by ignoring unknown fields.
pub fn validate_protocol_version(version: u32) -> crate::Result<()> {
    if version != YAP_PROTOCOL_VERSION {
        return Err(crate::ErrorKind::OtherError(format!(
            "Domain requires YAP protocol v{version}, but this launcher supports v{YAP_PROTOCOL_VERSION}. Please update the launcher."
        ))
        .into());
    }
    Ok(())
}

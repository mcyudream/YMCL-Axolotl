//! MIP manifest model (MIP §3.2): the versioned, immutable file list that
//! drives installs and updates. Only the fields the launcher consumes are
//! modeled; unknown fields are ignored per MIP §2 forward compatibility.

use serde::{Deserialize, Serialize};

pub const SUPPORTED_FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MipManifest {
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    #[serde(default)]
    pub pack_id: String,
    pub version: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub game: Option<MipGame>,
    #[serde(default)]
    pub features: Vec<MipFeature>,
    #[serde(default)]
    pub files: Vec<MipFileEntry>,
}

fn default_format_version() -> u32 {
    SUPPORTED_FORMAT_VERSION
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MipGame {
    #[serde(default)]
    pub minecraft: Option<String>,
    #[serde(default)]
    pub java: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MipFeature {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub conflicts: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MipFileEntry {
    pub path: String,
    pub sha512: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default = "default_policy")]
    pub policy: String,
    #[serde(default)]
    pub feature: Option<String>,
    #[serde(default)]
    pub moved_from: Option<String>,
    #[serde(default)]
    pub sources: Vec<MipSource>,
}

fn default_policy() -> String {
    "managed".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MipSource {
    #[serde(rename = "type", default = "default_source_type")]
    pub source_type: String,
    #[serde(default)]
    pub url: Option<String>,
}

fn default_source_type() -> String {
    "http".to_string()
}

impl MipManifest {
    pub fn validate(&self) -> crate::Result<()> {
        if self.format_version > SUPPORTED_FORMAT_VERSION {
            return Err(crate::ErrorKind::OtherError(format!(
                "Manifest formatVersion {} is newer than supported ({SUPPORTED_FORMAT_VERSION}); please update the launcher",
                self.format_version
            ))
            .into());
        }
        Ok(())
    }
}

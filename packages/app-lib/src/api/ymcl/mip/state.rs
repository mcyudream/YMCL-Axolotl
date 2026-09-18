//! MIP client state model: `<instance>/.pack-state.json` (MIP §3.4). This
//! file is the `base` of every incremental update and records the player's
//! feature selections, per-file hashes and lock/disable decisions.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct MipPackState {
    #[serde(default)]
    pub pack_id: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub selected_features: Vec<String>,
    /// Feature ids the installed version declared (MIP §3.2). Lets the next
    /// update distinguish "deselected by the player" from "added in a later
    /// version", so newly introduced default features get opted in (MIP WF-5
    /// default merge) instead of being silently skipped. `None` for states
    /// written before this field existed.
    #[serde(default)]
    pub declared_features: Option<Vec<String>>,
    #[serde(default)]
    pub files: std::collections::BTreeMap<String, StateFile>,
    #[serde(default)]
    pub locked_paths: Vec<String>,
    #[serde(default)]
    pub disabled_paths: Vec<String>,
    /// Domain binding that installed this instance (YAP §7): which domain
    /// server/season it came from. Absent for manually imported packs.
    #[serde(default)]
    pub binding: Option<MipInstanceBinding>,
    /// Admin publish profile (YAP §7): the file exclusions chosen at the
    /// initial publish plus the latest feature/policy declarations, so later
    /// delta publishes reuse the same selection instead of resurfacing
    /// intentionally-excluded files as additions and re-asking for feature
    /// globs. `None` for states written before this field existed.
    #[serde(default)]
    pub publish_profile: Option<PublishProfile>,
}

/// Which domain server/season an installed pack came from.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MipInstanceBinding {
    pub server_id: String,
    #[serde(default)]
    pub season_id: Option<String>,
}

/// Admin's persisted publish selection (YAP §7): paths kept out of the pack
/// (files or directories, `/`-separated, matched exact or as a directory
/// prefix) and the mip.json feature/policy declarations from the latest
/// publish. Publisher-side only; players never read this.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PublishProfile {
    #[serde(default)]
    pub excluded: Vec<String>,
    #[serde(default)]
    pub features: Vec<PublishFeature>,
    #[serde(default)]
    pub policies: Vec<PublishPolicy>,
}

/// Persisted optional-content declaration (mip.json features, MIP §3.5).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PublishFeature {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
}

/// Persisted file-policy rule (mip.json policies, MIP §3.5).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PublishPolicy {
    pub glob: String,
    pub policy: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StateFile {
    pub sha512: String,
    #[serde(default = "default_policy")]
    pub policy: String,
    /// Feature this file belongs to (MIP §3.2 `files[].feature`), recorded so
    /// later delta publishes can re-annotate changed/added files. Absent for
    /// core files and for states written before this field existed.
    #[serde(default)]
    pub feature: Option<String>,
}

fn default_policy() -> String {
    "managed".to_string()
}

pub const STATE_FILE_NAME: &str = ".pack-state.json";

/// Local scan cache (`path → size/mtime/sha512`) beside the state file, so
/// repeated publish diffs and update checks re-hash only changed files.
/// Bookkeeping only: excluded from scans and packs like the state file.
pub const HASH_CACHE_FILE_NAME: &str = ".pack-hashes.json";

/// Loads the pack state for an instance directory; `Ok(None)` when the
/// instance is not an MIP-managed instance.
pub async fn load(instance_dir: &Path) -> crate::Result<Option<MipPackState>> {
    let path = instance_dir.join(STATE_FILE_NAME);
    if !path.exists() {
        return Ok(None);
    }
    let contents = tokio::fs::read_to_string(&path).await?;
    let state = serde_json::from_str(&contents).map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Corrupted {STATE_FILE_NAME}: {e}"
        ))
    })?;
    Ok(Some(state))
}

/// Persists the state atomically (write temp + rename, MIP WF-5 step 9).
pub async fn save(
    instance_dir: &Path,
    state: &MipPackState,
) -> crate::Result<()> {
    let path = instance_dir.join(STATE_FILE_NAME);
    let temp = instance_dir.join(format!("{STATE_FILE_NAME}.tmp"));
    let contents = serde_json::to_vec_pretty(state)?;
    tokio::fs::write(&temp, contents).await?;
    tokio::fs::rename(&temp, &path).await?;
    Ok(())
}

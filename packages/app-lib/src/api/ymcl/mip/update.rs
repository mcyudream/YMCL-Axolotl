//! MIP update orchestration (YAP §7): launch-time binding check and
//! incremental update application for MIP-managed instances.
//!
//! Flow per YAP §7 / MIP WF-5: load the instance's `.pack-state.json` →
//! resolve the domain binding's current target version → diff → apply →
//! persist the new state. The check is read-only; applying is a separate
//! step so the UI can confirm (updatePolicy `prompt`).

use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

use super::apply::{BACKUP_DIR_NAME, STAGING_DIR_NAME};
use super::apply::{ObjectFetcher, apply_update};
use super::diff::compute_update_plan;
use super::manifest::{MipFileEntry, MipManifest};
use super::state::{self, MipPackState};
use crate::State;
use crate::api::ymcl::registry;
use crate::util::fetch::fetch_advanced;

#[derive(Serialize, Clone, Debug)]
pub struct YmclUpdateCheck {
    pub managed: bool,
    pub pack_id: Option<String>,
    pub current_version: Option<String>,
    pub target_version: Option<String>,
    pub server_id: Option<String>,
    pub season_id: Option<String>,
    /// Change counts from the computed plan (None until applied).
    pub pending_changes: Option<usize>,
    pub pending_deletions: Option<usize>,
}

#[derive(Serialize, Clone, Debug)]
pub struct YmclUpdateResult {
    pub applied: bool,
    pub staged_files: usize,
    pub deleted_files: usize,
    pub new_version: String,
}

/// Binding of a pack to a domain server (MIP appendix B.2), from
/// `GET {mip}/api/servers`.
#[derive(Deserialize, Clone, Debug)]
pub struct MipServerBinding {
    #[serde(rename = "serverId")]
    pub server_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub binding: Option<MipPackBinding>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MipPackBinding {
    #[serde(rename = "packId", default)]
    pub pack_id: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(rename = "pinnedVersion", default)]
    pub pinned_version: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MipVersionEntry {
    version: String,
    #[serde(default)]
    channel: Option<String>,
}

fn versions_url(mip_base: &str, pack_id: &str) -> String {
    format!("{mip_base}/api/packs/{pack_id}/versions")
}

fn servers_url(mip_base: &str) -> String {
    format!("{mip_base}/api/servers")
}

fn manifest_url(mip_base: &str, pack_id: &str, version: &str) -> String {
    format!("{mip_base}/api/packs/{pack_id}/manifest/{version}")
}

async fn fetch_json(
    state: &State,
    url: &str,
) -> crate::Result<serde_json::Value> {
    let bytes = fetch_advanced(
        Method::GET,
        url,
        None,
        None,
        None,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// HTTP-backed object fetcher: walks a manifest entry's `sources[]` in
/// order, verifying sha512 for each attempt (MIP §3.3).
pub struct HttpFetcher<'a> {
    state: &'a State,
}

#[async_trait]
impl ObjectFetcher for HttpFetcher<'_> {
    async fn fetch(&self, entry: &MipFileEntry) -> crate::Result<Vec<u8>> {
        let mut last_error: Option<crate::Error> = None;
        for source in &entry.sources {
            let Some(url) = &source.url else {
                continue;
            };
            match fetch_advanced(
                Method::GET,
                url,
                None,
                None,
                None,
                None,
                None,
                None,
                &self.state.api_semaphore,
                &self.state.pool,
            )
            .await
            {
                Ok(bytes) => return Ok(bytes.to_vec()),
                Err(error) => {
                    tracing::warn!(
                        "Source {url} failed for {}: {error}",
                        entry.path
                    );
                    last_error = Some(error);
                }
            }
        }
        Err(crate::ErrorKind::OtherError(format!(
            "All sources failed for {}: {}",
            entry.path,
            last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "no sources declared".to_string())
        ))
        .into())
    }
}

/// Scans the instance directory into a local hash index (MIP WF-5 step 3).
/// Protocol bookkeeping files are excluded.
pub fn scan_local_hashes(
    instance_dir: &Path,
) -> crate::Result<HashMap<String, String>> {
    let mut hashes = HashMap::new();
    let mut stack = vec![instance_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();
            if path.is_dir() {
                if name == STAGING_DIR_NAME || name == BACKUP_DIR_NAME {
                    continue;
                }
                stack.push(path);
            } else if name == state::STATE_FILE_NAME {
                continue;
            } else {
                let bytes = std::fs::read(&path)?;
                let digest = Sha512::digest(&bytes);
                let relative = path
                    .strip_prefix(instance_dir)
                    .map_err(|_| {
                        crate::ErrorKind::OtherError(
                            "Invalid instance path".to_string(),
                        )
                    })?
                    .to_string_lossy()
                    .replace('\\', "/");
                hashes.insert(
                    relative,
                    digest.iter().map(|byte| format!("{byte:02x}")).collect(),
                );
            }
        }
    }
    Ok(hashes)
}

/// Resolves the target version for a binding: pinned wins, else the latest
/// release on the binding's channel, else the latest overall. Pure helper,
/// unit tested.
pub fn resolve_target_version<'a>(
    versions: &'a [MipVersionEntry],
    channel: Option<&str>,
    pinned: Option<&str>,
) -> Option<&'a str> {
    if let Some(pinned) = pinned {
        return versions
            .iter()
            .find(|entry| entry.version == pinned)
            .map(|entry| entry.version.as_str());
    }
    let candidates: Vec<&MipVersionEntry> = versions
        .iter()
        .filter(|entry| {
            channel
                .is_none_or(|channel| entry.channel.as_deref() == Some(channel))
        })
        .collect();
    let pool: Vec<&MipVersionEntry> = if candidates.is_empty() {
        versions.iter().collect()
    } else {
        candidates
    };
    pool.first().map(|entry| entry.version.as_str())
}

/// Locates the binding for `pack_id` among the domain's servers.
pub fn find_binding<'a>(
    servers: &'a [MipServerBinding],
    pack_id: &str,
) -> Option<&'a MipPackBinding> {
    servers
        .iter()
        .find(|server| {
            server
                .binding
                .as_ref()
                .is_some_and(|binding| binding.pack_id == pack_id)
        })
        .and_then(|server| server.binding.as_ref())
}

/// Core orchestration (YAP §7 step 2/4): check — and optionally apply — the
/// incremental update for a MIP-managed instance. When `apply` is false the
/// instance is only inspected; nothing is written.
pub async fn check_instance(
    instance_id: &str,
    apply: bool,
) -> crate::Result<YmclUpdateCheck> {
    let state = State::get().await?;
    let instance_dir = crate::api::instance::get_full_path(instance_id).await?;

    let Some(pack_state) = state::load(&instance_dir).await? else {
        return Ok(YmclUpdateCheck {
            managed: false,
            pack_id: None,
            current_version: None,
            target_version: None,
            server_id: None,
            season_id: None,
            pending_changes: None,
            pending_deletions: None,
        });
    };
    let pack_id = pack_state.pack_id.clone();

    // Active domain context: capabilities carry the MIP base URL.
    let active = registry::active_domain_id(&state.pool).await?;
    if active == registry::PERSONAL_DOMAIN_ID {
        return Ok(unchanged(&pack_state, "not in a domain"));
    }
    let capabilities = registry::domain_capabilities(&active).await?;
    let Some(mip_base) = capabilities
        .mip
        .as_ref()
        .and_then(|mip| mip.base_url.clone())
    else {
        return Ok(unchanged(&pack_state, "domain has no MIP distribution"));
    };

    // Binding: prefer the instance's recorded server, else any server that
    // publishes this pack (MIP appendix B.3 step 2).
    let servers_value = fetch_json(&state, &servers_url(&mip_base)).await?;
    let servers: Vec<MipServerBinding> = serde_json::from_value(
        servers_value
            .get("servers")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )?;
    let binding = pack_state
        .binding
        .as_ref()
        .and_then(|binding| {
            servers
                .iter()
                .find(|server| server.server_id == binding.server_id)
        })
        .and_then(|server| server.binding.as_ref())
        .or_else(|| find_binding(&servers, &pack_id));
    let Some(binding) = binding.filter(|binding| binding.pack_id == pack_id)
    else {
        return Ok(unchanged(
            &pack_state,
            "pack is no longer bound to a server",
        ));
    };

    // Target version: pinned wins, else channel latest.
    let versions_value =
        fetch_json(&state, &versions_url(&mip_base, &pack_id)).await?;
    let versions: Vec<MipVersionEntry> = serde_json::from_value(
        versions_value
            .get("versions")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )?;
    let Some(target_version) = resolve_target_version(
        &versions,
        binding.channel.as_deref(),
        binding.pinned_version.as_deref(),
    ) else {
        return Ok(unchanged(&pack_state, "no released versions"));
    };
    if target_version == pack_state.version {
        return Ok(unchanged(&pack_state, "already up to date"));
    }

    // Manifest + plan.
    let manifest_value =
        fetch_json(&state, &manifest_url(&mip_base, &pack_id, target_version))
            .await?;
    let manifest: MipManifest = serde_json::from_value(manifest_value)?;
    manifest.validate()?;
    let local_hashes = scan_local_hashes(&instance_dir)?;
    let plan = compute_update_plan(&pack_state, &manifest, &local_hashes)?;

    let check = YmclUpdateCheck {
        managed: true,
        pack_id: Some(pack_id.clone()),
        current_version: Some(pack_state.version.clone()),
        target_version: Some(target_version.to_string()),
        server_id: Some(binding.pack_id.clone()),
        season_id: pack_state
            .binding
            .as_ref()
            .and_then(|binding| binding.season_id.clone()),
        pending_changes: Some(plan.changes.len()),
        pending_deletions: Some(plan.deletions.len()),
    };

    if !apply {
        return Ok(check);
    }

    let entries: HashMap<String, MipFileEntry> = manifest
        .files
        .iter()
        .map(|file| (file.path.clone(), file.clone()))
        .collect();
    let fetcher = HttpFetcher { state: &state };
    let outcome = apply_update(
        &instance_dir,
        &pack_state,
        &manifest,
        &plan,
        &entries,
        &fetcher,
    )
    .await?;
    let new_state = outcome.new_state;
    state::save(&instance_dir, &new_state).await?;

    Ok(YmclUpdateCheck {
        managed: true,
        pack_id: Some(pack_id),
        current_version: check.current_version,
        target_version: Some(new_state.version.clone()),
        server_id: check.server_id,
        season_id: check.season_id,
        pending_changes: Some(outcome.staged_files),
        pending_deletions: Some(outcome.deleted_files),
    })
}

fn unchanged(pack_state: &MipPackState, reason: &str) -> YmclUpdateCheck {
    tracing::debug!("MIP update check: {reason} (pack {})", pack_state.pack_id);
    YmclUpdateCheck {
        managed: true,
        pack_id: Some(pack_state.pack_id.clone()),
        current_version: Some(pack_state.version.clone()),
        target_version: Some(pack_state.version.clone()),
        server_id: pack_state
            .binding
            .as_ref()
            .map(|binding| binding.server_id.clone()),
        season_id: pack_state
            .binding
            .as_ref()
            .and_then(|binding| binding.season_id.clone()),
        pending_changes: Some(0),
        pending_deletions: Some(0),
    }
}

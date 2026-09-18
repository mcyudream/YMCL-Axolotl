//! MIP update orchestration (YAP §7): launch-time binding check and
//! incremental update application for MIP-managed instances.
//!
//! Flow per YAP §7 / MIP WF-5: load the instance's `.pack-state.json` →
//! resolve the domain binding's current target version → diff → apply →
//! persist the new state. The check is read-only; applying is a separate
//! step so the UI can confirm (updatePolicy `prompt`).

use std::collections::{HashMap, HashSet};
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
    /// Paths skipped during apply because every remote source failed.
    pub skipped_files: Option<Vec<String>>,
    /// Release notes of the target version (publisher-attached, optional) —
    /// shown to the player before they update.
    pub notes: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct YmclUpdateResult {
    pub applied: bool,
    pub staged_files: usize,
    pub deleted_files: usize,
    pub new_version: String,
    /// Paths whose remote object could not be fetched; local content was kept.
    pub skipped_files: Vec<String>,
}

/// Binding of a pack to a domain server (MIP appendix B.2), from
/// `GET {mip}/api/servers`.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct MipServerBinding {
    #[serde(rename = "serverId")]
    pub server_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub binding: Option<MipPackBinding>,
    /// Server address for join-flow matching (MIP appendix B.2 `mcAddress`).
    #[serde(rename = "mcAddress", default)]
    pub mc_address: Option<String>,
    /// Optional all lines (primary first) contributed by the adapter.
    #[serde(default)]
    pub endpoints: Option<Vec<MipServerEndpoint>>,
    /// Season currently running on the server (MIP appendix B.2).
    #[serde(rename = "currentSeason", default)]
    pub current_season: Option<MipSeason>,
}

/// One connect line on a domain server (primary + backup addresses).
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct MipServerEndpoint {
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub primary: Option<bool>,
    #[serde(default)]
    pub edition: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// Season currently running on the server (MIP appendix B.2).
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct MipSeason {
    #[serde(rename = "seasonId", default)]
    pub season_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct MipPackBinding {
    #[serde(rename = "packId", default)]
    pub pack_id: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(rename = "pinnedVersion", default)]
    pub pinned_version: Option<String>,
    /// Version requirement for pure/vanilla servers (no pack bound): players
    /// may join with any local instance of this version (YAP §7).
    #[serde(rename = "mcVersion", default)]
    pub mc_version: Option<String>,
    /// Optional vanilla-enhanced pack offered as a download choice on
    /// vanilla-requiring servers (YAP §7 原版增强包); bound alongside `packId`
    /// or `mcVersion` via merge-write, never replacing them.
    #[serde(rename = "optionalPackId", default)]
    pub optional_pack_id: Option<String>,
    #[serde(rename = "optionalChannel", default)]
    pub optional_channel: Option<String>,
    #[serde(rename = "optionalPinnedVersion", default)]
    pub optional_pinned_version: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MipVersionEntry {
    pub version: String,
    #[serde(default)]
    pub channel: Option<String>,
    /// Release notes the publisher attached at ingest (optional; absent for
    /// versions published before the field existed).
    #[serde(default)]
    pub notes: Option<String>,
}

pub(crate) fn versions_url(mip_base: &str, pack_id: &str) -> String {
    format!("{mip_base}/api/packs/{pack_id}/versions")
}

pub(crate) fn servers_url(mip_base: &str) -> String {
    format!("{mip_base}/api/servers")
}

pub(crate) fn manifest_url(mip_base: &str, pack_id: &str, version: &str) -> String {
    format!("{mip_base}/api/packs/{pack_id}/manifest/{version}")
}

pub(crate) async fn fetch_json(
    state: &State,
    url: &str,
) -> crate::Result<serde_json::Value> {
    // The adapter's MIP distribution face sits behind its plugin permission
    // wall, so attach the active domain's (freshest) session when one exists.
    let active = registry::active_domain_id(&state.pool).await?;
    let bytes = crate::api::ymcl::auth::domain_request_opt(
        state,
        &active,
        Method::GET,
        url,
        None,
    )
    .await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// HTTP-backed object fetcher: walks a manifest entry's `sources[]` in
/// order, verifying sha512 for each attempt (MIP §3.3). A source that
/// returns mismatched bytes is abandoned and the next source is tried so a
/// single corrupt CDN object cannot fail the whole update.
pub struct HttpFetcher<'a> {
    state: &'a State,
    mip_base: Option<&'a str>,
}

/// SPA / error pages must never count as a successful object download: some
/// hosts answer unknown `/mip/objects/...` paths with the site shell (HTTP
/// 200 + `text/html`), which then fails checksum and masks the real cause.
fn looks_like_error_page(bytes: &[u8]) -> bool {
    let head = bytes
        .iter()
        .take(256)
        .copied()
        .collect::<Vec<u8>>();
    let head = String::from_utf8_lossy(&head);
    let head = head.trim_start().to_ascii_lowercase();
    head.starts_with("<!doctype") || head.starts_with("<html")
}

/// Candidate object URLs for one manifest source. Adapters sometimes publish
/// CAS sources as `{origin}/mip/objects/{hash}` while the real MIP face is
/// advertised at `{mip_base}/objects/{hash}` — prefer the advertised face
/// first, then fall back to the published absolute URL.
pub(crate) fn candidate_object_urls(
    source: &super::manifest::MipSource,
    entry: &MipFileEntry,
    mip_base: Option<&str>,
) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    let looks_like_mip_object = source.source_type == "cas"
        || source
            .url
            .as_deref()
            .is_some_and(|url| url.contains("/mip/objects/"));
    if looks_like_mip_object
        && let Some(base) = mip_base
    {
        urls.push(format!(
            "{}/objects/{}",
            base.trim_end_matches('/'),
            entry.sha512
        ));
    }
    if let Some(url) = &source.url
        && !urls.iter().any(|existing| existing == url)
    {
        urls.push(url.clone());
    }
    urls
}

#[async_trait]
impl ObjectFetcher for HttpFetcher<'_> {
    async fn fetch(&self, entry: &MipFileEntry) -> crate::Result<Vec<u8>> {
        let mut last_error: Option<crate::Error> = None;
        for source in &entry.sources {
            for url in candidate_object_urls(source, entry, self.mip_base) {
                match fetch_advanced(
                    Method::GET,
                    &url,
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
                    Ok(bytes) => {
                        let bytes = bytes.to_vec();
                        if looks_like_error_page(&bytes) {
                            let error = crate::ErrorKind::OtherError(format!(
                                "Source {url} returned an HTML page instead of {}",
                                entry.path
                            ))
                            .into();
                            tracing::warn!(
                                "HTML page from {url} for {}",
                                entry.path
                            );
                            last_error = Some(error);
                            continue;
                        }
                        match super::apply::verify_payload(
                            &bytes,
                            &entry.sha512,
                            &entry.path,
                        ) {
                            Ok(()) => return Ok(bytes),
                            Err(error) => {
                                tracing::warn!(
                                    "Checksum mismatch from {url} for {}: {error}",
                                    entry.path
                                );
                                last_error = Some(error);
                            }
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            "Source {url} failed for {}: {error}",
                            entry.path
                        );
                        last_error = Some(error);
                    }
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
/// Paths that can never enter a pack (`is_excluded_from_publish`) are
/// pruned before hashing — saves/logs alone can be gigabytes and none of
/// them is ever a manifest entry. The cached variant
/// ([`scan_local_hashes_cached`]) is what callers should usually prefer.
pub fn scan_local_hashes(
    instance_dir: &Path,
) -> crate::Result<HashMap<String, String>> {
    Ok(scan_instance_files(instance_dir, None)?.0)
}

/// Cached scan: sha512 results are reused for files whose size and mtime
/// match the previous scan (persisted in `.pack-hashes.json`), so repeated
/// publish diffs and update checks re-hash only what changed. A missing or
/// corrupt cache degrades to a full rescan; the refreshed cache is written
/// back atomically.
pub fn scan_local_hashes_cached(
    instance_dir: &Path,
) -> crate::Result<HashMap<String, String>> {
    let cached = read_hash_cache(instance_dir);
    let (hashes, fresh) = scan_instance_files(instance_dir, cached)?;
    write_hash_cache(instance_dir, &fresh);
    Ok(hashes)
}

/// One cache entry: the stat fingerprint that makes a stored sha512 reusable.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct HashCacheEntry {
    size: u64,
    mtime_secs: u64,
    mtime_nanos: u32,
    sha512: String,
}

/// Walks the instance, pruning excluded paths, hashing what remains. When a
/// cache is supplied, entries whose size+mtime fingerprint still match skip
/// the file read entirely; the returned second map holds fresh entries for
/// exactly the files seen this walk (stale cache entries drop out).
fn scan_instance_files(
    instance_dir: &Path,
    cache: Option<std::collections::BTreeMap<String, HashCacheEntry>>,
) -> crate::Result<(
    HashMap<String, String>,
    std::collections::BTreeMap<String, HashCacheEntry>,
)> {
    let mut hashes = HashMap::new();
    let mut fresh: std::collections::BTreeMap<String, HashCacheEntry> =
        std::collections::BTreeMap::new();
    let mut stack = vec![(instance_dir.to_path_buf(), String::new())];
    while let Some((dir, dir_relative)) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();
            // Protocol bookkeeping never enters packs, wherever it sits.
            if name == super::state::STATE_FILE_NAME
                || name == super::state::HASH_CACHE_FILE_NAME
            {
                continue;
            }
            let relative = if dir_relative.is_empty() {
                name.clone()
            } else {
                format!("{dir_relative}/{name}")
            };
            if path.is_dir() {
                if name == STAGING_DIR_NAME
                    || name == BACKUP_DIR_NAME
                    || super::publish::is_excluded_from_publish(&relative)
                {
                    continue;
                }
                stack.push((path, relative));
            } else if super::publish::is_excluded_from_publish(&relative) {
                continue;
            } else {
                let metadata = std::fs::metadata(&path)?;
                let size = metadata.len();
                let (mtime_secs, mtime_nanos) = mtime_key(&metadata);
                if let Some(hit) = cache
                    .as_ref()
                    .and_then(|cache| cache.get(&relative))
                    .filter(|hit| {
                        hit.size == size
                            && hit.mtime_secs == mtime_secs
                            && hit.mtime_nanos == mtime_nanos
                    })
                {
                    hashes.insert(relative.clone(), hit.sha512.clone());
                    fresh.insert(relative, hit.clone());
                    continue;
                }
                let bytes = std::fs::read(&path)?;
                let digest = Sha512::digest(&bytes);
                let sha512: String =
                    digest.iter().map(|byte| format!("{byte:02x}")).collect();
                hashes.insert(relative.clone(), sha512.clone());
                fresh.insert(
                    relative,
                    HashCacheEntry {
                        size,
                        mtime_secs,
                        mtime_nanos,
                        sha512,
                    },
                );
            }
        }
    }
    Ok((hashes, fresh))
}

fn mtime_key(metadata: &std::fs::Metadata) -> (u64, u32) {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| (duration.as_secs(), duration.subsec_nanos()))
        .unwrap_or((0, 0))
}

fn read_hash_cache(
    instance_dir: &Path,
) -> Option<std::collections::BTreeMap<String, HashCacheEntry>> {
    let bytes =
        std::fs::read(instance_dir.join(super::state::HASH_CACHE_FILE_NAME)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_hash_cache(
    instance_dir: &Path,
    entries: &std::collections::BTreeMap<String, HashCacheEntry>,
) {
    let Ok(bytes) = serde_json::to_vec_pretty(entries) else {
        return;
    };
    let path = instance_dir.join(super::state::HASH_CACHE_FILE_NAME);
    let temp = instance_dir.join(format!("{}.tmp", super::state::HASH_CACHE_FILE_NAME));
    if std::fs::write(&temp, bytes).is_ok() {
        std::fs::rename(&temp, &path).ok();
    }
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
            skipped_files: None,
            notes: None,
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
    // publishes this pack (MIP appendix B.3 step 2). The matched server id
    // rides along: installs are keyed by server on the frontend, because a
    // republished pack gets a new pack id while the server binding stays.
    let servers_value = fetch_json(&state, &servers_url(&mip_base)).await?;
    let servers: Vec<MipServerBinding> = serde_json::from_value(
        servers_value
            .get("servers")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )?;
    let resolved = pack_state
        .binding
        .as_ref()
        .and_then(|instance_binding| {
            servers.iter().find_map(|server| {
                if server.server_id != instance_binding.server_id {
                    return None;
                }
                server
                    .binding
                    .as_ref()
                    .map(|binding| (server.server_id.clone(), binding))
            })
        })
        .or_else(|| find_bound_server(&servers, &pack_id))
        .filter(|(_, binding)| binding.pack_id == pack_id);
    let Some((bound_server_id, binding)) = resolved else {
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
    let local_hashes = scan_local_hashes_cached(&instance_dir)?;
    let plan = compute_update_plan(&pack_state, &manifest, &local_hashes)?;

    let check = YmclUpdateCheck {
        managed: true,
        pack_id: Some(pack_id.clone()),
        current_version: Some(pack_state.version.clone()),
        target_version: Some(target_version.to_string()),
        server_id: Some(bound_server_id),
        season_id: pack_state
            .binding
            .as_ref()
            .and_then(|binding| binding.season_id.clone()),
        pending_changes: Some(plan.changes.len()),
        pending_deletions: Some(plan.deletions.len()),
        skipped_files: None,
        notes: versions
            .iter()
            .find(|entry| entry.version == target_version)
            .and_then(|entry| entry.notes.clone()),
    };

    if !apply {
        return Ok(check);
    }

    let entries: HashMap<String, MipFileEntry> = manifest
        .files
        .iter()
        .map(|file| (file.path.clone(), file.clone()))
        .collect();
    let fetcher = HttpFetcher {
        state: &state,
        mip_base: Some(mip_base.as_str()),
    };
    let mut progress = super::apply::ApplyProgress::create(
        instance_id,
        &pack_id,
        target_version,
        &plan,
        &entries,
    )
    .await;
    let outcome = apply_update(
        &instance_dir,
        &pack_state,
        &manifest,
        &plan,
        &entries,
        &fetcher,
        progress.as_mut(),
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
        skipped_files: Some(
            outcome
                .skipped_files
                .iter()
                .map(|skipped| skipped.path.clone())
                .collect(),
        ),
        notes: check.notes,
    })
}

/// Finds the server that publishes `pack_id` (MIP appendix B.3 step 2
/// fallback): returns `(serverId, binding)` so callers can key installs by
/// server identity, which survives republishes that change the pack id.
fn find_bound_server<'a>(
    servers: &'a [MipServerBinding],
    pack_id: &str,
) -> Option<(String, &'a MipPackBinding)> {
    servers.iter().find_map(|server| {
        server
            .binding
            .as_ref()
            .filter(|binding| binding.pack_id == pack_id)
            .map(|binding| (server.server_id.clone(), binding))
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
        skipped_files: None,
        notes: None,
    }
}

/// Authoritative pre-launch step (MIP appendix B.3 / WF-5): a MIP-managed
/// instance is brought to the binding's target version before the game
/// starts. The probe is time-boxed so an unreachable adapter costs seconds,
/// not minutes; an actual update download runs to completion. Errors are
/// returned for the launch path to log and launch the local state anyway.
pub async fn pre_launch_update(instance_id: &str) -> crate::Result<()> {
    let probe = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        check_instance(instance_id, false),
    )
    .await
    {
        Ok(result) => result?,
        Err(_) => {
            tracing::warn!("MIP pre-launch check timed out; launching local state as-is");
            return Ok(());
        }
    };
    let up_to_date = !probe.managed
        || probe
            .target_version
            .as_ref()
            .is_some_and(|target| Some(target) == probe.current_version.as_ref());
    if up_to_date {
        return Ok(());
    }
    let applied = check_instance(instance_id, true).await?;
    tracing::info!(
        "MIP pre-launch updated pack {} to {}",
        applied.pack_id.as_deref().unwrap_or("?"),
        applied.target_version.as_deref().unwrap_or("?")
    );
    Ok(())
}

/// Feature catalog for a MIP-managed instance (YAP §7 player side): the
/// binding's target manifest features plus the instance's current
/// selection. `managed: false` when the instance is not MIP-managed or the
/// active domain has no MIP distribution for it.
#[derive(Serialize, Clone, Debug)]
pub struct YmclPackFeatures {
    pub managed: bool,
    pub pack_id: Option<String>,
    pub target_version: Option<String>,
    pub features: Vec<super::manifest::MipFeature>,
    pub selected: Vec<String>,
}

/// Resolves the binding's current target manifest for a managed instance.
/// The update check inlines its own flow so its up-to-date fast path stays
/// manifest-free; the feature and publish commands share this helper.
pub(crate) async fn resolve_target_manifest(
    state: &State,
    pack_state: &MipPackState,
) -> crate::Result<Option<(MipPackBinding, String, MipManifest)>> {
    let active = registry::active_domain_id(&state.pool).await?;
    if active == registry::PERSONAL_DOMAIN_ID {
        return Ok(None);
    }
    let capabilities = registry::domain_capabilities(&active).await?;
    let Some(mip_base) =
        capabilities.mip.as_ref().and_then(|mip| mip.base_url.clone())
    else {
        return Ok(None);
    };
    let servers_value = fetch_json(state, &servers_url(&mip_base)).await?;
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
        .or_else(|| find_binding(&servers, &pack_state.pack_id))
        .filter(|binding| binding.pack_id == pack_state.pack_id);
    let Some(binding) = binding else {
        return Ok(None);
    };
    let versions_value =
        fetch_json(state, &versions_url(&mip_base, &pack_state.pack_id)).await?;
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
        return Ok(None);
    };
    let manifest_value = fetch_json(
        state,
        &manifest_url(&mip_base, &pack_state.pack_id, target_version),
    )
    .await?;
    let manifest: MipManifest = serde_json::from_value(manifest_value)?;
    manifest.validate()?;
    Ok(Some((binding.clone(), target_version.to_string(), manifest)))
}

/// Lists the declared features and the current selection for a managed
/// instance so the player can toggle optional content.
pub async fn pack_features(instance_id: &str) -> crate::Result<YmclPackFeatures> {
    let state = State::get().await?;
    let instance_dir = crate::api::instance::get_full_path(instance_id).await?;
    let Some(pack_state) = state::load(&instance_dir).await? else {
        return Ok(YmclPackFeatures {
            managed: false,
            pack_id: None,
            target_version: None,
            features: Vec::new(),
            selected: Vec::new(),
        });
    };
    let Some((_binding, target_version, manifest)) =
        resolve_target_manifest(&state, &pack_state).await?
    else {
        return Ok(YmclPackFeatures {
            managed: false,
            pack_id: Some(pack_state.pack_id),
            target_version: None,
            features: Vec::new(),
            selected: pack_state.selected_features,
        });
    };
    Ok(YmclPackFeatures {
        managed: true,
        pack_id: Some(pack_state.pack_id),
        target_version: Some(target_version),
        features: manifest.features,
        selected: pack_state.selected_features,
    })
}

/// Applies a new feature selection (WF-5 selection-change special case):
/// newly selected features' files download through the normal apply
/// pipeline; deselecting deletes a feature's files only when they are
/// locally unmodified. Requires the instance to already be at the binding's
/// target version so the version stamp stays truthful; toggling never
/// changes the version (MIP WF-5).
pub async fn set_features(
    instance_id: &str,
    selected: Vec<String>,
) -> crate::Result<YmclUpdateResult> {
    let state = State::get().await?;
    let instance_dir = crate::api::instance::get_full_path(instance_id).await?;
    let Some(pack_state) = state::load(&instance_dir).await? else {
        return Err(crate::ErrorKind::OtherError(
            "This instance is not a MIP-managed instance".to_string(),
        )
        .into());
    };
    let Some((_binding, target_version, manifest)) =
        resolve_target_manifest(&state, &pack_state).await?
    else {
        return Err(crate::ErrorKind::OtherError(
            "The active domain has no MIP distribution for this pack"
                .to_string(),
        )
        .into());
    };
    if target_version != pack_state.version {
        return Err(crate::ErrorKind::OtherError(format!(
            "Update the instance to {target_version} before changing optional features"
        ))
        .into());
    }

    // 校验：所选必须是已声明 feature，且不与其它所选互斥（MIP §3.5）。
    validate_feature_selection(&manifest, &selected)?;

    let old: HashSet<&str> = pack_state
        .selected_features
        .iter()
        .map(|id| id.as_str())
        .collect();
    let new: HashSet<&str> = selected.iter().map(|id| id.as_str()).collect();
    let newly_selected: HashSet<&str> = new.difference(&old).copied().collect();
    let deselected: HashSet<&str> = old.difference(&new).copied().collect();
    let local_hashes = scan_local_hashes_cached(&instance_dir)?;

    let mut plan = super::diff::UpdatePlan::default();
    let mut entries: HashMap<String, MipFileEntry> = HashMap::new();
    for file in &manifest.files {
        entries.insert(file.path.clone(), file.clone());
        let Some(feature) = file.feature.as_deref() else {
            continue;
        };
        if newly_selected.contains(feature) {
            // 已存在且内容一致的不动；seed 语义走 SeedIfMissing。
            if local_hashes
                .get(&file.path)
                .is_some_and(|hash| hash.as_str() == file.sha512)
            {
                continue;
            }
            let action = if file.policy == "seed" {
                super::diff::ChangeAction::SeedIfMissing
            } else {
                super::diff::ChangeAction::Add
            };
            plan.changes.push(super::diff::PlannedChange {
                path: file.path.clone(),
                action,
                target_sha512: file.sha512.clone(),
                sources_needed: true,
            });
        } else if deselected.contains(feature)
            && local_hashes
                .get(&file.path)
                .is_some_and(|hash| hash.as_str() == file.sha512)
        {
            // 取消选中只删未修改的文件（MIP WF-5）。
            plan.deletions.push(file.path.clone());
        }
    }

    let mip_base = resolve_mip_base(&state).await?;
    let fetcher = HttpFetcher {
        state: &state,
        mip_base: mip_base.as_deref(),
    };
    let mut progress = super::apply::ApplyProgress::create(
        instance_id,
        &pack_state.pack_id,
        &target_version,
        &plan,
        &entries,
    )
    .await;
    let mut outcome = apply_update(
        &instance_dir,
        &pack_state,
        &manifest,
        &plan,
        &entries,
        &fetcher,
        progress.as_mut(),
    )
    .await?;
    // build_new_state 会把 default feature 并入；这里以玩家显式选择为准。
    outcome.new_state.selected_features = selected;
    outcome.new_state.version = pack_state.version.clone();
    state::save(&instance_dir, &outcome.new_state).await?;

    Ok(YmclUpdateResult {
        applied: !plan.changes.is_empty() || !plan.deletions.is_empty(),
        staged_files: outcome.staged_files,
        deleted_files: outcome.deleted_files,
        new_version: outcome.new_state.version.clone(),
        skipped_files: outcome
            .skipped_files
            .iter()
            .map(|skipped| skipped.path.clone())
            .collect(),
    })
}

/// Validates a feature selection against a manifest: every id must be
/// declared and no two selected features may conflict (MIP §3.5).
pub(crate) fn validate_feature_selection(
    manifest: &MipManifest,
    selected: &[String],
) -> crate::Result<()> {
    let declared: HashMap<&str, &super::manifest::MipFeature> = manifest
        .features
        .iter()
        .map(|feature| (feature.id.as_str(), feature))
        .collect();
    for id in selected {
        if !declared.contains_key(id.as_str()) {
            return Err(
                crate::ErrorKind::OtherError(format!("Unknown feature {id}")).into()
            );
        }
    }
    for id in selected {
        for conflict in &declared[id.as_str()].conflicts {
            if selected.contains(conflict) {
                return Err(crate::ErrorKind::OtherError(format!(
                    "Features {id} and {conflict} are mutually exclusive"
                ))
                .into());
            }
        }
    }
    Ok(())
}

/// Resolves the active domain's MIP base URL; `Ok(None)` when no domain is
/// active or the domain has no MIP distribution face.
pub(crate) async fn resolve_mip_base(state: &State) -> crate::Result<Option<String>> {
    let active = registry::active_domain_id(&state.pool).await?;
    if active == registry::PERSONAL_DOMAIN_ID {
        return Ok(None);
    }
    let capabilities = registry::domain_capabilities(&active).await?;
    Ok(capabilities
        .mip
        .as_ref()
        .and_then(|mip| mip.base_url.clone()))
}

/// Everything WF-4 needs resolved up front: chosen server binding plus its
/// target manifest.
struct InstallTarget {
    server: MipServerBinding,
    binding: MipPackBinding,
    target_version: String,
    manifest: MipManifest,
}

/// Picks the binding (explicit server wins, else the only bound server) and
/// resolves its target manifest. The inner error is a user-facing "nothing
/// to install here" reason; the outer error is a transport failure.
async fn resolve_install_target(
    state: &State,
    server_id: Option<&str>,
) -> crate::Result<std::result::Result<InstallTarget, String>> {
    let Some(mip_base) = resolve_mip_base(state).await? else {
        return Ok(Err("请先在顶栏选择一个域".to_string()));
    };
    let servers_value = fetch_json(state, &servers_url(&mip_base)).await?;
    let servers: Vec<MipServerBinding> = serde_json::from_value(
        servers_value
            .get("servers")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )?;
    let bound: Vec<&MipServerBinding> = servers
        .iter()
        .filter(|server| {
            server
                .binding
                .as_ref()
                .is_some_and(|binding| !binding.pack_id.is_empty())
        })
        .collect();
    let server = match server_id {
        Some(id) => bound.iter().copied().find(|server| server.server_id == id),
        None if bound.len() == 1 => bound.first().copied(),
        None => None,
    };
    let Some(server) = server else {
        return Ok(Err(if server_id.is_none() && bound.len() > 1 {
            "多个服务器绑定了整合包，请选择一个".to_string()
        } else {
            "当前域没有绑定整合包的服务器".to_string()
        }));
    };
    let Some(binding) = server.binding.clone() else {
        return Ok(Err("当前域没有绑定整合包的服务器".to_string()));
    };
    let versions_value =
        fetch_json(state, &versions_url(&mip_base, &binding.pack_id)).await?;
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
        return Ok(Err(format!(
            "整合包 {} 没有已发布的版本",
            binding.pack_id
        )));
    };
    let manifest_value =
        fetch_json(state, &manifest_url(&mip_base, &binding.pack_id, target_version))
            .await?;
    let manifest: MipManifest = serde_json::from_value(manifest_value)?;
    manifest.validate()?;
    Ok(Ok(InstallTarget {
        server: server.clone(),
        binding,
        target_version: target_version.to_string(),
        manifest,
    }))
}

/// First-install preview (MIP WF-4 prereq): what would be installed into an
/// unmanaged instance for the chosen server binding, plus the feature
/// catalog for selection. `available: false` carries a user-facing reason.
#[derive(Serialize, Clone, Debug)]
pub struct YmclInstallPreview {
    pub available: bool,
    pub reason: Option<String>,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub pack_id: Option<String>,
    pub channel: Option<String>,
    pub target_version: Option<String>,
    pub season_id: Option<String>,
    pub season_name: Option<String>,
    pub install_files: usize,
    pub install_bytes: u64,
    pub features: Vec<super::manifest::MipFeature>,
}

fn unavailable_preview(reason: String) -> YmclInstallPreview {
    YmclInstallPreview {
        available: false,
        reason: Some(reason),
        server_id: None,
        server_name: None,
        pack_id: None,
        channel: None,
        target_version: None,
        season_id: None,
        season_name: None,
        install_files: 0,
        install_bytes: 0,
        features: Vec::new(),
    }
}

/// Previews the WF-4 install for an unmanaged instance.
pub async fn install_preview(
    instance_id: &str,
    server_id: Option<&str>,
) -> crate::Result<YmclInstallPreview> {
    let state = State::get().await?;
    let instance_dir = crate::api::instance::get_full_path(instance_id).await?;
    if let Some(pack_state) = state::load(&instance_dir).await? {
        return Ok(unavailable_preview(format!(
            "该实例已由整合包 {} 管理",
            pack_state.pack_id
        )));
    }
    let target = match resolve_install_target(&state, server_id).await? {
        Ok(target) => target,
        Err(reason) => return Ok(unavailable_preview(reason)),
    };
    let install_bytes = target
        .manifest
        .files
        .iter()
        .map(|file| file.size.unwrap_or(0))
        .sum();
    Ok(YmclInstallPreview {
        available: true,
        reason: None,
        server_id: Some(target.server.server_id.clone()),
        server_name: target.server.name.clone(),
        pack_id: Some(target.binding.pack_id.clone()),
        channel: target.binding.channel.clone(),
        target_version: Some(target.target_version),
        season_id: target
            .server
            .current_season
            .as_ref()
            .and_then(|season| season.season_id.clone()),
        season_name: target
            .server
            .current_season
            .as_ref()
            .and_then(|season| season.name.clone()),
        install_files: target.manifest.files.len(),
        install_bytes,
        features: target.manifest.features,
    })
}

/// Local instance that already holds a server's bound pack, surfaced so the
/// join modal can offer a direct launch instead of reinstall (YAP §7
/// 自动备齐整合包 → 一键进服：已备齐的玩家直接走启动).
#[derive(Serialize, Clone, Debug)]
pub struct YmclInstalledPack {
    pub instance_id: String,
    pub pack_id: String,
    pub version: Option<String>,
    /// Whether the installed version matches the binding's current target.
    /// `true` when the target could not be resolved (no scary outdated note).
    pub up_to_date: bool,
}

/// Join-flow preview (YAP §7 进服分流): what a server requires before a
/// player can join — either its bound pack (`has_pack`, with feature
/// catalog for opt-in) or just a game version (pure/vanilla servers).
/// `required_version` drives the local-instance match and the
/// download-vanilla fallback in the join modal.
#[derive(Serialize, Clone, Debug)]
pub struct YmclJoinPreview {
    pub available: bool,
    pub reason: Option<String>,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub mc_address: Option<String>,
    pub status: Option<String>,
    pub has_pack: bool,
    pub pack_id: Option<String>,
    pub channel: Option<String>,
    pub target_version: Option<String>,
    pub required_version: Option<String>,
    pub season_id: Option<String>,
    pub season_name: Option<String>,
    pub features: Vec<super::manifest::MipFeature>,
    /// Vanilla-enhanced pack the player may optionally download instead of
    /// pure vanilla (YAP §7 原版增强包); set only on `!has_pack` servers
    /// whose binding carries `optionalPackId` with a resolvable version.
    pub optional_pack_id: Option<String>,
    pub optional_target_version: Option<String>,
    pub optional_features: Vec<super::manifest::MipFeature>,
    /// Local instance already holding this server's bound pack. `None` when
    /// the server has no required pack or no local instance carries it.
    pub installed: Option<YmclInstalledPack>,
}

fn join_unavailable(reason: String) -> YmclJoinPreview {
    YmclJoinPreview {
        available: false,
        reason: Some(reason),
        server_id: None,
        server_name: None,
        mc_address: None,
        status: None,
        has_pack: false,
        pack_id: None,
        channel: None,
        target_version: None,
        required_version: None,
        season_id: None,
        season_name: None,
        features: Vec::new(),
        optional_pack_id: None,
        optional_target_version: None,
        optional_features: Vec::new(),
        installed: None,
    }
}

/// Scans local instances for one holding the server's current bound pack.
/// The instance's `.pack-state.json` pack id must equal the binding's pack
/// id — a same-server instance carrying a *previous season's* pack is not
/// recognized (MIP B.3: season switch prompts reinstall, no silent reuse).
/// Prefers the instance recorded against this server, then the most
/// recently modified one. Errors degrade to `None` (the modal just offers
/// a fresh install).
async fn find_installed_instance(
    state: &State,
    server_id: &str,
    pack_id: &str,
    target_version: Option<&str>,
) -> Option<YmclInstalledPack> {
    let instances = match crate::state::list_instances(&state.pool).await {
        Ok(instances) => instances,
        Err(error) => {
            tracing::warn!("MIP join: instance scan failed: {error}");
            return None;
        }
    };
    let mut candidates: Vec<(bool, chrono::DateTime<chrono::Utc>, String, String)> =
        Vec::new();
    for metadata in instances {
        let dir = match crate::api::instance::get_full_path(&metadata.instance.id).await {
            Ok(dir) => dir,
            Err(_) => continue,
        };
        let pack_state = match state::load(&dir).await {
            Ok(Some(pack_state)) => pack_state,
            _ => continue,
        };
        if pack_state.pack_id != pack_id {
            continue;
        }
        let this_server = pack_state
            .binding
            .as_ref()
            .is_some_and(|binding| binding.server_id == server_id);
        candidates.push((
            this_server,
            metadata.instance.modified,
            metadata.instance.id.clone(),
            pack_state.version,
        ));
    }
    candidates
        .into_iter()
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
        .map(|(_, _, instance_id, version)| {
            let up_to_date =
                target_version.is_none_or(|target| target == version.as_str());
            YmclInstalledPack {
                instance_id,
                pack_id: pack_id.to_string(),
                version: (!version.is_empty()).then_some(version),
                up_to_date,
            }
        })
}

/// Previews the join requirements of a domain server.
pub async fn join_preview(server_id: Option<&str>) -> crate::Result<YmclJoinPreview> {
    let state = State::get().await?;
    let Some(mip_base) = resolve_mip_base(&state).await? else {
        return Ok(join_unavailable("请先在顶栏选择一个域".to_string()));
    };
    let servers_value = fetch_json(&state, &servers_url(&mip_base)).await?;
    let servers: Vec<MipServerBinding> = serde_json::from_value(
        servers_value
            .get("servers")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )?;
    let server = match server_id {
        Some(id) => servers.iter().find(|server| server.server_id == id),
        None => servers.first(),
    };
    let Some(server) = server else {
        return Ok(join_unavailable(
            if server_id.is_none() {
                "当前域没有可加入的服务器"
            } else {
                "未找到该服务器"
            }
            .to_string(),
        ));
    };
    let binding = server.binding.clone().unwrap_or_default();
    let has_pack = !binding.pack_id.is_empty();
    let mut preview = YmclJoinPreview {
        available: true,
        reason: None,
        server_id: Some(server.server_id.clone()),
        server_name: server.name.clone(),
        mc_address: server.mc_address.clone(),
        status: server.status.clone(),
        has_pack,
        pack_id: if has_pack {
            Some(binding.pack_id.clone())
        } else {
            None
        },
        channel: binding.channel.clone(),
        target_version: None,
        required_version: binding.mc_version.clone(),
        season_id: server
            .current_season
            .as_ref()
            .and_then(|season| season.season_id.clone()),
        season_name: server
            .current_season
            .as_ref()
            .and_then(|season| season.name.clone()),
        features: Vec::new(),
        optional_pack_id: None,
        optional_target_version: None,
        optional_features: Vec::new(),
        installed: None,
    };
    if has_pack {
        let versions_value =
            fetch_json(&state, &versions_url(&mip_base, &binding.pack_id)).await?;
        let versions: Vec<MipVersionEntry> = serde_json::from_value(
            versions_value
                .get("versions")
                .cloned()
                .unwrap_or(serde_json::Value::Array(vec![])),
        )?;
        if let Some(target) = resolve_target_version(
            &versions,
            binding.channel.as_deref(),
            binding.pinned_version.as_deref(),
        ) {
            preview.target_version = Some(target.to_string());
            let manifest_value = fetch_json(
                &state,
                &manifest_url(&mip_base, &binding.pack_id, target),
            )
            .await?;
            let manifest: MipManifest = serde_json::from_value(manifest_value)?;
            manifest.validate()?;
            preview.required_version = manifest
                .game
                .as_ref()
                .and_then(|game| game.minecraft.clone())
                // 存量包（game 透传上线前入库）的兜底：绑定上的 mcVersion。
                .or(binding.mc_version.clone());
            preview.features = manifest.features;
        } else {
            // 绑定指向的包已无任何版本（被删除/重传换了新 id）：绑定已死，
            // 按纯服语义降级——玩家仍可用本地实例或下载原版进服。
            preview.has_pack = false;
            preview.pack_id = None;
        }
    }
    if !preview.has_pack
        && let Some(optional_pack_id) = binding
            .optional_pack_id
            .as_ref()
            .filter(|id| !id.is_empty())
    {
        // 原版增强包：纯原版服给玩家的第三个下载选择（YAP §7 进服分流）。
        let versions_value =
            fetch_json(&state, &versions_url(&mip_base, optional_pack_id))
                .await?;
        let versions: Vec<MipVersionEntry> = serde_json::from_value(
            versions_value
                .get("versions")
                .cloned()
                .unwrap_or(serde_json::Value::Array(vec![])),
        )?;
        if let Some(target) = resolve_target_version(
            &versions,
            binding.optional_channel.as_deref(),
            binding.optional_pinned_version.as_deref(),
        ) {
            let manifest_value = fetch_json(
                &state,
                &manifest_url(&mip_base, optional_pack_id, target),
            )
            .await?;
            let manifest: MipManifest = serde_json::from_value(manifest_value)?;
            manifest.validate()?;
            preview.optional_pack_id = Some(optional_pack_id.clone());
            preview.optional_target_version = Some(target.to_string());
            preview.optional_features = manifest.features;
        }
    }
    if preview.has_pack && let Some(pack_id) = preview.pack_id.clone() {
        // 本地已装识别（YAP §7 选定即启动）：绑定包的持有者可直接进服，
        // 版本落后由 run.rs 的 pre_launch_update 在启动前权威补齐。
        preview.installed =
            find_installed_instance(&state, &server.server_id, &pack_id, preview.target_version.as_deref())
                .await;
    }
    Ok(preview)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ymcl::mip::manifest::{MipFileEntry, MipSource};

    fn entry(sha512: &str) -> MipFileEntry {
        MipFileEntry {
            path: "mods/a.jar".to_string(),
            sha512: sha512.to_string(),
            size: None,
            policy: "managed".to_string(),
            feature: None,
            moved_from: None,
            sources: Vec::new(),
        }
    }

    #[test]
    fn cas_source_prefers_advertised_mip_base() {
        let entry = entry("abc123");
        let source = MipSource {
            source_type: "cas".to_string(),
            url: Some("http://localhost:9002/mip/objects/abc123".to_string()),
        };
        let urls = candidate_object_urls(
            &source,
            &entry,
            Some("http://localhost:9002/api/plugins/ymcl-adapter/mip"),
        );
        assert_eq!(
            urls,
            vec![
                "http://localhost:9002/api/plugins/ymcl-adapter/mip/objects/abc123"
                    .to_string(),
                "http://localhost:9002/mip/objects/abc123".to_string(),
            ]
        );
    }

    #[test]
    fn http_cdn_source_is_kept_as_is() {
        let entry = entry("abc123");
        let source = MipSource {
            source_type: "http".to_string(),
            url: Some("https://edge.forgecdn.net/files/1/2/a.jar".to_string()),
        };
        let urls = candidate_object_urls(
            &source,
            &entry,
            Some("http://localhost:9002/api/plugins/ymcl-adapter/mip"),
        );
        assert_eq!(
            urls,
            vec!["https://edge.forgecdn.net/files/1/2/a.jar".to_string()]
        );
    }

    #[test]
    fn html_spa_shell_is_detected_as_error_page() {
        assert!(looks_like_error_page(b"<!DOCTYPE html>\n<html>"));
        assert!(looks_like_error_page(b"  <html lang=\"en\">"));
        assert!(!looks_like_error_page(b"PK\x03\x04jar-bytes"));
        assert!(!looks_like_error_page(b"{\"code\":404}"));
    }
}

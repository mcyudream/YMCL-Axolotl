//! Publish pipeline (YAP §7, admin side in the launcher): compute the
//! diff between a local MIP instance and the domain binding's current
//! base version, and push it to the adapter as an initial mrpack or an
//! incremental delta upload (`/ingest` / `/ingest/delta`).

use serde::Serialize;
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::path::Path;

use super::state::{self, MipPackState};
use crate::State;
use crate::api::ymcl::registry;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct PublishDiffEntry {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Size in bytes for local files (changed/added/moved); absent for
    /// base-only paths (deleted), whose size the launcher never sees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct PublishDiff {
    pub managed: bool,
    pub pack_id: Option<String>,
    pub base_version: Option<String>,
    /// Server this instance is currently bound to (from the local pack
    /// state), so the publish dialog defaults the binding selector to it
    /// instead of "no binding".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound_server_id: Option<String>,
    pub changed: Vec<PublishDiffEntry>,
    pub added: Vec<PublishDiffEntry>,
    pub deleted: Vec<String>,
    pub moved: Vec<PublishDiffEntry>,
    /// Player-side files excluded from the push (seed/locked/disabled), plus
    /// paths struck by the persisted publish profile, listed so the admin
    /// sees what was filtered out (YAP §7 rule 4).
    pub excluded: Vec<String>,
    /// The instance's persisted publish profile (initial-publish exclusions
    /// and the latest feature/policy declarations), so the publish dialog can
    /// prefill the next push instead of re-asking. Absent before the first
    /// profile was saved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<state::PublishProfile>,
    /// Local files currently hidden by the persisted publish exclusions
    /// (present on disk, publishable, but struck by a profile rule). Shown
    /// in the dialog's recover tree: re-including them pushes their content
    /// and drops the matching rules from the profile.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub excluded_by_policy: Vec<PublishDiffEntry>,
    /// The builtin publish filters for the dialog's file tree — a mirror of
    /// [`is_excluded_from_publish`] so frontend and backend agree from a
    /// single source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<PublishFilters>,
}

/// The builtin publish filter rules as data, mirrored to the publish dialog.
#[derive(Serialize, Clone, Debug)]
pub struct PublishFilters {
    /// Top-level directory names never packed.
    pub top_level: Vec<&'static str>,
    /// Bookkeeping file names excluded wherever they appear.
    pub file_names: Vec<&'static str>,
    /// File extensions (case-insensitive, no dot) never packed.
    pub suffixes: Vec<&'static str>,
}

fn publish_filters() -> PublishFilters {
    PublishFilters {
        top_level: PUBLISH_EXCLUDED_TOP_LEVEL.to_vec(),
        file_names: vec![state::STATE_FILE_NAME, state::HASH_CACHE_FILE_NAME],
        suffixes: EXCLUDED_PUBLISH_EXTENSIONS.to_vec(),
    }
}

/// Admin-chosen paths kept out of the pack (YAP §7): files or directories,
/// matched with the same exact-or-directory-prefix semantics as the archive
/// builder's inclusion check, so an unchecked folder keeps its whole subtree
/// out of every later push until the admin re-selects it.
#[derive(Clone, Debug, Default)]
pub struct PublishExclusions {
    paths: Vec<String>,
}

impl PublishExclusions {
    pub fn new(paths: &[String]) -> Self {
        let mut normalized: Vec<String> = paths
            .iter()
            .map(|path| path.replace('\\', "/"))
            .collect();
        normalized.sort();
        normalized.dedup();
        Self { paths: normalized }
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Normalized paths, for merging back into the persisted profile.
    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    pub fn matches(&self, path: &str) -> bool {
        self.paths.iter().any(|excluded| {
            path == excluded
                || path
                    .strip_prefix(excluded.as_str())
                    .is_some_and(|suffix| suffix.starts_with('/'))
        })
    }

    /// True when any recorded path is `prefix` itself or sits under it — the
    /// re-include direction of [`Self::matches`]: does a recorded (re-include)
    /// path fall under an exclusion rule?
    pub fn any_under(&self, prefix: &str) -> bool {
        self.paths.iter().any(|path| {
            path == prefix
                || path
                    .strip_prefix(prefix)
                    .is_some_and(|suffix| suffix.starts_with('/'))
        })
    }
}

fn is_player_side(base: &MipPackState, path: &str) -> bool {
    base.locked_paths.iter().any(|locked| locked == path)
        || base.disabled_paths.iter().any(|disabled| disabled == path)
}

/// Computes what a push from the local instance would contain, relative to
/// the binding's base version. Pure over its inputs (the caller supplies
/// the local hash index and the persisted publish exclusions), unit tested.
pub fn compute_publish_diff(
    base: &MipPackState,
    local_hashes: &HashMap<String, String>,
    publish_excluded: &PublishExclusions,
) -> PublishDiff {
    let mut diff = PublishDiff {
        managed: true,
        pack_id: Some(base.pack_id.clone()),
        base_version: Some(base.version.clone()),
        ..PublishDiff::default()
    };

    // Local index minus protocol bookkeeping, player-side content, and
    // runtime logs (never pack-managed: content changes after install).
    let local: HashMap<&String, &String> = local_hashes
        .iter()
        .filter(|(path, _)| !is_excluded_from_publish(path.as_str()))
        .collect();

    // Reverse indexes keep the moved pairing linear instead of O(n·m):
    // base hash → first (sorted) base path, and whether each base hash
    // still exists somewhere locally.
    let mut base_path_by_hash: HashMap<&str, &String> = HashMap::new();
    for (path, file) in &base.files {
        base_path_by_hash
            .entry(file.sha512.as_str())
            .or_insert(path);
    }
    let local_hash_set: std::collections::HashSet<&str> =
        local.values().map(|hash| hash.as_str()).collect();

    // Changed and deleted: base paths compared against local content.
    for (path, base_file) in &base.files {
        if is_player_side(base, path) || publish_excluded.matches(path) {
            diff.excluded.push(path.clone());
            continue;
        }
        match local.get(path) {
            Some(local_hash) if local_hash.as_str() == base_file.sha512 => {} // unchanged
            Some(local_hash) => diff.changed.push(PublishDiffEntry {
                path: path.clone(),
                sha512: Some(local_hash.to_string()),
                from: None,
                size: None,
            }),
            // The content may have been moved: its base hash still exists
            // locally under another path, which the moved pass below pairs.
            None => {
                let moved_elsewhere =
                    local_hash_set.contains(base_file.sha512.as_str());
                if !moved_elsewhere {
                    diff.deleted.push(path.clone());
                }
            }
        }
    }

    // Added and moved: local paths absent from base, paired against base
    // hashes for renames (local copies of moved files keep the base hash).
    for (path, local_hash) in &local {
        if base.files.contains_key(*path)
            || is_player_side(base, path)
            || publish_excluded.matches(path)
        {
            continue;
        }
        if let Some(base_path) = base_path_by_hash.get(local_hash.as_str()) {
            diff.moved.push(PublishDiffEntry {
                path: (*path).clone(),
                sha512: Some((*local_hash).clone()),
                from: Some((*base_path).clone()),
                size: None,
            });
        } else {
            diff.added.push(PublishDiffEntry {
                path: (*path).clone(),
                sha512: Some((*local_hash).clone()),
                from: None,
                size: None,
            });
        }
    }

    diff
}

/// Computes the publish diff for an instance against the domain binding's
/// current base version (active domain required). Unmanaged instances get
/// a `managed: false` diff listing what an initial publish would upload
/// instead of an error, so the publish UI can offer the initial-package
/// path (YAP §7 step 1). The base is **synced with the server**: the diff
/// runs against the binding's current target manifest, so a deleted or
/// advanced server-side base is reflected immediately — a vanished base
/// degrades to the initial-publish offer instead of a stale "no changes".
pub async fn diff_instance(instance_id: &str) -> crate::Result<PublishDiff> {
    let (instance_dir, _mip_base) = publish_target(instance_id).await?;
    let local_hashes = super::update::scan_local_hashes_cached(&instance_dir)?;
    let Some(pack_state) = state::load(&instance_dir).await? else {
        return Ok(unmanaged_diff(&instance_dir, &local_hashes, None).await);
    };
    let profile = pack_state.publish_profile.clone();
    let state = State::get().await?;
    let Some(server_base) = resolve_server_base(&state, &pack_state).await? else {
        // 服务端基线已消失（版本被删 / 绑定解除 / 包被删）：按首发处理。
        return Ok(unmanaged_diff(&instance_dir, &local_hashes, profile).await);
    };
    let exclusions =
        PublishExclusions::new(profile.as_ref().map_or(&[], |p| p.excluded.as_slice()));
    let mut diff = compute_publish_diff(&server_base, &local_hashes, &exclusions);
    // Recover list: local publishable files (the pruned scan only sees those)
    // currently hidden by a profile rule, so the dialog can offer them back.
    // Player-side (locked/disabled) files stay out — they are managed by the
    // player's own state, not the publish policy.
    if !exclusions.is_empty() {
        let mut excluded_by_policy: Vec<PublishDiffEntry> = local_hashes
            .iter()
            .filter(|(path, _)| {
                exclusions.matches(path.as_str())
                    && !is_player_side(&server_base, path.as_str())
            })
            .map(|(path, _)| PublishDiffEntry {
                path: path.clone(),
                sha512: None,
                from: None,
                size: None,
            })
            .collect();
        excluded_by_policy.sort_by(|a, b| a.path.cmp(&b.path));
        diff.excluded_by_policy = excluded_by_policy;
    }
    diff.bound_server_id = pack_state.binding.as_ref().map(|binding| binding.server_id.clone());
    diff.profile = profile;
    diff.filters = Some(publish_filters());
    fill_entry_sizes(&instance_dir, &mut diff).await;
    Ok(diff)
}

/// Builds the unmanaged (initial-publish) diff: everything publishable shows
/// up as `added`, minus the builtin exclusions and the instance's persisted
/// publish exclusions (a republish after the server base vanished must not
/// resurrect files the admin deliberately unchecked).
async fn unmanaged_diff(
    instance_dir: &Path,
    local_hashes: &HashMap<String, String>,
    profile: Option<state::PublishProfile>,
) -> PublishDiff {
    let exclusions =
        PublishExclusions::new(profile.as_ref().map_or(&[], |p| p.excluded.as_slice()));
    let (mut pack_files, mut excluded) = split_initial_files(local_hashes);
    if !exclusions.is_empty() {
        pack_files.retain(|(path, _)| {
            if exclusions.matches(path) {
                excluded.push(path.clone());
                false
            } else {
                true
            }
        });
    }
    pack_files.sort();
    excluded.sort();
    let mut diff = PublishDiff {
        managed: false,
        profile,
        filters: Some(publish_filters()),
        ..PublishDiff::default()
    };
    diff.added = pack_files
        .into_iter()
        .map(|(path, sha512)| PublishDiffEntry {
            path,
            sha512: Some(sha512),
            from: None,
            size: None,
        })
        .collect();
    diff.excluded = excluded;
    fill_entry_sizes(instance_dir, &mut diff).await;
    diff
}

/// Resolves the server-side publish base for a managed instance, synced
/// with the binding's current target version: the returned state carries
/// the target manifest's files (sha512/policy/feature). `Ok(None)` means
/// the base is gone server-side (versions deleted, binding removed or pack
/// deleted) and an initial republish is the right move; transport errors
/// propagate so a down adapter never masquerades as "no changes".
async fn resolve_server_base(
    state: &State,
    pack_state: &MipPackState,
) -> crate::Result<Option<MipPackState>> {
    let Some((_binding, target_version, manifest)) =
        super::update::resolve_target_manifest(state, pack_state).await?
    else {
        return Ok(None);
    };
    let mut base = MipPackState {
        pack_id: manifest.pack_id.clone(),
        version: target_version,
        channel: manifest.channel.clone(),
        ..MipPackState::default()
    };
    for file in &manifest.files {
        base.files.insert(
            file.path.clone(),
            state::StateFile {
                sha512: file.sha512.clone(),
                policy: file.policy.clone(),
                feature: file.feature.clone(),
            },
        );
    }
    Ok(Some(base))
}

/// Stamps local file sizes onto the diff entries so the publish dialog can
/// show them per file (deleted paths are base-only and stay sizeless).
async fn fill_entry_sizes(instance_dir: &std::path::Path, diff: &mut PublishDiff) {
    for entry in diff
        .changed
        .iter_mut()
        .chain(diff.added.iter_mut())
        .chain(diff.moved.iter_mut())
        .chain(diff.excluded_by_policy.iter_mut())
    {
        if let Ok(metadata) = tokio::fs::metadata(instance_dir.join(&entry.path)).await {
            entry.size = Some(metadata.len());
        }
    }
}

/// Top-level entry names that never enter a published archive (player-side
/// content, caches/maps/recording directories a game run generates, and
/// protocol bookkeeping — YAP §7 rule 4). Mirrored to the publish dialog
/// via [`PublishFilters`].
const PUBLISH_EXCLUDED_TOP_LEVEL: &[&str] = &[
    "saves",
    "logs",
    "crash-reports",
    "screenshots",
    ".pack-staging",
    ".pack-backup",
    ".cache",
    ".mixin.out",
    ".fabric",
    ".quilt",
    "journeymap",
    "XaeroWorldMap",
    "XaeroWaypoints",
    "essential",
    "recordings",
    "replay_videos",
];

/// Root-level files the game rewrites on every run (player caches), never
/// pack content.
const PUBLISH_EXCLUDED_ROOT_FILES: &[&str] =
    &["usercache.json", "usernamecache.json"];

/// File extensions the game rewrites or records at runtime — logs and
/// ReplayMod captures. Their content changes after install and breaks
/// remote checksum verification (e.g. `authlib-injector.log` at the
/// instance root), and recordings leak player data.
const EXCLUDED_PUBLISH_EXTENSIONS: &[&str] = &["log", "replay"];

/// Paths that never enter a published pack, regardless of install location.
pub(crate) fn is_excluded_from_publish(path: &str) -> bool {
    if path == state::STATE_FILE_NAME
        || path == state::HASH_CACHE_FILE_NAME
        || PUBLISH_EXCLUDED_ROOT_FILES.contains(&path)
        || PUBLISH_EXCLUDED_TOP_LEVEL.iter().any(|name| {
            path == *name || path.starts_with(&format!("{name}/"))
        })
        || has_excluded_extension(path)
    {
        return true;
    }
    false
}

fn has_excluded_extension(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            EXCLUDED_PUBLISH_EXTENSIONS
                .iter()
                .any(|suffix| extension.eq_ignore_ascii_case(suffix))
        })
}

/// Splits a local hash index into (pack files, excluded player-side paths).
/// Pure over its inputs, unit tested.
fn split_initial_files(
    local_hashes: &HashMap<String, String>,
) -> (Vec<(String, String)>, Vec<String>) {
    let mut pack_files = Vec::new();
    let mut excluded = Vec::new();
    for (path, hash) in local_hashes {
        if is_excluded_from_publish(path) {
            excluded.push(path.clone());
        } else {
            pack_files.push((path.clone(), hash.clone()));
        }
    }
    pack_files.sort();
    excluded.sort();
    (pack_files, excluded)
}

/// Lists the active domain's servers (MIP appendix B) for the publish
/// dialog's binding selector. Empty when the domain has no MIP face.
pub async fn list_servers() -> crate::Result<Vec<super::update::MipServerBinding>> {
    let state = State::get().await?;
    let mip_base = active_mip_base().await?;
    let value = super::update::fetch_json(&state, &super::update::servers_url(&mip_base)).await?;
    let servers: Vec<super::update::MipServerBinding> = serde_json::from_value(
        value
            .get("servers")
            .cloned()
            .unwrap_or(serde_json::Value::Array(Vec::new())),
    )?;
    Ok(servers)
}

/// Admin-declared optional feature for a publish (mip.json features, MIP
/// §3.5) — the same shape the profile persists for later pushes.
pub use super::state::{PublishFeature as PublishFeatureInput, PublishPolicy as PublishPolicyInput};

/// Export candidates for the publish archive: everything the pack should
/// own, minus player-side content, runtime logs, and bookkeeping files.
async fn publish_candidates(
    instance_id: &str,
) -> crate::Result<Vec<String>> {
    let candidates =
        crate::api::instance::get_pack_export_candidates(instance_id).await?;
    Ok(candidates
        .into_iter()
        .filter(|candidate| !is_excluded_from_publish(candidate.as_str()))
        .map(|candidate| candidate.as_str().to_string())
        .collect())
}

/// Expands directory candidates into their recursive file lists so per-file
/// exclusions (persisted profile, dialog unchecks) hold even under checked
/// directories: the archive builder includes exactly the listed files. Files
/// that can never be pack-managed (`is_excluded_from_publish`) are dropped
/// here too, so a checked custom directory can't smuggle runtime logs into
/// the pack and then flip to "deleted" on the next delta.
async fn expand_candidate_files(
    instance_dir: &Path,
    candidates: Vec<String>,
    exclusions: &PublishExclusions,
) -> Vec<String> {
    let mut files = Vec::new();
    for candidate in candidates {
        let absolute = instance_dir.join(&candidate);
        if absolute.is_dir() {
            let mut stack = vec![absolute];
            while let Some(dir) = stack.pop() {
                let Ok(entries) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    let Ok(relative) = path.strip_prefix(instance_dir) else {
                        continue;
                    };
                    let relative = relative.to_string_lossy().replace('\\', "/");
                    if path.is_dir() {
                        if !exclusions.matches(&relative) {
                            stack.push(path);
                        }
                    } else if !exclusions.matches(&relative)
                        && !is_excluded_from_publish(&relative)
                    {
                        files.push(relative);
                    }
                }
            }
        } else if !exclusions.matches(&candidate) {
            files.push(candidate);
        }
    }
    files.sort();
    files.dedup();
    files
}

/// Slugifies an instance name for pack-id display (ASCII only; CJK names
/// degrade to empty → pure UUID).
fn slugify_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .trim_matches('-')
        .chars()
        .take(24)
        .collect()
}

/// Generates the initial pack id when the caller does not pin one:
/// `instance-slug-<uuid v4>` — readable in pack lists, and the UUID suffix
/// makes collisions (same instance name, two admins, concurrent publishes)
/// practically impossible (YAP §7).
async fn generate_pack_id(instance_id: &str) -> crate::Result<String> {
    let name = crate::api::instance::get(instance_id)
        .await?
        .map(|metadata| metadata.instance.name.clone())
        .unwrap_or_default();
    let slug = slugify_name(&name);
    let uuid = uuid::Uuid::new_v4().to_string();
    if slug.is_empty() {
        Ok(uuid)
    } else {
        Ok(format!("{slug}-{uuid}"))
    }
}

/// GitHub-hosted resources the published pack depends on, mirrored into the
/// domain CAS so player installs never need GitHub (YAP §7 主动镜像). Today
/// that means the Cleanroom loader — its installer jar and the GitHub
/// releases listing players resolve versions through; other loaders ship
/// from maven hosts and need no mirroring. Best-effort per resource: a
/// mirror failure is reported in the publish report but never fails the
/// publish itself (the pack is already up; the mirror only adds
/// availability for GitHub-unreachable players).
async fn mirror_github_resources(
    instance_id: &str,
    mip_base: &str,
) -> Vec<serde_json::Value> {
    // (url, kind, force): the installer jar is immutable per version (skip
    // when already registered); the releases listing evolves, so re-push to
    // pick up tags published since the last mirror.
    let mut targets: Vec<(String, &str, bool)> = Vec::new();
    if let Ok(Some(metadata)) = crate::api::instance::get(instance_id).await
        && metadata.applied_content_set.loader == crate::state::ModLoader::Cleanroom
    {
        if let Some(loader_version) = metadata
            .applied_content_set
            .loader_version
            .as_deref()
            .filter(|version| !version.is_empty())
        {
            targets.push((
                crate::api::loader_metadata::cleanroom_installer_url(loader_version),
                "loader-installer",
                false,
            ));
        }
        targets.push((
            crate::api::loader_metadata::CLEANROOM_RELEASES_URL.to_string(),
            "cleanroom-releases",
            true,
        ));
    }
    let mut mirrors = Vec::with_capacity(targets.len());
    for (url, kind, force) in targets {
        mirrors.push(super::mirrors::ensure_mirrored(mip_base, &url, kind, force).await);
    }
    mirrors
}

/// Stamps the mirror summary onto the publish report; no-op when nothing
/// was mirror-eligible so non-GitHub publishes keep their report unchanged.
fn attach_mirrors(report: &mut serde_json::Value, mirrors: Vec<serde_json::Value>) {
    if mirrors.is_empty() {
        return;
    }
    if let Some(object) = report.as_object_mut() {
        object.insert("mirrors".to_string(), serde_json::Value::Array(mirrors));
    }
}

/// Pushes a non-managed local instance as an initial package (YAP §7 step
/// 1): the instance is packed into a standard mrpack (with mip.json, MIP
/// §3.5) and uploaded multipart to the adapter, which routes CDN references
/// into manifest sources and overrides into CAS (MIP WF-1). On success the
/// instance becomes MIP-managed — the pack state is adopted from the
/// returned manifest so later publishes are deltas. `pack_id` is optional:
/// when omitted the launcher generates a unique `slug-<uuid>` id. `notes`
/// is the optional release notes players see when updating.
#[allow(clippy::too_many_arguments)]
pub async fn push_initial(
    instance_id: &str,
    pack_id: Option<&str>,
    version: &str,
    channel: Option<&str>,
    bind: Option<serde_json::Value>,
    features: Vec<PublishFeatureInput>,
    policies: Vec<PublishPolicyInput>,
    exclude: Vec<String>,
    notes: Option<&str>,
) -> crate::Result<serde_json::Value> {
    let (instance_dir, mip_base) = publish_target(instance_id).await?;
    let pack_id = match pack_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(explicit) => {
            validate_pack_id(explicit)?;
            explicit.to_string()
        }
        None => generate_pack_id(instance_id).await?,
    };
    if version.trim().is_empty() {
        return Err(crate::ErrorKind::OtherError(
            "A version number is required for the initial publish".to_string(),
        )
        .into());
    }
    // Admin file selection (publish dialog tree): unchecked paths stay out of
    // the pack — and out of every later push, as the persisted profile.
    let exclusions = PublishExclusions::new(&exclude);
    let candidates = expand_candidate_files(
        &instance_dir,
        publish_candidates(instance_id).await?,
        &exclusions,
    )
    .await;
    if candidates.is_empty() {
        return Err(crate::ErrorKind::OtherError(
            "The instance has no files to publish".to_string(),
        )
        .into());
    }

    // mip.json（MIP §3.5）：features 用 glob 声明可选内容，policies 声明
    // seed/merge 等文件策略。
    let mip_json = serde_json::json!({
        "formatVersion": 1,
        "features": features
            .iter()
            .map(|feature| {
                serde_json::json!({
                    "id": feature.id,
                    "name": feature.name,
                    "default": feature.default,
                    "conflicts": feature.conflicts,
                    "files": feature.files,
                })
            })
            .collect::<Vec<_>>(),
        "policies": policies
            .iter()
            .map(|policy| {
                serde_json::json!({
                    "glob": policy.glob,
                    "policy": policy.policy,
                })
            })
            .collect::<Vec<_>>(),
    });
    let archive_path =
        crate::api::instance::build_publish_archive(
            instance_id,
            candidates,
            Some(version.to_string()),
            mip_json,
        )
        .await?;

    let upload = match build_initial_upload(
        &archive_path,
        version,
        channel,
        bind.as_ref(),
        notes,
    )
    .await
    {
        Ok(form) => form,
        Err(error) => {
            tokio::fs::remove_file(&archive_path).await.ok();
            return Err(error);
        }
    };
    let report =
        post_multipart(&mip_base, &format!("/api/packs/{pack_id}/ingest"), upload)
            .await;
    tokio::fs::remove_file(&archive_path).await.ok();
    let report = report?;

    // Record which server the pack was bound to so the instance's own state
    // is authoritative on the next diff (resolve_target_manifest matches the
    // state binding first, then falls back to a reverse pack-id lookup).
    let bind_server = bind
        .as_ref()
        .and_then(|bind| bind.get("serverId"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty());
    let bind_season = bind
        .as_ref()
        .and_then(|bind| bind.get("seasonId"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty());
    adopt_state_from_report(
        &instance_dir,
        &pack_id,
        version,
        channel,
        bind_server,
        bind_season,
        &report,
        state::PublishProfile {
            excluded: exclusions.paths().to_vec(),
            features,
            policies,
        },
    )
    .await?;
    let mut report = report;
    attach_mirrors(&mut report, mirror_github_resources(instance_id, &mip_base).await);
    Ok(report)
}

/// Builds the multipart ingest form: `file` = mrpack zip + metadata fields.
async fn build_initial_upload(
    archive_path: &std::path::Path,
    version: &str,
    channel: Option<&str>,
    bind: Option<&serde_json::Value>,
    notes: Option<&str>,
) -> crate::Result<reqwest::multipart::Form> {
    let bytes = tokio::fs::read(archive_path).await?;
    let mut form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(bytes)
            .mime_str("application/zip")?
            .file_name("pack.mrpack"),
    );
    form = form.text("version", version.to_string());
    if let Some(channel) = channel {
        form = form.text("channel", channel.to_string());
    }
    if let Some(notes) = notes.map(str::trim).filter(|notes| !notes.is_empty()) {
        form = form.text("notes", notes.to_string());
    }
    if let Some(bind) = bind {
        form = form.text("bind", bind.to_string());
    }
    Ok(form)
}

/// Adopts the publishing instance as MIP-managed from the adapter's ingest
/// report: state files come from the generated manifest (the source of
/// truth, covering CDN-referenced files), default-enabled features count as
/// selected (WF-4 default behavior for the author instance), and the admin's
/// publish profile (exclusions + feature/policy declarations) is persisted
/// so later deltas reuse the same selection.
#[allow(clippy::too_many_arguments)]
async fn adopt_state_from_report(
    instance_dir: &std::path::Path,
    pack_id: &str,
    version: &str,
    channel: Option<&str>,
    binding_server: Option<&str>,
    binding_season: Option<&str>,
    report: &serde_json::Value,
    publish_profile: state::PublishProfile,
) -> crate::Result<()> {
    let manifest = report.get("manifest").ok_or_else(|| {
        crate::ErrorKind::OtherError(
            "Adapter ingest report is missing the generated manifest"
                .to_string(),
        )
    })?;
    let mut pack_state = MipPackState {
        pack_id: pack_id.to_string(),
        version: version.to_string(),
        channel: channel.map(|value| value.to_string()),
        binding: binding_server.map(|server_id| super::state::MipInstanceBinding {
            server_id: server_id.to_string(),
            season_id: binding_season.map(|season| season.to_string()),
        }),
        publish_profile: Some(publish_profile),
        ..MipPackState::default()
    };
    if let Some(files) = manifest.get("files").and_then(|files| files.as_array()) {
        for file in files {
            let (Some(path), Some(sha512)) = (
                file.get("path").and_then(|path| path.as_str()),
                file.get("sha512").and_then(|sha| sha.as_str()),
            ) else {
                continue;
            };
            pack_state.files.insert(
                path.to_string(),
                state::StateFile {
                    sha512: sha512.to_string(),
                    policy: file
                        .get("policy")
                        .and_then(|policy| policy.as_str())
                        .unwrap_or("managed")
                        .to_string(),
                    feature: file
                        .get("feature")
                        .and_then(|feature| feature.as_str())
                        .map(|feature| feature.to_string()),
                },
            );
        }
    }
    if let Some(features) = manifest.get("features").and_then(|f| f.as_array()) {
        for feature in features {
            let Some(id) = feature.get("id").and_then(|id| id.as_str()) else {
                continue;
            };
            pack_state
                .declared_features
                .get_or_insert_with(Vec::new)
                .push(id.to_string());
            let default_on = feature
                .get("default")
                .and_then(|default| default.as_bool())
                .unwrap_or(false);
            if default_on {
                pack_state.selected_features.push(id.to_string());
            }
        }
    }
    state::save(instance_dir, &pack_state).await
}

/// Pack ids become URL path segments and storage doc ids on the adapter.
fn validate_pack_id(pack_id: &str) -> crate::Result<()> {
    let valid = !pack_id.is_empty()
        && pack_id.len() <= 64
        && pack_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !pack_id.starts_with('.')
        && !pack_id.starts_with('-');
    if valid {
        Ok(())
    } else {
        Err(crate::ErrorKind::OtherError(
            "Pack id must be 1-64 ASCII letters, digits, '-', '_' or '.', and cannot start with '.' or '-'"
                .to_string(),
        )
        .into())
    }
}

/// Sends a multipart request to the adapter with the freshest domain session
/// attached (publishes are audited against the operator, MIP 附录 B.5 /
/// YAP §7 rule 5). reqwest sets the multipart boundary itself. Shared by the
/// ingest/delta pushes and the CAS mirror registration (`mirrors.rs`).
pub(crate) async fn post_multipart(
    mip_base: &str,
    path: &str,
    form: reqwest::multipart::Form,
) -> crate::Result<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()?;
    let url = format!("{mip_base}{path}");
    let state = State::get().await?;
    let active = registry::active_domain_id(&state.pool).await?;
    let session = crate::api::ymcl::auth::ensure_session(&active)
        .await
        .ok()
        .flatten();
    let mut request = client.post(&url);
    if let Some(token) = session.as_ref().map(|s| s.access_token.as_str()) {
        request = request.header("Authorization", token);
    }
    let response = request.multipart(form).send().await?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Publish failed ({status}): {text}"
        ))
        .into());
    }
    Ok(serde_json::from_str(&text).unwrap_or(serde_json::Value::Null))
}

/// Pushes the diff as an incremental delta publish. The adapter composes
/// the standard immutable MIP manifest from base + delta (YAP §7). `exclude`
/// carries paths the admin unchecked in this push's file tree; they are left
/// out of the delta and merged into the persisted publish profile so they
/// stay out of every later push too. `include` is the recover direction:
/// paths picked from the excluded-by-policy tree are pushed with this delta
/// and their matching exclusion rules are dropped, so the content is managed
/// again from now on. The latest feature/policy declarations replace the
/// profile's after a successful upload.
/// later push via the persisted profile (merged after upload succeeds). The
/// latest feature/policy declarations replace the profile's after a
/// successful upload. `notes` is the optional release notes players see
/// when updating to this version.
#[allow(clippy::too_many_arguments)]
pub async fn push_delta(
    instance_id: &str,
    version: &str,
    channel: Option<&str>,
    bind: Option<serde_json::Value>,
    features: Vec<PublishFeatureInput>,
    policies: Vec<PublishPolicyInput>,
    exclude: Vec<String>,
    include: Vec<String>,
    notes: Option<&str>,
) -> crate::Result<serde_json::Value> {
    let (instance_dir, mut pack_state, mip_base) =
        instance_context(instance_id).await?;
    // 基线与服务端同步：对绑定当前目标版本做增量；基线已被删则拒绝并
    // 引导走初始包重发（前端由 diff 的 managed:false 分流）。首发时持久化的
    // 排除策略继续生效：管理员刻意排除的文件不再以「新增」反复出现。
    let state = State::get().await?;
    let Some(server_base) = resolve_server_base(&state, &pack_state).await? else {
        return Err(crate::ErrorKind::OtherError(
            "服务端基线已不存在（版本可能已被删除），请作为初始包重新发布"
                .to_string(),
        )
        .into());
    };
    // 找回路径命中规则即移除该规则：内容重新纳管，此后照常出现在变更里。
    let request_includes = PublishExclusions::new(&include);
    let effective_rules: Vec<String> = pack_state
        .publish_profile
        .as_ref()
        .map_or(Vec::new(), |profile| {
            profile
                .excluded
                .iter()
                .filter(|rule| !request_includes.any_under(rule))
                .cloned()
                .collect()
        });
    let profile_exclusions = PublishExclusions::new(&effective_rules);
    let local_hashes = super::update::scan_local_hashes_cached(&instance_dir)?;
    let mut diff =
        compute_publish_diff(&server_base, &local_hashes, &profile_exclusions);

    // This push's unchecked paths: out of the delta now, and out of every
    // later push via the persisted profile (merged after upload succeeds).
    let request_exclusions = PublishExclusions::new(&exclude);
    if !request_exclusions.is_empty() {
        diff.changed
            .retain(|entry| !request_exclusions.matches(&entry.path));
        diff.added
            .retain(|entry| !request_exclusions.matches(&entry.path));
        diff.moved
            .retain(|entry| !request_exclusions.matches(&entry.path));
    }
    if diff.changed.is_empty()
        && diff.added.is_empty()
        && diff.deleted.is_empty()
        && diff.moved.is_empty()
    {
        return Err(crate::ErrorKind::OtherError(
            "No changes to publish: the instance matches the bound version"
                .to_string(),
        )
        .into());
    }

    // Multipart（MIP 附录 B）：part "delta" = JSON 索引，changed/added 的
    // 二进制按出现顺序放在 "file0"、"file1"、…。changed 条目整条替换 base
    // 的 manifest 条目，所以显式带上 base 的 policy 与 feature（state 有
    // 记录时）；features/policies 声明交给适配器对新增条目做 glob 标注。
    //
    // 自愈存量 manifest：旧包入库时没有 game，delta 带上本实例的游戏版本与
    // 加载器，适配器优先用它回填（无则继承 base）；loader 键名与 mrpack
    // dependencies 对齐，让 delta 版本合成出的 mrpack 也带加载器。
    let metadata = crate::api::instance::get(instance_id).await?;
    let game = metadata
        .as_ref()
        .map(|metadata| &metadata.applied_content_set)
        .map(|content_set| {
            let loader_key = match content_set.loader {
                crate::state::ModLoader::Forge => Some("forge"),
                crate::state::ModLoader::NeoForge => Some("neoforge"),
                crate::state::ModLoader::Fabric => Some("fabric-loader"),
                crate::state::ModLoader::Quilt => Some("quilt-loader"),
                crate::state::ModLoader::Cleanroom => Some("cleanroom"),
                _ => None,
            };
            let loader = loader_key
                .zip(content_set.loader_version.clone())
                .map(|(loader_type, loader_version)| {
                    serde_json::json!({
                        "type": loader_type,
                        "version": loader_version,
                    })
                });
            serde_json::json!({
                "minecraft": content_set.game_version.clone(),
                "loader": loader,
            })
        });
    let mut delta = serde_json::json!({
        "baseVersion": diff.base_version,
        "version": version,
        "changed": [],
        "added": [],
        "deleted": diff.deleted,
        "moved": diff.moved,
        "game": game,
        "features": features
            .iter()
            .map(|feature| {
                serde_json::json!({
                    "id": feature.id,
                    "name": feature.name,
                    "default": feature.default,
                    "conflicts": feature.conflicts,
                    "files": feature.files,
                })
            })
            .collect::<Vec<_>>(),
        "policies": policies
            .iter()
            .map(|policy| {
                serde_json::json!({
                    "glob": policy.glob,
                    "policy": policy.policy,
                })
            })
            .collect::<Vec<_>>(),
    });
    let mut form = reqwest::multipart::Form::new();
    let mut part_index = 0usize;
    for (list, key) in [(&diff.changed, "changed"), (&diff.added, "added")] {
        for file in list {
            let bytes = tokio::fs::read(instance_dir.join(&file.path)).await?;
            form = form.part(
                format!("file{part_index}"),
                reqwest::multipart::Part::bytes(bytes)
                    .mime_str("application/octet-stream")?,
            );
            let mut entry = serde_json::json!({ "path": file.path });
            if let Some(base_file) = server_base.files.get(file.path.as_str()) {
                entry["policy"] =
                    serde_json::Value::String(base_file.policy.clone());
                if let Some(feature) = &base_file.feature {
                    entry["feature"] =
                        serde_json::Value::String(feature.clone());
                }
            }
            delta[key].as_array_mut().expect("array literal").push(entry);
            part_index += 1;
        }
    }
    if let Some(bind) = bind {
        delta["bind"] = bind.clone();
    }
    if let Some(channel_value) = channel {
        delta["channel"] = serde_json::Value::String(channel_value.to_string());
    }
    if let Some(notes) = notes.map(str::trim).filter(|notes| !notes.is_empty()) {
        delta["notes"] = serde_json::Value::String(notes.to_string());
    }
    form = form.text("delta", delta.to_string());

    let mut report = post_multipart(
        &mip_base,
        &format!("/api/packs/{}/ingest/delta", pack_state.pack_id),
        form,
    )
    .await?;

    // Persist the publish profile only after the upload succeeded: a failed
    // push must not silently record selections that never shipped. Rules hit
    // by a re-include are dropped (content managed again); this push's
    // unchecked paths are merged in.
    let mut profile = pack_state.publish_profile.take().unwrap_or_default();
    profile
        .excluded
        .retain(|rule| !request_includes.any_under(rule));
    for path in request_exclusions.paths() {
        if !profile.excluded.contains(path) {
            profile.excluded.push(path.clone());
        }
    }
    profile.excluded.sort();
    profile.features = features;
    profile.policies = policies;
    pack_state.publish_profile = Some(profile);
    state::save(&instance_dir, &pack_state).await?;

    attach_mirrors(&mut report, mirror_github_resources(instance_id, &mip_base).await);
    Ok(report)
}

/// Resolves the MIP distribution base URL for the active domain; requires
/// an active domain with a MIP capability. The cached capabilities
/// snapshot can predate the adapter gaining its MIP face, so a missing
/// node triggers one re-probe before giving up (self-heal).
async fn active_mip_base() -> crate::Result<String> {
    let state = State::get().await?;
    let active = registry::active_domain_id(&state.pool).await?;
    if active == registry::PERSONAL_DOMAIN_ID {
        return Err(crate::ErrorKind::OtherError(
            "Publishing requires an active domain".to_string(),
        )
        .into());
    }
    let capabilities = registry::domain_capabilities(&active).await?;
    let mut mip_base = capabilities
        .mip
        .as_ref()
        .and_then(|mip| mip.base_url.clone());
    if mip_base.is_none() {
        if let Ok(fresh) = registry::refresh_capabilities(&active).await {
            mip_base = fresh.mip.and_then(|mip| mip.base_url);
        }
    }
    mip_base.ok_or_else(|| {
        crate::ErrorKind::OtherError(
            "The active domain has no MIP distribution face".to_string(),
        )
        .into()
    })
}

/// Resolves the (instance dir, MIP base URL) pair every publish operation
/// needs.
async fn publish_target(instance_id: &str) -> crate::Result<(std::path::PathBuf, String)> {
    let instance_dir = crate::api::instance::get_full_path(instance_id).await?;
    let mip_base = active_mip_base().await?;
    Ok((instance_dir, mip_base))
}

/// Resolves the triple (instance dir, pack state, MIP base URL) that every
/// managed-instance publish operation needs.
async fn instance_context(
    instance_id: &str,
) -> crate::Result<(std::path::PathBuf, MipPackState, String)> {
    let (instance_dir, mip_base) = publish_target(instance_id).await?;
    let Some(pack_state) = state::load(&instance_dir).await? else {
        return Err(crate::ErrorKind::OtherError(
            "This instance is not a MIP-managed instance".to_string(),
        )
        .into());
    };
    Ok((instance_dir, pack_state, mip_base))
}

/// A released pack version the admin can inspect or withdraw.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PackVersionInfo {
    pub version: String,
    pub channel: Option<String>,
    /// Release notes attached at publish (optional).
    pub notes: Option<String>,
}

/// Lists released versions for a pack (newest first as returned by the
/// adapter). Used by the publish console's withdraw picker.
pub async fn list_versions(pack_id: &str) -> crate::Result<Vec<PackVersionInfo>> {
    let state = State::get().await?;
    let mip_base = active_mip_base().await?;
    let value = super::update::fetch_json(
        &state,
        &super::update::versions_url(&mip_base, pack_id),
    )
    .await?;
    let versions: Vec<super::update::MipVersionEntry> = serde_json::from_value(
        value
            .get("versions")
            .cloned()
            .unwrap_or(serde_json::Value::Array(Vec::new())),
    )?;
    Ok(versions
        .into_iter()
        .map(|entry| PackVersionInfo {
            version: entry.version.clone(),
            channel: entry.channel.clone(),
            notes: entry.notes.clone(),
        })
        .collect())
}

/// Lists released versions for the pack bound to a managed instance.
pub async fn list_instance_versions(
    instance_id: &str,
) -> crate::Result<Vec<PackVersionInfo>> {
    let (_, pack_state, _) = instance_context(instance_id).await?;
    list_versions(&pack_state.pack_id).await
}

/// Withdraws (revokes) a published pack version on the adapter so clients
/// stop resolving it as the update target. The instance keeps whatever
/// version it already has; a subsequent publish can supersede the withdrawn
/// release.
pub async fn withdraw_version(
    pack_id: &str,
    version: &str,
) -> crate::Result<serde_json::Value> {
    if pack_id.trim().is_empty() || version.trim().is_empty() {
        return Err(crate::ErrorKind::OtherError(
            "pack id and version are required to withdraw a release".to_string(),
        )
        .into());
    }
    let state = State::get().await?;
    let mip_base = active_mip_base().await?;
    let active = registry::active_domain_id(&state.pool).await?;
    // MIP §4: yank is a POST on the version resource — it hides the release
    // from the versions list while keeping the immutable manifest for
    // instances that already have it.
    let url = format!(
        "{mip_base}/api/packs/{pack_id}/versions/{version}/yank",
        pack_id = urlencoding_minimal(pack_id),
        version = urlencoding_minimal(version),
    );
    let bytes = crate::api::ymcl::auth::domain_request(
        &state,
        &active,
        reqwest::Method::POST,
        &url,
        None,
    )
    .await?;
    if bytes.is_empty() {
        return Ok(serde_json::json!({
            "packId": pack_id,
            "version": version,
            "withdrawn": true,
        }));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

/// Withdraws a release for the pack bound to a managed instance.
pub async fn withdraw_instance_version(
    instance_id: &str,
    version: &str,
) -> crate::Result<serde_json::Value> {
    let (_, pack_state, _) = instance_context(instance_id).await?;
    withdraw_version(&pack_state.pack_id, version).await
}

/// Minimal path-segment encoding: pack ids and versions are ASCII slugs, but
/// still escape characters that would break the URL path.
fn urlencoding_minimal(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
                ch.to_string()
            } else {
                format!("%{:02X}", ch as u32)
            }
        })
        .collect()
}

/// sha512 of file bytes; used by tests and kept public for the push
/// frontend to display hashes it computes client-side.
pub fn sha512_hex(bytes: &[u8]) -> String {
    let digest = Sha512::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with(files: &[(&str, &str)]) -> MipPackState {
        let mut state = MipPackState {
            pack_id: "test".into(),
            version: "1.0.0".into(),
            ..MipPackState::default()
        };
        for (path, hash) in files {
            state.files.insert(
                (*path).to_string(),
                super::super::state::StateFile {
                    sha512: (*hash).to_string(),
                    policy: "managed".into(),
                    feature: None,
                },
            );
        }
        state
    }

    fn local(hashes: &[(&str, &str)]) -> HashMap<String, String> {
        hashes
            .iter()
            .map(|(path, hash)| ((*path).to_string(), (*hash).to_string()))
            .collect()
    }

    #[test]
    fn initial_split_separates_player_side_files() {
        let local = local(&[
            ("mods/a.jar", "h1"),
            ("config/common.toml", "h2"),
            ("saves/world/level.dat", "h3"),
            ("logs/latest.log", "h4"),
            ("screenshots/2026-09-16.png", "h5"),
            ("crash-reports/crash.txt", "h6"),
            ("authlib-injector.log", "h7"),
            ("config/debug.LOG", "h8"),
        ]);
        let (pack_files, excluded) = split_initial_files(&local);
        assert_eq!(
            pack_files,
            vec![
                ("config/common.toml".to_string(), "h2".to_string()),
                ("mods/a.jar".to_string(), "h1".to_string()),
            ]
        );
        assert_eq!(
            excluded,
            vec![
                "authlib-injector.log",
                "config/debug.LOG",
                "crash-reports/crash.txt",
                "logs/latest.log",
                "saves/world/level.dat",
                "screenshots/2026-09-16.png",
            ]
        );
    }

    #[test]
    fn changed_added_deleted_moved_are_classified() {
        let base = state_with(&[
            ("mods/same.jar", "h-same"),
            ("mods/changed.jar", "h-old"),
            ("mods/gone.jar", "h-gone"),
            ("mods/renamed-old.jar", "h-renamed"),
        ]);
        let local = local(&[
            ("mods/same.jar", "h-same"),
            ("mods/changed.jar", "h-new"),
            ("mods/renamed-new.jar", "h-renamed"),
            ("mods/added.jar", "h-added"),
        ]);

        let diff = compute_publish_diff(&base, &local, &PublishExclusions::default());
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].path, "mods/changed.jar");
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].path, "mods/added.jar");
        assert_eq!(diff.deleted, vec!["mods/gone.jar"]);
        assert_eq!(diff.moved.len(), 1);
        assert_eq!(diff.moved[0].path, "mods/renamed-new.jar");
        assert_eq!(diff.moved[0].from.as_deref(), Some("mods/renamed-old.jar"));
    }

    #[test]
    fn player_side_files_are_excluded_from_push() {
        let mut base = state_with(&[
            ("mods/locked.jar", "h-l"),
            ("mods/disabled.jar", "h-d"),
        ]);
        base.locked_paths.push("mods/locked.jar".into());
        base.disabled_paths.push("mods/disabled.jar".into());
        // Local diverges wildly: still must not enter the push.
        let local = local(&[
            ("mods/locked.jar", "h-hacked"),
            ("mods/renamed.jar", "h-d"),
        ]);

        let diff = compute_publish_diff(&base, &local, &PublishExclusions::default());
        assert!(diff.changed.is_empty() && diff.deleted.is_empty());
        assert_eq!(diff.excluded.len(), 2);
        // The renamed copy of a disabled file counts as an addition of a
        // local file, but the disabled base path itself is excluded.
        assert!(
            diff.added
                .iter()
                .all(|entry| entry.path != "mods/disabled.jar")
        );
    }

    #[test]
    fn persisted_exclusions_keep_files_out_of_the_push() {
        let base = state_with(&[
            ("config/a.toml", "h-a-old"),
            ("mods/extra.jar", "h-extra-old"),
            ("mods/gone.jar", "h-gone"),
        ]);
        let local = local(&[
            ("config/a.toml", "h-a-new"),
            ("mods/extra.jar", "h-extra-new"),
            ("mods/new/thing.jar", "h-thing"),
            ("config/added.toml", "h-added"),
        ]);
        let exclusions = PublishExclusions::new(&[
            "mods/extra.jar".to_string(),
            "mods/new".to_string(),
        ]);

        let diff = compute_publish_diff(&base, &local, &exclusions);
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].path, "config/a.toml");
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].path, "config/added.toml");
        // The excluded changed file is neither pushed nor reported deleted,
        // and the excluded new directory stays out of the additions.
        assert!(diff.deleted.iter().all(|path| path != "mods/extra.jar"));
        assert!(diff.excluded.contains(&"mods/extra.jar".to_string()));
        assert!(diff
            .added
            .iter()
            .all(|entry| entry.path != "mods/new/thing.jar"));
    }

    #[test]
    fn exclusion_matcher_uses_exact_or_directory_prefix() {
        let exclusions = PublishExclusions::new(&["mods/new".to_string()]);
        assert!(exclusions.matches("mods/new"));
        assert!(exclusions.matches("mods/new/thing.jar"));
        assert!(!exclusions.matches("mods/newt.jar"));
        assert!(!exclusions.matches("config/mods/new"));
    }

    #[test]
    fn any_under_answers_whether_a_rule_covers_a_reinclude() {
        let includes = PublishExclusions::new(&[
            "mods/optifine/config.cfg".to_string(),
            "config/x.toml".to_string(),
        ]);
        // A directory rule covering the re-included file must be dropped…
        assert!(includes.any_under("mods/optifine"));
        assert!(includes.any_under("config/x.toml"));
        // …while unrelated rules survive.
        assert!(!includes.any_under("mods/other"));
        assert!(!includes.any_under("mods/optifine-extra"));
    }

    #[test]
    fn publish_profile_roundtrips_through_state_json() {
        let mut state = state_with(&[]);
        state.publish_profile = Some(super::super::state::PublishProfile {
            excluded: vec!["mods/bad.jar".into()],
            features: vec![super::super::state::PublishFeature {
                id: "shaders".into(),
                name: Some("Shaders".into()),
                default: true,
                conflicts: vec!["vanilla".into()],
                files: vec!["shaderpacks/**".into()],
            }],
            policies: vec![super::super::state::PublishPolicy {
                glob: "options.txt".into(),
                policy: "seed".into(),
            }],
        });
        let json = serde_json::to_string(&state).expect("serialize");
        let parsed: MipPackState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, state);
        // States written before the profile existed still load.
        let legacy: MipPackState =
            serde_json::from_str(r#"{"pack_id":"p","version":"1.0.0"}"#)
                .expect("legacy state");
        assert!(legacy.publish_profile.is_none());
    }

    #[test]
    fn runtime_junk_paths_are_excluded_from_publish() {
        for path in [
            ".mixin.out/mixin.log",
            "usercache.json",
            "usernamecache.json",
            "XaeroWorldMap/multiplayer/map.zip",
            "replay_2026_09_18_12_00_00.replay",
            state::HASH_CACHE_FILE_NAME,
        ] {
            assert!(
                is_excluded_from_publish(path),
                "{path} should be excluded"
            );
        }
        // Real pack content must not trip the rules.
        for path in ["mods/essential-mode.jar", "config/recordings.toml"] {
            assert!(
                !is_excluded_from_publish(path),
                "{path} should be publishable"
            );
        }
    }
}

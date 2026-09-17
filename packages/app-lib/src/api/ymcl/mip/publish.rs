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
    pub changed: Vec<PublishDiffEntry>,
    pub added: Vec<PublishDiffEntry>,
    pub deleted: Vec<String>,
    pub moved: Vec<PublishDiffEntry>,
    /// Player-side files excluded from the push (seed/locked/disabled),
    /// listed so the admin sees what was filtered out (YAP §7 rule 4).
    pub excluded: Vec<String>,
}

fn is_player_side(base: &MipPackState, path: &str) -> bool {
    base.locked_paths.iter().any(|locked| locked == path)
        || base.disabled_paths.iter().any(|disabled| disabled == path)
}

/// Computes what a push from the local instance would contain, relative to
/// the binding's base version. Pure over its inputs (the caller supplies
/// the local hash index), unit tested.
pub fn compute_publish_diff(
    base: &MipPackState,
    local_hashes: &HashMap<String, String>,
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

    // Changed and deleted: base paths compared against local content.
    for (path, base_file) in &base.files {
        if is_player_side(base, path) {
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
                let moved_elsewhere = local
                    .values()
                    .any(|hash| hash.as_str() == base_file.sha512);
                if !moved_elsewhere {
                    diff.deleted.push(path.clone());
                }
            }
        }
    }

    // Added and moved: local paths absent from base, paired against base
    // hashes for renames (local copies of moved files keep the base hash).
    for (path, local_hash) in &local {
        if base.files.contains_key(*path) || is_player_side(base, path) {
            continue;
        }
        if let Some((base_path, _)) = base
            .files
            .iter()
            .find(|(_, file)| &file.sha512 == *local_hash)
        {
            diff.moved.push(PublishDiffEntry {
                path: (*path).clone(),
                sha512: Some((*local_hash).clone()),
                from: Some(base_path.clone()),
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
    let local_hashes = super::update::scan_local_hashes(&instance_dir)?;
    let Some(pack_state) = state::load(&instance_dir).await? else {
        let (pack_files, excluded) = split_initial_files(&local_hashes);
        let mut diff = PublishDiff {
            managed: false,
            added: pack_files
                .into_iter()
                .map(|(path, sha512)| PublishDiffEntry {
                    path,
                    sha512: Some(sha512),
                    from: None,
                    size: None,
                })
                .collect(),
            excluded,
            ..PublishDiff::default()
        };
        fill_entry_sizes(&instance_dir, &mut diff).await;
        return Ok(diff);
    };
    let state = State::get().await?;
    let Some(server_base) = resolve_server_base(&state, &pack_state).await? else {
        // 服务端基线已消失（版本被删 / 绑定解除 / 包被删）：按首发处理。
        let (pack_files, excluded) = split_initial_files(&local_hashes);
        let mut diff = PublishDiff {
            managed: false,
            added: pack_files
                .into_iter()
                .map(|(path, sha512)| PublishDiffEntry {
                    path,
                    sha512: Some(sha512),
                    from: None,
                    size: None,
                })
                .collect(),
            excluded,
            ..PublishDiff::default()
        };
        fill_entry_sizes(&instance_dir, &mut diff).await;
        return Ok(diff);
    };
    let mut diff = compute_publish_diff(&server_base, &local_hashes);
    fill_entry_sizes(&instance_dir, &mut diff).await;
    Ok(diff)
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
    {
        if let Ok(metadata) = tokio::fs::metadata(instance_dir.join(&entry.path)).await {
            entry.size = Some(metadata.len());
        }
    }
}

/// Player-side directories that never enter an official pack (YAP §7 rule
/// 4 content sanitization, applied to unmanaged instances on initial
/// publish; managed instances already track these via locked/disabled).
const INITIAL_EXCLUDED_PREFIXES: &[&str] =
    &["saves/", "logs/", "crash-reports/", "screenshots/"];

/// Top-level entry names that never enter a published archive (player-side
/// content + protocol bookkeeping, YAP §7 rule 4).
const PUBLISH_EXCLUDED_TOP_LEVEL: &[&str] = &[
    "saves",
    "logs",
    "crash-reports",
    "screenshots",
    ".pack-staging",
    ".pack-backup",
];

/// Runtime log files are rewritten by the game and must never be pack-managed:
/// their content changes after install and breaks remote checksum verification
/// (e.g. `authlib-injector.log` at the instance root).
fn is_runtime_log_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("log"))
}

/// Paths that never enter a published pack, regardless of install location.
pub(crate) fn is_excluded_from_publish(path: &str) -> bool {
    if path == state::STATE_FILE_NAME
        || path.starts_with(super::apply::STAGING_DIR_NAME)
        || path.starts_with(super::apply::BACKUP_DIR_NAME)
        || INITIAL_EXCLUDED_PREFIXES
            .iter()
            .any(|prefix| path.starts_with(prefix))
        || PUBLISH_EXCLUDED_TOP_LEVEL.iter().any(|name| {
            path == *name || path.starts_with(&format!("{name}/"))
        })
        || is_runtime_log_path(path)
    {
        return true;
    }
    false
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

/// Admin-declared optional feature for the initial publish (mip.json
/// features, MIP §3.5): `files` are globs matched against the pack file set
/// by the adapter at ingest; `conflicts` are mutually exclusive feature ids.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct PublishFeatureInput {
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

/// Admin-declared file policy rule (mip.json policies, MIP §3.5): matching
/// files get `policy` (managed/seed/merge) instead of the default managed.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct PublishPolicyInput {
    pub glob: String,
    pub policy: String,
}

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
/// when omitted the launcher generates a unique `slug-<uuid>` id.
pub async fn push_initial(
    instance_id: &str,
    pack_id: Option<&str>,
    version: &str,
    channel: Option<&str>,
    bind: Option<serde_json::Value>,
    features: Vec<PublishFeatureInput>,
    policies: Vec<PublishPolicyInput>,
    exclude: Vec<String>,
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
    // the pack. Normalize separators so UI paths match the candidate list.
    let excluded: std::collections::HashSet<String> = exclude
        .into_iter()
        .map(|path| path.replace('\\', "/"))
        .collect();
    let candidates: Vec<String> = publish_candidates(instance_id)
        .await?
        .into_iter()
        .filter(|candidate| !excluded.contains(candidate))
        .collect();
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

    let upload = match build_initial_upload(&archive_path, version, channel, bind.as_ref())
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
    if let Some(bind) = bind {
        form = form.text("bind", bind.to_string());
    }
    Ok(form)
}

/// Adopts the publishing instance as MIP-managed from the adapter's ingest
/// report: state files come from the generated manifest (the source of
/// truth, covering CDN-referenced files), default-enabled features count as
/// selected (WF-4 default behavior for the author instance).
async fn adopt_state_from_report(
    instance_dir: &std::path::Path,
    pack_id: &str,
    version: &str,
    channel: Option<&str>,
    binding_server: Option<&str>,
    binding_season: Option<&str>,
    report: &serde_json::Value,
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
/// the standard immutable MIP manifest from base + delta (YAP §7).
pub async fn push_delta(
    instance_id: &str,
    version: &str,
    channel: Option<&str>,
    bind: Option<serde_json::Value>,
    features: Vec<PublishFeatureInput>,
    policies: Vec<PublishPolicyInput>,
) -> crate::Result<serde_json::Value> {
    let (instance_dir, pack_state, mip_base) =
        instance_context(instance_id).await?;
    // 基线与服务端同步：对绑定当前目标版本做增量；基线已被删则拒绝并
    // 引导走初始包重发（前端由 diff 的 managed:false 分流）。
    let state = State::get().await?;
    let Some(server_base) = resolve_server_base(&state, &pack_state).await? else {
        return Err(crate::ErrorKind::OtherError(
            "服务端基线已不存在（版本可能已被删除），请作为初始包重新发布"
                .to_string(),
        )
        .into());
    };
    let local_hashes = super::update::scan_local_hashes(&instance_dir)?;
    let diff = compute_publish_diff(&server_base, &local_hashes);
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
    form = form.text("delta", delta.to_string());

    let mut report = post_multipart(
        &mip_base,
        &format!("/api/packs/{}/ingest/delta", pack_state.pack_id),
        form,
    )
    .await?;
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

        let diff = compute_publish_diff(&base, &local);
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

        let diff = compute_publish_diff(&base, &local);
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
}

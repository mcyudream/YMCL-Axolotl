//! Publish pipeline (YAP §7, admin side in the launcher): compute the
//! diff between a local MIP instance and the domain binding's current
//! base version, and push it to the adapter as an initial mrpack or an
//! incremental delta upload (`/ingest` / `/ingest/delta`).

use serde::Serialize;
use sha2::{Digest, Sha512};
use std::collections::HashMap;

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

    // Local index minus protocol bookkeeping files.
    let local: HashMap<&String, &String> = local_hashes
        .iter()
        .filter(|(path, _)| {
            !path.starts_with(super::apply::STAGING_DIR_NAME)
                && !path.starts_with(super::apply::BACKUP_DIR_NAME)
                && path.as_str() != state::STATE_FILE_NAME
        })
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
            });
        } else {
            diff.added.push(PublishDiffEntry {
                path: (*path).clone(),
                sha512: Some((*local_hash).clone()),
                from: None,
            });
        }
    }

    diff
}

/// Computes the publish diff for an instance against the domain binding's
/// current base version (active domain required).
pub async fn diff_instance(instance_id: &str) -> crate::Result<PublishDiff> {
    let (instance_dir, pack_state, mip_base) =
        instance_context(instance_id).await?;
    let _ = mip_base;
    let local_hashes = super::update::scan_local_hashes(&instance_dir)?;
    Ok(compute_publish_diff(&pack_state, &local_hashes))
}

/// Pushes the diff as an incremental delta publish. The adapter composes
/// the standard immutable MIP manifest from base + delta (YAP §7).
pub async fn push_delta(
    instance_id: &str,
    version: &str,
    channel: Option<&str>,
    bind: Option<serde_json::Value>,
) -> crate::Result<serde_json::Value> {
    let (instance_dir, pack_state, mip_base) =
        instance_context(instance_id).await?;
    let local_hashes = super::update::scan_local_hashes(&instance_dir)?;
    let diff = compute_publish_diff(&pack_state, &local_hashes);
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

    // Adapter upload channel is base64-in-JSON: SPI request bodies are
    // Strings, so raw multipart bytes cannot survive transit (YAP §7 v1).
    use base64::Engine as _;
    let mut changed_payload = Vec::new();
    let mut added_payload = Vec::new();
    for (list, target) in [
        (&diff.changed, &mut changed_payload),
        (&diff.added, &mut added_payload),
    ] {
        for file in list {
            let bytes = tokio::fs::read(instance_dir.join(&file.path)).await?;
            target.push(serde_json::json!({
                "path": file.path,
                "data": base64::engine::general_purpose::STANDARD.encode(bytes),
            }));
        }
    }
    let mut body = serde_json::json!({
        "baseVersion": diff.base_version,
        "version": version,
        "changed": changed_payload,
        "added": added_payload,
        "deleted": diff.deleted,
        "moved": diff.moved,
    });
    if let Some(bind) = bind {
        body["bind"] = bind.clone();
    }
    if let Some(channel_value) = channel {
        body["channel"] = serde_json::Value::String(channel_value.to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()?;
    let url =
        format!("{mip_base}/api/packs/{}/ingest/delta", pack_state.pack_id);
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await?;
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

/// Resolves the triple (instance dir, pack state, MIP base URL) that every
/// publish operation needs.
async fn instance_context(
    instance_id: &str,
) -> crate::Result<(std::path::PathBuf, MipPackState, String)> {
    let state = State::get().await?;
    let instance_dir = crate::api::instance::get_full_path(instance_id).await?;
    let Some(pack_state) = state::load(&instance_dir).await? else {
        return Err(crate::ErrorKind::OtherError(
            "This instance is not a MIP-managed instance".to_string(),
        )
        .into());
    };
    let active = registry::active_domain_id(&state.pool).await?;
    if active == registry::PERSONAL_DOMAIN_ID {
        return Err(crate::ErrorKind::OtherError(
            "Publishing requires an active domain".to_string(),
        )
        .into());
    }
    let capabilities = registry::domain_capabilities(&active).await?;
    let mip_base = capabilities
        .mip
        .as_ref()
        .and_then(|mip| mip.base_url.clone())
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "The active domain has no MIP distribution face".to_string(),
            )
        })?;
    Ok((instance_dir, pack_state, mip_base))
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

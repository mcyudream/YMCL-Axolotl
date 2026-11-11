use crate::State;
use crate::state::instances::adapters::{filesystem, sqlite};
use crate::state::instances::{ContentSet, Instance, InstanceFile};
use crate::state::{
    CacheBehaviour, CachedEntry, CachedFileUpdate, ContentProvider,
    ContentProviderRef, DirectoryInfo, ModrinthVersionId, ProjectType,
    ReleaseChannel,
};
use crate::util::fetch::{self, FetchSemaphore};
use crate::util::io;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Resolves the directory whose game content (`mods`, `resourcepacks`, ...)
/// belongs to this instance.
///
/// Ordinary instances own their profile directory under YMCL's instances
/// folder, honouring a per-instance `game_dir_override`
/// (`DirectoryInfo::instance_game_dir`). Directly associated instances have no
/// profile directory: their content lives inside the externally managed
/// installation, resolved through the launcher dialect so PCL version
/// isolation (`versions/<id>` gameDir) is honored; the shared linked
/// `.minecraft` root is the fallback when the dialect resolution cannot be
/// completed (see `launcher::linked_game_dir`).
pub(crate) fn instance_content_root(
    directories: &DirectoryInfo,
    instance: &Instance,
) -> crate::Result<PathBuf> {
    if let Some(game_dir) =
        crate::launcher::linked_game_dir(instance).or_else(|| {
            instance
                .linked_dot_minecraft
                .as_deref()
                .map(str::trim)
                .filter(|linked| !linked.is_empty())
                .map(PathBuf::from)
        })
    {
        return Ok(io::canonicalize(game_dir)?);
    }

    Ok(io::canonicalize(directories.instance_game_dir(instance))?)
}

/// Joins a stored logical relative path using native path components. Content
/// rows intentionally use `/` as their cross-platform serialization format;
/// filesystem access must not concatenate that representation directly.
pub(crate) fn join_content_path(root: &Path, relative: &str) -> PathBuf {
    relative
        .split(['/', '\\'])
        .filter(|component| !component.is_empty() && *component != ".")
        .fold(root.to_path_buf(), |path, component| path.join(component))
}

pub(crate) async fn sync_content_files(
    instance_id: &str,
    state: &State,
) -> crate::Result<Vec<InstanceFile>> {
    let instance =
        sqlite::instance_rows::get_instance_by_id(instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError("Unknown instance".to_string())
            })?;

    sync_instance_content_files(&instance, state).await
}

pub(crate) async fn sync_instance_content_files(
    instance: &Instance,
    state: &State,
) -> crate::Result<Vec<InstanceFile>> {
    sync_instance_content_files_with_progress(instance, state, None).await
}

pub(crate) async fn sync_instance_content_files_with_progress(
    instance: &Instance,
    state: &State,
    progress: Option<&super::content_refresh::RefreshContext>,
) -> crate::Result<Vec<InstanceFile>> {
    // Keep the filesystem snapshot stable until its database rows commit.
    let _instance_lock = state.lock_instance_content(&instance.id).await;
    let content_root = instance_content_root(&state.directories, instance)?;
    // The hash-cache layer resolves key paths against YMCL's own instances
    // folder (`state/cache.rs`). For directly associated instances the
    // absolute linked root is passed as the "instance path": joining an
    // absolute path replaces the base, so the cache layer hashes the linked
    // files instead of failing on the nonexistent profile path. Managed
    // instances keep the relative profile path so the cache layer can resolve
    // a `game_dir_override` target from the database.
    let is_direct_linked = crate::launcher::linked_game_dir(instance).is_some()
        || instance
            .linked_dot_minecraft
            .as_deref()
            .map(str::trim)
            .is_some_and(|linked| !linked.is_empty());
    let cache_key_path = if is_direct_linked {
        content_root.to_string_lossy().into_owned()
    } else {
        instance.path.clone()
    };
    let scan_root = content_root.clone();
    let scanned = tokio::task::spawn_blocking(move || {
        filesystem::scan_content_files_from(&scan_root, &cache_key_path)
    })
    .await
    .map_err(|error| crate::ErrorKind::InputError(error.to_string()))??;
    let scanned_paths = scanned
        .iter()
        .map(|file| file.relative_path.clone())
        .collect::<HashSet<_>>();
    if let Some(progress) = progress {
        progress.begin_files(&scanned).await?;
    }
    let existing_files =
        sqlite::content_rows::get_instance_files(&instance.id, &state.pool)
            .await?;
    let mut existing_files_by_path = HashMap::new();
    let mut existing_files_by_sha1: HashMap<String, Vec<InstanceFile>> =
        HashMap::new();
    for file in existing_files {
        existing_files_by_sha1
            .entry(file.sha1.clone())
            .or_default()
            .push(file.clone());
        existing_files_by_path.insert(file.relative_path.clone(), file);
    }
    let content_set = sqlite::content_rows::get_applied_content_set(
        &instance.id,
        &state.pool,
    )
    .await?;
    let entry_file_ids = match content_set.as_ref() {
        Some(content_set) => sqlite::content_rows::get_content_entries(
            &content_set.id,
            &state.pool,
        )
        .await?
        .into_iter()
        .filter_map(|entry| entry.file_id)
        .collect(),
        None => HashSet::new(),
    };

    let now = Utc::now();
    let mut files: Vec<InstanceFile> = Vec::new();
    let mut synced_files = Vec::new();
    let mut reclaims: HashMap<String, String> = HashMap::new();
    let mut merges: HashMap<String, String> = HashMap::new();
    let mut claimed_reclaim_ids = HashSet::new();
    let mut externally_changed_file_ids = HashSet::new();

    for file in scanned {
        if let Some(progress) = progress {
            progress.check_cancelled()?;
            if !progress.includes(&file.relative_path) {
                continue;
            }
        }
        let existing_file = existing_files_by_path.get(&file.relative_path);
        let path = join_content_path(&content_root, &file.relative_path);
        let (scanned_sha1, scanned_size) = match fetch::sha1_file_async(&path)
            .await
        {
            Ok((size, sha1)) => (sha1, size),
            Err(error) => {
                if let Some(progress) = progress {
                    progress
                        .file_failed(&file.relative_path, &error.to_string())
                        .await?;
                    continue;
                }
                return Err(error);
            }
        };
        let reclaim_candidate = if existing_file.is_some() {
            None
        } else {
            reclaimable_existing_file(
                &scanned_sha1,
                &file.relative_path,
                &existing_files_by_sha1,
                &scanned_paths,
                &claimed_reclaim_ids,
            )
        };
        let merge_candidate = existing_file
            .filter(|file| !entry_file_ids.contains(&file.id))
            .and_then(|file| {
                mergeable_tracked_file(
                    &scanned_sha1,
                    &file.relative_path,
                    &file.id,
                    &existing_files_by_sha1,
                    &entry_file_ids,
                    &scanned_paths,
                    &claimed_reclaim_ids,
                )
            });
        let source_file = existing_file.or(reclaim_candidate);
        if let Some(existing_file) = existing_file
            && physical_file_identity_changed(
                existing_file,
                &scanned_sha1,
                scanned_size,
            )
        {
            externally_changed_file_ids.insert(existing_file.id.clone());
        }
        if let Some(candidate) = reclaim_candidate {
            claimed_reclaim_ids.insert(candidate.id.clone());
            reclaims.insert(
                file.relative_path.clone(),
                candidate.relative_path.clone(),
            );
        }
        if let Some(candidate) = merge_candidate {
            claimed_reclaim_ids.insert(candidate.id.clone());
            merges.insert(file.relative_path.clone(), candidate.id.clone());
        }

        files.push(InstanceFile {
            id: source_file
                .map(|file| file.id.clone())
                .unwrap_or_else(instance_file_id),
            instance_id: instance.id.clone(),
            relative_path: file.relative_path,
            file_name: file.file_name,
            enabled: file.enabled,
            sha1: scanned_sha1,
            size: scanned_size,
            missing: false,
            added_at: source_file.map(|file| file.added_at).unwrap_or(now),
            modified_at: now,
            local_mod_data: source_file
                .filter(|f| !externally_changed_file_ids.contains(&f.id))
                .and_then(|f| f.local_mod_data.clone()),
            icon_path: source_file
                .filter(|f| !externally_changed_file_ids.contains(&f.id))
                .and_then(|f| f.icon_path.clone()),
        });
        if progress.is_some() && files.len() >= 12 {
            enrich_local_metadata(&mut files, &content_root, state, progress)
                .await?;
            let batch = commit_scanned_files(
                instance,
                &files,
                &reclaims,
                &merges,
                &externally_changed_file_ids,
                content_set.as_ref(),
                state,
            )
            .await?;
            if let Some(progress) = progress {
                progress.local_ready(&batch).await?;
            }
            synced_files.extend(batch);
            files.clear();
        }
    }

    enrich_local_metadata(&mut files, &content_root, state, progress).await?;
    let batch = commit_scanned_files(
        instance,
        &files,
        &reclaims,
        &merges,
        &externally_changed_file_ids,
        content_set.as_ref(),
        state,
    )
    .await?;
    if let Some(progress) = progress {
        progress.local_ready(&batch).await?;
        progress.check_cancelled()?;
    }
    synced_files.extend(batch);
    if progress.is_none_or(|progress| !progress.is_retry()) {
        let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlite::content_rows::ensure_instance_exists(&instance.id, &mut tx)
            .await?;
        sqlx::query("UPDATE instance_files SET missing = 1 WHERE instance_id = ? AND relative_path NOT IN (SELECT value FROM json_each(?))")
            .bind(&instance.id).bind(serde_json::to_string(&scanned_paths)?).execute(&mut *tx).await?;
        if let Some(content_set) = content_set.as_ref() {
            sqlite::content_rows::bump_content_set_revision_in_transaction(
                &content_set.id,
                &mut tx,
            )
            .await?;
        }
        tx.commit().await?;
    }
    Ok(synced_files)
}

async fn enrich_local_metadata(
    files: &mut [InstanceFile],
    content_root: &Path,
    state: &State,
    progress: Option<&super::content_refresh::RefreshContext>,
) -> crate::Result<()> {
    let icon_cache_dir = state.directories.caches_dir().join("icons");
    for file in files {
        if let Some(progress) = progress {
            progress.check_cancelled()?;
        }
        let Some(project_type) = project_type_for_file(file) else {
            continue;
        };
        let extract_metadata = project_type == ProjectType::Mod
            && crate::mod_metadata::metadata_cache_needs_refresh(
                file.local_mod_data.as_deref(),
            );
        let extract_icon = file.icon_path.is_none()
            && matches!(
                project_type,
                ProjectType::Mod | ProjectType::ResourcePack
            );
        if !extract_metadata && !extract_icon {
            continue;
        }

        let path = join_content_path(content_root, &file.relative_path);

        // Resource packs are read entry-wise so large archives are not
        // materialized in memory just to fetch `pack.png`.
        if extract_icon && project_type == ProjectType::ResourcePack {
            let icon = tokio::task::spawn_blocking(move || {
                crate::mod_metadata::icon::extract_resource_pack_icon(&path)
            })
            .await
            .map_err(|error| crate::ErrorKind::InputError(error.to_string()))?;
            file.icon_path = Some(
                cache_extracted_icon(icon, &file.sha1, &icon_cache_dir, state)
                    .await,
            );
            continue;
        }

        // Mods: one in-memory read serves both metadata and icon extraction.
        let bytes = match tokio::fs::read(&path).await {
            Ok(data) => bytes::Bytes::from(data),
            Err(_) => {
                // File temporarily inaccessible; skip silently.
                continue;
            }
        };

        let cached = file.local_mod_data.clone();
        // ZIP decompression and class parsing are CPU/blocking work. Keep
        // them off Tokio's workers so progress and cancellation stay usable.
        let (metadata_json, icon) = tokio::task::spawn_blocking(move || {
            let metadata = if extract_metadata {
                crate::mod_metadata::extract_mod_metadata(&bytes)
            } else {
                cached.as_deref().and_then(|json| {
                    serde_json::from_str::<
                            crate::mod_metadata::LocalModMetadata,
                        >(json)
                        .ok()
                })
            };
            let metadata_json = extract_metadata
                .then(|| {
                    crate::mod_metadata::serialize_metadata_cache(
                        metadata.as_ref(),
                    )
                })
                .transpose()?;
            let icon = extract_icon
                .then(|| {
                    crate::mod_metadata::icon::extract_mod_icon(
                        &bytes,
                        metadata.as_ref(),
                    )
                })
                .flatten();
            Ok::<_, serde_json::Error>((metadata_json, icon))
        })
        .await
        .map_err(|error| crate::ErrorKind::InputError(error.to_string()))??;
        if let Some(json) = metadata_json {
            file.local_mod_data = Some(json);
        }
        if extract_icon {
            file.icon_path = Some(
                cache_extracted_icon(icon, &file.sha1, &icon_cache_dir, state)
                    .await,
            );
        }
    }
    Ok(())
}

async fn commit_scanned_files(
    instance: &Instance,
    files: &[InstanceFile],
    reclaims: &HashMap<String, String>,
    merges: &HashMap<String, String>,
    externally_changed_file_ids: &HashSet<String>,
    content_set: Option<&crate::state::instances::ContentSet>,
    state: &State,
) -> crate::Result<Vec<InstanceFile>> {
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlite::content_rows::ensure_instance_exists(&instance.id, &mut tx).await?;
    let mut _invalidated_provider_identity = false;
    if let Some(content_set) = content_set {
        for file_id in externally_changed_file_ids
            .iter()
            .filter(|id| files.iter().any(|file| &file.id == *id))
        {
            _invalidated_provider_identity |= sqlite::content_rows::invalidate_exact_provider_refs_for_file_in_transaction(
                &content_set.id,
                file_id,
                &mut tx,
            )
            .await?;
        }
    }

    // Upsert with a fresh id lookup inside the transaction. The ids assigned
    // during the scan may be stale if a concurrent operation (e.g. batch
    // disable renaming files to `.disabled`) moved a row after the snapshot;
    // reusing a stale id against the moved row would trip the UNIQUE
    // constraint on `instance_files.id` (code 1555).
    let mut synced_files: Vec<InstanceFile> = Vec::with_capacity(files.len());
    for file in files {
        let synced =
            if let Some(tracked_file_id) = merges.get(&file.relative_path) {
                sqlite::content_rows::adopt_untracked_file_in_transaction(
                    &instance.id,
                    &file.relative_path,
                    tracked_file_id,
                    &mut tx,
                )
                .await?;
                upsert_scanned_file(&instance.id, file, &mut tx).await?
            } else if let Some(old_relative_path) =
                reclaims.get(&file.relative_path)
            {
                match sqlite::content_rows::move_instance_file_in_transaction(
                    &instance.id,
                    old_relative_path,
                    &file.relative_path,
                    &file.file_name,
                    file.enabled,
                    &file.sha1,
                    file.size,
                    file.local_mod_data.as_deref(),
                    file.icon_path.as_deref(),
                    &mut tx,
                )
                .await?
                {
                    Some(file) => file,
                    None => {
                        upsert_scanned_file(&instance.id, file, &mut tx).await?
                    }
                }
            } else {
                upsert_scanned_file(&instance.id, file, &mut tx).await?
            };
        synced_files.push(synced);
    }

    if let Some(content_set) = content_set {
        sqlite::content_rows::bump_content_set_revision_in_transaction(
            &content_set.id,
            &mut tx,
        )
        .await?;
    }

    tx.commit().await?;

    Ok(synced_files)
}

async fn cache_extracted_icon(
    icon: Option<(String, Vec<u8>)>,
    sha1: &str,
    icon_cache_dir: &Path,
    state: &State,
) -> String {
    let Some((entry_name, icon_bytes)) = icon else {
        return String::new();
    };

    let extension = icon_extension(&entry_name);
    let cache_path = icon_cache_dir.join(format!("{sha1}.{extension}"));
    match fetch::write(&cache_path, &icon_bytes, &state.io_semaphore).await {
        Ok(()) => crate::util::io::canonicalize(&cache_path)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|_| cache_path.to_string_lossy().into_owned()),
        Err(_) => String::new(),
    }
}

fn icon_extension(entry_name: &str) -> &str {
    let extension = Path::new(entry_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if matches!(
        extension.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg"
    ) {
        extension
    } else {
        "png"
    }
}

pub(crate) fn project_type_for_file(
    file: &InstanceFile,
) -> Option<ProjectType> {
    filesystem::project_type_from_relative_path(&file.relative_path)
}

pub(crate) fn installed_modrinth_version_id(
    provider_refs: &[ContentProviderRef],
) -> Option<ModrinthVersionId> {
    provider_refs.iter().find_map(|reference| match reference {
        ContentProviderRef::Modrinth { version_id, .. } => {
            version_id.as_ref().cloned()
        }
        ContentProviderRef::CurseForge { .. }
        | ContentProviderRef::McArchive { .. } => None,
    })
}

pub(crate) fn content_update_cache_key(
    hash: &str,
    project_type: ProjectType,
    content_set: &ContentSet,
    channel: ReleaseChannel,
) -> String {
    let loaders = if project_type == ProjectType::Mod {
        content_set.loader.modrinth_loaders()
    } else {
        project_type.get_loaders()
    };

    format!(
        "{hash}-{}-{}-{}",
        loaders.join("+"),
        channel.key(),
        content_set.game_version
    )
}

pub(crate) async fn fetch_content_file_updates(
    update_key_refs: &[&str],
    cache_behaviour: Option<CacheBehaviour>,
    refresh: bool,
    pool: &sqlx::SqlitePool,
    fetch_semaphore: &FetchSemaphore,
) -> crate::Result<Vec<CachedFileUpdate>> {
    let update_behaviour = if refresh {
        Some(CacheBehaviour::Bypass)
    } else {
        cache_behaviour
    };

    CachedEntry::get_file_update_many(
        update_key_refs,
        update_behaviour,
        pool,
        fetch_semaphore,
    )
    .await
}

/// Whether a file may receive Modrinth update suggestions.
///
/// Files installed from Modrinth (origin `Modrinth`) always qualify. Untracked
/// or locally recorded files (no origin) qualify when their primary verified
/// identity is Modrinth; CurseForge-origin files are
/// handled by the CurseForge update path instead.
pub(crate) fn modrinth_update_enabled(
    origin_provider: Option<ContentProvider>,
    provider_refs: &[ContentProviderRef],
) -> bool {
    match origin_provider {
        Some(ContentProvider::Modrinth) => true,
        Some(ContentProvider::CurseForge)
        | Some(ContentProvider::McArchive)
        | Some(ContentProvider::Local) => false,
        None => ContentProviderRef::primary_reference(provider_refs, None)
            .is_none_or(|reference| {
                reference.provider() == ContentProvider::Modrinth
            }),
    }
}

fn instance_file_id() -> String {
    format!("instance-file:{}", Uuid::new_v4())
}

fn physical_file_identity_changed(
    existing: &InstanceFile,
    scanned_sha1: &str,
    scanned_size: u64,
) -> bool {
    existing.sha1 != scanned_sha1 || existing.size != scanned_size
}

async fn upsert_scanned_file(
    instance_id: &str,
    file: &InstanceFile,
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> crate::Result<InstanceFile> {
    sqlite::content_rows::upsert_instance_file_from_parts_in_transaction(
        sqlite::content_rows::UpsertInstanceFile {
            instance_id,
            relative_path: &file.relative_path,
            file_name: &file.file_name,
            enabled: file.enabled,
            sha1: &file.sha1,
            size: file.size,
            missing: false,
            local_mod_data: file.local_mod_data.as_deref(),
            icon_path: file.icon_path.as_deref(),
        },
        tx,
    )
    .await
}

/// Finds the single missing file row that can safely inherit a scanned file's
/// installed identity. A rename should keep its provider refs, entry, and
/// install history; ambiguous duplicates and cross-type moves stay untracked.
fn reclaimable_existing_file<'a>(
    sha1: &str,
    new_relative_path: &str,
    existing_files_by_sha1: &'a HashMap<String, Vec<InstanceFile>>,
    scanned_paths: &HashSet<String>,
    claimed_ids: &HashSet<String>,
) -> Option<&'a InstanceFile> {
    let candidates = existing_files_by_sha1.get(sha1)?;
    if candidates
        .iter()
        .any(|file| scanned_paths.contains(file.relative_path.as_str()))
    {
        return None;
    }
    let missing = candidates
        .iter()
        .filter(|file| {
            file.missing || !scanned_paths.contains(&file.relative_path)
        })
        .collect::<Vec<_>>();
    if missing.len() != 1 || claimed_ids.contains(&missing[0].id) {
        return None;
    }
    let candidate = missing[0];
    let new_type =
        filesystem::project_type_from_relative_path(new_relative_path)?;
    let old_type =
        filesystem::project_type_from_relative_path(&candidate.relative_path)?;
    (new_type == old_type).then_some(candidate)
}

/// Finds the single tracked row that should hand its installed identity to an
/// untracked row at a new path. This repairs instances that were already
/// broken by a rename before reclaim-on-move existed.
fn mergeable_tracked_file<'a>(
    sha1: &str,
    new_relative_path: &str,
    untracked_file_id: &str,
    existing_files_by_sha1: &'a HashMap<String, Vec<InstanceFile>>,
    entry_file_ids: &HashSet<String>,
    scanned_paths: &HashSet<String>,
    claimed_ids: &HashSet<String>,
) -> Option<&'a InstanceFile> {
    let candidates = existing_files_by_sha1.get(sha1)?;
    let tracked = candidates
        .iter()
        .filter(|file| entry_file_ids.contains(&file.id))
        .collect::<Vec<_>>();
    if tracked.is_empty()
        || tracked.iter().any(|file| {
            file.id != untracked_file_id
                && scanned_paths.contains(file.relative_path.as_str())
        })
    {
        return None;
    }
    let missing = tracked
        .iter()
        .filter(|file| {
            file.id != untracked_file_id
                && (file.missing
                    || !scanned_paths.contains(file.relative_path.as_str()))
        })
        .collect::<Vec<_>>();
    if missing.len() != 1 || claimed_ids.contains(&missing[0].id) {
        return None;
    }
    let candidate = missing[0];
    let new_type =
        filesystem::project_type_from_relative_path(new_relative_path)?;
    let old_type =
        filesystem::project_type_from_relative_path(&candidate.relative_path)?;
    (new_type == old_type).then_some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        CurseForgeFileId, CurseForgeProjectId, ModrinthProjectId,
        ModrinthVersionId,
    };
    use std::fs;
    #[cfg(not(feature = "tauri"))]
    use std::sync::Arc;

    #[test]
    fn cleanroom_content_update_keys_share_compatibility_filters() {
        use crate::state::ModLoader;
        use crate::state::instances::{ContentSetStatus, ContentSourceKind};

        let now = Utc::now();
        let mut content_set = ContentSet {
            id: "content-set".to_string(),
            instance_id: "instance".to_string(),
            name: "Cleanroom 0.6.8".to_string(),
            source_kind: ContentSourceKind::Local,
            status: ContentSetStatus::Available,
            game_version: "1.12.2".to_string(),
            protocol_version: None,
            loader: ModLoader::Cleanroom,
            loader_version: Some("0.6.8".to_string()),
            revision: 0,
            created: now,
            modified: now,
        };
        assert_eq!(
            content_update_cache_key(
                "sha1",
                ProjectType::Mod,
                &content_set,
                ReleaseChannel::Release
            ),
            "sha1-cleanroom+forge-release-1.12.2",
        );
        assert_eq!(
            content_update_cache_key(
                "sha1",
                ProjectType::ResourcePack,
                &content_set,
                ReleaseChannel::Beta
            ),
            "sha1-vanilla+canvas+minecraft-beta-1.12.2",
        );
        content_set.loader = ModLoader::Forge;
        assert_eq!(
            content_update_cache_key(
                "sha1",
                ProjectType::Mod,
                &content_set,
                ReleaseChannel::Release
            ),
            "sha1-forge-release-1.12.2",
        );
    }

    fn modrinth_ref() -> ContentProviderRef {
        ContentProviderRef::Modrinth {
            project_id: ModrinthProjectId::new("project").unwrap(),
            version_id: Some(ModrinthVersionId::new("version").unwrap()),
        }
    }

    fn curseforge_ref() -> ContentProviderRef {
        ContentProviderRef::CurseForge {
            project_id: CurseForgeProjectId::new(42).unwrap(),
            file_id: Some(CurseForgeFileId::new(7).unwrap()),
        }
    }

    #[test]
    fn untracked_files_qualify_for_modrinth_updates() {
        assert!(modrinth_update_enabled(None, &[]));
        assert!(modrinth_update_enabled(None, &[modrinth_ref()]));
    }

    #[test]
    fn curseforge_tracked_files_do_not_qualify_for_modrinth_updates() {
        assert!(!modrinth_update_enabled(
            Some(ContentProvider::CurseForge),
            &[curseforge_ref()],
        ));
        assert!(!modrinth_update_enabled(None, &[curseforge_ref()]));
    }

    #[test]
    fn modrinth_origin_always_qualifies() {
        assert!(modrinth_update_enabled(
            Some(ContentProvider::Modrinth),
            &[curseforge_ref(), modrinth_ref()],
        ));
    }

    #[test]
    fn content_scan_preserves_install_temporary_files() {
        let root = tempfile::tempdir().unwrap();
        let mods = root.path().join("instance/mods");
        fs::create_dir_all(&mods).unwrap();
        let temporary_names = [
            "example.jar.installing.download",
            "example.jar.installing",
            "example.jar.installing.previous",
        ];
        for name in temporary_names {
            fs::write(mods.join(name), name).unwrap();
        }

        let scanned =
            filesystem::scan_content_files(root.path(), "instance").unwrap();

        assert!(scanned.is_empty());
        for name in temporary_names {
            assert!(mods.join(name).is_file());
        }
    }

    // -----------------------------------------------------------------------
    // Directly associated instances scan their linked installation
    // -----------------------------------------------------------------------

    /// The launcher state is a process-wide singleton; initialize it once and
    /// reuse it so `State::get()` (used by the hash cache) resolves inside
    /// these APIs. The state root is intentionally leaked (`.keep()`) because
    /// the shared state outlives this function.
    #[cfg(not(feature = "tauri"))]
    async fn global_state() -> Arc<State> {
        if !State::initialized() {
            let root = tempfile::TempDir::new().unwrap().keep();
            let _ =
                State::init_for_test(root.to_string_lossy().to_string()).await;
        }
        State::get().await.unwrap()
    }

    #[cfg(not(feature = "tauri"))]
    fn write_self_contained_version(minecraft: &Path, version_id: &str) {
        let version_dir = minecraft.join("versions").join(version_id);
        fs::create_dir_all(&version_dir).unwrap();
        fs::write(
            version_dir.join(format!("{version_id}.json")),
            serde_json::to_vec(&serde_json::json!({
                "id": version_id,
                "mainClass": "net.minecraft.client.main.Main"
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn direct_link_refresh_scans_linked_dot_minecraft_content() {
        let state = global_state().await;
        let minecraft = tempfile::TempDir::new().unwrap();
        write_self_contained_version(minecraft.path(), "1.12.2-linked");
        fs::create_dir_all(minecraft.path().join("mods")).unwrap();
        fs::write(
            minecraft.path().join("mods/linked-mod.jar"),
            b"linked mod bytes",
        )
        .unwrap();

        let instance = crate::state::create_direct_link_instance(
            crate::state::CreateDirectLinkInstance {
                name: None,
                launcher_type:
                    crate::api::pack::import::ImportLauncherType::Generic,
                base_path: minecraft.path().to_path_buf(),
                instance_folder: "versions/1.12.2-linked".to_string(),
                instance_path: None,
                game_dir_mode: Some(
                    crate::launcher::ExternalGameDirMode::Shared,
                ),
            },
            &state,
        )
        .await
        .unwrap();

        let files = sync_instance_content_files(&instance, &state)
            .await
            .unwrap();

        assert!(
            files
                .iter()
                .any(|file| file.relative_path == "mods/linked-mod.jar"),
            "refresh must discover mods inside the linked `.minecraft`, got: {:?}",
            files
                .iter()
                .map(|file| &file.relative_path)
                .collect::<Vec<_>>(),
        );

        // A second refresh re-hashes the existing row. It must continue to
        // use the linked external root rather than the YMCL profile path.
        let refreshed = sync_instance_content_files(&instance, &state)
            .await
            .unwrap();
        assert!(
            refreshed
                .iter()
                .any(|file| file.relative_path == "mods/linked-mod.jar")
        );
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn pcl_isolated_direct_link_scans_version_isolated_content() {
        let state = global_state().await;
        let minecraft = tempfile::TempDir::new().unwrap();
        write_self_contained_version(minecraft.path(), "1.12.2-pcl");
        // Version isolation on: PCL resolves this version's gameDir to
        // versions/<id>, so its mods folder lives beside the version JSON.
        let version_dir = minecraft.path().join("versions/1.12.2-pcl");
        fs::create_dir_all(version_dir.join("PCL")).unwrap();
        fs::write(
            version_dir.join("PCL/Setup.ini"),
            "VersionArgumentIndieV2: true\n",
        )
        .unwrap();
        fs::create_dir_all(version_dir.join("mods")).unwrap();
        fs::write(
            version_dir.join("mods/isolated-mod.jar"),
            b"isolated mod bytes",
        )
        .unwrap();

        let instance = crate::state::create_direct_link_instance(
            crate::state::CreateDirectLinkInstance {
                name: None,
                launcher_type:
                    crate::api::pack::import::ImportLauncherType::PCL2,
                base_path: minecraft.path().to_path_buf(),
                instance_folder: "versions/1.12.2-pcl".to_string(),
                instance_path: None,
                game_dir_mode: None,
            },
            &state,
        )
        .await
        .unwrap();

        let files = sync_instance_content_files(&instance, &state)
            .await
            .unwrap();

        assert!(
            files
                .iter()
                .any(|file| file.relative_path == "mods/isolated-mod.jar"),
            "refresh must resolve the PCL-isolated gameDir, got: {:?}",
            files
                .iter()
                .map(|file| &file.relative_path)
                .collect::<Vec<_>>(),
        );
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn ordinary_instance_refresh_still_scans_its_profile_directory() {
        let state = global_state().await;
        let metadata = crate::api::instance::create(
            format!("sync-normal {}", uuid::Uuid::new_v4()),
            "1.20.1".to_string(),
            crate::state::ModLoader::Vanilla,
            None,
            None,
            crate::state::InstanceLink::Unmanaged,
            None,
            None,
        )
        .await
        .unwrap();

        let mods = state
            .directories
            .instances_dir()
            .join(&metadata.instance.path)
            .join("mods");
        fs::create_dir_all(&mods).unwrap();
        fs::write(mods.join("profile-mod.jar"), b"profile mod bytes").unwrap();

        let files = sync_instance_content_files(&metadata.instance, &state)
            .await
            .unwrap();

        assert!(
            files
                .iter()
                .any(|file| file.relative_path == "mods/profile-mod.jar"),
            "ordinary instances must keep scanning their profile directory, \
             got: {:?}",
            files
                .iter()
                .map(|file| &file.relative_path)
                .collect::<Vec<_>>(),
        );
    }

    /// Regression probe: an instance whose `game_dir_override` points to an
    /// external `.minecraft` root must have its content scanned from that root,
    /// not from the (empty) managed instance folder. This is the split-brain
    /// the content page empty-state used to hit.
    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn content_scan_uses_game_dir_override() {
        crate::event::EventState::init().await.unwrap();
        let root = tempfile::tempdir().unwrap().keep();
        let state =
            crate::State::init_for_test(root.to_string_lossy().to_string())
                .await
                .unwrap();

        // Create an external .minecraft root with a mod, outside the managed
        // profiles dir.
        let mc_root = tempfile::tempdir().unwrap();
        let mods_dir = mc_root.path().join("mods");
        fs::create_dir_all(&mods_dir).unwrap();
        fs::write(mods_dir.join("my-mod.jar"), "mod").unwrap();

        let created = crate::api::instance::create(
            "Override Instance".to_string(),
            "1.20.1".to_string(),
            crate::state::ModLoader::Vanilla,
            None,
            None,
            crate::state::InstanceLink::Unmanaged,
            None,
            Some(mc_root.path().to_string_lossy().to_string()),
        )
        .await
        .unwrap();
        crate::state::instances::commands::set_instance_install_stage(
            &created.instance.id,
            crate::state::InstanceInstallStage::Installed,
            &state.pool,
        )
        .await
        .unwrap();

        let instance =
            crate::state::instances::adapters::sqlite::instance_rows::get_instance_by_id(
                &created.instance.id,
                &state.pool,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            state.directories.instance_game_dir(&instance),
            mc_root.path(),
            "instance_game_dir must resolve to the override root"
        );
        let direct =
            filesystem::scan_content_files_from(mc_root.path(), &instance.path)
                .unwrap();
        assert_eq!(
            direct.len(),
            1,
            "direct scan of override root should find the mod"
        );
        let files = sync_instance_content_files(&instance, &state)
            .await
            .unwrap();

        assert_eq!(
            files.len(),
            1,
            "expected the override-root mod to be scanned"
        );
        assert!(files[0].relative_path.ends_with("mods/my-mod.jar"));
    }

    #[test]
    fn external_hash_or_size_change_invalidates_physical_identity() {
        let now = Utc::now();
        let file = InstanceFile {
            id: "file".to_string(),
            instance_id: "instance".to_string(),
            relative_path: "mods/lithium.jar".to_string(),
            file_name: "lithium.jar".to_string(),
            enabled: true,
            sha1: "official".to_string(),
            size: 10,
            missing: false,
            added_at: now,
            modified_at: now,
            local_mod_data: None,
            icon_path: None,
        };

        assert!(!physical_file_identity_changed(&file, "official", 10));
        assert!(physical_file_identity_changed(&file, "external", 10));
        assert!(physical_file_identity_changed(&file, "official", 11));
    }
}

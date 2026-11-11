use crate::state::instances::{
    ContentEntry, ContentSet, Instance, InstanceFile,
    adapters::sqlite::{content_rows, instance_rows},
};
use crate::state::{
    CacheBehaviour, CachedEntry, ContentProvider, ContentProviderRef,
    ModrinthProjectId, ModrinthVersionId, ProjectType, ReleaseChannel, State,
};
use std::collections::HashMap;

use super::curseforge_content_updates::{
    CurseForgeUpdateCandidate, persist_curseforge_update_checks,
    resolve_curseforge_content_updates,
};
use super::sync_content_files::{
    content_update_cache_key, fetch_content_file_updates,
    installed_modrinth_version_id, modrinth_update_enabled,
    project_type_for_file, sync_instance_content_files,
};

#[derive(Clone, Debug)]
pub(crate) enum ContentUpdate {
    Modrinth {
        relative_path: String,
        project_id: ModrinthProjectId,
        current_version_id: ModrinthVersionId,
        update_version_id: ModrinthVersionId,
    },
    CurseForge {
        relative_path: String,
    },
}

impl ContentUpdate {
    pub fn relative_path(&self) -> &str {
        match self {
            Self::Modrinth { relative_path, .. }
            | Self::CurseForge { relative_path, .. } => relative_path,
        }
    }

    pub fn modrinth_ids(
        &self,
    ) -> Option<(&ModrinthProjectId, &ModrinthVersionId, &ModrinthVersionId)>
    {
        match self {
            Self::Modrinth {
                project_id,
                current_version_id,
                update_version_id,
                ..
            } => Some((project_id, current_version_id, update_version_id)),
            Self::CurseForge { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
struct UpdateCandidate {
    entry: Option<ContentEntry>,
    file: InstanceFile,
    project_type: ProjectType,
    project_id: ModrinthProjectId,
    current_version_id: ModrinthVersionId,
}

pub(crate) async fn check_content_updates(
    instance_id: &str,
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> crate::Result<Vec<ContentUpdate>> {
    let context = load_installed_content(instance_id, state).await?;
    let candidates =
        modrinth_update_candidates(&context, cache_behaviour, state).await?;
    let mut output =
        resolve_modrinth_updates(&context, &candidates, cache_behaviour, state)
            .await?;
    output.extend(resolve_curseforge_updates(&context, state).await?);

    Ok(output)
}

/// Everything about an instance's currently applied content, loaded once and
/// shared across the update-check phases.
struct InstalledContentContext {
    instance: Instance,
    content_set: ContentSet,
    files: Vec<InstanceFile>,
    entries_by_file_id: HashMap<String, ContentEntry>,
    provider_refs_by_file_id: HashMap<String, Vec<ContentProviderRef>>,
    origin_provider_by_file_id: HashMap<String, Option<ContentProvider>>,
}

/// Phase 1 — load the instance's applied content set and file/provider state.
async fn load_installed_content(
    instance_id: &str,
    state: &State,
) -> crate::Result<InstalledContentContext> {
    let instance = instance_rows::get_instance_by_id(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
        })?;
    let files = sync_instance_content_files(&instance, state).await?;
    super::recognize_content_files::recognize_curseforge_content_files(
        &instance, &files, state,
    )
    .await?;
    let _instance_lock = state.lock_instance_content(instance_id).await;
    let content_set =
        content_rows::get_applied_content_set(&instance.id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Instance {} has no applied content set",
                    instance.id
                ))
            })?;
    let entries =
        content_rows::get_content_entries(&content_set.id, &state.pool).await?;
    let entries_by_file_id = entries
        .iter()
        .filter_map(|entry| {
            entry
                .file_id
                .as_deref()
                .map(|file_id| (file_id.to_string(), entry.clone()))
        })
        .collect();
    let mut provider_refs_by_file_id = HashMap::new();
    let mut origin_provider_by_file_id = HashMap::new();
    for entry in &entries {
        let Some(file_id) = entry.file_id.as_deref() else {
            continue;
        };
        provider_refs_by_file_id.insert(
            file_id.to_string(),
            content_rows::get_content_provider_refs(&entry.id, &state.pool)
                .await?,
        );
        origin_provider_by_file_id.insert(
            file_id.to_string(),
            content_rows::get_content_origin_provider(&entry.id, &state.pool)
                .await?,
        );
    }

    Ok(InstalledContentContext {
        instance,
        content_set,
        files,
        entries_by_file_id,
        provider_refs_by_file_id,
        origin_provider_by_file_id,
    })
}

/// Phase 2 — resolve the Modrinth version of every update-enabled file and
/// build an `UpdateCandidate` for each one that maps to a known project.
async fn modrinth_update_candidates(
    context: &InstalledContentContext,
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> crate::Result<Vec<UpdateCandidate>> {
    let hashes = context
        .files
        .iter()
        .filter(|file| {
            modrinth_update_enabled(
                context
                    .origin_provider_by_file_id
                    .get(&file.id)
                    .and_then(|provider| *provider),
                context
                    .provider_refs_by_file_id
                    .get(&file.id)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            )
        })
        .map(|file| file.sha1.as_str())
        .collect::<Vec<_>>();
    let file_info = CachedEntry::get_file_many(
        &hashes,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let file_info_by_hash = file_info
        .into_iter()
        .map(|file| (file.hash.clone(), file))
        .collect::<HashMap<_, _>>();
    let mut candidates = Vec::new();
    for file in &context.files {
        if !modrinth_update_enabled(
            context
                .origin_provider_by_file_id
                .get(&file.id)
                .and_then(|provider| *provider),
            context
                .provider_refs_by_file_id
                .get(&file.id)
                .map(Vec::as_slice)
                .unwrap_or_default(),
        ) {
            continue;
        }
        let Some(metadata) = file_info_by_hash.get(&file.sha1) else {
            continue;
        };
        let Some(project_type) = project_type_for_file(file) else {
            continue;
        };
        let provider_refs = context
            .provider_refs_by_file_id
            .get(&file.id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let project_id = ModrinthProjectId::new(metadata.project_id.clone())?;
        let current_version_id = installed_modrinth_version_id(provider_refs)
            .unwrap_or(ModrinthVersionId::new(metadata.version_id.clone())?);
        candidates.push(UpdateCandidate {
            entry: context.entries_by_file_id.get(&file.id).cloned(),
            file: file.clone(),
            project_type,
            project_id,
            current_version_id,
        });
    }

    Ok(candidates)
}

/// Phase 3 — query available Modrinth updates for the candidates, persist the
/// check result, and collect the differing versions.
async fn resolve_modrinth_updates(
    context: &InstalledContentContext,
    candidates: &[UpdateCandidate],
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> crate::Result<Vec<ContentUpdate>> {
    let mut output = Vec::new();
    if candidates.is_empty() {
        return Ok(output);
    }

    let installed_channels =
        installed_update_channels(candidates, cache_behaviour, state).await?;
    let update_keys = candidates
        .iter()
        .map(|candidate| {
            content_update_cache_key(
                &candidate.file.sha1,
                candidate.project_type,
                &context.content_set,
                effective_update_channel(
                    context.instance.update_channel,
                    installed_channels.get(&candidate.file.sha1).copied(),
                ),
            )
        })
        .collect::<Vec<_>>();
    let update_key_refs = update_keys
        .iter()
        .map(|key| key.as_str())
        .collect::<Vec<_>>();
    let updates = fetch_content_file_updates(
        &update_key_refs,
        cache_behaviour,
        true,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let mut updates_by_hash: HashMap<String, Vec<String>> = HashMap::new();
    for update in updates {
        updates_by_hash
            .entry(update.hash)
            .or_default()
            .push(update.update_version_id);
    }

    let _instance_lock =
        state.lock_instance_content(&context.instance.id).await;

    for candidate in candidates {
        let update_version_id = updates_by_hash
            .remove(&candidate.file.sha1)
            .unwrap_or_default()
            .into_iter()
            .find(|update_version_id| {
                update_version_id != candidate.current_version_id.as_str()
            })
            .map(ModrinthVersionId::new)
            .transpose()?;

        if let Some(entry) = &candidate.entry {
            content_rows::upsert_content_update_check(
                &entry.id,
                context.instance.update_channel,
                Some(ContentProvider::Modrinth),
                Some(candidate.project_id.as_str()),
                update_version_id.as_ref().map(ModrinthVersionId::as_str),
                &state.pool,
            )
            .await?;
        }

        if let Some(update_version_id) = update_version_id {
            output.push(ContentUpdate::Modrinth {
                relative_path: candidate.file.relative_path.clone(),
                project_id: candidate.project_id.clone(),
                current_version_id: candidate.current_version_id.clone(),
                update_version_id,
            });
        }
    }

    Ok(output)
}

/// Use the same complete-file selection as the content page.
async fn resolve_curseforge_updates(
    context: &InstalledContentContext,
    state: &State,
) -> crate::Result<Vec<ContentUpdate>> {
    let candidates = context
        .files
        .iter()
        .filter_map(|file| {
            CurseForgeUpdateCandidate::from_references(
                &file.relative_path,
                project_type_for_file(file)?,
                context
                    .origin_provider_by_file_id
                    .get(&file.id)
                    .copied()
                    .flatten(),
                context
                    .provider_refs_by_file_id
                    .get(&file.id)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    let updates = resolve_curseforge_content_updates(
        &context.instance,
        &context.content_set,
        &candidates,
    )
    .await?;
    persist_curseforge_update_checks(
        &context.instance,
        &context.content_set,
        &candidates,
        &updates,
        state,
    )
    .await?;
    let output = candidates
        .into_iter()
        .filter(|candidate| {
            updates
                .get(&candidate.relative_path)
                .is_some_and(|update| update.target.is_some())
        })
        .map(|candidate| ContentUpdate::CurseForge {
            relative_path: candidate.relative_path,
        })
        .collect();
    Ok(output)
}

async fn installed_update_channels(
    candidates: &[UpdateCandidate],
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> crate::Result<HashMap<String, ReleaseChannel>> {
    let version_ids = candidates
        .iter()
        .filter_map(|candidate| Some(candidate.current_version_id.clone()))
        .collect::<Vec<_>>();
    let versions = CachedEntry::get_version_many(
        &version_ids,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let channels_by_version_id = versions
        .into_iter()
        .map(|version| {
            (
                version.id,
                ReleaseChannel::from_version_type(&version.version_type),
            )
        })
        .collect::<HashMap<_, _>>();

    Ok(candidates
        .iter()
        .filter_map(|candidate| {
            channels_by_version_id
                .get(candidate.current_version_id.as_str())
                .copied()
                .map(|channel| (candidate.file.sha1.clone(), channel))
        })
        .collect())
}

fn effective_update_channel(
    preferred: ReleaseChannel,
    installed: Option<ReleaseChannel>,
) -> ReleaseChannel {
    installed.map_or(preferred, |channel| preferred.least_stable(channel))
}

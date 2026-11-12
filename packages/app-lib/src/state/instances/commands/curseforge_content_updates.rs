use std::collections::{HashMap, HashSet};
use std::future::Future;

use chrono::{DateTime, FixedOffset};
use futures::{StreamExt, stream};
use sqlx::Row;

use crate::api::curseforge::{
    self, CurseForgeCapabilityStatus, CurseForgeFile, CurseForgeFilesRequest,
    CurseForgeFilesResponse,
};
use crate::state::instances::{
    ContentSet, Instance, InstanceFile, adapters::sqlite::content_rows,
};
use crate::state::{
    ContentItemUpdate, ContentProvider, ContentProviderRef, CurseForgeFileId,
    CurseForgeProjectId, ProjectType, ReleaseChannel, State,
};

const UPDATE_LOOKUP_CONCURRENCY: usize = 4;

enum FileLookup {
    Current(Vec<u32>),
    Project(u32),
}

async fn fetch_update_files<F, Fut, G, Gut>(
    file_ids: HashSet<u32>,
    projects: HashSet<u32>,
    request: CurseForgeFilesRequest,
    current_lookup: F,
    project_lookup: G,
) -> crate::Result<(
    HashMap<u32, CurseForgeFile>,
    HashMap<u32, Vec<CurseForgeFile>>,
)>
where
    F: Fn(Vec<u32>) -> Fut + Sync,
    Fut: Future<Output = crate::Result<Vec<CurseForgeFile>>> + Send,
    G: Fn(u32, CurseForgeFilesRequest) -> Gut + Sync,
    Gut: Future<Output = crate::Result<CurseForgeFilesResponse>> + Send,
{
    let file_ids = file_ids.into_iter().collect::<Vec<_>>();
    let lookups = file_ids
        .chunks(50)
        .map(|batch| FileLookup::Current(batch.to_vec()))
        .chain(projects.into_iter().map(FileLookup::Project));
    let lookup_jobs = lookups
        .map(|lookup| {
            let current_lookup = &current_lookup;
            let project_lookup = &project_lookup;
            let request = request.clone();
            async move {
                match lookup {
                    FileLookup::Current(ids) => {
                        current_lookup(ids).await.map(|files| (None, files))
                    }
                    FileLookup::Project(id) => project_lookup(id, request)
                        .await
                        .map(|response| (Some(id), response.files)),
                }
            }
        })
        .collect::<Vec<_>>();
    let responses =
        stream::iter(lookup_jobs).buffer_unordered(UPDATE_LOOKUP_CONCURRENCY);
    futures::pin_mut!(responses);
    let mut current_files = HashMap::new();
    let mut files_by_project = HashMap::new();
    while let Some(response) = responses.next().await {
        let (project_id, files) = response?;
        if let Some(project_id) = project_id {
            files_by_project.insert(project_id, files);
        } else {
            current_files.extend(files.into_iter().map(|file| (file.id, file)));
        }
    }
    Ok((current_files, files_by_project))
}

#[derive(Clone, Debug)]
pub(super) struct CurseForgeUpdateCandidate {
    pub relative_path: String,
    pub project_type: ProjectType,
    pub project_id: CurseForgeProjectId,
    pub current_file_id: CurseForgeFileId,
}

impl CurseForgeUpdateCandidate {
    pub fn from_references(
        relative_path: &str,
        project_type: ProjectType,
        origin: Option<ContentProvider>,
        references: &[ContentProviderRef],
    ) -> Option<Self> {
        if origin.is_some_and(|origin| origin != ContentProvider::CurseForge) {
            return None;
        }
        match ContentProviderRef::primary_reference(references, origin)? {
            ContentProviderRef::CurseForge {
                project_id,
                file_id: Some(file_id),
            } => Some(Self {
                relative_path: relative_path.to_string(),
                project_type,
                project_id: *project_id,
                current_file_id: *file_id,
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct CurseForgeContentUpdate {
    pub current: CurseForgeFile,
    pub target: Option<CurseForgeFile>,
}

pub(super) async fn resolve_curseforge_content_updates(
    instance: &Instance,
    content_set: &ContentSet,
    candidates: &[CurseForgeUpdateCandidate],
) -> crate::Result<HashMap<String, CurseForgeContentUpdate>> {
    if candidates.is_empty()
        || curseforge::capability().status != CurseForgeCapabilityStatus::Ready
    {
        return Ok(HashMap::new());
    }
    let file_ids = candidates
        .iter()
        .map(|candidate| candidate.current_file_id.get())
        .collect::<HashSet<_>>();
    let projects = candidates
        .iter()
        .map(|candidate| candidate.project_id.get())
        .collect::<HashSet<_>>();
    // Fetch all compatible-game pages rather than a project's small latest-files summary.
    // Loader compatibility is checked locally, including responses from mirrors.
    let (current_files, files_by_project) = fetch_update_files(
        file_ids,
        projects,
        CurseForgeFilesRequest {
            game_version: Some(content_set.game_version.clone()),
            mod_loader_type: None,
            game_version_type_id: None,
            index: 0,
            page_size: 50,
        },
        curseforge::get_files_many,
        curseforge::get_files,
    )
    .await?;
    let mut output = HashMap::new();
    for candidate in candidates {
        let Some(current) = current_files
            .get(&candidate.current_file_id.get())
            .filter(|file| file.mod_id == candidate.project_id.get())
        else {
            continue;
        };
        let files = &files_by_project[&candidate.project_id.get()];
        let loader_type = (candidate.project_type == ProjectType::Mod)
            .then(|| content_set.loader.curseforge_loader_type())
            .flatten();
        let target = select_update(
            files,
            current,
            &content_set.game_version,
            loader_type,
            instance.update_channel,
        )
        .cloned();
        output.insert(
            candidate.relative_path.clone(),
            CurseForgeContentUpdate {
                current: current.clone(),
                target,
            },
        );
    }
    Ok(output)
}

pub(super) async fn refresh_curseforge_content_updates(
    instance: &Instance,
    files: &[InstanceFile],
    state: &State,
) -> crate::Result<()> {
    let Some(content_set) =
        content_rows::get_applied_content_set(&instance.id, &state.pool)
            .await?
    else {
        return Ok(());
    };
    let rows = sqlx::query(
        "SELECT file.relative_path, ref.provider_project_id, ref.provider_release_id
         FROM instance_content_entries entry
         JOIN instance_files file ON file.id = entry.file_id
         JOIN instance_content_provider_refs ref ON ref.content_entry_id = entry.id
         LEFT JOIN instance_content_provider_refs origin ON origin.content_entry_id = entry.id AND origin.is_origin = 1
         WHERE entry.content_set_id = ? AND ref.provider = 'curseforge'
           AND ref.provider_release_id IS NOT NULL AND file.missing = 0
           AND (origin.provider IS NULL OR (origin.provider = 'curseforge' AND origin.provider_project_id = ref.provider_project_id))
         ORDER BY ref.is_origin DESC, ref.provider_project_id, ref.provider_release_id"
    ).bind(&content_set.id).fetch_all(&state.pool).await?;
    let mut candidates_by_path = HashMap::new();
    let files_by_path = files
        .iter()
        .map(|file| (file.relative_path.as_str(), file))
        .collect::<HashMap<_, _>>();
    for row in rows {
        let path: String = row.try_get("relative_path")?;
        let Some(file) = files_by_path.get(path.as_str()) else {
            continue;
        };
        let Some(project_type) =
            super::sync_content_files::project_type_for_file(file)
        else {
            continue;
        };
        let reference = ContentProviderRef::from_database(
            "curseforge",
            row.try_get("provider_project_id")?,
            row.try_get::<Option<String>, _>("provider_release_id")?
                .as_deref(),
            None,
        )?;
        if let Some(candidate) = CurseForgeUpdateCandidate::from_references(
            &path,
            project_type,
            None,
            &[reference],
        ) {
            candidates_by_path.entry(path).or_insert(candidate);
        }
    }
    let candidates = candidates_by_path.into_values().collect::<Vec<_>>();
    let updates =
        resolve_curseforge_content_updates(instance, &content_set, &candidates)
            .await?;
    persist_curseforge_update_checks(
        instance,
        &content_set,
        &candidates,
        &updates,
        state,
    )
    .await
}

pub(super) async fn persist_curseforge_update_checks(
    instance: &Instance,
    content_set: &ContentSet,
    candidates: &[CurseForgeUpdateCandidate],
    updates: &HashMap<String, CurseForgeContentUpdate>,
    state: &State,
) -> crate::Result<()> {
    let _lock = state.lock_instance_content(&instance.id).await;
    for candidate in candidates {
        let Some(resolution) = updates.get(&candidate.relative_path) else {
            continue;
        };
        // Validate the source and target configuration in the same statement
        // that saves the response, so an in-flight check cannot revive stale data.
        sqlx::query(
            "INSERT INTO instance_content_update_checks (
                content_entry_id, update_channel, provider,
                provider_project_id, provider_release_id, checked_at
             )
             SELECT entry.id, ?, 'curseforge', ?, ?, ? FROM instance_content_entries entry
             JOIN instance_files file ON file.id = entry.file_id
             JOIN instance_content_provider_refs ref ON ref.content_entry_id = entry.id
             JOIN instance_content_sets content_set ON content_set.id = entry.content_set_id
             JOIN instances instance ON instance.applied_content_set_id = entry.content_set_id
             WHERE instance.id = ? AND file.relative_path = ? AND file.missing = 0
               AND ref.provider = 'curseforge' AND ref.provider_project_id = ?
               AND ref.provider_release_id = ?
               AND content_set.id = ? AND content_set.game_version = ? AND content_set.loader = ?
               AND instance.update_channel = ?
               AND NOT EXISTS (SELECT 1 FROM instance_content_provider_refs origin WHERE origin.content_entry_id = entry.id AND origin.is_origin = 1 AND (origin.provider != 'curseforge' OR origin.provider_project_id != ref.provider_project_id))
             LIMIT 1
             ON CONFLICT(content_entry_id) DO UPDATE SET
                update_channel = excluded.update_channel,
                provider = excluded.provider,
                provider_project_id = excluded.provider_project_id,
                provider_release_id = excluded.provider_release_id,
                checked_at = excluded.checked_at"
        ).bind(instance.update_channel.key())
            .bind(candidate.project_id.get().to_string())
            .bind(resolution.target.as_ref().map(|file| file.id.to_string()))
            .bind(chrono::Utc::now().timestamp())
            .bind(&instance.id).bind(&candidate.relative_path)
            .bind(candidate.project_id.get().to_string()).bind(candidate.current_file_id.get().to_string())
            .bind(&content_set.id).bind(&content_set.game_version).bind(content_set.loader.as_str())
            .bind(instance.update_channel.key())
            .execute(&state.pool).await?;
    }
    Ok(())
}

pub(super) async fn cached_curseforge_updates(
    instance: &Instance,
    content_set: &ContentSet,
    candidates: &[CurseForgeUpdateCandidate],
    state: &State,
) -> crate::Result<HashMap<String, ContentItemUpdate>> {
    let mut output = HashMap::new();
    for candidate in candidates {
        let target = sqlx::query_scalar::<_, String>(
            "SELECT checked.provider_release_id FROM instance_content_update_checks checked
             JOIN instance_content_entries entry ON entry.id = checked.content_entry_id
             JOIN instance_files file ON file.id = entry.file_id
             JOIN instance_content_provider_refs ref ON ref.content_entry_id = entry.id
             JOIN instance_content_sets content_set ON content_set.id = entry.content_set_id
             JOIN instances instance ON instance.applied_content_set_id = entry.content_set_id
             WHERE entry.content_set_id = ? AND file.relative_path = ? AND file.missing = 0
               AND checked.provider = 'curseforge' AND checked.provider_project_id = ?
               AND checked.update_channel = ?
               AND ref.provider = 'curseforge' AND ref.provider_project_id = checked.provider_project_id AND ref.provider_release_id = ?
               AND content_set.game_version = ? AND content_set.loader = ? AND instance.update_channel = checked.update_channel
               AND checked.provider_release_id IS NOT NULL"
        ).bind(&content_set.id).bind(&candidate.relative_path).bind(candidate.project_id.get().to_string())
            .bind(instance.update_channel.key()).bind(candidate.current_file_id.get().to_string())
            .bind(&content_set.game_version).bind(content_set.loader.as_str())
            .fetch_optional(&state.pool).await?;
        if let Some(target) = target
            .and_then(|id| id.parse::<u32>().ok())
            .filter(|id| *id != candidate.current_file_id.get())
        {
            output.insert(
                candidate.relative_path.clone(),
                ContentItemUpdate::CurseForge {
                    project_id: candidate.project_id,
                    current_file_id: candidate.current_file_id,
                    target_file_id: CurseForgeFileId::new(target)?,
                },
            );
        }
    }
    Ok(output)
}

pub(crate) fn select_update<'a>(
    files: &'a [CurseForgeFile],
    current: &CurseForgeFile,
    game_version: &str,
    loader_type: Option<u32>,
    channel: ReleaseChannel,
) -> Option<&'a CurseForgeFile> {
    let current_date = published(current)?;
    files
        .iter()
        .filter(|file| {
            file.mod_id == current.mod_id
                && file.id != current.id
                && file.is_available
                && match channel {
                    ReleaseChannel::Release => file.release_type == 1,
                    ReleaseChannel::Beta => matches!(file.release_type, 1 | 2),
                    ReleaseChannel::Alpha => matches!(file.release_type, 1..=3),
                }
                && curseforge::file_matches_dependency_target(
                    file,
                    Some(game_version),
                    loader_type,
                )
                && published(file).is_some_and(|date| date > current_date)
        })
        .max_by_key(|file| (published(file), file.id))
}

fn published(file: &CurseForgeFile) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(&file.file_date).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ModLoader;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    fn files_request() -> CurseForgeFilesRequest {
        CurseForgeFilesRequest {
            game_version: Some("1.12.2".into()),
            mod_loader_type: None,
            game_version_type_id: None,
            index: 0,
            page_size: 50,
        }
    }

    #[tokio::test]
    async fn parallel_update_lookups_share_a_limit_and_preserve_all_results() {
        let active = AtomicUsize::new(0);
        let current_active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let batches = AtomicUsize::new(0);
        let project_calls = AtomicUsize::new(0);
        let overlapping = AtomicBool::new(false);
        let release_slow_batch = tokio::sync::Notify::new();
        let future = fetch_update_files(
            (1..=151).collect(),
            (201..=207).collect(),
            files_request(),
            |ids| {
                assert!(!ids.is_empty() && ids.len() <= 50);
                let slow = batches.fetch_add(1, Ordering::SeqCst) == 0;
                let active = &active;
                let current_active = &current_active;
                let peak = &peak;
                let release_slow_batch = &release_slow_batch;
                async move {
                    let running = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(running, Ordering::SeqCst);
                    current_active.fetch_add(1, Ordering::SeqCst);
                    if slow {
                        release_slow_batch.notified().await;
                    } else {
                        tokio::task::yield_now().await;
                    }
                    current_active.fetch_sub(1, Ordering::SeqCst);
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(ids
                        .into_iter()
                        .map(|id| {
                            file(
                                id,
                                "2026-01-01T00:00:00Z",
                                "1.12.2",
                                "Forge",
                                1,
                            )
                        })
                        .collect())
                }
            },
            |id, request| {
                assert_eq!(request.game_version.as_deref(), Some("1.12.2"));
                assert_eq!(request.index, 0);
                assert_eq!(request.page_size, 50);
                assert!(request.mod_loader_type.is_none());
                let active = &active;
                let current_active = &current_active;
                let peak = &peak;
                let overlapping = &overlapping;
                let release_slow_batch = &release_slow_batch;
                project_calls.fetch_add(1, Ordering::SeqCst);
                async move {
                    if current_active.load(Ordering::SeqCst) > 0 {
                        overlapping.store(true, Ordering::SeqCst);
                    }
                    release_slow_batch.notify_one();
                    let running = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(running, Ordering::SeqCst);
                    tokio::task::yield_now().await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    let mut candidate = file(
                        id + 1000,
                        "2026-02-01T00:00:00Z",
                        "1.12.2",
                        "Forge",
                        1,
                    );
                    candidate.mod_id = id;
                    Ok(CurseForgeFilesResponse {
                        files: vec![candidate],
                        pagination: curseforge::CurseForgePagination {
                            index: 0,
                            page_size: 50,
                            result_count: 1,
                            total_count: 1,
                        },
                    })
                }
            },
        );
        let (current, projects) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            future,
        )
        .await
        .expect("project lookups must not wait for every current-file batch")
        .unwrap();
        assert_eq!(peak.load(Ordering::SeqCst), UPDATE_LOOKUP_CONCURRENCY);
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert_eq!(batches.load(Ordering::SeqCst), 4);
        assert_eq!(project_calls.load(Ordering::SeqCst), 7);
        assert!(overlapping.load(Ordering::SeqCst));
        assert_eq!(current.len(), 151);
        for id in 1..=151 {
            assert_eq!(current[&id].id, id);
        }
        assert_eq!(projects.len(), 7);
        for id in 201..=207 {
            assert_eq!(projects[&id][0].id, id + 1000);
            assert_eq!(projects[&id][0].mod_id, id);
        }
    }

    #[tokio::test]
    async fn parallel_update_lookups_do_not_return_partial_results_on_failure()
    {
        let error = fetch_update_files(
            (1..=51).collect(),
            (201..=207).collect(),
            files_request(),
            |_| async {
                Err(crate::ErrorKind::OtherError(
                    "fixture update batch failure".into(),
                )
                .into())
            },
            |_, _| async {
                Ok(CurseForgeFilesResponse {
                    files: Vec::new(),
                    pagination: curseforge::CurseForgePagination {
                        index: 0,
                        page_size: 50,
                        result_count: 0,
                        total_count: 0,
                    },
                })
            },
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("fixture update batch failure"));
    }

    fn file(
        id: u32,
        date: &str,
        game: &str,
        loader: &str,
        release: u32,
    ) -> CurseForgeFile {
        serde_json::from_value(serde_json::json!({
            "id": id, "modId": 123, "gameId": 432, "isAvailable": true,
            "displayName": "test", "fileName": "test.jar", "releaseType": release,
            "fileStatus": 4, "fileDate": date, "fileLength": 1, "downloadCount": 0,
            "fileFingerprint": 1, "gameVersions": [game, loader]
        })).unwrap()
    }

    #[test]
    fn recognized_aliases_allow_updates_without_changing_an_explicit_origin() {
        let reference = ContentProviderRef::CurseForge {
            project_id: CurseForgeProjectId::new(123).unwrap(),
            file_id: Some(CurseForgeFileId::new(456).unwrap()),
        };
        for (origin, allowed) in [
            (None, true),
            (Some(ContentProvider::CurseForge), true),
            (Some(ContentProvider::Modrinth), false),
            (Some(ContentProvider::McArchive), false),
            (Some(ContentProvider::Local), false),
        ] {
            assert_eq!(
                CurseForgeUpdateCandidate::from_references(
                    "mods/test.jar",
                    ProjectType::Mod,
                    origin,
                    std::slice::from_ref(&reference)
                )
                .is_some(),
                allowed
            );
        }

        let other_project = ContentProviderRef::CurseForge {
            project_id: CurseForgeProjectId::new(999).unwrap(),
            file_id: None,
        };
        assert!(
            CurseForgeUpdateCandidate::from_references(
                "mods/test.jar",
                ProjectType::Mod,
                Some(ContentProvider::CurseForge),
                &[other_project, reference],
            )
            .is_none(),
            "an alias must not change the origin project"
        );
    }

    #[test]
    fn complete_file_selection_finds_legacy_cleanroom_updates_and_filters_incompatible_files()
     {
        let current = file(456, "2026-01-01T00:00:00Z", "1.12.2", "Forge", 1);
        let mut unavailable =
            file(50, "2026-11-01T00:00:00Z", "1.12.2", "Forge", 1);
        unavailable.is_available = false;
        let mut other_project =
            file(51, "2026-11-02T00:00:00Z", "1.12.2", "Forge", 1);
        other_project.mod_id = 999;
        let files = vec![
            file(9999, "2025-01-01T00:00:00Z", "1.12.2", "Forge", 1),
            file(10, "2026-02-01T00:00:00Z", "1.12.2", "Forge", 1),
            file(11, "2026-03-01T00:00:00Z", "1.12.2", "Forge", 2),
            file(12, "2026-04-01T00:00:00Z", "1.12.2", "Fabric", 1),
            file(13, "2026-05-01T00:00:00Z", "1.20.1", "Forge", 1),
            unavailable,
            other_project,
        ];
        let loader = ModLoader::Cleanroom.curseforge_loader_type();
        assert_eq!(
            select_update(
                &files,
                &current,
                "1.12.2",
                loader,
                ReleaseChannel::Release
            )
            .unwrap()
            .id,
            10
        );
        assert_eq!(
            select_update(
                &files,
                &current,
                "1.12.2",
                loader,
                ReleaseChannel::Beta
            )
            .unwrap()
            .id,
            11
        );
    }

    #[test]
    fn update_checks_require_a_later_publication_not_just_a_different_file_id()
    {
        let current = file(456, "2026-02-01T00:00:00Z", "1.12.2", "Forge", 1);
        let files = vec![
            file(999, "2026-01-01T00:00:00Z", "1.12.2", "Forge", 1),
            file(1000, "2026-02-01T01:00:00+01:00", "1.12.2", "Forge", 1),
            file(1001, "unknown", "1.12.2", "Forge", 1),
        ];
        assert!(
            select_update(
                &files,
                &current,
                "1.12.2",
                Some(1),
                ReleaseChannel::Alpha
            )
            .is_none()
        );
    }

    #[test]
    fn resource_updates_do_not_require_a_mod_loader_tag() {
        let current = file(456, "2026-01-01T00:00:00Z", "1.12.2", "", 1);
        let files = vec![file(457, "2026-02-01T00:00:00Z", "1.12.2", "", 1)];
        assert_eq!(
            select_update(
                &files,
                &current,
                "1.12.2",
                None,
                ReleaseChannel::Release
            )
            .unwrap()
            .id,
            457
        );
    }
}

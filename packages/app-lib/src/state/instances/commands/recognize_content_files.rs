use std::collections::{HashMap, HashSet};
use std::future::Future;

use futures::{StreamExt, stream};

use crate::api::curseforge::{
    self, CurseForgeCapabilityStatus, CurseForgeFingerprintResult,
};
use crate::state::instances::{
    Instance, InstanceFile, adapters::sqlite::content_rows,
};
use crate::state::{
    ContentProviderRef, CurseForgeFileId, CurseForgeProjectId, State,
};
use crate::util::fetch::sha1_file_async;

use super::list_content::reconcile_exact_provider_matches;
use super::sync_content_files::{
    content_hash_concurrency, instance_content_root, join_content_path,
    project_type_for_file,
};

const FINGERPRINT_LOOKUP_CONCURRENCY: usize = 4;

/// File recognition supplies aliases, independently of how the file was installed.
pub(crate) async fn recognize_curseforge_content_files(
    instance: &Instance,
    files: &[InstanceFile],
    state: &State,
) -> crate::Result<()> {
    if curseforge::capability().status != CurseForgeCapabilityStatus::Ready {
        return Ok(());
    }
    recognize_with(instance, files, state, curseforge::match_fingerprints).await
}

async fn recognize_with<F, Fut>(
    instance: &Instance,
    files: &[InstanceFile],
    state: &State,
    lookup: F,
) -> crate::Result<()>
where
    F: Fn(Vec<u64>) -> Fut + Sync,
    Fut: Future<Output = crate::Result<CurseForgeFingerprintResult>> + Send,
{
    let Some(content_set) =
        content_rows::get_applied_content_set(&instance.id, &state.pool)
            .await?
    else {
        return Ok(());
    };
    let recognized = sqlx::query_scalar::<_, String>(
        "SELECT entry.file_id FROM instance_content_entries entry
         JOIN instance_content_provider_refs ref ON ref.content_entry_id = entry.id
         WHERE entry.content_set_id = ? AND entry.file_id IS NOT NULL
           AND ref.provider IN ('curseforge', 'modrinth') AND ref.provider_release_id IS NOT NULL",
    )
    .bind(&content_set.id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .collect::<HashSet<_>>();
    let root = instance_content_root(&state.directories, instance)?;
    let mut by_fingerprint = HashMap::<u64, Vec<usize>>::new();
    let hash_jobs = files
        .iter()
        .enumerate()
        .filter(|(_, file)| {
            !file.missing
                && !recognized.contains(&file.id)
                && project_type_for_file(file).is_some()
        })
        .map(|(index, file)| {
            let path = join_content_path(&root, &file.relative_path);
            async move {
                let result =
                    curseforge::fingerprint_and_sha1_file(&path, None).await;
                (index, result)
            }
        })
        .collect::<Vec<_>>();
    let hash_results =
        stream::iter(hash_jobs).buffered(content_hash_concurrency());
    futures::pin_mut!(hash_results);
    while let Some((index, result)) = hash_results.next().await {
        let file = &files[index];
        let (size, sha1, fingerprint) = result?;
        // A file can change while the scan or network request is in progress.
        if size == file.size && sha1 == file.sha1 && fingerprint != 0 {
            by_fingerprint
                .entry(fingerprint as u64)
                .or_default()
                .push(index);
        }
    }
    let fingerprints = by_fingerprint.keys().copied().collect::<Vec<_>>();
    let mut verification_candidates = Vec::new();
    let lookup_jobs = fingerprints
        .chunks(1000)
        .map(|batch| lookup(batch.to_vec()))
        .collect::<Vec<_>>();
    let responses =
        stream::iter(lookup_jobs).buffered(FINGERPRINT_LOOKUP_CONCURRENCY);
    futures::pin_mut!(responses);
    while let Some(response) = responses.next().await {
        for matched in response?.exact_matches {
            let Some(candidates) =
                by_fingerprint.get(&matched.file.file_fingerprint)
            else {
                continue;
            };
            let reference = ContentProviderRef::CurseForge {
                project_id: CurseForgeProjectId::new(matched.file.mod_id)?,
                file_id: Some(CurseForgeFileId::new(matched.file.id)?),
            };
            for &index in candidates {
                let file = &files[index];
                if matched.file.file_length != file.size
                    || !matched.file.hashes.iter().any(|hash| {
                        hash.algo == 1
                            && hash.value.eq_ignore_ascii_case(&file.sha1)
                    })
                {
                    continue;
                }
                verification_candidates.push((index, reference.clone()));
            }
        }
    }
    let verification_jobs = verification_candidates
        .into_iter()
        .map(|(index, reference)| {
            let file = &files[index];
            let path = join_content_path(&root, &file.relative_path);
            async move {
                let result = sha1_file_async(&path).await;
                (index, reference, result)
            }
        })
        .collect::<Vec<_>>();
    let verified =
        stream::iter(verification_jobs).buffered(content_hash_concurrency());
    futures::pin_mut!(verified);
    let mut references = HashMap::new();
    let mut verified_files = Vec::new();
    while let Some((index, reference, result)) = verified.next().await {
        let file = &files[index];
        let (size, sha1) = result?;
        if size != file.size || sha1 != file.sha1 {
            continue;
        }
        references.insert(sha1, reference);
        verified_files.push(file.clone());
    }
    reconcile_exact_provider_matches(
        &instance.id,
        &content_set.id,
        &verified_files,
        &references,
        state,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::curseforge_content_updates::{
        CurseForgeContentUpdate, CurseForgeUpdateCandidate,
        cached_curseforge_updates, persist_curseforge_update_checks,
    };
    use super::*;
    use crate::state::instances::{
        ContentOwnershipKind, ContentSourceKind, CreateInstance, InstanceLink,
        commands::apply_content_install::record_project_file_atomic,
        commands::sync_content_files::sync_instance_content_files,
    };
    use crate::state::{
        ContentProvider, DirectoryInfo, ModLoader, ModrinthProjectId,
        ProjectType,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    async fn fixture() -> (tempfile::TempDir, std::sync::Arc<State>, Instance) {
        let root = tempfile::tempdir().unwrap();
        let directories = DirectoryInfo {
            settings_dir: root.path().join("settings"),
            config_dir: root.path().join("config"),
            app_identifier: "recognition-test".into(),
        };
        std::fs::create_dir_all(directories.instances_dir()).unwrap();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let state = crate::state::test_state(directories, pool).await.unwrap();
        let instance = crate::state::instances::create_instance(
            CreateInstance {
                name: "MRPACK recognition".into(),
                path: Some("recognition".into()),
                game_version: "1.12.2".into(),
                loader: ModLoader::Vanilla,
                loader_version: None,
                icon_path: None,
                symlink_target: None,
                game_dir_override: None,
                link: InstanceLink::ImportedModpack {
                    project_id: None,
                    version_id: None,
                    name: Some("Test pack".into()),
                    version_number: Some("1".into()),
                    filename: Some("test.mrpack".into()),
                },
            },
            &state,
        )
        .await
        .unwrap();
        sqlx::query("UPDATE instance_content_sets SET loader = 'cleanroom', loader_version = '0.6.8' WHERE instance_id = ?")
            .bind(&instance.id).execute(&state.pool).await.unwrap();
        (root, state, instance)
    }

    fn response(bytes: &[u8]) -> CurseForgeFingerprintResult {
        serde_json::from_value(serde_json::json!({
            "isCacheBuilt": true,
            "exactMatches": [{
                "id": 123,
                "file": {
                    "id": 456, "modId": 123, "gameId": 432,
                    "isAvailable": true, "displayName": "1.0", "fileName": "mod.jar",
                    "releaseType": 1, "fileStatus": 4, "fileDate": "2026-01-01T00:00:00Z",
                    "fileLength": bytes.len(), "downloadCount": 1,
                    "fileFingerprint": curseforge::compute_fingerprint(bytes),
                    "hashes": [{ "algo": 1, "value": sha1_smol::Sha1::from(bytes).hexdigest() }]
                },
                "latestFiles": null
            }],
            "exactFingerprints": null,
            "partialMatches": null,
            "installedFingerprints": null,
            "unmatchedFingerprints": null
        })).unwrap()
    }

    #[tokio::test]
    async fn registered_mrpack_members_gain_identity_without_changing_pack_defaults()
     {
        for (path, project_type) in [
            ("mods/pack.jar", ProjectType::Mod),
            ("mods/override.jar.disabled", ProjectType::Mod),
            ("resourcepacks/pack.zip", ProjectType::ResourcePack),
        ] {
            let (_root, state, instance) = fixture().await;
            let bytes = b"a pack member not hosted on Modrinth";
            let root =
                instance_content_root(&state.directories, &instance).unwrap();
            crate::util::io::write(root.join(path), bytes)
                .await
                .unwrap();
            record_project_file_atomic(
                &instance.id,
                path,
                &sha1_smol::Sha1::from(bytes).hexdigest(),
                bytes.len() as u64,
                project_type,
                ContentSourceKind::ImportedModpack,
                ContentOwnershipKind::PackManaged,
                None,
                false,
                None,
                &state,
            )
            .await
            .unwrap();
            let files = sync_instance_content_files(&instance, &state)
                .await
                .unwrap();
            let set = content_rows::get_applied_content_set(
                &instance.id,
                &state.pool,
            )
            .await
            .unwrap()
            .unwrap();
            let entries_before =
                content_rows::get_content_entries(&set.id, &state.pool)
                    .await
                    .unwrap();
            let mut members_before =
                content_rows::get_pack_members(&set.id, &state.pool)
                    .await
                    .unwrap();
            let calls = AtomicUsize::new(0);
            for _ in 0..2 {
                recognize_with(&instance, &files, &state, |fingerprints| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(
                        fingerprints,
                        vec![curseforge::compute_fingerprint(bytes) as u64]
                    );
                    async { Ok(response(bytes)) }
                })
                .await
                .unwrap();
            }
            assert_eq!(
                calls.load(Ordering::SeqCst),
                1,
                "exact matches should be reused"
            );
            let entries =
                content_rows::get_content_entries(&set.id, &state.pool)
                    .await
                    .unwrap();
            let members = content_rows::get_pack_members(&set.id, &state.pool)
                .await
                .unwrap();
            assert_eq!(
                serde_json::to_value(&entries).unwrap(),
                serde_json::to_value(&entries_before).unwrap()
            );
            // Recognition can give an unidentified default a download identity,
            // while retaining its original bytes, path, disabled state and ownership.
            members_before[0].provider =
                Some(crate::state::ContentProvider::CurseForge);
            members_before[0].provider_project_id = Some("123".into());
            members_before[0].provider_release_id = Some("456".into());
            assert_eq!(
                serde_json::to_value(&members).unwrap(),
                serde_json::to_value(&members_before).unwrap()
            );
            let references =
                content_rows::get_content_provider_refs_with_origin(
                    &entries[0].id,
                    &state.pool,
                )
                .await
                .unwrap();
            assert_eq!(references.len(), 1);
            assert!(
                !references[0].1,
                "recognition must not invent an install origin"
            );
            let target = content_rows::get_content_mutation_target(
                &instance.id,
                &members[0].id,
                &state.pool,
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(
                target.provider,
                Some(crate::state::ContentProvider::CurseForge)
            );
            assert_eq!(target.provider_project_id.as_deref(), Some("123"));
            assert_eq!(target.provider_release_id.as_deref(), Some("456"));
            assert_eq!(
                target.ownership_kind,
                ContentOwnershipKind::PackManaged
            );
            assert_eq!(target.relative_path.as_deref(), Some(path));
            assert_eq!(
                content_rows::get_applied_content_set(
                    &instance.id,
                    &state.pool
                )
                .await
                .unwrap()
                .unwrap()
                .revision,
                set.revision + 1
            );
        }
    }

    #[tokio::test]
    async fn parallel_recognition_keeps_each_file_with_its_matching_project() {
        let (_root, state, instance) = fixture().await;
        let root =
            instance_content_root(&state.directories, &instance).unwrap();
        let mut matches = HashMap::new();
        for index in 0..12_u32 {
            let path = format!(
                "mods/member-{index}.jar{}",
                if index % 3 == 0 { ".disabled" } else { "" }
            );
            let bytes = vec![index as u8 + 65; 256 * 1024 + index as usize];
            crate::util::io::write(root.join(&path), &bytes)
                .await
                .unwrap();
            let mut matched = response(&bytes).exact_matches.remove(0);
            matched.id = 1000 + index;
            matched.file.mod_id = matched.id;
            matched.file.id = 2000 + index;
            matches.insert(matched.file.file_fingerprint, matched);
        }
        let files = sync_instance_content_files(&instance, &state)
            .await
            .unwrap();
        assert_eq!(files.len(), 12);
        recognize_with(&instance, &files, &state, |fingerprints| {
            let mut result = response(b"unused");
            result.exact_matches = fingerprints
                .into_iter()
                .rev()
                .map(|fingerprint| matches[&fingerprint].clone())
                .collect();
            async move { Ok(result) }
        })
        .await
        .unwrap();
        let rows = sqlx::query(
            "SELECT file.relative_path, ref.provider_project_id, ref.provider_release_id
             FROM instance_files file
             JOIN instance_content_entries entry ON entry.file_id = file.id
             JOIN instance_content_provider_refs ref ON ref.content_entry_id = entry.id
             WHERE file.instance_id = ? AND ref.provider = 'curseforge'",
        )
        .bind(&instance.id)
        .fetch_all(&state.pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 12);
        use sqlx::Row;
        for row in rows {
            let project: String = row.get("provider_project_id");
            let index = project.parse::<u32>().unwrap() - 1000;
            assert_eq!(
                row.get::<String, _>("provider_release_id"),
                (2000 + index).to_string()
            );
            assert_eq!(
                row.get::<String, _>("relative_path"),
                format!(
                    "mods/member-{index}.jar{}",
                    if index % 3 == 0 { ".disabled" } else { "" }
                )
            );
        }
    }

    #[tokio::test]
    async fn untracked_files_are_recognized_and_use_the_curseforge_mutation_route()
     {
        let (_root, state, instance) = fixture().await;
        let root =
            instance_content_root(&state.directories, &instance).unwrap();
        let bytes = b"local mod";
        // Identical bytes at multiple paths share a single fingerprint request.
        for path in ["mods/local.jar", "mods/local-copy.jar.disabled"] {
            crate::util::io::write(root.join(path), bytes)
                .await
                .unwrap();
        }
        let files = sync_instance_content_files(&instance, &state)
            .await
            .unwrap();
        recognize_with(&instance, &files, &state, |fingerprints| {
            assert_eq!(fingerprints.len(), 1);
            async { Ok(response(bytes)) }
        })
        .await
        .unwrap();
        let set =
            content_rows::get_applied_content_set(&instance.id, &state.pool)
                .await
                .unwrap()
                .unwrap();
        let entries = content_rows::get_content_entries(&set.id, &state.pool)
            .await
            .unwrap();
        assert_eq!(entries.len(), 2);
        for entry in entries {
            assert_eq!(entry.source_kind, ContentSourceKind::Local);
            assert_eq!(entry.ownership_kind, ContentOwnershipKind::UserAdded);
            assert_eq!(
                content_rows::get_content_origin_provider(
                    &entry.id,
                    &state.pool
                )
                .await
                .unwrap(),
                None
            );
            let target = content_rows::get_content_mutation_target(
                &instance.id,
                &entry.id,
                &state.pool,
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(
                target.provider,
                Some(crate::state::ContentProvider::CurseForge)
            );
            assert_eq!(
                entry.enabled,
                !target.relative_path.unwrap().ends_with(".disabled")
            );
            let by_file = content_rows::get_content_mutation_target(
                &instance.id,
                entry.file_id.as_ref().unwrap(),
                &state.pool,
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(by_file.entry_id.as_deref(), Some(entry.id.as_str()));
            assert_eq!(by_file.provider, Some(ContentProvider::CurseForge));
            assert_eq!(by_file.provider_project_id.as_deref(), Some("123"));
            assert_eq!(by_file.provider_release_id.as_deref(), Some("456"));
        }
    }

    #[tokio::test]
    async fn project_only_origins_do_not_borrow_versions_from_an_alias_or_pack_default()
     {
        for origin in [
            ContentProviderRef::Modrinth {
                project_id: ModrinthProjectId::new("AANobbMI").unwrap(),
                version_id: None,
            },
            ContentProviderRef::CurseForge {
                project_id: CurseForgeProjectId::new(999).unwrap(),
                file_id: None,
            },
        ] {
            let (_root, state, instance) = fixture().await;
            let bytes = b"recognized file with a project-only install origin";
            let path = "mods/local.jar";
            crate::util::io::write(
                instance_content_root(&state.directories, &instance)
                    .unwrap()
                    .join(path),
                bytes,
            )
            .await
            .unwrap();
            record_project_file_atomic(
                &instance.id,
                path,
                &sha1_smol::Sha1::from(bytes).hexdigest(),
                bytes.len() as u64,
                ProjectType::Mod,
                ContentSourceKind::ImportedModpack,
                ContentOwnershipKind::PackManaged,
                Some(&origin),
                true,
                None,
                &state,
            )
            .await
            .unwrap();
            let files = sync_instance_content_files(&instance, &state)
                .await
                .unwrap();
            recognize_with(&instance, &files, &state, |_| async {
                Ok(response(bytes))
            })
            .await
            .unwrap();
            let set = content_rows::get_applied_content_set(
                &instance.id,
                &state.pool,
            )
            .await
            .unwrap()
            .unwrap();
            let entries =
                content_rows::get_content_entries(&set.id, &state.pool)
                    .await
                    .unwrap();
            let members = content_rows::get_pack_members(&set.id, &state.pool)
                .await
                .unwrap();
            for target_id in [&entries[0].id, &members[0].id, &files[0].id] {
                let target = content_rows::get_content_mutation_target(
                    &instance.id,
                    target_id,
                    &state.pool,
                )
                .await
                .unwrap()
                .unwrap();
                assert_eq!(target.provider, Some(origin.provider()));
                assert_eq!(
                    target.provider_project_id.as_deref(),
                    Some(origin.database_project_id().as_str())
                );
                assert_eq!(target.provider_release_id, None);
            }
        }
    }

    #[tokio::test]
    async fn recognized_update_cache_rejects_changed_files_and_in_flight_target_changes()
     {
        let (_root, state, instance) = fixture().await;
        let bytes = b"local file whose updates survive a cached snapshot";
        let path = "mods/local.jar";
        let file_path = instance_content_root(&state.directories, &instance)
            .unwrap()
            .join(path);
        crate::util::io::write(&file_path, bytes).await.unwrap();
        let files = sync_instance_content_files(&instance, &state)
            .await
            .unwrap();
        recognize_with(&instance, &files, &state, |_| async {
            Ok(response(bytes))
        })
        .await
        .unwrap();
        let set =
            content_rows::get_applied_content_set(&instance.id, &state.pool)
                .await
                .unwrap()
                .unwrap();
        let candidates = vec![CurseForgeUpdateCandidate {
            relative_path: path.into(),
            project_type: ProjectType::Mod,
            project_id: CurseForgeProjectId::new(123).unwrap(),
            current_file_id: CurseForgeFileId::new(456).unwrap(),
        }];
        let current = response(bytes).exact_matches.remove(0).file;
        let mut target = current.clone();
        target.id = 457;
        let updates = HashMap::from([(
            path.into(),
            CurseForgeContentUpdate {
                current,
                target: Some(target),
            },
        )]);
        persist_curseforge_update_checks(
            &instance,
            &set,
            &candidates,
            &updates,
            &state,
        )
        .await
        .unwrap();
        assert_eq!(
            cached_curseforge_updates(&instance, &set, &candidates, &state)
                .await
                .unwrap()
                .len(),
            1
        );
        for change_loader in [false, true] {
            let mut changed = set.clone();
            if change_loader {
                changed.loader = ModLoader::Fabric;
            } else {
                changed.game_version = "1.20.1".into();
            }
            // Use the same timestamp to exercise invalidation within a single second.
            let mut tx = state.pool.begin().await.unwrap();
            content_rows::update_content_set(&changed, &mut tx)
                .await
                .unwrap();
            tx.commit().await.unwrap();
            persist_curseforge_update_checks(
                &instance,
                &set,
                &candidates,
                &updates,
                &state,
            )
            .await
            .unwrap();
            assert!(
                cached_curseforge_updates(
                    &instance,
                    &changed,
                    &candidates,
                    &state
                )
                .await
                .unwrap()
                .is_empty()
            );
            let mut tx = state.pool.begin().await.unwrap();
            content_rows::update_content_set(&set, &mut tx)
                .await
                .unwrap();
            tx.commit().await.unwrap();
            persist_curseforge_update_checks(
                &instance,
                &set,
                &candidates,
                &updates,
                &state,
            )
            .await
            .unwrap();
            assert_eq!(
                cached_curseforge_updates(&instance, &set, &candidates, &state)
                    .await
                    .unwrap()
                    .len(),
                1
            );
        }
        crate::util::io::write(&file_path, b"replacement bytes")
            .await
            .unwrap();
        sync_instance_content_files(&instance, &state)
            .await
            .unwrap();
        // Even a late successful response for the old bytes cannot repopulate the cache.
        persist_curseforge_update_checks(
            &instance,
            &set,
            &candidates,
            &updates,
            &state,
        )
        .await
        .unwrap();
        assert!(
            cached_curseforge_updates(&instance, &set, &candidates, &state)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn changed_bytes_and_hash_collisions_are_not_attached_to_a_remote_project()
     {
        for scenario in 0..4 {
            let (_root, state, instance) = fixture().await;
            let path = instance_content_root(&state.directories, &instance)
                .unwrap()
                .join("mods/local.jar");
            let bytes = b"original bytes";
            crate::util::io::write(&path, bytes).await.unwrap();
            let files = sync_instance_content_files(&instance, &state)
                .await
                .unwrap();
            recognize_with(&instance, &files, &state, |_| {
                let path = path.clone();
                async move {
                    let mut matched = response(bytes);
                    if scenario == 1 {
                        crate::util::io::write(path, b"changed bytes")
                            .await
                            .unwrap();
                    } else if scenario == 0 {
                        matched.exact_matches[0].file.hashes[0].value =
                            "different SHA-1".into();
                    } else if scenario == 2 {
                        matched.exact_matches[0].file.hashes.clear();
                    } else {
                        matched.exact_matches[0].file.file_length += 1;
                    }
                    Ok(matched)
                }
            })
            .await
            .unwrap();
            let set = content_rows::get_applied_content_set(
                &instance.id,
                &state.pool,
            )
            .await
            .unwrap()
            .unwrap();
            assert!(
                content_rows::get_content_entries(&set.id, &state.pool)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
    }
}

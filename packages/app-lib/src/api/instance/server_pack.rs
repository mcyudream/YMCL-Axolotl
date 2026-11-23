use super::{get, get_content_snapshot, get_full_path, resolve_export_loader};
use crate::event::LoadingBarType;
use crate::event::emit::{emit_loading, init_loading};
use crate::state::{
    CacheBehaviour, CachedEntry, ContentItem, ContentProviderRef,
    InstanceContentSnapshot, ModServerSupport, ModSideInfo,
    PackMemberMaterializationState, ProjectType, State,
};
use crate::util::io::{self, IOError};
use async_zip::tokio::write::ZipFileWriter;
use async_zip::{Compression, ZipEntryBuilder};
use futures::io::AsyncWriteExt;
use futures::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

pub const SERVER_PACK_MANIFEST: &str = "ymcl-server-manifest.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerPackMode {
    Full,
    Slim,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerPackFile {
    pub path: String,
    pub size: u64,
    pub selected: bool,
    pub mod_side: Option<ModSideInfo>,
    pub requires: Vec<String>,
    pub downloads: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerPackExcludedMod {
    pub path: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerPackPreview {
    pub revision: u64,
    pub files: Vec<ServerPackFile>,
    pub excluded_mods: Vec<ServerPackExcludedMod>,
    pub missing_mods: Vec<String>,
}

fn is_server_support_file(path: &str) -> bool {
    let parts = path.split('/').collect::<Vec<_>>();
    if parts
        .iter()
        .any(|part| part.starts_with('.') || *part == "__MACOSX")
    {
        return false;
    }
    if path.ends_with(".disabled")
        || path.ends_with(".old")
        || path.ends_with(".log")
    {
        return false;
    }
    match parts[0] {
        "server.properties" if parts.len() == 1 => true,
        "config" | "defaultconfigs" | "scripts" | "datapacks"
        | "global_packs" | "openloader" | "crafttweaker" | "structures" => true,
        "kubejs" => parts.get(1).is_some_and(|part| {
            matches!(
                *part,
                "server_scripts"
                    | "startup_scripts"
                    | "config"
                    | "data"
                    | "assets"
            )
        }),
        "paxi" => parts.get(1) == Some(&"datapacks"),
        _ => false,
    }
}

fn default_selected(side: &ModSideInfo) -> bool {
    side.server_support != ModServerSupport::Unsupported
}

async fn mod_downloads(
    item: &ContentItem,
    state: &State,
    curseforge_files: &HashMap<u32, crate::api::curseforge::CurseForgeFile>,
) -> Vec<String> {
    for reference in &item.provider_refs {
        match reference {
            ContentProviderRef::Modrinth {
                version_id: Some(version_id),
                ..
            } => {
                if let Ok(Some(version)) = CachedEntry::get_version(
                    version_id,
                    Some(CacheBehaviour::StaleWhileRevalidate),
                    &state.pool,
                    &state.api_semaphore,
                )
                .await
                    && let Some(file) = version.files.iter().find(|file| {
                        u64::from(file.size) == item.size
                            && file.hashes.get("sha1").is_some_and(|hash| {
                                hash.eq_ignore_ascii_case(&item.id)
                            })
                    })
                    && file.url.starts_with("https://")
                {
                    return vec![file.url.clone()];
                }
            }
            ContentProviderRef::CurseForge {
                project_id,
                file_id: Some(file_id),
            } => {
                if let Some(file) = curseforge_files.get(&file_id.get())
                    && file.mod_id == project_id.get()
                    && file.is_available
                    && file.file_length == item.size
                    && file.hashes.iter().any(|hash| {
                        hash.algo == 1
                            && hash.value.eq_ignore_ascii_case(&item.id)
                    })
                    && let Some(url) = file
                        .download_url
                        .as_ref()
                        .filter(|url| url.starts_with("https://"))
                {
                    return vec![url.clone()];
                }
            }
            _ => {}
        }
    }
    Vec::new()
}

fn dependencies_for(
    item: &crate::state::InstanceContentSnapshotItem,
    snapshot: &InstanceContentSnapshot,
    local_metadata: &HashMap<String, crate::mod_metadata::LocalModMetadata>,
) -> Vec<String> {
    let parent_path = item
        .content
        .as_ref()
        .map_or(item.expected_relative_path.as_str(), |content| {
            content.file_path.as_str()
        });
    let parent = local_metadata.get(parent_path);
    let server_ids = parent
        .and_then(|metadata| metadata.server_dependencies.as_ref())
        .map(|dependencies| {
            dependencies
                .iter()
                .map(|dependency| dependency.mod_id.as_str())
                .collect::<HashSet<_>>()
        });
    let mut requires = item
        .dependency
        .as_ref()
        .into_iter()
        .flat_map(|info| &info.requires)
        .filter_map(|reference| {
            let candidate = reference
                .entry_id
                .as_ref()
                .and_then(|id| {
                    snapshot.items.iter().find(|candidate| {
                        candidate.entry_id.as_ref() == Some(id)
                    })
                })
                .or_else(|| {
                    reference.file_id.as_ref().and_then(|id| {
                        snapshot.items.iter().find(|candidate| {
                            candidate.file_id.as_ref() == Some(id)
                        })
                    })
                })
                .or_else(|| {
                    snapshot.items.iter().find(|candidate| {
                        candidate
                            .content
                            .as_ref()
                            .is_some_and(|content| content.enabled)
                            && ((candidate.provider
                                == Some(reference.provider)
                                && candidate.provider_project_id.as_deref()
                                    == Some(reference.project_id.as_str()))
                                || candidate.content.as_ref().is_some_and(
                                    |content| {
                                        content.provider_refs.iter().any(
                                            |provider| {
                                                provider.provider()
                                                    == reference.provider
                                                    && provider
                                                        .database_project_id()
                                                        == reference.project_id
                                            },
                                        )
                                    },
                                ))
                    })
                });
            if let Some(candidate) = candidate
                && let Some(server_ids) = &server_ids
            {
                let path = candidate.content.as_ref().map_or(
                    candidate.expected_relative_path.as_str(),
                    |content| content.file_path.as_str(),
                );
                if let Some(dependency) = local_metadata.get(path)
                    && !dependency.mod_ids().any(|id| server_ids.contains(id))
                {
                    return None;
                }
            }
            Some(
                candidate
                    .map(|candidate| {
                        candidate.content.as_ref().map_or_else(
                            || candidate.expected_relative_path.clone(),
                            |content| content.file_path.clone(),
                        )
                    })
                    .unwrap_or_else(|| {
                        format!(
                            "{}:{}",
                            reference.provider.as_str(),
                            reference.project_id
                        )
                    }),
            )
        })
        .collect::<Vec<_>>();
    if let Some(server_ids) = server_ids {
        for id in server_ids {
            if parent.is_some_and(|parent| {
                parent.mod_ids().any(|provided| provided == id)
            }) {
                continue;
            }
            let path = snapshot
                .items
                .iter()
                .find_map(|candidate| {
                    let content = candidate.content.as_ref()?;
                    let metadata = local_metadata.get(&content.file_path)?;
                    (content.enabled
                        && metadata.mod_ids().any(|provided| provided == id))
                    .then(|| content.file_path.clone())
                })
                .unwrap_or_else(|| format!("local:{id}"));
            requires.push(path);
        }
    }
    requires.sort();
    requires.dedup();
    requires
}

fn select_dependencies(files: &mut [ServerPackFile]) {
    let by_path = files
        .iter()
        .enumerate()
        .map(|(index, file)| (file.path.clone(), index))
        .collect::<HashMap<_, _>>();
    let mut queue = files
        .iter()
        .filter(|file| file.selected)
        .flat_map(|file| file.requires.clone())
        .collect::<Vec<_>>();
    while let Some(path) = queue.pop() {
        if let Some(index) = by_path.get(&path).copied()
            && !files[index].selected
        {
            files[index].selected = true;
            queue.extend(files[index].requires.clone());
        }
    }
}

fn validate_selection<'a>(
    preview: &'a ServerPackPreview,
    paths: &'a [String],
) -> crate::Result<HashSet<&'a str>> {
    let selected = paths.iter().map(String::as_str).collect::<HashSet<_>>();
    let allowed = preview
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<HashSet<_>>();
    if selected.is_empty() || !selected.is_subset(&allowed) {
        return Err(crate::ErrorKind::InputError(
            "Selected server files changed. Reload the export preview.".into(),
        )
        .into());
    }
    for file in preview
        .files
        .iter()
        .filter(|file| selected.contains(file.path.as_str()))
    {
        if let Some(missing) = file
            .requires
            .iter()
            .find(|path| !selected.contains(path.as_str()))
        {
            return Err(crate::ErrorKind::InputError(format!(
                "{} requires {missing}. Include the dependency or correct its mod side before exporting.", file.path
            )).into());
        }
    }
    Ok(selected)
}

async fn preview_from_snapshot(
    instance_id: &str,
    mode: ServerPackMode,
    snapshot: &InstanceContentSnapshot,
) -> crate::Result<ServerPackPreview> {
    let root = get_full_path(instance_id).await?;
    let mut paths = Vec::new();
    // Do not enumerate worlds, assets and launcher caches just to discard them.
    let mut directories = io::read_dir(&root).await?;
    while let Some(entry) = directories
        .next_entry()
        .await
        .map_err(|error| IOError::with_path(error, &root))?
    {
        let name = entry.file_name();
        if !matches!(
            name.to_str(),
            Some(
                "mods"
                    | "config"
                    | "defaultconfigs"
                    | "scripts"
                    | "datapacks"
                    | "global_packs"
                    | "openloader"
                    | "crafttweaker"
                    | "structures"
                    | "kubejs"
                    | "paxi"
                    | "server.properties"
            )
        ) {
            continue;
        }
        let path = entry.path();
        let metadata = tokio::fs::symlink_metadata(&path).await?;
        if metadata.is_dir() && !io::is_symlink_or_reparse(&metadata) {
            super::export_mrpack::add_all_recursive_folder_paths(
                &path, &mut paths,
            )
            .await?;
        } else if metadata.is_file() && !io::is_symlink_or_reparse(&metadata) {
            paths.push(path);
        }
    }
    let mods = snapshot
        .items
        .iter()
        .filter(|item| item.project_type == ProjectType::Mod)
        .filter_map(|item| {
            item.content
                .as_ref()
                .map(|content| (content.file_path.as_str(), (item, content)))
        })
        .collect::<HashMap<_, _>>();
    let state = State::get().await?;
    let local_metadata = crate::state::instances::adapters::sqlite::content_rows::get_instance_files(instance_id, &state.pool)
        .await?.into_iter()
        .filter_map(|file| {
            let metadata = serde_json::from_str::<crate::mod_metadata::LocalModMetadata>(file.local_mod_data.as_deref()?).ok()?;
            Some((file.relative_path, metadata))
        }).collect::<HashMap<_, _>>();
    let mut files = Vec::new();
    let mut excluded_mods = Vec::new();
    for path in paths {
        let relative = path
            .strip_prefix(&root)
            .map_err(|error| crate::ErrorKind::InputError(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        let is_mod =
            relative.starts_with("mods/") && relative.ends_with(".jar");
        if relative.starts_with("mods/") && relative.ends_with(".jar.disabled")
        {
            excluded_mods.push(ServerPackExcludedMod {
                path: relative,
                reason: "disabled".into(),
            });
            continue;
        }
        if relative.split('/').any(|part| part.starts_with('.')) {
            continue;
        }
        let (mod_side, selected, requires) = if is_mod {
            let installed = mods.get(relative.as_str());
            let side = installed
                .and_then(|(_, content)| content.mod_side.clone())
                .unwrap_or_default();
            if installed.is_some_and(|(_, content)| !content.enabled)
                || side.server_support == ModServerSupport::Unsupported
            {
                excluded_mods.push(ServerPackExcludedMod {
                    path: relative,
                    reason: if side.server_support
                        == ModServerSupport::Unsupported
                    {
                        "client_only".into()
                    } else {
                        "disabled".into()
                    },
                });
                continue;
            }
            let selected = default_selected(&side);
            let requires = installed
                .map(|(item, _)| {
                    dependencies_for(item, snapshot, &local_metadata)
                })
                .unwrap_or_default();
            (Some(side), selected, requires)
        } else if is_server_support_file(&relative) {
            (None, true, Vec::new())
        } else {
            continue;
        };
        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|error| IOError::with_path(error, &path))?;
        files.push(ServerPackFile {
            path: relative,
            size: metadata.len(),
            selected,
            mod_side,
            requires,
            downloads: Vec::new(),
        });
    }
    select_dependencies(&mut files);
    if mode == ServerPackMode::Slim {
        let state = State::get().await?;
        let downloadable_items = files
            .iter()
            .filter_map(|file| {
                mods.get(file.path.as_str())
                    .map(|(_, item)| (*item).clone())
            })
            .collect::<Vec<_>>();
        let mut curseforge_ids = downloadable_items
            .iter()
            .flat_map(|item| &item.provider_refs)
            .filter_map(|reference| match reference {
                ContentProviderRef::CurseForge {
                    file_id: Some(file_id),
                    ..
                } => Some(file_id.get()),
                _ => None,
            })
            .collect::<Vec<_>>();
        curseforge_ids.sort_unstable();
        curseforge_ids.dedup();
        let chunks = curseforge_ids
            .chunks(50)
            .map(<[u32]>::to_vec)
            .collect::<Vec<_>>();
        let batches = stream::iter(chunks)
            .map(crate::api::curseforge::get_files_many)
            .buffer_unordered(4)
            .collect::<Vec<_>>()
            .await;
        let curseforge_files = batches
            .into_iter()
            .filter_map(Result::ok)
            .flatten()
            .map(|file| (file.id, file))
            .collect::<HashMap<_, _>>();
        let curseforge_files = std::sync::Arc::new(curseforge_files);
        let downloads = stream::iter(downloadable_items)
            .map(move |item| {
                let state = std::sync::Arc::clone(&state);
                let curseforge_files = std::sync::Arc::clone(&curseforge_files);
                async move {
                    (
                        item.file_path.clone(),
                        mod_downloads(&item, &state, &curseforge_files).await,
                    )
                }
            })
            .buffer_unordered(4)
            .collect::<HashMap<_, _>>()
            .await;
        for file in &mut files {
            file.downloads =
                downloads.get(&file.path).cloned().unwrap_or_default();
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    excluded_mods.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(ServerPackPreview {
        revision: snapshot.revision,
        files,
        excluded_mods,
        missing_mods: snapshot
            .items
            .iter()
            .filter(|item| {
                item.project_type == ProjectType::Mod
                    && item.required
                    && matches!(
                        item.materialization_state,
                        PackMemberMaterializationState::Missing
                            | PackMemberMaterializationState::PendingManual
                    )
            })
            .map(|item| item.expected_relative_path.clone())
            .collect(),
    })
}

pub async fn preview_server_pack(
    instance_id: &str,
    mode: ServerPackMode,
) -> crate::Result<ServerPackPreview> {
    let state = State::get().await?;
    let _guard = state.lock_instance_content(instance_id).await;
    let snapshot = get_content_snapshot(instance_id).await?;
    preview_from_snapshot(instance_id, mode, &snapshot).await
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerPackExportRequest {
    pub mode: ServerPackMode,
    pub selected_paths: Vec<String>,
    pub revision: u64,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
}

pub async fn export_server_pack(
    instance_id: &str,
    export_path: PathBuf,
    request: ServerPackExportRequest,
) -> crate::Result<()> {
    if request.name.trim().is_empty() || request.version.trim().is_empty() {
        return Err(crate::ErrorKind::InputError(
            "Pack name and version are required".into(),
        )
        .into());
    }
    let state = State::get().await?;
    let _guard = state.lock_instance_content(instance_id).await;
    let snapshot = get_content_snapshot(instance_id).await?;
    if snapshot.revision != request.revision {
        return Err(crate::ErrorKind::InputError(
            "Instance content changed. Reload the export preview.".into(),
        )
        .into());
    }
    let preview =
        preview_from_snapshot(instance_id, request.mode, &snapshot).await?;
    validate_selection(&preview, &request.selected_paths)?;
    let metadata = get(instance_id).await?.ok_or_else(|| {
        crate::ErrorKind::InputError("Unknown instance".into())
    })?;
    let (loader, loader_version) = resolve_export_loader(&metadata)?;
    let root = io::canonicalize(get_full_path(instance_id).await?)?;
    let export_path = std::path::absolute(&export_path)
        .map_err(|error| IOError::with_path(error, &export_path))?;
    let temporary = export_path
        .with_file_name(format!(".ymcl-server-{}.tmp", uuid::Uuid::new_v4()));
    let result = write_archive(
        instance_id,
        &root,
        &temporary,
        &export_path,
        &preview,
        &snapshot,
        &request,
        &metadata.instance.name,
        &metadata.applied_content_set.game_version,
        loader.as_str(),
        loader_version,
    )
    .await;
    if let Err(error) = result {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(error);
    }
    if let Err(error) = io::atomically_replace_file(&temporary, &export_path) {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(error.into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn write_archive(
    instance_id: &str,
    root: &PathBuf,
    temporary: &PathBuf,
    destination: &PathBuf,
    preview: &ServerPackPreview,
    snapshot: &InstanceContentSnapshot,
    request: &ServerPackExportRequest,
    instance_name: &str,
    game_version: &str,
    loader: &str,
    loader_version: Option<String>,
) -> crate::Result<()> {
    let selected = request.selected_paths.iter().collect::<HashSet<_>>();
    let files = preview
        .files
        .iter()
        .filter(|file| selected.contains(&file.path))
        .collect::<Vec<_>>();
    let total_bytes = files.iter().map(|file| file.size).sum::<u64>().max(1);
    let content = snapshot
        .items
        .iter()
        .filter_map(|item| item.content.as_ref())
        .map(|item| (item.file_path.as_str(), item))
        .collect::<HashMap<_, _>>();
    let loading = init_loading(
        LoadingBarType::ZipExtract {
            instance_id: instance_id.into(),
            instance_name: instance_name.into(),
        },
        100.0,
        "Exporting server pack",
    )
    .await?;
    let mut archive = tokio::fs::File::create(temporary)
        .await
        .map_err(|error| IOError::with_path(error, temporary))?;
    let mut writer = ZipFileWriter::with_tokio(&mut archive);
    let mut manifest_files = Vec::new();
    let mut buffer = vec![0u8; 256 * 1024];
    let mut written = 0u64;
    let mut progress = 0.0;
    for file in files {
        let path = root.join(&file.path);
        let canonical = io::canonicalize(&path)?;
        if !canonical.starts_with(root)
            || canonical == *destination
            || canonical == *temporary
        {
            return Err(crate::ErrorKind::InputError(format!(
                "Cannot export path: {}",
                file.path
            ))
            .into());
        }
        let mut input = tokio::fs::File::open(&canonical)
            .await
            .map_err(|error| IOError::with_path(error, &canonical))?;
        let compression =
            if super::export_mrpack::is_already_compressed(&file.path) {
                Compression::Stored
            } else {
                Compression::Deflate
            };
        let mut entry = if file.downloads.is_empty() {
            Some(
                writer
                    .write_entry_stream(ZipEntryBuilder::new(
                        file.path.clone().into(),
                        compression,
                    ))
                    .await?,
            )
        } else {
            None
        };
        let mut sha256 = Sha256::new();
        let mut sha1 = sha1_smol::Sha1::new();
        let mut size = 0u64;
        loop {
            let count = input
                .read(&mut buffer)
                .await
                .map_err(|error| IOError::with_path(error, &canonical))?;
            if count == 0 {
                break;
            }
            sha256.update(&buffer[..count]);
            sha1.update(&buffer[..count]);
            if let Some(entry) = entry.as_mut() {
                entry
                    .write_all(&buffer[..count])
                    .await
                    .map_err(|error| IOError::with_path(error, &canonical))?;
            }
            size += count as u64;
            written += count as u64;
            let next = 95.0 * (written as f64 / total_bytes as f64).min(1.0);
            if next - progress >= 0.1 {
                let _ =
                    emit_loading(&loading, next - progress, Some(&file.path));
                progress = next;
            }
        }
        if let Some(entry) = entry {
            entry.close().await?;
        }
        let item = content.get(file.path.as_str());
        if size != file.size
            || (file.mod_side.is_some()
                && item
                    .is_some_and(|item| item.id != sha1.digest().to_string()))
        {
            return Err(crate::ErrorKind::InputError(format!(
                "File changed during export: {}. Rescan content and retry.",
                file.path
            ))
            .into());
        }
        manifest_files.push(json!({
            "path": file.path, "size": size, "hashes": { "sha256": format!("{:x}", sha256.finalize()) },
            "modSide": file.mod_side,
            "downloads": file.downloads,
            "providerRefs": item.map(|item| &item.provider_refs).cloned().unwrap_or_else(Vec::<ContentProviderRef>::new),
        }));
    }
    // This is an updater contract, not a claim that a panel updater is installed.
    // Removals must be limited to paths managed by the previous manifest.
    let manifest = json!({
        "formatVersion": 1, "format": "ymcl-server-pack", "environment": "server",
        "name": request.name, "version": request.version, "description": request.description,
        "mode": request.mode,
        "minecraft": { "version": game_version, "loader": loader, "loaderVersion": loader_version },
        "files": manifest_files,
        "update": {
            "protocol": "ymcl-server-pack/v1",
            "deletePolicy": "previous_manifest_only",
            "preservePaths": ["world/**", "world_nether/**", "world_the_end/**", "server.properties", "ops.json", "whitelist.json", "banned-players.json", "banned-ips.json"],
            "endpoint": null
        }
    });
    for (name, data) in [
        (
            "ymcl-server-bootstrap.py",
            include_str!("server_bootstrap.py"),
        ),
        (
            "start-server.sh",
            "#!/bin/sh\nset -eu\ncd -- \"$(dirname -- \"$0\")\"\nexec python3 ymcl-server-bootstrap.py \"$@\"\n",
        ),
        (
            "start-server.cmd",
            "@echo off\r\ncd /d \"%~dp0\"\r\nwhere py >nul 2>nul\r\nif %errorlevel% equ 0 (\r\n  py -3 ymcl-server-bootstrap.py %*\r\n) else (\r\n  python ymcl-server-bootstrap.py %*\r\n)\r\n",
        ),
        (
            "SERVER-README.txt",
            "YMCL server pack / 服务端包\n\nRequires Java for your Minecraft version and Python 3.9+.\n需要对应 Minecraft 版本的 Java 和 Python 3.9 或更新版本。\n\nWindows: start-server.cmd\nLinux/macOS: sh start-server.sh\nDownload only / 仅准备 Mod: python3 ymcl-server-bootstrap.py --prepare-only\n\nThe slim pack downloads referenced mods on first launch and verifies SHA-256.\nLocal mods or mods without a verified public download URL are bundled.\n精简包首次启动下载有可用来源的 Mod 并校验 SHA-256；本地 Mod 和无可用下载地址的 Mod 随包附带。\n\nInstall the matching dedicated server core in this directory before launching.\nThis export does not include Java, the server core, worlds or player data.\n启动前请在此目录安装对应版本的服务端核心。导出不包含 Java、服务端核心、世界及玩家数据。\n\nReview unknown mod sides and client-specific config before hosting.\n开服前请检查未知端别的 Mod 和配置中的客户端选项。\n",
        ),
    ] {
        writer
            .write_entry_whole(
                ZipEntryBuilder::new(
                    name.to_string().into(),
                    Compression::Deflate,
                ),
                data.as_bytes(),
            )
            .await?;
    }
    writer
        .write_entry_whole(
            ZipEntryBuilder::new(
                SERVER_PACK_MANIFEST.to_string().into(),
                Compression::Deflate,
            ),
            &serde_json::to_vec_pretty(&manifest)?,
        )
        .await?;
    writer.close().await?;
    // Tokio files may still have a blocking write in flight when the ZIP
    // writer closes. Finish it before the caller renames or reads the archive.
    tokio::io::AsyncWriteExt::flush(&mut archive)
        .await
        .map_err(|error| IOError::with_path(error, temporary))?;
    archive
        .sync_all()
        .await
        .map_err(|error| IOError::with_path(error, temporary))?;
    let _ =
        emit_loading(&loading, 100.0 - progress, Some("Server pack exported"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ModSide, ModSideSource};

    #[test]
    fn server_pack_commands_can_run_on_the_desktop_async_executor() {
        fn assert_send<T: Send>(_: T) {}
        assert_send(preview_server_pack("pack", ServerPackMode::Slim));
        assert_send(export_server_pack(
            "pack",
            PathBuf::from("server.zip"),
            ServerPackExportRequest {
                mode: ServerPackMode::Slim,
                selected_paths: Vec::new(),
                revision: 0,
                name: "Pack".into(),
                version: "1.0".into(),
                description: String::new(),
            },
        ));
    }

    #[test]
    fn both_formats_keep_unknown_and_optional_mods_and_exclude_client_only() {
        let mut info =
            ModSideInfo::manual(ModSide::Unknown, ModSideSource::Unknown);
        assert!(default_selected(&info));
        info.server_support = ModServerSupport::Optional;
        assert!(default_selected(&info));
        info.server_support = ModServerSupport::Required;
        assert!(default_selected(&info));
        info.server_support = ModServerSupport::Unsupported;
        assert!(!default_selected(&info));
    }

    #[test]
    fn excludes_client_scripts_private_data_and_disabled_files() {
        for path in [
            "kubejs/client_scripts/hud.js",
            "options.txt",
            "servers.dat",
            "logs/latest.log",
            "saves/World/level.dat",
            "config/private/.token",
            "config/a.old",
        ] {
            assert!(!is_server_support_file(path), "{path}");
        }
        for path in [
            "config/a.toml",
            "defaultconfigs/a.toml",
            "kubejs/startup_scripts/registry.js",
            "kubejs/server_scripts/recipes.js",
            "scripts/recipes.zs",
            "datapacks/recipes.zip",
            "server.properties",
        ] {
            assert!(is_server_support_file(path), "{path}");
        }
    }

    #[test]
    fn slim_retains_transitive_required_dependencies_and_handles_cycles() {
        let file = |path: &str, selected, requires: Vec<&str>| ServerPackFile {
            path: path.into(),
            size: 1,
            selected,
            mod_side: None,
            requires: requires.into_iter().map(String::from).collect(),
            downloads: Vec::new(),
        };
        let mut files = vec![
            file("mod", true, vec!["lib"]),
            file("lib", false, vec!["nested"]),
            file("nested", false, vec!["mod"]),
            file("unrelated", false, vec![]),
        ];
        select_dependencies(&mut files);
        assert_eq!(
            files.iter().map(|file| file.selected).collect::<Vec<_>>(),
            [true, true, true, false]
        );
    }

    #[test]
    fn export_selection_cannot_bypass_exclusions_or_drop_required_dependencies()
    {
        let preview = ServerPackPreview {
            revision: 1,
            excluded_mods: Vec::new(),
            missing_mods: Vec::new(),
            files: vec![
                ServerPackFile {
                    path: "mods/parent.jar".into(),
                    size: 1,
                    selected: true,
                    mod_side: None,
                    requires: vec!["mods/library.jar".into()],
                    downloads: Vec::new(),
                },
                ServerPackFile {
                    path: "mods/library.jar".into(),
                    size: 1,
                    selected: true,
                    mod_side: None,
                    requires: Vec::new(),
                    downloads: Vec::new(),
                },
            ],
        };
        assert!(validate_selection(&preview, &[]).is_err());
        assert!(
            validate_selection(&preview, &["mods/client-only.jar".into()])
                .is_err()
        );
        assert!(validate_selection(&preview, &["../outside".into()]).is_err());
        assert!(
            validate_selection(&preview, &["mods/parent.jar".into()]).is_err()
        );
        assert!(
            validate_selection(
                &preview,
                &["mods/parent.jar".into(), "mods/library.jar".into()]
            )
            .is_ok()
        );
    }

    #[test]
    fn server_dependency_validation_ignores_client_only_edges_and_keeps_server_libraries()
     {
        let item = |id: &str| {
            serde_json::from_value::<crate::state::InstanceContentSnapshotItem>(json!({
            "ownershipKind": "user_added", "materializationState": "present", "overrideKind": "none",
            "entryId": id, "expectedRelativePath": format!("mods/{id}.jar"),
            "required": true, "projectType": "mod",
            "content": {
                "id": id, "file_name": format!("{id}.jar"), "file_path": format!("mods/{id}.jar"),
                "size": 1, "enabled": true, "project_type": "mod", "provider_refs": [], "external": true
            },
            "capabilities": { "canToggle": true, "canDelete": true, "canUpdate": false, "canChangeVersion": false, "canRestorePackDefault": false }
        })).unwrap()
        };
        let mut parent = item("parent");
        parent.dependency = Some(serde_json::from_value(json!({
            "autoDependency": false, "requiredBy": [], "requires": [
                { "entryId": "client_ui", "provider": "local", "projectId": "local:client_ui", "releaseId": "1" },
                { "entryId": "server_library", "provider": "local", "projectId": "local:server_library", "releaseId": "1" }
            ]
        })).unwrap());
        let snapshot = InstanceContentSnapshot {
            instance_id: "pack".into(),
            revision: 1,
            pack: None,
            items: vec![parent, item("client_ui"), item("server_library")],
            pending_manual_downloads: Vec::new(),
            warnings: Vec::new(),
        };
        let mut local_metadata = HashMap::new();
        for id in ["parent", "client_ui", "server_library"] {
            let metadata = serde_json::from_value::<crate::mod_metadata::LocalModMetadata>(json!({
                "mod_id": id,
                "server_dependencies": if id == "parent" { json!([{ "mod_id": "server_library" }]) } else { json!([]) }
            })).unwrap();
            local_metadata.insert(format!("mods/{id}.jar"), metadata);
        }
        assert_eq!(
            dependencies_for(&snapshot.items[0], &snapshot, &local_metadata),
            ["mods/server_library.jar"]
        );
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn archives_bundle_full_mods_or_emit_verified_slim_downloads() {
        use std::io::Read;
        crate::EventState::init().await.unwrap();
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        std::fs::create_dir_all(root.join("mods")).unwrap();
        std::fs::write(root.join("mods/remote.jar"), b"remote mod").unwrap();
        std::fs::write(root.join("mods/local.jar"), b"local mod").unwrap();
        let snapshot: InstanceContentSnapshot = serde_json::from_value(json!({
            "instanceId": "pack", "revision": 1, "pack": null, "items": [],
            "pendingManualDownloads": [], "warnings": []
        }))
        .unwrap();
        for mode in [ServerPackMode::Full, ServerPackMode::Slim] {
            let urls = if mode == ServerPackMode::Slim {
                vec!["https://cdn.example.test/remote.jar".into()]
            } else {
                Vec::new()
            };
            let preview = ServerPackPreview {
                revision: 1,
                excluded_mods: Vec::new(),
                missing_mods: Vec::new(),
                files: vec![
                    ServerPackFile {
                        path: "mods/remote.jar".into(),
                        size: 10,
                        selected: true,
                        mod_side: Some(ModSideInfo::default()),
                        requires: Vec::new(),
                        downloads: urls.clone(),
                    },
                    ServerPackFile {
                        path: "mods/local.jar".into(),
                        size: 9,
                        selected: true,
                        mod_side: Some(ModSideInfo::default()),
                        requires: Vec::new(),
                        downloads: Vec::new(),
                    },
                ],
            };
            let request = ServerPackExportRequest {
                mode,
                revision: 1,
                name: "Pack".into(),
                version: "1.0".into(),
                description: String::new(),
                selected_paths: preview
                    .files
                    .iter()
                    .map(|file| file.path.clone())
                    .collect(),
            };
            let path = root.join(format!("{mode:?}.zip"));
            write_archive(
                "pack",
                &root,
                &path,
                &root.join("destination.zip"),
                &preview,
                &snapshot,
                &request,
                "Pack",
                "1.20.1",
                "fabric",
                Some("0.16.0".into()),
            )
            .await
            .unwrap();
            let mut zip =
                zip::ZipArchive::new(std::fs::File::open(&path).unwrap())
                    .unwrap();
            assert_eq!(
                zip.by_name("mods/remote.jar").is_ok(),
                mode == ServerPackMode::Full
            );
            assert!(zip.by_name("mods/local.jar").is_ok());
            for name in [
                "start-server.sh",
                "start-server.cmd",
                "ymcl-server-bootstrap.py",
                "SERVER-README.txt",
            ] {
                assert!(zip.by_name(name).is_ok(), "{name}");
            }
            let mut data = String::new();
            zip.by_name(SERVER_PACK_MANIFEST)
                .unwrap()
                .read_to_string(&mut data)
                .unwrap();
            let manifest: serde_json::Value =
                serde_json::from_str(&data).unwrap();
            assert_eq!(manifest["files"][0]["downloads"], json!(urls));
            assert_eq!(
                manifest["files"][0]["hashes"]["sha256"],
                format!("{:x}", Sha256::digest(b"remote mod"))
            );
            assert_eq!(
                manifest["update"]["deletePolicy"],
                "previous_manifest_only"
            );
            assert_eq!(manifest["minecraft"]["loaderVersion"], "0.16.0");
        }
    }
}

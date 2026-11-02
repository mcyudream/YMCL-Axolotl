//! Source-neutral schematic discovery and acquisition.

mod public;

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Read;
pub(crate) const MAX_FILE_SIZE: usize = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Reden,
    #[serde(rename = "buildschematics")]
    BuildSchematics,
}

impl Source {
    pub fn id(self) -> &'static str {
        match self {
            Self::Reden => "reden",
            Self::BuildSchematics => "buildschematics",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub name: String,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub source: Source,
    pub id: String,
    pub title: String,
    pub summary: String,
    pub author: String,
    pub cover: Option<String>,
    pub game_versions: Vec<String>,
    pub files: Vec<File>,
    pub files_resolved: bool,
    pub source_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    #[serde(flatten)]
    pub project: Project,
    pub description: String,
    pub description_format: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub items: Vec<Project>,
    pub page_count: u32,
    pub total: u64,
    pub failed_sources: Vec<Source>,
}

fn failure(message: &str) -> crate::Error {
    crate::ErrorKind::InputError(message.to_owned()).into()
}

pub async fn list(
    sources: Vec<Source>,
    page: u32,
    query: String,
    page_size: u32,
    sort: String,
) -> crate::Result<Page> {
    if page == 0
        || page > 10000
        || query.chars().count() > 200
        || ![5, 10, 15, 20, 50, 100].contains(&page_size)
        || sources.is_empty()
    {
        return Err(failure("Invalid schematic search"));
    }
    let mut seen = HashSet::new();
    let sources: Vec<_> = sources
        .into_iter()
        .filter(|source| seen.insert(*source))
        .collect();
    if sources.len() == 1 {
        return list_source(sources[0], page, query, page_size, sort).await;
    }
    if sort != "default" {
        return Err(failure("Unsupported combined schematic sort"));
    }
    let replies = join_all(sources.iter().map(|source| {
        list_source(
            *source,
            1,
            query.clone(),
            page_size,
            default_sort(*source).into(),
        )
    }))
    .await;
    let mut feeds = Vec::new();
    let mut failed_sources = Vec::new();
    let mut first_error = None;
    for (source, reply) in sources.into_iter().zip(replies) {
        match reply {
            Ok(reply) => feeds.push((source, reply)),
            Err(error) => {
                failed_sources.push(source);
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }
    if feeds.is_empty() {
        return Err(first_error
            .unwrap_or_else(|| failure("No schematic sources are available")));
    }
    let total: u64 = feeds.iter().map(|(_, reply)| reply.total).sum();
    let start = (page as u64 - 1) * page_size as u64;
    let end = (start + page_size as u64).min(total);
    let mut consumed = vec![0u64; feeds.len()];
    let mut slots = Vec::new();
    let mut position = 0;
    // Interleave source-local order before pagination. Exhausted sources drop
    // out of the rotation, so the tail never develops empty pages or holes.
    while position < end {
        for (index, (_, reply)) in feeds.iter().enumerate() {
            if consumed[index] >= reply.total {
                continue;
            }
            if position >= start {
                slots.push((index, consumed[index]));
            }
            consumed[index] += 1;
            position += 1;
            if position >= end {
                break;
            }
        }
    }
    let mut needed = HashSet::new();
    for (index, offset) in &slots {
        needed.insert((*index, (*offset / page_size as u64 + 1) as u32));
    }
    let mut pages: HashMap<_, _> = feeds
        .iter()
        .enumerate()
        .map(|(index, (_, reply))| ((index, 1u32), reply.items.clone()))
        .collect();
    let requests = needed.into_iter().filter(|(_, page)| *page != 1).map(
        |(index, page)| {
            let source = feeds[index].0;
            let query = query.clone();
            async move {
                let reply = list_source(
                    source,
                    page,
                    query,
                    page_size,
                    default_sort(source).into(),
                )
                .await?;
                Ok::<_, crate::Error>(((index, page), reply.items))
            }
        },
    );
    for reply in join_all(requests).await {
        let (key, items) = reply?;
        pages.insert(key, items);
    }
    let mut items = Vec::with_capacity(slots.len());
    for (index, offset) in slots {
        let page = (offset / page_size as u64 + 1) as u32;
        let item = pages
            .get(&(index, page))
            .and_then(|items| items.get((offset % page_size as u64) as usize))
            .ok_or_else(|| {
                failure("The source changed during pagination. Try again")
            })?;
        items.push(item.clone());
    }
    Ok(Page {
        items,
        total,
        page_count: total.div_ceil(page_size as u64).max(1) as u32,
        failed_sources,
    })
}

fn default_sort(_source: Source) -> &'static str {
    "default"
}

async fn list_source(
    source: Source,
    page: u32,
    query: String,
    page_size: u32,
    sort: String,
) -> crate::Result<Page> {
    public::list(source, page, &query, page_size, &sort).await
}

pub async fn detail(source: Source, id: String) -> crate::Result<Detail> {
    Ok(public::detail(source, &id).await?.detail)
}

async fn acquire(
    source: Source,
    id: &str,
    file_name: &str,
) -> crate::Result<(String, Vec<u8>)> {
    let (name, bytes) = public::download(source, id, file_name).await?;
    validate_schematic(&bytes)?;
    Ok((name, bytes))
}

pub async fn preview_file(
    source: Source,
    id: &str,
    file_name: &str,
) -> crate::Result<(tempfile::TempDir, std::path::PathBuf)> {
    let (name, bytes) = acquire(source, id, file_name).await?;
    let directory = tempfile::Builder::new()
        .prefix("axolotl-schematic-")
        .tempdir()?;
    let path = directory.path().join(safe_file_name(&name)?);
    tokio::fs::write(&path, bytes).await?;
    Ok((directory, path))
}

pub async fn install(
    instance_id: String,
    source: Source,
    id: String,
    file_name: String,
) -> crate::Result<String> {
    let (name, bytes) = acquire(source, &id, &file_name).await?;
    let state = crate::State::get().await?;
    let path =
        install_verified(&instance_id, source, &id, &name, &bytes, &state)
            .await?;
    if let Err(error) =
        crate::instance::emit_content_changed(&instance_id).await
    {
        tracing::warn!(%instance_id, %error, "Installed schematic content notification failed");
    }
    Ok(path)
}

pub(crate) fn validate_schematic(bytes: &[u8]) -> crate::Result<()> {
    let mut decoded = Vec::new();
    if bytes.starts_with(&[0x1f, 0x8b]) {
        flate2::read::GzDecoder::new(bytes)
            .take(MAX_FILE_SIZE as u64 + 1)
            .read_to_end(&mut decoded)
            .map_err(|_| failure("Invalid compressed schematic"))?;
    } else {
        decoded.extend_from_slice(bytes);
    }
    if decoded.len() > MAX_FILE_SIZE {
        return Err(failure("The schematic is too large"));
    }
    let (root, _) = quartz_nbt::io::read_nbt(
        &mut std::io::Cursor::new(decoded),
        quartz_nbt::io::Flavor::Uncompressed,
    )
    .map_err(|_| failure("The downloaded file is not a schematic"))?;
    let valid = (root.contains_key("Regions") && root.contains_key("Metadata"))
        || (root.contains_key("Width")
            && root.contains_key("Height")
            && root.contains_key("Length")
            && (root.contains_key("Blocks") || root.contains_key("BlockData")))
        || root.contains_key("Schematic")
        || (root.contains_key("blocks")
            && root.contains_key("palette")
            && root.contains_key("size"));
    if !valid {
        return Err(failure("The downloaded file is not a schematic"));
    }
    Ok(())
}

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSchematic {
    pub source: String,
    pub project_id: String,
    pub file_name: String,
}

pub async fn installed(
    instance_id: &str,
) -> crate::Result<Vec<InstalledSchematic>> {
    let state = crate::State::get().await?;
    sqlx::query_as::<_, InstalledSchematic>("SELECT source.source, source.project_id, source.file_name FROM instance_schematic_sources source INNER JOIN instance_files file ON source.file_id = file.id WHERE file.instance_id = ? AND file.missing = 0 AND file.sha1 = source.sha1")
		.bind(instance_id).fetch_all(&state.pool).await.map_err(Into::into)
}

async fn install_verified(
    instance_id: &str,
    source: Source,
    project_id: &str,
    file_name: &str,
    bytes: &[u8],
    state: &crate::State,
) -> crate::Result<String> {
    use crate::state::instances::adapters::sqlite::content_rows;
    use crate::state::instances::commands::{
        instance_content_root, resolve_content_scope,
    };
    use crate::state::{
        ContentOwnershipKind, ContentRequirement, ContentSourceKind,
        ProjectType,
    };
    let file_name = safe_file_name(file_name)?;
    let _maintenance =
        crate::instance::lock_instance_maintenance(instance_id).await;
    if crate::instance::has_active_instance_operation(instance_id) {
        return Err(failure(
            "Wait for the instance backup operation to finish",
        ));
    }
    let _content = state.lock_instance_content(instance_id).await;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let instance = crate::state::get_instance(instance_id, &state.pool)
        .await?
        .ok_or_else(|| failure("Unknown instance"))?;
    if matches!(
        instance.link,
        crate::state::InstanceLink::ServerProject { .. }
            | crate::state::InstanceLink::ServerProjectModpack { .. }
    ) {
        return Err(failure("Schematics require a client instance"));
    }
    if scope.instance.install_stage
        != crate::state::InstanceInstallStage::Installed
    {
        return Err(failure("The instance must finish installing first"));
    }
    let root = instance_content_root(&state.directories, &scope.instance)?;
    let parent = root.join("schematics");
    tokio::fs::create_dir_all(&parent).await?;
    let parent = crate::util::io::canonicalize(&parent)?;
    if !parent.starts_with(&root) {
        return Err(failure(
            "The schematic directory points outside the instance",
        ));
    }
    let relative = format!("schematics/{file_name}");
    let destination = parent.join(&file_name);
    let hash = crate::util::fetch::sha1_async(bytes.to_vec().into()).await?;
    let temporary = tempfile::Builder::new()
        .prefix(".ymcl-schematic-")
        .tempfile_in(&parent)?;
    tokio::fs::write(temporary.path(), bytes).await?;
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    content_rows::ensure_content_write_parents(
        instance_id,
        &scope.content_set_id,
        &mut tx,
    )
    .await?;
    let file = content_rows::upsert_instance_file_from_parts_in_transaction(
        content_rows::UpsertInstanceFile {
            instance_id,
            relative_path: &relative,
            file_name: &file_name,
            enabled: true,
            sha1: &hash,
            size: bytes.len() as u64,
            missing: false,
            local_mod_data: None,
            icon_path: None,
        },
        &mut tx,
    )
    .await?;
    content_rows::upsert_content_entry_from_parts_in_transaction(
        content_rows::UpsertContentEntry {
            instance_id,
            content_set_id: &scope.content_set_id,
            file_id: Some(&file.id),
            project_type: ProjectType::Schematic,
            source_kind: ContentSourceKind::Local,
            ownership_kind: ContentOwnershipKind::UserAdded,
            auto_dependency: false,
            server_requirement: ContentRequirement::Unknown,
            client_requirement: ContentRequirement::Required,
            enabled: true,
        },
        &mut tx,
    )
    .await?;
    sqlx::query("INSERT INTO instance_schematic_sources(file_id, source, project_id, file_name, sha1) VALUES (?, ?, ?, ?, ?) ON CONFLICT(file_id) DO UPDATE SET source = excluded.source, project_id = excluded.project_id, file_name = excluded.file_name, sha1 = excluded.sha1")
		.bind(&file.id).bind(source.id()).bind(project_id).bind(&file_name).bind(&hash).execute(&mut *tx).await?;
    temporary.persist_noclobber(&destination).map_err(|error| {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            failure("schematic_target_exists")
        } else {
            error.error.into()
        }
    })?;
    if let Err(error) = tx.commit().await {
        let _ = tokio::fs::remove_file(destination).await;
        return Err(error.into());
    }
    Ok(relative)
}

fn safe_file_name(name: &str) -> crate::Result<String> {
    if name.is_empty()
        || name.len() > 240
        || name.trim() != name
        || name.starts_with('.')
        || name
            .chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
        || name.ends_with('.')
        || name.ends_with(' ')
    {
        return Err(failure("Invalid schematic filename"));
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.ends_with(['1', '2', '3', '4', '5', '6', '7', '8', '9']))
    {
        return Err(failure("Invalid schematic filename"));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CreateDirectLinkInstance, DirectoryInfo};

    fn schematic_bytes(compressed: bool) -> Vec<u8> {
        let mut root = quartz_nbt::NbtCompound::new();
        root.insert("Width", 1i16);
        root.insert("Height", 1i16);
        root.insert("Length", 1i16);
        root.insert("Blocks", quartz_nbt::NbtTag::ByteArray(vec![1]));
        let mut bytes = Vec::new();
        quartz_nbt::io::write_nbt(
            &mut bytes,
            None,
            &root,
            if compressed {
                quartz_nbt::io::Flavor::GzCompressed
            } else {
                quartz_nbt::io::Flavor::Uncompressed
            },
        )
        .unwrap();
        bytes
    }

    #[test]
    fn validates_download_payload_and_cross_platform_file_names() {
        assert!(validate_schematic(&schematic_bytes(false)).is_ok());
        assert!(validate_schematic(&schematic_bytes(true)).is_ok());
        for bytes in [
            b"<html>error</html>".as_slice(),
            &[0x1f, 0x8b, 0, 0],
            b"not nbt",
        ] {
            assert!(validate_schematic(bytes).is_err());
        }
        let mut empty = Vec::new();
        quartz_nbt::io::write_nbt(
            &mut empty,
            None,
            &quartz_nbt::NbtCompound::new(),
            quartz_nbt::io::Flavor::Uncompressed,
        )
        .unwrap();
        assert!(validate_schematic(&empty).is_err());
        for name in [
            "../bad.schem",
            "C:\\bad.schem",
            "bad:stream",
            "CON.schem",
            "LPT1.schem",
            "bad\0.schem",
            ".hidden.schem",
            "bad.schem ",
        ] {
            assert!(safe_file_name(name).is_err(), "{name:?}");
        }
        assert_eq!(safe_file_name("建筑.schem").unwrap(), "建筑.schem");
    }

    #[tokio::test]
    async fn installs_source_identity_without_clobbering_existing_files() {
        let root = tempfile::tempdir().unwrap();
        let directories = DirectoryInfo {
            settings_dir: root.path().join("settings"),
            config_dir: root.path().join("config"),
            app_identifier: "test".into(),
        };
        std::fs::create_dir_all(directories.instances_dir()).unwrap();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let state = crate::state::test_state(directories, pool).await.unwrap();
        let minecraft = root.path().join("minecraft");
        std::fs::create_dir_all(minecraft.join("versions/test")).unwrap();
        std::fs::write(
            minecraft.join("versions/test/test.json"),
            br#"{"id":"1.20.1","mainClass":"net.minecraft.client.main.Main"}"#,
        )
        .unwrap();
        let instance = crate::state::create_direct_link_instance(
            CreateDirectLinkInstance {
                name: None,
                launcher_type:
                    crate::api::pack::import::ImportLauncherType::Generic,
                base_path: minecraft.clone(),
                instance_folder: "versions/test".into(),
                instance_path: None,
                game_dir_mode: Some(
                    crate::launcher::ExternalGameDirMode::Shared,
                ),
            },
            &state,
        )
        .await
        .unwrap();
        let bytes = schematic_bytes(true);
        assert_eq!(
            install_verified(
                &instance.id,
                Source::Reden,
                "123",
                "test.schem",
                &bytes,
                &state
            )
            .await
            .unwrap(),
            "schematics/test.schem"
        );
        let identity: (String, String) = sqlx::query_as(
            "SELECT source, project_id FROM instance_schematic_sources",
        )
        .fetch_one(&state.pool)
        .await
        .unwrap();
        assert_eq!(identity, ("reden".into(), "123".into()));
        assert!(
            install_verified(
                &instance.id,
                Source::BuildSchematics,
                "different",
                "test.schem",
                b"replacement",
                &state
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("schematic_target_exists")
        );
        assert_eq!(
            std::fs::read(minecraft.join("schematics/test.schem")).unwrap(),
            bytes
        );
        let identity: (String, String) = sqlx::query_as(
            "SELECT source, project_id FROM instance_schematic_sources",
        )
        .fetch_one(&state.pool)
        .await
        .unwrap();
        assert_eq!(identity, ("reden".into(), "123".into()));
        let file_id: String = sqlx::query_scalar(
            "SELECT file_id FROM instance_schematic_sources",
        )
        .fetch_one(&state.pool)
        .await
        .unwrap();
        sqlx::query("DELETE FROM instance_files WHERE id = ?")
            .bind(file_id)
            .execute(&state.pool)
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM instance_schematic_sources",
        )
        .fetch_one(&state.pool)
        .await
        .unwrap();
        assert_eq!(count, 0);
        assert!(
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&state.pool)
                .await
                .unwrap()
                .is_empty()
        );
        #[cfg(unix)]
        {
            std::fs::remove_dir_all(minecraft.join("schematics")).unwrap();
            let outside = root.path().join("outside");
            std::fs::create_dir(&outside).unwrap();
            std::os::unix::fs::symlink(&outside, minecraft.join("schematics"))
                .unwrap();
            assert!(
                install_verified(
                    &instance.id,
                    Source::Reden,
                    "456",
                    "escape.schem",
                    &bytes,
                    &state
                )
                .await
                .is_err()
            );
            assert!(!outside.join("escape.schem").exists());
        }
    }
}

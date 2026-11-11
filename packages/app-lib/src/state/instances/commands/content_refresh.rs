use std::collections::HashSet;
use std::future::Future;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::State;
use crate::state::CacheBehaviour;
use crate::state::instances::InstanceFile;
use crate::state::instances::adapters::{
    filesystem::ScannedContentFile, sqlite::instance_rows,
};

static ACTIVE: LazyLock<DashMap<String, CancellationToken>> =
    LazyLock::new(DashMap::new);
static WORKERS: Semaphore = Semaphore::const_new(2);
const ONLINE_REFRESH_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ContentRefreshJob {
    pub instance_id: String,
    pub id: String,
    pub state: String,
    pub total: i64,
    pub processed: i64,
    pub failed: i64,
    pub error: Option<String>,
    pub updated_at: i64,
    #[sqlx(skip)]
    pub failures: Vec<ContentRefreshFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentRefreshFailure {
    pub path: String,
    pub error: String,
}

pub(crate) struct RefreshContext {
    instance_id: String,
    pool: SqlitePool,
    cancel: CancellationToken,
    retry_paths: Option<HashSet<String>>,
}

impl RefreshContext {
    async fn run_online_refresh(
        &self,
        work: impl Future<Output = crate::Result<()>>,
        timeout: Duration,
    ) -> crate::Result<()> {
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => self.check_cancelled(),
            result = tokio::time::timeout(timeout, work) => match result {
                Ok(result) => result,
                Err(_) => {
                    let message = "Online content information timed out. Local files and dependencies are available. Retry unfinished files to try again.";
                    sqlx::query("UPDATE instance_content_refresh_files SET state = 'failed', error = ? WHERE instance_id = ? AND state = 'local_ready'")
                        .bind(message).bind(&self.instance_id).execute(&self.pool).await?;
                    Err(crate::ErrorKind::InputError(message.into()).into())
                }
            },
        }
    }

    pub(crate) fn check_cancelled(&self) -> crate::Result<()> {
        if self.cancel.is_cancelled() {
            return Err(crate::ErrorKind::InputError(
                "Content scan cancelled".into(),
            )
            .into());
        }
        Ok(())
    }

    pub(crate) fn includes(&self, path: &str) -> bool {
        self.retry_paths
            .as_ref()
            .is_none_or(|paths| paths.contains(path))
    }

    pub(crate) fn is_retry(&self) -> bool {
        self.retry_paths.is_some()
    }

    pub(crate) async fn begin_files(
        &self,
        files: &[ScannedContentFile],
    ) -> crate::Result<()> {
        let paths = files
            .iter()
            .filter(|file| self.includes(&file.relative_path))
            .collect::<Vec<_>>();
        let mut tx = self.pool.begin().await?;
        for file in &paths {
            sqlx::query("INSERT INTO instance_content_refresh_files (instance_id, relative_path, state) VALUES (?, ?, 'pending') ON CONFLICT(instance_id, relative_path) DO UPDATE SET state = 'pending', error = NULL")
				.bind(&self.instance_id).bind(&file.relative_path).execute(&mut *tx).await?;
        }
        if let Some(retry_paths) = &self.retry_paths {
            let present = paths
                .iter()
                .map(|file| file.relative_path.as_str())
                .collect::<HashSet<_>>();
            for path in retry_paths
                .iter()
                .filter(|path| !present.contains(path.as_str()))
            {
                sqlx::query("UPDATE instance_content_refresh_files SET state = 'failed', error = 'File is no longer accessible' WHERE instance_id = ? AND relative_path = ?")
					.bind(&self.instance_id).bind(path).execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        self.set_stage("scanning", None).await?;
        self.update_counts().await
    }

    pub(crate) async fn file_failed(
        &self,
        path: &str,
        error: &str,
    ) -> crate::Result<()> {
        sqlx::query("UPDATE instance_content_refresh_files SET state = 'failed', error = ? WHERE instance_id = ? AND relative_path = ?")
			.bind(error).bind(&self.instance_id).bind(path).execute(&self.pool).await?;
        Ok(())
    }

    pub(crate) async fn local_ready(
        &self,
        files: &[InstanceFile],
    ) -> crate::Result<()> {
        let mut tx = self.pool.begin().await?;
        for file in files {
            sqlx::query("UPDATE instance_content_refresh_files SET state = 'local_ready', error = NULL WHERE instance_id = ? AND relative_path = ?")
				.bind(&self.instance_id).bind(&file.relative_path).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        self.update_counts().await?;
        self.notify().await;
        Ok(())
    }

    async fn set_stage(
        &self,
        stage: &str,
        error: Option<&str>,
    ) -> crate::Result<()> {
        sqlx::query("UPDATE instance_content_refreshes SET state = ?, error = ?, updated_at = ? WHERE instance_id = ?")
			.bind(stage).bind(error).bind(chrono::Utc::now().timestamp()).bind(&self.instance_id).execute(&self.pool).await?;
        Ok(())
    }

    async fn update_counts(&self) -> crate::Result<()> {
        sqlx::query("UPDATE instance_content_refreshes SET total = (SELECT COUNT(*) FROM instance_content_refresh_files WHERE instance_id = ?), processed = (SELECT COUNT(*) FROM instance_content_refresh_files WHERE instance_id = ? AND state IN ('completed', 'local_ready', 'failed')), failed = (SELECT COUNT(*) FROM instance_content_refresh_files WHERE instance_id = ? AND state = 'failed'), updated_at = ? WHERE instance_id = ?")
			.bind(&self.instance_id).bind(&self.instance_id).bind(&self.instance_id).bind(chrono::Utc::now().timestamp()).bind(&self.instance_id).execute(&self.pool).await?;
        Ok(())
    }

    async fn notify(&self) {
        let revision = sqlx::query_scalar::<_, i64>("UPDATE instance_content_sets SET revision = revision + 1 WHERE id = (SELECT applied_content_set_id FROM instances WHERE id = ?) RETURNING revision")
			.bind(&self.instance_id).fetch_optional(&self.pool).await.ok().flatten().unwrap_or(0) as u64;
        if let Err(error) = crate::event::emit::emit_instance(
            &self.instance_id,
            crate::event::InstancePayloadType::ContentChanged { revision },
        )
        .await
        {
            tracing::debug!(%error, "Content refresh listener is unavailable");
        }
    }
}

pub async fn get_content_refresh(
    instance_id: &str,
) -> crate::Result<Option<ContentRefreshJob>> {
    let state = State::get().await?;
    let mut job = sqlx::query_as::<_, ContentRefreshJob>(
        "SELECT * FROM instance_content_refreshes WHERE instance_id = ?",
    )
    .bind(instance_id)
    .fetch_optional(&state.pool)
    .await?;
    if let Some(job) = job.as_mut() {
        job.failures = sqlx::query("SELECT relative_path, error FROM instance_content_refresh_files WHERE instance_id = ? AND state = 'failed' ORDER BY relative_path")
			.bind(instance_id).fetch_all(&state.pool).await?.into_iter().map(|row| Ok(ContentRefreshFailure {
				path: row.try_get("relative_path")?, error: row.try_get::<Option<String>, _>("error")?.unwrap_or_default(),
			})).collect::<crate::Result<Vec<_>>>()?;
    }
    Ok(job)
}

pub async fn start_content_refresh(
    instance_id: &str,
    retry_failed: bool,
) -> crate::Result<ContentRefreshJob> {
    let state = State::get().await?;
    let instance = instance_rows::get_instance_by_id(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".into())
        })?;
    if instance.install_stage != crate::state::InstanceInstallStage::Installed {
        return Err(crate::ErrorKind::InputError(
            "Instance must be installed before scanning content".into(),
        )
        .into());
    }
    let cancel = CancellationToken::new();
    match ACTIVE.entry(instance_id.to_string()) {
        dashmap::mapref::entry::Entry::Occupied(entry) => {
            drop(entry);
            return get_content_refresh(instance_id).await?.ok_or_else(|| {
                crate::ErrorKind::InputError("Content scan is starting".into())
                    .into()
            });
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            entry.insert(cancel.clone());
        }
    }
    let initialized: crate::Result<_> = async {
		let retry_paths = if retry_failed {
			let paths = sqlx::query_scalar::<_, String>("SELECT relative_path FROM instance_content_refresh_files WHERE instance_id = ? AND state != 'completed'")
				.bind(instance_id).fetch_all(&state.pool).await?.into_iter().collect::<HashSet<_>>();
			if paths.is_empty() { None } else { Some(paths) }
		} else { None };
		let mut tx = state.pool.begin().await?;
		if retry_paths.is_none() {
			sqlx::query("DELETE FROM instance_content_refresh_files WHERE instance_id = ?").bind(instance_id).execute(&mut *tx).await?;
		}
		sqlx::query("INSERT INTO instance_content_refreshes (instance_id, id, state, updated_at) VALUES (?, ?, 'queued', ?) ON CONFLICT(instance_id) DO UPDATE SET id = excluded.id, state = 'queued', error = NULL, updated_at = excluded.updated_at")
			.bind(instance_id).bind(uuid::Uuid::new_v4().to_string()).bind(chrono::Utc::now().timestamp()).execute(&mut *tx).await?;
		tx.commit().await?;
		Ok(retry_paths)
	}.await;
    let retry_paths = match initialized {
        Ok(paths) => paths,
        Err(error) => {
            ACTIVE.remove(instance_id);
            return Err(error);
        }
    };
    let context = Arc::new(RefreshContext {
        instance_id: instance_id.to_string(),
        pool: state.pool.clone(),
        cancel,
        retry_paths,
    });
    let initial = match async {
        context.update_counts().await?;
        get_content_refresh(instance_id).await?.ok_or_else(|| {
            crate::ErrorKind::InputError("Content scan disappeared".into())
                .into()
        })
    }
    .await
    {
        Ok(job) => job,
        Err(error) => {
            ACTIVE.remove(instance_id);
            return Err(error);
        }
    };
    tokio::spawn(async move {
        let result = async {
			let _permit = tokio::select! {
				permit = WORKERS.acquire() => permit.map_err(|error| crate::ErrorKind::InputError(error.to_string()))?,
				_ = context.cancel.cancelled() => { context.check_cancelled()?; unreachable!() }
			};
			super::sync_content_files::sync_instance_content_files_with_progress(&instance, &state, Some(&context)).await?;
			// Publish local dependency relationships before waiting for any provider.
			super::content_snapshot::get_content_snapshot(&context.instance_id, false, &state).await?;
			context.set_stage("enriching", None).await?;
			context.run_online_refresh(refresh_online_content(&context, &state), ONLINE_REFRESH_TIMEOUT).await?;
			Ok::<(), crate::Error>(())
		}.await;
        let failed = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM instance_content_refresh_files WHERE instance_id = ? AND state = 'failed'").bind(&context.instance_id).fetch_one(&context.pool).await.unwrap_or(1);
        let status = if context.cancel.is_cancelled() {
            "cancelled"
        } else if result.is_err() {
            "failed"
        } else if failed > 0 {
            "partial"
        } else {
            "completed"
        };
        let error = result.err().map(|error| error.to_string());
        if let Err(error) = context.set_stage(status, error.as_deref()).await {
            tracing::warn!(%error, "Failed to save content scan state");
        }
        let _ = context.update_counts().await;
        ACTIVE.remove(&context.instance_id);
        context.notify().await;
    });
    Ok(initial)
}

async fn refresh_online_content(
    context: &RefreshContext,
    state: &State,
) -> crate::Result<()> {
    let instance =
        instance_rows::get_instance_by_id(&context.instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError("Unknown instance".into())
            })?;
    let files =
        super::super::adapters::sqlite::content_rows::get_instance_files(
            &context.instance_id,
            &state.pool,
        )
        .await?;
    let mut provider_errors = Vec::new();
    if let Err(error) =
        super::recognize_content_files::recognize_curseforge_content_files(
            &instance, &files, state,
        )
        .await
    {
        provider_errors.push(error.to_string());
    }
    let paths = sqlx::query_scalar::<_, String>("SELECT relative_path FROM instance_content_refresh_files WHERE instance_id = ? AND state = 'local_ready' ORDER BY relative_path")
        .bind(&context.instance_id).fetch_all(&context.pool).await?;
    for paths in paths.chunks(8) {
        context.check_cancelled()?;
        let batch = super::list_content::refresh_content_by_paths(
            &context.instance_id,
            paths,
            Some(CacheBehaviour::MustRevalidate),
            state,
        )
        .await;
        for path in paths {
            context.check_cancelled()?;
            let result = if batch.is_ok() {
                Ok(())
            } else {
                super::list_content::refresh_content_by_paths(
                    &context.instance_id,
                    std::slice::from_ref(path),
                    Some(CacheBehaviour::MustRevalidate),
                    state,
                )
                .await
                .map(|_| ())
            };
            match result {
                Ok(()) => {
                    sqlx::query("UPDATE instance_content_refresh_files SET state = 'completed', error = NULL WHERE instance_id = ? AND relative_path = ?")
                        .bind(&context.instance_id).bind(path).execute(&context.pool).await?;
                }
                Err(error) => {
                    context.file_failed(path, &error.to_string()).await?
                }
            }
        }
        context.update_counts().await?;
        context.notify().await;
    }
    context.check_cancelled()?;
    if let Err(error) =
        super::curseforge_content_updates::refresh_curseforge_content_updates(
            &instance, &files, state,
        )
        .await
    {
        provider_errors.push(error.to_string());
    }
    if let Err(error) = super::content_snapshot::finish_content_refresh(
        &context.instance_id,
        state,
    )
    .await
    {
        provider_errors.push(error.to_string());
    }
    if provider_errors.is_empty() {
        Ok(())
    } else {
        Err(crate::ErrorKind::InputError(provider_errors.join("\n")).into())
    }
}

pub async fn cancel_content_refresh(instance_id: &str) -> crate::Result<()> {
    if let Some(token) = ACTIVE.get(instance_id) {
        token.cancel();
    }
    Ok(())
}

pub(crate) async fn recover_content_refreshes(
    pool: &SqlitePool,
) -> crate::Result<()> {
    sqlx::query("UPDATE instance_content_refreshes SET state = 'interrupted', error = 'The launcher closed before the content scan finished', updated_at = ? WHERE state IN ('queued', 'scanning', 'enriching')")
		.bind(chrono::Utc::now().timestamp()).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CreateDirectLinkInstance, DirectoryInfo, Instance};

    #[tokio::test]
    async fn cached_snapshot_links_local_dependencies_before_online_enrichment()
    {
        use std::io::Write;
        let (root, state, instance, context) = fixture().await;
        for (name, metadata) in [
            (
                "parent",
                r#"[{"modid":"parent","version":"1","useDependencyInformation":true,"requiredMods":["library"]}]"#,
            ),
            ("library", r#"[{"modid":"library","version":"1"}]"#),
        ] {
            let file = std::fs::File::create(
                root.path().join(format!("minecraft/mods/{name}.jar")),
            )
            .unwrap();
            let mut archive = zip::ZipWriter::new(file);
            archive
                .start_file(
                    "mcmod.info",
                    zip::write::FileOptions::<()>::default(),
                )
                .unwrap();
            archive.write_all(metadata.as_bytes()).unwrap();
            archive.finish().unwrap();
        }
        super::super::sync_content_files::sync_instance_content_files_with_progress(&instance, &state, Some(&context)).await.unwrap();
        let snapshot = super::super::content_snapshot::get_content_snapshot(
            &instance.id,
            false,
            &state,
        )
        .await
        .unwrap();
        let parent = snapshot
            .items
            .iter()
            .find(|item| item.expected_relative_path == "mods/parent.jar")
            .unwrap();
        let dependency = parent.dependency.as_ref().unwrap();
        assert_eq!(dependency.requires.len(), 1);
        assert_eq!(dependency.requires[0].project_id, "local:library");
        let library = snapshot
            .items
            .iter()
            .find(|item| item.expected_relative_path == "mods/library.jar")
            .unwrap();
        assert_eq!(dependency.requires[0].entry_id, library.entry_id);
        assert_eq!(
            dependency.requires[0].provider,
            crate::state::ContentProvider::Local
        );
        assert!(library.entry_id.is_none());
        assert_eq!(dependency.requires[0].file_id, library.file_id);
        assert_eq!(
            library.ownership_kind,
            crate::state::instances::ContentOwnershipKind::LocalDiscovered
        );
        let persisted: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM instance_content_entries WHERE content_set_id = (SELECT applied_content_set_id FROM instances WHERE id = ?)")
            .bind(&instance.id).fetch_one(&state.pool).await.unwrap();
        assert_eq!(
            persisted, 0,
            "reading local dependencies must not change content ownership"
        );
    }

    #[tokio::test]
    async fn stalled_online_refresh_preserves_local_files_and_retry_paths() {
        let (root, state, instance, context) = fixture().await;
        std::fs::write(root.path().join("minecraft/mods/local.jar"), b"local")
            .unwrap();
        let files = super::super::sync_content_files::sync_instance_content_files_with_progress(&instance, &state, Some(&context)).await.unwrap();
        let result = context
            .run_online_refresh(
                std::future::pending(),
                Duration::from_millis(1),
            )
            .await;
        assert!(result.unwrap_err().to_string().contains("timed out"));
        let failed: (String, String) = sqlx::query_as("SELECT state, error FROM instance_content_refresh_files WHERE relative_path = 'mods/local.jar'")
            .fetch_one(&state.pool).await.unwrap();
        assert_eq!(failed.0, "failed");
        assert!(
            failed
                .1
                .contains("Local files and dependencies are available")
        );
        let stored = crate::state::instances::adapters::sqlite::content_rows::get_instance_files(&instance.id, &state.pool).await.unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].sha1, files[0].sha1);
        assert!(!crate::mod_metadata::metadata_cache_needs_refresh(
            stored[0].local_mod_data.as_deref()
        ));
    }

    #[tokio::test]
    async fn online_refresh_cancellation_does_not_wait_for_the_provider() {
        let (_root, _state, _instance, context) = fixture().await;
        context.cancel.cancel();
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            context.run_online_refresh(
                std::future::pending(),
                ONLINE_REFRESH_TIMEOUT,
            ),
        )
        .await
        .unwrap();
        assert!(result.unwrap_err().to_string().contains("cancelled"));
    }

    async fn fixture()
    -> (tempfile::TempDir, Arc<State>, Instance, RefreshContext) {
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
        std::fs::create_dir_all(minecraft.join("mods")).unwrap();
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
                base_path: minecraft,
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
        sqlx::query("INSERT INTO instance_content_refreshes (instance_id, id, state, updated_at) VALUES (?, 'test', 'queued', 0)")
			.bind(&instance.id).execute(&state.pool).await.unwrap();
        let context = RefreshContext {
            instance_id: instance.id.clone(),
            pool: state.pool.clone(),
            cancel: CancellationToken::new(),
            retry_paths: None,
        };
        (root, state, instance, context)
    }

    #[tokio::test]
    async fn cancelled_scan_recovers_pending_files_without_losing_committed_batches()
     {
        let (root, state, instance, mut context) = fixture().await;
        for index in 0..15 {
            std::fs::write(
                root.path().join(format!("minecraft/mods/{index}.jar")),
                format!("content {index}"),
            )
            .unwrap();
        }
        context.cancel.cancel();
        assert!(super::super::sync_content_files::sync_instance_content_files_with_progress(&instance, &state, Some(&context)).await.is_err());
        recover_content_refreshes(&state.pool).await.unwrap();
        let status: String =
            sqlx::query_scalar("SELECT state FROM instance_content_refreshes")
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert_eq!(status, "interrupted");
        let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM instance_content_refresh_files WHERE state = 'pending'").fetch_one(&state.pool).await.unwrap();
        assert_eq!(pending, 15);
        context.cancel = CancellationToken::new();
        context.retry_paths = Some(
            sqlx::query_scalar(
                "SELECT relative_path FROM instance_content_refresh_files",
            )
            .fetch_all(&state.pool)
            .await
            .unwrap()
            .into_iter()
            .collect(),
        );
        let files = super::super::sync_content_files::sync_instance_content_files_with_progress(&instance, &state, Some(&context)).await.unwrap();
        assert_eq!(files.len(), 15);
        let counts: (i64, i64, i64) = sqlx::query_as(
            "SELECT total, processed, failed FROM instance_content_refreshes",
        )
        .fetch_one(&state.pool)
        .await
        .unwrap();
        assert_eq!(counts, (15, 15, 0));
        let ready: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM instance_content_refresh_files WHERE state = 'local_ready'").fetch_one(&state.pool).await.unwrap();
        assert_eq!(ready, 15);
        assert!(files.iter().all(|file| !file.missing && file.size > 0));
        assert!(
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&state.pool)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn retry_preserves_completed_files_and_reports_unavailable_paths() {
        let (root, state, instance, mut context) = fixture().await;
        let mods = root.path().join("minecraft/mods");
        std::fs::write(mods.join("complete.jar"), b"complete").unwrap();
        std::fs::write(mods.join("retry.jar"), b"retry").unwrap();
        let before = super::super::sync_content_files::sync_instance_content_files_with_progress(&instance, &state, Some(&context)).await.unwrap();
        sqlx::query("UPDATE instance_content_refresh_files SET state = 'completed' WHERE relative_path = 'mods/complete.jar'").execute(&state.pool).await.unwrap();
        context
            .file_failed("mods/retry.jar", "temporary failure")
            .await
            .unwrap();
        std::fs::remove_file(mods.join("complete.jar")).unwrap();
        context.retry_paths =
            Some(HashSet::from(["mods/retry.jar".to_string()]));
        let retried = super::super::sync_content_files::sync_instance_content_files_with_progress(&instance, &state, Some(&context)).await.unwrap();
        assert_eq!(retried.len(), 1);
        assert_eq!(
            retried[0].id,
            before
                .iter()
                .find(|file| file.relative_path == "mods/retry.jar")
                .unwrap()
                .id
        );
        let preserved: (String, bool) = sqlx::query_as("SELECT progress.state, file.missing FROM instance_content_refresh_files progress JOIN instance_files file USING (instance_id, relative_path) WHERE progress.relative_path = 'mods/complete.jar'").fetch_one(&state.pool).await.unwrap();
        assert_eq!(preserved, ("completed".into(), false));
        std::fs::remove_file(mods.join("retry.jar")).unwrap();
        super::super::sync_content_files::sync_instance_content_files_with_progress(&instance, &state, Some(&context)).await.unwrap();
        let counts: (i64, i64, i64) = sqlx::query_as(
            "SELECT total, processed, failed FROM instance_content_refreshes",
        )
        .fetch_one(&state.pool)
        .await
        .unwrap();
        assert_eq!(counts, (2, 2, 1));
        context.set_stage("partial", None).await.unwrap();
        recover_content_refreshes(&state.pool).await.unwrap();
        let status: String =
            sqlx::query_scalar("SELECT state FROM instance_content_refreshes")
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert_eq!(status, "partial");
    }
}

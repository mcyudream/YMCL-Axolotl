//! Apply pipeline (MIP WF-5 steps 7-9): materializes an [`UpdatePlan`]
//! against an instance directory.
//!
//! Transaction model:
//! 1. **Stage** (zero-risk): download every needed object into
//!    `.pack-staging/`, verifying sha512; copy `Move` sources there too.
//!    Any failure aborts with the instance untouched.
//! 2. **Backup**: instance files that will be overwritten or deleted are
//!    copied into `.pack-backup/`.
//! 3. **Commit**: move staged content in, apply deletions. On failure the
//!    backup is restored and the error surfaced; on success the backup and
//!    staging are removed and the caller writes the new pack state.

use async_trait::async_trait;
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::diff::{ChangeAction, UpdatePlan};
use super::manifest::{MipFileEntry, MipManifest};
use super::state::MipPackState;

pub const STAGING_DIR_NAME: &str = ".pack-staging";
pub const BACKUP_DIR_NAME: &str = ".pack-backup";
pub const PACK_NEW_SUFFIX: &str = ".pack-new";

/// Source of pack objects. Production implementations download over HTTP
/// (MIP sources, CAS); tests use in-memory maps.
#[async_trait]
pub trait ObjectFetcher: Send + Sync {
    /// Fetches the bytes for a manifest entry, trying its sources in order.
    /// Returns `Err` when every source fails or any checksum mismatches.
    async fn fetch(&self, entry: &MipFileEntry) -> crate::Result<Vec<u8>>;
}

fn sha512_hex(bytes: &[u8]) -> String {
    let digest = Sha512::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub struct ApplyOutcome {
    pub new_state: MipPackState,
    pub staged_files: usize,
    pub deleted_files: usize,
}

/// Applies `plan` to `instance_dir`. `entry_by_path` maps each planned
/// change path to its target manifest entry. The new state is returned; the
/// caller persists it ( WF-5 step 9 ) via [`super::state::save`].
pub async fn apply_update<F>(
    instance_dir: &Path,
    base: &MipPackState,
    target: &MipManifest,
    plan: &UpdatePlan,
    entry_by_path: &HashMap<String, MipFileEntry>,
    fetcher: &F,
) -> crate::Result<ApplyOutcome>
where
    F: ObjectFetcher + ?Sized,
{
    target.validate()?;

    let staging = instance_dir.join(STAGING_DIR_NAME);
    let backup = instance_dir.join(BACKUP_DIR_NAME);

    // Clean leftovers from an interrupted run before starting (WF-4/5
    // reentrancy): stale staging/backup dirs are reconstructible.
    if staging.exists() || backup.exists() {
        tokio::fs::remove_dir_all(&staging).await.ok();
        tokio::fs::remove_dir_all(&backup).await.ok();
    }
    tokio::fs::create_dir_all(&staging).await?;

    let stage_result = stage_all(
        instance_dir,
        &staging,
        base,
        target,
        plan,
        entry_by_path,
        fetcher,
    )
    .await;
    if let Err(error) = stage_result {
        tokio::fs::remove_dir_all(&staging).await.ok();
        return Err(error);
    }

    // Backup phase: every instance file about to be overwritten or deleted.
    let mut backed_up: Vec<PathBuf> = Vec::new();
    let backup_result = backup_targets(instance_dir, &backup, plan).await;
    if let Err(error) = backup_result {
        restore_backup(instance_dir, &backup).await;
        tokio::fs::remove_dir_all(&staging).await.ok();
        return Err(error);
    }
    backed_up.push(backup.clone());

    // Commit phase: move staged content in, then delete.
    let commit_result = commit(instance_dir, &staging, plan).await;
    if let Err(error) = commit_result {
        restore_backup(instance_dir, &backup).await;
        tokio::fs::remove_dir_all(&staging).await.ok();
        tokio::fs::remove_dir_all(&backup).await.ok();
        return Err(error);
    }

    tokio::fs::remove_dir_all(&staging).await.ok();
    tokio::fs::remove_dir_all(&backup).await.ok();

    Ok(ApplyOutcome {
        new_state: build_new_state(base, target, plan, entry_by_path),
        staged_files: plan.changes.len(),
        deleted_files: plan.deletions.len(),
    })
}

/// Stage 1: download / copy every planned target into the staging dir.
async fn stage_all<F>(
    instance_dir: &Path,
    staging: &Path,
    base: &MipPackState,
    target: &MipManifest,
    plan: &UpdatePlan,
    entry_by_path: &HashMap<String, MipFileEntry>,
    fetcher: &F,
) -> crate::Result<()>
where
    F: ObjectFetcher + ?Sized,
{
    for change in &plan.changes {
        let Some(entry) = entry_by_path.get(&change.path) else {
            return Err(crate::ErrorKind::OtherError(format!(
                "Missing manifest entry for planned change {}",
                change.path
            ))
            .into());
        };

        let destination = staging.join(sanitize(&change.path)?);
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        match &change.action {
            ChangeAction::Move { from } => {
                // Local rename semantics: copy the local source (zero
                // download). If the content also changed, the downloaded
                // bytes replace the copied local content (MIP §6 挪+改).
                let source = instance_dir.join(sanitize(from)?);
                if source.exists() {
                    tokio::fs::copy(&source, &destination).await?;
                }
                if base
                    .files
                    .get(from.as_str())
                    .is_none_or(|base_file| base_file.sha512 != entry.sha512)
                {
                    let bytes = verify(
                        fetcher.fetch(entry).await?,
                        &entry.sha512,
                        &entry.path,
                    )?;
                    tokio::fs::write(&destination, bytes).await?;
                }
            }
            ChangeAction::SeedIfMissing => {
                // seed never overwrites: only stage when absent on disk.
                if !instance_dir.join(sanitize(&change.path)?).exists() {
                    let bytes = verify(
                        fetcher.fetch(entry).await?,
                        &entry.sha512,
                        &entry.path,
                    )?;
                    tokio::fs::write(&destination, bytes).await?;
                }
            }
            ChangeAction::MergeDegrade => {
                // MIP §9.2 degradation: keep ours, write theirs alongside as
                // `<path>.pack-new`.
                let bytes = verify(
                    fetcher.fetch(entry).await?,
                    &entry.sha512,
                    &entry.path,
                )?;
                let theirs = staging.join(sanitize(&format!(
                    "{path}{PACK_NEW_SUFFIX}",
                    path = change.path
                ))?);
                tokio::fs::write(theirs, bytes).await?;
            }
            ChangeAction::Add | ChangeAction::Replace => {
                let bytes = verify(
                    fetcher.fetch(entry).await?,
                    &entry.sha512,
                    &entry.path,
                )?;
                tokio::fs::write(&destination, bytes).await?;
            }
            // Deletions never appear in plan.changes (they live in
            // UpdatePlan.deletions and need no download).
            ChangeAction::Delete => {}
        }
    }

    // Merge conflicts may reference entries not in the change list; keep the
    // manifest reachable for the caller building the report.
    let _ = target;

    Ok(())
}

/// Stage 2: copy files that commit will overwrite or delete into backup.
async fn backup_targets(
    instance_dir: &Path,
    backup: &Path,
    plan: &UpdatePlan,
) -> crate::Result<()> {
    let mut targets: Vec<String> = plan.deletions.clone();
    for change in &plan.changes {
        match &change.action {
            ChangeAction::Replace => targets.push(change.path.clone()),
            ChangeAction::Move { from } => targets.push(from.clone()),
            _ => {}
        }
    }
    for path in targets {
        let source = instance_dir.join(sanitize(&path)?);
        if !source.exists() {
            continue;
        }
        let destination = backup.join(sanitize(&path)?);
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::copy(&source, &destination).await?;
    }
    Ok(())
}

/// Stage 3: commit — move staged files into the instance, delete planned
/// deletions and consumed `Move` sources. `.pack-new` files stay as staged.
async fn commit(
    instance_dir: &Path,
    staging: &Path,
    plan: &UpdatePlan,
) -> crate::Result<()> {
    for change in &plan.changes {
        let destination = staging.join(sanitize(&change.path)?);
        if !destination.exists() {
            // Seed skipped (file existed) or merge-degrade (ours kept):
            // nothing to move in.
            continue;
        }
        let target_path = instance_dir.join(sanitize(&change.path)?);
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::rename(&destination, &target_path).await?;
    }
    for path in &plan.deletions {
        let target = instance_dir.join(sanitize(path)?);
        if target.exists() {
            tokio::fs::remove_file(&target).await?;
        }
    }
    // Move sources are consumed by the rename-in of the new path.
    for change in &plan.changes {
        if let ChangeAction::Move { from } = &change.action {
            let source = instance_dir.join(sanitize(from)?);
            if source.exists() {
                tokio::fs::remove_file(&source).await?;
            }
        }
    }
    // Move `.pack-new` sidecars (merge degradation) into the instance next
    // to their kept originals (MIP §9.2).
    for change in &plan.changes {
        if change.action == ChangeAction::MergeDegrade {
            let sidecar_name = format!("{}{PACK_NEW_SUFFIX}", change.path);
            let staged = staging.join(sanitize(&sidecar_name)?);
            let target = instance_dir.join(sanitize(&sidecar_name)?);
            if staged.exists() {
                if let Some(parent) = target.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::rename(&staged, &target).await?;
            }
        }
    }
    Ok(())
}

/// Restores every backed-up file. Used when the commit phase fails; best
/// effort per file, mirroring MIP WF-5's "instance stays on the old version"
/// guarantee.
async fn restore_backup(instance_dir: &Path, backup: &Path) {
    if !backup.exists() {
        return;
    }
    restore_recursive_sync(instance_dir, backup, backup);
    tokio::fs::remove_dir_all(backup).await.ok();
}

fn restore_recursive_sync(
    instance_dir: &Path,
    backup_root: &Path,
    current: &Path,
) {
    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            restore_recursive_sync(instance_dir, backup_root, &path);
        } else {
            let Ok(relative) = path.strip_prefix(backup_root) else {
                continue;
            };
            let target = instance_dir.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::copy(&path, &target).ok();
        }
    }
}

fn sanitize(relative: &str) -> crate::Result<PathBuf> {
    let path = PathBuf::from(relative);
    if path.is_absolute()
        || relative.contains("..")
        || relative.starts_with('/')
    {
        return Err(crate::ErrorKind::OtherError(format!(
            "Unsafe pack path {relative}"
        ))
        .into());
    }
    Ok(path)
}

fn verify(
    bytes: Vec<u8>,
    expected: &str,
    path: &str,
) -> crate::Result<Vec<u8>> {
    let actual = sha512_hex(&bytes);
    if actual != expected.to_lowercase() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Checksum mismatch for {path}: expected {expected}, got {actual}"
        ))
        .into());
    }
    Ok(bytes)
}

/// Builds the post-apply state (MIP WF-5 step 9): target file map, same
/// channel, feature selections carried over.
fn build_new_state(
    base: &MipPackState,
    target: &MipManifest,
    plan: &UpdatePlan,
    entry_by_path: &HashMap<String, MipFileEntry>,
) -> MipPackState {
    let mut files = base.files.clone();
    for change in &plan.changes {
        if let Some(entry) = entry_by_path.get(&change.path) {
            files.insert(
                change.path.clone(),
                super::state::StateFile {
                    sha512: entry.sha512.clone(),
                    policy: entry.policy.clone(),
                },
            );
        }
    }
    for path in &plan.deletions {
        files.remove(path);
    }
    // Move sources are consumed by the rename; their base entry goes away.
    for change in &plan.changes {
        if let ChangeAction::Move { from } = &change.action {
            files.remove(from);
        }
    }
    MipPackState {
        pack_id: target.pack_id.clone(),
        version: target.version.clone(),
        channel: target.channel.clone(),
        selected_features: base.selected_features.clone(),
        files,
        locked_paths: base.locked_paths.clone(),
        disabled_paths: base.disabled_paths.clone(),
        binding: base.binding.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ymcl::mip::diff::compute_update_plan;
    use crate::api::ymcl::mip::manifest::{MipFileEntry, MipManifest};
    use crate::api::ymcl::mip::state::{self, StateFile};
    use std::collections::BTreeMap;

    struct MemoryFetcher {
        objects: HashMap<String, Vec<u8>>,
    }

    impl MemoryFetcher {
        fn new(objects: &[(&str, &[u8])]) -> Self {
            Self {
                objects: objects
                    .iter()
                    .map(|(hash, bytes)| ((*hash).to_string(), bytes.to_vec()))
                    .collect(),
            }
        }
    }

    #[async_trait]
    impl ObjectFetcher for MemoryFetcher {
        async fn fetch(&self, entry: &MipFileEntry) -> crate::Result<Vec<u8>> {
            self.objects.get(&entry.sha512).cloned().ok_or_else(|| {
                crate::Error::from(crate::ErrorKind::OtherError(format!(
                    "no object {}",
                    &entry.sha512[..8]
                )))
            })
        }
    }

    fn hash(bytes: &[u8]) -> String {
        sha512_hex(bytes)
    }

    fn entry(path: &str, sha512: &str) -> MipFileEntry {
        MipFileEntry {
            path: path.to_string(),
            sha512: sha512.to_string(),
            size: None,
            policy: "managed".to_string(),
            feature: None,
            moved_from: None,
            sources: Vec::new(),
        }
    }

    fn manifest(version: &str, files: Vec<MipFileEntry>) -> MipManifest {
        MipManifest {
            format_version: 1,
            pack_id: "test".to_string(),
            version: version.to_string(),
            parent: None,
            channel: Some("stable".to_string()),
            game: None,
            features: Vec::new(),
            files,
        }
    }

    fn plan_for(
        base: &MipPackState,
        target: &MipManifest,
        local: &HashMap<String, String>,
    ) -> (UpdatePlan, HashMap<String, MipFileEntry>) {
        let plan = compute_update_plan(base, target, local).unwrap();
        let entries = target
            .files
            .iter()
            .map(|file| (file.path.clone(), file.clone()))
            .collect();
        (plan, entries)
    }

    async fn write_instance(instance: &Path, path: &str, bytes: &[u8]) {
        let full = instance.join(path);
        tokio::fs::create_dir_all(full.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(full, bytes).await.unwrap();
    }

    fn state_file_from(bytes: &[u8]) -> StateFile {
        StateFile {
            sha512: hash(bytes),
            policy: "managed".to_string(),
        }
    }

    #[tokio::test]
    async fn add_replace_delete_full_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        let instance = dir.path();
        write_instance(instance, "mods/keep.jar", b"keep").await;
        write_instance(instance, "mods/replace.jar", b"old-replace").await;
        write_instance(instance, "mods/gone.jar", b"gone").await;

        let mut base = MipPackState {
            pack_id: "test".to_string(),
            version: "1.0.0".to_string(),
            ..MipPackState::default()
        };
        base.files
            .insert("mods/keep.jar".into(), state_file_from(b"keep"));
        base.files
            .insert("mods/replace.jar".into(), state_file_from(b"old-replace"));
        base.files
            .insert("mods/gone.jar".into(), state_file_from(b"gone"));

        let new_replace = b"new-replace";
        let new_add = b"added";
        let target = manifest(
            "2.0.0",
            vec![
                entry("mods/keep.jar", &hash(b"keep")),
                entry("mods/replace.jar", &hash(new_replace)),
                entry("mods/add.jar", &hash(new_add)),
            ],
        );

        let mut local = HashMap::new();
        local.insert("mods/keep.jar".to_string(), hash(b"keep"));
        local.insert("mods/replace.jar".to_string(), hash(b"old-replace"));
        local.insert("mods/gone.jar".to_string(), hash(b"gone"));

        let (plan, entries) = plan_for(&base, &target, &local);
        let fetcher = MemoryFetcher::new(&[
            (hash(new_replace).as_str(), new_replace.as_slice()),
            (hash(new_add).as_str(), new_add.as_slice()),
        ]);

        let outcome =
            apply_update(instance, &base, &target, &plan, &entries, &fetcher)
                .await
                .unwrap();

        assert_eq!(
            tokio::fs::read(instance.join("mods/replace.jar"))
                .await
                .unwrap(),
            new_replace
        );
        assert_eq!(
            tokio::fs::read(instance.join("mods/add.jar"))
                .await
                .unwrap(),
            new_add
        );
        assert_eq!(
            tokio::fs::read(instance.join("mods/keep.jar"))
                .await
                .unwrap(),
            b"keep"
        );
        assert!(!instance.join("mods/gone.jar").exists());
        assert!(!instance.join(STAGING_DIR_NAME).exists());
        assert!(!instance.join(BACKUP_DIR_NAME).exists());
        assert_eq!(outcome.new_state.version, "2.0.0");
        assert!(outcome.new_state.files.contains_key("mods/add.jar"));
        assert!(!outcome.new_state.files.contains_key("mods/gone.jar"));
    }

    #[tokio::test]
    async fn move_renames_local_file_with_zero_download() {
        let dir = tempfile::tempdir().unwrap();
        let instance = dir.path();
        write_instance(instance, "mods/old.jar", b"moved-content").await;

        let mut base = MipPackState::default();
        base.files
            .insert("mods/old.jar".into(), state_file_from(b"moved-content"));

        // Same hash at a new path: implicit move, no download needed.
        let target = manifest(
            "2.0.0",
            vec![entry("mods/new.jar", &hash(b"moved-content"))],
        );
        let mut local = HashMap::new();
        local.insert("mods/old.jar".to_string(), hash(b"moved-content"));

        let (plan, entries) = plan_for(&base, &target, &local);
        // Fetcher has no objects: any download attempt would fail the test.
        let fetcher = MemoryFetcher::new(&[]);

        let outcome =
            apply_update(instance, &base, &target, &plan, &entries, &fetcher)
                .await
                .unwrap();

        assert_eq!(
            tokio::fs::read(instance.join("mods/new.jar"))
                .await
                .unwrap(),
            b"moved-content"
        );
        assert!(!instance.join("mods/old.jar").exists());
        assert!(outcome.new_state.files.contains_key("mods/new.jar"));
        assert!(!outcome.new_state.files.contains_key("mods/old.jar"));
    }

    #[tokio::test]
    async fn seed_never_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let instance = dir.path();
        write_instance(instance, "options.txt", b"player options").await;

        let mut files = BTreeMap::new();
        files.insert("options.txt".into(), state_file_from(b"player options"));
        let base = MipPackState {
            files,
            ..MipPackState::default()
        };

        let mut seed = entry("options.txt", &hash(b"author options"));
        seed.policy = "seed".to_string();
        let target = manifest("2.0.0", vec![seed]);

        let mut local = HashMap::new();
        local.insert("options.txt".to_string(), hash(b"player options"));
        let (plan, entries) = plan_for(&base, &target, &local);
        let fetcher = MemoryFetcher::new(&[(
            hash(b"author options").as_str(),
            b"author options",
        )]);

        apply_update(instance, &base, &target, &plan, &entries, &fetcher)
            .await
            .unwrap();

        assert_eq!(
            tokio::fs::read(instance.join("options.txt")).await.unwrap(),
            b"player options"
        );
    }

    #[tokio::test]
    async fn merge_degrade_writes_pack_new_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let instance = dir.path();
        write_instance(instance, "config/x.toml", b"player-edited").await;

        let mut files = BTreeMap::new();
        files.insert("config/x.toml".into(), state_file_from(b"base"));
        let base = MipPackState {
            files,
            ..MipPackState::default()
        };

        let mut merge = entry("config/x.toml", &hash(b"author-updated"));
        merge.policy = "merge".to_string();
        let target = manifest("2.0.0", vec![merge]);

        let mut local = HashMap::new();
        local.insert("config/x.toml".to_string(), hash(b"player-edited"));
        let (plan, entries) = plan_for(&base, &target, &local);
        let fetcher = MemoryFetcher::new(&[(
            hash(b"author-updated").as_str(),
            b"author-updated",
        )]);

        apply_update(instance, &base, &target, &plan, &entries, &fetcher)
            .await
            .unwrap();

        // Ours kept, theirs written alongside (MIP §9.2 degradation).
        assert_eq!(
            tokio::fs::read(instance.join("config/x.toml"))
                .await
                .unwrap(),
            b"player-edited"
        );
        assert_eq!(
            tokio::fs::read(instance.join("config/x.toml.pack-new"))
                .await
                .unwrap(),
            b"author-updated"
        );
    }

    #[tokio::test]
    async fn checksum_failure_leaves_instance_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let instance = dir.path();
        write_instance(instance, "mods/a.jar", b"original").await;

        let mut base = MipPackState::default();
        base.files
            .insert("mods/a.jar".into(), state_file_from(b"original"));

        // Manifest claims a hash the fetcher cannot serve correctly.
        let bad_hash = "0".repeat(128);
        let target = manifest("2.0.0", vec![entry("mods/a.jar", &bad_hash)]);

        let mut local = HashMap::new();
        local.insert("mods/a.jar".to_string(), hash(b"original"));
        let (plan, entries) = plan_for(&base, &target, &local);
        let fetcher =
            MemoryFetcher::new(&[(bad_hash.as_str(), b"corrupt".as_slice())]);

        let result =
            apply_update(instance, &base, &target, &plan, &entries, &fetcher)
                .await;
        assert!(result.is_err());
        // Instance untouched, staging cleaned up.
        assert_eq!(
            tokio::fs::read(instance.join("mods/a.jar")).await.unwrap(),
            b"original"
        );
        assert!(!instance.join(STAGING_DIR_NAME).exists());
    }

    #[tokio::test]
    async fn state_round_trips_through_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut base = MipPackState::default();
        base.selected_features.push("shaders".to_string());
        base.files
            .insert("mods/a.jar".into(), state_file_from(b"a"));

        state::save(dir.path(), &base).await.unwrap();
        let loaded = state::load(dir.path()).await.unwrap().unwrap();
        assert_eq!(loaded, base);
    }
}

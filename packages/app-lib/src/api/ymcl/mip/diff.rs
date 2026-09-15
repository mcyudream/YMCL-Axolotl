//! Incremental diff between the installed state (base) and the target
//! manifest (theirs) per MIP §6 / WF-5 steps 3-6: explicit `movedFrom`
//! first, implicit same-hash pairing second, then per-path policy
//! decisions honoring locks and disabled paths.

use super::manifest::MipManifest;
use super::state::MipPackState;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeAction {
    /// Download and write the file.
    Add,
    /// `managed`: replace; the local file is overwritten (unmodified or not,
    /// locks are honored separately).
    Replace,
    /// `seed`: only write when the path does not exist locally.
    SeedIfMissing,
    /// Delete the local file when it still matches the base hash.
    Delete,
    /// Local rename with zero download (base hash found at the new path's
    /// source), then a possible content change handled by `Replace`.
    Move { from: String },
    /// Content changed and policy is `merge`: v1 applies the documented
    /// degradation (MIP §9.2) — keep ours and write theirs as `.pack-new`.
    MergeDegrade,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedChange {
    pub path: String,
    pub action: ChangeAction,
    pub target_sha512: String,
    /// Paths that cannot be resolved from the manifest alone: the local
    /// hash index must be consulted before download (WF-5 step 7 reuse).
    pub sources_needed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UpdatePlan {
    pub changes: Vec<PlannedChange>,
    pub deletions: Vec<String>,
    /// Local-only modified files whose content must be preserved.
    pub preserved: Vec<String>,
}

/// Computes the update plan from base state to target manifest.
///
/// `local_hashes` is the instance's local hash index (path → sha512,
/// covering all files including protocol-unknown ones, WF-5 step 3).
pub fn compute_update_plan(
    base: &MipPackState,
    target: &MipManifest,
    local_hashes: &HashMap<String, String>,
) -> crate::Result<UpdatePlan> {
    target.validate()?;

    let target_by_path: BTreeMap<&str, &super::manifest::MipFileEntry> = target
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect();
    let base_by_path: BTreeMap<&str, &super::state::StateFile> = base
        .files
        .iter()
        .map(|(path, file)| (path.as_str(), file))
        .collect();
    let locked: HashSet<&str> =
        base.locked_paths.iter().map(|path| path.as_str()).collect();
    let disabled: HashSet<&str> = base
        .disabled_paths
        .iter()
        .map(|path| path.as_str())
        .collect();

    // Hash → base path index for rename detection.
    let mut base_by_hash: HashMap<&str, &str> = HashMap::new();
    for (path, file) in &base_by_path {
        base_by_hash.entry(file.sha512.as_str()).or_insert(path);
    }

    let mut plan = UpdatePlan::default();
    let mut handled_moves: HashSet<&str> = HashSet::new();

    // Pass 1: explicit movedFrom declarations (validated against base).
    for file in &target.files {
        if let Some(moved_from) = &file.moved_from {
            if base_by_path.contains_key(moved_from.as_str()) {
                handled_moves.insert(moved_from.as_str());
                plan.changes.push(PlannedChange {
                    path: file.path.clone(),
                    action: ChangeAction::Move {
                        from: moved_from.clone(),
                    },
                    target_sha512: file.sha512.clone(),
                    sources_needed: true,
                });
            }
            // Invalid declaration degrades to "add" (MIP §6): fall through
            // to the generic passes below with no special handling.
        }
    }

    // Pass 2: additions, changes, and implicit moves.
    for file in &target.files {
        let Some(base_file) = base_by_path.get(file.path.as_str()) else {
            // New path. Before downloading, an implicit move may exist: the
            // same hash already in base under another path.
            if let Some(origin) = base_by_hash.get(file.sha512.as_str()) {
                if !handled_moves.contains(*origin) {
                    handled_moves.insert(origin);
                    plan.changes.push(PlannedChange {
                        path: file.path.clone(),
                        action: ChangeAction::Move {
                            from: (*origin).to_string(),
                        },
                        target_sha512: file.sha512.clone(),
                        sources_needed: true,
                    });
                    continue;
                }
            }
            // Feature gating: files of unselected features are skipped.
            if is_feature_selected(base, file) {
                plan.changes.push(PlannedChange {
                    path: file.path.clone(),
                    action: ChangeAction::Add,
                    target_sha512: file.sha512.clone(),
                    sources_needed: true,
                });
            }
            continue;
        };

        if base_file.sha512 == file.sha512 {
            continue; // unchanged
        }

        if locked.contains(file.path.as_str())
            || disabled.contains(file.path.as_str())
        {
            plan.preserved.push(file.path.clone());
            continue;
        }

        plan.changes.push(PlannedChange {
            path: file.path.clone(),
            action: match file.policy.as_str() {
                "seed" => ChangeAction::SeedIfMissing,
                "merge" => ChangeAction::MergeDegrade,
                _ => ChangeAction::Replace,
            },
            target_sha512: file.sha512.clone(),
            sources_needed: true,
        });
    }

    // Pass 3: deletions — base paths absent from the target, delete only
    // when the local file still matches the base hash (MIP §6 "删").
    for (path, base_file) in &base_by_path {
        if target_by_path.contains_key(path) || handled_moves.contains(path) {
            continue;
        }
        if locked.contains(path) || disabled.contains(path) {
            plan.preserved.push((*path).to_string());
            continue;
        }
        match local_hashes.get(*path) {
            Some(local_hash) if local_hash == &base_file.sha512 => {
                plan.deletions.push((*path).to_string());
            }
            Some(_) => plan.preserved.push((*path).to_string()),
            // Already gone locally: nothing to do.
            None => {}
        }
    }

    Ok(plan)
}

fn is_feature_selected(
    base: &MipPackState,
    file: &super::manifest::MipFileEntry,
) -> bool {
    match &file.feature {
        Some(feature) => base
            .selected_features
            .iter()
            .any(|selected| selected == feature),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::super::manifest::{MipFileEntry, MipManifest};
    use super::super::state::{MipPackState, StateFile};
    use super::*;
    use std::collections::HashMap;

    fn entry(path: &str, hash: &str) -> MipFileEntry {
        MipFileEntry {
            path: path.to_string(),
            sha512: hash.to_string(),
            size: None,
            policy: "managed".to_string(),
            feature: None,
            moved_from: None,
            sources: Vec::new(),
        }
    }

    fn state_file(hash: &str) -> StateFile {
        StateFile {
            sha512: hash.to_string(),
            policy: "managed".to_string(),
        }
    }

    fn manifest(files: Vec<MipFileEntry>) -> MipManifest {
        MipManifest {
            format_version: 1,
            pack_id: "test".to_string(),
            version: "2.0.0".to_string(),
            parent: Some("1.0.0".to_string()),
            channel: None,
            game: None,
            features: Vec::new(),
            files,
        }
    }

    fn local(hashes: &[(&str, &str)]) -> HashMap<String, String> {
        hashes
            .iter()
            .map(|(path, hash)| ((*path).to_string(), (*hash).to_string()))
            .collect()
    }

    #[test]
    fn add_change_delete_and_implicit_move() {
        let mut base = MipPackState {
            pack_id: "test".to_string(),
            version: "1.0.0".to_string(),
            ..MipPackState::default()
        };
        base.files.insert("mods/a.jar".into(), state_file("hash-a"));
        base.files.insert("mods/b.jar".into(), state_file("hash-b"));
        base.files
            .insert("mods/old.jar".into(), state_file("hash-moved"));

        let target = manifest(vec![
            entry("mods/a.jar", "hash-a2"),        // changed
            entry("mods/b.jar", "hash-b"),         // unchanged
            entry("mods/moved.jar", "hash-moved"), // implicit move
            entry("mods/new.jar", "hash-new"),     // added
        ]);

        let plan = compute_update_plan(
            &base,
            &target,
            &local(&[
                ("mods/a.jar", "hash-a"),
                ("mods/b.jar", "hash-b"),
                ("mods/old.jar", "hash-moved"),
            ]),
        )
        .unwrap();

        let by_path: HashMap<&str, &PlannedChange> = plan
            .changes
            .iter()
            .map(|change| (change.path.as_str(), change))
            .collect();

        assert_eq!(by_path["mods/a.jar"].action, ChangeAction::Replace);
        assert_eq!(
            by_path["mods/moved.jar"].action,
            ChangeAction::Move {
                from: "mods/old.jar".to_string()
            }
        );
        assert_eq!(by_path["mods/new.jar"].action, ChangeAction::Add);
        // The moved-away source is consumed by the Move (rename), not by a
        // deletion entry (MIP §6 "挪" = local rename with zero download).
        assert!(plan.deletions.is_empty());
    }

    #[test]
    fn explicit_moved_from_takes_precedence() {
        let mut base = MipPackState::default();
        base.files
            .insert("mods/old.jar".into(), state_file("hash-1"));

        let mut target_entry = entry("mods/new.jar", "hash-1");
        target_entry.moved_from = Some("mods/old.jar".to_string());
        let target = manifest(vec![target_entry]);

        let plan = compute_update_plan(
            &base,
            &target,
            &local(&[("mods/old.jar", "hash-1")]),
        )
        .unwrap();
        assert_eq!(
            plan.changes[0].action,
            ChangeAction::Move {
                from: "mods/old.jar".to_string()
            }
        );
        assert!(plan.deletions.is_empty());
    }

    #[test]
    fn locked_and_locally_modified_files_are_preserved() {
        let mut base = MipPackState::default();
        base.files
            .insert("mods/locked.jar".into(), state_file("hash-l"));
        base.files
            .insert("mods/user.jar".into(), state_file("hash-u"));
        base.locked_paths.push("mods/locked.jar".into());

        let target = manifest(vec![entry("mods/locked.jar", "hash-l2")]);
        let plan = compute_update_plan(
            &base,
            &target,
            &local(&[
                ("mods/locked.jar", "hash-l"),
                ("mods/user.jar", "hash-u2"),
            ]),
        )
        .unwrap();

        assert!(plan.changes.is_empty());
        assert!(plan.deletions.is_empty());
        assert!(plan.preserved.contains(&"mods/locked.jar".to_string()));
        assert!(plan.preserved.contains(&"mods/user.jar".to_string()));
    }

    #[test]
    fn seed_and_merge_policies_degrade_correctly() {
        let mut base = MipPackState::default();
        base.files
            .insert("options.txt".into(), state_file("seed-1"));

        let mut seed = entry("options.txt", "seed-2");
        seed.policy = "seed".to_string();
        let mut merge = entry("config/x.toml", "merge-1");
        merge.policy = "merge".to_string();
        let target = manifest(vec![seed, merge]);

        let plan = compute_update_plan(
            &base,
            &target,
            &local(&[("options.txt", "seed-1")]),
        )
        .unwrap();

        let by_path: HashMap<&str, &PlannedChange> = plan
            .changes
            .iter()
            .map(|change| (change.path.as_str(), change))
            .collect();
        // seed: changed seed never overwrites; existence check happens at
        // apply time on disk.
        assert_eq!(by_path["options.txt"].action, ChangeAction::SeedIfMissing);
        // merge on a brand-new path is a plain add; the local-collision case
        // (MIP §9.2 新增冲突) is resolved at apply time against the disk.
        assert_eq!(by_path["config/x.toml"].action, ChangeAction::Add);
    }

    #[test]
    fn future_format_version_is_rejected() {
        let mut files = Vec::new();
        files.push(entry("mods/a.jar", "hash-a"));
        let mut target = manifest(files);
        target.format_version = 99;
        let result = compute_update_plan(
            &MipPackState::default(),
            &target,
            &HashMap::new(),
        );
        assert!(result.is_err());
    }
}

//! Portable installations keep paths relative to the directory containing the
//! executable, while callers continue to work with absolute paths in memory.
//!
//! This follows HMCL's local-directory/relative-path model. No drive-letter
//! migration is needed when the same USB drive is opened on another computer.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const DATA_DIRECTORY: &str = ".Axolotl";
pub const MARKER_FILE: &str = "portable.txt";

static PATHS: OnceLock<PortablePaths> = OnceLock::new();

#[derive(Debug)]
pub struct PortablePaths {
    root: PathBuf,
}

impl PortablePaths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Store files inside the portable installation without an absolute prefix.
    /// Explicit external paths retain their original meaning.
    pub fn store(&self, path: &str) -> String {
        let normalized = normalize(path);
        let root = normalize(&self.root.to_string_lossy());
        let matches = normalized.get(..root.len()).is_some_and(|prefix| {
            if is_windows_path(&root) {
                prefix.eq_ignore_ascii_case(&root)
            } else {
                prefix == root
            }
        });
        if matches {
            let suffix = &normalized[root.len()..];
            if suffix.is_empty() {
                return ".".into();
            }
            if let Some(relative) = suffix.strip_prefix('/')
                && !relative.split('/').any(|part| part == "..")
            {
                return relative.to_owned();
            }
        }
        path.to_owned()
    }

    /// Resolve persisted relative paths against this copy of the launcher,
    /// independently of the shell's current working directory.
    pub fn resolve(&self, path: &str) -> String {
        if path.is_empty() || is_absolute(path) || path.contains("://") {
            return path.to_owned();
        }
        self.root
            .join(path.replace('\\', "/"))
            .to_string_lossy()
            .into_owned()
    }

    pub fn contains(&self, path: &str) -> bool {
        is_absolute(path) && !is_absolute(&self.store(path))
    }
}

fn normalize(path: &str) -> String {
    let path = path.strip_prefix(r"\\?\").unwrap_or(path);
    path.replace('\\', "/").trim_end_matches('/').to_owned()
}

fn is_windows_path(path: &str) -> bool {
    path.starts_with("//")
        || (path.as_bytes().get(1) == Some(&b':')
            && path.as_bytes()[0].is_ascii_alphabetic())
}

fn is_absolute(path: &str) -> bool {
    path.starts_with(['/', '\\']) || is_windows_path(path)
}

/// Called once by the desktop shell, before initializing launcher state.
pub fn initialize(root: PathBuf) {
    PATHS
        .set(PortablePaths::new(root))
        .expect("Portable paths must only be initialized once");
}

pub fn root() -> Option<&'static Path> {
    PATHS.get().map(|paths| paths.root.as_path())
}

pub fn is_enabled() -> bool {
    PATHS.get().is_some()
}

pub(crate) fn store(path: &str) -> String {
    PATHS
        .get()
        .map_or_else(|| path.to_owned(), |paths| paths.store(path))
}

pub(crate) fn resolve(path: &str) -> String {
    PATHS
        .get()
        .map_or_else(|| path.to_owned(), |paths| paths.resolve(path))
}

pub(crate) fn contains(path: &str) -> bool {
    PATHS.get().is_some_and(|paths| paths.contains(path))
}

pub(crate) fn store_optional(path: Option<&str>) -> Option<String> {
    path.map(store)
}

pub(crate) fn resolve_optional(path: Option<String>) -> Option<String> {
    path.map(|path| resolve(&path))
}

/// Convert paths written by earlier portable builds while the installation is
/// still at its original location. New writes go through the storage adapters.
pub(crate) async fn normalize_database_paths(
    pool: &sqlx::SqlitePool,
) -> crate::Result<()> {
    let Some(paths) = PATHS.get() else {
        return Ok(());
    };
    normalize_database_paths_with(pool, paths).await
}

async fn normalize_database_paths_with(
    pool: &sqlx::SqlitePool,
    paths: &PortablePaths,
) -> crate::Result<()> {
    let mut tx = pool.begin().await?;
    for (table, columns) in [
        (
            "settings",
            &["custom_background_path", "backup_repository_path"][..],
        ),
        ("java_versions", &["path"][..]),
        ("discovered_javas", &["path"][..]),
        ("pending_backup_repository_cleanups", &["path"][..]),
        (
            "instances",
            &[
                "icon_path",
                "symlink_target",
                "game_dir_override",
                "linked_launcher_root",
                "linked_dot_minecraft",
                "linked_version_json_path",
            ][..],
        ),
        ("instance_files", &["icon_path"][..]),
    ] {
        for column in columns {
            // Identifiers only come from the static list above.
            let values: Vec<String> = sqlx::query_scalar(&format!(
                "SELECT DISTINCT {column} FROM {table} WHERE {column} IS NOT NULL"
            )).fetch_all(&mut *tx).await?;
            for value in values {
                let stored = paths.store(&value);
                if value != stored {
                    sqlx::query(&format!(
                        "UPDATE OR IGNORE {table} SET {column} = ? WHERE {column} = ?"
                    )).bind(stored).bind(value).execute(&mut *tx).await?;
                }
            }
        }
    }
    // Updating java_versions.path also updates java_default_versions through
    // its existing ON UPDATE CASCADE foreign key.
    let overrides: Vec<(String, String)> = sqlx::query_as(
        "SELECT instance_id, json_extract(overrides, '$.java_path')
         FROM instance_launch_overrides WHERE json_type(overrides, '$.java_path') = 'text'"
    ).fetch_all(&mut *tx).await?;
    for (id, java_path) in overrides {
        let stored = paths.store(&java_path);
        if stored != java_path {
            sqlx::query("UPDATE instance_launch_overrides SET overrides = jsonb(json_set(overrides, '$.java_path', ?)) WHERE instance_id = ?")
                .bind(stored).bind(id).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

/// An explicit marker also activates a new portable installation when an
/// archive tool has omitted its empty data directory. A broken or read-only
/// installation is an error, so it cannot silently open an unrelated database.
pub fn prepare_directory(app_dir: &Path) -> std::io::Result<Option<PathBuf>> {
    let data_dir = app_dir.join(DATA_DIRECTORY);
    if !app_dir.join(MARKER_FILE).is_file() && !data_dir.try_exists()? {
        return Ok(None);
    }
    std::fs::create_dir_all(&data_dir)?;
    let probe = tempfile::NamedTempFile::new_in(&data_dir)?;
    probe.close()?;
    Ok(Some(data_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_follow_the_usb_drive_letter() {
        let old = PortablePaths::new(PathBuf::from(r"E:\游戏\YMCL"));
        let new = PortablePaths::new(PathBuf::from(r"G:\游戏\YMCL"));
        for suffix in [
            ".Axolotl/meta/java_versions/zulu-21/bin/javaw.exe",
            ".Axolotl/profiles/生存/saves/world/level.dat",
            ".Axolotl/caches/icons/example.png",
            "Java/bin/java.exe",
        ] {
            let stored = old.store(&format!(r"E:\游戏\YMCL\{suffix}"));
            assert_eq!(stored, suffix);
            assert_eq!(
                new.resolve(&stored),
                new.root.join(suffix).to_string_lossy()
            );
        }
    }

    #[test]
    fn keeps_external_paths_and_sibling_directories() {
        let paths = PortablePaths::new(PathBuf::from(r"E:\YMCL"));
        for path in [
            r"C:\Java\bin\java.exe",
            r"E:\YMCL-other\icon.png",
            r"E:\YMCL\..\other\icon.png",
        ] {
            assert_eq!(paths.store(path), path);
            assert_eq!(paths.resolve(path), path);
        }
    }

    #[test]
    fn handles_windows_path_spelling_without_touching_instance_names() {
        let paths = PortablePaths::new(PathBuf::from(r"E:\YMCL"));
        assert_eq!(
            paths.store(r"\\?\e:\ymcl\.Axolotl\icons\x.png"),
            ".Axolotl/icons/x.png"
        );
        assert!(paths.contains(r"\\?\e:\ymcl\Java\bin\java.exe"));
        assert!(!paths.contains(r"E:\YMCL-other\bin\java.exe"));
        assert!(!paths.contains(r"E:\YMCL\..\Java\bin\java.exe"));
        assert_eq!(paths.store("My Instance"), "My Instance");
        assert_eq!(
            paths.resolve("https://example.org/icon.png"),
            "https://example.org/icon.png"
        );
    }

    #[test]
    fn marker_creates_a_new_data_directory() {
        let root = tempfile::tempdir().unwrap();
        assert!(prepare_directory(root.path()).unwrap().is_none());
        std::fs::write(root.path().join(MARKER_FILE), "YMCL portable").unwrap();
        assert_eq!(
            prepare_directory(root.path()).unwrap(),
            Some(root.path().join(DATA_DIRECTORY))
        );
        assert_eq!(
            std::fs::read_dir(root.path().join(DATA_DIRECTORY))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn recognizes_existing_copies_and_rejects_a_broken_data_directory() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join(DATA_DIRECTORY);
        std::fs::create_dir(&data).unwrap();
        assert!(prepare_directory(root.path()).unwrap().is_some());
        std::fs::remove_dir(&data).unwrap();
        std::fs::write(&data, "not a directory").unwrap();
        assert!(prepare_directory(root.path()).is_err());
    }

    #[tokio::test]
    async fn earlier_absolute_paths_keep_java_defaults_and_launch_overrides() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new().foreign_keys(true),
            )
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let paths = PortablePaths::new(PathBuf::from(r"E:\YMCL"));
        let java = r"E:\YMCL\.Axolotl\meta\java_versions\test\bin\javaw.exe";
        sqlx::query("INSERT INTO java_versions (major_version, full_version, architecture, path) VALUES (21, '21.0.1', 'amd64', ?)")
            .bind(java).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO java_default_versions (major_version, path) VALUES (21, ?)")
            .bind(java).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO instances (id, path, install_stage, launcher_feature_version, name, created, modified, icon_path, game_dir_override) VALUES ('test', '生存', 'installed', '1', '生存', 0, 0, ?, ?)")
            .bind(r"E:\YMCL\.Axolotl\caches\icons\test.png")
            .bind(r"E:\YMCL\Games\生存")
            .execute(&pool).await.unwrap();
        let overrides = serde_json::json!({ "java_path": java }).to_string();
        sqlx::query("INSERT INTO instance_launch_overrides (instance_id, overrides) VALUES ('test', jsonb(?))")
            .bind(overrides).execute(&pool).await.unwrap();

        // Startup can repeat this conversion without breaking the composite
        // Java-default foreign key or rewriting instance-relative identifiers.
        for _ in 0..2 {
            normalize_database_paths_with(&pool, &paths).await.unwrap();
        }
        let default: String = sqlx::query_scalar(
            "SELECT path FROM java_default_versions WHERE major_version = 21",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(default, ".Axolotl/meta/java_versions/test/bin/javaw.exe");
        let launch: String = sqlx::query_scalar("SELECT json_extract(overrides, '$.java_path') FROM instance_launch_overrides WHERE instance_id = 'test'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(launch, default);
        let (path, icon, game): (String, String, String) = sqlx::query_as("SELECT path, icon_path, game_dir_override FROM instances WHERE id = 'test'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(path, "生存");
        assert_eq!(icon, ".Axolotl/caches/icons/test.png");
        assert_eq!(game, "Games/生存");
        pool.close().await;
    }
}

//! Separate child processes exercise the actual persistence adapters with a
//! different portable root on each launch, like moving a USB drive between PCs.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn database_and_game_files_survive_a_portable_directory_move() {
    let temporary = tempfile::tempdir().unwrap();
    let old = temporary.path().join("E drive 游戏");
    let new = temporary.path().join("G drive 游戏");
    for (root, stage) in [(&old, "write"), (&new, "read")] {
        if stage == "read" {
            std::fs::rename(&old, &new).unwrap();
        }
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "portable_child",
                "--ignored",
                "--test-threads=1",
            ])
            .env("YMCL_PORTABLE_TEST_ROOT", root)
            .env("YMCL_PORTABLE_TEST_STAGE", stage)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{stage} child failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[tokio::test]
#[ignore = "invoked by the portable directory test in an isolated process"]
async fn portable_child() {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use theseus::data::JavaVersion;
    use theseus::settings::Settings;

    let root =
        PathBuf::from(std::env::var_os("YMCL_PORTABLE_TEST_ROOT").unwrap());
    let write = std::env::var("YMCL_PORTABLE_TEST_STAGE").unwrap() == "write";
    theseus::portable::initialize(root.clone());
    let data = root.join(".Axolotl");
    std::fs::create_dir_all(&data).unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(data.join("app.db"))
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let java = data.join("meta/java_versions/test/bin/javaw.exe");
    let background = data.join("backgrounds/local.png");
    let backups = data.join("Backups/custom");
    let world = data.join("profiles/生存/saves/world/level.dat");
    if write {
        for (path, content) in [
            (&java, "java runtime"),
            (&background, "background"),
            (&world, "world data"),
        ] {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        JavaVersion {
            parsed_version: 21,
            version: "21.0.1".into(),
            architecture: "amd64".into(),
            path: java.to_string_lossy().into_owned(),
            distribution: Some("Test".into()),
        }
        .upsert(&pool)
        .await
        .unwrap();
        JavaVersion::set_default(21, &java.to_string_lossy(), &pool)
            .await
            .unwrap();
        let mut settings = Settings::get(&pool).await.unwrap();
        settings.custom_background_path =
            Some(background.to_string_lossy().into_owned());
        settings.backup_repository_path =
            Some(backups.to_string_lossy().into_owned());
        settings.update(&pool).await.unwrap();
        let path: String = sqlx::query_scalar("SELECT path FROM java_versions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(path, ".Axolotl/meta/java_versions/test/bin/javaw.exe");
        let path: String =
            sqlx::query_scalar("SELECT custom_background_path FROM settings")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(path, ".Axolotl/backgrounds/local.png");
    } else {
        let runtime = JavaVersion::get(21, &pool).await.unwrap().unwrap();
        assert_eq!(PathBuf::from(&runtime.path), java);
        assert_eq!(
            std::fs::read_to_string(&runtime.path).unwrap(),
            "java runtime"
        );
        let settings = Settings::get(&pool).await.unwrap();
        assert_eq!(
            settings
                .custom_background_path
                .as_deref()
                .map(PathBuf::from),
            Some(background.clone())
        );
        assert_eq!(
            settings
                .backup_repository_path
                .as_deref()
                .map(PathBuf::from),
            Some(backups)
        );
        assert_eq!(std::fs::read_to_string(background).unwrap(), "background");
        assert_eq!(std::fs::read_to_string(&world).unwrap(), "world data");
        JavaVersion::remove_default_for_path(&runtime.path, &pool)
            .await
            .unwrap();
        JavaVersion::delete(&runtime.path, &pool).await.unwrap();
        assert!(JavaVersion::get_all(&pool).await.unwrap().is_empty());
    }
    pool.close().await;
}

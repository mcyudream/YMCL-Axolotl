use crate::State;
use crate::state::{CacheBehaviour, CachedEntry};
use crate::util::fetch::FetchSemaphore;
pub use daedalus::minecraft::VersionManifest;
pub use daedalus::modded::Manifest;
use sqlx::SqlitePool;

#[tracing::instrument]
pub async fn get_minecraft_versions() -> crate::Result<VersionManifest> {
    get_minecraft_versions_with_cache(None).await
}

pub async fn get_minecraft_versions_with_cache(
    cache_behaviour: Option<CacheBehaviour>,
) -> crate::Result<VersionManifest> {
    let state = State::get().await?;
    let minecraft_versions = CachedEntry::get_minecraft_manifest(
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::NoValueFor("minecraft versions".to_string())
    })?;

    Ok(minecraft_versions)
}

// #[tracing::instrument]
pub async fn get_loader_versions(loader: &str) -> crate::Result<Manifest> {
    match get_loader_versions_with_cache(
        loader,
        Some(CacheBehaviour::MustRevalidate),
    )
    .await
    {
        Ok(manifest) => Ok(manifest),
        Err(refresh_error) => {
            match get_loader_versions_with_cache(
                loader,
                Some(CacheBehaviour::CacheOnly),
            )
            .await
            {
                Ok(manifest) => {
                    tracing::warn!(
                        loader,
                        error = %refresh_error,
                        "Loader manifest refresh failed; serving cached data"
                    );
                    Ok(manifest)
                }
                Err(_) => Err(refresh_error),
            }
        }
    }
}

pub async fn get_loader_versions_with_cache(
    loader: &str,
    cache_behaviour: Option<CacheBehaviour>,
) -> crate::Result<Manifest> {
    let state = State::get().await?;
    get_cached_loader_versions(
        loader,
        None,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await
}

pub async fn get_loader_versions_for_game(
    loader: &str,
    game_version: &str,
) -> crate::Result<Manifest> {
    match get_loader_versions_for_game_with_cache(
        loader,
        game_version,
        Some(CacheBehaviour::MustRevalidate),
    )
    .await
    {
        Ok(manifest) => Ok(manifest),
        Err(refresh_error) => {
            match get_loader_versions_for_game_with_cache(
                loader,
                game_version,
                Some(CacheBehaviour::CacheOnly),
            )
            .await
            {
                Ok(manifest) => {
                    tracing::warn!(
                        loader,
                        game_version,
                        error = %refresh_error,
                        "Scoped loader metadata refresh failed; serving cached data"
                    );
                    Ok(manifest)
                }
                Err(_) => Err(refresh_error),
            }
        }
    }
}

pub async fn get_loader_versions_for_game_with_cache(
    loader: &str,
    game_version: &str,
    cache_behaviour: Option<CacheBehaviour>,
) -> crate::Result<Manifest> {
    let state = State::get().await?;
    get_cached_loader_versions(
        loader,
        Some(game_version),
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await
}

async fn get_cached_loader_versions(
    loader: &str,
    game_version: Option<&str>,
    cache_behaviour: Option<CacheBehaviour>,
    pool: &SqlitePool,
    fetch_semaphore: &FetchSemaphore,
) -> crate::Result<Manifest> {
    let metadata = match game_version {
        Some(game_version) if loader != "cleanroom" => {
            daedalus::modded::loader_manifest_metadata_for_game(
                loader,
                game_version,
            )
        }
        _ => daedalus::modded::loader_manifest_metadata(loader),
    };
    let loaders = CachedEntry::get_loader_manifest(
        &metadata.cache_key,
        cache_behaviour,
        pool,
        fetch_semaphore,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::NoValueFor(match game_version {
            Some(game_version) => {
                format!("{loader} loader versions for Minecraft {game_version}")
            }
            None => format!("{loader} loader versions"),
        })
    })?;

    Ok(match game_version {
        Some(game_version) if loader == "cleanroom" => {
            super::loader_metadata::scope_manifest(
                loaders.manifest,
                game_version,
            )
        }
        _ => loaders.manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CacheValue, CachedLoaderManifest};
    use chrono::Utc;
    use sqlx::sqlite::SqlitePoolOptions;
    use tokio::sync::Semaphore;

    fn cached_manifest(
        key: &str,
        loader: &str,
        game_version: &str,
        loader_version: &str,
        expires: i64,
    ) -> CachedEntry {
        let mut entry = CacheValue::LoaderManifest(CachedLoaderManifest {
            loader: key.to_string(),
            manifest: serde_json::from_value(serde_json::json!({
                "gameVersions": [{
                    "id": game_version,
                    "stable": true,
                    "loaders": [{
                        "id": loader_version,
                        "url": "https://example.com/installer.jar",
                        "stable": false,
                        "profileSource": "installer"
                    }]
                }],
                "versionGroups": []
            }))
            .unwrap(),
        })
        .get_entry();
        assert!(key.starts_with(loader));
        entry.expires = expires;
        entry
    }

    async fn cache_pool(entries: &[CachedEntry]) -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE cache (
				id TEXT NOT NULL,
				data_type TEXT NOT NULL,
				alias TEXT NULL,
				data BLOB NULL,
				expires INTEGER NOT NULL,
				PRIMARY KEY (id, data_type),
				UNIQUE (data_type, alias)
			)",
        )
        .execute(&pool)
        .await
        .unwrap();
        CachedEntry::upsert_many(entries, &pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn cleanroom_scoped_list_uses_refreshed_global_manifest() {
        let expires = Utc::now().timestamp() + 1800;
        let pool = cache_pool(&[
            cached_manifest(
                "cleanroom-v1",
                "cleanroom",
                "1.12.2",
                "0.3.15-alpha",
                expires,
            ),
            cached_manifest(
                "cleanroom-v1:1.12.2",
                "cleanroom",
                "1.12.2",
                "0.3.15-alpha",
                expires,
            ),
            cached_manifest(
                "cleanroom-v2:1.12.2",
                "cleanroom",
                "1.12.2",
                "0.3.15-alpha",
                expires,
            ),
            cached_manifest(
                "cleanroom-v2",
                "cleanroom",
                "1.12.2",
                "0.7.14-alpha",
                expires,
            ),
        ])
        .await;
        let semaphore = FetchSemaphore(Semaphore::new(1));
        for game_version in [None, Some("1.12.2")] {
            let manifest = get_cached_loader_versions(
                "cleanroom",
                game_version,
                Some(CacheBehaviour::MustRevalidate),
                &pool,
                &semaphore,
            )
            .await
            .unwrap();
            assert_eq!(manifest.game_versions[0].id, "1.12.2");
            assert_eq!(manifest.game_versions[0].loaders[0].id, "0.7.14-alpha");
        }
    }

    #[tokio::test]
    async fn cleanroom_shared_cache_preserves_game_support_and_offline_access()
    {
        let pool = cache_pool(&[cached_manifest(
            "cleanroom-v2",
            "cleanroom",
            "1.12.2",
            "0.7.14-alpha",
            Utc::now().timestamp() - 60,
        )])
        .await;
        let semaphore = FetchSemaphore(Semaphore::new(1));
        for game_version in ["1.12.2", "26.3"] {
            let manifest = get_cached_loader_versions(
                "cleanroom",
                Some(game_version),
                Some(CacheBehaviour::CacheOnly),
                &pool,
                &semaphore,
            )
            .await
            .unwrap();
            assert_eq!(manifest.game_versions[0].id, game_version);
            assert_eq!(
                manifest.game_versions[0].loaders.len(),
                usize::from(game_version == "1.12.2")
            );
        }
    }

    #[tokio::test]
    async fn forge_loader_cache_remains_scoped_by_game_version() {
        let expires = Utc::now().timestamp() + 1800;
        let pool = cache_pool(&[
            cached_manifest(
                "forge-v3:1.20.1",
                "forge",
                "1.20.1",
                "47.4.22",
                expires,
            ),
            cached_manifest(
                "forge-v3:26.2",
                "forge",
                "26.2",
                "65.1.1",
                expires,
            ),
        ])
        .await;
        let semaphore = FetchSemaphore(Semaphore::new(1));
        for (game_version, expected) in
            [("1.20.1", "47.4.22"), ("26.2", "65.1.1")]
        {
            let manifest = get_cached_loader_versions(
                "forge",
                Some(game_version),
                Some(CacheBehaviour::CacheOnly),
                &pool,
                &semaphore,
            )
            .await
            .unwrap();
            assert_eq!(manifest.game_versions[0].id, game_version);
            assert_eq!(manifest.game_versions[0].loaders[0].id, expected);
        }
    }
}

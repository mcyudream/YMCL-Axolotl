//! Last successful MIP documents, scoped to a domain, account and URL.
//! Update/install mutations still require a fresh response.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bytes::Bytes;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::manifest::MipManifest;
use super::update::{MipServerBinding, MipVersionEntry};
use crate::ErrorKind;

#[cfg(not(test))]
const CACHED_REFRESH_TIMEOUT: Duration = Duration::from_secs(3);
#[cfg(test)]
const CACHED_REFRESH_TIMEOUT: Duration = Duration::from_millis(50);

pub(super) struct MetadataSnapshot {
    pub value: Value,
    pub cached: bool,
}

pub(super) struct MetadataCache {
    path: PathBuf,
}

impl MetadataCache {
    pub fn new(root: &Path, scope: &str, url: &str) -> Self {
        let key = serde_json::to_vec(&(scope, url)).unwrap();
        let digest = format!("{:x}", Sha256::digest(key));
        Self {
            path: root.join("ymcl-mip-metadata-v1").join(digest),
        }
    }

    pub async fn fetch(
        &self,
        url: &str,
        allow_cached: bool,
        request: impl Future<Output = crate::Result<Bytes>>,
    ) -> crate::Result<MetadataSnapshot> {
        let cached = if allow_cached {
            tokio::fs::read(&self.path)
                .await
                .ok()
                .and_then(|bytes| parse_document(url, &bytes).ok())
        } else {
            None
        };
        let response = if cached.is_some() {
            tokio::time::timeout(CACHED_REFRESH_TIMEOUT, request)
                .await
                .unwrap_or_else(|_| {
                    Err(ErrorKind::NetworkError(
                        "MIP metadata refresh timed out".to_string(),
                    )
                    .into())
                })
        } else {
            request.await
        };
        match response.and_then(|bytes| parse_document(url, &bytes)) {
            Ok(value) => {
                if let Err(error) = self.store(&value).await {
                    tracing::warn!(%error, "Could not save MIP metadata cache");
                }
                Ok(MetadataSnapshot {
                    value,
                    cached: false,
                })
            }
            Err(error) => {
                if can_use_cached_metadata(&error)
                    && let Some(value) = cached
                {
                    tracing::warn!(
                        url = %crate::util::fetch::sanitize_url_for_log(url),
                        %error,
                        "MIP refresh failed; keeping the last successful document"
                    );
                    return Ok(MetadataSnapshot {
                        value,
                        cached: true,
                    });
                }
                Err(error)
            }
        }
    }

    async fn store(&self, value: &Value) -> crate::Result<()> {
        tokio::fs::create_dir_all(self.path.parent().unwrap()).await?;
        crate::util::io::write(&self.path, serde_json::to_vec(value)?).await?;
        Ok(())
    }
}

pub(super) fn can_use_cached_metadata(error: &crate::Error) -> bool {
    match error.raw.as_ref() {
        ErrorKind::HttpError { status, .. } => {
            *status == 408 || *status == 429 || *status >= 500
        }
        ErrorKind::LabrinthError(error) => error.status.is_some_and(|status| {
            status == 408 || status == 429 || status >= 500
        }),
        ErrorKind::FetchError(error) => {
            !error.is_builder() && !error.is_redirect()
        }
        ErrorKind::Any(error) => error.chain().any(|cause| {
            cause.is::<reqwest::Error>() || cause.is::<serde_json::Error>()
        }),
        ErrorKind::JSONError(_) | ErrorKind::NetworkError(_) => true,
        _ => false,
    }
}

fn parse_document(url: &str, bytes: &[u8]) -> crate::Result<Value> {
    let value: Value = serde_json::from_slice(bytes)?;
    let url = url::Url::parse(url)?;
    let path = url.path();
    if path.ends_with("/api/servers") {
        // Missing keys and business-error envelopes must not replace a good
        // list. An explicitly empty array is a valid fresh list.
        serde_json::from_value::<Vec<MipServerBinding>>(
            value.get("servers").cloned().unwrap_or(Value::Null),
        )?;
    } else if path.ends_with("/versions") {
        serde_json::from_value::<Vec<MipVersionEntry>>(
            value.get("versions").cloned().unwrap_or(Value::Null),
        )?;
    } else if path.contains("/manifest/") {
        let manifest: MipManifest = serde_json::from_value(value.clone())?;
        manifest.validate().map_err(|error| {
            ErrorKind::NetworkError(format!("Invalid MIP manifest: {error}"))
        })?;
        let requested_version = path.rsplit('/').next().unwrap_or_default();
        let requested_version = urlencoding::decode(requested_version)
            .map_err(|error| {
                ErrorKind::NetworkError(format!(
                    "Invalid MIP manifest version: {error}"
                ))
            })?;
        if requested_version != manifest.version {
            return Err(ErrorKind::NetworkError(
                "MIP manifest version does not match the requested version"
                    .to_string(),
            )
            .into());
        }
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "https://domain.example/mip/api/servers";
    const SERVERS: &[u8] =
        br#"{"servers":[{"serverId":"server","mcAddress":"play.example"}]}"#;

    fn unavailable() -> crate::Result<Bytes> {
        Err(ErrorKind::NetworkError("broken pipe".to_string()).into())
    }

    #[tokio::test]
    async fn successful_documents_survive_restart_and_transport_failure() {
        let dir = tempfile::tempdir().unwrap();
        let cache = MetadataCache::new(dir.path(), "domain/player", URL);
        let first = cache
            .fetch(URL, true, async { Ok(Bytes::from_static(SERVERS)) })
            .await
            .unwrap();
        assert!(!first.cached);
        let restarted = MetadataCache::new(dir.path(), "domain/player", URL);
        let fallback = restarted
            .fetch(URL, true, async { unavailable() })
            .await
            .unwrap();
        assert!(fallback.cached);
        assert_eq!(fallback.value, first.value);
    }

    #[tokio::test]
    async fn fresh_empty_lists_replace_old_servers_and_bad_bodies_do_not() {
        let dir = tempfile::tempdir().unwrap();
        let cache = MetadataCache::new(dir.path(), "domain/player", URL);
        cache
            .fetch(URL, true, async { Ok(Bytes::from_static(SERVERS)) })
            .await
            .unwrap();
        for body in [
            b"{\"servers\":[".as_slice(),
            br#"{"code":1000,"message":"upstream error"}"#,
            br#"{"servers":"invalid"}"#,
        ] {
            let snapshot = cache
                .fetch(URL, true, async { Ok(Bytes::copy_from_slice(body)) })
                .await
                .unwrap();
            assert!(snapshot.cached);
            assert_eq!(snapshot.value["servers"][0]["serverId"], "server");
        }
        let empty = cache
            .fetch(URL, true, async {
                Ok(Bytes::from_static(br#"{"servers":[]}"#))
            })
            .await
            .unwrap();
        assert!(!empty.cached);
        let fallback = cache
            .fetch(URL, true, async { unavailable() })
            .await
            .unwrap();
        assert_eq!(fallback.value, empty.value);
    }

    #[tokio::test]
    async fn cache_is_scoped_and_updates_cannot_use_stale_documents() {
        let dir = tempfile::tempdir().unwrap();
        let cache = MetadataCache::new(dir.path(), "domain/player", URL);
        cache
            .fetch(URL, true, async { Ok(Bytes::from_static(SERVERS)) })
            .await
            .unwrap();
        assert!(
            cache
                .fetch(URL, false, async { unavailable() })
                .await
                .is_err()
        );
        for (scope, url) in [
            ("other-domain/player", URL),
            ("domain/other-player", URL),
            ("domain/anonymous", URL),
            ("domain/player", "https://other.example/mip/api/servers"),
        ] {
            assert!(
                MetadataCache::new(dir.path(), scope, url)
                    .fetch(url, true, async { unavailable() })
                    .await
                    .is_err()
            );
        }
        for status in [401, 403, 404] {
            assert!(
                cache
                    .fetch(URL, true, async {
                        Err(ErrorKind::HttpError {
                            status,
                            method: "GET".to_string(),
                            url: URL.to_string(),
                        }
                        .into())
                    })
                    .await
                    .is_err()
            );
        }
    }

    #[tokio::test]
    async fn stalled_refresh_returns_cached_data_without_waiting_for_retries() {
        let dir = tempfile::tempdir().unwrap();
        let cache = MetadataCache::new(dir.path(), "domain/player", URL);
        cache
            .fetch(URL, true, async { Ok(Bytes::from_static(SERVERS)) })
            .await
            .unwrap();
        let snapshot = tokio::time::timeout(
            Duration::from_secs(1),
            cache.fetch(URL, true, std::future::pending()),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(snapshot.cached);
    }

    #[tokio::test]
    async fn corrupt_cache_is_ignored_and_manifest_versions_are_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let url = "https://domain.example/mip/api/packs/pack/manifest/1.0";
        let cache = MetadataCache::new(dir.path(), "domain/player", url);
        cache
            .fetch(url, true, async {
                Ok(Bytes::from_static(br#"{"version":"1.0","files":[]}"#))
            })
            .await
            .unwrap();
        assert!(
            cache
                .fetch(url, false, async {
                    Ok(Bytes::from_static(br#"{"version":"2.0","files":[]}"#))
                })
                .await
                .is_err()
        );
        let snapshot = cache
            .fetch(url, true, async { unavailable() })
            .await
            .unwrap();
        assert_eq!(snapshot.value["version"], "1.0");
        tokio::fs::write(&cache.path, b"partial").await.unwrap();
        assert!(
            cache
                .fetch(url, true, async { unavailable() })
                .await
                .is_err()
        );
    }
}

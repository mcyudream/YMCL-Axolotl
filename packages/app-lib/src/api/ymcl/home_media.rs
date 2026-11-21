//! Persistent background videos. Only complete files become playable.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::{
    CONTENT_TYPE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Semaphore};

use crate::util::download::foreground::FOREGROUND_REQUESTS;
use crate::util::fetch::{
    ResourceClass, authority_uses_http1_fallback, http1_fallback_client,
    is_h2_protocol_failure, official_route, record_authority_h2_failure,
    sanitize_url_for_log, url_authority,
};

const MAX_VIDEO_BYTES: u64 = 512 * 1024 * 1024;
const MAX_CACHE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_UNUSED_SECONDS: u64 = 30 * 24 * 60 * 60;
const REVALIDATE_AFTER: u64 = 24 * 60 * 60;
static DOWNLOADS: Semaphore = Semaphore::const_new(1);
static ACTIVE_DIRECTORIES: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static LOCKS: LazyLock<dashmap::DashMap<PathBuf, Weak<Mutex<()>>>> =
    LazyLock::new(dashmap::DashMap::new);

#[derive(Clone, Serialize, Deserialize)]
struct CachedVideo {
    file: String,
    size: u64,
    checked_at: u64,
    etag: Option<String>,
    last_modified: Option<String>,
}

fn error(message: &str) -> crate::Error {
    crate::ErrorKind::OtherError(message.to_string()).into()
}

fn video_url(raw: &str) -> crate::Result<url::Url> {
    let url = url::Url::parse(raw)?;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(error(
            "Background video must use an HTTP(S) URL without credentials",
        ));
    }
    Ok(url)
}

fn video_extension(
    content_type: Option<&str>,
    url: &url::Url,
) -> crate::Result<&'static str> {
    let mime = content_type
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    match mime {
        "video/mp4" | "video/x-m4v" => return Ok("mp4"),
        "video/webm" => return Ok("webm"),
        "video/ogg" | "application/ogg" => return Ok("ogv"),
        "video/quicktime" => return Ok("mov"),
        "" | "application/octet-stream" => {}
        _ => {
            return Err(error(
                "The background URL did not return a supported video",
            ));
        }
    }
    match Path::new(url.path())
        .extension()
        .and_then(|ext| ext.to_str())
    {
        Some(ext)
            if ext.eq_ignore_ascii_case("mp4")
                || ext.eq_ignore_ascii_case("m4v") =>
        {
            Ok("mp4")
        }
        Some(ext) if ext.eq_ignore_ascii_case("webm") => Ok("webm"),
        Some(ext)
            if ext.eq_ignore_ascii_case("ogv")
                || ext.eq_ignore_ascii_case("ogg") =>
        {
            Ok("ogv")
        }
        Some(ext) if ext.eq_ignore_ascii_case("mov") => Ok("mov"),
        _ => Err(error(
            "The background URL did not identify a supported video",
        )),
    }
}

fn safe_cached_name(name: &str) -> bool {
    let Some((hash, extension)) = name.split_once('.') else {
        return false;
    };
    hash.len() == 64
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        && matches!(extension, "mp4" | "webm" | "ogv" | "mov")
}

async fn read_cached(dir: &Path) -> Option<CachedVideo> {
    let bytes = tokio::fs::read(dir.join("metadata.json")).await.ok()?;
    let cached: CachedVideo = serde_json::from_slice(&bytes).ok()?;
    if !safe_cached_name(&cached.file)
        || cached.size == 0
        || cached.size > MAX_VIDEO_BYTES
    {
        return None;
    }
    let metadata = tokio::fs::metadata(dir.join(&cached.file)).await.ok()?;
    (metadata.is_file() && metadata.len() == cached.size).then_some(cached)
}

async fn write_metadata(dir: &Path, cached: &CachedVideo) -> crate::Result<()> {
    let dir = dir.to_path_buf();
    let bytes = serde_json::to_vec(cached)?;
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new_in(&dir)?;
        file.write_all(&bytes)?;
        file.persist(dir.join("metadata.json"))
            .map_err(|err| err.error)?;
        Ok(())
    })
    .await??;
    Ok(())
}

async fn prune_superseded(dir: &Path) -> std::io::Result<()> {
    let current = read_cached(dir).await.map(|cached| cached.file);
    let mut files = tokio::fs::read_dir(dir).await?;
    while let Some(file) = files.next_entry().await? {
        let name = file.file_name();
        let name = name.to_string_lossy();
        if file.file_type().await?.is_file()
            && (safe_cached_name(&name) || name.starts_with(".tmp"))
            && current.as_deref() != Some(name.as_ref())
        {
            tokio::fs::remove_file(file.path()).await?;
        }
    }
    Ok(())
}

/// Active directories stay intact until the next launcher session so WebView range reads remain valid.
async fn prepare_cache(
    root: &Path,
    requested: &Path,
    active: &mut HashSet<PathBuf>,
    now: u64,
    max_bytes: u64,
) -> std::io::Result<()> {
    if active.contains(requested) {
        return Ok(());
    }
    tokio::fs::create_dir_all(requested).await?;
    prune_superseded(requested).await?;
    active.insert(requested.to_path_buf());
    tokio::fs::write(requested.join("last-used"), now.to_string()).await?;
    prune_inactive_cache(root, active, now, max_bytes).await
}

async fn prune_inactive_cache(
    root: &Path,
    active: &HashSet<PathBuf>,
    now: u64,
    max_bytes: u64,
) -> std::io::Result<()> {
    let mut directories = tokio::fs::read_dir(root).await?;
    let mut candidates = Vec::new();
    let mut total = 0_u64;
    while let Some(directory) = directories.next_entry().await? {
        let name = directory.file_name();
        let name = name.to_string_lossy();
        if !directory.file_type().await?.is_dir()
            || name.len() != 64
            || !name.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            continue;
        }
        let dir = directory.path();
        let protected = active.contains(&dir);
        if !protected {
            prune_superseded(&dir).await?;
        }
        let mut files = tokio::fs::read_dir(&dir).await?;
        let mut size = 0_u64;
        while let Some(file) = files.next_entry().await? {
            if file.file_type().await?.is_file() {
                size = size.saturating_add(file.metadata().await?.len());
            }
        }
        total = total.saturating_add(size);
        if !protected {
            let last_used = tokio::fs::read_to_string(dir.join("last-used"))
                .await
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(
                    read_cached(&dir)
                        .await
                        .map_or(0, |cached| cached.checked_at),
                );
            candidates.push((last_used, dir, size));
        }
    }
    candidates.sort_by_key(|(last_used, _, _)| *last_used);
    for (last_used, dir, size) in candidates {
        if now.saturating_sub(last_used) >= MAX_UNUSED_SECONDS
            || total > max_bytes
        {
            tokio::fs::remove_dir_all(dir).await?;
            total = total.saturating_sub(size);
        }
    }
    Ok(())
}

async fn download_video(
    dir: &Path,
    url: &url::Url,
    client: &reqwest::Client,
    cached: Option<&CachedVideo>,
    now: u64,
) -> crate::Result<PathBuf> {
    let _permit = DOWNLOADS.acquire().await?;
    let authority = url_authority(url.as_str());
    let mut use_http1 = authority
        .as_deref()
        .is_some_and(authority_uses_http1_fallback);
    let mut fallback_client = None;
    let mut attempt = 0;
    loop {
        attempt += 1;
        FOREGROUND_REQUESTS.wait_for_idle().await;
        if use_http1 && fallback_client.is_none() {
            let route = official_route(url.as_str(), ResourceClass::Other);
            fallback_client =
                Some(http1_fallback_client(route.proxy, false, false).await?);
        }
        let client = fallback_client.as_ref().unwrap_or(client);
        match download_video_once(dir, url, client, cached, now).await {
            Ok(path) => return Ok(path),
            Err(error) => {
                let retryable = matches!(
                    error.raw.as_ref(),
                    crate::ErrorKind::FetchError(source)
                        if !source.is_builder()
                            && !source.is_redirect()
                            && source.status().is_none_or(|status| {
                                status.is_server_error()
                                    || status == reqwest::StatusCode::REQUEST_TIMEOUT
                                    || status == reqwest::StatusCode::TOO_MANY_REQUESTS
                            })
                );
                if attempt >= 3 || !retryable {
                    return Err(error);
                }
                use_http1 |= authority
                    .as_deref()
                    .is_some_and(authority_uses_http1_fallback);
                tracing::warn!(
                    url = %sanitize_url_for_log(url.as_str()),
                    attempt,
                    http1_fallback = use_http1,
                    "Background video download failed; retrying"
                );
                tokio::time::sleep(Duration::from_millis(100 * attempt)).await;
            }
        }
    }
}

fn video_transport_error(
    url: &url::Url,
    version: Option<reqwest::Version>,
    error: reqwest::Error,
) -> crate::Error {
    if (version == Some(reqwest::Version::HTTP_2)
        || is_h2_protocol_failure(&error))
        && let Some(authority) = url_authority(url.as_str())
    {
        record_authority_h2_failure(&authority);
    }
    error.into()
}

async fn download_video_once(
    dir: &Path,
    url: &url::Url,
    client: &reqwest::Client,
    cached: Option<&CachedVideo>,
    now: u64,
) -> crate::Result<PathBuf> {
    let mut request = client
        .get(url.clone())
        .header("Priority", "u=7")
        .timeout(Duration::from_secs(30 * 60));
    if let Some(cached) = cached {
        if let Some(etag) = &cached.etag {
            request = request.header(IF_NONE_MATCH, etag);
        }
        if let Some(modified) = &cached.last_modified {
            request = request.header(IF_MODIFIED_SINCE, modified);
        }
    }
    let mut response = request
        .send()
        .await
        .map_err(|error| video_transport_error(url, None, error))?
        .error_for_status()?;
    let http_version = response.version();
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        let cached = cached.ok_or_else(|| {
            error("Video server returned 304 without a cached file")
        })?;
        write_metadata(
            dir,
            &CachedVideo {
                checked_at: now,
                ..cached.clone()
            },
        )
        .await?;
        return Ok(dir.join(&cached.file));
    }
    if response.status() != reqwest::StatusCode::OK {
        return Err(error("Video server did not return a complete video"));
    }
    let extension = video_extension(
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        url,
    )?;
    if response
        .content_length()
        .is_some_and(|size| size > MAX_VIDEO_BYTES)
    {
        return Err(error("Background video exceeds the 512 MiB cache limit"));
    }
    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let last_modified = response
        .headers()
        .get(LAST_MODIFIED)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let temporary = tempfile::NamedTempFile::new_in(dir)?;
    let mut file = tokio::fs::File::from_std(temporary.reopen()?);
    let mut size = 0;
    let mut hash = Sha256::new();
    loop {
        FOREGROUND_REQUESTS.wait_for_idle().await;
        let Some(chunk) = response.chunk().await.map_err(|error| {
            video_transport_error(url, Some(http_version), error)
        })?
        else {
            break;
        };
        size += chunk.len() as u64;
        if size > MAX_VIDEO_BYTES {
            return Err(error(
                "Background video exceeds the 512 MiB cache limit",
            ));
        }
        hash.update(&chunk);
        file.write_all(&chunk).await?;
    }
    if size == 0 {
        return Err(error("The background video is empty"));
    }
    file.flush().await?;
    drop(file);
    let name = format!("{:x}.{extension}", hash.finalize());
    let path = dir.join(&name);
    if tokio::fs::metadata(&path)
        .await
        .map_or(true, |metadata| metadata.len() != size)
    {
        let destination = path.clone();
        tokio::task::spawn_blocking(move || {
            temporary.persist(destination).map_err(|err| err.error)
        })
        .await??;
    }
    write_metadata(
        dir,
        &CachedVideo {
            file: name,
            size,
            checked_at: now,
            etag,
            last_modified,
        },
    )
    .await?;
    Ok(path)
}

async fn ensure_video(
    root: &Path,
    url: &url::Url,
    client: &reqwest::Client,
    now: u64,
) -> crate::Result<PathBuf> {
    let dir =
        root.join(format!("{:x}", Sha256::digest(url.as_str().as_bytes())));
    let lock = {
        let mut entry = LOCKS.entry(dir.clone()).or_default();
        if let Some(lock) = entry.upgrade() {
            lock
        } else {
            let lock = Arc::new(Mutex::new(()));
            *entry = Arc::downgrade(&lock);
            lock
        }
    };
    let _guard = lock.lock().await;
    let cached = read_cached(&dir).await;
    if let Some(cached) = &cached
        && now.saturating_sub(cached.checked_at) < REVALIDATE_AFTER
    {
        return Ok(dir.join(&cached.file));
    }
    tokio::fs::create_dir_all(&dir).await?;
    match download_video(&dir, url, client, cached.as_ref(), now).await {
        Ok(path) => Ok(path),
        Err(_) if cached.is_some() => {
            tracing::warn!(
                "Background video refresh failed; retaining the complete local copy"
            );
            Ok(dir.join(cached.unwrap().file))
        }
        Err(error) => Err(error),
    }
}

/// Returns a local path, revalidating daily and retaining the last complete copy offline.
pub async fn cache_video(raw: &str) -> crate::Result<String> {
    let url = video_url(raw)?;
    let state = crate::State::get().await?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let root = state.directories.caches_dir().join("ymcl-home-videos");
    let dir =
        root.join(format!("{:x}", Sha256::digest(url.as_str().as_bytes())));
    let client = http1_fallback_client(
        crate::util::fetch::ProxyPolicy::System,
        false,
        false,
    )
    .await?;
    {
        let mut active = ACTIVE_DIRECTORIES.lock().await;
        if let Err(error) =
            prepare_cache(&root, &dir, &mut active, now, MAX_CACHE_BYTES).await
        {
            active.insert(dir.clone());
            tracing::warn!(%error, "Could not clean up inactive background videos");
        }
    }
    if let Some(cached) = read_cached(&dir).await {
        let path = dir.join(&cached.file);
        if now.saturating_sub(cached.checked_at) >= REVALIDATE_AFTER {
            tokio::spawn(async move {
                let _ = ensure_video(&root, &url, &client, now).await;
                let active = ACTIVE_DIRECTORIES.lock().await;
                let _ =
                    prune_inactive_cache(&root, &active, now, MAX_CACHE_BYTES)
                        .await;
            });
        }
        return Ok(path.to_string_lossy().to_string());
    }
    let path = ensure_video(&root, &url, &client, now).await?;
    {
        let active = ACTIVE_DIRECTORIES.lock().await;
        let _ =
            prune_inactive_cache(&root, &active, now, MAX_CACHE_BYTES).await;
    }
    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    async fn server(
        responses: Vec<String>,
    ) -> (url::Url, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = url::Url::parse(&format!(
            "http://{}/background.mp4",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for response in responses {
                let (mut stream, _) = tokio::time::timeout(
                    Duration::from_secs(5),
                    listener.accept(),
                )
                .await
                .unwrap()
                .unwrap();
                let mut bytes = Vec::new();
                while !bytes.ends_with(b"\r\n\r\n") {
                    bytes.push(stream.read_u8().await.unwrap());
                    assert!(bytes.len() < 16 * 1024);
                }
                requests.push(
                    String::from_utf8(bytes).unwrap().to_ascii_lowercase(),
                );
                stream.write_all(response.as_bytes()).await.unwrap();
                stream.shutdown().await.unwrap();
            }
            requests
        });
        (url, task)
    }

    fn response(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nETag: \"v1\"\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn background_video_waits_for_foreground_metadata_before_connecting()
    {
        let root = tempfile::tempdir().unwrap();
        let (url, requests) = server(vec![response("video-data")]).await;
        let client = client();
        let foreground = FOREGROUND_REQUESTS.start();
        let download = ensure_video(root.path(), &url, &client, 1);
        tokio::pin!(download);
        assert!(
            tokio::time::timeout(Duration::from_millis(40), &mut download)
                .await
                .is_err()
        );
        assert!(!requests.is_finished());
        drop(foreground);
        let path = tokio::time::timeout(Duration::from_secs(30), download)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(tokio::fs::read(path).await.unwrap(), b"video-data");
        let requests = requests.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].contains("priority: u=7"));
    }

    #[tokio::test]
    async fn transient_status_and_truncated_video_retry_until_a_complete_file()
    {
        let root = tempfile::tempdir().unwrap();
        let (url, requests) = server(vec![
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
            "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: 100\r\nConnection: close\r\n\r\ncut".to_string(),
            response("video-data"),
        ])
        .await;
        let path = ensure_video(root.path(), &url, &client(), 1).await.unwrap();
        assert_eq!(tokio::fs::read(&path).await.unwrap(), b"video-data");
        assert_eq!(requests.await.unwrap().len(), 3);
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            2
        );
    }

    #[tokio::test]
    async fn denied_video_requests_are_not_retried() {
        let root = tempfile::tempdir().unwrap();
        let (url, requests) = server(vec![
            "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
        ])
        .await;
        assert!(ensure_video(root.path(), &url, &client(), 1).await.is_err());
        assert_eq!(requests.await.unwrap().len(), 1);
    }

    async fn cached_fixture(
        root: &Path,
        key: char,
        checked_at: u64,
    ) -> (PathBuf, PathBuf) {
        let dir = root.join(key.to_string().repeat(64));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let name = format!("{}.mp4", "a".repeat(64));
        let file = dir.join(&name);
        tokio::fs::write(&file, b"video-data").await.unwrap();
        write_metadata(
            &dir,
            &CachedVideo {
                file: name,
                size: 10,
                checked_at,
                etag: None,
                last_modified: None,
            },
        )
        .await
        .unwrap();
        (dir, file)
    }

    #[tokio::test]
    async fn prunes_superseded_copies_on_first_use_but_keeps_session_paths() {
        let root = tempfile::tempdir().unwrap();
        let (dir, current) = cached_fixture(root.path(), 'a', 1).await;
        let old = dir.join(format!("{}.mp4", "b".repeat(64)));
        tokio::fs::write(&old, b"previous-video").await.unwrap();
        let mut active = HashSet::new();
        prepare_cache(root.path(), &dir, &mut active, 2, MAX_CACHE_BYTES)
            .await
            .unwrap();
        assert!(!old.exists());
        assert!(current.exists());

        tokio::fs::write(&old, b"playing-video").await.unwrap();
        prepare_cache(root.path(), &dir, &mut active, 3, MAX_CACHE_BYTES)
            .await
            .unwrap();
        assert!(old.exists());
        let (other, _) = cached_fixture(root.path(), 'c', 3).await;
        prepare_cache(root.path(), &other, &mut active, 4, 1)
            .await
            .unwrap();
        assert!(current.exists());
        assert!(old.exists());

        let mut next_session = HashSet::new();
        prepare_cache(root.path(), &dir, &mut next_session, 5, MAX_CACHE_BYTES)
            .await
            .unwrap();
        assert!(!old.exists());
        assert!(current.exists());
    }

    #[tokio::test]
    async fn removes_interrupted_temporary_files_before_activating_the_directory()
     {
        let root = tempfile::tempdir().unwrap();
        let (dir, current) = cached_fixture(root.path(), 'a', 1).await;
        let temporary = tempfile::NamedTempFile::new_in(&dir).unwrap();
        let (_, leftover) = temporary.keep().unwrap();
        tokio::fs::write(&leftover, b"interrupted-download")
            .await
            .unwrap();
        let mut active = HashSet::new();
        prepare_cache(root.path(), &dir, &mut active, 2, MAX_CACHE_BYTES)
            .await
            .unwrap();
        assert!(!leftover.exists());
        assert!(current.exists());
    }

    #[tokio::test]
    async fn removes_expired_and_oldest_inactive_directories_under_the_budget()
    {
        let root = tempfile::tempdir().unwrap();
        let (requested, _) =
            cached_fixture(root.path(), 'a', MAX_UNUSED_SECONDS + 10).await;
        let (expired, _) = cached_fixture(root.path(), 'b', 1).await;
        let (oldest, _) =
            cached_fixture(root.path(), 'c', MAX_UNUSED_SECONDS + 1).await;
        let (recent, recent_file) =
            cached_fixture(root.path(), 'd', MAX_UNUSED_SECONDS + 9).await;
        let mut active = HashSet::new();
        prepare_cache(
            root.path(),
            &requested,
            &mut active,
            MAX_UNUSED_SECONDS + 10,
            MAX_CACHE_BYTES,
        )
        .await
        .unwrap();
        assert!(!expired.exists());
        assert!(oldest.exists());
        assert!(recent.exists());

        let requested_size =
            tokio::fs::metadata(requested.join("metadata.json"))
                .await
                .unwrap()
                .len()
                + tokio::fs::metadata(
                    requested.join(format!("{}.mp4", "a".repeat(64))),
                )
                .await
                .unwrap()
                .len()
                + tokio::fs::metadata(requested.join("last-used"))
                    .await
                    .unwrap()
                    .len();
        let recent_size = tokio::fs::metadata(recent.join("metadata.json"))
            .await
            .unwrap()
            .len()
            + tokio::fs::metadata(recent_file).await.unwrap().len();
        let mut next_session = HashSet::new();
        prepare_cache(
            root.path(),
            &requested,
            &mut next_session,
            MAX_UNUSED_SECONDS + 11,
            requested_size + recent_size,
        )
        .await
        .unwrap();
        assert!(!oldest.exists());
        assert!(recent.exists());
        assert!(requested.exists());
    }

    #[tokio::test]
    async fn coalesces_downloads_reuses_disk_and_revalidates_without_redownloading()
     {
        let root = tempfile::tempdir().unwrap();
        let (url, requests) = server(vec![
            response("video-data"),
            "HTTP/1.1 304 Not Modified\r\nConnection: close\r\n\r\n"
                .to_string(),
        ])
        .await;
        let client = client();
        let (first, second) = tokio::join!(
            ensure_video(root.path(), &url, &client, 1),
            ensure_video(root.path(), &url, &client, 1)
        );
        let first = first.unwrap();
        assert_eq!(first, second.unwrap());
        assert_eq!(tokio::fs::read(&first).await.unwrap(), b"video-data");
        assert_eq!(
            ensure_video(root.path(), &url, &client, 2).await.unwrap(),
            first
        );
        assert_eq!(
            ensure_video(root.path(), &url, &client, 1 + REVALIDATE_AFTER)
                .await
                .unwrap(),
            first
        );
        let requests = requests.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[1].contains("if-none-match: \"v1\""));
        assert_eq!(
            read_cached(first.parent().unwrap())
                .await
                .unwrap()
                .checked_at,
            1 + REVALIDATE_AFTER
        );
        assert_eq!(
            ensure_video(root.path(), &url, &client, 1 + REVALIDATE_AFTER * 2)
                .await
                .unwrap(),
            first
        );
    }

    #[tokio::test]
    async fn interrupted_refresh_keeps_complete_copy_and_a_changed_video_gets_a_new_path()
     {
        let root = tempfile::tempdir().unwrap();
        let interrupted = "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: 100\r\nConnection: close\r\n\r\ncut".to_string();
        let (url, requests) = server(vec![
            response("original-video"),
            interrupted.clone(),
            interrupted.clone(),
            interrupted,
            response("replacement-video"),
        ])
        .await;
        let client = client();
        let first = ensure_video(root.path(), &url, &client, 1).await.unwrap();
        let failed =
            ensure_video(root.path(), &url, &client, 1 + REVALIDATE_AFTER)
                .await
                .unwrap();
        assert_eq!(first, failed);
        assert_eq!(
            read_cached(first.parent().unwrap())
                .await
                .unwrap()
                .checked_at,
            1
        );
        let next =
            ensure_video(root.path(), &url, &client, 1 + REVALIDATE_AFTER)
                .await
                .unwrap();
        assert_ne!(first, next);
        assert_eq!(tokio::fs::read(&first).await.unwrap(), b"original-video");
        assert_eq!(tokio::fs::read(&next).await.unwrap(), b"replacement-video");
        assert_eq!(
            read_cached(next.parent().unwrap()).await.unwrap().file,
            next.file_name().unwrap().to_str().unwrap()
        );
        assert_eq!(requests.await.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn incomplete_empty_html_and_oversized_responses_never_enter_the_cache()
     {
        for response in [
            "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: 100\r\nConnection: close\r\n\r\ncut".to_string(),
            response(""),
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 5\r\nConnection: close\r\n\r\nerror".to_string(),
            format!("HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", MAX_VIDEO_BYTES + 1),
        ] {
            let root = tempfile::tempdir().unwrap();
            let (url, request) = server(vec![response]).await;
            assert!(ensure_video(root.path(), &url, &client(), 1).await.is_err());
            request.await.unwrap();
            let mut directories = tokio::fs::read_dir(root.path()).await.unwrap();
            let dir = directories.next_entry().await.unwrap().unwrap().path();
            assert!(read_cached(&dir).await.is_none());
            assert!(tokio::fs::read_dir(dir).await.unwrap().next_entry().await.unwrap().is_none());
        }
    }

    #[tokio::test]
    async fn truncated_local_files_are_redownloaded() {
        let root = tempfile::tempdir().unwrap();
        let (url, requests) = server(vec![
            response("complete-video"),
            response("complete-video"),
        ])
        .await;
        let client = client();
        let first = ensure_video(root.path(), &url, &client, 1).await.unwrap();
        tokio::fs::write(&first, b"cut").await.unwrap();
        assert!(read_cached(first.parent().unwrap()).await.is_none());
        let next = ensure_video(root.path(), &url, &client, 2).await.unwrap();
        assert_eq!(first, next);
        assert_eq!(tokio::fs::read(next).await.unwrap(), b"complete-video");
        assert_eq!(requests.await.unwrap().len(), 2);
    }

    #[test]
    fn validates_remote_urls_cache_names_and_video_response_types() {
        for invalid in [
            "file:///etc/passwd",
            "data:video/mp4;base64,aA==",
            "https://user:pass@domain.test/video.mp4",
        ] {
            assert!(video_url(invalid).is_err());
        }
        assert!(!safe_cached_name("../video.mp4"));
        assert!(!safe_cached_name(&format!("{}.html", "a".repeat(64))));
        let url =
            video_url("https://domain.test/background.MP4?version=2").unwrap();
        assert_eq!(video_extension(None, &url).unwrap(), "mp4");
        assert_eq!(
            video_extension(Some("video/webm; codecs=vp9"), &url).unwrap(),
            "webm"
        );
        assert!(video_extension(Some("text/html"), &url).is_err());
    }
}

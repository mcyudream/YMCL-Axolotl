//! Multi-range download of one file over a shared HTTP/2 connection.

use super::h2_download::{H2DownloadFailure, H2DownloadOutcome};
use super::h2_pool::SharedH2Connection;
use crate::util::fetch::{
    self, DownloadRequest, DownloadResult, DownloadRoute,
};
use futures::stream::{FuturesUnordered, StreamExt};
use http::header::{ACCEPT_ENCODING, RANGE};
use http::{HeaderValue, StatusCode, Uri};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

struct H2Range {
    start: u64,
    end: u64,
}

struct PartCleanupGuard {
    path: PathBuf,
    armed: bool,
}

impl PartCleanupGuard {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for PartCleanupGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub(crate) async fn download(
    connection: &Arc<SharedH2Connection>,
    uri: &Uri,
    request: &DownloadRequest,
    route: &DownloadRoute,
    destination: &Path,
    part_path: &Path,
    total_size: u64,
    concurrency: usize,
) -> H2DownloadOutcome {
    if request
        .cancellation
        .as_ref()
        .is_some_and(tokio_util::sync::CancellationToken::is_cancelled)
    {
        return H2DownloadOutcome::Canceled;
    }
    if total_size == 0 {
        return H2DownloadOutcome::Fallback {
            failure: H2DownloadFailure::Content,
            preserve_partial: false,
        };
    }
    let count = concurrency
        .max(1)
        .min(usize::try_from(total_size).unwrap_or(usize::MAX).max(1));
    let mut cleanup = PartCleanupGuard::new(part_path);
    let output =
        match super::range_output::RangeOutput::create(part_path, total_size)
            .await
        {
            Ok(output) => output,
            Err(_) => {
                return H2DownloadOutcome::Fallback {
                    failure: H2DownloadFailure::Io,
                    preserve_partial: false,
                };
            }
        };
    let ranges = split_ranges(total_size, count);
    let downloaded = Arc::new(AtomicU64::new(0));
    let reported_bucket = Arc::new(AtomicU64::new(0));
    let progress_delta = (total_size / 200).max(256 * 1024);
    let mut tasks = FuturesUnordered::new();
    for range in ranges {
        tasks.push(download_range(
            Arc::clone(connection),
            uri.clone(),
            request,
            route,
            Arc::clone(&output),
            range,
            total_size,
            Arc::clone(&downloaded),
            Arc::clone(&reported_bucket),
            progress_delta,
        ));
    }
    loop {
        let next = if let Some(cancellation) = request.cancellation.as_ref() {
            tokio::select! {
                biased;
                _ = cancellation.cancelled() => {
                    drop(tasks);
                    drop(output);
                    return H2DownloadOutcome::Canceled;
                }
                result = tasks.next() => result,
            }
        } else {
            tasks.next().await
        };
        let Some(result) = next else {
            break;
        };
        if let Err(failure) = result {
            drop(tasks);
            drop(output);
            return H2DownloadOutcome::Fallback {
                failure,
                preserve_partial: false,
            };
        }
    }
    drop(output);
    let verification = if let Some(cancellation) = request.cancellation.as_ref()
    {
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => return H2DownloadOutcome::Canceled,
            result = fetch::verify_file(part_path, &request.integrity) => result,
        }
    } else {
        fetch::verify_file(part_path, &request.integrity).await
    };
    if let Err(error) = verification {
        let failure = if fetch::is_integrity_error(&error) {
            H2DownloadFailure::Integrity
        } else {
            H2DownloadFailure::Content
        };
        return H2DownloadOutcome::Fallback {
            failure,
            preserve_partial: false,
        };
    }
    let finalized = if let Some(cancellation) = request.cancellation.as_ref() {
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => return H2DownloadOutcome::Canceled,
            result = fetch::finalize_download(part_path, destination) => result,
        }
    } else {
        fetch::finalize_download(part_path, destination).await
    };
    if finalized.is_err() {
        return H2DownloadOutcome::Fallback {
            failure: H2DownloadFailure::Io,
            preserve_partial: false,
        };
    }
    cleanup.disarm();
    H2DownloadOutcome::Completed(DownloadResult {
        path: destination.to_path_buf(),
        url: uri.to_string(),
        source: route.source,
        size: total_size,
        attempts: 1,
        fallback_count: 0,
    })
}

#[allow(clippy::too_many_arguments)]
async fn download_range(
    connection: Arc<SharedH2Connection>,
    uri: Uri,
    request: &DownloadRequest,
    route: &DownloadRoute,
    output: Arc<super::range_output::RangeOutput>,
    range: H2Range,
    total_size: u64,
    downloaded: Arc<AtomicU64>,
    reported_bucket: Arc<AtomicU64>,
    progress_delta: u64,
) -> Result<(), H2DownloadFailure> {
    super::h2_download::record_install_stage(
        request,
        crate::install::DownloadItemStatus::WaitingForResource,
    )
    .await;
    let _permit = super::h2_stream_budget::acquire(route)
        .await
        .map_err(|_| H2DownloadFailure::Connect)?;
    super::h2_download::record_install_stage(
        request,
        crate::install::DownloadItemStatus::Downloading,
    )
    .await;
    let mut writer = output
        .open_range(range.start, range.end + 1)
        .await
        .map_err(|_| H2DownloadFailure::Io)?;
    let mut headers = super::h2_download::request_headers(request, route);
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    headers.insert(
        RANGE,
        HeaderValue::from_str(&format!("bytes={}-{}", range.start, range.end))
            .map_err(|_| H2DownloadFailure::Http)?,
    );
    let (response, mut stream) =
        super::h2_download::open_stream(&connection, &uri, headers)
            .await
            .map_err(|_| H2DownloadFailure::Protocol)?;
    if response.status() != StatusCode::PARTIAL_CONTENT
        || !content_range_matches(response.headers(), &range, total_size)
    {
        return Err(H2DownloadFailure::Http);
    }
    let activity = super::h2_receive::H2TransferActivity::begin();
    let mut offset = range.start;
    while let Some(chunk) =
        super::h2_receive::receive_chunk(&mut stream, "range")
            .await
            .map_err(|_| H2DownloadFailure::Protocol)?
    {
        let remaining = range.end.saturating_add(1).saturating_sub(offset);
        if chunk.len() as u64 > remaining {
            return Err(H2DownloadFailure::Protocol);
        }
        let accepted = chunk.len();
        writer
            .write_next(&chunk[..accepted])
            .await
            .map_err(|_| H2DownloadFailure::Io)?;
        activity.record_bytes(accepted);
        super::h2_receive::release_capacity(&mut stream, chunk.len())
            .map_err(|_| H2DownloadFailure::Protocol)?;
        offset += accepted as u64;
        let current = downloaded.fetch_add(accepted as u64, Ordering::Relaxed)
            + accepted as u64;
        let bucket = current / progress_delta;
        let previous = reported_bucket.fetch_max(bucket, Ordering::Relaxed);
        if current >= total_size || bucket > previous {
            super::h2_download::record_install_progress(
                request,
                current.min(total_size),
                total_size,
            )
            .await;
        }
    }
    (offset == range.end + 1)
        .then_some(())
        .ok_or(H2DownloadFailure::Protocol)?;
    writer.flush().await.map_err(|_| H2DownloadFailure::Io)
}

fn split_ranges(size: u64, count: usize) -> Vec<H2Range> {
    let base = size / count as u64;
    let remainder = size % count as u64;
    let mut start = 0;
    (0..count)
        .map(|index| {
            let length = base + u64::from(index < remainder as usize);
            let range = H2Range {
                start,
                end: start + length - 1,
            };
            start += length;
            range
        })
        .collect()
}

fn content_range_matches(
    headers: &http::HeaderMap,
    range: &H2Range,
    total_size: u64,
) -> bool {
    headers
        .get(http::header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                == format!("bytes {}-{}/{}", range.start, range.end, total_size)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use futures::future::poll_fn;
    use http::{Response, StatusCode};
    use sha1_smol::Sha1;

    #[test]
    fn splits_file_into_eight_contiguous_ranges() {
        let ranges = split_ranges(83, 8);
        assert_eq!(ranges.len(), 8);
        assert_eq!(ranges.first().unwrap().start, 0);
        assert_eq!(ranges.last().unwrap().end, 82);
        assert!(
            ranges
                .windows(2)
                .all(|pair| pair[0].end + 1 == pair[1].start)
        );
    }

    #[test]
    fn splits_file_into_sixteen_contiguous_ranges() {
        let ranges = split_ranges(1024 * 1024 + 7, 16);
        assert_eq!(ranges.len(), 16);
        assert_eq!(ranges.first().unwrap().start, 0);
        assert_eq!(ranges.last().unwrap().end, 1024 * 1024 + 6);
        assert!(
            ranges
                .windows(2)
                .all(|pair| pair[0].end + 1 == pair[1].start)
        );
    }

    #[tokio::test]
    async fn downloads_one_file_over_eight_h2_range_streams() {
        let data = Arc::new(
            (0..2 * 1024 * 1024)
                .map(|index| (index % 251) as u8)
                .collect::<Vec<_>>(),
        );
        let request_count = Arc::new(AtomicU64::new(0));
        let (client_io, server_io) = tokio::io::duplex(256 * 1024);
        let server_data = Arc::clone(&data);
        let server_requests = Arc::clone(&request_count);
        let server = tokio::spawn(async move {
            let mut connection =
                h2::server::handshake(server_io).await.unwrap();
            while let Some(result) = connection.accept().await {
                let (request, mut respond) = result.unwrap();
                let data = Arc::clone(&server_data);
                let requests = Arc::clone(&server_requests);
                tokio::spawn(async move {
                    requests.fetch_add(1, Ordering::Relaxed);
                    let value = request.headers()[RANGE].to_str().unwrap();
                    let value = value.strip_prefix("bytes=").unwrap();
                    let (start, end) = value.split_once('-').unwrap();
                    let start = start.parse::<usize>().unwrap();
                    let end = end.parse::<usize>().unwrap();
                    let response = Response::builder()
                        .status(StatusCode::PARTIAL_CONTENT)
                        .header(
                            http::header::CONTENT_RANGE,
                            format!("bytes {start}-{end}/{}", data.len()),
                        )
                        .body(())
                        .unwrap();
                    let mut stream =
                        respond.send_response(response, false).unwrap();
                    let mut offset = start;
                    while offset <= end {
                        let wanted = (end - offset + 1).min(16 * 1024);
                        stream.reserve_capacity(wanted);
                        let capacity = poll_fn(|cx| stream.poll_capacity(cx))
                            .await
                            .unwrap()
                            .unwrap();
                        let length = capacity.min(wanted);
                        let finished = offset + length > end;
                        stream
                            .send_data(
                                Bytes::copy_from_slice(
                                    &data[offset..offset + length],
                                ),
                                finished,
                            )
                            .unwrap();
                        offset += length;
                    }
                });
            }
        });

        let mut builder = h2::client::Builder::new();
        builder
            .initial_window_size(1024 * 1024)
            .initial_connection_window_size(64 * 1024 * 1024);
        let (sender, mut driver) =
            builder.handshake::<_, Bytes>(client_io).await.unwrap();
        driver.set_target_window_size(64 * 1024 * 1024);
        let client_driver = tokio::spawn(async move { driver.await });
        let connection = Arc::new(SharedH2Connection::for_test(sender));
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("pack.mrpack");
        let part_path = directory.path().join("pack.mrpack.part");
        let hash = Sha1::from(&data[..]).hexdigest();
        let request = DownloadRequest::new(
            "https://h2-range.test/pack.mrpack",
            fetch::ResourceClass::Modpack,
        )
        .with_integrity(
            fetch::Integrity::sha1(hash).with_size(data.len() as u64),
        );
        let route = DownloadRoute {
            url: request.url.clone(),
            source: fetch::DownloadRouteSource::Official,
            is_mirror: false,
            allow_sensitive_headers: true,
            supports_range: true,
            proxy: fetch::ProxyPolicy::Direct,
        };
        let uri = route.url.parse().unwrap();

        let result = download(
            &connection,
            &uri,
            &request,
            &route,
            &destination,
            &part_path,
            data.len() as u64,
            8,
        )
        .await;

        assert!(matches!(result, H2DownloadOutcome::Completed(_)));
        assert_eq!(request_count.load(Ordering::Relaxed), 8);
        assert_eq!(tokio::fs::read(destination).await.unwrap(), *data);
        client_driver.abort();
        server.abort();
    }

    #[tokio::test]
    async fn cancellation_stops_all_ranges_and_removes_partial_output() {
        let request_count = Arc::new(AtomicU64::new(0));
        let (client_io, server_io) = tokio::io::duplex(256 * 1024);
        let server_requests = Arc::clone(&request_count);
        let server = tokio::spawn(async move {
            let mut connection =
                h2::server::handshake(server_io).await.unwrap();
            while let Some(result) = connection.accept().await {
                let (request, mut respond) = result.unwrap();
                let requests = Arc::clone(&server_requests);
                tokio::spawn(async move {
                    requests.fetch_add(1, Ordering::Relaxed);
                    let value = request.headers()[RANGE].to_str().unwrap();
                    let value = value.strip_prefix("bytes=").unwrap();
                    let (start, end) = value.split_once('-').unwrap();
                    let response = Response::builder()
                        .status(StatusCode::PARTIAL_CONTENT)
                        .header(
                            http::header::CONTENT_RANGE,
                            format!("bytes {start}-{end}/2097152"),
                        )
                        .body(())
                        .unwrap();
                    let _stream =
                        respond.send_response(response, false).unwrap();
                    futures::future::pending::<()>().await;
                });
            }
        });

        let (sender, driver) = h2::client::handshake(client_io).await.unwrap();
        let client_driver = tokio::spawn(async move { driver.await });
        let connection = Arc::new(SharedH2Connection::for_test(sender));
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("pack.zip");
        let part_path = directory.path().join("pack.zip.part");
        let cancellation = tokio_util::sync::CancellationToken::new();
        let mut request = DownloadRequest::new(
            "https://h2-range.test/pack.zip",
            fetch::ResourceClass::CurseForge,
        )
        .with_integrity(fetch::Integrity::default().with_size(2 * 1024 * 1024));
        request.cancellation = Some(cancellation.clone());
        let route = DownloadRoute {
            url: request.url.clone(),
            source: fetch::DownloadRouteSource::Official,
            is_mirror: false,
            allow_sensitive_headers: true,
            supports_range: true,
            proxy: fetch::ProxyPolicy::Direct,
        };
        let uri = route.url.parse().unwrap();
        let part_for_task = part_path.clone();
        let destination_for_task = destination.clone();
        let download_task = tokio::spawn(async move {
            download(
                &connection,
                &uri,
                &request,
                &route,
                &destination_for_task,
                &part_for_task,
                2 * 1024 * 1024,
                16,
            )
            .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while request_count.load(Ordering::Relaxed) < 16 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        cancellation.cancel();
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            download_task,
        )
        .await
        .unwrap()
        .unwrap();

        assert!(matches!(outcome, H2DownloadOutcome::Canceled));
        assert!(!part_path.exists());
        assert!(!destination.exists());
        client_driver.abort();
        server.abort();
    }
}

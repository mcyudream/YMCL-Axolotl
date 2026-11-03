use futures::StreamExt;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::State;
use crate::api::ymcl::registry;

pub const CHUNK_BYTES: u64 = 8 * 1024 * 1024;
const MAX_BATCH_FILES: usize = 32;

#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PublishStage {
    Preparing,
    Packing,
    Uploading,
    Publishing,
    Mirroring,
    Completed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishProgress {
    pub stage: PublishStage,
    pub uploaded_bytes: u64,
    pub total_bytes: u64,
    pub batch: usize,
    pub total_batches: usize,
    pub current_file: Option<String>,
}

pub type PublishProgressCallback = Arc<dyn Fn(PublishProgress) + Send + Sync>;

#[derive(Clone)]
pub(super) struct Progress {
    inner: Arc<Mutex<(PublishProgress, Instant)>>,
    callback: PublishProgressCallback,
}

impl Progress {
    pub fn new(callback: PublishProgressCallback) -> Self {
        let progress = Self {
            inner: Arc::new(Mutex::new((
                PublishProgress {
                    stage: PublishStage::Preparing,
                    uploaded_bytes: 0,
                    total_bytes: 0,
                    batch: 0,
                    total_batches: 0,
                    current_file: None,
                },
                Instant::now(),
            ))),
            callback,
        };
        progress.update(true, |_| {});
        progress
    }

    fn update(&self, force: bool, update: impl FnOnce(&mut PublishProgress)) {
        let snapshot = {
            let mut inner =
                self.inner.lock().unwrap_or_else(|error| error.into_inner());
            update(&mut inner.0);
            if !force && inner.1.elapsed() < Duration::from_millis(100) {
                return;
            }
            inner.1 = Instant::now();
            inner.0.clone()
        };
        (self.callback)(snapshot);
    }

    pub fn stage(&self, stage: PublishStage) {
        self.update(true, |progress| {
            progress.stage = stage;
            progress.current_file = None;
        });
    }

    pub fn start_upload(&self, total_bytes: u64, total_batches: usize) {
        self.update(true, |progress| {
            progress.stage = PublishStage::Uploading;
            progress.total_bytes = total_bytes;
            progress.total_batches = total_batches;
            progress.batch = usize::from(total_batches > 0);
        });
    }

    pub fn batch(&self, batch: usize, uploaded_bytes: u64) {
        self.update(true, |progress| {
            progress.batch = batch;
            progress.uploaded_bytes = uploaded_bytes;
        });
    }

    pub async fn part(
        &self,
        path: &Path,
        offset: u64,
        length: u64,
        name: &str,
        uploaded_before: u64,
        publish_after_upload: bool,
    ) -> crate::Result<reqwest::multipart::Part> {
        let mut file = tokio::fs::File::open(path).await?;
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        let progress = self.clone();
        let name = name.to_string();
        self.update(true, |progress| {
            progress.current_file = Some(name.clone())
        });
        let mut sent = 0;
        let stream = tokio_util::io::ReaderStream::with_capacity(
            file.take(length),
            256 * 1024,
        )
        .map(move |chunk| {
            if let Ok(bytes) = &chunk {
                sent += bytes.len() as u64;
                progress.update(sent == length, |progress| {
                    progress.uploaded_bytes = uploaded_before + sent;
                    progress.current_file = Some(name.clone());
                    if publish_after_upload && sent == length {
                        progress.stage = PublishStage::Publishing;
                    }
                });
            }
            chunk
        });
        Ok(reqwest::multipart::Part::stream_with_length(
            reqwest::Body::wrap_stream(stream),
            length,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(size: u64) -> UploadFile {
        UploadFile {
            path: PathBuf::from("mods/test.jar"),
            name: "mods/test.jar".to_string(),
            sha512: "a".repeat(128),
            size,
        }
    }

    #[test]
    fn bounds_batches_by_bytes_and_count_and_splits_large_files() {
        let mut files = vec![file(CHUNK_BYTES * 2 + 7), file(CHUNK_BYTES / 2)];
        files.extend((0..70).map(|_| file(1)));
        files.push(file(0));
        let plan = batches(&files);
        let mut sizes = vec![0; files.len()];
        let mut offsets = vec![0; files.len()];
        for batch in &plan {
            assert!(batch.len() <= MAX_BATCH_FILES);
            assert!(
                batch.iter().map(|chunk| chunk.length).sum::<u64>()
                    <= CHUNK_BYTES
            );
            for chunk in batch {
                assert_eq!(chunk.index * CHUNK_BYTES, offsets[chunk.file]);
                sizes[chunk.file] += chunk.length;
                offsets[chunk.file] += chunk.length;
            }
        }
        assert_eq!(
            sizes,
            files.iter().map(|file| file.size).collect::<Vec<_>>()
        );
        assert_eq!(
            plan.iter()
                .flatten()
                .filter(|chunk| chunk.file == 0)
                .count(),
            3
        );
        assert_eq!(
            plan.iter()
                .flatten()
                .filter(|chunk| chunk.file == files.len() - 1)
                .count(),
            1
        );
    }

    #[test]
    fn exact_chunk_boundary_does_not_add_an_empty_chunk() {
        let plan = batches(&[file(CHUNK_BYTES)]);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].len(), 1);
        assert_eq!(plan[0][0].length, CHUNK_BYTES);
        assert!(batches(&[]).is_empty());
    }

    #[tokio::test]
    async fn streams_only_the_requested_chunk_with_absolute_progress() {
        let directory = tempfile::TempDir::new().unwrap();
        let path = directory.path().join("data");
        tokio::fs::write(&path, b"beforeCONTENTafter")
            .await
            .unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        let progress = Progress::new(Arc::new(move |event| {
            captured.lock().unwrap().push(event)
        }));
        progress.start_upload(20, 2);
        progress.batch(2, 10);
        let part = progress.part(&path, 6, 7, "data", 10, false).await.unwrap();
        let form = reqwest::multipart::Form::new().part("file", part);
        let mut body = form.into_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = body.next().await {
            bytes.extend_from_slice(&chunk.unwrap());
        }
        assert!(bytes.windows(7).any(|window| window == b"CONTENT"));
        assert!(!bytes.windows(6).any(|window| window == b"before"));
        assert!(!bytes.windows(5).any(|window| window == b"after"));
        let events = events.lock().unwrap();
        let event = events.last().unwrap();
        assert_eq!(event.uploaded_bytes, 17);
        assert_eq!(event.batch, 2);
        assert_eq!(event.stage, PublishStage::Uploading);
    }
}

#[derive(Clone, Debug)]
pub(super) struct UploadFile {
    pub path: PathBuf,
    pub name: String,
    pub sha512: String,
    pub size: u64,
}

#[derive(Clone, Debug)]
struct Chunk {
    file: usize,
    index: u64,
    length: u64,
}

fn batches(files: &[UploadFile]) -> Vec<Vec<Chunk>> {
    let mut batches = Vec::new();
    let mut batch = Vec::new();
    let mut bytes = 0;
    for (file, entry) in files.iter().enumerate() {
        for index in 0..entry.size.div_ceil(CHUNK_BYTES).max(1) {
            let length = (entry.size - index * CHUNK_BYTES).min(CHUNK_BYTES);
            if !batch.is_empty()
                && (bytes + length > CHUNK_BYTES
                    || batch.len() == MAX_BATCH_FILES)
            {
                batches.push(std::mem::take(&mut batch));
                bytes = 0;
            }
            batch.push(Chunk {
                file,
                index,
                length,
            });
            bytes += length;
        }
    }
    if !batch.is_empty() {
        batches.push(batch);
    }
    batches
}

pub(super) struct PublishClient {
    client: reqwest::Client,
    base: String,
    token: Option<String>,
}

impl PublishClient {
    pub async fn new(base: &str) -> crate::Result<Self> {
        let state = State::get().await?;
        let domain = registry::active_domain_id(&state.pool).await?;
        let session = crate::api::ymcl::auth::ensure_session(&domain).await?;
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(600))
                .build()?,
            base: base.to_string(),
            token: session.map(|session| session.access_token),
        })
    }

    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> reqwest::RequestBuilder {
        let mut request =
            self.client.request(method, format!("{}{path}", self.base));
        if let Some(token) = &self.token {
            request = request.header("Authorization", token);
        }
        request
    }

    pub async fn post(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> crate::Result<serde_json::Value> {
        let response = self
            .request(reqwest::Method::POST, path)
            .multipart(form)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            let detail = if status.as_u16() == 524 {
                "网关等待服务端超时（524）。请检查该版本是否已发布，再重试。"
                    .to_string()
            } else {
                format!("Publish failed ({status}): {text}")
            };
            return Err(crate::ErrorKind::OtherError(detail).into());
        }
        Ok(serde_json::from_str(&text)?)
    }

    pub async fn upload(
        &self,
        pack_id: &str,
        files: &[UploadFile],
        progress: &Progress,
    ) -> crate::Result<()> {
        if files.is_empty() {
            return Ok(());
        }
        let path = format!("/api/packs/{pack_id}/ingest/objects");
        let response = self.request(reqwest::Method::GET, &path).send().await?;
        if matches!(response.status().as_u16(), 404 | 405) {
            return Err(crate::ErrorKind::OtherError(
				"域适配器尚不支持分批上传，请将 YMCL 适配器升级到 0.16.0 或更高版本。".to_string()
			).into());
        }
        let limits: serde_json::Value =
            response.error_for_status()?.json().await?;
        if limits["chunkBytes"].as_u64() != Some(CHUNK_BYTES) {
            return Err(crate::ErrorKind::OtherError(
                "域适配器分批上传协议不兼容".to_string(),
            )
            .into());
        }
        let batches = batches(files);
        progress.start_upload(
            files.iter().map(|file| file.size).sum(),
            batches.len(),
        );
        let mut uploaded = 0;
        for (index, batch) in batches.iter().enumerate() {
            progress.batch(index + 1, uploaded);
            let mut form = reqwest::multipart::Form::new();
            let mut objects = Vec::new();
            let mut position = uploaded;
            for (part_index, chunk) in batch.iter().enumerate() {
                let file = &files[chunk.file];
                let part = progress
                    .part(
                        &file.path,
                        chunk.index * CHUNK_BYTES,
                        chunk.length,
                        &file.name,
                        position,
                        false,
                    )
                    .await?
                    .mime_str("application/octet-stream")?;
                form = form.part(format!("file{part_index}"), part);
                objects.push(serde_json::json!({"sha512": file.sha512, "size": file.size, "chunk": chunk.index}));
                position += chunk.length;
            }
            let result = self
                .post(
                    &path,
                    form.text("objects", serde_json::to_string(&objects)?),
                )
                .await?;
            if result["accepted"].as_u64() != Some(batch.len() as u64) {
                return Err(crate::ErrorKind::OtherError(
                    "服务端未确认完整的上传批次".to_string(),
                )
                .into());
            }
            uploaded = position;
        }
        Ok(())
    }
}

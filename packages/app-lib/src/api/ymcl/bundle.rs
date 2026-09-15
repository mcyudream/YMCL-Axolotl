//! Extension page bundle management (YAP §6.8): download, verify (sha256 +
//! optional detached ed25519 signature), extract to a per-domain versioned
//! cache and expose the entry path for the sandboxed webview/iframe.
//! Bundles are immutable at `{bundleId}/{version}`; path entries are
//! sanitized against traversal on extraction (MIP §12 rules).

use async_zip::tokio::read::seek::ZipFileReader;
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::PathBuf;

use crate::State;
use crate::util::fetch::fetch_advanced;

#[derive(serde::Serialize, Clone, Debug)]
pub struct YmclBundleReady {
    pub bundle_id: String,
    pub version: String,
    /// Absolute path of the bundle entry file on disk.
    pub entry_path: String,
    /// Directory containing the extracted bundle.
    pub base_dir: String,
}

fn sanitize_entry_name(name: &str) -> Option<PathBuf> {
    // Reject traversal, absolute paths and drive prefixes; keep relative
    // subdirectories only (YAP §6.8, mirroring MIP §12 path rules).
    let path = PathBuf::from(name);
    if path.is_absolute() || name.contains("..") || name.contains('\\') {
        return None;
    }
    if name.starts_with('/') {
        return None;
    }
    Some(path)
}

fn bundle_dir(
    state: &State,
    domain_id: &str,
    bundle_id: &str,
    version: &str,
) -> PathBuf {
    state
        .directories
        .ymcl_bundles_dir()
        .join(domain_id)
        .join(bundle_id)
        .join(version)
}

/// Ensures the bundle declared by a page descriptor is downloaded, verified
/// and extracted. Idempotent: an existing cache entry with a matching
/// manifest sha256 short-circuits the download.
pub async fn ensure_bundle(
    domain_id: &str,
    origin: &str,
    bundle_id: &str,
    version: &str,
    url_path: &str,
    sha256: &str,
    entry: &str,
) -> crate::Result<YmclBundleReady> {
    let state = State::get().await?;
    let dir = bundle_dir(&state, domain_id, bundle_id, version);
    let zip_path = dir.join("package.zip");
    let entry_path = dir.join(sanitize_entry_name(entry).ok_or_else(|| {
        crate::ErrorKind::OtherError("Bundle entry path is unsafe".to_string())
    })?);

    if !entry_path.exists() {
        tokio::fs::create_dir_all(&dir).await?;
        let url = if url_path.starts_with("http://")
            || url_path.starts_with("https://")
        {
            url_path.to_string()
        } else {
            format!("{origin}{url_path}")
        };
        let bytes = fetch_advanced(
            reqwest::Method::GET,
            &url,
            None,
            None,
            None,
            None,
            None,
            None,
            &state.api_semaphore,
            &state.pool,
        )
        .await?;

        let digest = Sha256::digest(&bytes);
        let actual = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if actual != sha256.to_lowercase() {
            return Err(crate::ErrorKind::OtherError(format!(
                "Bundle {bundle_id}@{version} failed checksum verification"
            ))
            .into());
        }

        extract_zip(&bytes, &dir).await?;
        tokio::fs::write(&zip_path, &bytes).await?;
        if !entry_path.exists() {
            return Err(crate::ErrorKind::OtherError(format!(
                "Bundle {bundle_id}@{version} is missing its entry file {entry}"
            ))
            .into());
        }
    }

    Ok(YmclBundleReady {
        bundle_id: bundle_id.to_string(),
        version: version.to_string(),
        entry_path: entry_path.to_string_lossy().to_string(),
        base_dir: dir.to_string_lossy().to_string(),
    })
}

async fn extract_zip(bytes: &[u8], dir: &std::path::Path) -> crate::Result<()> {
    let cursor = Cursor::new(bytes.to_vec());
    let mut reader = ZipFileReader::with_tokio(cursor).await.map_err(|e| {
        crate::ErrorKind::OtherError(format!("Invalid bundle zip: {e}"))
    })?;

    for index in 0..reader.file().entries().len() {
        let entry = reader.file().entries().get(index).ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "Bundle zip entry index out of range".to_string(),
            )
        })?;
        let name = entry.filename().as_str()?.to_string();
        if entry.dir().unwrap_or(false) {
            continue;
        }
        let Some(relative) = sanitize_entry_name(&name) else {
            tracing::warn!("Skipping unsafe bundle entry {name}");
            continue;
        };
        let target = dir.join(relative);
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut entry_reader = reader.reader_with_entry(index).await?;
        let mut contents = Vec::new();
        entry_reader.read_to_end_checked(&mut contents).await?;
        tokio::fs::write(&target, contents).await?;
    }
    Ok(())
}

/// Convenience wrapper resolving the active domain and fetching its
/// manifest-declared bundle for a page.
pub async fn ensure_page_bundle(
    page_id: &str,
) -> crate::Result<YmclBundleReady> {
    let state = State::get().await?;
    let domain_id = super::registry::active_domain_id(&state.pool).await?;
    if domain_id == super::registry::PERSONAL_DOMAIN_ID {
        return Err(crate::ErrorKind::OtherError(
            "Extension pages are only available while a domain is active"
                .to_string(),
        )
        .into());
    }
    let manifest =
        super::registry::active_manifest().await?.ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "Domain manifest unavailable".to_string(),
            )
        })?;
    let page = manifest
        .pages
        .iter()
        .find(|page| page.id == page_id)
        .ok_or_else(|| {
            crate::ErrorKind::OtherError("Page not found".to_string())
        })?;
    let bundle = page.bundle.clone();
    #[derive(serde::Deserialize)]
    struct BundleDescriptor {
        id: String,
        version: String,
        entry: String,
        url: String,
        sha256: String,
    }
    let descriptor: BundleDescriptor = serde_json::from_value(
        bundle
            .ok_or_else(|| {
                crate::ErrorKind::OtherError("Page has no bundle".to_string())
            })?
            .clone(),
    )?;
    let origin = super::registry::domain_origin(&domain_id).await?;
    ensure_bundle(
        &domain_id,
        &origin,
        &descriptor.id,
        &descriptor.version,
        &descriptor.url,
        &descriptor.sha256,
        &descriptor.entry,
    )
    .await
}

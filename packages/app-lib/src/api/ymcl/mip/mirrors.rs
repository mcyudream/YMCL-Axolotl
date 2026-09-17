//! GitHub 资源的域 CAS 镜像（YAP §7 扩展）：发布端把 GitHub 托管的资源
//! （如 Cleanroom loader 安装包与版本清单）主动推入适配器 CAS 并按原始
//! URL 登记；玩家侧下载同 URL 时镜像优先、GitHub 直连兜底——GitHub 不可
//! 达的环境也能完成安装。镜像不改变信任模型：sha512 仍是唯一信任锚。

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use reqwest::Method;
use serde::Deserialize;
use sha2::{Digest, Sha512};

use super::publish;
use crate::State;
use crate::api::ymcl::registry;
use crate::util::fetch::{FetchSemaphore, fetch, fetch_json};

/// Lookup results are cached so repeated GitHub fetches (a loader install
/// touches the releases listing and the installer) do not hammer the
/// adapter. Hits outlive misses so a freshly pushed mirror is picked up
/// quickly by the next install.
const HIT_TTL: Duration = Duration::from_secs(600);
const MISS_TTL: Duration = Duration::from_secs(60);

static LOOKUP_CACHE: LazyLock<Mutex<HashMap<String, (Option<MirrorHit>, Instant)>>> =
	LazyLock::new(|| Mutex::new(HashMap::new()));

/// A mirror registration served by the adapter. `object_url` is absolute and
/// regenerated per request origin by the adapter, so it stays reachable from
/// wherever the lookup was made.
#[derive(Deserialize, Clone, Debug)]
pub struct MirrorHit {
	pub url: String,
	pub sha512: String,
	#[serde(default)]
	pub size: Option<u64>,
	#[serde(default)]
	pub kind: Option<String>,
	#[serde(rename = "objectUrl")]
	pub object_url: String,
}

/// GitHub hosts are the mirror-eligible set: release assets (`github.com`)
/// and the API listing the launcher reads (`api.github.com`). Other loader
/// ecosystems (Forge/NeoForge maven) are not GitHub-hosted and stay direct.
pub(crate) fn is_github_url(url: &str) -> bool {
	url.split("://")
		.nth(1)
		.and_then(|rest| rest.split(['/', '?', '#']).next())
		.is_some_and(|host| {
			let host = host.to_ascii_lowercase();
			host == "github.com" || host == "api.github.com" || host.ends_with(".github.com")
		})
}

/// Minimal percent-encoding for a query value: everything outside the
/// unreserved set is escaped, so URLs (with `/?&=`) survive as one value.
fn encode_query_value(value: &str) -> String {
	value
		.chars()
		.map(|ch| {
			if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
				ch.to_string()
			} else {
				format!("%{:02X}", ch as u32)
			}
		})
		.collect()
}

fn lookup_url(mip_base: &str, url: &str) -> String {
	format!(
		"{}/api/mirrors?url={}",
		mip_base.trim_end_matches('/'),
		encode_query_value(url)
	)
}

/// Resolves the active domain's MIP base URL, or `None` when no domain is
/// active (personal domain) or the domain has no MIP face — neither can host
/// mirrors. Uses the cached capabilities snapshot without re-probing: a
/// domain that just gained its MIP face self-heals on the next refresh.
async fn active_mip_base(pool: &sqlx::SqlitePool) -> Option<String> {
	let active = registry::active_domain_id(pool).await.ok()?;
	if active == registry::PERSONAL_DOMAIN_ID {
		return None;
	}
	let capabilities = registry::domain_capabilities(&active).await.ok()?;
	capabilities
		.mip
		.as_ref()
		.and_then(|mip| mip.base_url.clone())
}

/// Looks up the mirror for a GitHub URL through the cache. `None` means
/// "no usable mirror" — not GitHub-hosted, no domain MIP face, lookup
/// failed, or not registered; callers fall back to the origin.
pub(crate) async fn lookup(
	url: &str,
	fetch_semaphore: &FetchSemaphore,
	pool: &sqlx::SqlitePool,
) -> Option<MirrorHit> {
	if !is_github_url(url) {
		return None;
	}
	if let Some((hit, at)) = LOOKUP_CACHE.lock().get(url).cloned() {
		let ttl = if hit.is_some() { HIT_TTL } else { MISS_TTL };
		if at.elapsed() < ttl {
			return hit;
		}
	}
	let Some(mip_base) = active_mip_base(pool).await else {
		// 没有域 MIP 面：短缓存避免每次 GitHub 拉取都打 DB。
		cache_miss(url);
		return None;
	};
	let hit = fetch_json::<MirrorHit>(
		Method::GET,
		&lookup_url(&mip_base, url),
		None,
		None,
		None,
		fetch_semaphore,
		pool,
	)
	.await
	.ok();
	match hit {
		Some(mut hit) => {
			// The adapter reports object URLs as `{origin}/mip/objects/{sha}`,
			// but plugin endpoints are only reachable through the dispatch
			// prefix the advertised MIP base already carries — the same quirk
			// `update::candidate_object_urls` defends against for manifest
			// CAS sources. Rebuild the object URL from the advertised face.
			hit.object_url = format!(
				"{}/objects/{}",
				mip_base.trim_end_matches('/'),
				hit.sha512
			);
			LOOKUP_CACHE
				.lock()
				.insert(url.to_string(), (Some(hit.clone()), Instant::now()));
			Some(hit)
		}
		None => {
			cache_miss(url);
			None
		}
	}
}

fn cache_miss(url: &str) {
	LOOKUP_CACHE
		.lock()
		.insert(url.to_string(), (None, Instant::now()));
}

fn sha512_hex(bytes: &[u8]) -> String {
	let digest = Sha512::digest(bytes);
	digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Downloads a URL preferring the domain CAS mirror, falling back to a
/// direct fetch. Only GitHub URLs are mirror-eligible; a mirror whose bytes
/// fail sha512 verification is abandoned (warn + fallback) so a corrupt or
/// stale mirror can never break an install that direct fetch could serve.
pub(crate) async fn fetch_mirrored_or_direct(
	url: &str,
	fetch_semaphore: &FetchSemaphore,
	pool: &sqlx::SqlitePool,
) -> crate::Result<bytes::Bytes> {
	if let Some(mirror) = lookup(url, fetch_semaphore, pool).await {
		match fetch(&mirror.object_url, None, None, None, fetch_semaphore, pool).await {
			Ok(bytes) if sha512_hex(&bytes) == mirror.sha512 => {
				tracing::debug!(url, "Served GitHub resource from the domain CAS mirror");
				return Ok(bytes);
			}
			Ok(_) => {
				tracing::warn!(url, "Domain mirror hash mismatch; falling back to GitHub");
			}
			Err(error) => {
				tracing::warn!(url, error = %error, "Domain mirror fetch failed; falling back to GitHub");
			}
		}
	}
	fetch(url, None, None, None, fetch_semaphore, pool).await
}

/// Publish-side mirror push (YAP §7 主动镜像): fetch the resource from its
/// origin on the publishing machine (which can reach GitHub), then register
/// it in the adapter's CAS. `force` re-pushes even when a registration
/// already exists — used for content that evolves (the releases listing);
/// immutable per-version assets (installer jars) skip the re-upload.
/// Returns a human-readable status for the publish report.
pub(crate) async fn ensure_mirrored(
	mip_base: &str,
	url: &str,
	kind: &str,
	force: bool,
) -> serde_json::Value {
	let entry = |status: &str, detail: Option<String>| {
		let mut value = serde_json::json!({
			"url": url,
			"kind": kind,
			"status": status,
		});
		if let Some(detail) = detail {
			value["detail"] = serde_json::Value::String(detail);
		}
		value
	};
	if !is_github_url(url) {
		return entry("skipped", Some("not a GitHub URL".to_string()));
	}
	let Ok(state) = State::get().await else {
		return entry("failed", Some("launcher state unavailable".to_string()));
	};
	if !force {
		let existing = fetch_json::<MirrorHit>(
			Method::GET,
			&lookup_url(mip_base, url),
			None,
			None,
			None,
			&state.api_semaphore,
			&state.pool,
		)
		.await;
		if existing.is_ok() {
			return entry("skipped", Some("already mirrored".to_string()));
		}
	}
	let bytes = match fetch(url, None, None, None, &state.api_semaphore, &state.pool).await {
		Ok(bytes) => bytes,
		Err(error) => return entry("failed", Some(error.to_string())),
	};
	let sha512 = sha512_hex(&bytes);
	let file_part = match reqwest::multipart::Part::bytes(bytes.to_vec())
		.mime_str("application/octet-stream")
	{
		Ok(part) => part,
		Err(error) => return entry("failed", Some(error.to_string())),
	};
	let form = reqwest::multipart::Form::new()
		.text("url", url.to_string())
		.text("sha512", sha512)
		.text("kind", kind.to_string())
		.part("file", file_part);
	if let Err(error) = publish::post_multipart(mip_base, "/api/mirrors", form).await {
		return entry("failed", Some(error.to_string()));
	}
	entry("mirrored", None)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn github_urls_are_mirror_eligible() {
		assert!(is_github_url(
			"https://github.com/CleanroomMC/Cleanroom/releases/download/\
			 0.6.11-alpha/cleanroom-0.6.11-alpha-installer.jar"
		));
		assert!(is_github_url(crate::api::loader_metadata::CLEANROOM_RELEASES_URL));
		assert!(is_github_url("http://github.com/owner/repo/asset.jar"));
		assert!(!is_github_url(
			"https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml"
		));
		assert!(!is_github_url("https://cdn.modrinth.com/data/abc/versions/xyz.jar"));
		// Look-alike hosts must not match: the suffix check anchors on dot
		// boundaries, so a GitHub prefix glued to another domain misses.
		assert!(!is_github_url("https://github.com.evil.example/asset.jar"));
		assert!(!is_github_url("https://notgithub.com/owner/repo"));
	}

	#[test]
	fn query_value_encoding_keeps_url_as_one_value() {
		let encoded = encode_query_value(
			"https://github.com/CleanroomMC/Cleanroom/releases/download/\
			 v1/cleanroom-v1-installer.jar",
		);
		assert!(!encoded.contains(['/', ':']));
		assert_eq!(
			lookup_url("https://example.com/mip", "https://github.com/a?b=1"),
			"https://example.com/mip/api/mirrors?url=https%3A%2F%2Fgithub.com%2Fa%3Fb%3D1"
		);
	}
}

//! mc-interop v1 client: cross-site discovery, catalog and union probing.
//!
//! A site publishes exactly one exit document at
//! `/.well-known/mc-interop-v1.json` (see the mc-interop v1 spec): site
//! identity, admin-selected pack/instance entries and peer sites. Unlike YAP
//! domains an interop site has no nav/pages manifest — the launcher renders
//! its `packs[]` as a plain catalog and joins the embedded server addresses.
//!
//! The same origins may run the `yudream-union` federated-auth plugin; its
//! discovery document at `/api/plugins/yudream-union/peer/meta` advertises the
//! site's Yggdrasil root so accounts can log in against it (union one-click
//! access). All structs tolerate unknown fields (spec §2.2).

use serde::{Deserialize, Serialize};

use crate::util::fetch::fetch_json;

/// Protocol path of the unified exit document. The major version lives in the
/// path; published paths never change meaning (spec §2.1).
pub const INTEROP_WELL_KNOWN_PATH: &str = "/.well-known/mc-interop-v1.json";

/// YDA 插件端点路径（well-known 未网关转发时的回退发现地址）。
pub const FALLBACK_CATALOG_PATH: &str = "/api/plugins/mc-interop/interop/v1/site";

/// Discovery path of the yudream-union federated-auth plugin.
pub const UNION_META_PATH: &str =
    "/api/plugins/yudream-union/peer/meta";

/// Union major version this launcher understands.
pub const UNION_VERSION: &str = "1";

pub fn interop_url(origin: &str) -> String {
    format!(
        "{}{INTEROP_WELL_KNOWN_PATH}",
        origin.trim_end_matches('/')
    )
}

pub fn union_meta_url(origin: &str) -> String {
    format!("{}{UNION_META_PATH}", origin.trim_end_matches('/'))
}

/// `site` block of the exit document (spec §3).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
pub struct InteropSiteInfo {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
}

/// Distribution backend pointer (spec §4). Launchers that understand a
/// system use its native reconciliation; others fall back to the full
/// artifact download.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
pub struct InteropDistribution {
    pub system: String,
    #[serde(default)]
    pub pack_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
}

/// Artifact of a pack entry (spec §4). Both hashes are always present in the
/// wire format; unknown ones arrive as `null`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
pub struct InteropArtifact {
    #[serde(default)]
    pub format: Option<String>,
    pub url: String,
    #[serde(default)]
    pub size: Option<i64>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
    #[serde(default)]
    pub distribution: Option<InteropDistribution>,
}

/// Server bound to an entry (spec §5). `port` omitted means "unspecified —
/// let SRV records or the client default decide".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
pub struct InteropServer {
    pub address: String,
    #[serde(default)]
    pub port: Option<u16>,
}

/// Source attribution of a merged peer entry (spec §7.2). Present only on
/// foreign entries.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
pub struct InteropSource {
    pub site_id: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub stale: Option<bool>,
}

/// One catalog entry: a pack (optional) bound to exactly one server (spec §4).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
pub struct InteropPackEntry {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub source: Option<InteropSource>,
    #[serde(default)]
    pub revision: Option<i64>,
    pub game_version: Option<String>,
    #[serde(default)]
    pub loader: Option<String>,
    #[serde(default)]
    pub loader_version: Option<String>,
    #[serde(default)]
    pub artifact: Option<InteropArtifact>,
    #[serde(default)]
    pub server: Option<InteropServer>,
}

impl InteropPackEntry {
    /// Whether this entry carries a downloadable pack (vs a plain server).
    pub fn has_artifact(&self) -> bool {
        self.artifact.is_some()
    }
}

/// Peer site declaration (spec §7.1). Launchers may ignore it entirely; the
/// site-side aggregator is the consumer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
pub struct InteropPeerEntry {
    pub site_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub manifest_url: Option<String>,
}

/// The unified exit document (spec §3).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct InteropSiteDocument {
    #[serde(default)]
    pub site: Option<InteropSiteInfo>,
    #[serde(default)]
    pub packs: Vec<InteropPackEntry>,
    #[serde(default)]
    pub peers: Vec<InteropPeerEntry>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl InteropSiteDocument {
    /// Display name: the site's own name, else its id, else the origin.
    pub fn display_name(&self, origin: &str) -> String {
        self.site
            .as_ref()
            .and_then(|site| {
                site.name
                    .as_deref()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
            })
            .or_else(|| {
                self.site.as_ref().map(|site| site.id.clone())
            })
            .unwrap_or_else(|| origin.to_string())
    }
}

/// `site` block of the union discovery document.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
pub struct UnionSiteInfo {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub origin: Option<String>,
}

/// `authlib` block: the site's Yggdrasil API root.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
pub struct UnionAuthlibInfo {
    #[serde(default, rename = "apiRoot")]
    pub api_root: Option<String>,
}

/// yudream-union discovery document (union protocol §3.1). Absent fields are
/// tolerated; `unionVersion` decides compatibility.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UnionMeta {
    #[serde(default, rename = "unionVersion")]
    pub union_version: Option<String>,
    #[serde(default)]
    pub site: Option<UnionSiteInfo>,
    #[serde(default)]
    pub authlib: Option<UnionAuthlibInfo>,
    #[serde(default, rename = "textureBaseUrl")]
    pub texture_base_url: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
}

impl UnionMeta {
    /// Whether this launcher speaks the site's union major version.
    pub fn is_supported(&self) -> bool {
        self.union_version.as_deref() == Some(UNION_VERSION)
    }
}

/// Fetches and parses the unified exit document of an origin.
pub async fn fetch_site_document(
    origin: &str,
    api_semaphore: &crate::util::fetch::FetchSemaphore,
    pool: &sqlx::SqlitePool,
) -> crate::Result<InteropSiteDocument> {
    fetch_catalog_url(&interop_url(origin), api_semaphore, pool).await
}

/// Fetches the unified exit document of an arbitrary origin (State-backed).
///
/// 先按协议读 `{origin}/.well-known/mc-interop-v1.json`；未网关转发的站点
/// （如开发环境前端代理不覆盖根路径）回退插件端点 `/api/plugins/mc-interop/interop/v1/site`。
pub async fn fetch_origin_catalog(
    origin: &str,
) -> crate::Result<InteropSiteDocument> {
    let state = State::get().await?;
    let base = origin.trim_end_matches('/').to_string();
    match fetch_site_document(&base, &state.api_semaphore, &state.pool).await {
        Ok(document) => Ok(document),
        Err(well_known_error) => {
            let fallback_url = format!(
                "{base}{FALLBACK_CATALOG_PATH}"
            );
            fetch_catalog_url(&fallback_url, &state.api_semaphore, &state.pool)
                .await
                .map_err(|fallback_error| {
                    crate::ErrorKind::OtherError(format!(
                        "well-known 与插件端点均不可用：{well_known_error} / {fallback_error}"
                    ))
                    .into()
                })
        }
    }
}

/// 按完整 URL 拉取并解析出口文档。
pub async fn fetch_catalog_url(
    url: &str,
    api_semaphore: &crate::util::fetch::FetchSemaphore,
    pool: &sqlx::SqlitePool,
) -> crate::Result<InteropSiteDocument> {
    fetch_json(
        reqwest::Method::GET,
        url,
        None,
        None,
        None,
        api_semaphore,
        pool,
    )
    .await
}

/// Probes a site for the union federated-auth plugin. `None` when the site
/// runs no union, serves an unsupported major version, or is unreachable —
/// in every case the launcher falls back to plain interop behavior.
pub async fn probe_union(
    origin: &str,
    api_semaphore: &crate::util::fetch::FetchSemaphore,
    pool: &sqlx::SqlitePool,
) -> Option<UnionMeta> {
    let meta: crate::Result<UnionMeta> = fetch_json(
        reqwest::Method::GET,
        &union_meta_url(origin),
        None,
        None,
        None,
        api_semaphore,
        pool,
    )
    .await;
    match meta {
        Ok(meta) if meta.is_supported() => Some(meta),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_protocol_urls() {
        assert_eq!(
            interop_url("https://sitmc.example.com/"),
            "https://sitmc.example.com/.well-known/mc-interop-v1.json"
        );
        assert_eq!(
            union_meta_url("https://sitmc.example.com"),
            "https://sitmc.example.com/api/plugins/yudream-union/peer/meta"
        );
    }

    #[test]
    fn parses_the_spec_example_document() {
        let document: InteropSiteDocument = serde_json::from_str(
            r#"{
                "site": {
                    "id": "sitmc",
                    "name": "SIT Minecraft",
                    "description": "SIT MC 公共服务器与整合包",
                    "domain": "skin.sitmc.club"
                },
                "packs": [
                    {
                        "id": "survival",
                        "name": "生存服整合包",
                        "revision": 12,
                        "gameVersion": "26.2",
                        "loader": "fabric",
                        "loaderVersion": "0.19.5",
                        "artifact": {
                            "format": "mrpack",
                            "url": "https://skin.sitmc.club/api/launcher/packs/survival/12",
                            "size": 511459395,
                            "sha1": "b00e0021aada1321b81cbd9693782df39b5f67a0",
                            "sha512": null
                        },
                        "server": { "address": "play.sitmc.club", "port": 25565 }
                    },
                    {
                        "id": "creative",
                        "name": "创造服",
                        "gameVersion": "26.2",
                        "loader": "vanilla",
                        "server": { "address": "build.sitmc.club" }
                    }
                ],
                "peers": [],
                "updatedAt": "2026-10-21T08:00:00Z"
            }"#,
        )
        .expect("spec example must deserialize");
        assert_eq!(document.packs.len(), 2);
        let survival = &document.packs[0];
        assert!(survival.has_artifact());
        assert_eq!(survival.revision, Some(12));
        assert_eq!(
            survival.artifact.as_ref().and_then(|a| a.sha512.as_deref()),
            None
        );
        let creative = &document.packs[1];
        assert!(!creative.has_artifact());
        assert_eq!(
            creative.server.as_ref().map(|s| s.address.as_str()),
            Some("build.sitmc.club")
        );
        assert_eq!(document.peers.len(), 0);
        assert_eq!(
            document.display_name("https://skin.sitmc.club"),
            "SIT Minecraft"
        );
    }

    #[test]
    fn falls_back_to_origin_when_site_is_unnamed() {
        let document: InteropSiteDocument =
            serde_json::from_str(r#"{"packs":[],"peers":[]}"#)
                .expect("empty document must deserialize");
        assert_eq!(
            document.display_name("https://site.example.com"),
            "https://site.example.com"
        );
    }

    #[test]
    fn accepts_only_supported_union_versions() {
        let meta: UnionMeta = serde_json::from_str(
            r#"{
                "unionVersion": "1",
                "site": { "code": "sitmc", "name": "SIT MC" },
                "authlib": { "apiRoot": "https://skin.sitmc.club/api/plugins/authlib-injector" },
                "features": ["join-assertions", "ban-ledger"]
            }"#,
        )
        .expect("union meta must deserialize");
        assert!(meta.is_supported());
        assert_eq!(
            meta.authlib.and_then(|a| a.api_root).as_deref(),
            Some("https://skin.sitmc.club/api/plugins/authlib-injector")
        );
        let future: UnionMeta =
            serde_json::from_str(r#"{"unionVersion":"2"}"#)
                .expect("version-only meta must deserialize");
        assert!(!future.is_supported());
    }
}

// ----- artifact acquisition (catalog entry → local mrpack for the importer) -----

use crate::event::LoadingBarId;
use crate::event::LoadingBarType;
use crate::event::emit::{emit_loading, init_loading};
use crate::State;

use sha2::Digest;

/// Outcome of [`download_entry_artifact`]: the local archive path for the
/// built-in importer (`{ type: 'fromFile' }`) plus display metadata.
#[derive(Serialize, Clone, Debug)]
pub struct InteropArtifactDownload {
    pub path: String,
    pub entry_id: String,
    pub name: Option<String>,
    pub game_version: Option<String>,
    pub loader: Option<String>,
}

/// Hard cap on a downloaded artifact regardless of declared size — a
/// runaway mirror must not fill the disk (spec §10 fail-closed spirit).
const MAX_ARTIFACT_BYTES: i64 = 4 * 1024 * 1024 * 1024;

/// Downloads a catalog entry's pack artifact into the launcher cache,
/// verifying the site-published hash (sha512 wins when both are present) and
/// returning the path the built-in mrpack importer accepts.
pub async fn download_entry_artifact(
    domain_id: &str,
    entry_id: &str,
) -> crate::Result<InteropArtifactDownload> {
    let site = super::registry::site_document(domain_id, false)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "Not an mc-interop domain".to_string(),
            )
        })?;
    let cache_scope = domain_id.to_string();
    download_for_site(&site, &cache_scope, entry_id).await
}

/// Same as [`download_entry_artifact`] but keyed by origin — works for YAP
/// domains too (their well-known carries the merged interop catalog).
pub async fn download_entry_artifact_by_origin(
    origin: &str,
    entry_id: &str,
) -> crate::Result<InteropArtifactDownload> {
    let site = fetch_origin_catalog(origin).await?;
    let cache_scope = crate::ymcl::registry::domain_id_for_origin(origin);
    download_for_site(&site, &cache_scope, entry_id).await
}

async fn download_for_site(
    site: &InteropSiteDocument,
    cache_scope: &str,
    entry_id: &str,
) -> crate::Result<InteropArtifactDownload> {
    let entry = site
        .packs
        .iter()
        .find(|pack| pack.id == entry_id)
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "Catalog entry {entry_id} not found"
            ))
        })?;
    let artifact = entry.artifact.clone().ok_or_else(|| {
        crate::ErrorKind::OtherError(format!(
            "Entry {entry_id} has no pack artifact"
        ))
    })?;
    // Spec §10: artifacts are https-only; never fetch a private http URL.
    if !artifact.url.starts_with("https://") {
        return Err(crate::ErrorKind::OtherError(
            "Artifact URL must be https".to_string(),
        )
        .into());
    }
    if artifact.size.unwrap_or(0) > MAX_ARTIFACT_BYTES {
        return Err(crate::ErrorKind::OtherError(
            "Artifact exceeds the size limit".to_string(),
        )
        .into());
    }

    let state = State::get().await?;
    let name = entry
        .name
        .clone()
        .unwrap_or_else(|| entry_id.to_string());
    let bar = init_loading(
        LoadingBarType::PackFileDownload {
            instance_id: String::new(),
            pack_name: name.clone(),
            icon: None,
            pack_version: entry.game_version.clone().unwrap_or_default(),
        },
        100.0,
        &format!("正在下载整合包 {name}"),
    )
    .await
    .ok();
    let streamed = stream_artifact(&state, &artifact, bar.as_ref()).await;
    drop(bar);
    let bytes = streamed?;

    verify_hash(&bytes, &artifact)?;

    // 临时文件名用条目名：安装器从文件 stem 取实例名，固定 pack.mrpack
    // 会让所有任务都显示「pack」。非法字符折叠为下划线，重名由
    // cache_scope/entry_id 目录隔离。
    let destination = state
        .directories
        .caches_dir()
        .join("modpacks")
        .join("mc-interop")
        .join(cache_scope)
        .join(entry_id)
        .join(format!("{}.mrpack", sanitize_file_stem(&name)));
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&destination, &bytes).await?;
    Ok(InteropArtifactDownload {
        path: destination.to_string_lossy().to_string(),
        entry_id: entry_id.to_string(),
        name: entry.name.clone(),
        game_version: entry.game_version.clone(),
        loader: entry.loader.clone(),
    })
}

/// 文件名安全的条目名（安装器实例名来源）；空白收敛，空/点回退 pack。
fn sanitize_file_stem(name: &str) -> String {
    let folded: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let collapsed = folded.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() || collapsed == "." {
        "pack".to_string()
    } else {
        collapsed
    }
}

async fn stream_artifact(
    state: &State,
    artifact: &InteropArtifact,
    bar: Option<&LoadingBarId>,
) -> crate::Result<Vec<u8>> {
    let mut response = state
        .configured_http_client()
        .get(&artifact.url)
        .send()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Artifact download failed: {e}"
            ))
        })?
        .error_for_status()
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Artifact download failed: {e}"
            ))
        })?;
    let total: Option<i64> =
        response.content_length().map(|value| value as i64).or(artifact.size);
    if let Some(total) = total {
        if total > MAX_ARTIFACT_BYTES {
            return Err(crate::ErrorKind::OtherError(
                "Artifact exceeds the size limit".to_string(),
            )
            .into());
        }
    }
    let mut bytes: Vec<u8> = Vec::new();
    let mut emitted = 0.0f64;
    loop {
        let chunk = response.chunk().await.map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Artifact download failed: {e}"
            ))
        })?;
        let Some(chunk) = chunk else { break };
        bytes.extend_from_slice(&chunk);
        if bytes.len() as i64 > MAX_ARTIFACT_BYTES {
            return Err(crate::ErrorKind::OtherError(
                "Artifact exceeds the size limit".to_string(),
            )
            .into());
        }
        if let (Some(bar), Some(total)) = (bar, total) {
            if total > 0 {
                let fraction =
                    bytes.len() as f64 / total as f64 * 100.0;
                let delta = fraction - emitted;
                if delta > 0.0 {
                    emit_loading(bar, delta, None)?;
                    emitted = fraction;
                }
            }
        }
    }
    Ok(bytes)
}

/// sha512 wins when the site published both hashes (spec §4).
fn verify_hash(bytes: &[u8], artifact: &InteropArtifact) -> crate::Result<()> {
    if let Some(expected) =
        artifact.sha512.as_deref().map(str::trim).filter(|s| !s.is_empty())
    {
        let digest = hex_lower(&sha2::Sha512::digest(bytes));
        if !digest.eq_ignore_ascii_case(expected) {
            return Err(crate::ErrorKind::OtherError(
                "Artifact sha512 mismatch; the download is corrupted or the site published a wrong hash".to_string(),
            )
            .into());
        }
        return Ok(());
    }
    if let Some(expected) =
        artifact.sha1.as_deref().map(str::trim).filter(|s| !s.is_empty())
    {
        let mut hasher = sha1_smol::Sha1::new();
        hasher.update(bytes);
        let digest = hasher.digest().to_string();
        if !digest.eq_ignore_ascii_case(expected) {
            return Err(crate::ErrorKind::OtherError(
                "Artifact sha1 mismatch; the download is corrupted or the site published a wrong hash".to_string(),
            )
            .into());
        }
    }
    Ok(())
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Live 集成测试：直连真实端点验证 mc-interop v1 协议契约（发现文档形状、
/// 身份、目录内容）。默认 `#[ignore]`（依赖本地宿主/外网），显式
/// `cargo test -p theseus ymcl::interop -- --ignored` 执行。
///
/// 注：registry 组合层（add_domain → 入库）依赖全局 State，无法在纯测试中
/// 运行；此处验证协议层数据契约，组合逻辑由 GUI 手工验收覆盖。
#[cfg(test)]
mod live_tests {
    use super::*;

    const SITMC: &str = "https://skin.sitmc.club";
    const LOCAL: &str = "http://localhost:9000";

    fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("client")
    }

    /// sitmc（第三方真实站）发现文档契约：身份、目录、artifact 完整性。
    #[tokio::test]
    #[ignore = "live：需要外网可达 skin.sitmc.club（--ignored 执行）"]
    async fn live_sitmc_discovery_contract() {
        let response = client()
            .get(format!("{SITMC}/.well-known/mc-interop-v1.json"))
            .send()
            .await
            .expect("sitmc 清单可拉取");
        assert!(response.status().is_success());
        let document: InteropSiteDocument = response.json().await.expect("sitmc 清单应为本插件模型可解析的合法 v1 文档");
        let site = document.site.as_ref().expect("site 块必填");
        assert_eq!(site.id, "sitmc");
        assert!(document.packs.len() >= 3, "sitmc 目录应含已知实例");
        let survival = document
            .packs
            .iter()
            .find(|pack| pack.id == "survival")
            .expect("survival 条目");
        let artifact = survival.artifact.as_ref().expect("survival 应带整合包");
        assert!(artifact.sha1.is_some() || artifact.sha512.is_some());
        let server = survival.server.as_ref().expect("server 必填");
        assert!(!server.address.is_empty());
    }

    /// 本地宿主：mc-interop 出口（union 身份 + 测试目录）与 union 发现文档。
    #[tokio::test]
    #[ignore = "live：需要本地宿主在 9000 端口运行（--ignored 执行）"]
    async fn live_local_exit_and_union() {
        let http = client();

        let response = http
            .get(format!("{LOCAL}/api/plugins/mc-interop/interop/v1/site"))
            .send()
            .await
            .expect("本地出口可拉取");
        assert!(response.status().is_success());
        let document: InteropSiteDocument = response.json().await.expect("本地出口应为合法 v1 文档");
        assert_eq!(document.site.as_ref().expect("site 块").id, "yda");
        assert!(document.packs.iter().any(|pack| pack.id == "survival"));
        // 未发布条目在任何出口形态（插件直连/宿主聚合）都不可见
        assert!(!document
            .packs
            .iter()
            .any(|pack| pack.id == "beta-test"));

        let meta: UnionMeta = http
            .get(format!("{LOCAL}/api/plugins/yudream-union/peer/meta"))
            .send()
            .await
            .expect("union meta 可拉取")
            .json()
            .await
            .expect("union meta 应可解析");
        assert!(meta.is_supported());
        assert_eq!(
            meta.site.as_ref().and_then(|site| site.code.clone()).as_deref(),
            Some("yda")
        );
    }
}

#[cfg(test)]
mod artifact_tests {
    use super::*;

    #[test]
    fn verifies_sha512_over_sha1() {
        // No hashes at all: pass-through (spec says at least one is present,
        // but the launcher must not fail-closed on a legacy site).
        let mut artifact = InteropArtifact {
            format: Some("mrpack".to_string()),
            url: "https://site.example.com/pack.mrpack".to_string(),
            size: None,
            sha1: None,
            sha512: None,
            distribution: None,
        };
        verify_hash(b"anything", &artifact).expect("no hash must pass");

        // A wrong sha512 must be rejected even when sha1 would match.
        let empty_sha512 = sha2::Sha512::digest(b"")
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        artifact.sha1 = Some(sha1_of(b""));
        artifact.sha512 = Some(empty_sha512);
        verify_hash(b"", &artifact).expect("matching sha512 must pass");
        artifact.sha512 = Some("f".repeat(128));
        assert!(verify_hash(b"", &artifact).is_err());

        // sha512 absent: sha1 is honored.
        artifact.sha512 = None;
        verify_hash(b"", &artifact).expect("matching sha1 must pass");
        artifact.sha1 = Some("0".repeat(40));
        assert!(verify_hash(b"", &artifact).is_err());
    }

    fn sha1_of(bytes: &[u8]) -> String {
        let mut hasher = sha1_smol::Sha1::new();
        hasher.update(bytes);
        hasher.digest().to_string()
    }
}

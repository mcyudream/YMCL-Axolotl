//! YAP 下载分发扩展（download face）：站点下发的按类别下载源链、内存快照
//! 与玩家下载令牌。
//!
//! 分发激活期间，站点配置取代启动器本地「下载源」设置：每个资源类别持有
//! 一条有序来源链（顺序即回退顺序），官方源按配置追加链尾兜底。快照随域
//! 激活/清单刷新更新，并按 TTL 后台自愈；拉不到配置或站点未宣告 download
//! face 时自动清空快照，回退本地设置。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use reqwest::Method;

use super::manifest::{
    YmclCapabilities, download_config_url, download_token_url,
};
use crate::util::fetch::{DownloadRoute, ResourceClass, fetch_json};

/// 配置快照 TTL：站点改配置最迟一个周期内生效。
#[cfg(not(test))]
pub const CONFIG_TTL_MILLIS: u64 = 5 * 60 * 1000;
#[cfg(test)]
pub const CONFIG_TTL_MILLIS: u64 = 60 * 1000;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct YmclGatewayConfig {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    /// `token`（玩家下载令牌）/ `static`（站点配置的固定 apikey）/ `none`。
    #[serde(default)]
    pub auth: Option<String>,
    /// `auth=static` 时的固定 apikey（网关只给 base+key 的场景）。
    #[serde(default)]
    pub static_key: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclDownloadSourceEntry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
}

/// `GET {config_url}` 的协议载荷（snake_case，与 ymcl-adapter
/// `YmclDownloadSourceService.protocolView` 逐字对应；容忍未知字段）。
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct YmclDownloadConfig {
    #[serde(default = "default_protocol_version")]
    pub protocol_version: u32,
    #[serde(default)]
    pub managed: bool,
    /// 宿主文档存储把 Long 字符串化（长 ID 防精度约定），实测回传
    /// `"1792780416351"` 而非数字；宽容解析两种形态。仅展示用途。
    #[serde(default, deserialize_with = "de_lenient_u64")]
    pub updated_at: Option<u64>,
    #[serde(default)]
    pub gateway: Option<YmclGatewayConfig>,
    #[serde(default = "default_true")]
    pub official_fallback: bool,
    #[serde(default)]
    pub sources: Vec<YmclDownloadSourceEntry>,
    #[serde(default)]
    pub categories: BTreeMap<String, Vec<String>>,
}

fn default_protocol_version() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

/// 数字或十进制字符串都接受；其余形态按缺失处理（前向兼容）。
fn de_lenient_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(number)) => Ok(number.as_u64()),
        Some(serde_json::Value::String(text)) => {
            Ok(text.trim().parse::<u64>().ok())
        }
        Some(_) => Ok(None),
    }
}

/// 当前激活域的分发状态快照。
#[derive(Clone, Debug)]
pub struct DownloadDistribution {
    pub domain_id: String,
    pub domain_name: String,
    pub config: YmclDownloadConfig,
    /// 网关下载令牌（玩家身份）；无会话时为 None，网关路由 401 后自动回退。
    pub gateway_token: Option<String>,
}

static DISTRIBUTION: OnceLock<RwLock<Option<Arc<DownloadDistribution>>>> =
    OnceLock::new();
static LAST_REFRESH_MILLIS: AtomicU64 = AtomicU64::new(0);
static REFRESH_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

fn distribution_slot() -> &'static RwLock<Option<Arc<DownloadDistribution>>> {
    DISTRIBUTION.get_or_init(|| RwLock::new(None))
}

/// 当前分发的快照（sync，热路径可调）；过期时安排后台刷新，不阻塞调用方。
pub fn active_distribution() -> Option<Arc<DownloadDistribution>> {
    let snapshot = distribution_slot().read().ok()?.clone();
    maybe_schedule_refresh();
    snapshot
}

/// 分发是否托管了给定资源类别的路由。
pub fn category_for_resource(resource: ResourceClass) -> Option<&'static str> {
    match resource {
        ResourceClass::Metadata => Some("minecraft_metadata"),
        ResourceClass::MinecraftAsset | ResourceClass::MinecraftLibrary => {
            Some("minecraft_files")
        }
        ResourceClass::Loader => Some("loaders"),
        ResourceClass::Modrinth | ResourceClass::Modpack => Some("modrinth"),
        ResourceClass::CurseForge => Some("curseforge"),
        // Java 运行时与其余未知主机 v1 不分发，保持本地设置。
        ResourceClass::Java | ResourceClass::Other => None,
    }
}

/// 分发序路由：按站点下发的来源链构造候选列表。返回 `None` 表示该类别
/// 未被分发（调用方回退本地设置）。
pub fn managed_routes(
    url: &str,
    resource: ResourceClass,
) -> Option<Vec<DownloadRoute>> {
    let distribution = active_distribution()?;
    let category = category_for_resource(resource)?;
    let chain = distribution.config.categories.get(category)?;
    if chain.is_empty() {
        return None;
    }

    let mut routes: Vec<DownloadRoute> = Vec::with_capacity(chain.len() + 1);
    let push_unique = |route: DownloadRoute,
                       routes: &mut Vec<DownloadRoute>| {
        if !routes.iter().any(|existing| existing.url == route.url) {
            routes.push(route);
        }
    };

    for source_id in chain {
        let Some(source) = distribution
            .config
            .sources
            .iter()
            .find(|source| &source.id == source_id)
        else {
            // 未知来源 id：容忍并跳过（YAP §6.1 前向兼容）。
            continue;
        };
        match source.provider.as_str() {
            "official" => {
                push_unique(
                    crate::util::fetch::official_route(url, resource),
                    &mut routes,
                );
            }
            "bmclapi" => append_family_routes(
                url,
                resource,
                crate::util::fetch::MirrorKind::BmclapiCompat,
                crate::util::fetch::DownloadRouteSource::Bmclapi,
                &mut routes,
            ),
            "mcim" => append_family_routes(
                url,
                resource,
                crate::util::fetch::MirrorKind::Mcim,
                crate::util::fetch::DownloadRouteSource::Mcim,
                &mut routes,
            ),
            "tianpao" => append_family_routes(
                url,
                resource,
                crate::util::fetch::MirrorKind::Tianpao,
                crate::util::fetch::DownloadRouteSource::Tianpao,
                &mut routes,
            ),
            "aliyun" => append_family_routes(
                url,
                resource,
                crate::util::fetch::MirrorKind::Aliyun,
                crate::util::fetch::DownloadRouteSource::Aliyun,
                &mut routes,
            ),
            // 网关与自定义都按 BMCLAPI 路由兼容基址构造（契约见 YDA 侧
            // docs/ymcl-download-gateway-v1.md）；缺基址的条目跳过。
            "gateway" | "custom" => {
                let Some(base) =
                    source.base_url.as_deref().filter(|base| !base.is_empty())
                else {
                    continue;
                };
                append_gateway_routes(
                    url,
                    resource,
                    base,
                    &distribution,
                    &mut routes,
                );
            }
            _ => continue,
        }
    }

    if routes.is_empty() {
        return None;
    }
    // 官方兜底：配置未显式包含官方源且开关开启时追加链尾。
    if distribution.config.official_fallback
        && !routes.iter().any(|route| !route.is_mirror)
    {
        routes.push(crate::util::fetch::official_route(url, resource));
    }
    Some(routes)
}

fn append_family_routes(
    url: &str,
    resource: ResourceClass,
    kind: crate::util::fetch::MirrorKind,
    source: crate::util::fetch::DownloadRouteSource,
    routes: &mut Vec<DownloadRoute>,
) {
    let base = crate::util::fetch::builtin_mirror_base(kind);
    for route in crate::util::fetch::mirror_routes_for_family(
        url, resource, kind, base, source,
    ) {
        if !routes.iter().any(|existing| existing.url == route.url) {
            routes.push(route);
        }
    }
}

/// 网关/自定义源：BMCLAPI 兼容路径 + `?key=<下载令牌>`（auth=token 时）。
/// 令牌随快照下发；缺失时路由照常生成，网关 401 由回退链吸收。
fn append_gateway_routes(
    url: &str,
    resource: ResourceClass,
    base: &str,
    distribution: &DownloadDistribution,
    routes: &mut Vec<DownloadRoute>,
) {
    for route in crate::util::fetch::mirror_routes_for_family(
        url,
        resource,
        crate::util::fetch::MirrorKind::BmclapiCompat,
        base,
        crate::util::fetch::DownloadRouteSource::Gateway,
    ) {
        let mut route = route;
        let auth = distribution
            .config
            .gateway
            .as_ref()
            .map(|gateway| gateway.auth.as_deref())
            .flatten()
            .unwrap_or("token");
        let key = match auth {
            "static" => distribution
                .config
                .gateway
                .as_ref()
                .and_then(|gateway| gateway.static_key.as_deref()),
            "token" => distribution.gateway_token.as_deref(),
            _ => None,
        };
        if let Some(key) = key {
            let separator = if route.url.contains('?') { '&' } else { '?' };
            route.url = format!("{}{}key={key}", route.url, separator);
        }
        if !routes.iter().any(|existing| existing.url == route.url) {
            routes.push(route);
        }
    }
}

// ── 快照维护 ─────────────────────────────────────────────────────────────

/// OptiFine 镜像端点：OptiFine 的元数据与安装器不走统一路由表（独立硬编码
/// BMCLAPI 端点），分发激活时改指 loaders 链中第一个 BMCLAPI 兼容源，
/// 网关/自定义按 auth 附 `?key=<下载令牌>`；未分发或链内无兼容源时返回
/// None（调用方回退内置 BMCLAPI）。
pub fn optifine_url(path: &str) -> Option<String> {
    let distribution = active_distribution()?;
    let chain = distribution.config.categories.get("loaders")?;
    for id in chain {
        let Some(source) = distribution
            .config
            .sources
            .iter()
            .find(|source| &source.id == id)
        else {
            continue;
        };
        let host = match source.provider.as_str() {
            "bmclapi" => "https://bmclapi2.bangbang93.com".to_string(),
            "gateway" | "custom" => {
                let Some(base) =
                    source.base_url.as_deref().filter(|base| !base.is_empty())
                else {
                    continue;
                };
                base.trim_end_matches('/').to_string()
            }
            _ => continue,
        };
        let mut url = format!("{host}/optifine{path}");
        let auth = distribution
            .config
            .gateway
            .as_ref()
            .and_then(|gateway| gateway.auth.as_deref())
            .unwrap_or("token");
        let key = match (source.provider.as_str(), auth) {
            ("gateway", "static") => distribution
                .config
                .gateway
                .as_ref()
                .and_then(|gateway| gateway.static_key.as_deref()),
            (_, "token") => distribution.gateway_token.as_deref(),
            _ => None,
        };
        if source.provider != "bmclapi"
            && auth != "none"
            && let Some(key) = key
        {
            url = format!("{url}?key={key}");
        }
        return Some(url);
    }
    None
}

fn store(distribution: Option<Arc<DownloadDistribution>>) {
    if let Ok(mut slot) = distribution_slot().write() {
        *slot = distribution;
    }
    LAST_REFRESH_MILLIS.store(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0),
        Ordering::Release,
    );
}

fn maybe_schedule_refresh() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    if now.saturating_sub(LAST_REFRESH_MILLIS.load(Ordering::Acquire))
        < CONFIG_TTL_MILLIS
    {
        return;
    }
    if REFRESH_IN_FLIGHT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            handle.spawn(async move {
                if let Err(error) = refresh_active().await {
                    tracing::debug!(
                        "Download distribution refresh failed: {error}"
                    );
                }
                REFRESH_IN_FLIGHT.store(false, Ordering::Release);
            });
        }
        Err(_) => {
            REFRESH_IN_FLIGHT.store(false, Ordering::Release);
        }
    }
}

/// 重新拉取激活域的分发配置（域激活/域移除/清单刷新/后台 TTL 时调用）。
/// 个人域或站点未宣告 download face 时清空快照。
pub async fn refresh_active() -> crate::Result<()> {
    let Some((domain_id, domain_name)) =
        super::registry::active_domain_brief().await?
    else {
        store(None);
        return Ok(());
    };
    let capabilities: YmclCapabilities =
        super::registry::domain_capabilities(&domain_id).await?;
    let Some(face) = capabilities.download else {
        store(None);
        return Ok(());
    };
    let state = crate::State::get().await?;
    let origin = super::registry::domain_origin(&domain_id).await?;
    let config_url = download_config_url(&origin, face.config_url.as_deref());
    let config: YmclDownloadConfig = fetch_json(
        Method::GET,
        &config_url,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    if !config.managed {
        store(None);
        return Ok(());
    }
    let gateway_token = fetch_gateway_token(&state, &domain_id, &origin, face.token_url.as_deref())
        .await
        .inspect_err(|error| {
            tracing::debug!(
                "Download token unavailable for {domain_name}: {error}; gateway routes degrade to fallback"
            )
        })
        .ok()
        .map(|issued| issued.token);
    store(Some(Arc::new(DownloadDistribution {
        domain_id,
        domain_name,
        config,
        gateway_token,
    })));
    Ok(())
}

#[derive(Deserialize)]
struct IssuedDownloadToken {
    #[serde(default)]
    token: String,
}

/// 为当前域会话玩家取下载令牌（网关 ?key= 鉴权用）；无会话/失败时 Err。
async fn fetch_gateway_token(
    state: &crate::State,
    domain_id: &str,
    origin: &str,
    configured: Option<&str>,
) -> crate::Result<IssuedDownloadToken> {
    let token_url = download_token_url(origin, configured);
    let bytes = super::auth::domain_request_opt(
        state,
        domain_id,
        Method::POST,
        &token_url,
        None,
    )
    .await?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod config_tests {
    use super::{
        DownloadDistribution, YmclDownloadConfig, YmclDownloadSourceEntry,
        YmclGatewayConfig, store,
    };
    use crate::util::fetch::ResourceClass;
    use std::sync::Arc;

    /// 运行时行为锁：auth=static 的网关源必须以 `?key=<static_key>` 出现在
    /// 链首（dev 实测曾因二进制/快照旧版而裸 URL 403）。
    #[test]
    fn managed_routes_append_static_key_for_gateway_source() {
        let config: YmclDownloadConfig = serde_json::from_str(
            r#"{
                "protocol_version": 1, "managed": true,
                "gateway": { "name": "生产网关",
                    "base_url": "http://gw.test:8322",
                    "auth": "static", "static_key": "mk_unit_test" },
                "official_fallback": true,
                "sources": [
                    { "id": "gateway", "provider": "gateway",
                      "base_url": "http://gw.test:8322" },
                    { "id": "official", "provider": "official" }
                ],
                "categories": { "minecraft_files": ["gateway", "official"] }
            }"#,
        )
        .expect("static config must deserialize");
        store(Some(Arc::new(DownloadDistribution {
            domain_id: "test".into(),
            domain_name: "测试域".into(),
            config,
            gateway_token: None,
        })));

        let routes = super::managed_routes(
            "https://libraries.minecraft.net/org/jna/jna/5.17.0/jna-5.17.0.jar",
            ResourceClass::MinecraftLibrary,
        )
        .expect("managed chain must produce routes");
        assert!(
            routes[0].url.starts_with("http://gw.test:8322/maven/"),
            "网关源应排首位: {}",
            routes[0].url
        );
        assert!(
            routes[0].url.contains("key=mk_unit_test"),
            "static key 必须拼进网关路由: {}",
            routes[0].url
        );
        // 清理全局快照，避免污染其它测试。
        store(None);
    }

    use super::YmclDownloadConfig as _ConfigAlias;
    #[allow(unused_imports)]
    use super::YmclDownloadSourceEntry as _EntryAlias;
    #[allow(unused_imports)]
    use YmclGatewayConfig as _GwAlias;

    /// Contract lock against `YmclDownloadSourceService.protocolView`
    /// (yudream-admin-plugins ymcl-adapter 0.12.0, snake_case).
    #[test]
    fn parses_download_config_payload() {
        let payload = r#"{
            "protocol_version": 1,
            "managed": true,
            "updated_at": 1761180000000,
            "gateway": {
                "name": "YDA 镜像",
                "base_url": "https://mirror.example.com",
                "auth": "token"
            },
            "official_fallback": true,
            "sources": [
                { "id": "gateway", "provider": "gateway", "name": "域网关",
                  "base_url": "https://mirror.example.com" },
                { "id": "bmclapi", "provider": "bmclapi", "name": null, "base_url": null }
            ],
            "categories": {
                "minecraft_metadata": ["official", "bmclapi", "gateway"],
                "minecraft_files": ["bmclapi", "gateway"],
                "loaders": ["bmclapi", "gateway", "official"],
                "modrinth": ["mcim", "official"],
                "curseforge": ["mcim"]
            }
        }"#;
        let config: YmclDownloadConfig = serde_json::from_str(payload)
            .expect("config payload must deserialize");
        assert!(config.managed);
        assert!(config.official_fallback);
        let gateway = config.gateway.as_ref().expect("gateway block");
        assert_eq!(
            gateway.base_url.as_deref(),
            Some("https://mirror.example.com")
        );
        assert_eq!(config.sources.len(), 2);
        assert_eq!(
            config.categories.get("loaders").map(Vec::as_slice),
            Some(
                &[
                    "bmclapi".to_string(),
                    "gateway".to_string(),
                    "official".to_string()
                ][..]
            ),
        );
    }

    /// Unmanaged / future-versioned payloads degrade to the defaults: the
    /// launcher then falls back to its local download source settings.
    #[test]
    fn tolerates_unmanaged_and_unknown_fields() {
        let config: YmclDownloadConfig = serde_json::from_str(
            r#"{ "protocol_version": 2, "managed": false, "future_field": { "x": 1 } }"#,
        )
        .expect("unmanaged payload must deserialize");
        assert!(!config.managed);
        assert!(config.official_fallback, "缺省兜底开关为开");
        assert!(config.sources.is_empty());
        assert!(config.categories.is_empty());
    }

    /// The host document store stringifies Longs (yudreamadmin 全局长 ID 防
    /// 精度约定)，实测 dev 环境 `updated_at` 回传字符串数字；解析必须容忍。
    #[test]
    fn parses_stringified_updated_at_from_live_host() {
        let config: YmclDownloadConfig = serde_json::from_str(
            r#"{ "protocol_version": 1, "managed": true,
                 "updated_at": "1792780416351", "official_fallback": true }"#,
        )
        .expect("stringified timestamp must deserialize");
        assert_eq!(config.updated_at, Some(1_792_780_416_351));
        // 数字形态同样接受；垃圾值按缺失处理，不让整个配置解析失败。
        let numeric: YmclDownloadConfig = serde_json::from_str(
            r#"{ "managed": true, "updated_at": 1792780416351 }"#,
        )
        .expect("numeric timestamp must deserialize");
        assert_eq!(numeric.updated_at, Some(1_792_780_416_351));
        let garbage: YmclDownloadConfig = serde_json::from_str(
            r#"{ "managed": true, "updated_at": "not-a-number" }"#,
        )
        .expect("garbage timestamp must not fail the whole config");
        assert_eq!(garbage.updated_at, None);
    }
}

//! MIP first install over mrpack (YAP §7 / WF-4): the player's first fetch
//! of a domain pack downloads the pack's mrpack archive — the built-in
//! importer materializes game version, loader and files from it — and then
//! adopts the MIP state so later updates stay incremental.
//!
//! Delta-only versions have no mrpack on the adapter (only the root and
//! other full releases carry one), so the launcher merges: the target
//! version's manifest `parent` chain is walked to the nearest ancestor that
//! still serves a full mrpack (the *baseline*), that archive is installed,
//! and the ordinary WF-5 incremental update then catches the instance up to
//! the target — objects still coming from CAS/sources.

use std::collections::HashSet;

use serde::Serialize;

use super::manifest::MipManifest;
use super::state::{self, MipInstanceBinding, MipPackState, StateFile};
use super::update::{
    MipServerBinding, MipServerEndpoint, MipVersionEntry, fetch_json,
    manifest_url, resolve_mip_base, resolve_target_version, servers_url,
    validate_feature_selection, versions_url,
};
use crate::State;
use crate::api::ymcl::{auth, registry};
use crate::event::LoadingBarId;
use crate::event::LoadingBarType;
use crate::event::emit::{emit_loading, init_loading};

/// Outcome of [`download_pack_mrpack`]: the downloaded archive path plus the
/// resolved pack identity the adopt step needs.
#[derive(Serialize, Clone, Debug)]
pub struct YmclMrpackDownload {
    pub path: String,
    pub pack_id: String,
    /// Version the archive materializes: the target version itself, or the
    /// baseline found along the parent chain when the target has no mrpack.
    pub version: String,
    pub channel: Option<String>,
    /// Set when the archive is a baseline that still needs the incremental
    /// update to this version after the import (baseline mrpack + WF-5
    /// catch-up). `None` when the archive already is the target version.
    pub target_version: Option<String>,
}

/// A resolved download target: which pack at which version from which MIP
/// base.
struct MrpackTarget {
    pack_id: String,
    version: String,
    channel: Option<String>,
    mip_base: String,
}

/// How many parent hops the baseline search follows before giving up.
const MAX_PARENT_HOPS: usize = 64;

/// True when the domain answered "this artifact does not exist" — how "this
/// version has no full mrpack" (delta-only releases) and "the parent link
/// points at a withdrawn version" both surface. Other errors are
/// transport/permission problems and must fail loudly instead of being read
/// as "keep walking the chain".
///
/// YDA reports missing artifacts as HTTP 400 + domain envelope
/// `{code, message: "文件不存在"}` rather than a bare 404, so the message is
/// part of the signal.
fn is_not_found(error: &crate::Error) -> bool {
    match error.raw.as_ref() {
        crate::ErrorKind::HttpError { status: 404, .. } => true,
        crate::ErrorKind::LabrinthError(error) => {
            if error.status == Some(404) {
                return true;
            }
            let message = error.description.as_str();
            message.contains("文件不存在")
                || message.contains("资源不存在")
                || message.contains("file not found")
                || message.eq_ignore_ascii_case("not found")
        }
        _ => false,
    }
}

/// Manifests of one pack by version. Production reads them from the domain
/// (authenticated MIP face); tests use an in-memory map.
trait ManifestSource {
    fn fetch(&self, version: &str) -> impl std::future::Future<Output = crate::Result<MipManifest>> + Send;
}

/// mrpack archives of one pack by version. A 404 surfaces as `Err` and
/// means "no full archive for this version".
trait MrpackSource {
    fn fetch(
        &self,
        version: &str,
        features: &[String],
    ) -> impl std::future::Future<Output = crate::Result<Vec<u8>>> + Send;
}

struct DomainManifestSource<'a> {
    state: &'a State,
    mip_base: &'a str,
    pack_id: &'a str,
}

impl ManifestSource for DomainManifestSource<'_> {
    async fn fetch(&self, version: &str) -> crate::Result<MipManifest> {
        let value =
            fetch_json(self.state, &manifest_url(self.mip_base, self.pack_id, version))
                .await?;
        let manifest: MipManifest = serde_json::from_value(value)?;
        manifest.validate()?;
        Ok(manifest)
    }
}

struct DomainMrpackSource<'a> {
    state: &'a State,
    mip_base: &'a str,
    pack_id: &'a str,
    active_domain: &'a str,
    /// Progress bar shared by every probe/download of one install: 404
    /// probes transfer no bytes, the successful archive streams to 100%.
    bar: Option<&'a LoadingBarId>,
}

fn mrpack_url(
    mip_base: &str,
    pack_id: &str,
    version: &str,
    features: &[String],
) -> String {
    let mut url = format!("{mip_base}/api/packs/{pack_id}/versions/{version}/mrpack");
    if !features.is_empty() {
        let features = features
            .iter()
            .map(|id| urlencoding::encode(id).into_owned())
            .collect::<Vec<_>>()
            .join(",");
        url.push_str(&format!("?features={features}"));
    }
    url
}

impl MrpackSource for DomainMrpackSource<'_> {
    async fn fetch(
        &self,
        version: &str,
        features: &[String],
    ) -> crate::Result<Vec<u8>> {
        let url = mrpack_url(self.mip_base, self.pack_id, version, features);
        if let Some(bar) = self.bar {
            let _ = emit_loading(
                bar,
                0.0,
                Some(&format!("正在下载整合包 {} {version}", self.pack_id)),
            );
        }
        // The adapter's MIP face sits behind its plugin permission wall and
        // is plain http (localhost dev / self-hosted domains), so this must
        // go through the domain request path — the mod-CDN download engine
        // is https-only and rejects the URL at request-build time, and the
        // domain path also renews expired sessions and surfaces the yda
        // error envelope. Whole archive in memory (same as CAS object
        // fetches), streamed through the progress bar.
        let bytes = auth::domain_request_opt_with_bar(
            self.state,
            self.active_domain,
            reqwest::Method::GET,
            &url,
            None,
            self.bar.map(|bar| (bar, 100.0)),
        )
        .await?;
        Ok(bytes.to_vec())
    }
}

/// Walks `manifest.parent` from the target version towards the root,
/// returning the ancestor manifests nearest-first. Broken links (withdrawn
/// manifests) end the walk — everything beyond is unreachable — and cycles
/// or runaway chains are cut off at [`MAX_PARENT_HOPS`].
async fn parent_chain<S: ManifestSource>(
    target: &MipManifest,
    source: &S,
) -> crate::Result<Vec<MipManifest>> {
    let mut chain = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(target.version.clone());
    let mut parent = target.parent.clone();
    while let Some(version) = parent {
        if chain.len() >= MAX_PARENT_HOPS || !visited.insert(version.clone()) {
            break;
        }
        match source.fetch(&version).await {
            Ok(manifest) => {
                parent = manifest.parent.clone();
                chain.push(manifest);
            }
            Err(error) if is_not_found(&error) => break,
            Err(error) => return Err(error),
        }
    }
    Ok(chain)
}

/// Finds the nearest ancestor in `chain` (nearest-first) that still serves a
/// full mrpack, returning its version and bytes. The feature query is
/// filtered to what the baseline declares: the checkbox list is compiled
/// against the target's feature catalog, and the baseline's server-side
/// synthesis only knows its own.
async fn baseline_mrpack<S: MrpackSource>(
    chain: &[MipManifest],
    source: &S,
    selected: &[String],
) -> crate::Result<Option<(String, Vec<u8>)>> {
    for manifest in chain {
        let declared: HashSet<&str> =
            manifest.features.iter().map(|feature| feature.id.as_str()).collect();
        let features: Vec<String> = selected
            .iter()
            .filter(|id| declared.contains(id.as_str()))
            .cloned()
            .collect();
        match source.fetch(&manifest.version, &features).await {
            Ok(bytes) => return Ok(Some((manifest.version.clone(), bytes))),
            Err(error) if is_not_found(&error) => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}

/// Resolves what to download for `server_id`: `pack_id` absent means the
/// server's required pack; an explicit id matches either the required or the
/// optional (vanilla-enhanced) binding and takes that side's channel/pin.
async fn resolve_mrpack_target(
    state: &State,
    server_id: &str,
    pack_id: Option<&str>,
) -> crate::Result<MrpackTarget> {
    let Some(mip_base) = resolve_mip_base(state).await? else {
        return Err(
            crate::ErrorKind::OtherError("请先在顶栏选择一个域".to_string())
                .into(),
        );
    };
    let servers_value = fetch_json(state, &servers_url(&mip_base)).await?;
    let servers: Vec<MipServerBinding> = serde_json::from_value(
        servers_value
            .get("servers")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )?;
    let Some(server) = servers
        .iter()
        .find(|server| server.server_id == server_id)
    else {
        return Err(
            crate::ErrorKind::OtherError("未找到该服务器".to_string()).into()
        );
    };
    let binding = server.binding.clone().unwrap_or_default();
    let required_pack =
        (!binding.pack_id.is_empty()).then_some(binding.pack_id.as_str());
    let optional_pack = binding
        .optional_pack_id
        .as_deref()
        .filter(|id| !id.is_empty());
    let (pack_id, channel, pinned) = match pack_id {
        None => match required_pack {
            Some(id) => (
                id,
                binding.channel.as_deref(),
                binding.pinned_version.as_deref(),
            ),
            None => {
                return Err(crate::ErrorKind::OtherError(
                    "该服务器没有绑定必装整合包".to_string(),
                )
                .into());
            }
        },
        Some(id) if Some(id) == required_pack => (
            id,
            binding.channel.as_deref(),
            binding.pinned_version.as_deref(),
        ),
        Some(id) if Some(id) == optional_pack => (
            id,
            binding.optional_channel.as_deref(),
            binding.optional_pinned_version.as_deref(),
        ),
        Some(id) => {
            return Err(crate::ErrorKind::OtherError(format!(
                "服务器未绑定整合包 {id}"
            ))
            .into());
        }
    };
    let versions_value =
        fetch_json(state, &versions_url(&mip_base, pack_id)).await?;
    let versions: Vec<MipVersionEntry> = serde_json::from_value(
        versions_value
            .get("versions")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )?;
    let Some(version) = resolve_target_version(&versions, channel, pinned)
    else {
        return Err(crate::ErrorKind::OtherError(format!(
            "整合包 {pack_id} 没有已发布的版本"
        ))
        .into());
    };
    Ok(MrpackTarget {
        pack_id: pack_id.to_string(),
        version: version.to_string(),
        channel: channel.map(|value| value.to_string()),
        mip_base,
    })
}

/// WF-4 first-install step 1: downloads the mrpack archive for the target
/// version — or, when the target carries no full mrpack (delta-only
/// release), for the nearest ancestor on its `parent` chain that does —
/// into the cache directory. The frontend hands the archive to the built-in
/// mrpack importer, calls [`adopt_pack_state`] once the install job settles,
/// and runs the incremental update when [`YmclMrpackDownload::target_version`]
/// is set. Download progress streams through a standard pack-download
/// loading bar (download toast + Downloads page).
pub async fn download_pack_mrpack(
    server_id: &str,
    pack_id: Option<String>,
    selected: Vec<String>,
) -> crate::Result<YmclMrpackDownload> {
    let state = State::get().await?;
    let target =
        resolve_mrpack_target(&state, server_id, pack_id.as_deref()).await?;

    // One bar for the whole acquisition: probing delta-only versions costs
    // no bytes (404s), the successful archive streams the bar to full.
    let bar = init_loading(
        LoadingBarType::PackFileDownload {
            instance_id: String::new(),
            pack_name: target.pack_id.clone(),
            icon: None,
            pack_version: target.version.clone(),
        },
        100.0,
        &format!("正在下载整合包 {} {}", target.pack_id, target.version),
    )
    .await
    .ok();
    let active = registry::active_domain_id(&state.pool).await?;
    let mrpack = DomainMrpackSource {
        state: &state,
        mip_base: &target.mip_base,
        pack_id: &target.pack_id,
        active_domain: &active,
        bar: bar.as_ref(),
    };

    let (installed_version, bytes) = match mrpack.fetch(&target.version, &selected).await {
        Ok(bytes) => (target.version.clone(), bytes),
        Err(error) if is_not_found(&error) => {
            // Delta-only target: walk the parent chain to the nearest full
            // release. The catch-up to the target happens after the import.
            let manifests = DomainManifestSource {
                state: &state,
                mip_base: &target.mip_base,
                pack_id: &target.pack_id,
            };
            let target_manifest = manifests.fetch(&target.version).await?;
            let chain = parent_chain(&target_manifest, &manifests).await?;
            let Some((baseline, bytes)) =
                baseline_mrpack(&chain, &mrpack, &selected).await?
            else {
                return Err(crate::ErrorKind::OtherError(format!(
                    "整合包 {} {} 没有完整包（mrpack），parent 链上也找不到可用的基线版本",
                    target.pack_id, target.version
                ))
                .into());
            };
            if let Some(bar) = &bar {
                let _ = emit_loading(
                    bar,
                    0.0,
                    Some(&format!(
                        "基线版本 {baseline} 下载完成，安装后将增量追平到 {}",
                        target.version
                    )),
                );
            }
            (baseline, bytes)
        }
        Err(error) => return Err(error),
    };
    drop(mrpack);
    drop(bar);

    let destination = state
        .directories
        .caches_dir()
        .join("modpacks")
        .join("ymcl")
        .join(&target.pack_id)
        .join(&installed_version)
        .join("pack.mrpack");
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&destination, &bytes).await?;
    Ok(YmclMrpackDownload {
        path: destination.to_string_lossy().to_string(),
        pack_id: target.pack_id,
        version: installed_version.clone(),
        channel: target.channel,
        target_version: (installed_version != target.version)
            .then_some(target.version),
    })
}

/// WF-4 first-install step 2: records the instance as MIP-managed after the
/// built-in mrpack importer has materialized the archive. State files come
/// from the domain manifest (the source of truth, covering CDN-referenced
/// files) filtered to the install set (selected ∪ default-on features) that
/// the downloaded mrpack carried. Pattern follows publish.rs
/// `adopt_state_from_report`.
pub async fn adopt_pack_state(
    instance_id: &str,
    server_id: &str,
    pack_id: &str,
    version: &str,
    channel: Option<String>,
    selected: Vec<String>,
    force: bool,
) -> crate::Result<()> {
    let state = State::get().await?;
    let instance_dir = crate::api::instance::get_full_path(instance_id).await?;
    // force 允许接管另一个包的实例（服务器删包重传换了 packId 后的"更新"
    // 就是重装当前绑定包）；不带 force 的异包绑定照旧拒绝，防误触。
    if let Some(existing) = state::load(&instance_dir).await?
        && existing.pack_id != pack_id
        && !force
    {
        return Err(crate::ErrorKind::OtherError(format!(
            "该实例已由整合包 {} 管理",
            existing.pack_id
        ))
        .into());
    }
    let Some(mip_base) = resolve_mip_base(&state).await? else {
        return Err(
            crate::ErrorKind::OtherError("请先在顶栏选择一个域".to_string())
                .into(),
        );
    };
    let manifest_value =
        fetch_json(&state, &manifest_url(&mip_base, pack_id, version)).await?;
    let manifest: MipManifest = serde_json::from_value(manifest_value)?;
    manifest.validate()?;
    // 勾选面向的是目标版本的特性目录；装基线 mrpack 时里面可能有本版本
    // 尚未声明的特性。过滤到本版本已声明的集合——多出来的那份等增量追平
    // 时由 default 合并补齐。
    let declared: HashSet<&str> =
        manifest.features.iter().map(|feature| feature.id.as_str()).collect();
    let selected: Vec<String> = selected
        .into_iter()
        .filter(|id| declared.contains(id.as_str()))
        .collect();
    validate_feature_selection(&manifest, &selected)?;

    // 勾选 = 显式选择 ∪ default（WF-4 默认值），与服务端合成 mrpack 的
    // 安装集口径一致。
    let mut final_selection = selected;
    for feature in &manifest.features {
        if feature.default && !final_selection.contains(&feature.id) {
            final_selection.push(feature.id.clone());
        }
    }
    let selected_set: HashSet<&str> =
        final_selection.iter().map(|id| id.as_str()).collect();
    let mut files = std::collections::BTreeMap::new();
    for file in &manifest.files {
        let in_install_set = file
            .feature
            .as_deref()
            .map(|feature| selected_set.contains(feature))
            .unwrap_or(true);
        if !in_install_set {
            continue;
        }
        files.insert(
            file.path.clone(),
            StateFile {
                sha512: file.sha512.clone(),
                policy: file.policy.clone(),
                feature: file.feature.clone(),
            },
        );
    }
    drop(selected_set);
    let mut pack_state = MipPackState {
        pack_id: pack_id.to_string(),
        version: version.to_string(),
        channel,
        selected_features: final_selection,
        declared_features: Some(
            manifest
                .features
                .iter()
                .map(|feature| feature.id.clone())
                .collect(),
        ),
        files,
        ..MipPackState::default()
    };
    // 记录来源服务器/赛季（YAP §7 绑定语义）。服务器查询失败不阻断安装收尾：
    // 更新检查对 season 缺失本就是容忍的。
    let bound_server: Option<MipServerBinding> = async {
        let servers_value = fetch_json(&state, &servers_url(&mip_base)).await?;
        let servers: Vec<MipServerBinding> = serde_json::from_value(
            servers_value
                .get("servers")
                .cloned()
                .unwrap_or(serde_json::Value::Array(vec![])),
        )?;
        crate::Result::Ok(
            servers
                .into_iter()
                .find(|server| server.server_id == server_id),
        )
    }
    .await
    .unwrap_or_else(|error| {
        tracing::warn!("MIP adopt: server lookup failed for {server_id}: {error}");
        None
    });
    pack_state.binding = Some(MipInstanceBinding {
        server_id: server_id.to_string(),
        season_id: bound_server
            .as_ref()
            .and_then(|server| server.current_season.as_ref())
            .and_then(|season| season.season_id.clone()),
    });
    state::save(&instance_dir, &pack_state).await?;

    // 注入连接地址到实例的服务器列表（主线+备用线路，有就不管，无就加），
    // 玩家在 MC 多人游戏列表里直接就有这台服。注入失败不回滚安装。
    if let Some(server) = &bound_server {
        let server_name = server.name.clone().filter(|name| !name.is_empty());
        for address in server_join_addresses(server) {
            let entry_name = server_name.clone().unwrap_or_else(|| address.clone());
            match crate::api::worlds::ensure_server_in_instance(
                instance_id,
                entry_name,
                address.clone(),
            )
            .await
            {
                Ok(true) => tracing::info!(
                    "MIP adopt: injected server address {address} into instance {instance_id}"
                ),
                Ok(false) => {}
                Err(error) => tracing::warn!(
                    "MIP adopt: server entry injection failed for {address}: {error}"
                ),
            }
        }
    }
    Ok(())
}

/// The server-list injection set for a domain server: every joinable line,
/// primary first then backups in adapter order, falling back to `mcAddress`
/// when the adapter declares no usable endpoints (MIP appendix B.2).
/// Deduplicated: repeated lines and a backup equal to the primary only
/// enter once.
fn server_join_addresses(server: &MipServerBinding) -> Vec<String> {
    let joinable = |endpoint: &MipServerEndpoint| {
        // 基岩版线路进不了 Java 启动器的服务器列表。
        endpoint
            .edition
            .as_deref()
            .is_none_or(|edition| edition.eq_ignore_ascii_case("java"))
            && endpoint
                .address
                .as_deref()
                .is_some_and(|address| !address.trim().is_empty())
    };
    let mut endpoints: Vec<&MipServerEndpoint> = server
        .endpoints
        .iter()
        .flatten()
        .filter(|endpoint| joinable(endpoint))
        .collect();
    // 主线路排最前；稳定排序保持同旗标间适配器给的顺序。
    endpoints.sort_by_key(|endpoint| !endpoint.primary.unwrap_or(false));
    let mut addresses: Vec<String> = Vec::new();
    let mut push = |address: Option<&String>| {
        if let Some(address) = address
            .map(|address| address.trim())
            .filter(|address| !address.is_empty())
            && !addresses.iter().any(|existing| existing == address)
        {
            addresses.push(address.to_string());
        }
    };
    if endpoints.is_empty() {
        push(server.mc_address.as_ref());
    } else {
        for endpoint in endpoints {
            push(endpoint.address.as_ref());
        }
    }
    addresses
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn not_found_error() -> crate::Error {
        crate::ErrorKind::HttpError {
            status: 404,
            method: "GET".to_string(),
            url: "test://mip".to_string(),
        }
        .into()
    }

    /// YDA missing-artifact shape: HTTP 400 + `文件不存在` envelope.
    fn yda_missing_artifact_error() -> crate::Error {
        crate::ErrorKind::LabrinthError(crate::LabrinthError {
            error: "domain_error".to_string(),
            description: "文件不存在".to_string(),
            status: Some(400),
            method: Some("GET".to_string()),
            url: Some("test://mip/mrpack".to_string()),
            route: None,
        })
        .into()
    }

    #[test]
    fn yda_missing_artifact_counts_as_not_found() {
        assert!(is_not_found(&yda_missing_artifact_error()));
        assert!(is_not_found(&not_found_error()));
        assert!(!is_not_found(&other_error()));
    }

    fn other_error() -> crate::Error {
        crate::ErrorKind::OtherError("boom".to_string()).into()
    }

    fn manifest(version: &str, parent: Option<&str>) -> MipManifest {
        MipManifest {
            format_version: 1,
            pack_id: "pack".to_string(),
            version: version.to_string(),
            parent: parent.map(str::to_string),
            channel: None,
            game: None,
            features: Vec::new(),
            files: Vec::new(),
        }
    }

    struct MemoryManifests {
        manifests: HashMap<String, MipManifest>,
    }

    impl ManifestSource for MemoryManifests {
        async fn fetch(&self, version: &str) -> crate::Result<MipManifest> {
            self.manifests
                .get(version)
                .cloned()
                .ok_or_else(not_found_error)
        }
    }

    struct MemoryMrpack {
        full_versions: Vec<&'static str>,
    }

    impl MrpackSource for MemoryMrpack {
        async fn fetch(
            &self,
            version: &str,
            _features: &[String],
        ) -> crate::Result<Vec<u8>> {
            if self.full_versions.contains(&version) {
                Ok(b"mrpack-bytes".to_vec())
            } else {
                Err(not_found_error())
            }
        }
    }

    #[tokio::test]
    async fn baseline_walk_picks_nearest_ancestor_with_mrpack() {
        let mut chain_manifests = HashMap::new();
        chain_manifests.insert("1.0.0".to_string(), manifest("1.0.0", None));
        chain_manifests.insert("1.1.0".to_string(), manifest("1.1.0", Some("1.0.0")));
        let target = manifest("1.2.0", Some("1.1.0"));

        let chain = parent_chain(&target, &MemoryManifests { manifests: chain_manifests })
            .await
            .unwrap();
        assert_eq!(
            chain.iter().map(|m| m.version.as_str()).collect::<Vec<_>>(),
            vec!["1.1.0", "1.0.0"]
        );

        // 1.1.0 (nearest) has no mrpack either; the root 1.0.0 does.
        let source = MemoryMrpack { full_versions: vec!["1.0.0"] };
        let (baseline, bytes) = baseline_mrpack(&chain, &source, &[])
            .await
            .unwrap()
            .expect("root baseline must be found");
        assert_eq!(baseline, "1.0.0");
        assert_eq!(bytes, b"mrpack-bytes");
    }

    #[tokio::test]
    async fn baseline_walk_returns_none_when_no_ancestor_has_mrpack() {
        let mut chain_manifests = HashMap::new();
        chain_manifests.insert("1.0.0".to_string(), manifest("1.0.0", None));
        let target = manifest("1.1.0", Some("1.0.0"));
        let chain = parent_chain(&target, &MemoryManifests { manifests: chain_manifests })
            .await
            .unwrap();

        let source = MemoryMrpack { full_versions: vec![] };
        assert!(baseline_mrpack(&chain, &source, &[]).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn broken_parent_link_ends_the_walk_without_failing() {
        // 1.1.0 declares parent 1.0.0 whose manifest is gone (withdrawn).
        let mut chain_manifests = HashMap::new();
        chain_manifests.insert("1.1.0".to_string(), manifest("1.1.0", Some("1.0.0")));
        let target = manifest("1.2.0", Some("1.1.0"));

        let chain = parent_chain(&target, &MemoryManifests { manifests: chain_manifests })
            .await
            .unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].version, "1.1.0");
    }

    #[tokio::test]
    async fn transport_errors_fail_the_walk_loudly() {
        struct FailingSource;
        impl ManifestSource for FailingSource {
            async fn fetch(&self, _version: &str) -> crate::Result<MipManifest> {
                Err(other_error())
            }
        }
        let target = manifest("1.1.0", Some("1.0.0"));
        let result = parent_chain(&target, &FailingSource).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn parent_cycles_are_cut_off() {
        // 1.0.0 → 1.1.0 → 1.0.0 → …: the walk must terminate.
        let mut chain_manifests = HashMap::new();
        chain_manifests.insert("1.0.0".to_string(), manifest("1.0.0", Some("1.1.0")));
        chain_manifests.insert("1.1.0".to_string(), manifest("1.1.0", Some("1.0.0")));
        let target = manifest("1.2.0", Some("1.1.0"));

        let chain = parent_chain(&target, &MemoryManifests { manifests: chain_manifests })
            .await
            .unwrap();
        assert_eq!(chain.len(), 2);
    }

    #[tokio::test]
    async fn baseline_feature_query_is_filtered_to_the_baseline_catalog() {
        let mut baseline = manifest("1.0.0", None);
        baseline.features.push(super::super::manifest::MipFeature {
            id: "old-feature".to_string(),
            name: None,
            default: false,
            conflicts: Vec::new(),
        });
        let chain = vec![baseline];

        struct RecordingSource {
            seen: std::sync::Mutex<Vec<Vec<String>>>,
        }
        impl MrpackSource for RecordingSource {
            async fn fetch(
                &self,
                _version: &str,
                features: &[String],
            ) -> crate::Result<Vec<u8>> {
                self.seen.lock().unwrap().push(features.to_vec());
                Ok(vec![])
            }
        }

        let source = RecordingSource { seen: std::sync::Mutex::new(Vec::new()) };
        baseline_mrpack(
            &chain,
            &source,
            &[
                "old-feature".to_string(),
                "added-in-later-version".to_string(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(
            source.seen.into_inner().unwrap(),
            vec![vec!["old-feature".to_string()]]
        );
    }

    #[test]
    fn only_404_counts_as_missing_mrpack() {
        assert!(is_not_found(&not_found_error()));
        assert!(!is_not_found(&other_error()));
        assert!(!is_not_found(&crate::ErrorKind::HttpError {
            status: 400,
            method: "GET".to_string(),
            url: "test://mip".to_string(),
        }
        .into()));
    }

    fn join_server(mc_address: Option<&str>, endpoints: Vec<MipServerEndpoint>) -> MipServerBinding {
        MipServerBinding {
            server_id: "srv".to_string(),
            name: None,
            status: None,
            binding: None,
            mc_address: mc_address.map(str::to_string),
            endpoints: (!endpoints.is_empty()).then_some(endpoints),
            current_season: None,
        }
    }

    fn endpoint(address: &str, primary: bool, edition: Option<&str>) -> MipServerEndpoint {
        MipServerEndpoint {
            address: Some(address.to_string()),
            primary: Some(primary),
            edition: edition.map(str::to_string),
            name: None,
        }
    }

    #[test]
    fn join_addresses_cover_primary_and_backup_lines() {
        // 主线 + 备用线路都进列表；与主线重复的备用行只进一次。
        let server = join_server(
            Some("main.example.org"),
            vec![
                endpoint("main.example.org", true, None),
                endpoint("backup-1.example.org:25566", false, None),
                endpoint("main.example.org", false, None),
            ],
        );
        assert_eq!(
            server_join_addresses(&server),
            vec!["main.example.org", "backup-1.example.org:25566"]
        );
    }

    #[test]
    fn join_addresses_put_flagged_primary_first_without_losing_order() {
        // 无 primary 旗标时首个线路当主线，适配器给的顺序保持。
        let server = join_server(
            None,
            vec![
                endpoint("line-b.example.org", false, None),
                endpoint("line-a.example.org", false, None),
            ],
        );
        assert_eq!(
            server_join_addresses(&server),
            vec!["line-b.example.org", "line-a.example.org"]
        );
    }

    #[test]
    fn join_addresses_fall_back_to_mc_address() {
        let server = join_server(Some("main.example.org"), vec![]);
        assert_eq!(server_join_addresses(&server), vec!["main.example.org"]);

        // 端点存在但地址全空同样回落到 mcAddress。
        let empty = join_server(
            Some("main.example.org"),
            vec![endpoint("   ", true, None)],
        );
        assert_eq!(
            server_join_addresses(&empty),
            vec!["main.example.org"]
        );
    }

    #[test]
    fn join_addresses_drop_non_java_lines_and_trim() {
        let server = join_server(
            None,
            vec![
                endpoint("  main.example.org  ", true, Some("java")),
                endpoint("bedrock.example.org", false, Some("bedrock")),
            ],
        );
        assert_eq!(server_join_addresses(&server), vec!["main.example.org"]);
    }
}

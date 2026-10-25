//! 运行时解析 authlib-injector-union 代理 jar。
//!
//! union fork 经 Yudream Nexus maven 仓库分发，优先于编译期内置的上游
//! agent：每次 Yggdrasil 启动先查 metadata 上的最新 release，命中缓存
//! 直接复用，有新版则下载更新；union 源不可达（包括首次启动就断网）
//! 时回退到缓存里的历史 union jar，再退到调用方内置的上游 jar。

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::State;

const UNION_METADATA_URL: &str = "https://nexus.yudream.online/repository/maven-releases/online/yudream/authlib-injector-union/maven-metadata.xml";
const UNION_JAR_URL_TEMPLATE: &str = "https://nexus.yudream.online/repository/maven-releases/online/yudream/authlib-injector-union/{version}/authlib-injector-union-{version}.jar";
/// metadata 不存在（Nexus 未生成）但站点可达时的兜底版本。
const UNION_FALLBACK_VERSION: &str = "1.4.0";
/// 启动链路不能被失效的源长时间阻塞：元数据探测要短。
const METADATA_TIMEOUT: Duration = Duration::from_secs(5);
const JAR_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);
/// 防御异常源塞大文件填满磁盘。
const MAX_JAR_BYTES: u64 = 16 * 1024 * 1024;
const CACHE_DIR_NAME: &str = "authlib-injector-union";
const CACHE_FILE_PREFIX: &str = "authlib-injector-union-";

/// 解析本次 Yggdrasil 启动应注入的 union agent jar 路径。
///
/// 返回 `None` 表示 union 源不可用且无任何缓存，调用方应回退到编译期
/// 内置的上游 jar。任何失败都不应阻断启动。
pub async fn resolve_union_agent_jar() -> Option<PathBuf> {
    let state = State::get().await.ok()?;
    let cache_dir = state.directories.caches_dir().join(CACHE_DIR_NAME);
    match fetch_metadata_outcome(&state).await {
        MetadataOutcome::Version(version) => {
            let target =
                cache_dir.join(format!("{CACHE_FILE_PREFIX}{version}.jar"));
            if !target.exists()
                && download_union_jar(&state, &version, &target).await.is_err()
            {
                // 新版下载失败：退回缓存里已有的历史版本。
                return newest_cached_jar(&cache_dir);
            }
            prune_stale_cached_jars(&cache_dir, &target);
            Some(target)
        }
        // 站点可达但没有 metadata（Nexus 手工部署的仓库常见）：
        // 退回用户当前指定的字面量版本尝试下载，失败再吃缓存。
        MetadataOutcome::NoMetadata => {
            let target = cache_dir.join(format!(
                "{CACHE_FILE_PREFIX}{UNION_FALLBACK_VERSION}.jar"
            ));
            if !target.exists()
                && download_union_jar(&state, UNION_FALLBACK_VERSION, &target)
                    .await
                    .is_err()
            {
                return newest_cached_jar(&cache_dir);
            }
            prune_stale_cached_jars(&cache_dir, &target);
            Some(target)
        }
        // 站点不可达：绝不在线重试下载，只能吃缓存；无缓存则回退内置。
        MetadataOutcome::SiteDown => newest_cached_jar(&cache_dir),
    }
}

/// metadata 探测结果：解析出版本 / 站点在但无 metadata / 站点不可达。
#[derive(Clone, Debug, Eq, PartialEq)]
enum MetadataOutcome {
    Version(String),
    NoMetadata,
    SiteDown,
}

/// 从 maven-metadata.xml 取最新 release 版本号。
async fn fetch_metadata_outcome(state: &State) -> MetadataOutcome {
    let attempt = tokio::time::timeout(METADATA_TIMEOUT, async {
        state
            .configured_http_client()
            .get(UNION_METADATA_URL)
            .send()
            .await
    })
    .await;
    let response = match attempt {
        Ok(Ok(response)) => response,
        // 超时或连接失败：站点不可达。
        _ => return MetadataOutcome::SiteDown,
    };
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return MetadataOutcome::NoMetadata;
    }
    if !response.status().is_success() {
        return MetadataOutcome::SiteDown;
    }
    let Ok(xml) = response.text().await else {
        return MetadataOutcome::SiteDown;
    };
    match extract_tag(&xml, "release").or_else(|| extract_tag(&xml, "latest")) {
        Some(release) if release == extract_version(&release) => {
            MetadataOutcome::Version(release)
        }
        // 站点在但 metadata 内容不可解析：按字面量版本兜底。
        _ => MetadataOutcome::Version(String::from(UNION_FALLBACK_VERSION)),
    }
}

/// 从 `<tag>value</tag>` 提取去空白值；不做完整 XML 解析。
fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = start + xml[start..].find(&close)?;
    let value = xml[start..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// 仅接受 `1.2.3[-meta]` 式安全版本号：拼 URL 前挡住路径注入。
fn extract_version(version: &str) -> &str {
    let version = version.trim();
    let safe = !version.is_empty()
        && version.len() <= 32
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if safe { version } else { "" }
}

async fn download_union_jar(
    state: &State,
    version: &str,
    destination: &Path,
) -> crate::Result<()> {
    let url = UNION_JAR_URL_TEMPLATE.replace("{version}", version);
    let bytes = tokio::time::timeout(JAR_DOWNLOAD_TIMEOUT, async {
        state
            .configured_http_client()
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await
    })
    .await
    .map_err(|_| {
        crate::ErrorKind::OtherError(
            "authlib-injector-union download timed out".to_string(),
        )
    })??;
    if bytes.len() as u64 > MAX_JAR_BYTES {
        return Err(crate::ErrorKind::OtherError(format!(
            "authlib-injector-union jar exceeds {MAX_JAR_BYTES} bytes"
        ))
        .into());
    }
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    // 临时文件 + 原子改名：中断不会留下半截 jar 被下次启动当缓存吃掉。
    let temporary =
        destination.with_extension(format!("jar.{}.tmp", std::process::id()));
    tokio::fs::write(&temporary, &bytes).await?;
    tokio::fs::rename(&temporary, destination).await?;
    Ok(())
}

/// 缓存目录里版本最新的 union jar（无缓存返回 `None`）。
fn newest_cached_jar(cache_dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(cache_dir).ok()?;
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            let version = name
                .strip_prefix(CACHE_FILE_PREFIX)?
                .strip_suffix(".jar")?
                .to_string();
            (extract_version(&version) == version)
                .then_some((version, entry.path()))
        })
        .max_by(|(left, _), (right, _)| {
            compare_versions(left, right).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, path)| path)
}

/// 只保留当前在用的 jar，避免旧版本无限堆积；失败静默（非关键路径）。
fn prune_stale_cached_jars(cache_dir: &Path, keep: &Path) {
    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path != keep
                && path
                    .extension()
                    .is_some_and(|ext| ext == "jar" || ext == "tmp")
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

/// 按点分段数值比较；段数不齐时短者在前，非数字段按字符串比。
fn compare_versions(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let numeric = |part: &str| part.parse::<u64>().ok();
    let mut lefts = left.split('.');
    let mut rights = right.split('.');
    loop {
        match (lefts.next(), rights.next()) {
            (None, None) => return Some(std::cmp::Ordering::Equal),
            (None, Some(_)) => return Some(std::cmp::Ordering::Less),
            (Some(_), None) => return Some(std::cmp::Ordering::Greater),
            (Some(l), Some(r)) => {
                let ordering = match (numeric(l), numeric(r)) {
                    (Some(ln), Some(rn)) => ln.cmp(&rn),
                    _ => l.cmp(r),
                };
                if ordering != std::cmp::Ordering::Equal {
                    return Some(ordering);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_METADATA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata>
  <groupId>online.yudream</groupId>
  <artifactId>authlib-injector-union</artifactId>
  <versioning>
    <latest>1.4.0</latest>
    <release>1.4.0</release>
    <versions>
      <version>1.4.0</version>
    </versions>
    <lastUpdated>20261025211411</lastUpdated>
  </versioning>
</metadata>"#;

    #[test]
    fn metadata_release_tag_wins() {
        assert_eq!(
            extract_tag(SAMPLE_METADATA, "release").as_deref(),
            Some("1.4.0")
        );
        assert_eq!(
            extract_tag(SAMPLE_METADATA, "latest").as_deref(),
            Some("1.4.0")
        );
        assert_eq!(extract_tag(SAMPLE_METADATA, "missing"), None);
    }

    #[test]
    fn unsafe_versions_are_rejected() {
        assert_eq!(extract_version("1.4.0"), "1.4.0");
        assert_eq!(extract_version("1.4.0-union"), "1.4.0-union");
        assert_eq!(extract_version(""), "");
        assert_eq!(extract_version("../../etc"), "");
        assert_eq!(extract_version("1.4.0\n<script>"), "");
    }

    #[test]
    fn version_comparison_is_numeric_per_segment() {
        assert_eq!(
            compare_versions("1.4.0", "1.10.0"),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            compare_versions("1.4.0", "1.4.0"),
            Some(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            compare_versions("1.10", "1.4"),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(
            compare_versions("1.4", "1.4.0"),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            compare_versions("1.4.0", "1.4.0-union"),
            Some(std::cmp::Ordering::Less)
        );
    }
}

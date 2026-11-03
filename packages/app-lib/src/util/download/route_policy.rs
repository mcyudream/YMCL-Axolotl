//! Pure route classification helpers used by the download coordinator.

use crate::util::fetch::ResourceClass;
use url::Url;

pub(crate) fn is_official_modrinth_download_url(url: &str) -> bool {
    Url::parse(url).is_ok_and(|url| {
        matches!(
            url.host_str(),
            Some(
                "api.modrinth.com"
                    | "cdn.modrinth.com"
                    | "cdn-alt.modrinth.com"
            )
        )
    })
}

pub(crate) fn is_official_version_manifest_url(url: &str) -> bool {
    Url::parse(url).is_ok_and(|url| {
        matches!(
            url.host_str(),
            Some("piston-meta.mojang.com" | "launchermeta.mojang.com")
        ) && url.path().contains("version_manifest")
    })
}

/// Stable PDS object and resource-proxy entrances advertised by the adapter.
/// Match the path so custom domain origins and deployment prefixes work.
pub(crate) fn is_pds_download_url(url: &str) -> bool {
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    if let Some((_, hash)) = url.path().rsplit_once("/mip/objects/") {
        return hash.len() == 128
            && hash.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    let Some((_, path)) = url
        .path()
        .rsplit_once("/api/plugins/ymcl-adapter/v1/proxy/")
    else {
        return false;
    };
    let parts = path.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        ["data", project, "versions", version, file] => {
            !project.is_empty() && !version.is_empty() && !file.is_empty()
        }
        ["files", prefix, suffix, file] => {
            !prefix.is_empty() && !suffix.is_empty() && !file.is_empty()
        }
        _ => false,
    }
}

/// Modrinth and CurseForge content defaults to mirror-first in Automatic
/// mode: the built-in mirrors lead and the official source stays available
/// as the fallback for files they have not synced.
pub(crate) fn prefers_mirror_routes(resource: ResourceClass) -> bool {
    matches!(
        resource,
        ResourceClass::Modrinth
            | ResourceClass::Modpack
            | ResourceClass::CurseForge
    )
}

/// Whether Automatic ordering ranks mirrors ahead of the official source
/// for this URL and resource class.
pub(crate) fn orders_mirrors_first(url: &str, resource: ResourceClass) -> bool {
    uses_mirror_first_loader_routes(url, resource)
        || prefers_mirror_routes(resource)
}

/// Loader Maven repositories served mirror-first while retaining an official
/// fallback for content the mirrors have not synced yet.
pub(crate) fn uses_mirror_first_loader_routes(
    url: &str,
    resource: ResourceClass,
) -> bool {
    if !matches!(
        resource,
        ResourceClass::MinecraftLibrary | ResourceClass::Loader
    ) {
        return false;
    }
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    if matches!(
        url.host_str(),
        Some(
            "maven.minecraftforge.net"
                | "maven.fabricmc.net"
                | "maven.neoforged.net"
        )
    ) {
        return true;
    }
    let path = url.path().to_ascii_lowercase();
    ["minecraftforge", "fabricmc", "neoforged"]
        .iter()
        .any(|loader| path.contains(loader))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pds_download_urls_accept_objects_and_resource_proxies() {
        let hash = "aB01".repeat(32);
        for url in [
            format!("https://site.example/mip/objects/{hash}"),
            format!(
                "https://site.example/prefix/api/plugins/ymcl-adapter/mip/objects/{hash}?download=1"
            ),
            format!("http://127.0.0.1:8080/mip/objects/{hash}"),
            "https://site.example/prefix/api/plugins/ymcl-adapter/v1/proxy/data/project/versions/version/mod.jar?key=token".to_string(),
            "https://site.example/api/plugins/ymcl-adapter/v1/proxy/files/1234/567/mod.jar".to_string(),
        ] {
            assert!(is_pds_download_url(&url), "{url}");
        }
    }

    #[test]
    fn pds_download_urls_reject_other_sources_and_invalid_paths() {
        for url in [
            "not a url".to_string(),
            "https://cdn.modrinth.com/data/project/versions/version/mod.jar".to_string(),
            "https://site.example/mip/objects/invalid".to_string(),
            format!("https://site.example/mip/objects/{}", "g".repeat(128)),
            format!("ftp://site.example/mip/objects/{}", "a".repeat(128)),
            "https://site.example/api/plugins/ymcl-adapter/v1/proxy/data/project/versions/version/".to_string(),
            "https://site.example/api/plugins/ymcl-adapter/v1/proxy/files/1234/mod.jar".to_string(),
            "https://site.example/api/plugins/ymcl-adapter/v1/proxy/maven/mod.jar".to_string(),
        ] {
            assert!(!is_pds_download_url(&url), "{url}");
        }
    }
}

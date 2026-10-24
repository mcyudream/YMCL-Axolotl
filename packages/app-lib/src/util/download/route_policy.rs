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

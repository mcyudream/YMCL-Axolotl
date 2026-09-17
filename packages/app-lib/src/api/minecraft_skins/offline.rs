use std::{
    io::{Cursor, Write},
    path::Path,
};

use serde_json::json;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    ErrorKind, State,
    state::minecraft_skins::{CustomMinecraftSkin, OfflineMinecraftSkin},
};

use super::{Credentials, png_util};

pub(crate) const OFFLINE_SKIN_PACK_FILE_NAME: &str = "Axolotl Offline Skin.zip";
pub(crate) const OFFLINE_SKIN_PACK_LEGACY_ID: &str = "Axolotl Offline Skin.zip";
pub(crate) const OFFLINE_SKIN_PACK_MODERN_ID: &str =
    "file/Axolotl Offline Skin.zip";

#[derive(Debug, Clone, Copy)]
pub(crate) struct OfflineSkinPackOptions {
    pub enabled_pack_id: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MinecraftReleaseVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl MinecraftReleaseVersion {
    fn parse(version: &str) -> Option<Self> {
        let mut parts = version.split('.');
        let major = leading_number(parts.next()?)?;
        let minor = leading_number(parts.next()?)?;
        let patch = parts.next().and_then(leading_number).unwrap_or(0);

        Some(Self {
            major,
            minor,
            patch,
        })
    }

    fn supports_resource_packs(self) -> bool {
        self.major > 1 || self.minor >= 6
    }

    fn uses_modern_pack_id(self) -> bool {
        self.major > 1 || self.minor >= 13
    }

    fn uses_modern_player_texture_paths(self) -> bool {
        self.major > 1
            || self.minor > 19
            || (self.minor == 19 && self.patch >= 3)
    }

    fn needs_legacy_skin_height(self) -> bool {
        self.major == 1 && matches!(self.minor, 6 | 7)
    }

    fn resource_pack_format(self) -> u32 {
        if self.major > 1 {
            return 75;
        }

        match self.minor {
            0..=8 => 1,
            9..=10 => 2,
            11..=12 => 3,
            13..=14 => 4,
            15 => 5,
            16 if self.patch <= 1 => 5,
            16 => 6,
            17 => 7,
            18 => 8,
            19 if self.patch <= 2 => 9,
            19 if self.patch == 3 => 12,
            19 => 13,
            20 if self.patch <= 1 => 15,
            20 if self.patch == 2 => 18,
            20 if self.patch <= 4 => 22,
            20 => 32,
            21 if self.patch <= 1 => 34,
            21 if self.patch <= 3 => 42,
            21 if self.patch == 4 => 46,
            21 if self.patch == 5 => 55,
            21 if self.patch == 6 => 63,
            21 if self.patch <= 8 => 64,
            21 if self.patch <= 10 => 69,
            21 => 75,
            _ => 75,
        }
    }
}

fn leading_number(value: &str) -> Option<u32> {
    let digits = value
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
}

/// Builds (or removes) the resource pack that replaces vanilla's local default
/// player texture with the selected offline skin.
pub(crate) async fn prepare_offline_skin_resource_pack(
    credentials: &Credentials,
    instance_path: &Path,
    game_version: &str,
) -> crate::Result<OfflineSkinPackOptions> {
    let resource_pack_dir = instance_path.join("resourcepacks");
    let resource_pack_path =
        resource_pack_dir.join(OFFLINE_SKIN_PACK_FILE_NAME);
    let Some(version) = MinecraftReleaseVersion::parse(game_version) else {
        remove_pack_if_present(&resource_pack_path).await?;
        return Ok(OfflineSkinPackOptions {
            enabled_pack_id: None,
        });
    };

    if !credentials.uses_local_skins() || !version.supports_resource_packs() {
        remove_pack_if_present(&resource_pack_path).await?;
        return Ok(OfflineSkinPackOptions {
            enabled_pack_id: None,
        });
    }

    let state = State::get().await?;
    let Some(offline_skin) =
        OfflineMinecraftSkin::get(credentials.offline_profile.id, &state.pool)
            .await?
    else {
        remove_pack_if_present(&resource_pack_path).await?;
        return Ok(OfflineSkinPackOptions {
            enabled_pack_id: None,
        });
    };

    let Some(saved_skin) = CustomMinecraftSkin::get_by_texture(
        credentials.offline_profile.id,
        &offline_skin.texture_key,
        &state.pool,
    )
    .await?
    else {
        OfflineMinecraftSkin::clear(
            credentials.offline_profile.id,
            &state.pool,
        )
        .await?;
        remove_pack_if_present(&resource_pack_path).await?;
        return Ok(OfflineSkinPackOptions {
            enabled_pack_id: None,
        });
    };

    let mut texture = saved_skin.texture_blob(&state.pool).await?;
    if version.needs_legacy_skin_height()
        && png_util::dimensions(&texture)?.1 == 64
    {
        texture = png_util::to_legacy_client_texture(&texture)?.to_vec();
    }

    let zip_data = build_resource_pack(&texture, version)?;
    tokio::fs::create_dir_all(&resource_pack_dir).await?;
    tokio::fs::write(&resource_pack_path, zip_data).await?;

    Ok(OfflineSkinPackOptions {
        enabled_pack_id: Some(if version.uses_modern_pack_id() {
            OFFLINE_SKIN_PACK_MODERN_ID
        } else {
            OFFLINE_SKIN_PACK_LEGACY_ID
        }),
    })
}

async fn remove_pack_if_present(path: &Path) -> crate::Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn build_resource_pack(
    texture: &[u8],
    version: MinecraftReleaseVersion,
) -> crate::Result<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    let pack_format = version.resource_pack_format();
    let metadata = json!({
        "pack": {
            "pack_format": pack_format,
            "description": "YMCL (YuDream Launcher) offline skin"
        }
    });

    start_zip_file(&mut zip, "pack.mcmeta", options)?;
    zip.write_all(&serde_json::to_vec(&metadata)?)?;

    if version.uses_modern_player_texture_paths() {
        for model in ["slim", "wide"] {
            for skin_name in [
                "alex", "ari", "efe", "kai", "makena", "noor", "steve",
                "sunny", "zuri",
            ] {
                let path = format!(
                    "assets/minecraft/textures/entity/player/{model}/{skin_name}.png"
                );
                start_zip_file(&mut zip, &path, options)?;
                zip.write_all(texture)?;
            }
        }
    } else {
        // Replace both models. This keeps the standard offline UUID stable, so
        // changing a skin cannot split single-player saves into a new player.
        for path in [
            "assets/minecraft/textures/entity/steve.png",
            "assets/minecraft/textures/entity/alex.png",
        ] {
            start_zip_file(&mut zip, path, options)?;
            zip.write_all(texture)?;
        }
    }

    zip.finish().map(Cursor::into_inner).map_err(zip_error)
}

fn start_zip_file(
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    path: &str,
    options: SimpleFileOptions,
) -> crate::Result<()> {
    zip.start_file(path, options).map_err(zip_error)
}

fn zip_error(error: zip::result::ZipError) -> crate::Error {
    ErrorKind::OtherError(format!(
        "Failed to build offline skin resource pack: {error}"
    ))
    .as_error()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_versions_and_selects_pack_formats() {
        let version = MinecraftReleaseVersion::parse("1.19.2").unwrap();
        assert_eq!(version.resource_pack_format(), 9);
        assert!(!version.uses_modern_player_texture_paths());

        let version = MinecraftReleaseVersion::parse("1.19.3").unwrap();
        assert_eq!(version.resource_pack_format(), 12);
        assert!(version.uses_modern_player_texture_paths());

        let version = MinecraftReleaseVersion::parse("1.21.8").unwrap();
        assert_eq!(version.resource_pack_format(), 64);
        assert!(version.uses_modern_pack_id());
    }

    #[test]
    fn rejects_snapshot_versions_without_a_release_shape() {
        assert_eq!(MinecraftReleaseVersion::parse("25w31a"), None);
    }

    #[test]
    fn generated_pack_replaces_all_modern_default_player_textures() {
        let texture = include_bytes!("assets/default/MissingNo.png");
        let version = MinecraftReleaseVersion::parse("1.19.3").unwrap();
        let pack = build_resource_pack(texture, version).unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(pack)).unwrap();

        assert!(archive.by_name("pack.mcmeta").is_ok());
        assert!(
            archive
                .by_name(
                    "assets/minecraft/textures/entity/player/slim/alex.png"
                )
                .is_ok()
        );
        assert!(
            archive
                .by_name(
                    "assets/minecraft/textures/entity/player/wide/zuri.png"
                )
                .is_ok()
        );
        assert_eq!(archive.len(), 19);
    }
}

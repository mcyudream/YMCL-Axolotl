use crate::mod_metadata::LocalModMetadata;
use crate::state::{Project, SideType, VersionEnvironment};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ModSide {
    Client,
    Server,
    Both,
    #[default]
    Unknown,
}

impl ModSide {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Client => "client",
            Self::Server => "server",
            Self::Both => "both",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModServerSupport {
    Required,
    Optional,
    Unsupported,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModSideSource {
    Instance,
    Local,
    Modrinth,
    Curseforge,
    Embedded,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModSideInfo {
    pub side: ModSide,
    pub server_support: ModServerSupport,
    pub source: ModSideSource,
}

impl Default for ModSideInfo {
    fn default() -> Self {
        Self {
            side: ModSide::Unknown,
            server_support: ModServerSupport::Unknown,
            source: ModSideSource::Unknown,
        }
    }
}

impl ModSideInfo {
    pub fn manual(side: ModSide, source: ModSideSource) -> Self {
        Self {
            side,
            server_support: match side {
                ModSide::Client => ModServerSupport::Unsupported,
                ModSide::Server | ModSide::Both => ModServerSupport::Required,
                ModSide::Unknown => ModServerSupport::Unknown,
            },
            source,
        }
    }
}

/// Only explicit environment metadata is evidence. Categories, dependency
/// sides and Forge's displayTest are not declarations of a mod's own side.
pub(crate) fn detect_mod_side(
    environment: Option<VersionEnvironment>,
    project: Option<&Project>,
    curseforge_environments: impl Iterator<Item = impl AsRef<str>>,
    local: Option<&LocalModMetadata>,
) -> ModSideInfo {
    use ModServerSupport as Support;
    use ModSide as Side;
    use ModSideSource as Source;
    use VersionEnvironment as Env;
    if let Some((side, server_support)) = environment.and_then(|env| {
        Some(match env {
            Env::ClientOnly | Env::SingleplayerOnly => {
                (Side::Client, Support::Unsupported)
            }
            Env::ClientOnlyServerOptional => (Side::Both, Support::Optional),
            Env::ServerOnly | Env::DedicatedServerOnly => {
                (Side::Server, Support::Required)
            }
            Env::ServerOnlyClientOptional => (Side::Both, Support::Required),
            Env::ClientAndServer => (Side::Both, Support::Required),
            Env::ClientOrServer | Env::ClientOrServerPrefersBoth => {
                (Side::Both, Support::Optional)
            }
            Env::Unknown => return None,
        })
    }) {
        return ModSideInfo {
            side,
            server_support,
            source: Source::Modrinth,
        };
    }
    if let Some(project) = project {
        let client = &project.client_side;
        let server = &project.server_side;
        let supports = |side: &SideType| {
            matches!(side, SideType::Required | SideType::Optional)
        };
        let side = if *server == SideType::Unsupported && supports(client) {
            Some(Side::Client)
        } else if *client == SideType::Unsupported && supports(server) {
            Some(Side::Server)
        } else if supports(client) && supports(server) {
            Some(Side::Both)
        } else {
            None
        };
        if let Some(side) = side {
            return ModSideInfo {
                side,
                server_support: match server {
                    SideType::Required => Support::Required,
                    SideType::Optional => Support::Optional,
                    SideType::Unsupported => Support::Unsupported,
                    SideType::Unknown => Support::Unknown,
                },
                source: Source::Modrinth,
            };
        }
    }
    let mut client = false;
    let mut server = false;
    for environment in curseforge_environments {
        match environment.as_ref().trim().to_ascii_lowercase().as_str() {
            "client" | "client-side" | "client only" => client = true,
            "server" | "server-side" | "server only" => server = true,
            "client and server" | "client & server" | "both" => {
                client = true;
                server = true;
            }
            _ => {}
        }
    }
    if client || server {
        return ModSideInfo::manual(
            match (client, server) {
                (true, true) => Side::Both,
                (true, false) => Side::Client,
                _ => Side::Server,
            },
            Source::Curseforge,
        );
    }
    local
        .and_then(|metadata| metadata.environment)
        .map(|side| ModSideInfo::manual(side, Source::Embedded))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_server_support_is_not_client_only() {
        let optional = detect_mod_side(
            Some(VersionEnvironment::ClientOnlyServerOptional),
            None,
            std::iter::empty::<&str>(),
            None,
        );
        assert_eq!(optional.side, ModSide::Both);
        assert_eq!(optional.server_support, ModServerSupport::Optional);
        let client = detect_mod_side(
            Some(VersionEnvironment::SingleplayerOnly),
            None,
            ["Server"].into_iter(),
            None,
        );
        assert_eq!(client.server_support, ModServerSupport::Unsupported);
        assert_eq!(client.source, ModSideSource::Modrinth);
    }

    #[test]
    fn curseforge_requires_explicit_environment_tags() {
        let detect = |tags: Vec<&str>| {
            detect_mod_side(None, None, tags.into_iter(), None)
        };
        assert_eq!(detect(vec!["1.20.1", "Forge"]).side, ModSide::Unknown);
        assert_eq!(detect(vec!["Client"]).side, ModSide::Client);
        assert_eq!(detect(vec!["Server"]).side, ModSide::Server);
        assert_eq!(detect(vec!["Client", "Server"]).side, ModSide::Both);
    }
}

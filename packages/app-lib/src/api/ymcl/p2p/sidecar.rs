//! `mcpanel-p2p` sidecar lifecycle for the direct-connect face (YAP §6.12):
//! locate the binary, spawn it against the domain session, wait for its
//! `MCP2P-READY` handshake and keep it alive until the game exits.
//!
//! The sidecar is the only component that talks to the domain's rendezvous:
//! it opens the session itself, binds a loopback listener and forwards the
//! game client's bytes to the node. The launcher just consumes the loopback
//! address, so players never see (or configure) any of this.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};

/// Handshake line the sidecar prints on stdout once its listener is ready.
pub const READY_PREFIX: &str = "MCP2P-READY ";

/// How long the sidecar may take to bind and report readiness. Session
/// creation is one round trip; anything slower is a network problem the
/// join flow should fall back from.
const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// Grace period for a sidecar to exit after its stdin closes (it closes the
/// session and returns on EOF) before it is killed outright.
const STOP_GRACE: Duration = Duration::from_secs(3);

const BINARY_NAME: &str = "mcpanel-p2p";
const BINARY_ENV: &str = "MCPANEL_P2P_BIN";

/// Payload of the `MCP2P-READY` line. Field names are snake_case; the
/// camelCase aliases keep 0.13.0-era sidecars working.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ReadyHandshake {
    /// Loopback address the game client should connect to.
    pub local: String,
    #[serde(rename = "session_id", alias = "sessionId", default)]
    pub session_id: Option<String>,
    #[serde(rename = "instance_id", alias = "instanceId", default)]
    pub instance_id: Option<String>,
    #[serde(rename = "target_port", alias = "targetPort", default)]
    pub target_port: Option<u16>,
}

/// Parses one stdout line; `None` for anything but a well-formed handshake.
pub fn parse_ready_line(line: &str) -> Option<ReadyHandshake> {
    let payload = line.trim().strip_prefix(READY_PREFIX)?;
    serde_json::from_str(payload).ok()
}

/// True when `address` is a loopback endpoint. The sidecar only ever binds
/// loopback; this guards the launcher side against a tampered payload
/// pointing the game at a remote listener.
pub fn is_loopback_address(address: &str) -> bool {
    let host = address
        .rsplit_once(':')
        .map(|(host, _)| host)
        .unwrap_or(address)
        .trim_start_matches('[')
        .trim_end_matches(']');
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

/// Binary file name for the running platform.
pub fn executable_name() -> String {
    if cfg!(windows) {
        format!("{BINARY_NAME}.exe")
    } else {
        BINARY_NAME.to_string()
    }
}

/// Where the sidecar may live: explicit override first, then the locations
/// packaged builds ship it to (beside the executable, its `bin/` and Tauri's
/// resource directory).
fn candidate_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os(BINARY_ENV).map(PathBuf::from) {
        candidates.push(path);
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(dir) = executable.parent()
    {
        let name = executable_name();
        candidates.push(dir.join(&name));
        candidates.push(dir.join("bin").join(&name));
        candidates.push(dir.join("resources").join(&name));
        // macOS bundles resources next to the executable directory.
        candidates.push(dir.join("..").join("Resources").join(&name));
    }
    candidates
}

/// Resolves the sidecar binary, or `None` when the build ships without it
/// (the join flow then keeps using the server's public address).
pub fn resolve_binary() -> Option<PathBuf> {
    candidate_paths().into_iter().find(|path| path.is_file())
}

/// Spawn parameters for one instance's sidecar.
#[derive(Clone, Debug)]
pub struct SidecarConfig {
    pub binary: PathBuf,
    /// Adapter rendezvous base (`…/v1/p2p`), derived from `session_url`.
    pub base_url: String,
    /// Domain access token; passed through the environment so it never
    /// shows up in a process listing.
    pub token: String,
    pub instance_id: String,
    pub log_path: Option<PathBuf>,
}

impl SidecarConfig {
    /// Strips the `/session` suffix from an advertised `session_url`, so the
    /// sidecar can derive `/session/signal` and `/session` from one base.
    pub fn base_from_session_url(session_url: &str) -> String {
        session_url
            .trim()
            .trim_end_matches('/')
            .trim_end_matches("/session")
            .trim_end_matches('/')
            .to_string()
    }
}

/// A running sidecar plus its handshake.
pub struct Sidecar {
    child: Child,
    stdin: Option<ChildStdin>,
    handshake: ReadyHandshake,
}

impl Sidecar {
    pub fn handshake(&self) -> &ReadyHandshake {
        &self.handshake
    }

    /// True while the sidecar process is still alive.
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Closes stdin (the sidecar's documented shutdown signal), waits out
    /// the grace period and kills it when it does not exit on its own.
    pub async fn stop(&mut self) {
        if let Some(mut stdin) = self.stdin.take() {
            let _ = stdin.shutdown().await;
            drop(stdin);
        }
        match tokio::time::timeout(STOP_GRACE, self.child.wait()).await {
            Ok(_) => {}
            Err(_) => {
                let _ = self.child.start_kill();
                let _ = self.child.wait().await;
            }
        }
    }
}

/// Spawns a sidecar and waits for its readiness handshake.
pub async fn spawn(config: &SidecarConfig) -> crate::Result<Sidecar> {
    let mut command = Command::new(&config.binary);
    // stdin doubles as the shutdown channel, so it must stay piped and held:
    // the sidecar treats EOF as "launcher gone" and exits at once.
    command
        .arg("--panel")
        .arg(&config.base_url)
        .arg("--instance")
        .arg(&config.instance_id)
        .arg("--local")
        .arg("127.0.0.1:0")
        .env("MCPANEL_TOKEN", &config.token)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(log_path) = config.log_path.as_deref()
        && let Some(parent) = log_path.parent()
    {
        let _ = tokio::fs::create_dir_all(parent).await;
        if let Ok(file) = std::fs::File::create(log_path) {
            command.stderr(Stdio::from(file));
        }
    }
    let mut child = command.spawn().map_err(|error| {
        crate::ErrorKind::LauncherError(format!(
            "Failed to start the direct-connect helper ({}): {error}",
            config.binary.display()
        ))
    })?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take().ok_or_else(|| {
        crate::ErrorKind::LauncherError(
            "Direct-connect helper has no output channel".to_string(),
        )
    })?;
    let mut lines = BufReader::new(stdout).lines();
    let handshake = match tokio::time::timeout(READY_TIMEOUT, async {
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(handshake) = parse_ready_line(&line) {
                return Some(handshake);
            }
        }
        None
    })
    .await
    {
        Ok(Some(handshake)) => handshake,
        Ok(None) => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(crate::ErrorKind::LauncherError(
                "Direct-connect helper exited before it was ready".to_string(),
            )
            .into());
        }
        Err(_) => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(crate::ErrorKind::LauncherError(
                "Direct-connect helper did not become ready in time"
                    .to_string(),
            )
            .into());
        }
    };
    if !is_loopback_address(&handshake.local) {
        let _ = child.start_kill();
        let _ = child.wait().await;
        return Err(crate::ErrorKind::LauncherError(format!(
            "Direct-connect helper reported a non-loopback address ({})",
            handshake.local
        ))
        .into());
    }
    // Keep draining stdout so the child never blocks on a full pipe.
    tokio::spawn(
        async move { while let Ok(Some(_)) = lines.next_line().await {} },
    );
    Ok(Sidecar {
        child,
        stdin,
        handshake,
    })
}

/// True when `path` is the sidecar shipped beside `dir` (used by the
/// resolution tests).
#[cfg(test)]
fn resolve_in(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_snake_case_handshake() {
        let line = r#"MCP2P-READY {"local":"127.0.0.1:52341","session_id":"sess-1","instance_id":"inst-1","target_port":25600}"#;
        let handshake = parse_ready_line(line).expect("handshake must parse");
        assert_eq!(handshake.local, "127.0.0.1:52341");
        assert_eq!(handshake.session_id.as_deref(), Some("sess-1"));
        assert_eq!(handshake.instance_id.as_deref(), Some("inst-1"));
        assert_eq!(handshake.target_port, Some(25600));
    }

    #[test]
    fn parses_camel_case_handshake_from_older_sidecar() {
        let line = r#"MCP2P-READY {"local":"127.0.0.1:1","sessionId":"s","instanceId":"i","targetPort":25565}"#;
        let handshake =
            parse_ready_line(line).expect("legacy handshake must parse");
        assert_eq!(handshake.session_id.as_deref(), Some("s"));
        assert_eq!(handshake.target_port, Some(25565));
    }

    #[test]
    fn rejects_lines_that_are_not_handshakes() {
        assert!(parse_ready_line("").is_none());
        assert!(parse_ready_line("MCP2P-READY ").is_none());
        assert!(parse_ready_line("starting up").is_none());
        assert!(parse_ready_line(r#"{"local":"127.0.0.1:1"}"#).is_none());
        assert!(parse_ready_line("MCP2P-READY not-json").is_none());
    }

    #[test]
    fn keeps_log_lines_away_from_the_handshake() {
        // The sidecar logs to stderr; stdout carries only the handshake.
        assert!(parse_ready_line("MCP2P-READY-EXTRA {}").is_none());
    }

    #[test]
    fn loopback_guard_accepts_only_local_addresses() {
        assert!(is_loopback_address("127.0.0.1:25565"));
        assert!(is_loopback_address("localhost:25565"));
        assert!(is_loopback_address("[::1]:25565"));
        assert!(!is_loopback_address("0.0.0.0:25565"));
        assert!(!is_loopback_address("example.com:25565"));
        assert!(!is_loopback_address("10.0.0.5:25565"));
    }

    #[test]
    fn derives_base_url_from_advertised_session_url() {
        assert_eq!(
            SidecarConfig::base_from_session_url(
                "https://panel.example.com/api/plugins/ymcl-adapter/v1/p2p/session"
            ),
            "https://panel.example.com/api/plugins/ymcl-adapter/v1/p2p"
        );
        assert_eq!(
            SidecarConfig::base_from_session_url(
                "https://panel.example.com/api/plugins/ymcl-adapter/v1/p2p"
            ),
            "https://panel.example.com/api/plugins/ymcl-adapter/v1/p2p"
        );
        // Trailing slash from a hand-written config must not survive.
        assert_eq!(
            SidecarConfig::base_from_session_url(
                " https://panel.example.com/x/session/ "
            ),
            "https://panel.example.com/x"
        );
    }

    #[test]
    fn resolves_the_binary_from_the_expected_locations() {
        let dir = std::env::temp_dir().join("ymcl-p2p-resolve-test");
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).expect("temp dir");
        let binary = bin.join(executable_name());
        std::fs::write(&binary, b"").expect("temp binary");

        assert_eq!(
            resolve_in(&[dir.clone(), bin.clone()], &executable_name()),
            Some(binary)
        );
        assert_eq!(
            resolve_in(&[dir.clone().join("missing")], &executable_name()),
            None
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn executable_name_matches_the_platform() {
        let name = executable_name();
        assert!(name.starts_with("mcpanel-p2p"));
        #[cfg(windows)]
        assert!(name.ends_with(".exe"));
    }
}

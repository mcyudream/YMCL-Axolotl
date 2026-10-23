//! Direct-connect entry face (YAP §6.12, `p2p` capability): domain servers
//! that are not reachable on a public port from the player's side. Joining
//! such a server goes through a rendezvous session plus a loopback sidecar,
//! so the game client still connects to `127.0.0.1:<port>` and nothing in
//! the player's UI changes.
//!
//! The launcher owns three things here: the capability gate, the sidecar
//! registry (one per instance, reused across relaunches) and the teardown
//! when the game exits. Session creation, candidate dialing and the ticket
//! handshake all live in the sidecar, so the token only travels to the
//! process that needs it.

pub mod sidecar;

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::manifest::{P2pEndpointConfig, YmclCapabilities, p2p_session_url};
use crate::State;

/// One node candidate offered by the rendezvous (relay endpoint or
/// hole-punching address). Informational for the launcher; the sidecar
/// walks them by priority.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct P2pCandidate {
    #[serde(default)]
    pub proto: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub priority: i32,
}

/// A rendezvous session. `ticket` is only returned to the caller that opens
/// the session, and `target_port` is pinned by the domain side — the
/// launcher cannot ask for a different host or port.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct P2pSession {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub state: String,
    /// `true` once the session reached a state it cannot leave (failed,
    /// closed, expired); the sidecar stops forwarding.
    #[serde(default)]
    pub terminal: bool,
    #[serde(default)]
    pub ticket: Option<String>,
    #[serde(rename = "target_port", default)]
    pub target_port: Option<u16>,
    #[serde(rename = "rate_kbps", default)]
    pub rate_kbps: Option<u32>,
    #[serde(rename = "expires_at", default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub candidates: Vec<P2pCandidate>,
    #[serde(default)]
    pub reason: Option<String>,
}

/// What the join flow needs to reach a server through the direct face.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct LocalEndpoint {
    /// Loopback address for the game client's quick-play connect.
    pub address: String,
    pub session_id: Option<String>,
    pub instance_id: String,
    pub target_port: Option<u16>,
}

/// Player-invisible status of the direct face, for diagnostics only.
#[derive(Serialize, Clone, Debug)]
pub struct P2pStatus {
    /// The active domain advertises the `p2p` capability.
    pub enabled: bool,
    /// A sidecar is running for this instance.
    pub active: bool,
    pub address: Option<String>,
    pub session_id: Option<String>,
    pub target_port: Option<u16>,
    /// Why the face is unavailable, when it is (shown to support, not to
    /// players).
    pub reason: Option<String>,
}

static SIDECARS: LazyLock<Mutex<HashMap<String, sidecar::Sidecar>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// True when the domain declares the `p2p` capability (YAP §6.12). A face
/// node without the capability entry still counts, mirroring how the skin
/// wardrobe is gated.
pub fn face_declared(capabilities: &YmclCapabilities) -> bool {
    capabilities.capabilities.iter().any(|cap| cap == "p2p")
        || capabilities.p2p.is_some()
}

/// Rendezvous session URL for a domain, honouring the advertised override.
pub fn session_url(origin: &str, capabilities: &YmclCapabilities) -> String {
    let configured = capabilities
        .p2p
        .as_ref()
        .and_then(|p2p: &P2pEndpointConfig| p2p.session_url.as_deref());
    p2p_session_url(origin, configured)
}

/// Whether the active domain offers the direct face.
pub async fn enabled() -> crate::Result<bool> {
    let state = State::get().await?;
    let domain_id = super::registry::active_domain_id(&state.pool).await?;
    if domain_id == super::registry::PERSONAL_DOMAIN_ID {
        return Ok(false);
    }
    let capabilities = super::registry::domain_capabilities(&domain_id).await?;
    Ok(face_declared(&capabilities))
}

/// Opens a rendezvous session for an instance (launcher-side diagnostics and
/// the explicit release path; the sidecar opens its own session when it
/// starts).
pub async fn open_session(instance_id: &str) -> crate::Result<P2pSession> {
    let (state, session_url) = resolve_face().await?;
    let domain_id = super::registry::active_domain_id(&state.pool).await?;
    let bytes = super::auth::domain_request_opt(
        &state,
        &domain_id,
        Method::POST,
        &session_url,
        Some(serde_json::json!({ "instance_id": instance_id })),
    )
    .await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Reads a session's current state.
pub async fn session_status(session_id: &str) -> crate::Result<P2pSession> {
    let (state, session_url) = resolve_face().await?;
    let domain_id = super::registry::active_domain_id(&state.pool).await?;
    let url = format!("{session_url}?session_id={}", encode_query(session_id));
    let bytes = super::auth::domain_request_opt(
        &state,
        &domain_id,
        Method::GET,
        &url,
        None,
    )
    .await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Closes a session (idempotent). Used when a sidecar died without cleaning
/// up after itself; the domain sweeper reaps the rest.
pub async fn close_session(
    session_id: &str,
    reason: &str,
) -> crate::Result<()> {
    let (state, session_url) = resolve_face().await?;
    let domain_id = super::registry::active_domain_id(&state.pool).await?;
    let url = format!(
        "{session_url}?session_id={}&reason={}",
        encode_query(session_id),
        encode_query(reason)
    );
    super::auth::domain_request_opt(
        &state,
        &domain_id,
        Method::DELETE,
        &url,
        None,
    )
    .await?;
    Ok(())
}

/// Percent-encodes a query value without pulling in a URL builder.
fn encode_query(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => encoded.push(byte as char),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// Resolves the active domain, its session URL and its capability gate.
async fn resolve_face() -> crate::Result<(std::sync::Arc<State>, String)> {
    let state = State::get().await?;
    let domain_id = super::registry::active_domain_id(&state.pool).await?;
    if domain_id == super::registry::PERSONAL_DOMAIN_ID {
        return Err(unavailable(
            "A domain is required for the direct-connect entry".to_string(),
        ));
    }
    let capabilities = super::registry::domain_capabilities(&domain_id).await?;
    if !face_declared(&capabilities) {
        return Err(unavailable(
            "This domain does not offer the direct-connect entry".to_string(),
        ));
    }
    let origin = super::registry::domain_origin(&domain_id).await?;
    Ok((state, session_url(&origin, &capabilities)))
}

fn unavailable(reason: String) -> crate::Error {
    crate::ErrorKind::OtherError(reason).into()
}

/// Returns the loopback endpoint of a running sidecar for this instance,
/// starting one when needed. Repeat joins reuse the live sidecar, so a
/// relaunch does not open a second rendezvous session.
pub async fn ensure_local_endpoint(
    instance_id: &str,
) -> crate::Result<LocalEndpoint> {
    if let Some(endpoint) = active_endpoint(instance_id) {
        return Ok(endpoint);
    }
    let (state, session_url) = resolve_face().await?;
    let domain_id = super::registry::active_domain_id(&state.pool).await?;
    let token = super::auth::ensure_session(&domain_id)
        .await?
        .map(|session| session.access_token)
        .ok_or_else(|| {
            unavailable(
                "Log in to the domain to use the direct-connect entry"
                    .to_string(),
            )
        })?;
    let binary = sidecar::resolve_binary().ok_or_else(|| {
        unavailable(format!(
            "The direct-connect helper ({}) is not installed",
            sidecar::executable_name()
        ))
    })?;
    let config = sidecar::SidecarConfig {
        binary,
        base_url: sidecar::SidecarConfig::base_from_session_url(&session_url),
        token,
        instance_id: instance_id.to_string(),
        log_path: state
            .directories
            .launcher_logs_dir()
            .map(|dir| dir.join("mcpanel-p2p.log")),
    };
    let spawned = sidecar::spawn(&config).await?;
    let handshake = spawned.handshake().clone();
    let endpoint = LocalEndpoint {
        address: handshake.local.clone(),
        session_id: handshake.session_id.clone(),
        instance_id: instance_id.to_string(),
        target_port: handshake.target_port,
    };
    // A concurrent join may have won the race; keep the live sidecar and
    // let this one shut down so the domain sees a single session.
    let mut spawned = spawned;
    if let Some(existing) = active_endpoint(instance_id) {
        spawned.stop().await;
        return Ok(existing);
    }
    registry().insert(instance_id.to_string(), spawned);
    Ok(endpoint)
}

/// Endpoint of a live sidecar, dropping entries whose process died.
fn active_endpoint(instance_id: &str) -> Option<LocalEndpoint> {
    let mut registry = registry();
    if let Some(sidecar) = registry.get_mut(instance_id) {
        if sidecar.is_running() {
            let handshake = sidecar.handshake();
            return Some(LocalEndpoint {
                address: handshake.local.clone(),
                session_id: handshake.session_id.clone(),
                instance_id: instance_id.to_string(),
                target_port: handshake.target_port,
            });
        }
    }
    registry.remove(instance_id);
    None
}

/// Stops the sidecar of one instance (best effort; called when the game
/// exits). Its stdin closes, the sidecar closes the domain session and the
/// node entry is reclaimed.
pub async fn release_for_instance(instance_id: &str) {
    let sidecar = registry().remove(instance_id);
    let Some(mut sidecar) = sidecar else {
        return;
    };
    sidecar.stop().await;
}

/// Stops every sidecar; used when the launcher shuts down.
pub async fn release_all() {
    let sidecars: Vec<sidecar::Sidecar> =
        registry().drain().map(|(_, sidecar)| sidecar).collect();
    for mut sidecar in sidecars {
        sidecar.stop().await;
    }
}

/// Diagnostics for the join flow and support: is the face usable, is a
/// sidecar up, and on which loopback address.
pub async fn status_for_instance(instance_id: &str) -> P2pStatus {
    let (enabled, reason) = match enabled().await {
        Ok(true) => (true, None),
        Ok(false) => (
            false,
            Some(
                "This domain does not offer the direct-connect entry"
                    .to_string(),
            ),
        ),
        Err(error) => (false, Some(error.to_string())),
    };
    let endpoint = active_endpoint(instance_id);
    P2pStatus {
        enabled,
        active: endpoint.is_some(),
        address: endpoint.as_ref().map(|endpoint| endpoint.address.clone()),
        session_id: endpoint
            .as_ref()
            .and_then(|endpoint| endpoint.session_id.clone()),
        target_port: endpoint.and_then(|endpoint| endpoint.target_port),
        reason,
    }
}

/// Endpoint the join flow should use for a server, or `None` when the
/// direct face is unavailable — callers then keep the server's public
/// address, so a missing helper or an offline rendezvous never blocks a
/// join.
pub async fn launch_endpoint(instance_id: &str) -> Option<LocalEndpoint> {
    match ensure_local_endpoint(instance_id).await {
        Ok(endpoint) => Some(endpoint),
        Err(error) => {
            tracing::warn!(
                instance = %instance_id,
                %error,
                "Direct-connect entry unavailable; falling back to the public address"
            );
            None
        }
    }
}

fn registry()
-> std::sync::MutexGuard<'static, HashMap<String, sidecar::Sidecar>> {
    SIDECARS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities(entries: &[&str], node: Option<&str>) -> YmclCapabilities {
        YmclCapabilities {
            capabilities: entries
                .iter()
                .map(|entry| entry.to_string())
                .collect(),
            p2p: node.map(|session_url| P2pEndpointConfig {
                session_url: Some(session_url.to_string()),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn capability_gate_follows_both_declarations() {
        assert!(face_declared(&capabilities(&["p2p"], None)));
        assert!(face_declared(&capabilities(
            &[],
            Some("https://panel.example.com/x")
        )));
        assert!(!face_declared(&capabilities(&["mip", "skins"], None)));
        assert!(!face_declared(&capabilities(&[], None)));
    }

    #[test]
    fn session_url_prefers_the_advertised_face() {
        let advertised = capabilities(
            &["p2p"],
            Some(
                "https://panel.example.com/api/plugins/ymcl-adapter/v1/p2p/session",
            ),
        );
        assert_eq!(
            session_url("https://other.example.com", &advertised),
            "https://panel.example.com/api/plugins/ymcl-adapter/v1/p2p/session"
        );
        // No override (or an empty one): derive from the origin, tolerating
        // a stored origin with a trailing slash.
        assert_eq!(
            session_url(
                "https://panel.example.com/",
                &capabilities(&["p2p"], None)
            ),
            "https://panel.example.com/api/plugins/ymcl-adapter/v1/p2p/session"
        );
        assert_eq!(
            session_url(
                "https://panel.example.com",
                &capabilities(&["p2p"], Some("  "))
            ),
            "https://panel.example.com/api/plugins/ymcl-adapter/v1/p2p/session"
        );
    }

    #[test]
    fn parses_session_payload_from_the_adapter() {
        let payload = r#"{
            "session_id": "sess-1",
            "instance_id": "inst-1",
            "node_id": "node-1",
            "state": "direct",
            "terminal": false,
            "ticket": "ticket-1",
            "target_port": 25600,
            "rate_kbps": 2048,
            "expires_at": 1760000000000,
            "candidates": [
                { "proto": "tcp", "host": "203.0.113.7", "port": 25601, "priority": 10 }
            ],
            "reason": null
        }"#;
        let session: P2pSession =
            serde_json::from_str(payload).expect("session payload must parse");
        assert_eq!(session.state, "direct");
        assert!(!session.terminal);
        assert_eq!(session.target_port, Some(25600));
        assert_eq!(session.candidates.len(), 1);
        assert_eq!(session.candidates[0].host, "203.0.113.7");
        // The ticket is only ever present right after opening.
        assert_eq!(session.ticket.as_deref(), Some("ticket-1"));
    }

    #[test]
    fn parses_terminal_session_without_ticket() {
        let payload = r#"{
            "session_id": "sess-1",
            "state": "expired",
            "terminal": true,
            "reason": "票据过期"
        }"#;
        let session: P2pSession =
            serde_json::from_str(payload).expect("terminal payload must parse");
        assert!(session.terminal);
        assert!(session.ticket.is_none());
        assert!(session.candidates.is_empty());
        assert_eq!(session.reason.as_deref(), Some("票据过期"));
    }

    #[test]
    fn encodes_query_values() {
        assert_eq!(encode_query("sess-1"), "sess-1");
        assert_eq!(encode_query("a b&c=d"), "a%20b%26c%3Dd");
        assert_eq!(encode_query("票据"), "%E7%A5%A8%E6%8D%AE");
    }
}

//! YAP HTTP calls against a domain's adapter endpoints.

use reqwest::Method;

use super::manifest::{
    YmclCapabilities, YmclManifest, capabilities_url, manifest_url,
};
use crate::State;
use crate::util::fetch::{FetchSemaphore, fetch_json};

pub async fn fetch_capabilities(
    origin: &str,
    semaphore: &FetchSemaphore,
    exec: &sqlx::SqlitePool,
) -> crate::Result<YmclCapabilities> {
    fetch_json::<YmclCapabilities>(
        Method::GET,
        &capabilities_url(origin),
        None,
        None,
        None,
        semaphore,
        exec,
    )
    .await
}

pub async fn fetch_manifest(
    origin: &str,
    semaphore: &FetchSemaphore,
    exec: &sqlx::SqlitePool,
) -> crate::Result<YmclManifest> {
    fetch_json::<YmclManifest>(
        Method::GET,
        &manifest_url(origin),
        None,
        None,
        None,
        semaphore,
        exec,
    )
    .await
}

/// Convenience wrapper resolving state once for both fetches.
pub async fn fetch_capabilities_with_state(
    origin: &str,
) -> crate::Result<YmclCapabilities> {
    let state = State::get().await?;
    fetch_capabilities(origin, &state.api_semaphore, &state.pool).await
}

pub async fn fetch_manifest_with_state(
    origin: &str,
) -> crate::Result<YmclManifest> {
    let state = State::get().await?;
    fetch_manifest(origin, &state.api_semaphore, &state.pool).await
}

#[cfg(test)]
mod adapter_contract_tests {
    /// Contract lock against the yudream-admin-plugins repo's
    /// `ymcl-adapter` plugin: this is the exact JSON shape
    /// `YmclCapabilitiesController.buildCapabilities` emits for
    /// `GET /api/plugins/ymcl-adapter/v1/capabilities` (0.1.0 skeleton:
    /// no `mip` node yet — the distribution face ships later and the
    /// launcher must degrade gracefully). If either side changes field
    /// names, this test fails.
    #[test]
    fn parses_ymcl_adapter_capabilities_payload() {
        let payload = r#"{
            "protocol_version": 1,
            "adapter_version": "0.1.0",
            "domain": {
                "name": "https://yda.example.com",
                "description": null,
                "logo_url": null
            },
            "capabilities": [],
            "auth": {
                "required": true,
                "methods": [
                    {
                        "type": "password",
                        "endpoint": "https://yda.example.com/api/user/login"
                    },
                    {
                        "type": "oauth-web",
                        "authorize_url": "https://yda.example.com/api/oauth/authorize",
                        "token_url": "https://yda.example.com/api/oauth/token",
                        "client_id": "ymcl",
                        "scopes": ["profile", "plugin:ymcl-adapter:view"],
                        "pkce": "S256"
                    }
                ]
            }
        }"#;
        let capabilities: super::super::manifest::YmclCapabilities =
            serde_json::from_str(payload)
                .expect("adapter payload must deserialize");
        assert_eq!(capabilities.protocol_version, 1);
        assert_eq!(capabilities.adapter_version.as_deref(), Some("0.1.0"));
        let domain = capabilities.domain.as_ref().expect("domain present");
        assert_eq!(domain.name, "https://yda.example.com");
        assert!(capabilities.capabilities.is_empty());
        assert!(capabilities.mip.is_none());
        let auth = capabilities.auth.as_ref().expect("auth present");
        assert!(auth.required);
        assert_eq!(auth.methods.len(), 2);
        assert_eq!(auth.methods[0].r#type, "password");
        assert_eq!(
            auth.methods[0].endpoint.as_deref(),
            Some("https://yda.example.com/api/user/login")
        );
        assert_eq!(auth.methods[1].r#type, "oauth-web");
        assert_eq!(auth.methods[1].client_id.as_deref(), Some("ymcl"));
    }

    /// Forward compatibility: a future adapter declaring the MIP
    /// distribution face (YAP §7) must deserialize with the node intact.
    #[test]
    fn parses_mip_enabled_capabilities_payload() {
        let payload = r#"{
            "protocol_version": 1,
            "domain": { "name": "YuDream" },
            "capabilities": ["mip"],
            "auth": { "required": true, "methods": [] },
            "mip": { "base_url": "https://yda.example.com/mip" }
        }"#;
        let capabilities: super::super::manifest::YmclCapabilities =
            serde_json::from_str(payload)
                .expect("mip payload must deserialize");
        assert_eq!(
            capabilities
                .mip
                .as_ref()
                .and_then(|mip| mip.base_url.as_deref()),
            Some("https://yda.example.com/mip")
        );
    }

    /// Minimal payload: an adapter without optional capabilities still
    /// deserializes (YAP §6.1 forward compatibility).
    #[test]
    fn parses_minimal_capabilities_payload() {
        let payload = r#"{
            "protocol_version": 1,
            "domain": { "name": "YuDream" },
            "capabilities": [],
            "auth": { "required": false, "methods": [] }
        }"#;
        let capabilities: super::super::manifest::YmclCapabilities =
            serde_json::from_str(payload)
                .expect("minimal payload must deserialize");
        assert!(capabilities.capabilities.is_empty());
        assert!(capabilities.mip.is_none());
        assert!(!capabilities.auth.as_ref().expect("auth").required);
    }

    /// Contract lock for `GET /v1/session` (yudream-admin-plugins
    /// `YmclSessionController`): snake_case SessionInfo with a context
    /// node whose current selections may be null while options are listed.
    #[test]
    fn parses_ymcl_session_payload() {
        use crate::state::ymcl_session::YmclSessionInfo;
        let payload = r#"{
            "user_id": "1",
            "username": "admin",
            "nickname": "Admin",
            "avatar": null,
            "permissions": ["plugin:ymcl-adapter:view", "plugin:minecraft-server:view"],
            "issued_via": null,
            "context": {
                "dept": null,
                "role": null,
                "available_depts": [{ "id": "1", "name": "运营部" }],
                "available_roles": [{ "id": "2", "name": "服主" }]
            }
        }"#;
        let session: YmclSessionInfo = serde_json::from_str(payload)
            .expect("session payload must deserialize");
        assert_eq!(session.username, "admin");
        assert_eq!(session.permissions.len(), 2);
        let context = session.context.as_ref().expect("context present");
        assert_eq!(context.available_depts.len(), 1);
        assert_eq!(context.available_roles[0].name, "服主");
        // Context-less users omit the node entirely.
        let bare: YmclSessionInfo = serde_json::from_str(
            r#"{"user_id":"1","username":"u","permissions":[]}"#,
        )
        .expect("bare session must deserialize");
        assert!(bare.context.is_none());
    }

    /// Contract lock for `GET /v1/manifest` (yudream-admin-plugins
    /// `YmclManifestController`): navigation tree with native items, empty
    /// page registry, and the action whitelist.
    #[test]
    fn parses_ymcl_manifest_payload() {
        use super::super::manifest::YmclManifest;
        let payload = r#"{
            "protocol_version": 1,
            "navigation": [
                { "id": "home", "type": "native", "route": "/", "title": "首页",
                  "icon": "home", "sort": 0, "required_permission": null },
                { "id": "skins", "type": "native", "route": "/skins", "title": "皮肤",
                  "icon": "skins", "sort": 30, "required_permission": null }
            ],
            "pages": [],
            "data_sources": [],
            "actions": { "allow": ["client:launch-server", "client:reload", "server:*"] }
        }"#;
        let manifest: YmclManifest = serde_json::from_str(payload)
            .expect("manifest payload must deserialize");
        assert_eq!(manifest.protocol_version, 1);
        assert_eq!(manifest.navigation.len(), 2);
        assert_eq!(manifest.navigation[0].route.as_deref(), Some("/"));
        assert_eq!(manifest.navigation[1].title.as_deref(), Some("皮肤"));
        assert!(manifest.pages.is_empty());
    }
}

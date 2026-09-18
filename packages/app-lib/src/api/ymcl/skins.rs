//! Domain skin wardrobe (YAP §6.11, `skins` capability): closet listing,
//! one-shot equip (skin + cape), upload and removal against the adapter's
//! skins face, authenticated by the domain session. The launcher never
//! talks to the skin station directly — the adapter aggregates it.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::manifest::{SkinsEndpointConfig, YAP_API_BASE};
use crate::State;
use crate::util::fetch::fetch_advanced;

/// A Minecraft profile owned by the domain session's user.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclSkinProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub current: bool,
}

/// A skin entry in the wardrobe.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclClosetSkin {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// `CLASSIC | SLIM | UNKNOWN` (YAP §6.11); UNKNOWN lets the launcher
    /// detect the arm style from the texture itself.
    #[serde(default)]
    pub variant: Option<String>,
    /// URL of the skin PNG (adapter or skin station served).
    pub url: String,
    #[serde(default)]
    pub hash: Option<String>,
}

/// A cape entry in the wardrobe.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclClosetCape {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub url: String,
}

/// What a profile currently wears. `skin` is null when the profile has no
/// custom skin; `cape_id` is null when no cape is worn.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct YmclEquippedState {
    #[serde(default)]
    pub skin: Option<YmclClosetSkin>,
    #[serde(default)]
    pub cape_id: Option<String>,
}

/// The wardrobe of one profile: owned skins and capes plus what is worn.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclCloset {
    pub profile: YmclSkinProfile,
    #[serde(default)]
    pub equipped: Option<YmclEquippedState>,
    #[serde(default)]
    pub skins: Vec<YmclClosetSkin>,
    #[serde(default)]
    pub capes: Vec<YmclClosetCape>,
}

/// Result of an upload (`POST …/skins`) or collect: the new closet entry.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclSkinUploadResult {
    pub skin: YmclClosetSkin,
}

/// Parses an upload/collect response: protocol shape `{ "skin": { … } }`,
/// tolerating adapters that return the bare closet-skin object.
fn parse_skin_mutation(value: serde_json::Value) -> crate::Result<YmclSkinUploadResult> {
    let node = match value.get("skin") {
        Some(skin) if skin.is_object() => skin.clone(),
        // 容错：适配器直回裸皮肤对象。
        _ => value,
    };
    Ok(YmclSkinUploadResult {
        skin: serde_json::from_value(node)?,
    })
}

/// One entry of the public skin library (YAP §6.11 `library`).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclLibraryEntry {
    pub id: String,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    /// `skin | cape`.
    #[serde(default, alias = "type")]
    pub entry_type: Option<String>,
    #[serde(default)]
    pub variant: Option<String>,
    pub url: String,
}

/// A page of the public skin library.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclLibraryPage {
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub has_more: bool,
    #[serde(default)]
    pub items: Vec<YmclLibraryEntry>,
}

pub fn default_skins_base_url(origin: &str) -> String {
    format!("{origin}{YAP_API_BASE}/skins")
}

/// Resolves the skins base URL of a domain: the adapter's advertised
/// override when present, otherwise the default adapter face.
pub async fn skins_base_url(domain_id: &str) -> crate::Result<String> {
    let capabilities = super::registry::domain_capabilities(domain_id).await?;
    let origin = super::registry::domain_origin(domain_id).await?;
    let base = capabilities
        .skins
        .as_ref()
        .and_then(|skins: &SkinsEndpointConfig| skins.base_url.clone())
        .unwrap_or_else(|| default_skins_base_url(&origin));
    Ok(base.trim_end_matches('/').to_string())
}

/// True when the domain declares the `skins` capability (YAP §6.11).
pub async fn skins_enabled(domain_id: &str) -> crate::Result<bool> {
    let capabilities = super::registry::domain_capabilities(domain_id).await?;
    Ok(capabilities.capabilities.iter().any(|cap| cap == "skins")
        || capabilities.skins.is_some())
}

/// Finds an added domain whose Yggdrasil root matches a Minecraft account's
/// `api_root`, so the skins page can tell which domain (if any) owns the
/// active account. Both provider shapes are accepted, since the account was
/// stored with whichever one the node served at login time.
pub async fn domain_for_yggdrasil_root(
    api_root: &str,
) -> crate::Result<Option<YmclSkinDomainMatch>> {
    let normalized = api_root.trim_end_matches('/');
    let domains = super::registry::domains_state().await?;
    for domain in domains.domains {
        let Some(origin) = domain.origin.as_deref() else {
            continue;
        };
        let matches = super::yggroot::YmclYggEndpoints::candidates(origin)
            .iter()
            .any(|endpoints| {
                endpoints.yggdrasil_root.trim_end_matches('/') == normalized
            });
        if matches {
            let skins_enabled = skins_enabled(&domain.id).await?;
            return Ok(Some(YmclSkinDomainMatch {
                domain_id: domain.id,
                domain_name: domain.display_name,
                skins_enabled,
            }));
        }
    }
    Ok(None)
}

/// A domain matched from an account's Yggdrasil api_root.
#[derive(Serialize, Clone, Debug)]
pub struct YmclSkinDomainMatch {
    pub domain_id: String,
    pub domain_name: String,
    pub skins_enabled: bool,
}

/// Performs an authenticated skins-face request and decodes the JSON
/// response, surfacing YAP §6.2 error envelopes as readable errors.
async fn skins_request(
    domain_id: &str,
    method: Method,
    path: &str,
    json_body: Option<serde_json::Value>,
) -> crate::Result<serde_json::Value> {
    let base = skins_base_url(domain_id).await?;
    let state = State::get().await?;
    let url = format!("{base}{path}");
    let bytes = super::auth::domain_request(&state, domain_id, method, &url, json_body)
        .await
        .map_err(|error| {
            crate::ErrorKind::OtherError(format!(
                "Domain skin service request to {url} failed: {error}"
            ))
        })?;
    if bytes.is_empty() {
        return Ok(serde_json::Value::Null);
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Domain skin service response from {url} is not valid JSON: {error}"
        ))
    })?;
    if let Some(code) = value.get("error").and_then(|error| error.as_str()) {
        let message = value
            .get("message")
            .and_then(|message| message.as_str())
            .unwrap_or(code);
        return Err(crate::ErrorKind::OtherError(format!(
            "Domain skin service error: {message}"
        ))
        .into());
    }
    Ok(value)
}

/// Lists the Minecraft profiles the domain session's user owns.
pub async fn list_profiles(domain_id: &str) -> crate::Result<Vec<YmclSkinProfile>> {
    let value =
        skins_request(domain_id, Method::GET, "/profiles", None).await?;
    let profiles = match value.get("profiles") {
        Some(profiles) => profiles.clone(),
        // Tolerate adapters returning a bare array.
        None if value.is_array() => value,
        None => serde_json::Value::Array(Vec::new()),
    };
    Ok(serde_json::from_value(profiles)?)
}

/// Fetches a profile's wardrobe.
pub async fn fetch_closet(domain_id: &str, profile_id: &str) -> crate::Result<YmclCloset> {
    let value = skins_request(
        domain_id,
        Method::GET,
        &format!("/closet/{}", urlencoding::encode(profile_id)),
        None,
    )
    .await?;
    Ok(serde_json::from_value(value)?)
}

/// One-shot equip: applies a skin and a cape (either may be `None` to take
/// the item off) and returns the new equipped state (YAP §6.11).
pub async fn equip(
    domain_id: &str,
    profile_id: &str,
    skin_id: Option<&str>,
    cape_id: Option<&str>,
) -> crate::Result<YmclEquippedState> {
    let value = skins_request(
        domain_id,
        Method::POST,
        &format!(
            "/closet/{}/equip",
            urlencoding::encode(profile_id)
        ),
        Some(serde_json::json!({
            "skin_id": skin_id,
            "cape_id": cape_id,
        })),
    )
    .await?;
    let equipped = value
        .get("equipped")
        .cloned()
        .unwrap_or(value);
    Ok(serde_json::from_value(equipped)?)
}

/// Uploads a skin PNG into the wardrobe without equipping it. `data` is the
/// PNG as base64 (optionally a `data:image/png;base64,…` URL) — the transfer
/// channel is base64-in-JSON, matching the adapter upload constraint that
/// MIP publishing already works around (YAP §6.11 / §7 v1).
pub async fn upload_skin(
    domain_id: &str,
    profile_id: &str,
    name: Option<&str>,
    model: &str,
    filename: &str,
    data: &str,
) -> crate::Result<YmclSkinUploadResult> {
    use base64::Engine as _;
    let payload = data.rsplit("base64,").next().unwrap_or(data);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Invalid skin image payload: {error}"
            ))
        })?;
    let value = skins_request(
        domain_id,
        Method::POST,
        &format!(
            "/closet/{}/skins",
            urlencoding::encode(profile_id)
        ),
        Some(serde_json::json!({
            "filename": filename,
            "model": model,
            "name": name,
            "data": base64::engine::general_purpose::STANDARD.encode(bytes),
        })),
    )
    .await?;
    parse_skin_mutation(value)
}

/// Result of a cape upload (`POST …/capes`): the new closet cape entry.
/// Cape nodes share the closet-skin shape (id/name/url/hash) on the wire.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclCapeUploadResult {
    pub cape: YmclClosetSkin,
}

/// Uploads a cape PNG into the wardrobe without equipping it.
pub async fn upload_cape(
    domain_id: &str,
    profile_id: &str,
    name: Option<&str>,
    filename: &str,
    data: &str,
) -> crate::Result<YmclCapeUploadResult> {
    use base64::Engine as _;
    let payload = data.rsplit("base64,").next().unwrap_or(data);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Invalid cape image payload: {error}"
            ))
        })?;
    let value = skins_request(
        domain_id,
        Method::POST,
        &format!(
            "/closet/{}/capes",
            urlencoding::encode(profile_id)
        ),
        Some(serde_json::json!({
            "filename": filename,
            "name": name,
            "data": base64::engine::general_purpose::STANDARD.encode(bytes),
        })),
    )
    .await?;
    let node = match value.get("cape") {
        Some(cape) if cape.is_object() => cape.clone(),
        _ => value,
    };
    Ok(YmclCapeUploadResult {
        cape: serde_json::from_value(node)?,
    })
}

/// Creates a Minecraft character for the signed-in domain user.
/// Domain login is blocked in the launcher until at least one character exists.
pub async fn create_profile(
    domain_id: &str,
    name: &str,
) -> crate::Result<YmclSkinProfile> {
    let value = skins_request(
        domain_id,
        Method::POST,
        "/profiles",
        Some(serde_json::json!({ "name": name })),
    )
    .await?;
    let node = match value.get("profile") {
        Some(profile) if profile.is_object() => profile.clone(),
        _ => value,
    };
    Ok(serde_json::from_value(node)?)
}

/// Removes a skin from the wardrobe.
pub async fn delete_skin(
    domain_id: &str,
    profile_id: &str,
    skin_id: &str,
) -> crate::Result<()> {
    skins_request(
        domain_id,
        Method::DELETE,
        &format!(
            "/closet/{}/skins/{}",
            urlencoding::encode(profile_id),
            urlencoding::encode(skin_id),
        ),
        None,
    )
    .await?;
    Ok(())
}

/// Fetches a page of the public skin library (YAP §6.11).
pub async fn fetch_library(
    domain_id: &str,
    page: u32,
    limit: u32,
) -> crate::Result<YmclLibraryPage> {
    let value = skins_request(
        domain_id,
        Method::GET,
        &format!("/library?page={page}&limit={limit}"),
        None,
    )
    .await?;
    Ok(serde_json::from_value(value)?)
}

/// Collects a public-library texture into the wardrobe (idempotent).
pub async fn collect(
    domain_id: &str,
    profile_id: &str,
    hash: &str,
    name: Option<&str>,
) -> crate::Result<YmclSkinUploadResult> {
    let value = skins_request(
        domain_id,
        Method::POST,
        &format!(
            "/closet/{}/collect",
            urlencoding::encode(profile_id)
        ),
        Some(serde_json::json!({
            "hash": hash,
            "name": name,
        })),
    )
    .await?;
    parse_skin_mutation(value)
}

/// Downloads an arbitrary texture URL (cape PNGs) and returns it as a
/// `data:image/png;base64,…` URL. Unlike `normalize_skin_texture` this
/// performs no skin-format conversion, so cape textures survive untouched.
pub async fn fetch_texture_data_url(url: &str) -> crate::Result<String> {
    use base64::Engine as _;
    let state = State::get().await?;
    let bytes = fetch_advanced(
        Method::GET,
        url,
        None,
        None,
        None,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await
    .map_err(|error| {
        crate::ErrorKind::OtherError(format!("Failed to download texture from {url}: {error}"))
    })?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[cfg(test)]
mod adapter_contract_tests {
    use super::*;

    /// Contract lock for `GET {base}/closet/{profileId}` (YAP §6.11):
    /// profile + equipped node (skin/cape_id nullable) + skins/capes lists.
    #[test]
    fn parses_closet_payload() {
        let payload = r#"{
            "profile": { "id": "d3b07384d9", "name": "Steve", "current": true },
            "equipped": {
                "skin": { "id": "s1", "name": "夜行", "variant": "SLIM",
                          "url": "https://d.example/s1.png", "hash": "abc" },
                "cape_id": "c1"
            },
            "skins": [
                { "id": "s1", "name": "夜行", "variant": "SLIM",
                  "url": "https://d.example/s1.png" },
                { "id": "s2", "variant": "UNKNOWN", "url": "https://d.example/s2.png" }
            ],
            "capes": [ { "id": "c1", "name": "域庆披风", "url": "https://d.example/c1.png" } ]
        }"#;
        let closet: YmclCloset =
            serde_json::from_str(payload).expect("closet payload must deserialize");
        assert_eq!(closet.profile.name, "Steve");
        let equipped = closet.equipped.as_ref().expect("equipped present");
        assert_eq!(equipped.skin.as_ref().expect("skin worn").variant.as_deref(), Some("SLIM"));
        assert_eq!(equipped.cape_id.as_deref(), Some("c1"));
        assert_eq!(closet.skins.len(), 2);
        assert!(closet.skins[1].name.is_none());
        assert_eq!(closet.capes.len(), 1);
    }

    /// Bare-closet adapter response (no custom skin, no cape) must parse.
    #[test]
    fn parses_bare_closet_payload() {
        let payload = r#"{
            "profile": { "id": "d3b07384d9", "name": "Alex" },
            "equipped": { "skin": null, "cape_id": null },
            "skins": [],
            "capes": []
        }"#;
        let closet: YmclCloset =
            serde_json::from_str(payload).expect("bare closet must deserialize");
        let equipped = closet.equipped.as_ref().expect("equipped present");
        assert!(equipped.skin.is_none());
        assert!(equipped.cape_id.is_none());
        assert!(closet.skins.is_empty());
        assert!(closet.capes.is_empty());
    }

    /// Contract lock for `GET {base}/profiles`.
    #[test]
    fn parses_profiles_payload() {
        let payload = r#"{ "profiles": [
            { "id": "p1", "name": "Steve", "current": true },
            { "id": "p2", "name": "Stella" }
        ] }"#;
        let value: serde_json::Value = serde_json::from_str(payload).unwrap();
        let profiles: Vec<YmclSkinProfile> =
            serde_json::from_value(value.get("profiles").cloned().unwrap()).unwrap();
        assert_eq!(profiles.len(), 2);
        assert!(profiles[0].current);
        assert!(!profiles[1].current);
    }

    /// Equip response shape: `{ "equipped": { … } }`.
    #[test]
    fn parses_equip_payload() {
        let payload = r#"{
            "equipped": {
                "skin": { "id": "s1", "variant": "CLASSIC", "url": "https://d.example/s1.png" },
                "cape_id": null
            }
        }"#;
        let value: serde_json::Value = serde_json::from_str(payload).unwrap();
        let equipped: YmclEquippedState =
            serde_json::from_value(value.get("equipped").cloned().unwrap()).unwrap();
        assert_eq!(equipped.skin.as_ref().expect("skin").id, "s1");
        assert!(equipped.cape_id.is_none());
    }

    /// Upload/collect response shape: `{ "skin": { … } }` — the wrapper must
    /// not be re-parsed from the already-extracted inner node.
    #[test]
    fn parses_skin_mutation_payload() {
        let wrapped = r#"{
            "skin": { "id": "s9", "variant": "CLASSIC", "url": "https://d.example/s9.png" }
        }"#;
        let result: YmclSkinUploadResult = parse_skin_mutation(
            serde_json::from_str(wrapped).expect("wrapped payload must be valid JSON"),
        )
        .expect("wrapped skin payload must deserialize");
        assert_eq!(result.skin.id, "s9");

        // 容错：适配器直回裸皮肤对象。
        let bare = r#"{ "id": "s9", "variant": "CLASSIC", "url": "https://d.example/s9.png" }"#;
        let result: YmclSkinUploadResult = parse_skin_mutation(
            serde_json::from_str(bare).expect("bare payload must be valid JSON"),
        )
        .expect("bare skin payload must deserialize");
        assert_eq!(result.skin.id, "s9");
    }

    /// Contract lock for `GET {base}/library` (YAP §6.11 extension):
    /// paginated public skin library with skin/cape typed entries.
    #[test]
    fn parses_library_payload() {
        let payload = r#"{
            "page": 2,
            "has_more": true,
            "items": [
                { "id": "abc", "hash": "abc", "name": "夜行", "type": "skin",
                  "variant": "SLIM", "url": "https://d.example/abc.png" },
                { "id": "def", "hash": "def", "name": "域庆披风", "type": "cape",
                  "variant": "UNKNOWN", "url": "https://d.example/def.png" }
            ]
        }"#;
        let page: YmclLibraryPage =
            serde_json::from_str(payload).expect("library payload must deserialize");
        assert!(page.has_more);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].entry_type.as_deref(), Some("skin"));
        assert_eq!(page.items[1].entry_type.as_deref(), Some("cape"));
        assert_eq!(page.items[0].variant.as_deref(), Some("SLIM"));
    }

    /// YAP §6.2 error envelope must surface as a readable message.
    #[test]
    fn error_envelope_shape() {
        let payload = r#"{ "error": "forbidden", "message": "不是该档案的拥有者" }"#;
        let value: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert_eq!(
            value.get("error").and_then(|error| error.as_str()),
            Some("forbidden")
        );
        assert_eq!(
            value.get("message").and_then(|message| message.as_str()),
            Some("不是该档案的拥有者")
        );
    }
}

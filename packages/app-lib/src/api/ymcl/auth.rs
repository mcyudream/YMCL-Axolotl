//! YAP authentication flows (YAP §6.4): password, OAuth (PKCE + loopback,
//! orchestrated by the GUI layer), external login polling, session
//! normalization, refresh and department/role context switching.
//!
//! The adapter accepts both Sa-Token and OAuth tokens on its endpoints
//! (YAP §6.2), so the `Authorization` header carries the bare token.

use base64::Engine;
use rand::RngCore;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::client::fetch_capabilities;
use super::manifest::YAP_API_BASE;
use crate::State;
use crate::state::ymcl_session::{self, YmclSessionInfo, YmclStoredSession};
use crate::util::fetch::fetch_advanced;

pub fn session_url(origin: &str) -> String {
    format!("{origin}{YAP_API_BASE}/session")
}

pub fn external_providers_url(origin: &str) -> String {
    format!("{origin}{YAP_API_BASE}/auth/external/providers")
}

pub fn external_begin_url(origin: &str, provider: &str) -> String {
    format!("{origin}{YAP_API_BASE}/auth/external/{provider}/begin")
}

pub fn external_poll_url(
    origin: &str,
    provider: &str,
    flow_id: &str,
) -> String {
    format!(
        "{origin}{YAP_API_BASE}/auth/external/{provider}/poll?flowId={flow_id}"
    )
}

pub fn context_switch_url(origin: &str) -> String {
    format!("{origin}{YAP_API_BASE}/context/switch")
}

/// PKCE material for the OAuth authorization-code flow (S256).
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

pub fn generate_pkce() -> PkceChallenge {
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    let verifier =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    PkceChallenge {
        verifier,
        challenge,
    }
}

/// Random nonce for the OAuth `state` parameter (CSRF protection).
pub fn generate_state_nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn authorize_url(
    authorize_endpoint: &str,
    client_id: &str,
    redirect_uri: &str,
    challenge: &PkceChallenge,
    state: &str,
) -> String {
    let mut url = url::Url::parse(authorize_endpoint)
        .expect("adapter capabilities carry valid authorize endpoints");
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("code_challenge", &challenge.challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state);
    url.to_string()
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    #[serde(alias = "token")]
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

async fn request_tokens(
    url: &str,
    form_body: String,
) -> crate::Result<TokenResponse> {
    // OAuth token endpoints are form-urlencoded by spec (YAP §6.4); the shared
    // fetch helper only speaks JSON bodies, so this uses a one-off client.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let response = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        let message = response.text().await.unwrap_or_default();
        return Err(crate::ErrorKind::OtherError(format!(
            "Token exchange failed ({status}): {message}"
        ))
        .into());
    }
    Ok(response.json::<TokenResponse>().await?)
}

async fn fetch_session_info(
    state: &State,
    origin: &str,
    access_token: &str,
) -> crate::Result<YmclSessionInfo> {
    let bytes = fetch_advanced(
        Method::GET,
        &session_url(origin),
        None,
        None,
        Some(("Authorization", access_token)),
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    Ok(serde_json::from_slice::<YmclSessionInfo>(&bytes)?)
}

async fn normalize_and_store(
    domain_id: &str,
    origin: &str,
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
) -> crate::Result<YmclStoredSession> {
    let state = State::get().await?;
    let session = fetch_session_info(&state, origin, &access_token).await?;
    let stored = YmclStoredSession {
        domain_id: domain_id.to_string(),
        access_token,
        refresh_token,
        expires_at: expires_in
            .map(|seconds| chrono::Utc::now().timestamp() + seconds.max(0)),
        session,
        updated_at: chrono::Utc::now().timestamp(),
    };
    ymcl_session::upsert(&stored, &state.pool).await?;
    Ok(stored)
}

fn password_endpoint(
    capabilities: &super::manifest::YmclCapabilities,
) -> crate::Result<String> {
    let auth = capabilities.auth.as_ref().ok_or_else(|| {
        crate::ErrorKind::OtherError(
            "This domain does not offer login".to_string(),
        )
    })?;
    match auth
        .methods
        .iter()
        .find(|method| method.r#type == "password")
    {
        Some(method) => method.endpoint.clone().ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "Password login method is missing its endpoint".to_string(),
            )
            .into()
        }),
        None => Err(crate::ErrorKind::OtherError(
            "This domain does not allow username and password login"
                .to_string(),
        )
        .into()),
    }
}

#[derive(Deserialize)]
struct YdaLoginResponse {
    #[serde(default)]
    token: Option<String>,
    #[serde(default, alias = "accessToken")]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default, alias = "expiresIn")]
    expires_in: Option<i64>,
}

/// Password login against the domain's configured host login endpoint
/// (yda `/api/user/login` semantics, tolerant of both token field names).
pub async fn password_login(
    domain_id: &str,
    username: &str,
    password: &str,
) -> crate::Result<YmclStoredSession> {
    let state = State::get().await?;
    let origin = super::registry::domain_origin(domain_id).await?;
    let capabilities =
        fetch_capabilities(&origin, &state.api_semaphore, &state.pool).await?;
    let endpoint = password_endpoint(&capabilities)?;

    let body =
        serde_json::json!({ "username": username, "password": password });
    let bytes = fetch_advanced(
        Method::POST,
        &endpoint,
        None,
        Some(body),
        None,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    let login: YdaLoginResponse = serde_json::from_slice(&bytes)?;
    let access_token = login.access_token.or(login.token).ok_or_else(|| {
        crate::ErrorKind::OtherError(
            "Login response did not contain a token".to_string(),
        )
    })?;
    normalize_and_store(
        domain_id,
        &origin,
        access_token,
        login.refresh_token,
        login.expires_in,
    )
    .await
}

/// Exchanges an OAuth authorization code for tokens and normalizes the
/// session. `code_verifier` must match the PKCE challenge sent in the
/// authorize request.
pub async fn oauth_finish(
    domain_id: &str,
    token_url: &str,
    client_id: &str,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> crate::Result<YmclStoredSession> {
    let origin = super::registry::domain_origin(domain_id).await?;
    let form = form_urlencoded(&[
        ("grant_type", "authorization_code"),
        ("client_id", client_id),
        ("code", code),
        ("code_verifier", code_verifier),
        ("redirect_uri", redirect_uri),
    ]);
    let tokens = request_tokens(token_url, form).await?;
    normalize_and_store(
        domain_id,
        &origin,
        tokens.access_token,
        tokens.refresh_token,
        tokens.expires_in,
    )
    .await
}

fn form_urlencoded(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                urlencoding::encode(key),
                urlencoding::encode(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YmclExternalProvider {
    pub code: String,
    #[serde(default)]
    pub name: Option<String>,
}

pub async fn external_providers(
    domain_id: &str,
) -> crate::Result<Vec<YmclExternalProvider>> {
    let state = State::get().await?;
    let origin = super::registry::domain_origin(domain_id).await?;
    let bytes = fetch_advanced(
        Method::GET,
        &external_providers_url(&origin),
        None,
        None,
        None,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    #[derive(Deserialize)]
    struct Providers {
        #[serde(default)]
        providers: Vec<YmclExternalProvider>,
    }
    Ok(serde_json::from_slice::<Providers>(&bytes)?.providers)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YmclExternalFlow {
    pub flow_id: String,
    pub authorize_url: String,
}

pub async fn external_begin(
    domain_id: &str,
    provider: &str,
) -> crate::Result<YmclExternalFlow> {
    let state = State::get().await?;
    let origin = super::registry::domain_origin(domain_id).await?;
    let bytes = fetch_advanced(
        Method::GET,
        &external_begin_url(&origin, provider),
        None,
        None,
        None,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    Ok(serde_json::from_slice::<YmclExternalFlow>(&bytes)?)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YmclExternalPoll {
    pub status: String,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<i64>,
}

pub async fn external_poll(
    domain_id: &str,
    provider: &str,
    flow_id: &str,
) -> crate::Result<YmclExternalPoll> {
    let state = State::get().await?;
    let origin = super::registry::domain_origin(domain_id).await?;
    let bytes = fetch_advanced(
        Method::GET,
        &external_poll_url(&origin, provider, flow_id),
        None,
        None,
        None,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    Ok(serde_json::from_slice::<YmclExternalPoll>(&bytes)?)
}

/// Completes an external login once polling reported `completed`.
pub async fn external_finish(
    domain_id: &str,
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
) -> crate::Result<YmclStoredSession> {
    let origin = super::registry::domain_origin(domain_id).await?;
    normalize_and_store(
        domain_id,
        &origin,
        access_token,
        refresh_token,
        expires_in,
    )
    .await
}

pub async fn get_session(
    domain_id: &str,
) -> crate::Result<Option<YmclStoredSession>> {
    let state = State::get().await?;
    ymcl_session::get(domain_id, &state.pool).await
}

pub async fn logout(domain_id: &str) -> crate::Result<()> {
    let state = State::get().await?;
    ymcl_session::remove(domain_id, &state.pool).await
}

#[derive(Deserialize, Debug)]
struct RefreshResponse {
    #[serde(default, alias = "token")]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default, alias = "expiresIn")]
    expires_in: Option<i64>,
}

/// Refreshes the stored tokens. OAuth refresh goes through the adapter's
/// token URL; password sessions fall back to the host's
/// `/api/user/token/refresh`. Returns `None` when no refresh token exists.
pub async fn refresh_session(
    domain_id: &str,
) -> crate::Result<Option<YmclStoredSession>> {
    let state = State::get().await?;
    let Some(stored) = ymcl_session::get(domain_id, &state.pool).await? else {
        return Ok(None);
    };
    let refresh_token = match stored.refresh_token.clone() {
        Some(token) => token,
        None => return Ok(None),
    };
    let origin = super::registry::domain_origin(domain_id).await?;

    let capabilities =
        fetch_capabilities(&origin, &state.api_semaphore, &state.pool).await?;
    let oauth_token_url = capabilities.auth.as_ref().and_then(|auth| {
        auth.methods
            .iter()
            .find(|method| method.r#type == "oauth-web")
            .and_then(|method| method.token_url.clone())
    });

    let refreshed = if let Some(token_url) = oauth_token_url {
        let client_id = capabilities
            .auth
            .as_ref()
            .and_then(|auth| {
                auth.methods
                    .iter()
                    .find(|method| method.r#type == "oauth-web")
                    .and_then(|method| method.client_id.clone())
            })
            .unwrap_or_else(|| "ymcl".to_string());
        let form = form_urlencoded(&[
            ("grant_type", "refresh_token"),
            ("client_id", &client_id),
            ("refresh_token", &refresh_token),
        ]);
        request_tokens(&token_url, form).await?
    } else {
        let endpoint = format!("{origin}/api/user/token/refresh");
        let body = serde_json::json!({ "refreshToken": refresh_token });
        let bytes = fetch_advanced(
            Method::POST,
            &endpoint,
            None,
            Some(body),
            None,
            None,
            None,
            None,
            &state.api_semaphore,
            &state.pool,
        )
        .await?;
        let response: RefreshResponse = serde_json::from_slice(&bytes)?;
        TokenResponse {
            access_token: response.access_token.ok_or_else(|| {
                crate::ErrorKind::OtherError(
                    "Refresh response did not contain a token".to_string(),
                )
            })?,
            refresh_token: response.refresh_token,
            expires_in: response.expires_in,
        }
    };

    normalize_and_store(
        domain_id,
        &origin,
        refreshed.access_token,
        refreshed.refresh_token.or(Some(refresh_token)),
        refreshed.expires_in,
    )
    .await
    .map(Some)
}

/// Switches the department or role context of the session (YAP §6.4,
/// `context` capability) and stores the re-issued session info.
pub async fn context_switch(
    domain_id: &str,
    dept_id: Option<&str>,
    role_id: Option<&str>,
) -> crate::Result<YmclSessionInfo> {
    let state = State::get().await?;
    let Some(stored) = ymcl_session::get(domain_id, &state.pool).await? else {
        return Err(crate::ErrorKind::OtherError(
            "Not logged in to this domain".to_string(),
        )
        .into());
    };
    let origin = super::registry::domain_origin(domain_id).await?;
    let body = if let Some(dept_id) = dept_id {
        serde_json::json!({ "deptId": dept_id })
    } else if let Some(role_id) = role_id {
        serde_json::json!({ "roleId": role_id })
    } else {
        return Err(crate::ErrorKind::OtherError(
            "Context switch requires a department or role".to_string(),
        )
        .into());
    };
    let bytes = fetch_advanced(
        Method::POST,
        &context_switch_url(&origin),
        None,
        Some(body),
        Some(("Authorization", stored.access_token.as_str())),
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    let session = serde_json::from_slice::<YmclSessionInfo>(&bytes)?;
    ymcl_session::update_session_info(domain_id, &session, &state.pool).await?;
    Ok(session)
}

/// Ensures a usable session for the domain: refreshes expired tokens
/// up-front. Sessions without refresh capability are surfaced as-is so the
/// UI can prompt a re-login.
pub async fn ensure_session(
    domain_id: &str,
) -> crate::Result<Option<YmclStoredSession>> {
    let Some(session) = get_session(domain_id).await? else {
        return Ok(None);
    };
    if session.is_expired() && session.refresh_token.is_some() {
        return refresh_session(domain_id).await;
    }
    Ok(Some(session))
}

//! YAP authentication flows (YAP §6.4): password, OAuth (PKCE + loopback,
//! orchestrated by the GUI layer), external login polling, session
//! normalization, refresh and department/role context switching.
//!
//! The adapter accepts both Sa-Token and OAuth tokens on its endpoints
//! (YAP §6.2), so the `Authorization` header carries the bare token.

use std::sync::{Arc, LazyLock};
use std::time::Duration;

use base64::Engine;
use rand::RngCore;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use super::client::fetch_capabilities;
use super::manifest::YAP_API_BASE;
use crate::State;
use crate::event::LoadingBarId;
use crate::state::ymcl_session::{self, YmclSessionInfo, YmclStoredSession};
use crate::util::fetch::fetch_advanced;

/// Renew an access token this many seconds before its local expiry so the
/// next domain call never races the host's 2h lifetime.
const REFRESH_SKEW_SECS: i64 = 15 * 60;
/// Fallback lifetime when a host refresh response omits `expires_in`.
const DEFAULT_TOKEN_LIFETIME_SECS: i64 = 7200;
/// Background keeper cadence: enough to slide 2h tokens indefinitely while
/// the launcher stays open, without hammering the host.
const SESSION_KEEPER_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// One refresh at a time per domain so concurrent requests cannot race a
/// rotated refresh token or stampede the host refresh endpoint.
static REFRESH_LOCKS: LazyLock<dashmap::DashMap<String, Arc<Mutex<()>>>> =
    LazyLock::new(dashmap::DashMap::new);

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

/// Adapters self-report absolute endpoint URLs against their own host, but
/// the joined origin is the only address the launcher (and especially the
/// browser) may use: site-origin routing (reverse proxy in production, dev
/// proxy in development) carries `/api` to the backend, and the browser's
/// domain session cookie lives on the site origin — navigating to the
/// adapter host directly both exposes the internal address and drops the
/// session. Every adapter-reported URL is rebased onto the joined origin,
/// keeping path and query.
pub fn rebase_to_origin(raw_url: &str, origin: &str) -> String {
    if let Some(path) = raw_url.strip_prefix('/') {
        // Adapter-reported relative path: resolve against the joined origin.
        return format!(
            "{}/{}",
            origin.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
    }
    let Ok(mut url) = url::Url::parse(raw_url) else {
        return raw_url.to_string();
    };
    let Ok(base) = url::Url::parse(origin) else {
        return raw_url.to_string();
    };
    if url.set_scheme(base.scheme()).is_err()
        || url.set_host(base.host_str()).is_err()
    {
        return raw_url.to_string();
    }
    let _ = url.set_port(base.port());
    url.to_string()
}

/// The browser-facing authorize entry. yda serves the interactive OAuth
/// page at the site route `/oauth/authorize` (web-session backed, redirects
/// to login when unauthenticated); its raw `/api/oauth/authorize` endpoint
/// only answers JSON and 401s without a session. Adapters before 0.2.1
/// report the API path — upgrade it to the page path here.
pub fn browser_authorize_url(endpoint: &str, origin: &str) -> String {
    let rebased = rebase_to_origin(endpoint, origin);
    let Ok(mut url) = url::Url::parse(&rebased) else {
        return rebased;
    };
    if url.path() == "/api/oauth/authorize" {
        url.set_path("/oauth/authorize");
    }
    url.to_string()
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

/// OAuth authorization-code exchange response (YAP §6.4). Its token only
/// bootstraps the session: the launcher immediately swaps it for the host's
/// login session, whose `refresh_token`/`expires_in` (not these — the OAuth
/// provider's credentials never renew a domain session) are what get stored.
#[derive(Deserialize, Debug)]
struct TokenResponse {
    #[serde(alias = "token", alias = "accessToken")]
    access_token: String,
    #[serde(default, alias = "refreshToken")]
    #[allow(dead_code)]
    refresh_token: Option<String>,
    #[serde(default, alias = "expiresIn", deserialize_with = "de_opt_i64_lenient")]
    #[allow(dead_code)]
    expires_in: Option<i64>,
}

/// Hosts may wrap token payloads in a `{code, message, data}` envelope (yda
/// business endpoints do); accept both the flat OAuth payload and the
/// enveloped form by preferring whichever object actually carries a token.
fn unwrap_token_payload(value: serde_json::Value) -> serde_json::Value {
    let carries_token = |value: &serde_json::Value| {
        value.get("access_token").is_some()
            || value.get("accessToken").is_some()
            || value.get("token").is_some()
    };
    if carries_token(&value) {
        return value;
    }
    if let Some(data) = value.get("data") {
        if carries_token(data) {
            return data.clone();
        }
    }
    value
}

/// Hosts occasionally serialize numeric token lifetimes as strings (yda
/// sends `expires_in: "7200"`); accept both numbers and numeric strings.
fn de_opt_i64_lenient<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Lenient {
        Number(i64),
        Text(String),
    }
    Ok(match Lenient::deserialize(deserializer) {
        Ok(Lenient::Number(value)) => Some(value),
        Ok(Lenient::Text(text)) => text.trim().parse::<i64>().ok(),
        Err(_) => None,
    })
}

/// Compact, truncated body for error messages (keeps diagnostics readable).
fn body_snippet(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(300).collect()
}

async fn request_tokens(
    url: &str,
    form_body: String,
) -> crate::Result<TokenResponse> {
    // OAuth token endpoints are form-urlencoded by spec (YAP §6.4); the shared
    // fetch helper only speaks JSON bodies, so this uses a one-off client.
    // Loopback origins bypass the system proxy, mirroring the fetch routes.
    let client = if crate::util::fetch::is_loopback_url(url) {
        reqwest::Client::builder().no_proxy()
    } else {
        reqwest::Client::builder()
    }
    .timeout(std::time::Duration::from_secs(30))
    .build()?;
    let response = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .send()
        .await?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| {
            crate::ErrorKind::OtherError(format!(
                "Token response from {url} could not be read: {error}"
            ))
        })?;
    if !status.is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Token exchange failed ({status}): {}",
            body_snippet(&text)
        ))
        .into());
    }
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Token response from {url} is not valid JSON: {error}; body: {}",
            body_snippet(&text)
        ))
    })?;
    let tokens: TokenResponse =
        serde_json::from_value(unwrap_token_payload(value)).map_err(|error| {
            crate::ErrorKind::OtherError(format!(
                "Token response from {url} has an unexpected shape: {error}; body: {}",
                body_snippet(&text)
            ))
        })?;
    Ok(tokens)
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
    // Silent renewal depends entirely on the host issuing a refresh token;
    // surface its absence so a no-renewal deployment is visible in the log.
    if refresh_token
        .as_deref()
        .is_none_or(|token| token.trim().is_empty())
    {
        tracing::warn!(
            "Domain {domain_id} login did not include a refresh token; the session will require interactive re-login after the access token expires (expires_in={expires_in:?})"
        );
    } else {
        tracing::debug!(
            "Storing domain session for {domain_id}: refresh_token={}, expires_in={:?}",
            true,
            expires_in
        );
    }
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

#[derive(Deserialize, Default)]
struct YdaLoginResponse {
    #[serde(default)]
    token: Option<String>,
    #[serde(default, alias = "accessToken")]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default, alias = "expiresIn", deserialize_with = "de_opt_i64_lenient")]
    expires_in: Option<i64>,
    /// yda business endpoints envelope the payload in `{code, data}`.
    #[serde(default)]
    data: Option<Box<YdaLoginResponse>>,
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
        fetch_capabilities(&origin, &state.api_semaphore, &state.pool)
            .await
            .map_err(|error| {
                crate::ErrorKind::OtherError(format!(
                    "Could not fetch login methods from {origin}: {error}"
                ))
            })?;
    let endpoint = rebase_to_origin(&password_endpoint(&capabilities)?, &origin);

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
    .await
    .map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Login request to {endpoint} failed: {error}"
        ))
    })?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Login response from {endpoint} is not valid JSON: {error}"
        ))
    })?;
    let login: YdaLoginResponse =
        serde_json::from_value(unwrap_token_payload(value)).map_err(|error| {
            crate::ErrorKind::OtherError(format!(
                "Login response from {endpoint} has an unexpected shape: {error}"
            ))
        })?;
    let (access_token, refresh_token, expires_in) = login_session_parts(&login)?;
    normalize_and_store(domain_id, &origin, access_token, refresh_token, expires_in)
        .await
}

/// Registers a new domain account via the host `/api/user/register` endpoint
/// (advertised in capabilities.auth.registration when the adapter provides it).
/// Older adapters without a registration block fall back to the host's
/// well-known path so already-joined domains keep working. Does not auto-login.
pub async fn register(
    domain_id: &str,
    username: &str,
    email: &str,
    password: &str,
    nickname: Option<&str>,
) -> crate::Result<()> {
    let state = State::get().await?;
    let origin = super::registry::domain_origin(domain_id).await?;
    let capabilities =
        fetch_capabilities(&origin, &state.api_semaphore, &state.pool)
            .await
            .map_err(|error| {
                crate::ErrorKind::OtherError(format!(
                    "Could not fetch domain capabilities from {origin}: {error}"
                ))
            })?;
    let registration = capabilities
        .auth
        .as_ref()
        .and_then(|auth| auth.registration.as_ref());
    // Explicit disable is the only hard stop; a missing block falls back to
    // the host register endpoint (same as the web login page).
    if let Some(registration) = registration {
        if registration.enabled == Some(false) {
            return Err(crate::ErrorKind::OtherError(
                "该域未开放账号注册".to_string(),
            )
            .into());
        }
    }
    let endpoint = rebase_to_origin(
        registration
            .and_then(|registration| registration.endpoint.as_deref())
            .unwrap_or("/api/user/register"),
        &origin,
    );
    let mut body = serde_json::json!({
        "username": username,
        "email": email,
        "password": password,
    });
    if let Some(nickname) = nickname.filter(|value| !value.trim().is_empty()) {
        body["nickname"] = serde_json::Value::String(nickname.trim().to_string());
    }
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
    .await
    .map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Registration request to {endpoint} failed: {error}"
        ))
    })?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    // Host envelopes business errors as `{code, message}` with HTTP 200.
    let code = value.get("code").and_then(|c| c.as_i64()).unwrap_or(200);
    if code != 200 {
        let message = value
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("注册失败");
        return Err(crate::ErrorKind::OtherError(message.to_string()).into());
    }
    Ok(())
}

/// Picks the login (Sa-Token) session material out of a host login response,
/// merging the flat payload with its enveloped `{data}` form.
fn login_session_parts(
    login: &YdaLoginResponse,
) -> crate::Result<(String, Option<String>, Option<i64>)> {
    let data = login.data.as_deref();
    let pick = |flat: Option<&String>, nested: Option<&String>| -> Option<String> {
        flat.cloned().or_else(|| nested.cloned())
    };
    let access_token = pick(login.access_token.as_ref(), login.token.as_ref())
        .or_else(|| pick(data.and_then(|data| data.access_token.as_ref()), data.and_then(|data| data.token.as_ref())))
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "Login response did not contain a token".to_string(),
            )
        })?;
    let refresh_token = pick(login.refresh_token.as_ref(), data.and_then(|data| data.refresh_token.as_ref()));
    let expires_in = login
        .expires_in
        .or_else(|| data.and_then(|data| data.expires_in));
    Ok((access_token, refresh_token, expires_in))
}

/// Exchanges the OAuth access token for the host's login (Sa-Token) session:
/// subsequent domain calls carry the user's real RBAC permissions (same as a
/// website login) instead of scope-mirrored ones. The OAuth token itself is
/// only the bootstrap for this exchange. The host filter expects
/// `Bearer <oauth token>` on this call.
async fn exchange_for_login_session(
    origin: &str,
    oauth_token: &str,
) -> crate::Result<YdaLoginResponse> {
    let url = format!("{origin}/api/oauth/satoken/exchange");
    let client = if crate::util::fetch::is_loopback_url(&url) {
        reqwest::Client::builder().no_proxy()
    } else {
        reqwest::Client::builder()
    }
    .timeout(std::time::Duration::from_secs(30))
    .build()?;
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {oauth_token}"))
        .send()
        .await
        .map_err(|error| {
            crate::ErrorKind::OtherError(format!(
                "Session exchange request to {url} failed: {error}"
            ))
        })?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Session exchange failed ({status}): {}",
            body_snippet(&text)
        ))
        .into());
    }
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Session exchange response from {url} is not valid JSON: {error}; body: {}",
            body_snippet(&text)
        ))
    })?;
    serde_json::from_value(unwrap_token_payload(value))
        .map_err(|error| {
            crate::ErrorKind::OtherError(format!(
                "Session exchange response from {url} has an unexpected shape: {error}; body: {}",
                body_snippet(&text)
            ))
        })
        .map_err(crate::Error::from)
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
    // The OAuth token only bootstraps the session: swap it for the host's
    // login session so the launcher speaks to the domain as the real user.
    let login = exchange_for_login_session(&origin, &tokens.access_token).await?;
    let (access_token, refresh_token, expires_in) = login_session_parts(&login)?;
    normalize_and_store(domain_id, &origin, access_token, refresh_token, expires_in)
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

/// Refreshes the stored login (Sa-Token) session via the host's
/// `/api/user/token/refresh` using only the stored refresh token in the
/// request body — no `Authorization` header, so an already-expired access
/// token can still be renewed. Returns `None` when no refresh token exists.
///
/// Concurrent callers for the same domain coalesce on one refresh. When the
/// host rotates the refresh token only sometimes, the previous token is kept
/// so a response without `refreshToken` does not wipe silent renewal.
pub async fn refresh_session(
    domain_id: &str,
) -> crate::Result<Option<YmclStoredSession>> {
    let lock = REFRESH_LOCKS
        .entry(domain_id.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone();
    let _guard = lock.lock().await;
    refresh_session_locked(domain_id).await
}

async fn refresh_session_locked(
    domain_id: &str,
) -> crate::Result<Option<YmclStoredSession>> {
    let state = State::get().await?;
    let Some(stored) = ymcl_session::get(domain_id, &state.pool).await? else {
        return Ok(None);
    };
    if !stored.can_refresh() {
        return Ok(None);
    }
    // Another waiter may have already renewed while we queued on the lock.
    if !stored.needs_refresh(0) {
        return Ok(Some(stored));
    }
    let previous_refresh_token = stored.refresh_token.clone();
    let origin = super::registry::domain_origin(domain_id).await?;

    // Refresh is a refresh-token-only call: no Authorization header. The
    // access token may already be dead, and requiring it would break silent
    // renewal for the next expiry. The host authenticates solely via the
    // refresh token in the body.
    let endpoint = format!("{origin}/api/user/token/refresh");
    let body = serde_json::json!({
        "refreshToken": previous_refresh_token.as_deref().unwrap_or_default(),
    });
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
    .await
    .map_err(|error| {
        // Keep host auth rejections intact so the caller can distinguish a
        // dead refresh token from a transport blip.
        if is_host_auth_rejection(&error) {
            return error;
        }
        crate::ErrorKind::OtherError(format!(
            "Refresh request to {endpoint} failed: {error}"
        ))
        .into()
    })?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Refresh response from {endpoint} is not valid JSON: {error}"
        ))
    })?;
    let login: YdaLoginResponse = serde_json::from_value(unwrap_token_payload(value))
        .map_err(|error| {
            crate::ErrorKind::OtherError(format!(
                "Refresh response from {endpoint} has an unexpected shape: {error}"
            ))
        })?;
    let (access_token, mut refresh_token, expires_in) = login_session_parts(&login)?;
    // Hosts that do not rotate refresh tokens omit the field on refresh;
    // keep the previous one so the next expiry can still renew silently.
    if refresh_token.as_deref().is_none_or(|token| token.trim().is_empty()) {
        refresh_token = previous_refresh_token;
    }
    // A missing lifetime on refresh would clear expires_at and disable
    // proactive renewal; fall back to the host's usual 2h window.
    let expires_in = Some(expires_in.unwrap_or(DEFAULT_TOKEN_LIFETIME_SECS).max(0));

    normalize_and_store(domain_id, &origin, access_token, refresh_token, expires_in)
        .await
        .map(Some)
}

#[derive(serde::Serialize, Clone)]
pub struct YmclYggProfile {
    pub id: String,
    pub name: String,
}

pub type YmclYggProfileList = Vec<YmclYggProfile>;

#[derive(serde::Serialize)]
pub struct YmclYggExchange {
    pub credentials: crate::state::Credentials,
    pub profiles: Vec<YmclYggProfile>,
}

/// Exchanges the domain login session (Sa-Token) for a Yggdrasil session at
/// the domain's Yggdrasil service — whichever provider plugin the node runs —
/// so the Minecraft account attaches without re-entering the password.
/// `profile_name` picks among multiple owned profiles; `None` selects the
/// first one.
pub async fn ygg_exchange(
    domain_id: &str,
    profile_name: Option<&str>,
) -> crate::Result<YmclYggExchange> {
    use crate::state::{
        create_credentials, YggdrasilMetadata, YggdrasilProfile,
    };

    let state = State::get().await?;
    let origin = super::registry::domain_origin(domain_id).await?;
    let stored = ensure_session(domain_id)
        .await?
        .ok_or_else(|| crate::ErrorKind::OtherError("Not logged in to this domain".to_string()))?;
    // The provider shape is resolved by probe; a call that fails on a cached
    // shape re-probes once, in case the node switched providers.
    let mut reprobed = false;
    let (api_root, url, bytes) = loop {
        let endpoints = super::yggroot::ygg_endpoints(&origin).await?;
        let url = match profile_name {
            Some(name) => format!(
                "{}?profile={}",
                endpoints.exchange_url,
                urlencoding::encode(name)
            ),
            None => endpoints.exchange_url.clone(),
        };
        match domain_request(&state, domain_id, Method::POST, &url, None).await
        {
            Ok(bytes) => break (endpoints.yggdrasil_root, url, bytes),
            Err(error)
                if !reprobed
                    && super::yggroot::is_retryable_ygg_error(&error) =>
            {
                super::yggroot::invalidate_ygg_endpoints(&origin);
                reprobed = true;
            }
            Err(error) => {
                return Err(crate::ErrorKind::OtherError(format!(
                    "Minecraft session exchange from {url} failed: {error}"
                ))
                .into())
            }
        }
    };
    let snippet = body_snippet(&String::from_utf8_lossy(&bytes));
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| {
            crate::ErrorKind::OtherError(format!(
                "Exchange response from {url} is not valid JSON: {error}; body: {snippet}"
            ))
        })?;
    let access_token = value
        .get("access_token")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "Exchange response from {url} did not contain an access token; body: {snippet}"
            ))
        })?
        .to_string();
    let client_token = value
        .get("client_token")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let profiles: Vec<YmclYggProfile> = value
        .get("profiles")
        .and_then(|value| value.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|entry| {
                    Some(YmclYggProfile {
                        id: entry.get("id")?.as_str()?.to_string(),
                        name: entry.get("name")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let selected = match value.get("profile") {
        Some(profile) => Some(YmclYggProfile {
            id: profile
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    crate::ErrorKind::OtherError(format!(
                        "Exchange response from {url} has a profile without an id; body: {snippet}"
                    ))
                })?
                .to_string(),
            name: profile
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
        }),
        None => None,
    };
    let selected = match selected {
        Some(selected) => selected,
        None if !profiles.is_empty() => profiles[0].clone(),
        None => {
            return Err(crate::ErrorKind::OtherError(
                "This account has no available Minecraft profile".to_string(),
            )
            .into())
        }
    };

    let display_name: Option<(String,)> =
        sqlx::query_as("SELECT display_name FROM ymcl_domains WHERE id = $1")
            .bind(domain_id)
            .fetch_optional(&state.pool)
            .await?;
    let server_name = display_name
        .map(|row| row.0)
        .unwrap_or_else(|| origin.clone());
    let profile_id = uuid::Uuid::parse_str(&selected.id)
        .or_else(|_| uuid::Uuid::parse_str(&selected.id.replace('-', "")))
        .map_err(|error| {
            crate::ErrorKind::OtherError(format!(
                "Exchange returned an invalid profile id {}: {error}",
                selected.id
            ))
        })?;
    let login = stored.session.username;
    let credentials = create_credentials(
        YggdrasilProfile { id: profile_id, name: selected.name },
        access_token,
        client_token,
        YggdrasilMetadata {
            api_root,
            server_name,
            raw: String::new(),
        },
        &login,
    );
    credentials.upsert(&state.pool).await?;
    Ok(YmclYggExchange { credentials, profiles })
}

/// Lists the Minecraft profiles the signed-in user owns on the domain's
/// Yggdrasil service. No session is issued (list-only mode).
pub async fn ygg_profiles(domain_id: &str) -> crate::Result<Vec<YmclYggProfile>> {
    let state = State::get().await?;
    let origin = super::registry::domain_origin(domain_id).await?;
    let mut reprobed = false;
    let (url, bytes) = loop {
        let endpoints = super::yggroot::ygg_endpoints(&origin).await?;
        let url = endpoints.list_url;
        match domain_request(&state, domain_id, Method::POST, &url, None).await
        {
            Ok(bytes) => break (url, bytes),
            Err(error)
                if !reprobed
                    && super::yggroot::is_retryable_ygg_error(&error) =>
            {
                super::yggroot::invalidate_ygg_endpoints(&origin);
                reprobed = true;
            }
            Err(error) => {
                return Err(crate::ErrorKind::OtherError(format!(
                    "Profile list from {url} failed: {error}"
                ))
                .into())
            }
        }
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Profile list from {url} is not valid JSON: {error}; body: {}",
            body_snippet(&String::from_utf8_lossy(&bytes))
        ))
    })?;
    Ok(value
        .get("profiles")
        .and_then(|value| value.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|entry| {
                    Some(YmclYggProfile {
                        id: entry.get("id")?.as_str()?.to_string(),
                        name: entry.get("name")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Switches the department or role context of the session (YAP §6.4,
/// `context` capability) and stores the re-issued session info.
pub async fn context_switch(
    domain_id: &str,
    dept_id: Option<&str>,
    role_id: Option<&str>,
) -> crate::Result<YmclSessionInfo> {
    let state = State::get().await?;
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
    let bytes = domain_request(
        &state,
        domain_id,
        Method::POST,
        &context_switch_url(&origin),
        Some(body),
    )
    .await?;
    let session = serde_json::from_slice::<YmclSessionInfo>(&bytes)?;
    ymcl_session::update_session_info(domain_id, &session, &state.pool).await?;
    Ok(session)
}

/// Ensures a usable session for the domain: refreshes expired tokens
/// up-front. Sessions without refresh capability are surfaced as-is so the
/// UI can prompt a re-login.
///
/// Tokens approaching expiry are renewed before the host rejects them. When
/// a still-valid token's refresh fails (offline blip), the existing session
/// is kept so the next request can retry instead of forcing a re-login.
pub async fn ensure_session(
    domain_id: &str,
) -> crate::Result<Option<YmclStoredSession>> {
    let Some(session) = get_session(domain_id).await? else {
        return Ok(None);
    };
    if !session.can_refresh() || !session.needs_refresh(REFRESH_SKEW_SECS) {
        return Ok(Some(session));
    }
    let was_expired = session.is_expired();
    match refresh_session(domain_id).await {
        Ok(Some(renewed)) => Ok(Some(renewed)),
        Ok(None) => Ok(Some(session)),
        Err(error) if !was_expired => {
            tracing::warn!(
                "Proactive domain session refresh for {domain_id} failed while the token is still valid: {error}"
            );
            Ok(Some(session))
        }
        Err(error) => Err(error),
    }
}

/// Spawns the background domain-session keeper once at launcher startup.
/// While the app stays open it silently renews access tokens before expiry
/// (sliding the host's ~2h lifetime indefinitely for active players) and
/// touches still-valid sessions so host idle timeouts do not kill them.
pub fn start_session_keeper() {
    #[cfg(feature = "tauri")]
    tauri::async_runtime::spawn(async move {
        run_session_keeper().await;
    });
    #[cfg(not(feature = "tauri"))]
    {
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                run_session_keeper().await;
            });
        }
    }
}

async fn run_session_keeper() {
    let mut interval = tokio::time::interval(SESSION_KEEPER_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // Skip the immediate first tick: startup already loads sessions via
    // ensure_session, and a burst of refreshes here only races that path.
    interval.tick().await;
    loop {
        interval.tick().await;
        if let Err(error) = keep_sessions_alive().await {
            tracing::debug!("YMCL session keeper pass failed: {error}");
        }
    }
}

async fn keep_sessions_alive() -> crate::Result<()> {
    let state = State::get().await?;
    let domain_ids = ymcl_session::list_domain_ids(&state.pool).await?;
    for domain_id in domain_ids {
        let Ok(Some(session)) = get_session(&domain_id).await else {
            continue;
        };
        if session.can_refresh() && session.needs_refresh(REFRESH_SKEW_SECS) {
            match refresh_session(&domain_id).await {
                Ok(Some(_)) => {
                    tracing::info!(
                        "Silently renewed domain session for {domain_id}"
                    );
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(
                        "Failed to renew domain session for {domain_id}: {error}"
                    );
                }
            }
            continue;
        }
        // Still-valid tokens get a cheap authenticated ping so Sa-Token
        // active-timeout on the host does not expire a quiet session.
        touch_domain_session(&domain_id).await;
    }
    Ok(())
}

async fn touch_domain_session(domain_id: &str) {
    let Ok(state) = State::get().await else {
        return;
    };
    let Ok(origin) = super::registry::domain_origin(domain_id).await else {
        return;
    };
    let Ok(Some(session)) = get_session(domain_id).await else {
        return;
    };
    if session.is_expired() {
        return;
    }
    let result = fetch_session_info(
        &state,
        &origin,
        &session.access_token,
    )
    .await;
    if let Err(error) = result {
        tracing::debug!(
            "Domain session touch for {domain_id} failed (ignored): {error}"
        );
    }
}

/// The host's BizException text when the request principal is anonymous or
/// lacks a plugin permission — `无此插件权限：<permission>`. Surfaced through
/// the fetch layer as a `domain_error` LabrinthError.
const DOMAIN_PERMISSION_DENIED_PREFIX: &str = "无此插件权限：";

/// Sentinel surfaced when the domain token is dead and cannot be renewed —
/// the frontend localizes it and pops the domain login dialog.
const SESSION_EXPIRED_MESSAGE: &str = "Domain session expired — sign in to this domain again";

fn session_expired_error() -> crate::Error {
    crate::ErrorKind::OtherError(SESSION_EXPIRED_MESSAGE.to_string()).into()
}

/// Tells the frontend the domain session is unrecoverable and interactive
/// sign-in is required (YAP §6.10 envelope over the shared `ymcl://event`
/// channel). No-op outside the Tauri runtime.
async fn emit_session_expired(domain_id: &str) {
    #[cfg(feature = "tauri")]
    {
        use tauri::Emitter;
        if let Ok(event_state) = crate::event::EventState::get() {
            let _ = event_state.app.emit(
                "ymcl://event",
                serde_json::json!({ "type": "session.expired", "domain_id": domain_id }),
            );
        }
    }
    #[cfg(not(feature = "tauri"))]
    let _ = domain_id;
}

/// True when the host rejected the request as unauthenticated — the session
/// token expired or was invalidated before its locally stored expiry (host
/// restart, admin kick), so a refresh may still recover it.
///
/// The host never answers a spec 401 for a dead domain session on plugin
/// endpoints: an unresolvable principal falls into the same permission check
/// as an ungranted one and surfaces as `400 无此插件权限：…`. Either way the
/// only launcher-side recovery is an interactive re-login, so both shapes
/// count as a rejected token.
fn is_unauthorized_error(error: &crate::Error) -> bool {
    match error.raw.as_ref() {
        crate::ErrorKind::HttpError { status: 401, .. } => true,
        crate::ErrorKind::LabrinthError(error) => {
            error.status == Some(401)
                || (error.status == Some(400)
                    && error.error == "domain_error"
                    && error
                        .description
                        .starts_with(DOMAIN_PERMISSION_DENIED_PREFIX))
        }
        _ => false,
    }
}

async fn send_domain_request(
    state: &State,
    url: &str,
    method: Method,
    json_body: Option<serde_json::Value>,
    token: Option<&str>,
    loading_bar: Option<(&LoadingBarId, f64)>,
) -> crate::Result<bytes::Bytes> {
    let header = token.map(|token| ("Authorization", token));
    fetch_advanced(
        method,
        url,
        None,
        json_body,
        header,
        None,
        loading_bar,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await
}

async fn retry_on_rejected_token(
    state: &State,
    domain_id: &str,
    url: &str,
    method: Method,
    json_body: Option<serde_json::Value>,
    can_refresh: bool,
    had_session: bool,
    loading_bar: Option<(&LoadingBarId, f64)>,
    error: crate::Error,
) -> crate::Result<bytes::Bytes> {
    if !is_unauthorized_error(&error) {
        return Err(error);
    }
    if !can_refresh {
        // Dead token and nothing to renew it with — pop the domain login
        // dialog instead of the raw HTTP status. Endpoints that legitimately
        // serve anonymous callers (no session attached) keep the raw error.
        if had_session {
            emit_session_expired(domain_id).await;
            return Err(session_expired_error());
        }
        return Err(error);
    }
    // The host invalidated the token before its local expiry; renew it via
    // the refresh token and retry once — silent re-login. Only a host-rejected
    // refresh (refresh token itself dead) degrades to the login dialog;
    // transport failures keep the session for a later retry.
    let renewed = match refresh_session(domain_id).await {
        Ok(Some(renewed)) => renewed,
        Ok(None) => {
            emit_session_expired(domain_id).await;
            return Err(session_expired_error());
        }
        Err(refresh_error) => {
            tracing::warn!(
                "Domain session refresh for {domain_id} failed: {refresh_error}"
            );
            if is_host_auth_rejection(&refresh_error) {
                emit_session_expired(domain_id).await;
                return Err(session_expired_error());
            }
            return Err(error);
        }
    };
    tracing::info!(
        "Domain session token for {domain_id} was rejected; renewed via refresh token and retried"
    );
    send_domain_request(
        state,
        url,
        method,
        json_body,
        Some(&renewed.access_token),
        loading_bar,
    )
    .await
}

/// True when the host itself rejected the refresh (expired/invalid refresh
/// token), as opposed to a local transport or parse failure that a later
/// retry can still recover from.
fn is_host_auth_rejection(error: &crate::Error) -> bool {
    match error.raw.as_ref() {
        crate::ErrorKind::HttpError { status, .. } => {
            matches!(status, 400 | 401 | 403)
        }
        crate::ErrorKind::LabrinthError(error) => {
            matches!(error.status, Some(400 | 401 | 403))
        }
        _ => false,
    }
}

/// One authenticated request against a domain endpoint: refreshes an expired
/// token up-front, and when the host still rejects it, refreshes once and
/// retries before surfacing the error. Fails when the domain has no session.
pub async fn domain_request(
    state: &State,
    domain_id: &str,
    method: Method,
    url: &str,
    json_body: Option<serde_json::Value>,
) -> crate::Result<bytes::Bytes> {
    let Some(session) = ensure_session(domain_id).await? else {
        return Err(crate::ErrorKind::OtherError(
            "Not logged in to this domain".to_string(),
        )
        .into());
    };
    let can_refresh = session.can_refresh();
    let token = session.access_token;
    match send_domain_request(state, url, method.clone(), json_body.clone(), Some(&token), None)
        .await
    {
        Ok(bytes) => Ok(bytes),
        Err(error) => {
            retry_on_rejected_token(
                state,
                domain_id,
                url,
                method,
                json_body,
                can_refresh,
                true,
                None,
                error,
            )
            .await
        }
    }
}

/// Same as [`domain_request`] but for endpoints that also serve anonymous
/// callers: attaches the domain session only when one exists, still with the
/// refresh-and-retry recovery when it does.
pub async fn domain_request_opt(
    state: &State,
    domain_id: &str,
    method: Method,
    url: &str,
    json_body: Option<serde_json::Value>,
) -> crate::Result<bytes::Bytes> {
    domain_request_opt_with_bar(state, domain_id, method, url, json_body, None).await
}

/// [`domain_request_opt`] with a streaming download progress bar: the
/// response body is streamed and the bar advances with each chunk (only
/// while the server sends a `Content-Length`; see the fetch layer).
pub async fn domain_request_opt_with_bar(
    state: &State,
    domain_id: &str,
    method: Method,
    url: &str,
    json_body: Option<serde_json::Value>,
    loading_bar: Option<(&LoadingBarId, f64)>,
) -> crate::Result<bytes::Bytes> {
    let session = ensure_session(domain_id).await?;
    let can_refresh = session.as_ref().is_some_and(YmclStoredSession::can_refresh);
    let had_session = session.is_some();
    let token = session.as_ref().map(|session| session.access_token.as_str());
    match send_domain_request(
        state,
        url,
        method.clone(),
        json_body.clone(),
        token,
        loading_bar,
    )
    .await
    {
        Ok(bytes) => Ok(bytes),
        Err(error) => {
            retry_on_rejected_token(
                state,
                domain_id,
                url,
                method,
                json_body,
                can_refresh,
                had_session,
                loading_bar,
                error,
            )
            .await
        }
    }
}

#[cfg(test)]
mod url_rebase_tests {
    use super::*;

    /// The joined origin is the only address the launcher may use; adapter
    /// self-reported hosts (dev backend ports, internal hostnames) must never
    /// leak into requests or browser navigation.
    #[test]
    fn rebases_adapter_urls_onto_joined_origin() {
        assert_eq!(
            rebase_to_origin(
                "http://localhost:8080/api/user/login",
                "http://localhost:9002"
            ),
            "http://localhost:9002/api/user/login"
        );
        assert_eq!(
            rebase_to_origin(
                "https://yda.example.com/api/oauth/token",
                "https://yda.example.com"
            ),
            "https://yda.example.com/api/oauth/token"
        );
        // Relative adapter-reported paths resolve against the joined origin.
        assert_eq!(
            rebase_to_origin("/api/user/login", "http://localhost:9002"),
            "http://localhost:9002/api/user/login"
        );
    }

    /// Browsers get the site OAuth page; the raw authorize API answers JSON
    /// and 401s without a web session, so an adapter still reporting the API
    /// path is upgraded to `/oauth/authorize`.
    #[test]
    fn authorize_url_upgrades_to_browser_page() {
        assert_eq!(
            browser_authorize_url(
                "http://localhost:8080/api/oauth/authorize",
                "http://localhost:9002"
            ),
            "http://localhost:9002/oauth/authorize"
        );
        // Adapters already reporting the page path are kept as-is.
        assert_eq!(
            browser_authorize_url(
                "https://yda.example.com/oauth/authorize",
                "https://yda.example.com"
            ),
            "https://yda.example.com/oauth/authorize"
        );
    }
}

#[cfg(test)]
mod domain_rejection_tests {
    use super::*;

    /// The host's dead-session shape: `400 {code, message}` whose message is
    /// the BizException 无此插件权限 text, surfaced by the fetch layer as a
    /// `domain_error` LabrinthError. Must count as a rejected token so the
    /// re-login flow fires instead of the raw permission error.
    #[test]
    fn host_permission_denied_counts_as_rejected_token() {
        let error: crate::Error = crate::ErrorKind::LabrinthError(crate::LabrinthError {
            error: "domain_error".to_string(),
            description: format!("{DOMAIN_PERMISSION_DENIED_PREFIX}plugin:ymcl-adapter:view"),
            status: Some(400),
            method: Some("GET".to_string()),
            url: Some("http://localhost:9002/api/plugins/ymcl-adapter/v1/skins/profiles"
                .to_string()),
            route: None,
        })
        .into();
        assert!(is_unauthorized_error(&error));
    }

    /// Spec 401s keep counting as rejected tokens.
    #[test]
    fn unauthorized_status_counts_as_rejected_token() {
        let error: crate::Error = crate::ErrorKind::HttpError {
            status: 401,
            method: "GET".to_string(),
            url: "http://localhost:9002/api/user/session".to_string(),
        }
        .into();
        assert!(is_unauthorized_error(&error));
    }

    /// Other 400 domain errors (genuine business failures) are not session
    /// rejections and must surface raw.
    #[test]
    fn other_domain_errors_stay_raw() {
        let error: crate::Error = crate::ErrorKind::LabrinthError(crate::LabrinthError {
            error: "domain_error".to_string(),
            description: "不是该档案的拥有者".to_string(),
            status: Some(400),
            method: Some("POST".to_string()),
            url: Some("http://localhost:9002/api/plugins/ymcl-adapter/v1/skins/closet/p1/equip"
                .to_string()),
            route: None,
        })
        .into();
        assert!(!is_unauthorized_error(&error));
    }

    /// A bare transport-level 400 without the permission text is not a
    /// rejected token.
    #[test]
    fn plain_http_400_stays_raw() {
        let error: crate::Error = crate::ErrorKind::HttpError {
            status: 400,
            method: "GET".to_string(),
            url: "http://localhost:9002/api/plugins/ymcl-adapter/v1/skins/profiles".to_string(),
        }
        .into();
        assert!(!is_unauthorized_error(&error));
    }
}

#[cfg(test)]
mod session_renewal_tests {
    use super::*;
    use chrono::Utc;

    fn stored(
        expires_in: Option<i64>,
        refresh_token: Option<&str>,
    ) -> YmclStoredSession {
        YmclStoredSession {
            domain_id: "example".to_string(),
            access_token: "access".to_string(),
            refresh_token: refresh_token.map(str::to_string),
            expires_at: expires_in
                .map(|seconds| Utc::now().timestamp() + seconds),
            session: YmclSessionInfo::default(),
            updated_at: Utc::now().timestamp(),
        }
    }

    /// Tokens without expiry information are never proactively refreshed.
    #[test]
    fn missing_expiry_does_not_need_refresh() {
        assert!(!stored(None, Some("refresh")).needs_refresh(REFRESH_SKEW_SECS));
    }

    /// A fresh 2h token is left alone until the skew window.
    #[test]
    fn fresh_token_outside_skew_is_not_refreshed() {
        assert!(!stored(Some(DEFAULT_TOKEN_LIFETIME_SECS), Some("refresh"))
            .needs_refresh(REFRESH_SKEW_SECS));
    }

    /// Tokens inside the skew window (or already expired) renew up-front.
    #[test]
    fn near_expiry_token_is_refreshed_proactively() {
        assert!(stored(Some(REFRESH_SKEW_SECS - 1), Some("refresh"))
            .needs_refresh(REFRESH_SKEW_SECS));
        assert!(stored(Some(-1), Some("refresh")).needs_refresh(REFRESH_SKEW_SECS));
    }

    /// Blank refresh tokens are treated as missing so the keeper skips them.
    #[test]
    fn blank_refresh_token_cannot_renew() {
        assert!(!stored(Some(60), Some("   ")).can_refresh());
        assert!(stored(Some(60), Some("token")).can_refresh());
        assert!(!stored(Some(60), None).can_refresh());
    }

    /// Host auth rejections (401/403/400 on refresh) force interactive login;
    /// transport-shaped errors must not.
    #[test]
    fn host_rejection_distinguishes_from_transport() {
        let rejected: crate::Error = crate::ErrorKind::HttpError {
            status: 401,
            method: "POST".to_string(),
            url: "http://localhost:9002/api/user/token/refresh".to_string(),
        }
        .into();
        assert!(is_host_auth_rejection(&rejected));

        let timeout: crate::Error = crate::ErrorKind::OtherError(
            "Refresh request to http://localhost:9002/api/user/token/refresh failed: timed out"
                .to_string(),
        )
        .into();
        assert!(!is_host_auth_rejection(&timeout));

        let local: crate::Error =
            crate::ErrorKind::OtherError("disk full".to_string()).into();
        assert!(!is_host_auth_rejection(&local));
    }
}

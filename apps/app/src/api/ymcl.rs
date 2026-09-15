use crate::api::oauth_utils;
use crate::api::{Result, TheseusSerializableError};
use tauri::Manager;
use tauri::Runtime;
use tauri::plugin::TauriPlugin;
use tauri_plugin_opener::OpenerExt;
use theseus::ymcl::auth::{
    self, PkceChallenge, YmclExternalFlow, YmclExternalPoll,
    YmclExternalProvider,
};
use theseus::ymcl::registry::{self, YmclDomainsState, YmclManifest};
use theseus::ymcl::{YmclSessionInfo, YmclStoredSession};
use tokio::sync::oneshot;

const OAUTH_LOGIN_TIMEOUT_SECS: u64 = 300;

pub fn init<R: tauri::Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("ymcl")
        .invoke_handler(tauri::generate_handler![
            ymcl_domains_state,
            ymcl_domain_add,
            ymcl_domain_remove,
            ymcl_domain_activate,
            ymcl_manifest_get,
            ymcl_manifest_refresh,
            ymcl_data_fetch,
            ymcl_session_get,
            ymcl_auth_password_login,
            ymcl_auth_oauth_login,
            ymcl_auth_external_providers,
            ymcl_auth_external_begin,
            ymcl_auth_external_poll,
            ymcl_auth_external_finish,
            ymcl_auth_logout,
            ymcl_context_switch,
            ymcl_chrome_home_get,
            ymcl_chrome_home_put,
            ymcl_bundle_get,
            ymcl_pre_launch_check,
            ymcl_pack_apply_update,
            ymcl_publish_diff,
            ymcl_publish_push
        ])
        .build()
}

// List all domains with the active marker
// invoke('plugin:ymcl|ymcl_domains_state')
#[tauri::command]
pub async fn ymcl_domains_state() -> Result<YmclDomainsState> {
    Ok(registry::domains_state().await?)
}

// Add a domain by probing its adapter capabilities
// invoke('plugin:ymcl|ymcl_domain_add', { origin })
#[tauri::command]
pub async fn ymcl_domain_add(origin: String) -> Result<YmclDomainsState> {
    Ok(registry::add_domain(&origin).await?)
}

// Remove a joined domain
// invoke('plugin:ymcl|ymcl_domain_remove', { id })
#[tauri::command]
pub async fn ymcl_domain_remove(id: String) -> Result<YmclDomainsState> {
    Ok(registry::remove_domain(&id).await?)
}

// Activate a domain (or the personal domain)
// invoke('plugin:ymcl|ymcl_domain_activate', { id })
#[tauri::command]
pub async fn ymcl_domain_activate(id: String) -> Result<YmclDomainsState> {
    Ok(registry::activate_domain(&id).await?)
}

// Manifest of the active domain; None for the personal domain
// invoke('plugin:ymcl|ymcl_manifest_get')
#[tauri::command]
pub async fn ymcl_manifest_get() -> Result<Option<YmclManifest>> {
    Ok(registry::active_manifest().await?)
}

// Force a re-fetch of the active domain's manifest
// invoke('plugin:ymcl|ymcl_manifest_refresh')
#[tauri::command]
pub async fn ymcl_manifest_refresh() -> Result<Option<YmclManifest>> {
    Ok(registry::refresh_manifest().await?)
}

// Stored session for a domain, refreshing expired tokens when possible
// invoke('plugin:ymcl|ymcl_session_get', { domainId })
#[tauri::command]
pub async fn ymcl_session_get(
    domain_id: String,
) -> Result<Option<YmclStoredSession>> {
    Ok(auth::ensure_session(&domain_id).await?)
}

// Username + password login against the domain's host endpoint
// invoke('plugin:ymcl|ymcl_auth_password_login', { domainId, username, password })
#[tauri::command]
pub async fn ymcl_auth_password_login(
    domain_id: String,
    username: String,
    password: String,
) -> Result<YmclStoredSession> {
    Ok(auth::password_login(&domain_id, &username, &password).await?)
}

/// Runs the full OAuth authorization-code + PKCE flow: loopback listener,
/// browser hand-off, code capture and token exchange (YAP §6.4 oauth-web).
#[tauri::command]
pub async fn ymcl_auth_oauth_login<R: Runtime>(
    app: tauri::AppHandle<R>,
    domain_id: String,
) -> Result<YmclStoredSession> {
    let capabilities = registry::domain_capabilities(&domain_id).await?;
    let oauth_method = capabilities
        .auth
        .as_ref()
        .and_then(|auth| {
            auth.methods
                .iter()
                .find(|method| method.r#type == "oauth-web")
        })
        .ok_or_else(|| {
            TheseusSerializableError::Theseus(
                theseus::ErrorKind::OtherError(
                    "This domain does not offer OAuth login".into(),
                )
                .into(),
            )
        })?;
    let authorize_endpoint =
        oauth_method.authorize_url.clone().ok_or_else(|| {
            TheseusSerializableError::Theseus(
                theseus::ErrorKind::OtherError(
                    "OAuth login method is missing its authorize endpoint"
                        .into(),
                )
                .into(),
            )
        })?;
    let token_url = oauth_method.token_url.clone().ok_or_else(|| {
        TheseusSerializableError::Theseus(
            theseus::ErrorKind::OtherError(
                "OAuth login method is missing its token endpoint".into(),
            )
            .into(),
        )
    })?;
    let client_id = oauth_method
        .client_id
        .clone()
        .unwrap_or_else(|| "ymcl".to_string());

    let (listen_socket_tx, listen_socket) = oneshot::channel();
    let listener_task =
        tokio::spawn(oauth_utils::auth_code_reply::listen(listen_socket_tx));
    let socket = listen_socket.await.unwrap()?;
    let redirect_uri =
        format!("http://127.0.0.1:{}/auth/callback", socket.port());

    let challenge = auth::generate_pkce();
    let state_nonce = auth::generate_state_nonce();
    let authorize = auth::authorize_url(
        &authorize_endpoint,
        &client_id,
        &redirect_uri,
        &PkceChallenge {
            verifier: challenge.verifier.clone(),
            challenge: challenge.challenge.clone(),
        },
        &state_nonce,
    );

    app.opener()
        .open_url(&authorize, None::<&str>)
        .map_err(|e| {
            TheseusSerializableError::Theseus(
                theseus::ErrorKind::OtherError(format!(
                    "Failed to open authorize URL: {e}"
                ))
                .into(),
            )
        })?;

    let reply = match tokio::time::timeout(
        std::time::Duration::from_secs(OAUTH_LOGIN_TIMEOUT_SECS),
        listener_task,
    )
    .await
    {
        Ok(Ok(Ok(Some(reply)))) => reply,
        Ok(_) => {
            return Err(TheseusSerializableError::Theseus(
                theseus::ErrorKind::OtherError("Login canceled".into()).into(),
            ));
        }
        Err(_) => {
            oauth_utils::auth_code_reply::stop_listeners();
            return Err(TheseusSerializableError::Theseus(
                theseus::ErrorKind::OtherError("Login timed out".into()).into(),
            ));
        }
    };

    if let Some(reply_state) = reply.state {
        if reply_state != state_nonce {
            return Err(TheseusSerializableError::Theseus(
                theseus::ErrorKind::OtherError("OAuth state mismatch".into())
                    .into(),
            ));
        }
    }

    if let Some(main_window) = app.get_window("main") {
        main_window.set_focus().ok();
    }

    Ok(auth::oauth_finish(
        &domain_id,
        &token_url,
        &client_id,
        &reply.code,
        &challenge.verifier,
        &redirect_uri,
    )
    .await?)
}

// List third-party login providers of a domain
// invoke('plugin:ymcl|ymcl_auth_external_providers', { domainId })
#[tauri::command]
pub async fn ymcl_auth_external_providers(
    domain_id: String,
) -> Result<Vec<YmclExternalProvider>> {
    Ok(auth::external_providers(&domain_id).await?)
}

// Begin a third-party login flow; the frontend opens authorize_url in the browser
// invoke('plugin:ymcl|ymcl_auth_external_begin', { domainId, provider })
#[tauri::command]
pub async fn ymcl_auth_external_begin<R: Runtime>(
    app: tauri::AppHandle<R>,
    domain_id: String,
    provider: String,
) -> Result<YmclExternalFlow> {
    let flow = auth::external_begin(&domain_id, &provider).await?;
    app.opener()
        .open_url(&flow.authorize_url, None::<&str>)
        .map_err(|e| {
            TheseusSerializableError::Theseus(
                theseus::ErrorKind::OtherError(format!(
                    "Failed to open external login URL: {e}"
                ))
                .into(),
            )
        })?;
    Ok(flow)
}

// Poll a third-party login flow; completes and stores the session on success
// invoke('plugin:ymcl|ymcl_auth_external_poll', { domainId, provider, flowId })
#[tauri::command]
pub async fn ymcl_auth_external_poll(
    domain_id: String,
    provider: String,
    flow_id: String,
) -> Result<YmclExternalPoll> {
    Ok(auth::external_poll(&domain_id, &provider, &flow_id).await?)
}

// Finalize an external login after polling reported completion
// invoke('plugin:ymcl|ymcl_auth_external_finish', { domainId, accessToken, refreshToken, expiresIn })
#[tauri::command]
pub async fn ymcl_auth_external_finish(
    domain_id: String,
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
) -> Result<YmclStoredSession> {
    Ok(auth::external_finish(
        &domain_id,
        access_token,
        refresh_token,
        expires_in,
    )
    .await?)
}

// Log out of a domain, dropping stored tokens
// invoke('plugin:ymcl|ymcl_auth_logout', { domainId })
#[tauri::command]
pub async fn ymcl_auth_logout(domain_id: String) -> Result<()> {
    Ok(auth::logout(&domain_id).await?)
}

// Switch the department or role context of the active session
// invoke('plugin:ymcl|ymcl_context_switch', { domainId, deptId, roleId })
#[tauri::command]
pub async fn ymcl_context_switch(
    domain_id: String,
    dept_id: Option<String>,
    role_id: Option<String>,
) -> Result<YmclSessionInfo> {
    Ok(
        auth::context_switch(
            &domain_id,
            dept_id.as_deref(),
            role_id.as_deref(),
        )
        .await?,
    )
}

// Fetch a YAP data envelope from the active domain
// invoke('plugin:ymcl|ymcl_data_fetch', { providerCode, sourceCode, query })
#[tauri::command]
pub async fn ymcl_data_fetch(
    provider_code: String,
    source_code: String,
    query: Option<String>,
) -> Result<serde_json::Value> {
    Ok(
        theseus::ymcl::data::fetch_data(&provider_code, &source_code, query)
            .await?,
    )
}

// Fetch the domain's current home layout config (home designer)
// invoke('plugin:ymcl|ymcl_chrome_home_get')
#[tauri::command]
pub async fn ymcl_chrome_home_get() -> Result<serde_json::Value> {
    Ok(theseus::ymcl::chrome::get_home_config().await?)
}

// Save the domain's home layout config; members are notified by the adapter
// invoke('plugin:ymcl|ymcl_chrome_home_put', { config })
#[tauri::command]
pub async fn ymcl_chrome_home_put(config: serde_json::Value) -> Result<()> {
    Ok(theseus::ymcl::chrome::put_home_config(config).await?)
}

// Ensure a page's extension bundle is downloaded+verified; return entry path
// invoke('plugin:ymcl|ymcl_bundle_get', { pageId })
#[tauri::command]
pub async fn ymcl_bundle_get(page_id: String) -> Result<theseus::ymcl::bundle::YmclBundleReady> {
    Ok(theseus::ymcl::bundle::ensure_page_bundle(&page_id).await?)
}

// Launch-time binding check for a MIP-managed instance (read-only)
// invoke('plugin:ymcl|ymcl_pre_launch_check', { instanceId })
#[tauri::command]
pub async fn ymcl_pre_launch_check(
    instance_id: String,
) -> Result<theseus::ymcl::mip::update::YmclUpdateCheck> {
    Ok(theseus::ymcl::mip::update::check_instance(&instance_id, false).await?)
}

// Apply a pending incremental update to a MIP-managed instance
// invoke('plugin:ymcl|ymcl_pack_apply_update', { instanceId })
#[tauri::command]
pub async fn ymcl_pack_apply_update(
    instance_id: String,
) -> Result<theseus::ymcl::mip::update::YmclUpdateResult> {
    let check = theseus::ymcl::mip::update::check_instance(&instance_id, true).await?;
    Ok(theseus::ymcl::mip::update::YmclUpdateResult {
        applied: check
            .pending_changes
            .map(|changes| changes > 0)
            .unwrap_or(false),
        staged_files: check.pending_changes.unwrap_or(0),
        deleted_files: check.pending_deletions.unwrap_or(0),
        new_version: check.target_version.unwrap_or_default(),
    })
}

// Compute what a publish from this instance would contain (release console)
// invoke('plugin:ymcl|ymcl_publish_diff', { instanceId })
#[tauri::command]
pub async fn ymcl_publish_diff(
    instance_id: String,
) -> Result<theseus::ymcl::mip::publish::PublishDiff> {
    Ok(theseus::ymcl::mip::publish::diff_instance(&instance_id).await?)
}

// Push the diff as an incremental delta to the domain (release console)
// invoke('plugin:ymcl|ymcl_publish_push', { instanceId, version, channel, bind })
#[tauri::command]
pub async fn ymcl_publish_push(
    instance_id: String,
    version: String,
    channel: Option<String>,
    bind: Option<serde_json::Value>,
) -> Result<serde_json::Value> {
    Ok(
        theseus::ymcl::mip::publish::push_delta(
            &instance_id,
            &version,
            channel.as_deref(),
            bind,
        )
        .await?,
    )
}

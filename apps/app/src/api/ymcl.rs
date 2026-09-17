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
            ymcl_action_execute,
            ymcl_session_get,
            ymcl_auth_password_login,
            ymcl_auth_register,
            ymcl_auth_oauth_login,
            ymcl_ygg_exchange,
            ymcl_ygg_profiles,
            ymcl_auth_oauth_cancel,
            ymcl_auth_external_providers,
            ymcl_auth_external_begin,
            ymcl_auth_external_poll,
            ymcl_auth_external_finish,
            ymcl_auth_logout,
            ymcl_context_switch,
            ymcl_chrome_home_get,
            ymcl_chrome_home_put,
            ymcl_bundle_get,
            ymcl_skin_profiles,
            ymcl_skin_create_profile,
            ymcl_skin_closet,
            ymcl_skin_equip,
            ymcl_skin_upload,
            ymcl_cape_upload,
            ymcl_skin_delete,
            ymcl_skin_library,
            ymcl_skin_collect,
            ymcl_skin_texture,
            ymcl_skin_domain_for_account,
            ymcl_pre_launch_check,
            ymcl_pack_apply_update,
            ymcl_pack_features,
            ymcl_pack_set_features,
            ymcl_pack_install_preview,
            ymcl_pack_mrpack_download,
            ymcl_pack_adopt_state,
            ymcl_join_preview,
            ymcl_publish_diff,
            ymcl_publish_push,
            ymcl_publish_initial,
            ymcl_publish_list_versions,
            ymcl_publish_withdraw,
            ymcl_mip_servers
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

/// Exchanges the domain login session (Sa-Token) for a Yggdrasil Minecraft
/// account at the domain's authlib-injector service, storing the account so
/// the game can launch as the domain user without re-entering the password.
// invoke('plugin:ymcl|ymcl_ygg_exchange', { domainId, profileName })
#[tauri::command]
pub async fn ymcl_ygg_exchange(
    domain_id: String,
    profile_name: Option<String>,
) -> Result<auth::YmclYggExchange> {
    Ok(auth::ygg_exchange(&domain_id, profile_name.as_deref()).await?)
}

/// Lists the Minecraft profiles the signed-in user owns on the domain's
/// authlib-injector service.
// invoke('plugin:ymcl|ymcl_ygg_profiles', { domainId })
#[tauri::command]
pub async fn ymcl_ygg_profiles(domain_id: String) -> Result<auth::YmclYggProfileList> {
    Ok(auth::ygg_profiles(&domain_id).await?)
}

/// Cancels a pending OAuth browser login: stops the loopback listener so the
/// awaiting `ymcl_auth_oauth_login` resolves immediately as canceled instead
/// of waiting out its full timeout window.
// invoke('plugin:ymcl|ymcl_auth_oauth_cancel')
#[tauri::command]
pub fn ymcl_auth_oauth_cancel() {
    oauth_utils::auth_code_reply::stop_listeners();
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

    // The adapter reports endpoint URLs against its own host, but the
    // browser must only ever see the origin the user joined: the OAuth
    // authorize page lives on the site (web session + login redirect live
    // there) and exposing the internal backend address is not acceptable.
    let origin = registry::domain_origin(&domain_id).await?;
    let authorize_endpoint =
        auth::browser_authorize_url(&authorize_endpoint, &origin);
    let token_url = auth::rebase_to_origin(&token_url, &origin);

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

// Execute a YAP server:* action against the active domain's adapter
// (YAP §6.7); resolves with the adapter's response envelope, if any.
// invoke('plugin:ymcl|ymcl_action_execute', { providerCode, actionCode, params })
#[tauri::command]
pub async fn ymcl_action_execute(
    provider_code: String,
    action_code: String,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    Ok(
        theseus::ymcl::data::execute_action(&provider_code, &action_code, params)
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

// List the Minecraft profiles the domain session's user owns (skin wardrobe)
// invoke('plugin:ymcl|ymcl_skin_profiles', { domainId })
#[tauri::command]
pub async fn ymcl_skin_profiles(
    domain_id: String,
) -> Result<Vec<theseus::ymcl::skins::YmclSkinProfile>> {
    Ok(theseus::ymcl::skins::list_profiles(&domain_id).await?)
}

// Fetch a profile's skin wardrobe: skins, capes and current equipment
// invoke('plugin:ymcl|ymcl_skin_closet', { domainId, profileId })
#[tauri::command]
pub async fn ymcl_skin_closet(
    domain_id: String,
    profile_id: String,
) -> Result<theseus::ymcl::skins::YmclCloset> {
    Ok(theseus::ymcl::skins::fetch_closet(&domain_id, &profile_id).await?)
}

// One-shot equip: apply a skin and/or cape to a domain profile (null = take off)
// invoke('plugin:ymcl|ymcl_skin_equip', { domainId, profileId, skinId, capeId })
#[tauri::command]
pub async fn ymcl_skin_equip(
    domain_id: String,
    profile_id: String,
    skin_id: Option<String>,
    cape_id: Option<String>,
) -> Result<theseus::ymcl::skins::YmclEquippedState> {
    Ok(
        theseus::ymcl::skins::equip(
            &domain_id,
            &profile_id,
            skin_id.as_deref(),
            cape_id.as_deref(),
        )
        .await?,
    )
}

// Upload a skin PNG (base64 data) into a domain profile's wardrobe
// invoke('plugin:ymcl|ymcl_skin_upload', { domainId, profileId, name, model, filename, data })
#[tauri::command]
pub async fn ymcl_skin_upload(
    domain_id: String,
    profile_id: String,
    name: Option<String>,
    model: String,
    filename: String,
    data: String,
) -> Result<theseus::ymcl::skins::YmclSkinUploadResult> {
    Ok(
        theseus::ymcl::skins::upload_skin(
            &domain_id,
            &profile_id,
            name.as_deref(),
            &model,
            &filename,
            &data,
        )
        .await?,
    )
}

// Upload a cape PNG (base64 data) into a domain profile's wardrobe
// invoke('plugin:ymcl|ymcl_cape_upload', { domainId, profileId, name, filename, data })
#[tauri::command]
pub async fn ymcl_cape_upload(
    domain_id: String,
    profile_id: String,
    name: Option<String>,
    filename: String,
    data: String,
) -> Result<theseus::ymcl::skins::YmclCapeUploadResult> {
    Ok(theseus::ymcl::skins::upload_cape(
        &domain_id,
        &profile_id,
        name.as_deref(),
        &filename,
        &data,
    )
    .await?)
}

// Create a Minecraft character for the signed-in domain user
// invoke('plugin:ymcl|ymcl_skin_create_profile', { domainId, name })
#[tauri::command]
pub async fn ymcl_skin_create_profile(
    domain_id: String,
    name: String,
) -> Result<theseus::ymcl::skins::YmclSkinProfile> {
    Ok(theseus::ymcl::skins::create_profile(&domain_id, &name).await?)
}

// Register a new domain account (does not auto-login)
// invoke('plugin:ymcl|ymcl_auth_register', { domainId, username, email, password, nickname })
#[tauri::command]
pub async fn ymcl_auth_register(
    domain_id: String,
    username: String,
    email: String,
    password: String,
    nickname: Option<String>,
) -> Result<()> {
    Ok(theseus::ymcl::auth::register(
        &domain_id,
        &username,
        &email,
        &password,
        nickname.as_deref(),
    )
    .await?)
}

// Remove a skin from a domain profile's wardrobe
// invoke('plugin:ymcl|ymcl_skin_delete', { domainId, profileId, skinId })
#[tauri::command]
pub async fn ymcl_skin_delete(
    domain_id: String,
    profile_id: String,
    skin_id: String,
) -> Result<()> {
    Ok(theseus::ymcl::skins::delete_skin(&domain_id, &profile_id, &skin_id).await?)
}

// Fetch a page of the domain's public skin library
// invoke('plugin:ymcl|ymcl_skin_library', { domainId, page, limit })
#[tauri::command]
pub async fn ymcl_skin_library(
    domain_id: String,
    page: u32,
    limit: u32,
) -> Result<theseus::ymcl::skins::YmclLibraryPage> {
    Ok(theseus::ymcl::skins::fetch_library(&domain_id, page, limit).await?)
}

// Collect a public-library texture into a profile's wardrobe (idempotent)
// invoke('plugin:ymcl|ymcl_skin_collect', { domainId, profileId, hash, name })
#[tauri::command]
pub async fn ymcl_skin_collect(
    domain_id: String,
    profile_id: String,
    hash: String,
    name: Option<String>,
) -> Result<theseus::ymcl::skins::YmclSkinUploadResult> {
    Ok(
        theseus::ymcl::skins::collect(
            &domain_id,
            &profile_id,
            &hash,
            name.as_deref(),
        )
        .await?,
    )
}

// Download a texture URL (cape PNGs) as a data URL, with no skin-format
// conversion — normalize_skin_texture would corrupt cape textures
// invoke('plugin:ymcl|ymcl_skin_texture', { url })
#[tauri::command]
pub async fn ymcl_skin_texture(url: String) -> Result<String> {
    Ok(theseus::ymcl::skins::fetch_texture_data_url(&url).await?)
}

// Match a Minecraft account's Yggdrasil api_root against joined domains,
// telling the skins page whether the account is domain-owned with a wardrobe
// invoke('plugin:ymcl|ymcl_skin_domain_for_account', { apiRoot })
#[tauri::command]
pub async fn ymcl_skin_domain_for_account(
    api_root: String,
) -> Result<Option<theseus::ymcl::skins::YmclSkinDomainMatch>> {
    Ok(theseus::ymcl::skins::domain_for_yggdrasil_root(&api_root).await?)
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
        skipped_files: check.skipped_files.unwrap_or_default(),
    })
}

// Join-flow preview: what a server requires (bound pack or bare version)
// invoke('plugin:ymcl|ymcl_join_preview', { serverId })
#[tauri::command]
pub async fn ymcl_join_preview(
    server_id: Option<String>,
) -> Result<theseus::ymcl::mip::update::YmclJoinPreview> {
    Ok(theseus::ymcl::mip::update::join_preview(server_id.as_deref()).await?)
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
// invoke('plugin:ymcl|ymcl_publish_push', { instanceId, version, channel, bind, features, policies })
#[tauri::command]
pub async fn ymcl_publish_push(
    instance_id: String,
    version: String,
    channel: Option<String>,
    bind: Option<serde_json::Value>,
    features: Option<Vec<theseus::ymcl::mip::publish::PublishFeatureInput>>,
    policies: Option<Vec<theseus::ymcl::mip::publish::PublishPolicyInput>>,
) -> Result<serde_json::Value> {
    Ok(
        theseus::ymcl::mip::publish::push_delta(
            &instance_id,
            &version,
            channel.as_deref(),
            bind,
            features.unwrap_or_default(),
            policies.unwrap_or_default(),
        )
        .await?,
    )
}

// invoke('plugin:ymcl|ymcl_publish_initial', { instanceId, packId?, version, channel, bind, features, policies, exclude? })
// packId omitted → the launcher generates a unique `slug-<uuid>` id.
#[tauri::command]
pub async fn ymcl_publish_initial(
    instance_id: String,
    pack_id: Option<String>,
    version: String,
    channel: Option<String>,
    bind: Option<serde_json::Value>,
    features: Option<Vec<theseus::ymcl::mip::publish::PublishFeatureInput>>,
    policies: Option<Vec<theseus::ymcl::mip::publish::PublishPolicyInput>>,
    exclude: Option<Vec<String>>,
) -> Result<serde_json::Value> {
    Ok(
        theseus::ymcl::mip::publish::push_initial(
            &instance_id,
            pack_id.as_deref(),
            &version,
            channel.as_deref(),
            bind,
            features.unwrap_or_default(),
            policies.unwrap_or_default(),
            exclude.unwrap_or_default(),
        )
        .await?,
    )
}

// invoke('plugin:ymcl|ymcl_publish_list_versions', { instanceId })
#[tauri::command]
pub async fn ymcl_publish_list_versions(
    instance_id: String,
) -> Result<Vec<theseus::ymcl::mip::publish::PackVersionInfo>> {
    Ok(theseus::ymcl::mip::publish::list_instance_versions(&instance_id).await?)
}

// invoke('plugin:ymcl|ymcl_publish_withdraw', { instanceId, version })
#[tauri::command]
pub async fn ymcl_publish_withdraw(
    instance_id: String,
    version: String,
) -> Result<serde_json::Value> {
    Ok(theseus::ymcl::mip::publish::withdraw_instance_version(
        &instance_id,
        &version,
    )
    .await?)
}

// WF-4 install preview for an unmanaged instance
// invoke('plugin:ymcl|ymcl_pack_install_preview', { instanceId, serverId })
#[tauri::command]
pub async fn ymcl_pack_install_preview(
    instance_id: String,
    server_id: Option<String>,
) -> Result<theseus::ymcl::mip::update::YmclInstallPreview> {
    Ok(theseus::ymcl::mip::update::install_preview(
        &instance_id,
        server_id.as_deref(),
    )
    .await?)
}

// WF-4 first install step 1: download the binding's target mrpack archive
// invoke('plugin:ymcl|ymcl_pack_mrpack_download', { serverId, packId, selected })
#[tauri::command]
pub async fn ymcl_pack_mrpack_download(
    server_id: String,
    pack_id: Option<String>,
    selected: Vec<String>,
) -> Result<theseus::ymcl::mip::first_install::YmclMrpackDownload> {
    Ok(
        theseus::ymcl::mip::first_install::download_pack_mrpack(
            &server_id,
            pack_id,
            selected,
        )
        .await?,
    )
}

// WF-4 first install step 2: adopt MIP state after the mrpack import settles
// invoke('plugin:ymcl|ymcl_pack_adopt_state', { instanceId, serverId, packId, version, channel, selected, force? })
// force=true takes the instance over from a different pack id (republish reinstall).
#[tauri::command]
pub async fn ymcl_pack_adopt_state(
    instance_id: String,
    server_id: String,
    pack_id: String,
    version: String,
    channel: Option<String>,
    selected: Vec<String>,
    force: Option<bool>,
) -> Result<()> {
    Ok(
        theseus::ymcl::mip::first_install::adopt_pack_state(
            &instance_id,
            &server_id,
            &pack_id,
            &version,
            channel,
            selected,
            force.unwrap_or(false),
        )
        .await?,
    )
}

// Feature catalog + current selection for a MIP-managed instance
// invoke('plugin:ymcl|ymcl_pack_features', { instanceId })
#[tauri::command]
pub async fn ymcl_pack_features(
    instance_id: String,
) -> Result<theseus::ymcl::mip::update::YmclPackFeatures> {
    Ok(theseus::ymcl::mip::update::pack_features(&instance_id).await?)
}

// Apply a new optional-feature selection to a MIP-managed instance
// invoke('plugin:ymcl|ymcl_pack_set_features', { instanceId, selected })
#[tauri::command]
pub async fn ymcl_pack_set_features(
    instance_id: String,
    selected: Vec<String>,
) -> Result<theseus::ymcl::mip::update::YmclUpdateResult> {
    Ok(theseus::ymcl::mip::update::set_features(&instance_id, selected).await?)
}

// invoke('plugin:ymcl|ymcl_mip_servers')
#[tauri::command]
pub async fn ymcl_mip_servers() -> Result<Vec<theseus::ymcl::mip::update::MipServerBinding>> {
    Ok(theseus::ymcl::mip::publish::list_servers().await?)
}

//! YAP chrome endpoints (YAP §6.5 home capability, admin side): read and
//! save the domain-hosted home card layout from the launcher's designer.

use reqwest::Method;

use super::manifest::YAP_API_BASE;
use crate::State;
use crate::state::ymcl_session;
use crate::util::fetch::fetch_advanced;

pub fn chrome_home_url(origin: &str) -> String {
    format!("{origin}{YAP_API_BASE}/chrome/home")
}

/// Fetches the current home layout config from the active domain.
pub async fn get_home_config() -> crate::Result<serde_json::Value> {
    let state = State::get().await?;
    let active = super::registry::active_domain_id(&state.pool).await?;
    if active == super::registry::PERSONAL_DOMAIN_ID {
        return Err(crate::ErrorKind::OtherError(
            "The home designer requires an active domain".to_string(),
        )
        .into());
    }
    let origin = super::registry::domain_origin(&active).await?;
    let bytes =
        authenticated_request(&state, &origin, Method::GET, None).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Saves the home layout config; the adapter rebuilds its manifest and
/// notifies members (YAP §6.10 manifest.updated).
pub async fn put_home_config(config: serde_json::Value) -> crate::Result<()> {
    let state = State::get().await?;
    let active = super::registry::active_domain_id(&state.pool).await?;
    if active == super::registry::PERSONAL_DOMAIN_ID {
        return Err(crate::ErrorKind::OtherError(
            "The home designer requires an active domain".to_string(),
        )
        .into());
    }
    let origin = super::registry::domain_origin(&active).await?;
    authenticated_request(&state, &origin, Method::POST, Some(config)).await?;
    Ok(())
}

async fn authenticated_request(
    state: &State,
    origin: &str,
    method: Method,
    body: Option<serde_json::Value>,
) -> crate::Result<bytes::Bytes> {
    let Some(session) = ymcl_session::get(
        &super::registry::active_domain_id(&state.pool).await?,
        &state.pool,
    )
    .await?
    else {
        return Err(crate::ErrorKind::OtherError(
            "Sign in to this domain to manage its home layout".to_string(),
        )
        .into());
    };
    fetch_advanced(
        method,
        &chrome_home_url(origin),
        None,
        body,
        Some(("Authorization", session.access_token.as_str())),
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await
}

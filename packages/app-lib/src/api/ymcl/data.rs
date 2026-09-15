//! YAP data envelope fetch (YAP §6.6): the adapter returns declarative
//! records plus page/item actions; the launcher renders and executes them.
//! The envelope is passed through as JSON — renderers own schemaVersion.

use reqwest::Method;

use super::manifest::YAP_API_BASE;
use crate::State;
use crate::state::ymcl_session;
use crate::util::fetch::fetch_advanced;

pub fn data_url(
    origin: &str,
    provider_code: &str,
    source_code: &str,
) -> String {
    format!("{origin}{YAP_API_BASE}/data/{provider_code}/{source_code}")
}

/// Fetches a data envelope for the active domain. Requires an active
/// session when the domain's data sources are protected (adapter decides).
pub async fn fetch_data(
    provider_code: &str,
    source_code: &str,
    query: Option<String>,
) -> crate::Result<serde_json::Value> {
    let state = State::get().await?;
    let active = super::registry::active_domain_id(&state.pool).await?;
    if active == super::registry::PERSONAL_DOMAIN_ID {
        return Err(crate::ErrorKind::OtherError(
            "Data sources are only available while a domain is active"
                .to_string(),
        )
        .into());
    }
    let origin = super::registry::domain_origin(&active).await?;

    let mut url = data_url(&origin, provider_code, source_code);
    if let Some(query) = query {
        if !query.is_empty() {
            url.push('?');
            url.push_str(&query);
        }
    }

    let session = ymcl_session::get(&active, &state.pool).await?;
    let header = session
        .as_ref()
        .map(|session| ("Authorization", session.access_token.clone()));

    let bytes = fetch_advanced(
        Method::GET,
        &url,
        None,
        None,
        header.as_ref().map(|(name, value)| (*name, value.as_str())),
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn action_url(
    origin: &str,
    provider_code: &str,
    action_code: &str,
) -> String {
    format!("{origin}{YAP_API_BASE}/action/{provider_code}/{action_code}")
}

/// Executes a `server:*` action against the active domain's adapter
/// (YAP §6.7) and returns the adapter's response envelope, if any.
pub async fn execute_action(
    provider_code: &str,
    action_code: &str,
    params: serde_json::Value,
) -> crate::Result<serde_json::Value> {
    let state = State::get().await?;
    let active = super::registry::active_domain_id(&state.pool).await?;
    if active == super::registry::PERSONAL_DOMAIN_ID {
        return Err(crate::ErrorKind::OtherError(
            "Actions are only available while a domain is active".to_string(),
        )
        .into());
    }
    let origin = super::registry::domain_origin(&active).await?;
    let session = ymcl_session::get(&active, &state.pool).await?;
    let header = session
        .as_ref()
        .map(|session| ("Authorization", session.access_token.clone()));
    let bytes = fetch_advanced(
        Method::POST,
        &action_url(&origin, provider_code, action_code),
        None,
        Some(params),
        header.as_ref().map(|(name, value)| (*name, value.as_str())),
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    Ok(serde_json::from_slice(&bytes)?)
}

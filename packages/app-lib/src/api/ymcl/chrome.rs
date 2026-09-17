//! YAP chrome endpoints (YAP §6.5 home capability, admin side): read and
//! save the domain-hosted home card layout from the launcher's designer.

use reqwest::Method;

use super::manifest::YAP_API_BASE;
use crate::State;

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
        super::auth::domain_request(&state, &active, Method::GET, &chrome_home_url(&origin), None)
            .await?;
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
    super::auth::domain_request(
        &state,
        &active,
        Method::PUT,
        &chrome_home_url(&origin),
        Some(config),
    )
    .await?;
    Ok(())
}

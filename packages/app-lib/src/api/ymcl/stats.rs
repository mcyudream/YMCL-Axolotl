//! Domain launch stats reporting (YAP extension, adapter `POST /v1/stats/launch`).
//! Fire-and-forget: the launcher reports once after a domain (MIP-managed)
//! instance starts successfully. User attribution is resolved server-side from
//! the domain session, so the payload carries no identity. Failures are logged
//! and swallowed — stats must never affect launching.

use crate::state::State;
use serde_json::json;
use std::time::Duration;

/// Best-effort wrapper used by the launch path: swallows every error and
/// bounds the whole attempt so a slow domain cannot keep a task alive.
pub async fn report_launch(
    instance_id: String,
    quick_play: crate::instance::QuickPlayType,
) {
    let attempt = tokio::time::timeout(
        Duration::from_secs(10),
        try_report_launch(&instance_id, &quick_play),
    );
    if let Err(error) = attempt.await {
        tracing::debug!("Domain launch stats report skipped: {error}");
    }
}

async fn try_report_launch(
    instance_id: &str,
    quick_play: &crate::instance::QuickPlayType,
) -> crate::Result<()> {
    let state = State::get().await?;
    let active = super::registry::active_domain_id(&state.pool).await?;
    if active == super::registry::PERSONAL_DOMAIN_ID {
        return Ok(());
    }
    // Only MIP-managed instances belong to a domain; everything else is a
    // local install the domain never distributed.
    let instance_dir = crate::api::instance::get_full_path(instance_id).await?;
    let Some(pack_state) = super::mip::state::load(&instance_dir).await? else {
        return Ok(());
    };
    let origin = super::registry::domain_origin(&active).await?;
    let url = format!("{origin}{}/stats/launch", super::manifest::YAP_API_BASE);
    let quick_play_label = match quick_play {
        crate::instance::QuickPlayType::Server(_) => {
            serde_json::json!("server")
        }
        crate::instance::QuickPlayType::Singleplayer(_) => {
            serde_json::json!("singleplayer")
        }
        crate::instance::QuickPlayType::None => serde_json::Value::Null,
    };
    let binding = pack_state.binding.as_ref();
    let body = json!({
        "packId": pack_state.pack_id,
        "packVersion": pack_state.version,
        "serverId": binding.map(|binding| binding.server_id.clone()),
        "seasonId": binding.and_then(|binding| binding.season_id.clone()),
        "quickPlay": quick_play_label,
        "launcherVersion": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    });
    // `domain_request_opt`: send even without a live session (the adapter
    // requires one for attribution and will reject anonymous reports — that
    // rejection is an expected, silently-ignored outcome for logged-out play).
    super::auth::domain_request_opt(
        &state,
        &active,
        reqwest::Method::POST,
        &url,
        Some(body),
    )
    .await?;
    Ok(())
}

//! Domain registry persistence and high-level domain operations.
//!
//! A domain is one yda deployment; the personal domain is virtual (id
//! `personal`) and always exists without touching this registry.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use super::client;
use super::client::fetch_capabilities;
use super::manifest::validate_protocol_version;
pub use super::manifest::{YmclCapabilities, YmclManifest};
use crate::State;

pub const PERSONAL_DOMAIN_ID: &str = "personal";
const PERSONAL_DOMAIN_LABEL: &str = "个人域";

#[derive(Serialize, Clone, Debug)]
pub struct YmclDomainSummary {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    pub is_personal: bool,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<YmclCapabilities>,
}

#[derive(Serialize, Clone, Debug)]
pub struct YmclDomainsState {
    pub active_domain_id: String,
    pub domains: Vec<YmclDomainSummary>,
}

#[derive(Deserialize, sqlx::FromRow)]
struct DomainRow {
    id: String,
    origin: String,
    display_name: String,
    logo_url: Option<String>,
    capabilities_json: Option<String>,
    manifest_json: Option<String>,
}

/// Normalizes user input into a canonical origin used for storage and all
/// protocol calls: scheme defaults to https, trailing slashes are removed.
pub fn normalize_origin(input: &str) -> crate::Result<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(crate::ErrorKind::OtherError(
            "Domain address cannot be empty".to_string(),
        )
        .into());
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let url = url::Url::parse(&with_scheme).map_err(|e| {
        crate::ErrorKind::OtherError(format!("Invalid domain address: {e}"))
    })?;
    if url.host_str().is_none() {
        return Err(crate::ErrorKind::OtherError(
            "Invalid domain address: missing host".to_string(),
        )
        .into());
    }
    let mut normalized =
        format!("{}://{}", url.scheme(), url.host_str().unwrap_or_default());
    if let Some(port) = url.port() {
        normalized.push_str(&format!(":{port}"));
    }
    let normalized = normalized.trim_end_matches('/').to_string();
    Ok(normalized)
}

pub fn domain_id_for_origin(origin: &str) -> String {
    let digest = Sha256::digest(origin.as_bytes());
    digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

/// Adapters without a configured display name report their own base URL as
/// `domain.name`, which would surface as `http://host:port` in the UI — and
/// worse, that URL can differ from the origin the user actually joined.
/// A URL-shaped stored name is therefore not a usable label: show the joined
/// origin (scheme stripped) instead until the domain configures a real name.
fn display_label(stored_name: &str, origin: &str) -> String {
    let trimmed = stored_name.trim();
    if trimmed.is_empty() || trimmed.contains("://") {
        origin
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .to_string()
    } else {
        trimmed.to_string()
    }
}

fn summary_from_row(
    row: &DomainRow,
    active_domain_id: &str,
) -> YmclDomainSummary {
    YmclDomainSummary {
        id: row.id.clone(),
        origin: Some(row.origin.clone()),
        display_name: display_label(&row.display_name, &row.origin),
        logo_url: row.logo_url.clone(),
        is_personal: false,
        is_active: row.id == active_domain_id,
        capabilities: row
            .capabilities_json
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok()),
    }
}

fn personal_summary(active_domain_id: &str) -> YmclDomainSummary {
    YmclDomainSummary {
        id: PERSONAL_DOMAIN_ID.to_string(),
        origin: None,
        display_name: PERSONAL_DOMAIN_LABEL.to_string(),
        logo_url: None,
        is_personal: true,
        is_active: active_domain_id == PERSONAL_DOMAIN_ID,
        capabilities: None,
    }
}

async fn read_active_domain_id(exec: &SqlitePool) -> crate::Result<String> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT active_domain_id FROM ymcl_state WHERE id = 0")
            .fetch_optional(exec)
            .await?;
    Ok(row
        .map(|r| r.0)
        .unwrap_or_else(|| PERSONAL_DOMAIN_ID.to_string()))
}

pub async fn set_active_domain_id(
    id: &str,
    exec: &SqlitePool,
) -> crate::Result<()> {
    sqlx::query("UPDATE ymcl_state SET active_domain_id = $1 WHERE id = 0")
        .bind(id)
        .execute(exec)
        .await?;
    Ok(())
}

async fn list_domain_rows(exec: &SqlitePool) -> crate::Result<Vec<DomainRow>> {
    let rows: Vec<DomainRow> = sqlx::query_as(
        "SELECT id, origin, display_name, logo_url, capabilities_json, manifest_json \
         FROM ymcl_domains ORDER BY added_at ASC",
    )
    .fetch_all(exec)
    .await?;
    Ok(rows)
}

async fn domains_state_impl(
    pool: &SqlitePool,
) -> crate::Result<YmclDomainsState> {
    let active = read_active_domain_id(pool).await?;
    let rows = list_domain_rows(pool).await?;
    let mut domains = vec![personal_summary(&active)];
    for row in &rows {
        domains.push(summary_from_row(row, &active));
    }
    Ok(YmclDomainsState {
        active_domain_id: active,
        domains,
    })
}

/// The active domain id; `personal` when no domain is active.
pub async fn active_domain_id(pool: &SqlitePool) -> crate::Result<String> {
    read_active_domain_id(pool).await
}

pub async fn domains_state() -> crate::Result<YmclDomainsState> {
    let state = State::get().await?;
    domains_state_impl(&state.pool).await
}

/// Resolves the canonical origin of a joined domain.
pub async fn domain_origin(domain_id: &str) -> crate::Result<String> {
    let state = State::get().await?;
    let row: Option<(String,)> =
        sqlx::query_as("SELECT origin FROM ymcl_domains WHERE id = $1")
            .bind(domain_id)
            .fetch_optional(&state.pool)
            .await?;
    row.map(|r| r.0).ok_or_else(|| {
        crate::ErrorKind::OtherError("Domain not found".to_string()).into()
    })
}

/// Loads the cached capabilities of a joined domain, re-probing on absence.
pub async fn domain_capabilities(
    domain_id: &str,
) -> crate::Result<YmclCapabilities> {
    let state = State::get().await?;
    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT capabilities_json FROM ymcl_domains WHERE id = $1",
    )
    .bind(domain_id)
    .fetch_optional(&state.pool)
    .await?;
    if let Some((Some(json),)) = row {
        if let Ok(capabilities) =
            serde_json::from_str::<YmclCapabilities>(&json)
        {
            return Ok(capabilities);
        }
    }
    refresh_capabilities(domain_id).await
}

/// Re-probes a domain's capabilities and updates the cached row. Called
/// when the cached snapshot is absent or lacks a capability a caller needs
/// (e.g. mip appearing after the adapter gains the distribution face), so
/// adapter-side rollouts self-heal without a manual domain refresh.
pub async fn refresh_capabilities(
    domain_id: &str,
) -> crate::Result<YmclCapabilities> {
    let state = State::get().await?;
    let origin = domain_origin(domain_id).await?;
    let capabilities =
        fetch_capabilities(&origin, &state.api_semaphore, &state.pool).await?;
    sqlx::query("UPDATE ymcl_domains SET capabilities_json = $1 WHERE id = $2")
        .bind(&serde_json::to_string(&capabilities)?)
        .bind(domain_id)
        .execute(&state.pool)
        .await?;
    Ok(capabilities)
}

/// Adds a domain by probing its adapter capabilities. Fails when the origin is
/// already registered, unreachable, or speaks an unsupported protocol version.
pub async fn add_domain(origin_input: &str) -> crate::Result<YmclDomainsState> {
    let origin = normalize_origin(origin_input)?;
    let id = domain_id_for_origin(&origin);
    let state = State::get().await?;

    let existing = sqlx::query("SELECT id FROM ymcl_domains WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?;
    if existing.is_some() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Domain {origin} has already been added"
        ))
        .into());
    }

    let capabilities =
        client::fetch_capabilities(&origin, &state.api_semaphore, &state.pool)
            .await
            .map_err(|e| {
                crate::ErrorKind::OtherError(format!(
                    "Could not reach domain {origin}: {e}"
                ))
            })?;
    validate_protocol_version(capabilities.protocol_version)?;

    let display_name = capabilities
        .domain
        .as_ref()
        .map(|d| d.name.clone())
        .unwrap_or_else(|| origin.clone());
    let logo_url = capabilities
        .domain
        .as_ref()
        .and_then(|d| d.logo_url.clone());
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO ymcl_domains (id, origin, display_name, logo_url, capabilities_json, added_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&id)
    .bind(&origin)
    .bind(&display_name)
    .bind(&logo_url)
    .bind(&serde_json::to_string(&capabilities)?)
    .bind(now)
    .execute(&state.pool)
    .await?;

    domains_state_impl(&state.pool).await
}

pub async fn remove_domain(id: &str) -> crate::Result<YmclDomainsState> {
    if id == PERSONAL_DOMAIN_ID {
        return Err(crate::ErrorKind::OtherError(
            "The personal domain cannot be removed".to_string(),
        )
        .into());
    }
    let state = State::get().await?;
    sqlx::query("DELETE FROM ymcl_domains WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    let active = read_active_domain_id(&state.pool).await?;
    if active == id {
        set_active_domain_id(PERSONAL_DOMAIN_ID, &state.pool).await?;
    }
    domains_state_impl(&state.pool).await
}

/// Activates a domain (or the personal domain). Domains without a cached
/// manifest fetch one immediately so navigation is ready on activation.
pub async fn activate_domain(id: &str) -> crate::Result<YmclDomainsState> {
    let state = State::get().await?;
    if id != PERSONAL_DOMAIN_ID {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT origin FROM ymcl_domains WHERE id = $1")
                .bind(id)
                .fetch_optional(&state.pool)
                .await?;
        let origin = row.map(|r| r.0).ok_or_else(|| {
            crate::ErrorKind::OtherError("Domain not found".to_string())
        })?;

        let has_manifest: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT manifest_json FROM ymcl_domains WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
        if has_manifest.and_then(|r| r.0).is_none() {
            let manifest = client::fetch_manifest(
                &origin,
                &state.api_semaphore,
                &state.pool,
            )
            .await
            .map_err(|e| {
                crate::ErrorKind::OtherError(format!(
                    "Could not load domain manifest: {e}"
                ))
            })?;
            validate_protocol_version(manifest.protocol_version)?;
            store_manifest(id, &manifest, &state.pool).await?;
        }
    }
    let now = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE ymcl_domains SET last_active_at = $1 WHERE id = $2")
        .bind(now)
        .bind(id)
        .execute(&state.pool)
        .await?;
    set_active_domain_id(id, &state.pool).await?;
    domains_state_impl(&state.pool).await
}

async fn store_manifest(
    id: &str,
    manifest: &YmclManifest,
    pool: &SqlitePool,
) -> crate::Result<()> {
    let json = serde_json::to_string(manifest)?;
    sqlx::query("UPDATE ymcl_domains SET manifest_json = $1 WHERE id = $2")
        .bind(&json)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn active_domain_row(
    pool: &SqlitePool,
) -> crate::Result<Option<DomainRow>> {
    let active = read_active_domain_id(pool).await?;
    if active == PERSONAL_DOMAIN_ID {
        return Ok(None);
    }
    let rows = list_domain_rows(pool).await?;
    Ok(rows.into_iter().find(|row| row.id == active))
}

/// The manifest of the active domain, or `None` for the personal domain.
/// Serves the cached manifest when present; only fetches when nothing has
/// been cached yet so an unreachable domain degrades to its last known state.
pub async fn active_manifest() -> crate::Result<Option<YmclManifest>> {
    let state = State::get().await?;
    let Some(row) = active_domain_row(&state.pool).await? else {
        return Ok(None);
    };
    if let Some(json) = row.manifest_json.as_deref() {
        if let Ok(manifest) = serde_json::from_str::<YmclManifest>(json) {
            return Ok(Some(manifest));
        }
    }
    let manifest = match fetch_manifest_authed(&row.id, &row.origin, &state).await {
        Ok(manifest) => manifest,
        Err(error) => {
            tracing::warn!(
                "Failed to fetch manifest for domain {}: {error}",
                row.origin
            );
            return Ok(None);
        }
    };
    store_manifest(&row.id, &manifest, &state.pool).await?;
    Ok(Some(manifest))
}

/// Prefer session-authenticated manifest (full nav/pages per user RBAC);
/// fall back to anonymous fetch when no session exists yet.
async fn fetch_manifest_authed(
    domain_id: &str,
    origin: &str,
    state: &State,
) -> crate::Result<YmclManifest> {
    match client::fetch_manifest_for_domain(domain_id).await {
        Ok(manifest) => Ok(manifest),
        Err(auth_error) => {
            tracing::debug!(
                "Authed manifest fetch failed for {domain_id} ({auth_error}); trying anonymous"
            );
            client::fetch_manifest(origin, &state.api_semaphore, &state.pool).await
        }
    }
}

/// Forces a re-fetch of the active domain's manifest. Also re-probes the
/// domain's capabilities so admin-side changes (display name, logo, auth
/// endpoints) propagate without removing and re-adding the domain.
pub async fn refresh_manifest() -> crate::Result<Option<YmclManifest>> {
    let state = State::get().await?;
    let Some(row) = active_domain_row(&state.pool).await? else {
        return Ok(None);
    };
    let manifest = fetch_manifest_authed(&row.id, &row.origin, &state).await?;
    validate_protocol_version(manifest.protocol_version)?;
    store_manifest(&row.id, &manifest, &state.pool).await?;

    if let Ok(capabilities) =
        client::fetch_capabilities(&row.origin, &state.api_semaphore, &state.pool)
            .await
    {
        let name = capabilities
            .domain
            .as_ref()
            .map(|domain| domain.name.clone())
            .unwrap_or_else(|| row.display_name.clone());
        let logo_url = capabilities.domain.as_ref().and_then(|domain| {
            domain.logo_url.clone().or_else(|| row.logo_url.clone())
        });
        sqlx::query(
            "UPDATE ymcl_domains \
             SET capabilities_json = $1, display_name = $2, logo_url = $3 \
             WHERE id = $4",
        )
        .bind(&serde_json::to_string(&capabilities)?)
        .bind(name)
        .bind(logo_url)
        .bind(&row.id)
        .execute(&state.pool)
        .await?;
    }
    Ok(Some(manifest))
}

//! Per-domain YMCL session persistence. One row per joined domain; the
//! personal domain never has a session. Mirrors the `mr_auth` storage
//! pattern: SQLite-backed, refreshed by the API layer.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct YmclContextOption {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct YmclSessionContext {
    #[serde(default)]
    pub dept: Option<YmclContextOption>,
    #[serde(default)]
    pub role: Option<YmclContextOption>,
    #[serde(default)]
    pub available_depts: Vec<YmclContextOption>,
    #[serde(default)]
    pub available_roles: Vec<YmclContextOption>,
}

/// Normalized session info as returned by `GET /v1/session` (YAP §6.4).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct YmclSessionInfo {
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub issued_via: Option<String>,
    #[serde(default)]
    pub context: Option<YmclSessionContext>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YmclStoredSession {
    pub domain_id: String,
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Unix seconds; `None` for tokens without expiry information.
    #[serde(default)]
    pub expires_at: Option<i64>,
    pub session: YmclSessionInfo,
    pub updated_at: i64,
}

impl YmclStoredSession {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires| expires <= Utc::now().timestamp())
    }

	/// True when the access token is expired or will expire within `skew_secs`,
	/// so the API layer can renew it before the host rejects the next request.
	pub fn needs_refresh(&self, skew_secs: i64) -> bool {
		let Some(expires) = self.expires_at else {
			return false;
		};
		expires <= Utc::now().timestamp() + skew_secs.max(0)
	}

	pub fn can_refresh(&self) -> bool {
		self.refresh_token
			.as_deref()
			.is_some_and(|token| !token.trim().is_empty())
	}
}

pub async fn get(
    domain_id: &str,
    exec: &SqlitePool,
) -> crate::Result<Option<YmclStoredSession>> {
    let row: Option<(String, Option<String>, Option<i64>, String)> =
        sqlx::query_as(
            "SELECT access_token, refresh_token, expires_at, session_json \
         FROM ymcl_sessions WHERE domain_id = $1",
        )
        .bind(domain_id)
        .fetch_optional(exec)
        .await?;

    let Some((access_token, refresh_token, expires_at, session_json)) = row
    else {
        return Ok(None);
    };
    let session = serde_json::from_str::<YmclSessionInfo>(&session_json)
        .unwrap_or_default();

    Ok(Some(YmclStoredSession {
        domain_id: domain_id.to_string(),
        access_token,
        refresh_token,
        expires_at,
        session,
        updated_at: Utc::now().timestamp(),
    }))
}

pub async fn upsert(
    session: &YmclStoredSession,
    exec: &SqlitePool,
) -> crate::Result<()> {
    let session_json = serde_json::to_string(&session.session)?;
    sqlx::query(
        "INSERT INTO ymcl_sessions \
         (domain_id, access_token, refresh_token, expires_at, session_json, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT(domain_id) DO UPDATE SET \
         access_token = $2, refresh_token = $3, expires_at = $4, \
         session_json = $5, updated_at = $6",
    )
    .bind(&session.domain_id)
    .bind(&session.access_token)
    .bind(&session.refresh_token)
    .bind(session.expires_at)
    .bind(&session_json)
    .bind(Utc::now().timestamp())
    .execute(exec)
    .await?;
    Ok(())
}

pub async fn remove(domain_id: &str, exec: &SqlitePool) -> crate::Result<()> {
    sqlx::query("DELETE FROM ymcl_sessions WHERE domain_id = $1")
        .bind(domain_id)
        .execute(exec)
        .await?;
    Ok(())
}

/// Every domain that still has a stored session row.
pub async fn list_domain_ids(exec: &SqlitePool) -> crate::Result<Vec<String>> {
	let rows: Vec<(String,)> =
		sqlx::query_as("SELECT domain_id FROM ymcl_sessions")
			.fetch_all(exec)
			.await?;
	Ok(rows.into_iter().map(|row| row.0).collect())
}

pub async fn update_session_info(
    domain_id: &str,
    session: &YmclSessionInfo,
    exec: &SqlitePool,
) -> crate::Result<()> {
    let session_json = serde_json::to_string(session)?;
    sqlx::query("UPDATE ymcl_sessions SET session_json = $1, updated_at = $2 WHERE domain_id = $3")
        .bind(&session_json)
        .bind(Utc::now().timestamp())
        .bind(domain_id)
        .execute(exec)
        .await?;
    Ok(())
}

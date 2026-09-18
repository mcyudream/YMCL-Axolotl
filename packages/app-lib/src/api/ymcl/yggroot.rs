//! Yggdrasil provider resolution for YMCL domains.
//!
//! A yda node exposes its Minecraft (Yggdrasil) API through one of two
//! provider plugins: the historical `authlib-injector` plugin, or the `yggc`
//! plugin. Their path shapes differ — `yggc` serves the Yggdrasil protocol
//! under a deeper root and keeps its session exchange outside that root — so
//! the launcher probes the candidates once per origin and remembers the shape
//! that answered.

use std::collections::HashMap;
use std::sync::LazyLock;

use parking_lot::Mutex;
use reqwest::Method;
use serde::Deserialize;

use crate::State;
use crate::util::fetch::fetch_json;

/// Yggdrasil endpoints of one resolved domain origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YmclYggEndpoints {
    /// Root handed to the authlib-injector javaagent as its `api_root`.
    pub yggdrasil_root: String,
    /// `POST` session-exchange endpoint; accepts `?profile=<name>`.
    pub exchange_url: String,
    /// `POST` profile-list endpoint (`?list=true`).
    pub list_url: String,
}

impl YmclYggEndpoints {
    /// `authlib-injector` plugin shape: the Yggdrasil root also carries the
    /// launcher session exchange.
    pub fn authlib_injector(origin: &str) -> Self {
        let root = format!(
            "{}/api/plugins/authlib-injector",
            origin.trim_end_matches('/')
        );
        Self {
            exchange_url: format!("{root}/launcher/exchange"),
            list_url: format!("{root}/launcher/exchange?list=true"),
            yggdrasil_root: root,
        }
    }

    /// `yggc` plugin shape: Yggdrasil protocol under `…/api/yggdrasil`, the
    /// launcher exchange kept beside that root rather than under it.
    pub fn yggc(origin: &str) -> Self {
        let origin = origin.trim_end_matches('/');
        let exchange_url =
            format!("{origin}/api/plugins/yggc/launcher/exchange");
        Self {
            yggdrasil_root: format!("{origin}/api/plugins/yggc/api/yggdrasil"),
            list_url: format!("{exchange_url}?list=true"),
            exchange_url,
        }
    }

    /// Every provider shape of an origin, in probe order.
    pub fn candidates(origin: &str) -> [Self; 2] {
        [Self::authlib_injector(origin), Self::yggc(origin)]
    }
}

/// One probe per origin per launcher session.
static YGG_ENDPOINTS: LazyLock<Mutex<HashMap<String, YmclYggEndpoints>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn origin_key(origin: &str) -> String {
    origin.trim_end_matches('/').to_string()
}

/// Yggdrasil metadata document (`GET {root}`): the one answer both providers
/// share, identified by its public signature key.
#[derive(Deserialize)]
struct YmclYggMetadata {
    #[serde(
        default,
        rename = "signaturePublickey",
        alias = "signaturePublicKey"
    )]
    signature_public_key: Option<String>,
}

impl YmclYggMetadata {
    /// Whether this is a working Yggdrasil metadata document rather than some
    /// other 2xx JSON response served at the same path.
    fn is_yggdrasil(&self) -> bool {
        self.signature_public_key
            .as_deref()
            .is_some_and(|key| !key.trim().is_empty())
    }
}

/// Probes the provider candidates of an origin and returns the first whose
/// Yggdrasil metadata document parses. `None` when neither answers, which is
/// the case for a personal domain or a node running no provider at all.
async fn probe(origin: &str) -> crate::Result<Option<YmclYggEndpoints>> {
    let state = State::get().await?;
    for candidate in YmclYggEndpoints::candidates(origin) {
        let metadata = fetch_json::<YmclYggMetadata>(
            Method::GET,
            &candidate.yggdrasil_root,
            None,
            None,
            None,
            &state.api_semaphore,
            &state.pool,
        )
        .await;
        match metadata {
            Ok(metadata) if metadata.is_yggdrasil() => {
                return Ok(Some(candidate));
            }
            // A 2xx without a signature key, a non-Yggdrasil JSON body, or a
            // transport/HTTP failure: this candidate is not the provider.
            Ok(_) | Err(_) => continue,
        }
    }
    Ok(None)
}

/// Resolves the Yggdrasil provider of a domain origin, probing at most once
/// per launcher session.
///
/// A node running neither provider (a personal domain, a plain yda site)
/// keeps the historical authlib-injector shape; that fallback is deliberately
/// not cached, so a node that gains a provider is picked up on the next call
/// instead of after a restart.
pub async fn ygg_endpoints(origin: &str) -> crate::Result<YmclYggEndpoints> {
    let key = origin_key(origin);
    if let Some(cached) = YGG_ENDPOINTS.lock().get(&key).cloned() {
        return Ok(cached);
    }
    match probe(&key).await? {
        Some(endpoints) => {
            YGG_ENDPOINTS.lock().insert(key, endpoints.clone());
            Ok(endpoints)
        }
        None => Ok(YmclYggEndpoints::authlib_injector(&key)),
    }
}

/// Drops a cached resolution so the next call probes the providers again.
pub fn invalidate_ygg_endpoints(origin: &str) {
    YGG_ENDPOINTS.lock().remove(&origin_key(origin));
}

/// Whether a failed Yggdrasil call may simply have used the wrong provider
/// shape: transport failures and non-2xx replies, except host auth rejections,
/// which no other path can fix.
pub fn is_retryable_ygg_error(error: &crate::Error) -> bool {
    match error.raw.as_ref() {
        crate::ErrorKind::HttpError { status, .. } => {
            !matches!(status, 401 | 403)
        }
        crate::ErrorKind::FetchError(_) | crate::ErrorKind::NetworkError(_) => {
            true
        }
        crate::ErrorKind::LabrinthError(error) => error
            .status
            .is_some_and(|status| !matches!(status, 401 | 403)),
        _ => false,
    }
}

/// Resolved Yggdrasil root of a joined domain, for frontend account matching.
/// `None` for the personal domain, which owns no provider.
pub async fn ygg_root_for_domain(
    domain_id: &str,
) -> crate::Result<Option<String>> {
    if domain_id == super::registry::PERSONAL_DOMAIN_ID {
        return Ok(None);
    }
    let origin = super::registry::domain_origin(domain_id).await?;
    Ok(Some(ygg_endpoints(&origin).await?.yggdrasil_root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_both_provider_shapes() {
        let [authlib, yggc] =
            YmclYggEndpoints::candidates("https://yda.example.com/");
        assert_eq!(
            authlib.yggdrasil_root,
            "https://yda.example.com/api/plugins/authlib-injector"
        );
        assert_eq!(
            authlib.exchange_url,
            "https://yda.example.com/api/plugins/authlib-injector/launcher/exchange"
        );
        assert_eq!(
            authlib.list_url,
            "https://yda.example.com/api/plugins/authlib-injector/launcher/exchange?list=true"
        );
        assert_eq!(
            yggc.yggdrasil_root,
            "https://yda.example.com/api/plugins/yggc/api/yggdrasil"
        );
        assert_eq!(
            yggc.exchange_url,
            "https://yda.example.com/api/plugins/yggc/launcher/exchange"
        );
        assert_eq!(
            yggc.list_url,
            "https://yda.example.com/api/plugins/yggc/launcher/exchange?list=true"
        );
    }

    #[test]
    fn accepts_only_metadata_with_a_signature_key() {
        let metadata: YmclYggMetadata = serde_json::from_str(
            r#"{"signaturePublickey":"abc","skinDomains":[]}"#,
        )
        .expect("yggdrasil metadata must deserialize");
        assert!(metadata.is_yggdrasil());
        let other: YmclYggMetadata =
            serde_json::from_str(r#"{"code":200,"message":"ok"}"#)
                .expect("unrelated json must deserialize");
        assert!(!other.is_yggdrasil());
        let empty: YmclYggMetadata =
            serde_json::from_str(r#"{"signaturePublickey":"  "}"#)
                .expect("blank key must deserialize");
        assert!(!empty.is_yggdrasil());
    }

    #[test]
    fn retries_only_when_another_shape_could_answer() {
        let not_found: crate::Error = crate::ErrorKind::HttpError {
            status: 404,
            method: "GET".to_string(),
            url: "https://yda.example.com/api/plugins/authlib-injector"
                .to_string(),
        }
        .into();
        assert!(is_retryable_ygg_error(&not_found));
        let rejected: crate::Error = crate::ErrorKind::HttpError {
            status: 401,
            method: "GET".to_string(),
            url: "https://yda.example.com/api/plugins/authlib-injector"
                .to_string(),
        }
        .into();
        assert!(!is_retryable_ygg_error(&rejected));
        let malformed: crate::Error = crate::ErrorKind::JSONError(
            serde_json::from_str::<i32>("nope").expect_err("not json"),
        )
        .into();
        assert!(!is_retryable_ygg_error(&malformed));
    }
}

//! YAP real-time event channel (YAP §6.10, `push` capability): a long-lived
//! SSE connection to the active domain. Events carry only locating metadata;
//! the frontend reacts by pulling (manifest refresh, data reload). The
//! channel is best-effort — the launcher falls back to polling on failure.

use std::time::Duration;

use futures::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::manifest::YAP_API_BASE;
use crate::State;
use crate::state::ymcl_session;

const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(45);
const INITIAL_BACKOFF: Duration = Duration::from_secs(2);
const MAX_BACKOFF: Duration = Duration::from_secs(120);

#[derive(Serialize, Clone, Debug)]
pub struct YmclEvent {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: serde_json::Value,
}

struct SseEvent {
    id: Option<String>,
    event_type: String,
    data: String,
}

fn parse_sse_block(block: &str) -> Option<SseEvent> {
    let mut id = None;
    let mut event_type = String::new();
    let mut data = Vec::new();
    for line in block.lines() {
        if let Some(rest) = line.strip_prefix("id:") {
            id = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("event:") {
            event_type = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("data:") {
            data.push(rest.trim_start().to_string());
        }
    }
    if data.is_empty() {
        return None;
    }
    Some(SseEvent {
        id,
        event_type,
        data: data.join("\n"),
    })
}

fn events_url(origin: &str, last_event_id: Option<&str>) -> String {
    let mut url = format!("{origin}{YAP_API_BASE}/events");
    if let Some(last) = last_event_id {
        url.push_str("?lastEventId=");
        url.push_str(&urlencoding::encode(last));
    }
    url
}

/// Task loop: connects to the active domain's event stream, forwards each
/// event as a `ymcl://event` Tauri event, reconnects with backoff and
/// `Last-Event-ID` resume while the domain stays active. Exits when the
/// personal domain becomes active or the app shuts down.
pub async fn run_event_loop(app: AppHandle) {
    let mut last_event_id: Option<String> = None;
    let mut backoff = INITIAL_BACKOFF;

    loop {
        let Some((origin, token)) = current_session(&app).await else {
            // Personal domain active (or state unavailable): idle quietly
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        };

        let client = match reqwest::Client::builder().build() {
            Ok(client) => client,
            Err(_) => {
                tokio::time::sleep(backoff).await;
                continue;
            }
        };

        let request = client
            .get(events_url(&origin, last_event_id.as_deref()))
            .header("Authorization", &token)
            .header("Accept", "text/event-stream")
            .timeout(HEARTBEAT_TIMEOUT);

        let response = match request.send().await {
            Ok(response) if response.status().is_success() => response,
            _ => {
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
                continue;
            }
        };
        backoff = INITIAL_BACKOFF;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        loop {
            match tokio::time::timeout(HEARTBEAT_TIMEOUT, stream.next()).await {
                Ok(Some(Ok(chunk))) => {
                    buffer.push_str(&String::from_utf8_lossy(&chunk));
                    while let Some(boundary) = buffer.find("\n\n") {
                        let block = buffer[..boundary].to_string();
                        buffer.drain(..boundary + 2);
                        if let Some(event) = parse_sse_block(&block) {
                            if let Some(id) = event.id.clone() {
                                last_event_id = Some(id);
                            }
                            let payload =
                                serde_json::from_str::<serde_json::Value>(
                                    &event.data,
                                )
                                .unwrap_or(serde_json::Value::Null);
                            let forwarded = YmclEvent {
                                id: last_event_id.clone(),
                                event_type: if event.event_type.is_empty() {
                                    "message".to_string()
                                } else {
                                    event.event_type
                                },
                                payload,
                            };
                            app.emit("ymcl://event", &forwarded).ok();
                        }
                    }
                }
                // heartbeat timeout, stream end, or transport error → reconnect
                _ => break,
            }
        }

        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

async fn current_session(app: &AppHandle) -> Option<(String, String)> {
    let _ = app;
    let state = State::get_if_initialized()?;
    let active = super::registry::active_domain_id(&state.pool).await.ok()?;
    if active == super::registry::PERSONAL_DOMAIN_ID {
        return None;
    }
    let origin = super::registry::domain_origin(&active).await.ok()?;
    let session = ymcl_session::get(&active, &state.pool).await.ok()??;
    Some((origin, session.access_token))
}

/// Spawns the event loop once at app startup; the loop self-manages its
/// lifecycle based on the active domain.
pub fn start_event_loop(handle: AppHandle) {
    // Called from the synchronous `.setup()` hook on the main thread, where
    // no Tokio runtime context exists — route through Tauri's managed runtime.
    tauri::async_runtime::spawn(async move {
        run_event_loop(handle).await;
    });
}

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use serde::Serialize;
use tokio::sync::broadcast;

use crate::web::events::BmuxEvent;
use crate::web::routes::AppState;

// ── Full state snapshot (sent on first connect for Story 2.3) ────────────────

#[derive(Serialize)]
struct AgentSnapshot {
    name: String,
    agent_type: String,
    model: String,
    status: String,
    tokens: u64,
    cost: f64,
    pane_id: Option<usize>,
}

#[derive(Serialize)]
struct StateSnapshot {
    #[serde(rename = "type")]
    kind: &'static str,
    session: String,
    agents: Vec<AgentSnapshot>,
    timestamp: chrono::DateTime<Utc>,
}

/// Handle one WebSocket connection.
///
/// - Sends full state snapshot on connect (Story 2.3).
/// - Subscribes to the event broadcast channel.
/// - Fans out JSON events to the client as they arrive.
/// - Supports 5+ concurrent clients via broadcast (Story 2.3).
pub async fn ws_handler(mut socket: WebSocket, state: AppState) {
    // Subscribe before sending snapshot to avoid missing events during snapshot
    let mut rx = state.events_tx.subscribe();

    // Send full state snapshot on connect (Story 2.3: reconnect gets snapshot)
    if let Ok(snapshot_json) = build_snapshot(&state).await {
        let _ = socket.send(Message::Text(snapshot_json)).await;
    }

    loop {
        tokio::select! {
            // Fan out events from the broadcast channel
            event = rx.recv() => {
                match event {
                    Ok(ev) => {
                        match serde_json::to_string(&*ev) {
                            Ok(json) => {
                                if socket.send(Message::Text(json)).await.is_err() {
                                    break; // Client disconnected
                                }
                            }
                            Err(e) => tracing::warn!("WS serialize error: {e}"),
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WS client lagged, skipped {n} events");
                        // Continue — client is slow but still connected
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }

            // Receive frames from client (ping/pong, close)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    _ => {} // Ignore text/binary from client
                }
            }
        }
    }
}

/// Build a JSON state snapshot for a newly connected WS client.
async fn build_snapshot(state: &AppState) -> Result<String, serde_json::Error> {
    let reg = state.daemon.agents.lock().await;
    let agents: Vec<AgentSnapshot> = reg
        .list()
        .into_iter()
        .map(|a| AgentSnapshot {
            name: a.name.clone(),
            agent_type: a.agent_type.clone(),
            model: a.model.clone(),
            status: a.status.label().to_string(),
            tokens: a.tokens_used,
            cost: a.cost_usd,
            pane_id: a.pane_id,
        })
        .collect();
    drop(reg);

    serde_json::to_string(&StateSnapshot {
        kind: "state_snapshot",
        session: state.daemon.session_name.clone(),
        agents,
        timestamp: Utc::now(),
    })
}

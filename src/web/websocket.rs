use axum::extract::ws::{Message, WebSocket};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::web::events::BmuxEvent;
use crate::web::routes::{AgentInfo as AgentJson, AppState, ContextEntryJson, TaskJson};

/// Handle one WebSocket connection.
///
/// - Sends current state as individual events on connect (Story 2.3).
/// - Subscribes to the event broadcast channel.
/// - Fans out JSON events as they arrive.
pub async fn ws_handler(mut socket: WebSocket, state: AppState) {
    // Subscribe before sending snapshot to avoid missing events
    let mut rx = state.events_tx.subscribe();

    // Send current state as individual typed events the store already handles.
    // This means the frontend doesn't need a special "state_snapshot" handler —
    // it just processes agent_spawned, context_updated, task_created etc.
    send_initial_state(&mut socket, &state).await;

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(ev) => {
                        match serde_json::to_string(&*ev) {
                            Ok(json) => {
                                if socket.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => tracing::warn!("WS serialize error: {e}"),
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WS client lagged, skipped {n} events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Send the current daemon state as individual BmuxEvent messages.
/// The frontend store handles these the same way as live events.
async fn send_initial_state(socket: &mut WebSocket, state: &AppState) {
    // Agents → one agent_spawned per agent
    {
        let reg = state.daemon.agents.lock().await;
        for a in reg.list() {
            let event = BmuxEvent::AgentSpawned {
                agent: AgentJson {
                    id: a.name.clone(),
                    name: a.name.clone(),
                    agent_type: a.agent_type.clone(),
                    model: a.model.clone(),
                    status: a.status.label().to_string(),
                    tokens_used: a.tokens_used,
                    cost_usd: a.cost_usd,
                    uptime_seconds: a.uptime_seconds(),
                    pane_id: a.pane_id,
                    last_task: a.last_task.clone(),
                    spawned_at: {
                        let elapsed = a.spawned_at.elapsed();
                        (chrono::Utc::now() - chrono::Duration::from_std(elapsed).unwrap_or_default()).to_rfc3339()
                    },
                },
            };
            if let Ok(json) = serde_json::to_string(&event) {
                if socket.send(Message::Text(json)).await.is_err() {
                    return;
                }
            }
        }
    }

    // Context → one context_updated per entry
    if let Ok(entries) = state.daemon.context.list() {
        for (key, value) in entries {
            let event = BmuxEvent::ContextUpdated {
                entry: ContextEntryJson {
                    key,
                    value,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    session: state.daemon.session_name.clone(),
                },
            };
            if let Ok(json) = serde_json::to_string(&event) {
                if socket.send(Message::Text(json)).await.is_err() {
                    return;
                }
            }
        }
    }

    // Tasks → one task_created per task
    {
        let tasks = state.daemon.task_router.list_tasks().await;
        for t in tasks {
            let event = BmuxEvent::TaskCreated {
                task: TaskJson {
                    id: t.id,
                    from_agent: "user".to_string(),
                    to_agent: t.destination,
                    content: t.content,
                    status: t.status.to_string(),
                    submitted_at: t.submitted_at.to_rfc3339(),
                    completed_at: t.result.as_ref().map(|r| r.completed_at.to_rfc3339()),
                    cost_usd: t.result.as_ref().map(|r| r.cost_usd),
                },
            };
            if let Ok(json) = serde_json::to_string(&event) {
                if socket.send(Message::Text(json)).await.is_err() {
                    return;
                }
            }
        }
    }
}

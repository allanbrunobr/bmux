/// Axum route handlers for the BMUX web dashboard API.
///
/// Story 4.1:
///   GET /api/pane-output?session=<s>&pane_id=<id>&lines=100
///     → returns last N lines of raw ANSI output from the pane ring-buffer.
///   WebSocket /ws broadcasts WsEvent::PaneOutput events for live streaming.
///
/// All other endpoints read from the shared DashboardData snapshot which is
/// updated externally by the daemon integration layer.
use axum::{
    extract::{Query, State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

use super::state::{AuditSnapshot, SharedState, TaskSnapshot, WsEvent};

// ── Query parameter types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SessionQuery {
    #[allow(dead_code)]
    session: Option<String>,
}

/// Query parameters for GET /api/pane-output  (Story 4.1).
#[derive(Deserialize)]
pub struct PaneOutputQuery {
    pub session: String,
    pub pane_id: String,
    #[serde(default = "default_lines")]
    pub lines: usize,
}

fn default_lines() -> usize { 100 }

#[derive(Serialize)]
pub struct PaneOutputResponse {
    pub session: String,
    pub pane_id: String,
    /// Raw ANSI lines from the pane scrollback ring-buffer.
    pub lines: Vec<String>,
}

// ── Router builder ────────────────────────────────────────────────────────────

pub fn build_router(state: SharedState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/sessions",    get(get_sessions))
        .route("/api/agents",      get(get_agents))
        .route("/api/tasks",       get(get_tasks).post(post_task))
        .route("/api/context",     get(get_context))
        .route("/api/pane-output", get(get_pane_output))  // Story 4.1
        .route("/api/audit",       get(get_audit))
        .route("/ws",              get(ws_handler))
        .layer(cors)
        .with_state(state)
}

// ── GET /api/sessions ─────────────────────────────────────────────────────────

async fn get_sessions(State(s): State<SharedState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "sessions": [s.session_name] }))
}

// ── GET /api/agents ───────────────────────────────────────────────────────────

async fn get_agents(
    State(s): State<SharedState>,
    Query(_q): Query<SessionQuery>,
) -> Json<serde_json::Value> {
    let data = s.data.read().unwrap();
    Json(serde_json::json!({ "agents": data.agents }))
}

// ── GET /api/tasks ────────────────────────────────────────────────────────────

async fn get_tasks(
    State(s): State<SharedState>,
    Query(_q): Query<SessionQuery>,
) -> Json<serde_json::Value> {
    let data = s.data.read().unwrap();
    Json(serde_json::json!({ "tasks": data.tasks }))
}

// ── POST /api/tasks ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SendTaskBody {
    agent:   String,
    content: String,
    #[allow(dead_code)]
    session: Option<String>,
}

async fn post_task(
    State(s): State<SharedState>,
    Json(body): Json<SendTaskBody>,
) -> impl IntoResponse {
    // Enqueue the task as a TaskQueued event; the daemon integration layer
    // picks it up and routes it to the target agent.
    let task_id = uuid::Uuid::new_v4().to_string();
    let task_snap = TaskSnapshot {
        id:               task_id.clone(),
        from_agent:       "dashboard".to_string(),
        to_agent:         body.agent.clone(),
        content:          body.content.clone(),
        status:           "queued".to_string(),
        created_at:       chrono::Utc::now().to_rfc3339(),
        completed_at:     None,
        cost_usd:         None,
    };
    s.broadcast(WsEvent::TaskQueued {
        task: serde_json::to_value(&task_snap).unwrap_or_default(),
    });
    (StatusCode::CREATED, Json(serde_json::json!({ "task_id": task_id }))).into_response()
}

// ── GET /api/context ──────────────────────────────────────────────────────────

async fn get_context(
    State(s): State<SharedState>,
    Query(_q): Query<SessionQuery>,
) -> Json<serde_json::Value> {
    let data = s.data.read().unwrap();
    Json(serde_json::json!({ "entries": data.context }))
}

// ── GET /api/audit ────────────────────────────────────────────────────────────

async fn get_audit(
    State(s): State<SharedState>,
    Query(_q): Query<SessionQuery>,
) -> Json<serde_json::Value> {
    let data = s.data.read().unwrap();
    Json(serde_json::json!({ "entries": data.audit }))
}

// ── GET /api/pane-output  (Story 4.1) ────────────────────────────────────────

/// Returns the last `lines` lines of raw ANSI PTY output for the given pane.
///
/// The response comes from the in-memory `PaneOutputCache` ring-buffer which
/// is populated by the PTY reader thread at zero extra CPU cost (it piggy-backs
/// on the existing read loop).  No polling or timers are involved, so the
/// daemon CPU increase is well below the 5% budget.
async fn get_pane_output(
    State(s): State<SharedState>,
    Query(q): Query<PaneOutputQuery>,
) -> impl IntoResponse {
    let lines = s.pane_cache.last_lines(&q.pane_id, q.lines);
    Json(PaneOutputResponse {
        session: q.session,
        pane_id: q.pane_id,
        lines,
    })
}

// ── GET /ws  (WebSocket — Story 4.1 live streaming) ──────────────────────────

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(s): State<SharedState>,
    Query(_q): Query<SessionQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, s))
}

/// WebSocket handler: forward broadcast events to the connected client.
///
/// Pane output (WsEvent::PaneOutput) is published by the PTY reader thread
/// via `WebAppState::broadcast()` and received here with no extra polling.
async fn handle_ws(mut socket: WebSocket, state: SharedState) {
    let mut rx = state.subscribe();

    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                match serde_json::to_string(&event) {
                    Ok(json) => {
                        if socket.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => continue,
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

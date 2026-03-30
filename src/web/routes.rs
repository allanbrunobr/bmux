use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::tui::server::DaemonState;
use crate::web::events::BmuxEvent;
use crate::web::websocket::ws_handler;

// ── Shared Axum state ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub daemon: Arc<DaemonState>,
    pub events_tx: broadcast::Sender<Arc<BmuxEvent>>,
}

// ── Query param helpers ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SessionParam {
    pub session: Option<String>,
}

// ── Response shapes ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SessionInfo {
    pub name: String,
    pub pid: u32,
    pub window_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct AgentInfo {
    pub name: String,
    pub agent_type: String,
    pub model: String,
    pub status: String,
    pub tokens: u64,
    pub cost: f64,
    pub uptime_seconds: u64,
    pub pane_id: Option<usize>,
    pub last_task: Option<String>,
}

#[derive(Serialize)]
pub struct ContextEntryJson {
    pub key: String,
    pub value: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

#[derive(Serialize)]
pub struct TaskJson {
    pub id: String,
    pub destination: String,
    pub content: String,
    pub status: String,
    pub submitted_at: String,
    pub cost: Option<f64>,
}

#[derive(Serialize)]
pub struct MetricsJson {
    pub total_tokens: u64,
    pub total_cost: f64,
    pub active_agents: usize,
    pub completed_tasks: usize,
}

#[derive(Deserialize)]
pub struct PostTaskBody {
    pub session: String,
    pub agent: Option<String>,
    pub content: String,
}

#[derive(Serialize)]
pub struct PostTaskResponse {
    pub task_id: String,
    pub status: String,
}

// ── Helper: validate session param ───────────────────────────────────────────

/// Returns Err(404) if the given session name doesn't match this daemon's session.
fn require_session<'a>(
    param: Option<&'a str>,
    daemon_session: &'a str,
) -> Result<(), Response> {
    match param {
        Some(s) if s != daemon_session => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Unknown session '{s}'")})),
        )
            .into_response()),
        _ => Ok(()),
    }
}

// ── Story 1.1: GET /api/sessions ─────────────────────────────────────────────

pub async fn get_sessions(State(state): State<AppState>) -> impl IntoResponse {
    use crate::tui::session::list_sessions;

    let sessions: Vec<SessionInfo> = list_sessions()
        .into_iter()
        .map(|m| SessionInfo {
            name: m.name,
            pid: m.pid,
            window_count: m.window_count,
            created_at: m.created_at,
        })
        .collect();

    // Always include our own session if missing (race between write and read)
    let own = &state.daemon.session_name;
    let has_own = sessions.iter().any(|s| &s.name == own);
    let mut sessions = sessions;
    if !has_own {
        let window_count = state.daemon.session.lock().await.window_count();
        sessions.push(SessionInfo {
            name: own.clone(),
            pid: std::process::id(),
            window_count,
            created_at: Utc::now(),
        });
    }

    Json(sessions)
}

// ── Story 1.2: GET /api/agents ────────────────────────────────────────────────

pub async fn get_agents(
    State(state): State<AppState>,
    Query(params): Query<SessionParam>,
) -> Response {
    if let Err(e) = require_session(params.session.as_deref(), &state.daemon.session_name) {
        return e;
    }

    let reg = state.daemon.agents.lock().await;
    let agents: Vec<AgentInfo> = reg
        .list()
        .into_iter()
        .map(|a| AgentInfo {
            name: a.name.clone(),
            agent_type: a.agent_type.clone(),
            model: a.model.clone(),
            status: a.status.label().to_string(),
            tokens: a.tokens_used,
            cost: a.cost_usd,
            uptime_seconds: a.uptime_seconds(),
            pane_id: a.pane_id,
            last_task: a.last_task.clone(),
        })
        .collect();

    Json(agents).into_response()
}

// ── Story 1.3: GET /api/context ───────────────────────────────────────────────

pub async fn get_context(
    State(state): State<AppState>,
    Query(params): Query<SessionParam>,
) -> Response {
    if let Err(e) = require_session(params.session.as_deref(), &state.daemon.session_name) {
        return e;
    }

    match state.daemon.context.dump() {
        Ok(entries) => {
            let json: Vec<ContextEntryJson> = entries
                .into_iter()
                .map(|e| ContextEntryJson {
                    key: e.key,
                    value: e.value,
                    created_at: e.created_at,
                    expires_at: e.expires_at,
                })
                .collect();
            Json(json).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// ── Story 1.3: GET /api/tasks ─────────────────────────────────────────────────

pub async fn get_tasks(
    State(state): State<AppState>,
    Query(params): Query<SessionParam>,
) -> Response {
    if let Err(e) = require_session(params.session.as_deref(), &state.daemon.session_name) {
        return e;
    }

    let tasks = state.daemon.task_router.list_tasks().await;
    let json: Vec<TaskJson> = tasks
        .into_iter()
        .map(|t| TaskJson {
            id: t.id,
            destination: t.destination,
            content: t.content,
            status: t.status.to_string(),
            submitted_at: t.submitted_at.to_rfc3339(),
            cost: t.result.as_ref().map(|r| r.cost_usd),
        })
        .collect();

    Json(json).into_response()
}

// ── Story 1.3: GET /api/metrics ───────────────────────────────────────────────

pub async fn get_metrics(
    State(state): State<AppState>,
    Query(params): Query<SessionParam>,
) -> Response {
    if let Err(e) = require_session(params.session.as_deref(), &state.daemon.session_name) {
        return e;
    }

    let reg = state.daemon.agents.lock().await;
    let stats = reg.aggregate_stats();
    drop(reg);

    let tasks = state.daemon.task_router.list_tasks().await;
    let completed_tasks = tasks
        .iter()
        .filter(|t| {
            t.status == crate::orchestration::task_router::TaskStatus::Completed
        })
        .count();

    Json(MetricsJson {
        total_tokens: stats.total_tokens,
        total_cost: stats.total_cost,
        active_agents: stats.active_agents,
        completed_tasks,
    })
    .into_response()
}

// ── Story 1.5: POST /api/tasks ────────────────────────────────────────────────

pub async fn post_task(
    State(state): State<AppState>,
    Json(body): Json<PostTaskBody>,
) -> Response {
    if body.session != state.daemon.session_name {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Unknown session '{}'", body.session)})),
        )
            .into_response();
    }

    let (task_id, status, target_agent) = if let Some(ref agent_name) = body.agent {
        match state
            .daemon
            .task_router
            .send_to_agent(agent_name, &body.content, 1)
            .await
        {
            Ok(crate::orchestration::task_router::SendResult::Dispatched { task_id }) => {
                (task_id, "dispatched".to_string(), Some(agent_name.clone()))
            }
            Ok(crate::orchestration::task_router::SendResult::Queued {
                task_id,
                agent_id,
                ..
            }) => (task_id, "queued".to_string(), Some(agent_id)),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        }
    } else {
        match state
            .daemon
            .task_router
            .send_auto(&body.content, None, None)
            .await
        {
            Ok(crate::orchestration::task_router::AutoRouteResult::Dispatched {
                task_id,
                agent_id,
                ..
            }) => (task_id, "dispatched".to_string(), Some(agent_id)),
            Ok(crate::orchestration::task_router::AutoRouteResult::AllBusy {
                task_id, ..
            }) => (task_id, "queued".to_string(), None),
            Err(e) => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        }
    };

    // Deliver to PTY if dispatched
    if let Some(ref target) = target_agent {
        if status == "dispatched" {
            let reg = state.daemon.agents.lock().await;
            if let Some(info) = reg.get(target) {
                if let Some(pane_id) = info.pane_id {
                    let mut sess = state.daemon.session.lock().await;
                    let mut input = body.content.into_bytes();
                    input.push(b'\r');
                    let _ = sess.send_input_to_pane(pane_id, &input);
                }
            }
        }
    }

    // Emit TaskDispatched event
    let _ = state.events_tx.send(Arc::new(BmuxEvent::TaskDispatched {
        session: state.daemon.session_name.clone(),
        task_id: task_id.clone(),
        agent: target_agent.unwrap_or_default(),
        content: String::new(), // omit content from event for brevity
        timestamp: Utc::now(),
    }));

    Json(PostTaskResponse {
        task_id,
        status,
    })
    .into_response()
}

// ── Story 2.1: GET /ws ────────────────────────────────────────────────────────

pub async fn get_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<SessionParam>,
) -> Response {
    // Validate session
    if let Some(ref s) = params.session {
        if s != &state.daemon.session_name {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Unknown session '{s}'")})),
            )
                .into_response();
        }
    }

    ws.on_upgrade(move |socket| ws_handler(socket, state))
}

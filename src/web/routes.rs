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
    pub agents: usize,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub model: String,
    pub status: String,
    pub tokens_used: u64,
    pub cost_usd: f64,
    pub uptime_seconds: u64,
    pub pane_id: Option<usize>,
    pub last_task: Option<String>,
    pub spawned_at: String,
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
    pub from_agent: String,
    pub to_agent: String,
    pub content: String,
    pub status: String,
    pub submitted_at: String,
    pub completed_at: Option<String>,
    pub cost_usd: Option<f64>,
}

#[derive(Serialize)]
pub struct MetricsJson {
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub active_agents: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub uptime_seconds: u64,
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
            agents: m.window_count.saturating_sub(1), // first window is the shell, rest are agents
            created_at: m.created_at.to_rfc3339(),
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
            agents: window_count.saturating_sub(1),
            created_at: Utc::now().to_rfc3339(),
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
            spawned_at: chrono::Utc::now().to_rfc3339(),
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
            from_agent: "user".to_string(), // tasks initiated by user or auto-router
            to_agent: t.destination,
            content: t.content,
            status: t.status.to_string(),
            submitted_at: t.submitted_at.to_rfc3339(),
            completed_at: t.result.as_ref().map(|_| chrono::Utc::now().to_rfc3339()),
            cost_usd: t.result.as_ref().map(|r| r.cost_usd),
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

    let failed_tasks = tasks
        .iter()
        .filter(|t| {
            t.status == crate::orchestration::task_router::TaskStatus::Failed
        })
        .count();

    Json(MetricsJson {
        total_tokens: stats.total_tokens,
        total_cost_usd: stats.total_cost,
        active_agents: stats.active_agents,
        tasks_completed: completed_tasks,
        tasks_failed: failed_tasks,
        uptime_seconds: stats.session_seconds,
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

// ── Story 4.1: GET /api/pane-output ──────────────────────────────────────────

#[derive(Deserialize)]
pub struct PaneOutputParam {
    session: Option<String>,
    pane_id: Option<usize>,
    lines: Option<usize>,
}

pub async fn get_pane_output(
    State(state): State<AppState>,
    Query(params): Query<PaneOutputParam>,
) -> Response {
    if let Err(e) = require_session(params.session.as_deref(), &state.daemon.session_name) {
        return e;
    }

    let pane_id = match params.pane_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "pane_id parameter required"})),
            )
                .into_response();
        }
    };

    let max_lines = params.lines.unwrap_or(100);

    // Get pane snapshot from session
    let sess = state.daemon.session.lock().await;
    let snap = sess.snapshot();
    drop(sess);

    // Find the pane in the snapshot
    for window in &snap.windows {
        if let Some(pane_snap) = window.pane_cells.iter().find(|p| p.id == pane_id) {
            // Convert cell data to text lines
            let lines: Vec<String> = pane_snap
                .cells
                .iter()
                .take(max_lines)
                .map(|row| {
                    row.iter().map(|c| c.ch).collect::<String>().trim_end().to_string()
                })
                .collect();

            return Json(serde_json::json!({
                "pane_id": pane_id,
                "lines": lines,
                "rows": pane_snap.rows,
                "cols": pane_snap.cols,
            }))
            .into_response();
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": format!("Pane {} not found", pane_id)})),
    )
        .into_response()
}

// ── Story 6.1: GET /api/audit ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AuditParam {
    session: Option<String>,
    agent: Option<String>,
    event_type: Option<String>,
    limit: Option<usize>,
}

pub async fn get_audit(
    State(state): State<AppState>,
    Query(params): Query<AuditParam>,
) -> Response {
    if let Err(e) = require_session(params.session.as_deref(), &state.daemon.session_name) {
        return e;
    }

    // Read audit log from disk
    let audit_dir = dirs::data_local_dir()
        .unwrap_or_default()
        .join("bmux")
        .join("audit");

    let mut entries: Vec<serde_json::Value> = Vec::new();

    if audit_dir.exists() {
        // Read all .jsonl files, newest first
        let mut files: Vec<_> = std::fs::read_dir(&audit_dir)
            .unwrap_or_else(|_| std::fs::read_dir(".").unwrap())
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

        for file in files {
            if let Ok(content) = std::fs::read_to_string(file.path()) {
                for line in content.lines().rev() {
                    if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
                        // Apply filters
                        // Filter by session_id — prevent cross-session leaks
                        if entry.get("session_id").and_then(|v| v.as_str()) != Some(&state.daemon.session_name) {
                            continue;
                        }
                        if let Some(ref agent_filter) = params.agent {
                            if entry.get("agent_id").and_then(|v| v.as_str()) != Some(agent_filter) {
                                continue;
                            }
                        }
                        if let Some(ref type_filter) = params.event_type {
                            if entry.get("event_type").and_then(|v| v.as_str()) != Some(type_filter) {
                                continue;
                            }
                        }
                        entries.push(entry);
                        if entries.len() >= params.limit.unwrap_or(200) {
                            break;
                        }
                    }
                }
            }
            if entries.len() >= params.limit.unwrap_or(200) {
                break;
            }
        }
    }

    Json(entries).into_response()
}

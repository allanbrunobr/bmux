/// Shared state for the BMUX HTTP/WebSocket server (web dashboard backend).
///
/// This module is deliberately independent of the TUI internals — it receives
/// data via channels rather than holding references to Session/Window/Pane.
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

// ── Pane output cache (Story 4.1) ─────────────────────────────────────────────

/// Maximum number of lines stored per pane to bound memory usage.
const PANE_SCROLLBACK: usize = 5_000;

/// Thread-safe ring-buffer of raw ANSI lines for each pane.
///
/// The PTY reader thread pushes new lines here; the HTTP endpoint and
/// WebSocket broadcaster read from it. Using a bounded `VecDeque` keeps
/// memory constant and CPU overhead negligible (<1% per pane).
#[derive(Default, Clone)]
pub struct PaneOutputCache {
    inner: Arc<RwLock<HashMap<String, VecDeque<String>>>>,
}

impl PaneOutputCache {
    pub fn new() -> Self { Self::default() }

    /// Append raw ANSI lines for `pane_id`. Called from the PTY reader path.
    pub fn push_lines(&self, pane_id: &str, lines: impl IntoIterator<Item = String>) {
        let mut map = self.inner.write().unwrap();
        let buf = map.entry(pane_id.to_string()).or_default();
        for line in lines {
            buf.push_back(line);
            if buf.len() > PANE_SCROLLBACK {
                buf.pop_front();
            }
        }
    }

    /// Return the last `n` lines for `pane_id`. O(n) clone — acceptable for
    /// HTTP requests; no locking held across await points.
    pub fn last_lines(&self, pane_id: &str, n: usize) -> Vec<String> {
        let map = self.inner.read().unwrap();
        let Some(buf) = map.get(pane_id) else { return vec![] };
        let start = buf.len().saturating_sub(n);
        buf.iter().skip(start).cloned().collect()
    }
}

// ── WebSocket event types ─────────────────────────────────────────────────────

/// Events broadcast to all connected dashboard WebSocket clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    AgentUpdated       { agent:  serde_json::Value },
    AgentSpawned       { agent:  serde_json::Value },
    AgentStopped       { name:   String },
    TaskQueued         { task:   serde_json::Value },
    TaskStarted        { task:   serde_json::Value },
    TaskCompleted      { task:   serde_json::Value },
    ContextUpdated     { entry:  serde_json::Value },
    /// Live PTY output chunk for a specific pane (Story 4.1).
    PaneOutput         { pane_id: String, data: String },
    AgentTokensUpdated { name: String, tokens_delta: u64, cost_delta_usd: f64 },
    AuditEvent         { entry:  serde_json::Value },
}

pub type BroadcastTx = broadcast::Sender<WsEvent>;

// ── Agent / Task / Context snapshots ─────────────────────────────────────────

/// Lightweight agent snapshot pushed to the HTTP /api/agents endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub name: String,
    pub agent_type: String,
    pub model: String,
    pub status: String,
    pub provider: String,
    pub tokens_total: u64,
    pub cost_usd: f64,
    pub uptime_secs: u64,
    pub last_task_preview: String,
    pub pane_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub key: String,
    pub value: String,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSnapshot {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub agent: String,
    pub session: String,
    pub metadata: serde_json::Value,
    pub lgpd_compliant: Option<bool>,
}

// ── Shared mutable dashboard state ───────────────────────────────────────────

/// All mutable dashboard state behind RwLock (no async needed — reads dominate).
#[derive(Default)]
pub struct DashboardData {
    pub agents:  Vec<AgentSnapshot>,
    pub tasks:   Vec<TaskSnapshot>,
    pub context: Vec<ContextSnapshot>,
    pub audit:   Vec<AuditSnapshot>,
}

/// The shared state threaded through all Axum handlers.
pub struct WebAppState {
    pub session_name: String,
    pub data:         RwLock<DashboardData>,
    pub pane_cache:   PaneOutputCache,
    pub event_tx:     BroadcastTx,
}

impl WebAppState {
    pub fn new(session_name: String) -> Arc<Self> {
        let (tx, _) = broadcast::channel(1024);
        Arc::new(Self {
            session_name,
            data: RwLock::default(),
            pane_cache: PaneOutputCache::new(),
            event_tx: tx,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.event_tx.subscribe()
    }

    /// Broadcast an event to all WebSocket clients.
    pub fn broadcast(&self, event: WsEvent) {
        let _ = self.event_tx.send(event);
    }
}

pub type SharedState = Arc<WebAppState>;

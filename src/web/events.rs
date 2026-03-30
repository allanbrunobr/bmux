use serde::Serialize;

use crate::web::routes::{AgentInfo as AgentJson, ContextEntryJson, TaskJson};

/// Events emitted by the daemon when state changes.
/// Serialized as JSON and pushed to all connected WebSocket clients.
///
/// IMPORTANT: These shapes MUST match bmux-web/src/lib/types.ts BmuxEvent union.
/// The `#[serde(tag = "type")]` produces `{ "type": "agent_spawned", ...fields }`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BmuxEvent {
    // ── Agent lifecycle ──────────────────────────────────────────────────
    AgentSpawned {
        agent: AgentJson,
    },
    AgentKilled {
        agent_id: String,
    },
    AgentStatusChanged {
        agent_id: String,
        status: String,
    },
    AgentTokensUpdated {
        agent_id: String,
        tokens: u64,
        cost_usd: f64,
    },

    // ── Context ──────────────────────────────────────────────────────────
    ContextUpdated {
        entry: ContextEntryJson,
    },

    // ── Tasks ────────────────────────────────────────────────────────────
    TaskCreated {
        task: TaskJson,
    },
    TaskUpdated {
        task: TaskJson,
    },

    // ── Metrics ──────────────────────────────────────────────────────────
    MetricsUpdated {
        metrics: crate::web::routes::MetricsJson,
    },

    // ── Pane output ──────────────────────────────────────────────────────
    PaneOutput {
        pane_id: usize,
        data: String,
    },

    // ── Audit ────────────────────────────────────────────────────────────
    AuditEvent {
        entry: serde_json::Value,
    },

    // ── Adversarial mode ─────────────────────────────────────────────────
    AdversarialStarted {
        config: serde_json::Value,
        total_sprints: Option<u32>,
    },
    AdversarialNegotiating {
        contract_proposal: serde_json::Value,
    },
    AdversarialBuilding {
        sprint: u32,
        attempt: u32,
    },
    AdversarialEvaluating {
        sprint: u32,
    },
    AdversarialScores {
        sprint: u32,
        attempt: u32,
        scores: Vec<serde_json::Value>,
        passed: bool,
    },
    AdversarialRetry {
        sprint: u32,
        attempt: u32,
        feedback: Vec<String>,
    },
    AdversarialSprintPassed {
        sprint: u32,
        total_attempts: u32,
    },
    AdversarialFailed {
        sprint: u32,
        reason: String,
    },
    AdversarialComplete {
        sprints_passed: u32,
        total_duration_ms: u64,
    },
}

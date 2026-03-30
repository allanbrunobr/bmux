use chrono::{DateTime, Utc};
use serde::Serialize;

/// Events emitted by the daemon when state changes.
/// Serialized as JSON and pushed to all connected WebSocket clients.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BmuxEvent {
    AgentSpawned {
        session: String,
        agent_name: String,
        agent_type: String,
        pane_id: usize,
        timestamp: DateTime<Utc>,
    },
    AgentKilled {
        session: String,
        agent_name: String,
        timestamp: DateTime<Utc>,
    },
    ContextUpdated {
        session: String,
        key: String,
        value: String,
        timestamp: DateTime<Utc>,
    },
    TaskDispatched {
        session: String,
        task_id: String,
        agent: String,
        content: String,
        timestamp: DateTime<Utc>,
    },
    TaskCompleted {
        session: String,
        task_id: String,
        agent: String,
        timestamp: DateTime<Utc>,
    },
}

// AgentAdapter trait — the common interface for all BMUX agents.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Runtime status of an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum AgentStatus {
    #[default]
    Idle,
    Working,
    WaitingInput,
    Error(String),
}

impl AgentStatus {
    /// Returns the display indicator character.
    pub fn indicator(&self) -> &'static str {
        match self {
            AgentStatus::Working => "●",
            _ => "○",
        }
    }

    /// Returns a short string label.
    pub fn label(&self) -> String {
        match self {
            AgentStatus::Idle => "idle".to_string(),
            AgentStatus::Working => "working".to_string(),
            AgentStatus::WaitingInput => "waiting".to_string(),
            AgentStatus::Error(e) => format!("error: {e}"),
        }
    }
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Idle => write!(f, "idle"),
            AgentStatus::Working => write!(f, "working"),
            AgentStatus::WaitingInput => write!(f, "waiting_input"),
            AgentStatus::Error(e) => write!(f, "error: {e}"),
        }
    }
}


/// Configuration for spawning an agent process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub binary: String,
    pub model: String,
    pub cost_per_1k_tokens: f64,
    #[serde(default)]
    pub args: Vec<String>,
}

/// A task to be sent to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub content: String,
    pub priority: u32,
    pub preferred_model: Option<String>,
    pub max_cost_usd: Option<f64>,
}

impl Task {
    pub fn new(content: impl Into<String>) -> Self {
        Task {
            id: uuid::Uuid::new_v4().to_string(),
            content: content.into(),
            priority: 1,
            preferred_model: None,
            max_cost_usd: None,
        }
    }
}

/// Output produced by an agent after completing a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub content: String,
    pub tokens_used: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
}

/// Common interface for all BMUX agent adapters.
///
/// Each adapter wraps a child process (PTY or pipe) and provides
/// lifecycle management (spawn, kill) and task I/O (send_task, recv_output).
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Unique name for this agent instance (e.g. "arch", "tests").
    fn id(&self) -> &str;

    /// Agent type identifier (e.g. "claude-code", "shell", "custom").
    fn agent_type(&self) -> &str;

    /// Model name used by this agent (e.g. "claude-sonnet-4-5").
    fn model_name(&self) -> &str;

    /// Cost in USD per 1,000 tokens.
    fn cost_per_1k_tokens(&self) -> f64;

    /// Spawn the agent process using the given config.
    /// Returns an error if the binary is not found in PATH.
    async fn spawn(&mut self, config: &AgentConfig) -> anyhow::Result<()>;

    /// Send a task to the agent (writes to stdin).
    async fn send_task(&self, task: &Task) -> anyhow::Result<()>;

    /// Receive output from the agent (reads from stdout).
    async fn recv_output(&mut self) -> anyhow::Result<AgentOutput>;

    /// Query the current runtime status of the agent.
    async fn status(&self) -> AgentStatus;

    /// Terminate the agent process and clean up resources.
    async fn kill(&mut self) -> anyhow::Result<()>;
}

/// Check if a binary exists in PATH.
pub fn binary_in_path(binary: &str) -> bool {
    which::which(binary).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_status_indicator() {
        assert_eq!(AgentStatus::Idle.indicator(), "○");
        assert_eq!(AgentStatus::Working.indicator(), "●");
        assert_eq!(AgentStatus::WaitingInput.indicator(), "○");
    }

    #[test]
    fn test_agent_status_label() {
        assert_eq!(AgentStatus::Idle.label(), "idle");
        assert_eq!(AgentStatus::Working.label(), "working");
        assert_eq!(AgentStatus::Error("boom".to_string()).label(), "error: boom");
    }

    #[test]
    fn test_task_new_has_uuid() {
        let t1 = Task::new("do something");
        let t2 = Task::new("do something");
        assert_ne!(t1.id, t2.id);
        assert_eq!(t1.content, "do something");
        assert_eq!(t1.priority, 1);
    }

    #[test]
    fn test_agent_status_default_is_idle() {
        let status = AgentStatus::default();
        assert_eq!(status, AgentStatus::Idle);
    }
}

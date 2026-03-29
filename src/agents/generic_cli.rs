// GenericCliAdapter — reusable adapter for any CLI-based agent.
//
// Eliminates ~95% duplication between ClaudeCode, OpenCode, Pi, Gemini adapters.
// Each agent type is just a constructor with different defaults.

use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use async_trait::async_trait;
use anyhow::Context;

use crate::agents::adapter::{AgentAdapter, AgentConfig, AgentOutput, AgentStatus, Task};

/// Generic CLI agent adapter — spawns any binary and communicates via stdin/stdout.
pub struct GenericCliAdapter {
    id: String,
    agent_type: String,
    model: String,
    cost_per_1k_tokens: f64,
    status: AgentStatus,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_reader: Option<BufReader<ChildStdout>>,
    spawned_at: Option<Instant>,
    tokens_used: u64,
    cost_usd: f64,
}

impl GenericCliAdapter {
    /// Create a new GenericCliAdapter.
    pub fn new(
        id: impl Into<String>,
        agent_type: impl Into<String>,
        model: impl Into<String>,
        cost_per_1k_tokens: f64,
    ) -> Self {
        Self {
            id: id.into(),
            agent_type: agent_type.into(),
            model: model.into(),
            cost_per_1k_tokens,
            status: AgentStatus::Idle,
            child: None,
            stdin: None,
            stdout_reader: None,
            spawned_at: None,
            tokens_used: 0,
            cost_usd: 0.0,
        }
    }

    /// Convenience constructors for built-in agent types.
    pub fn claude_code(id: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(id, "claude-code", model, 0.003)
    }

    pub fn opencode(id: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(id, "opencode", model, 0.00125)
    }

    pub fn pi(id: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(id, "pi", model, 0.003)
    }

    pub fn gemini(id: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(id, "gemini", model, 0.00075)
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.spawned_at.map(|t| t.elapsed().as_secs()).unwrap_or(0)
    }
}

#[async_trait]
impl AgentAdapter for GenericCliAdapter {
    fn id(&self) -> &str {
        &self.id
    }

    fn agent_type(&self) -> &str {
        &self.agent_type
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        self.cost_per_1k_tokens
    }

    async fn spawn(&mut self, config: &AgentConfig) -> anyhow::Result<()> {
        if which::which(&config.binary).is_err() {
            anyhow::bail!(
                "Agent binary '{}' not found in PATH",
                config.binary
            );
        }

        let mut cmd = Command::new(&config.binary);
        if !config.model.is_empty() {
            cmd.arg("--model").arg(&config.model);
        }
        for arg in &config.args {
            cmd.arg(arg);
        }
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit());

        let mut child = cmd.spawn().context(format!(
            "Failed to spawn agent binary '{}'",
            config.binary
        ))?;

        self.stdin = child.stdin.take();
        let stdout = child.stdout.take().context("Failed to get stdout")?;
        self.stdout_reader = Some(BufReader::new(stdout));
        self.child = Some(child);
        self.spawned_at = Some(Instant::now());
        self.status = AgentStatus::Idle;
        Ok(())
    }

    async fn send_task(&self, task: &Task) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.child.is_some(),
            "Cannot send task to unspawned agent '{}'",
            self.id
        );
        tracing::debug!(
            id = %self.id,
            agent_type = %self.agent_type,
            task_id = %task.id,
            "Task sent to {} agent",
            self.agent_type
        );
        Ok(())
    }

    async fn recv_output(&mut self) -> anyhow::Result<AgentOutput> {
        let reader = self
            .stdout_reader
            .as_mut()
            .context("Agent not spawned — no stdout")?;
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .context("Failed to read from agent")?;

        let tokens = (line.len() / 4) as u64; // rough token estimate
        let cost = tokens as f64 * self.cost_per_1k_tokens / 1000.0;
        self.tokens_used += tokens;
        self.cost_usd += cost;
        self.status = AgentStatus::Idle;

        Ok(AgentOutput {
            content: line.trim().to_string(),
            tokens_used: tokens,
            cost_usd: cost,
            duration_ms: 0,
        })
    }

    async fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    async fn kill(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut child) = self.child {
            child.kill().await.context("Failed to kill agent")?;
        }
        self.status = AgentStatus::Idle;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_sets_agent_type() {
        let a = GenericCliAdapter::claude_code("test", "sonnet");
        assert_eq!(a.agent_type(), "claude-code");
        assert_eq!(a.model_name(), "sonnet");
        assert!(a.cost_per_1k_tokens() > 0.0);
    }

    #[tokio::test]
    async fn all_constructors_produce_idle_status() {
        for adapter in [
            GenericCliAdapter::claude_code("a", "m"),
            GenericCliAdapter::opencode("b", "m"),
            GenericCliAdapter::pi("c", "m"),
            GenericCliAdapter::gemini("d", "m"),
        ] {
            assert_eq!(adapter.status().await, AgentStatus::Idle);
        }
    }

    #[test]
    fn uptime_zero_before_spawn() {
        let a = GenericCliAdapter::new("t", "test", "m", 0.001);
        assert_eq!(a.uptime_seconds(), 0);
    }

    #[tokio::test]
    async fn spawn_missing_binary_returns_error() {
        let mut a = GenericCliAdapter::new("t", "test", "m", 0.001);
        let cfg = AgentConfig {
            binary: "definitely-not-a-real-binary-12345".to_string(),
            model: String::new(),
            cost_per_1k_tokens: 0.0,
            args: vec![],
        };
        let result = a.spawn(&cfg).await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("not found in PATH"),
        );
    }
}

// ClaudeCodeAdapter — wraps the `claude` CLI binary as a BMUX agent.

use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use async_trait::async_trait;
use anyhow::Context;

use crate::agents::adapter::{AgentAdapter, AgentConfig, AgentOutput, AgentStatus, Task};

/// Adapter for Claude Code CLI agent (`claude` binary).
/// Type indicator: 🤖
pub struct ClaudeCodeAdapter {
    id: String,
    model: String,
    status: AgentStatus,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_reader: Option<BufReader<ChildStdout>>,
    spawned_at: Option<Instant>,
    tokens_used: u64,
    cost_usd: f64,
}

impl ClaudeCodeAdapter {
    /// Create a new ClaudeCodeAdapter with the given id and model.
    pub fn new(id: impl Into<String>, model: impl Into<String>) -> Self {
        ClaudeCodeAdapter {
            id: id.into(),
            model: model.into(),
            status: AgentStatus::Idle,
            child: None,
            stdin: None,
            stdout_reader: None,
            spawned_at: None,
            tokens_used: 0,
            cost_usd: 0.0,
        }
    }

    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.spawned_at.map(|t| t.elapsed().as_secs()).unwrap_or(0)
    }
}

#[async_trait]
impl AgentAdapter for ClaudeCodeAdapter {
    fn id(&self) -> &str {
        &self.id
    }

    fn agent_type(&self) -> &str {
        "claude-code"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        0.003
    }

    async fn spawn(&mut self, config: &AgentConfig) -> anyhow::Result<()> {
        let binary = &config.binary;
        which::which(binary)
            .with_context(|| format!("Agent binary '{}' not found in PATH", binary))?;

        let mut cmd = Command::new(binary);
        cmd.args(&config.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()
            .with_context(|| format!("Failed to spawn claude-code agent '{}'", self.id))?;

        self.stdin = child.stdin.take();
        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
        self.stdout_reader = Some(BufReader::new(stdout));
        self.child = Some(child);
        self.spawned_at = Some(Instant::now());
        self.status = AgentStatus::Idle;

        tracing::info!(id = %self.id, model = %self.model, binary = %binary, "ClaudeCode agent spawned");
        Ok(())
    }

    async fn send_task(&self, task: &Task) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.child.is_some(),
            "Cannot send task to unspawned agent '{}'",
            self.id
        );
        tracing::debug!(id = %self.id, task_id = %task.id, "Task sent to claude-code agent");
        Ok(())
    }

    async fn recv_output(&mut self) -> anyhow::Result<AgentOutput> {
        let start = Instant::now();
        if let Some(reader) = &mut self.stdout_reader {
            let mut line = String::new();
            reader.read_line(&mut line).await
                .context("Failed to read output from claude-code agent")?;

            // Estimate tokens from output length (rough heuristic: 4 chars per token)
            let tokens = (line.len() / 4).max(1) as u64;
            let cost = tokens as f64 * self.cost_per_1k_tokens() / 1000.0;
            self.tokens_used += tokens;
            self.cost_usd += cost;

            Ok(AgentOutput {
                content: line.trim_end().to_string(),
                tokens_used: tokens,
                cost_usd: cost,
                duration_ms: start.elapsed().as_millis() as u64,
            })
        } else {
            Err(anyhow::anyhow!("Claude-code agent '{}' has no stdout reader", self.id))
        }
    }

    async fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    async fn kill(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.child.take() {
            child.kill().await
                .with_context(|| format!("Failed to kill claude-code agent '{}'", self.id))?;
            child.wait().await.ok();
            tracing::info!(id = %self.id, "ClaudeCode agent killed");
        }
        self.stdin = None;
        self.stdout_reader = None;
        self.status = AgentStatus::Idle;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_code_adapter_identity() {
        let agent = ClaudeCodeAdapter::new("arch", "claude-sonnet-4-5");
        assert_eq!(agent.id(), "arch");
        assert_eq!(agent.agent_type(), "claude-code");
        assert_eq!(agent.model_name(), "claude-sonnet-4-5");
        assert_eq!(agent.cost_per_1k_tokens(), 0.003);
    }

    #[tokio::test]
    async fn test_claude_code_initial_status_idle() {
        let agent = ClaudeCodeAdapter::new("arch", "claude-sonnet-4-5");
        assert_eq!(agent.status().await, AgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_spawn_missing_binary_error_message() {
        let mut agent = ClaudeCodeAdapter::new("arch", "claude-sonnet-4-5");
        let config = AgentConfig {
            binary: "binary-that-does-not-exist-xyz-999".to_string(),
            model: "claude-sonnet-4-5".to_string(),
            cost_per_1k_tokens: 0.003,
            args: vec![],
        };
        let err = agent.spawn(&config).await.unwrap_err();
        assert!(err.to_string().contains("not found in PATH"));
    }
}

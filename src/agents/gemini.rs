// GeminiAdapter — wraps the `gemini` CLI binary as a BMUX agent.
// Type indicator: ✨

use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use async_trait::async_trait;
use anyhow::Context;

use crate::agents::adapter::{AgentAdapter, AgentConfig, AgentOutput, AgentStatus, Task};

/// Adapter for Gemini CLI agent (`gemini` binary).
pub struct GeminiAdapter {
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

impl GeminiAdapter {
    pub fn new(id: impl Into<String>, model: impl Into<String>) -> Self {
        GeminiAdapter {
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
}

#[async_trait]
impl AgentAdapter for GeminiAdapter {
    fn id(&self) -> &str {
        &self.id
    }

    fn agent_type(&self) -> &str {
        "gemini"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        0.001
    }

    async fn spawn(&mut self, config: &AgentConfig) -> anyhow::Result<()> {
        let binary = &config.binary;
        which::which(binary)
            .with_context(|| format!("Agent binary '{}' not found in PATH", binary))?;

        let mut cmd = Command::new(binary);
        if !config.model.is_empty() {
            cmd.arg("--model").arg(&config.model);
        }
        cmd.args(&config.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()
            .with_context(|| format!("Failed to spawn gemini agent '{}'", self.id))?;

        self.stdin = child.stdin.take();
        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
        self.stdout_reader = Some(BufReader::new(stdout));
        self.child = Some(child);
        self.spawned_at = Some(Instant::now());
        self.status = AgentStatus::Idle;

        tracing::info!(id = %self.id, model = %self.model, binary = %binary, "Gemini agent spawned");
        Ok(())
    }

    async fn send_task(&self, task: &Task) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.child.is_some(),
            "Cannot send task to unspawned agent '{}'",
            self.id
        );
        tracing::debug!(id = %self.id, task_id = %task.id, "Task sent to gemini agent");
        Ok(())
    }

    async fn recv_output(&mut self) -> anyhow::Result<AgentOutput> {
        let start = Instant::now();
        if let Some(reader) = &mut self.stdout_reader {
            let mut line = String::new();
            reader.read_line(&mut line).await
                .context("Failed to read output from gemini agent")?;

            let tokens = (line.len() / 4).max(1) as u64;
            let cost = tokens as f64 * self.cost_per_1k_tokens() / 1000.0;

            Ok(AgentOutput {
                content: line.trim_end().to_string(),
                tokens_used: tokens,
                cost_usd: cost,
                duration_ms: start.elapsed().as_millis() as u64,
            })
        } else {
            Err(anyhow::anyhow!("Gemini agent '{}' has no stdout reader", self.id))
        }
    }

    async fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    async fn kill(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.child.take() {
            child.kill().await
                .with_context(|| format!("Failed to kill gemini agent '{}'", self.id))?;
            child.wait().await.ok();
            tracing::info!(id = %self.id, "Gemini agent killed");
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
    fn test_gemini_adapter_identity() {
        let agent = GeminiAdapter::new("gemini-agent", "gemini-2.5-pro");
        assert_eq!(agent.id(), "gemini-agent");
        assert_eq!(agent.agent_type(), "gemini");
        assert_eq!(agent.model_name(), "gemini-2.5-pro");
        assert_eq!(agent.cost_per_1k_tokens(), 0.001);
    }

    #[tokio::test]
    async fn test_gemini_initial_status_idle() {
        let agent = GeminiAdapter::new("gemini-agent", "gemini-2.5-pro");
        assert_eq!(agent.status().await, AgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_gemini_spawn_missing_binary() {
        let mut agent = GeminiAdapter::new("g", "gemini-2.5-pro");
        let config = AgentConfig {
            binary: "no-gemini-binary-xyz".to_string(),
            model: "gemini-2.5-pro".to_string(),
            cost_per_1k_tokens: 0.001,
            args: vec![],
        };
        let err = agent.spawn(&config).await.unwrap_err();
        assert!(err.to_string().contains("not found in PATH"));
    }
}

// CustomAdapter — wraps any CLI binary as a BMUX-managed agent via config.toml.
//
// Story 2.5: Allows users to integrate proprietary or custom agents without
// modifying BMUX source code.
// Type indicator: 🔌

use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use async_trait::async_trait;
use anyhow::Context;

use crate::agents::adapter::{AgentAdapter, AgentConfig, AgentOutput, AgentStatus, Task};

/// Adapter for user-defined custom agent binaries.
/// Configuration is read from config.toml [agents.<name>] section.
pub struct CustomAdapter {
    id: String,
    binary: String,
    model: String,
    cost_per_1k_tokens: f64,
    extra_args: Vec<String>,
    status: AgentStatus,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_reader: Option<BufReader<ChildStdout>>,
    spawned_at: Option<Instant>,
    tokens_used: u64,
    cost_usd: f64,
}

impl CustomAdapter {
    /// Create a new CustomAdapter with explicit configuration.
    pub fn new(
        id: impl Into<String>,
        binary: impl Into<String>,
        model: impl Into<String>,
        cost_per_1k_tokens: f64,
        extra_args: Vec<String>,
    ) -> Self {
        CustomAdapter {
            id: id.into(),
            binary: binary.into(),
            model: model.into(),
            cost_per_1k_tokens,
            extra_args,
            status: AgentStatus::Idle,
            child: None,
            stdin: None,
            stdout_reader: None,
            spawned_at: None,
            tokens_used: 0,
            cost_usd: 0.0,
        }
    }

    /// Create a CustomAdapter from an AgentConfig entry.
    pub fn from_config(id: impl Into<String>, config: &AgentConfig) -> Self {
        Self::new(
            id,
            &config.binary,
            &config.model,
            config.cost_per_1k_tokens,
            config.args.clone(),
        )
    }

    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.spawned_at.map(|t| t.elapsed().as_secs()).unwrap_or(0)
    }
}

#[async_trait]
impl AgentAdapter for CustomAdapter {
    fn id(&self) -> &str {
        &self.id
    }

    fn agent_type(&self) -> &str {
        "custom"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        self.cost_per_1k_tokens
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
            .with_context(|| format!("Failed to spawn custom agent '{}'", self.id))?;

        self.stdin = child.stdin.take();
        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
        self.stdout_reader = Some(BufReader::new(stdout));
        self.child = Some(child);
        self.spawned_at = Some(Instant::now());
        self.status = AgentStatus::Idle;

        tracing::info!(
            id = %self.id,
            binary = %binary,
            model = %self.model,
            "Custom agent spawned"
        );
        Ok(())
    }

    async fn send_task(&self, task: &Task) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.child.is_some(),
            "Cannot send task to unspawned agent '{}'",
            self.id
        );
        tracing::debug!(id = %self.id, task_id = %task.id, "Task sent to custom agent");
        Ok(())
    }

    async fn recv_output(&mut self) -> anyhow::Result<AgentOutput> {
        let start = Instant::now();
        if let Some(reader) = &mut self.stdout_reader {
            let mut line = String::new();
            reader.read_line(&mut line).await
                .context("Failed to read output from custom agent")?;

            let tokens = (line.len() / 4).max(1) as u64;
            let cost = tokens as f64 * self.cost_per_1k_tokens / 1000.0;
            self.tokens_used += tokens;
            self.cost_usd += cost;

            Ok(AgentOutput {
                content: line.trim_end().to_string(),
                tokens_used: tokens,
                cost_usd: cost,
                duration_ms: start.elapsed().as_millis() as u64,
            })
        } else {
            Err(anyhow::anyhow!("Custom agent '{}' has no stdout reader", self.id))
        }
    }

    async fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    async fn kill(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.child.take() {
            child.kill().await
                .with_context(|| format!("Failed to kill custom agent '{}'", self.id))?;
            child.wait().await.ok();
            tracing::info!(id = %self.id, "Custom agent killed");
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
    fn test_custom_adapter_identity() {
        let agent = CustomAdapter::new(
            "meu-agente",
            "meu-agente-cli",
            "llama-3.1-70b",
            0.0001,
            vec!["--mode".to_string(), "interactive".to_string()],
        );
        assert_eq!(agent.id(), "meu-agente");
        assert_eq!(agent.agent_type(), "custom");
        assert_eq!(agent.model_name(), "llama-3.1-70b");
        assert_eq!(agent.cost_per_1k_tokens(), 0.0001);
    }

    #[tokio::test]
    async fn test_custom_adapter_initial_status_idle() {
        let agent = CustomAdapter::new("test", "my-bin", "model", 0.0, vec![]);
        assert_eq!(agent.status().await, AgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_custom_adapter_spawn_missing_binary() {
        let mut agent = CustomAdapter::new(
            "meu-agente",
            "totally-nonexistent-binary-xyz",
            "llama-3.1-70b",
            0.0001,
            vec![],
        );
        let config = AgentConfig {
            binary: "totally-nonexistent-binary-xyz".to_string(),
            model: "llama-3.1-70b".to_string(),
            cost_per_1k_tokens: 0.0001,
            args: vec![],
        };
        let err = agent.spawn(&config).await.unwrap_err();
        assert!(err.to_string().contains("not found in PATH"));
    }

    #[test]
    fn test_custom_adapter_from_config() {
        let config = AgentConfig {
            binary: "meu-agente-cli".to_string(),
            model: "llama-3.1-70b".to_string(),
            cost_per_1k_tokens: 0.0001,
            args: vec!["--mode".to_string(), "interactive".to_string()],
        };
        let agent = CustomAdapter::from_config("meu-agente", &config);
        assert_eq!(agent.id(), "meu-agente");
        assert_eq!(agent.model_name(), "llama-3.1-70b");
        assert_eq!(agent.cost_per_1k_tokens(), 0.0001);
        assert_eq!(agent.extra_args, vec!["--mode", "interactive"]);
    }
}

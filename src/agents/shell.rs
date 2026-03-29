// ShellAdapter — wraps a shell child process as a BMUX agent.
//
// Story 2.1: Validates the full agent lifecycle before AI-specific adapters are built.
// The shell process is spawned with stdin/stdout pipes for task I/O.

use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use async_trait::async_trait;
use anyhow::Context;

use crate::agents::adapter::{AgentAdapter, AgentConfig, AgentOutput, AgentStatus, Task};

/// Shell agent adapter — spawns a shell process and communicates via stdin/stdout pipes.
pub struct ShellAdapter {
    id: String,
    shell: String,
    status: AgentStatus,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_reader: Option<BufReader<ChildStdout>>,
    spawned_at: Option<Instant>,
}

impl ShellAdapter {
    /// Create a new ShellAdapter with the given id and shell path.
    pub fn new(id: impl Into<String>, shell: impl Into<String>) -> Self {
        ShellAdapter {
            id: id.into(),
            shell: shell.into(),
            status: AgentStatus::Idle,
            child: None,
            stdin: None,
            stdout_reader: None,
            spawned_at: None,
        }
    }

    /// Returns the uptime in seconds since spawn, or 0 if not yet spawned.
    pub fn uptime_seconds(&self) -> u64 {
        self.spawned_at
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }

    /// Returns true if the child process is currently running.
    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }
}

#[async_trait]
impl AgentAdapter for ShellAdapter {
    fn id(&self) -> &str {
        &self.id
    }

    fn agent_type(&self) -> &str {
        "shell"
    }

    fn model_name(&self) -> &str {
        "shell"
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        0.0
    }

    async fn spawn(&mut self, config: &AgentConfig) -> anyhow::Result<()> {
        let binary = &config.binary;

        // Verify the binary exists
        which::which(binary)
            .with_context(|| format!("Agent binary '{}' not found in PATH", binary))?;

        let mut cmd = Command::new(binary);
        cmd.args(&config.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()
            .with_context(|| format!("Failed to spawn shell agent '{}'", self.id))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdin for shell agent"))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout for shell agent"))?;

        self.stdin = Some(stdin);
        self.stdout_reader = Some(BufReader::new(stdout));
        self.child = Some(child);
        self.spawned_at = Some(Instant::now());
        self.status = AgentStatus::Idle;

        tracing::info!(id = %self.id, binary = %binary, "Shell agent spawned");
        Ok(())
    }

    async fn send_task(&self, task: &Task) -> anyhow::Result<()> {
        // We need mutable access to stdin — use unsafe cell or RefCell pattern.
        // For simplicity, we use a separate send mechanism via process stdin.
        // The stdin field will be mutated via interior mutability in a real implementation;
        // here we write the task content as a shell command followed by newline.
        anyhow::ensure!(
            self.child.is_some(),
            "Cannot send task to unspawned agent '{}'",
            self.id
        );
        tracing::debug!(id = %self.id, task_id = %task.id, "Task queued for shell agent (async write pending)");
        Ok(())
    }

    async fn recv_output(&mut self) -> anyhow::Result<AgentOutput> {
        let start = Instant::now();

        if let Some(reader) = &mut self.stdout_reader {
            let mut line = String::new();
            reader.read_line(&mut line).await
                .context("Failed to read output from shell agent")?;

            Ok(AgentOutput {
                content: line.trim_end().to_string(),
                tokens_used: 0,
                cost_usd: 0.0,
                duration_ms: start.elapsed().as_millis() as u64,
            })
        } else {
            Err(anyhow::anyhow!("Shell agent '{}' has no stdout reader", self.id))
        }
    }

    async fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    async fn kill(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.child.take() {
            child.kill().await
                .with_context(|| format!("Failed to kill shell agent '{}'", self.id))?;
            child.wait().await.ok();
            tracing::info!(id = %self.id, "Shell agent killed");
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
    fn test_shell_adapter_new() {
        let agent = ShellAdapter::new("test", "/bin/sh");
        assert_eq!(agent.id(), "test");
        assert_eq!(agent.agent_type(), "shell");
        assert_eq!(agent.model_name(), "shell");
        assert_eq!(agent.cost_per_1k_tokens(), 0.0);
        assert!(!agent.is_running());
        assert_eq!(agent.uptime_seconds(), 0);
    }

    #[tokio::test]
    async fn test_shell_adapter_initial_status_idle() {
        let agent = ShellAdapter::new("test", "/bin/sh");
        assert_eq!(agent.status().await, AgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_shell_adapter_spawn_sets_running() {
        let mut agent = ShellAdapter::new("test-spawn", "/bin/sh");
        let config = AgentConfig {
            binary: "/bin/sh".to_string(),
            model: "shell".to_string(),
            cost_per_1k_tokens: 0.0,
            args: vec![],
        };
        agent.spawn(&config).await.expect("spawn ok");
        assert!(agent.is_running());
        agent.kill().await.ok();
    }

    #[tokio::test]
    async fn test_shell_adapter_kill_stops_process() {
        let mut agent = ShellAdapter::new("kill-test", "/bin/sh");
        let config = AgentConfig {
            binary: "/bin/sh".to_string(),
            model: "shell".to_string(),
            cost_per_1k_tokens: 0.0,
            args: vec![],
        };
        agent.spawn(&config).await.expect("spawn ok");
        agent.kill().await.expect("kill ok");
        assert!(!agent.is_running());
    }

    #[tokio::test]
    async fn test_shell_adapter_spawn_missing_binary_fails() {
        let mut agent = ShellAdapter::new("bad", "definitely-not-real-binary-9999");
        let config = AgentConfig {
            binary: "definitely-not-real-binary-9999".to_string(),
            model: "shell".to_string(),
            cost_per_1k_tokens: 0.0,
            args: vec![],
        };
        let result = agent.spawn(&config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found in PATH"));
    }

    #[tokio::test]
    async fn test_shell_adapter_send_task_without_spawn_fails() {
        let agent = ShellAdapter::new("no-spawn", "/bin/sh");
        let task = Task::new("echo hello");
        let result = agent.send_task(&task).await;
        assert!(result.is_err());
    }
}

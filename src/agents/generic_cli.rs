// GenericCliAdapter — reusable adapter for any CLI-based agent.
//
// Uses portable-pty to spawn agents with a real pseudoterminal,
// which is required for interactive CLI tools like claude, opencode, pi.
// This is the same PTY pattern used by src/tui/pane.rs.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::Instant;

use anyhow::Context;
use async_trait::async_trait;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::agents::adapter::{AgentAdapter, AgentConfig, AgentOutput, AgentStatus, Task};

/// Generic CLI agent adapter — spawns any binary in a PTY.
pub struct GenericCliAdapter {
    id: String,
    agent_type: String,
    model: String,
    cost_per_1k_tokens: f64,
    status: AgentStatus,
    /// PTY master handle — used for read/write to the agent process.
    pty_master: Option<Arc<Mutex<Box<dyn MasterPty + Send>>>>,
    /// Writer side of the PTY (for sending input/tasks).
    pty_writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    /// Child process handle.
    child: Option<Arc<Mutex<Box<dyn portable_pty::Child + Send>>>>,
    /// PID of the child process.
    child_pid: Option<u32>,
    /// Flag set by background reader when new output arrives.
    has_output: Arc<AtomicBool>,
    /// Captured output lines from the PTY reader thread.
    output_buffer: Arc<Mutex<Vec<String>>>,
    spawned_at: Option<Instant>,
    tokens_used: u64,
    cost_usd: f64,
    /// BMUX session name — injected as env var on spawn.
    session_name: Option<String>,
    /// BMUX IPC socket path — injected as env var on spawn.
    socket_path: Option<String>,
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
            pty_master: None,
            pty_writer: None,
            child: None,
            child_pid: None,
            has_output: Arc::new(AtomicBool::new(false)),
            output_buffer: Arc::new(Mutex::new(Vec::new())),
            spawned_at: None,
            tokens_used: 0,
            cost_usd: 0.0,
            session_name: None,
            socket_path: None,
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

    /// Set BMUX session context — injected as env vars when spawning.
    pub fn set_session_context(&mut self, session_name: &str, socket_path: &str) {
        self.session_name = Some(session_name.to_string());
        self.socket_path = Some(socket_path.to_string());
    }

    /// Get the child process PID, if spawned.
    pub fn pid(&self) -> Option<u32> {
        self.child_pid
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
        // For non-shell agents, verify binary exists
        if self.agent_type != "shell" && which::which(&config.binary).is_err() {
            anyhow::bail!(
                "Agent binary '{}' not found in PATH",
                config.binary
            );
        }

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY for agent")?;

        let mut cmd = CommandBuilder::new(&config.binary);

        // Add model flag if present
        if !config.model.is_empty() && self.agent_type != "shell" {
            cmd.arg("--model");
            cmd.arg(&config.model);
        }

        // Add extra args
        for arg in &config.args {
            cmd.arg(arg);
        }

        // Inject BMUX session context
        if let Some(ref sess) = self.session_name {
            cmd.env("BMUX_SESSION", sess);
        }
        if let Some(ref sock) = self.socket_path {
            cmd.env("BMUX_SOCKET", sock);
        }
        cmd.env("BMUX_AGENT_NAME", &self.id);
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .context(format!("Failed to spawn agent binary '{}'", config.binary))?;

        // Get PID before moving child
        self.child_pid = child.process_id();

        let writer = pair
            .master
            .take_writer()
            .context("Failed to acquire PTY writer")?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .context("Failed to acquire PTY reader")?;

        // Background thread: read PTY output into buffer
        let output_buf = Arc::clone(&self.output_buffer);
        let has_output = Arc::clone(&self.has_output);
        let agent_id = self.id.clone();
        std::thread::Builder::new()
            .name(format!("bmux-agent-pty-{}", agent_id))
            .spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&buf[..n]).to_string();
                            if let Ok(mut lines) = output_buf.lock() {
                                lines.push(text);
                                // Cap buffer at 1000 entries
                                if lines.len() > 1000 {
                                    lines.drain(..500);
                                }
                            }
                            has_output.store(true, Ordering::Relaxed);
                        }
                    }
                }
            })
            .context("Failed to spawn PTY reader thread")?;

        self.pty_master = Some(Arc::new(Mutex::new(pair.master)));
        self.pty_writer = Some(Arc::new(Mutex::new(writer)));
        self.child = Some(Arc::new(Mutex::new(child)));
        self.spawned_at = Some(Instant::now());
        self.status = AgentStatus::Idle;

        tracing::info!(
            agent = %self.id,
            pid = ?self.child_pid,
            binary = %config.binary,
            "Agent spawned with PTY"
        );

        Ok(())
    }

    async fn send_task(&self, task: &Task) -> anyhow::Result<()> {
        let writer = self
            .pty_writer
            .as_ref()
            .context(format!("Agent '{}' not spawned — no PTY", self.id))?;
        let mut writer = writer.lock().unwrap();
        writer.write_all(task.content.as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        tracing::debug!(
            id = %self.id,
            agent_type = %self.agent_type,
            task_id = %task.id,
            "Task written to {} agent PTY",
            self.agent_type
        );
        Ok(())
    }

    async fn recv_output(&mut self) -> anyhow::Result<AgentOutput> {
        // Drain output buffer
        let content = if let Ok(mut lines) = self.output_buffer.lock() {
            let text = lines.join("");
            lines.clear();
            text
        } else {
            String::new()
        };

        let tokens = (content.len() / 4) as u64;
        let cost = tokens as f64 * self.cost_per_1k_tokens / 1000.0;
        self.tokens_used += tokens;
        self.cost_usd += cost;
        self.has_output.store(false, Ordering::Relaxed);

        Ok(AgentOutput {
            content,
            tokens_used: tokens,
            cost_usd: cost,
            duration_ms: 0,
        })
    }

    async fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    async fn kill(&mut self) -> anyhow::Result<()> {
        if let Some(ref child) = self.child {
            let mut child = child.lock().unwrap();
            child.kill().context("Failed to kill agent process")?;
            tracing::info!(agent = %self.id, pid = ?self.child_pid, "Agent process killed");
        }
        self.status = AgentStatus::Idle;
        Ok(())
    }
}

/// Write a string to the agent's PTY (e.g. to send a task prompt).
/// This is used by the daemon to inject task content into the agent's terminal.
pub fn write_to_pty(adapter: &GenericCliAdapter, input: &str) -> anyhow::Result<()> {
    let writer = adapter
        .pty_writer
        .as_ref()
        .context("Agent PTY writer not available")?;
    let mut writer = writer.lock().unwrap();
    writer.write_all(input.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
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

    #[test]
    fn pid_none_before_spawn() {
        let a = GenericCliAdapter::new("t", "test", "m", 0.001);
        assert_eq!(a.pid(), None);
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

    #[tokio::test]
    async fn spawn_shell_creates_pty_process() {
        let mut a = GenericCliAdapter::new("test-sh", "shell", "shell", 0.0);
        let cfg = AgentConfig {
            binary: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()),
            model: String::new(),
            cost_per_1k_tokens: 0.0,
            args: vec![],
        };
        let result = a.spawn(&cfg).await;
        assert!(result.is_ok(), "spawn failed: {:?}", result.err());
        assert!(a.pid().is_some(), "PID should be set after spawn");
        assert!(a.pid().unwrap() > 0);
        assert!(a.pty_writer.is_some(), "PTY writer should be set");
        assert!(a.child.is_some(), "child handle should be set");

        // Clean up
        a.kill().await.ok();
    }

    #[tokio::test]
    async fn session_context_sets_fields() {
        let mut a = GenericCliAdapter::new("t", "test", "m", 0.001);
        a.set_session_context("my-session", "/tmp/bmux-my-session-ipc.sock");
        assert_eq!(a.session_name.as_deref(), Some("my-session"));
        assert_eq!(a.socket_path.as_deref(), Some("/tmp/bmux-my-session-ipc.sock"));
    }
}

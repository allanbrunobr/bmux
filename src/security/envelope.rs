//! Security envelope — single entry point for spawn hardening, task sanitization, and audit scrubbing.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use thiserror::Error;

use crate::config::settings::SecurityConfig;
use crate::storage::audit_log::{AuditEntry, AuditLogger, EventType};

use super::secrets::SecretsManager;

#[derive(Debug, Error)]
pub enum EnvelopeError {
    #[error("task content rejected: {0}")]
    TaskRejected(String),

    #[error("audit error: {0}")]
    Audit(#[from] crate::storage::audit_log::AuditError),
}

/// Security policies applied at runtime boundaries.
#[derive(Debug)]
pub struct SecurityEnvelope {
    pub session_id: String,
    pub project_dir: PathBuf,
    pub security: SecurityConfig,
    pub secrets: SecretsManager,
    pub audit: Arc<Mutex<AuditLogger>>,
}

impl SecurityEnvelope {
    pub fn new(
        session_id: impl Into<String>,
        project_dir: PathBuf,
        security: SecurityConfig,
        audit_enabled: bool,
    ) -> Self {
        let secrets = SecretsManager::load(
            &SecretsManager::default_path().unwrap_or_else(|| PathBuf::from("/dev/null")),
        )
        .unwrap_or_default();

        let audit = AuditLogger::with_default_dir(audit_enabled)
            .unwrap_or_else(|| AuditLogger::new(PathBuf::from("/tmp/bmux-audit"), audit_enabled));

        Self {
            session_id: session_id.into(),
            project_dir,
            security,
            secrets,
            audit: Arc::new(Mutex::new(audit)),
        }
    }

    /// Scrub known secrets from text before logging or persisting.
    pub fn scrub(&self, text: &str) -> String {
        self.secrets.scrub(text)
    }

    /// Sanitize task content before writing to a PTY.
    ///
    /// Rejects NUL bytes and shell control sequences that could break out of a
    /// single-line prompt injection.
    pub fn sanitize_task_content(&self, content: &str) -> Result<Vec<u8>, EnvelopeError> {
        if content.contains('\0') {
            return Err(EnvelopeError::TaskRejected(
                "task content contains NUL byte".into(),
            ));
        }
        // Single-line tasks only — multiline could inject shell commands on Enter.
        if content.contains('\n') {
            return Err(EnvelopeError::TaskRejected(
                "multiline task content is not allowed (use a file or workflow step)".into(),
            ));
        }
        let mut bytes = content.as_bytes().to_vec();
        bytes.push(b'\r');
        Ok(bytes)
    }

    /// Shell-quote a string for safe inclusion in a shell command.
    pub fn shell_quote(s: &str) -> String {
        if s.is_empty() {
            return "''".to_string();
        }
        if s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:@".contains(c))
        {
            return s.to_string();
        }
        format!("'{}'", s.replace('\'', "'\"'\"'"))
    }

    /// Build a safe shell command to launch an agent inside an existing pane shell.
    pub fn build_agent_launch_command(
        &self,
        agent_name: &str,
        ipc_socket: &Path,
        binary: &str,
        args: &[String],
        model: &str,
        agent_type: &str,
    ) -> String {
        let mut parts = vec![
            format!("export BMUX_SESSION={}", Self::shell_quote(&self.session_id)),
            format!(
                "BMUX_SOCKET={}",
                Self::shell_quote(&ipc_socket.display().to_string())
            ),
            format!("BMUX_AGENT_NAME={}", Self::shell_quote(agent_name)),
            format!("BMUX_AGENT_TYPE={}", Self::shell_quote(agent_type)),
            binary.to_string(),
        ];

        for arg in args {
            parts.push(Self::shell_quote(arg));
        }

        if !model.is_empty() && agent_type != "shell" {
            parts.push("--model".to_string());
            parts.push(Self::shell_quote(model));
        }

        parts.join(" ")
    }

    pub fn log_agent_spawned(
        &self,
        agent_name: &str,
        agent_type: &str,
        model: &str,
    ) -> Result<(), EnvelopeError> {
        let mut audit = self.audit.lock().expect("audit mutex");
        audit.log_event(AuditEntry::new(
            EventType::AgentSpawned,
            Some(agent_name),
            &self.session_id,
            serde_json::json!({
                "agent_type": agent_type,
                "model": self.scrub(model),
            }),
        ))?;
        Ok(())
    }

    pub fn log_agent_killed(&self, agent_name: &str, agent_type: &str) -> Result<(), EnvelopeError> {
        let mut audit = self.audit.lock().expect("audit mutex");
        audit.log_event(AuditEntry::new(
            EventType::AgentKilled,
            Some(agent_name),
            &self.session_id,
            serde_json::json!({ "agent_type": agent_type }),
        ))?;
        Ok(())
    }

    pub fn log_task_sent(
        &self,
        agent_name: &str,
        task_id: &str,
        content: &str,
    ) -> Result<(), EnvelopeError> {
        let mut audit = self.audit.lock().expect("audit mutex");
        audit.log_event(AuditEntry::new(
            EventType::TaskSent,
            Some(agent_name),
            &self.session_id,
            serde_json::json!({
                "task_id": task_id,
                "content_preview": self.scrub(&content.chars().take(80).collect::<String>()),
            }),
        ))?;
        Ok(())
    }

    pub fn log_context_set(&self, key: &str, value: &str) -> Result<(), EnvelopeError> {
        let mut audit = self.audit.lock().expect("audit mutex");
        audit.log_event(AuditEntry::new(
            EventType::AgentStatusChange,
            None::<String>,
            &self.session_id,
            serde_json::json!({
                "action": "context_set",
                "key": key,
                "value_preview": self.scrub(&value.chars().take(80).collect::<String>()),
            }),
        ))?;
        Ok(())
    }
}

/// Restrict a Unix socket to owner-only (0600) after bind.
pub fn harden_socket_permissions(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_multiline_task() {
        let env = SecurityEnvelope::new(
            "test",
            PathBuf::from("/tmp"),
            SecurityConfig::default(),
            false,
        );
        assert!(env.sanitize_task_content("line1\nline2").is_err());
    }

    #[test]
    fn shell_quote_escapes_spaces() {
        assert_eq!(
            SecurityEnvelope::shell_quote("hello world"),
            "'hello world'"
        );
    }
}
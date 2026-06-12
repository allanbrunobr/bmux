//! Security envelope — single entry point for spawn hardening, task sanitization, and audit scrubbing.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use thiserror::Error;
use tracing::warn;

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
    ///
    /// Non-shell agents use `env -i` with a minimal allowlist (no inherited secrets/env).
    /// macOS may wrap with `sandbox-exec` when `[security] sandbox` is enabled.
    pub fn build_agent_launch_command(
        &self,
        agent_name: &str,
        ipc_socket: &Path,
        binary: &str,
        args: &[String],
        model: &str,
        agent_type: &str,
    ) -> String {
        let inner = if agent_type == "shell" {
            self.build_shell_launch_inner(ipc_socket, binary, args, model, agent_type, agent_name)
        } else {
            self.build_isolated_launch_inner(
                ipc_socket, binary, args, model, agent_type, agent_name,
            )
        };
        self.wrap_sandbox_if_enabled(&inner, agent_name, agent_type)
    }

    fn build_shell_launch_inner(
        &self,
        ipc_socket: &Path,
        binary: &str,
        args: &[String],
        model: &str,
        agent_type: &str,
        agent_name: &str,
    ) -> String {
        let mut parts = self.bmux_env_exports(ipc_socket, agent_name, agent_type);
        parts.push(binary.to_string());
        for arg in args {
            parts.push(Self::shell_quote(arg));
        }
        if !model.is_empty() {
            parts.push("--model".to_string());
            parts.push(Self::shell_quote(model));
        }
        parts.join(" ")
    }

    fn build_isolated_launch_inner(
        &self,
        ipc_socket: &Path,
        binary: &str,
        args: &[String],
        model: &str,
        agent_type: &str,
        agent_name: &str,
    ) -> String {
        let mut parts = vec!["env".to_string(), "-i".to_string()];
        parts.extend(self.bmux_env_pair_strings(ipc_socket, agent_name, agent_type));
        parts.push(binary.to_string());
        for arg in args {
            parts.push(Self::shell_quote(arg));
        }
        if !model.is_empty() {
            parts.push("--model".to_string());
            parts.push(Self::shell_quote(model));
        }
        parts.join(" ")
    }

    fn bmux_env_exports(
        &self,
        ipc_socket: &Path,
        agent_name: &str,
        agent_type: &str,
    ) -> Vec<String> {
        self.bmux_env_pairs(ipc_socket, agent_name, agent_type)
            .into_iter()
            .map(|(k, v)| format!("export {k}={}", Self::shell_quote(&v)))
            .collect()
    }

    /// Key/value pairs for `env -i` or CommandBuilder::env (values unquoted).
    pub fn bmux_env_pairs(
        &self,
        ipc_socket: &Path,
        agent_name: &str,
        agent_type: &str,
    ) -> Vec<(String, String)> {
        let mut pairs = vec![
            ("BMUX_SESSION".to_string(), self.session_id.clone()),
            (
                "BMUX_SOCKET".to_string(),
                ipc_socket.display().to_string(),
            ),
            ("BMUX_AGENT_NAME".to_string(), agent_name.to_string()),
            ("BMUX_AGENT_TYPE".to_string(), agent_type.to_string()),
            (
                "BMUX_PROJECT_DIR".to_string(),
                self.project_dir.display().to_string(),
            ),
        ];
        if let Some(path) = SecretsManager::default_path() {
            if path.exists() && !self.secrets.is_empty() {
                pairs.push((
                    "BMUX_SECRETS_PATH".to_string(),
                    path.display().to_string(),
                ));
            }
        }
        pairs
    }

    fn bmux_env_pair_strings(
        &self,
        ipc_socket: &Path,
        agent_name: &str,
        agent_type: &str,
    ) -> Vec<String> {
        self.bmux_env_pairs(ipc_socket, agent_name, agent_type)
            .into_iter()
            .map(|(k, v)| format!("{k}={}", Self::shell_quote(&v)))
            .collect()
    }

    fn wrap_sandbox_if_enabled(&self, inner_cmd: &str, agent_name: &str, agent_type: &str) -> String {
        if agent_type == "shell" || self.security.sandbox == "none" {
            return inner_cmd.to_string();
        }

        #[cfg(target_os = "macos")]
        {
            use std::io::Write;
            let profile = crate::security::sandbox::build_seatbelt_profile(&self.project_dir);
            let path = std::env::temp_dir().join(format!(
                "bmux-sandbox-{}-{}.sb",
                self.session_id.replace(['/', '\\', ':'], "_"),
                agent_name.replace(['/', '\\', ':'], "_"),
            ));
            if let Ok(mut f) = std::fs::File::create(&path) {
                if f.write_all(profile.as_bytes()).is_ok() {
                    return format!(
                        "sandbox-exec -f {} sh -c {}",
                        Self::shell_quote(&path.display().to_string()),
                        Self::shell_quote(inner_cmd),
                    );
                }
            }
            warn!(agent = %agent_name, "Failed to write Seatbelt profile; spawning without sandbox");
            inner_cmd.to_string()
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = agent_name;
            warn!(
                sandbox = %self.security.sandbox,
                "PTY spawn cannot apply Landlock to agent child; use env -i isolation only"
            );
            inner_cmd.to_string()
        }
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

    #[test]
    fn isolated_launch_uses_env_i() {
        let mut security = SecurityConfig::default();
        security.sandbox = "none".to_string();
        let env = SecurityEnvelope::new(
            "test",
            PathBuf::from("/tmp/project"),
            security,
            false,
        );
        let cmd = env.build_agent_launch_command(
            "arch",
            Path::new("/tmp/bmux-test-ipc.sock"),
            "claude",
            &[],
            "claude-sonnet-4-20250514",
            "claude-code",
        );
        assert!(cmd.starts_with("env -i "));
        assert!(cmd.contains("BMUX_SESSION=test"));
        assert!(cmd.contains("BMUX_PROJECT_DIR="));
    }
}
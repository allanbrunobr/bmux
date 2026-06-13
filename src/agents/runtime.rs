//! Agent runtime — unified spawn configuration and launch command building for the daemon.

use std::path::Path;

use crate::agents::adapter::AgentConfig;
use crate::config::agent_catalog;
use crate::config::settings::BmuxConfig;
use crate::security::agent_spawn::{self, AgentPtySpawn};
use crate::security::envelope::SecurityEnvelope;

/// Resolves agent configuration and builds safe launch commands for daemon spawns.
pub struct AgentRuntime<'a> {
    config: &'a BmuxConfig,
    envelope: &'a SecurityEnvelope,
    ipc_socket: &'a Path,
}

impl<'a> AgentRuntime<'a> {
    pub fn new(
        config: &'a BmuxConfig,
        envelope: &'a SecurityEnvelope,
        ipc_socket: &'a Path,
    ) -> Self {
        Self {
            config,
            envelope,
            ipc_socket,
        }
    }

    /// Resolve full agent config for a type, with optional model override.
    pub fn resolve_config(
        &self,
        agent_type: &str,
        model_override: Option<&str>,
    ) -> AgentConfig {
        let settings = agent_catalog::resolve_agent_config(
            agent_type,
            &self.config.agents,
            self.config.security.allow_dangerous_permissions,
        );

        let mut config = AgentConfig {
            binary: settings.binary,
            model: settings.model,
            cost_per_1k_tokens: settings.cost_per_1k_tokens,
            args: settings.args,
        };

        if let Some(m) = model_override {
            config.model = m.to_string();
        }

        config
    }

    /// Build a shell-safe launch command for `split_and_run_command`.
    pub fn build_launch_command(
        &self,
        agent_name: &str,
        agent_type: &str,
        config: &AgentConfig,
    ) -> String {
        self.envelope.build_agent_launch_command(
            agent_name,
            self.ipc_socket,
            &config.binary,
            &config.args,
            &config.model,
            agent_type,
        )
    }

    /// Build a structured PTY spawn bundle for direct spawn (preferred daemon path).
    pub fn build_pty_command(
        &self,
        agent_name: &str,
        agent_type: &str,
        config: &AgentConfig,
        ipc_hmac_key: Option<&[u8]>,
    ) -> AgentPtySpawn {
        agent_spawn::build_agent_pty_command(
            self.envelope,
            self.ipc_socket,
            agent_name,
            agent_type,
            &config.binary,
            &config.args,
            &config.model,
            ipc_hmac_key,
        )
    }
}
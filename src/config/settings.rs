use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level BMUX configuration loaded from config.toml.
///
/// All fields have sensible defaults so the file is optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BmuxConfig {
    /// Prefix key combination (e.g. "ctrl-b")
    pub prefix: String,
    /// Scrollback buffer line limit per pane
    pub history_limit: usize,
    /// Default shell to launch in new panes
    pub default_shell: String,
    /// Enable mouse support
    pub mouse_support: bool,
    /// `[tui]` section. Its values take precedence over the flat keys above.
    pub tui: TuiConfig,
    /// Agent-related configuration
    pub agents: AgentsConfig,
    /// Security controls (sandbox, dangerous CLI flags)
    pub security: SecurityConfig,
    /// Audit logging toggle
    pub audit: AuditConfig,
}

/// `[tui]` section of config.toml.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiConfig {
    /// Shell for new panes. Falls back to the top-level `default_shell`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_shell: Option<String>,
    /// Prefix key, e.g. `"ctrl-b"` or `"ctrl-a"`. Falls back to `prefix`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_key: Option<String>,
    /// Show the status bar at the bottom of the screen.
    pub status_bar: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            default_shell: None,
            prefix_key: None,
            status_bar: true,
        }
    }
}

impl Default for BmuxConfig {
    fn default() -> Self {
        Self {
            prefix: "ctrl-b".to_string(),
            history_limit: 10_000,
            default_shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()),
            mouse_support: false,
            tui: TuiConfig::default(),
            agents: AgentsConfig::default(),
            security: SecurityConfig::default(),
            audit: AuditConfig::default(),
        }
    }
}

/// Security-related runtime policies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Filesystem sandbox mode: landlock | namespace | seatbelt | none
    pub sandbox: String,
    /// Opt-in: allow `--dangerously-skip-permissions` for claude-code (not recommended)
    pub allow_dangerous_permissions: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            sandbox: "landlock".to_string(),
            allow_dangerous_permissions: false,
        }
    }
}

/// Audit logging configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditConfig {
    pub enabled: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl BmuxConfig {
    /// Load config from `path`. Returns `Err` on parse failure.
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Cannot read {}: {}", path.display(), e))?;
        let config: BmuxConfig =
            toml::from_str(&content).map_err(|e| anyhow::anyhow!("TOML parse error: {e}"))?;
        if crate::tui::keybindings::PrefixKey::parse(config.prefix_key()).is_none() {
            anyhow::bail!(
                "invalid prefix key '{}': use ctrl-<letter> or ctrl-space",
                config.prefix_key()
            );
        }
        Ok(config)
    }

    /// Load from `path`, falling back to defaults if the file is absent or unreadable.
    /// Returns the config and an optional error message for display.
    pub fn load_or_default(path: &Path) -> Self {
        if path.exists() {
            Self::load_from_path(path).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Hot-reload: try to load a new config from `path`.
    ///
    /// Returns `Ok(new_config)` on success.
    /// Returns `Err(message)` on failure — the caller retains the old config.
    pub fn hot_reload(path: &Path) -> Result<Self, String> {
        Self::load_from_path(path).map_err(|e| format!("{e}"))
    }

    /// Load config from the default path (~/.config/bmux/config.toml).
    pub fn load() -> anyhow::Result<Self> {
        // Same path as `bmux config show/validate/reload` and `Ctrl-b r`.
        let path = crate::config::default_config_path();
        if path.exists() {
            Self::load_from_path(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Return default agent config for a built-in agent type.
    pub fn default_agent_config(agent_type: &str) -> AgentSettingsConfig {
        crate::config::agent_catalog::resolve_agent_config(
            agent_type,
            &AgentsConfig::default(),
            false,
        )
    }

    /// Map of agent type names available (for workflow validation).
    pub fn agent_types(&self) -> std::collections::HashMap<String, AgentSettingsConfig> {
        crate::config::agent_catalog::all_builtin_types(
            &self.agents,
            self.security.allow_dangerous_permissions,
        )
    }

    /// Format config as a human-readable string for `bmux config show`.
    /// Effective prefix key: `[tui] prefix_key`, else top-level `prefix`.
    pub fn prefix_key(&self) -> &str {
        self.tui.prefix_key.as_deref().unwrap_or(&self.prefix)
    }

    /// Effective shell for new panes: `[tui] default_shell`, else `default_shell`.
    pub fn shell(&self) -> &str {
        self.tui
            .default_shell
            .as_deref()
            .unwrap_or(&self.default_shell)
    }

    /// Agent type used by `bmux agent spawn` when no type is given.
    pub fn default_agent_type(&self) -> &str {
        self.agents.default_type.as_deref().unwrap_or("claude-code")
    }

    pub fn display(&self) -> String {
        format!(
            "[tui]\n  prefix_key    = {}\n  default_shell = {}\n  status_bar    = {}\n[agents]\n  default_type       = {}\n  claude_code.binary = {}\n  opencode.binary    = {}\n  pi.binary          = {}\n[security]\n  sandbox = {}\n[audit]\n  enabled = {}",
            self.prefix_key(),
            self.shell(),
            self.tui.status_bar,
            self.default_agent_type(),
            self.agents.claude_code.binary,
            self.agents.opencode.binary,
            self.agents.pi.binary,
            self.security.sandbox,
            self.audit.enabled,
        )
    }
}

/// Agent binary configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentsConfig {
    /// Agent type for `bmux agent spawn` without a type (default: claude-code).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_type: Option<String>,
    pub claude_code: AgentBinaryConfig,
    pub opencode: AgentBinaryConfig,
    pub pi: AgentBinaryConfig,
    /// User-defined agents: `[agents.<name>]` sections in config.toml.
    #[serde(flatten)]
    pub custom: HashMap<String, CustomAgentConfig>,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            default_type: None,
            claude_code: AgentBinaryConfig {
                binary: "claude".to_string(),
            },
            opencode: AgentBinaryConfig {
                binary: "opencode".to_string(),
            },
            pi: AgentBinaryConfig {
                binary: "pi".to_string(),
            },
            custom: HashMap::new(),
        }
    }
}

impl AgentsConfig {
    /// Look up built-in agent settings by type name.
    pub fn get(
        &self,
        name: &str,
        allow_dangerous_permissions: bool,
    ) -> Option<AgentSettingsConfig> {
        if crate::config::agent_catalog::builtin_spec(name).is_some() {
            Some(crate::config::agent_catalog::resolve_agent_config(
                name,
                self,
                allow_dangerous_permissions,
            ))
        } else {
            None
        }
    }

    /// Look up a user-defined custom agent by name (`[agents.<name>]`).
    pub fn get_custom(&self, name: &str) -> Option<AgentSettingsConfig> {
        self.custom.get(name).map(CustomAgentConfig::to_settings)
    }
}

/// Full configuration for a user-defined custom agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomAgentConfig {
    pub binary: String,
    pub model: String,
    pub cost_per_1k_tokens: f64,
    pub args: Vec<String>,
}

impl Default for CustomAgentConfig {
    fn default() -> Self {
        Self {
            binary: String::new(),
            model: "unknown".to_string(),
            cost_per_1k_tokens: 0.0,
            args: vec![],
        }
    }
}

impl CustomAgentConfig {
    fn to_settings(&self) -> AgentSettingsConfig {
        AgentSettingsConfig {
            binary: self.binary.clone(),
            model: self.model.clone(),
            cost_per_1k_tokens: self.cost_per_1k_tokens,
            args: self.args.clone(),
        }
    }
}

/// Per-agent binary configuration block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgentBinaryConfig {
    pub binary: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn rejects_invalid_prefix_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[tui]\nprefix_key = \"ctrl-1\"\n").unwrap();
        assert!(BmuxConfig::load_from_path(&path).is_err());
        std::fs::write(&path, "[tui]\nprefix_key = \"ctrl-a\"\n").unwrap();
        assert_eq!(
            BmuxConfig::load_from_path(&path).unwrap().prefix_key(),
            "ctrl-a"
        );
    }

    #[test]
    fn tui_section_overrides_flat_keys() {
        let cfg: BmuxConfig = toml::from_str(
            r#"
prefix = "ctrl-b"
default_shell = "/bin/bash"

[tui]
default_shell = "/bin/zsh"
prefix_key    = "ctrl-a"
status_bar    = false

[agents]
default_type = "claude"

[agents.reviewer]
binary = "my-agent"
"#,
        )
        .unwrap();
        assert_eq!(cfg.prefix_key(), "ctrl-a");
        assert_eq!(cfg.shell(), "/bin/zsh");
        assert!(!cfg.tui.status_bar);
        assert_eq!(cfg.default_agent_type(), "claude");
        assert!(cfg.agents.get_custom("reviewer").is_some());
        assert!(cfg.agents.get_custom("default_type").is_none());
    }

    #[test]
    fn flat_keys_still_work_without_tui_section() {
        let cfg: BmuxConfig =
            toml::from_str("prefix = \"ctrl-a\"\ndefault_shell = \"/bin/fish\"\n").unwrap();
        assert_eq!(cfg.prefix_key(), "ctrl-a");
        assert_eq!(cfg.shell(), "/bin/fish");
        assert!(cfg.tui.status_bar);
        assert_eq!(cfg.default_agent_type(), "claude-code");
    }

    fn write_toml(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn default_config_has_sensible_values() {
        let cfg = BmuxConfig::default();
        assert_eq!(cfg.prefix, "ctrl-b");
        assert_eq!(cfg.history_limit, 10_000);
        assert!(!cfg.default_shell.is_empty());
        assert!(!cfg.mouse_support);
    }

    #[test]
    fn load_valid_toml() {
        let f = write_toml(
            r#"
prefix = "ctrl-a"
history_limit = 5000
default_shell = "/bin/zsh"
mouse_support = true
"#,
        );
        let cfg = BmuxConfig::load_from_path(f.path()).unwrap();
        assert_eq!(cfg.prefix, "ctrl-a");
        assert_eq!(cfg.history_limit, 5000);
        assert_eq!(cfg.default_shell, "/bin/zsh");
        assert!(cfg.mouse_support);
    }

    #[test]
    fn load_invalid_toml_returns_error() {
        let f = write_toml("this is not valid toml !!!");
        let result = BmuxConfig::load_from_path(f.path());
        assert!(result.is_err());
    }

    #[test]
    fn load_or_default_missing_file_uses_defaults() {
        let cfg = BmuxConfig::load_or_default(Path::new("/nonexistent/path/config.toml"));
        assert_eq!(cfg.prefix, "ctrl-b");
        assert_eq!(cfg.history_limit, 10_000);
    }

    #[test]
    fn load_or_default_invalid_file_uses_defaults() {
        let f = write_toml("invalid !!!");
        let cfg = BmuxConfig::load_or_default(f.path());
        assert_eq!(cfg.prefix, "ctrl-b");
    }

    #[test]
    fn hot_reload_success() {
        let f = write_toml(r#"prefix = "ctrl-x""#);
        let result = BmuxConfig::hot_reload(f.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().prefix, "ctrl-x");
    }

    #[test]
    fn hot_reload_invalid_toml_returns_err() {
        let f = write_toml("bad toml {{{{");
        let result = BmuxConfig::hot_reload(f.path());
        assert!(result.is_err());
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing_fields() {
        let f = write_toml(r#"prefix = "ctrl-x""#);
        let cfg = BmuxConfig::load_from_path(f.path()).unwrap();
        assert_eq!(cfg.prefix, "ctrl-x");
        assert_eq!(cfg.history_limit, 10_000); // default
        assert!(!cfg.mouse_support); // default
    }

    #[test]
    fn display_contains_key_fields() {
        let cfg = BmuxConfig::default();
        let display = cfg.display();
        assert!(display.contains("prefix_key"));
        assert!(display.contains("default_shell"));
        assert!(display.contains("status_bar"));
        assert!(display.contains("default_type"));
    }

    #[test]
    fn agents_config_defaults() {
        let cfg = BmuxConfig::default();
        assert_eq!(cfg.agents.claude_code.binary, "claude");
        assert_eq!(cfg.agents.opencode.binary, "opencode");
        assert_eq!(cfg.agents.pi.binary, "pi");
    }

    #[test]
    fn agents_config_from_toml() {
        let f = write_toml(
            r#"
[agents.claude_code]
binary = "claude-custom"
"#,
        );
        let cfg = BmuxConfig::load_from_path(f.path()).unwrap();
        assert_eq!(cfg.agents.claude_code.binary, "claude-custom");
        assert_eq!(cfg.agents.opencode.binary, "opencode"); // default
    }
}

/// Alias for backward compatibility
pub type Settings = BmuxConfig;

/// Agent settings as expected by the agents module.
#[derive(Debug, Clone)]
pub struct AgentSettingsConfig {
    pub binary: String,
    pub model: String,
    pub cost_per_1k_tokens: f64,
    pub args: Vec<String>,
}

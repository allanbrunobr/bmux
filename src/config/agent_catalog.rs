//! Single source of truth for built-in agent type defaults (binary, model, cost, args).

use super::settings::{AgentSettingsConfig, AgentsConfig};

/// Static definition for a built-in agent type.
#[derive(Debug, Clone)]
pub struct BuiltinAgentSpec {
    pub agent_type: &'static str,
    pub binary: &'static str,
    pub model: &'static str,
    pub cost_per_1k_tokens: f64,
    pub default_args: &'static [&'static str],
}

const BUILTIN_AGENTS: &[BuiltinAgentSpec] = &[
    BuiltinAgentSpec {
        agent_type: "claude-code",
        binary: "claude",
        model: "claude-sonnet-4-20250514",
        cost_per_1k_tokens: 0.003,
        default_args: &[],
    },
    BuiltinAgentSpec {
        agent_type: "opencode",
        binary: "opencode",
        model: "gemini-2.5-pro",
        cost_per_1k_tokens: 0.00125,
        default_args: &[],
    },
    BuiltinAgentSpec {
        agent_type: "pi",
        binary: "pi",
        model: "claude-sonnet-4-20250514",
        cost_per_1k_tokens: 0.003,
        default_args: &[],
    },
    BuiltinAgentSpec {
        agent_type: "gemini",
        binary: "gemini",
        model: "gemini-2.5-pro",
        cost_per_1k_tokens: 0.001,
        default_args: &[],
    },
    BuiltinAgentSpec {
        agent_type: "shell",
        binary: "sh",
        model: "shell",
        cost_per_1k_tokens: 0.0,
        default_args: &[],
    },
];

/// Look up a built-in spec by type name.
pub fn builtin_spec(agent_type: &str) -> Option<&'static BuiltinAgentSpec> {
    BUILTIN_AGENTS.iter().find(|s| s.agent_type == agent_type)
}

/// Resolve agent settings, applying optional TOML binary overrides from `agents`.
pub fn resolve_agent_config(
    agent_type: &str,
    agents: &AgentsConfig,
    allow_dangerous_permissions: bool,
) -> AgentSettingsConfig {
    let spec = builtin_spec(agent_type).unwrap_or(&BuiltinAgentSpec {
        agent_type: "unknown",
        binary: "",
        model: "unknown",
        cost_per_1k_tokens: 0.0,
        default_args: &[],
    });

    let binary = match agent_type {
        "claude-code" => agents.claude_code.binary.clone(),
        "opencode" => agents.opencode.binary.clone(),
        "pi" => agents.pi.binary.clone(),
        _ => spec.binary.to_string(),
    };

    let mut args: Vec<String> = spec
        .default_args
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    if allow_dangerous_permissions && agent_type == "claude-code" {
        args.push("--dangerously-skip-permissions".to_string());
    }

    AgentSettingsConfig {
        binary: if binary.is_empty() {
            spec.binary.to_string()
        } else {
            binary
        },
        model: spec.model.to_string(),
        cost_per_1k_tokens: spec.cost_per_1k_tokens,
        args,
    }
}

/// All built-in types for workflow validation.
pub fn all_builtin_types(agents: &AgentsConfig, allow_dangerous_permissions: bool) -> std::collections::HashMap<String, AgentSettingsConfig> {
    BUILTIN_AGENTS
        .iter()
        .filter(|s| s.agent_type != "shell" && s.agent_type != "unknown")
        .map(|s| {
            (
                s.agent_type.to_string(),
                resolve_agent_config(s.agent_type, agents, allow_dangerous_permissions),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_permissions_opt_in_only() {
        let agents = AgentsConfig::default();
        let safe = resolve_agent_config("claude-code", &agents, false);
        assert!(!safe.args.iter().any(|a| a.contains("dangerously-skip-permissions")));

        let unsafe_cfg = resolve_agent_config("claude-code", &agents, true);
        assert!(unsafe_cfg
            .args
            .iter()
            .any(|a| a.contains("dangerously-skip-permissions")));
    }
}
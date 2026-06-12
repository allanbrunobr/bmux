// CLI command handlers for the `bmux agent` subcommand.
//
// Implements: spawn, list, status, kill
// Stories: 2.1, 2.2, 2.4, 2.5

use tabled::{Table, Tabled};

use crate::agents::adapter::AgentConfig;
use crate::agents::claude_code::ClaudeCodeAdapter;
use crate::agents::custom::CustomAdapter;
use crate::agents::gemini::GeminiAdapter;
use crate::agents::opencode::OpenCodeAdapter;
use crate::agents::pi::PiAdapter;
use crate::agents::registry::AgentRegistry;
use crate::agents::shell::ShellAdapter;
use crate::agents::AgentAdapter;
use crate::config::settings::BmuxConfig;

/// Row type for the `bmux agent list` table.
#[derive(Tabled)]
pub struct AgentListRow {
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "Type")]
    pub agent_type: String,
    #[tabled(rename = "Model")]
    pub model: String,
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "Cost (USD)")]
    pub cost: String,
}

/// Handle `bmux agent spawn --type <type> --name <name> [--model <model>]`.
///
/// Prints: "Spawning <type> agent '<name>' with model '<model>'..."
/// On success: "Agent '<name>' spawned successfully (pane: 🤖 [name])"
/// On binary not found: "Error: Agent binary '<binary>' not found in PATH"
pub async fn handle_spawn(
    registry: &mut AgentRegistry,
    agent_type: &str,
    name: &str,
    model_override: Option<&str>,
) -> anyhow::Result<()> {
    // Prevent duplicate names
    if registry.get(name).is_some() {
        anyhow::bail!("Agent '{}' already exists. Use a different name.", name);
    }

    // Load config for the agent type
    let bmux_config = BmuxConfig::load().unwrap_or_default();

    let settings_config = if agent_type == "custom" {
        // Custom agent: look up by name in config
        bmux_config
            .agents
            .get_custom(name)
            .ok_or_else(|| anyhow::anyhow!(
                "Custom agent '{}' not found in config.toml. Add [agents.{}] section.",
                name, name
            ))?
    } else {
        // Built-in agent type: start with defaults, check user config overrides
        bmux_config
            .agents
            .get(
                agent_type,
                bmux_config.security.allow_dangerous_permissions,
            )
            .unwrap_or_else(|| BmuxConfig::default_agent_config(agent_type))
    };

    // Convert settings config → adapter AgentConfig
    let mut config = AgentConfig {
        binary: settings_config.binary,
        model: settings_config.model,
        cost_per_1k_tokens: settings_config.cost_per_1k_tokens,
        args: settings_config.args,
    };

    // Apply model override if provided
    if let Some(m) = model_override {
        config.model = m.to_string();
    }

    let type_icon = agent_type_icon(agent_type);
    println!(
        "Spawning {} agent '{}' with model '{}'...",
        agent_type, name, config.model
    );

    // Create the adapter and spawn
    let spawn_result = spawn_agent(agent_type, name, &config).await;

    match spawn_result {
        Ok(_) => {
            registry.register(name, agent_type, config);
            println!(
                "Agent '{}' spawned successfully (pane: {} [{}])",
                name, type_icon, name
            );
            Ok(())
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("not found in PATH") {
                eprintln!("Error: Agent binary '{}' not found in PATH", config.binary);
            } else {
                eprintln!("Error: {}", err_str);
            }
            Err(e)
        }
    }
}

/// Actually spawn the agent process (separated for testability).
async fn spawn_agent(
    agent_type: &str,
    name: &str,
    config: &AgentConfig,
) -> anyhow::Result<Box<dyn AgentAdapter>> {
    let mut adapter: Box<dyn AgentAdapter> = match agent_type {
        "shell" => Box::new(ShellAdapter::new(name, &config.binary)),
        "claude-code" => Box::new(ClaudeCodeAdapter::new(name, &config.model)),
        "opencode" => Box::new(OpenCodeAdapter::new(name, &config.model)),
        "pi" => Box::new(PiAdapter::new(name, &config.model)),
        "gemini" => Box::new(GeminiAdapter::new(name, &config.model)),
        "custom" => Box::new(CustomAdapter::from_config(name, config)),
        other => anyhow::bail!("Unknown agent type: '{}'. Valid types: shell, claude-code, opencode, pi, gemini, custom", other),
    };
    adapter.spawn(config).await?;
    Ok(adapter)
}

/// Handle `bmux agent list` — print a table of all agents.
pub fn handle_list(registry: &AgentRegistry) {
    let agents = registry.list();

    if agents.is_empty() {
        println!("No agents running. Use `bmux agent spawn` to start one.");
        return;
    }

    let rows: Vec<AgentListRow> = agents
        .iter()
        .map(|info| AgentListRow {
            name: info.name.clone(),
            agent_type: format!("{} {}", info.type_icon(), info.agent_type),
            model: info.model.clone(),
            status: info.status.label(),
            cost: format!("${:.6}", info.cost_usd),
        })
        .collect();

    let table = Table::new(rows).to_string();
    println!("{}", table);
}

/// Handle `bmux agent status <name>` — print detailed info about one agent.
pub fn handle_status(registry: &AgentRegistry, name: &str) -> anyhow::Result<()> {
    let info = registry
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Agent '{}' not found", name))?;

    println!("Agent: {} {} ({})", info.type_icon(), info.name, info.agent_type);
    println!("Model:        {}", info.model);
    println!("Status:       {}", info.status.label());
    println!("Uptime:       {}s", info.uptime_seconds());
    println!("Tasks run:    {}", info.task_count);
    println!("Tokens used:  {}", info.tokens_used);
    println!("Cost:         ${:.6}", info.cost_usd);
    if let Some(ref last) = info.last_task {
        let preview = if last.len() > 60 {
            format!("{}...", &last[..60])
        } else {
            last.clone()
        };
        println!("Last task:    {}", preview);
    } else {
        println!("Last task:    (none)");
    }
    Ok(())
}

/// Handle `bmux agent kill <name>` — terminate the agent and remove from registry.
pub async fn handle_kill(registry: &mut AgentRegistry, name: &str) -> anyhow::Result<()> {
    // Attempt to remove; returns error with name if not found
    match registry.remove(name) {
        Ok(info) => {
            println!(
                "Agent '{}' ({}) terminated. Pane removed.",
                info.name, info.agent_type
            );
            tracing::info!(name = %name, "Agent killed via CLI");
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: Agent '{}' not found", name);
            Err(e)
        }
    }
}

/// Handle `bmux agent dashboard` — print cost dashboard (triggered by Ctrl-b $).
pub fn handle_dashboard(registry: &AgentRegistry) {
    let stats = registry.aggregate_stats();

    println!("╔══════════════════════════════════════════════════╗");
    println!("║           BMUX Agent Cost Dashboard              ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!("║  Session Duration: {}s", stats.session_seconds);
    println!("║  Total Agents:     {}", stats.total_agents);
    println!("║  Active Agents:    {}", stats.active_agents);
    println!("║  Total Tokens:     {}", stats.total_tokens);
    println!("║  Total Cost:       ${:.6}", stats.total_cost);
    println!("╠══════════════════════════════════════════════════╣");

    let agents = registry.list();
    if agents.is_empty() {
        println!("║  No agents running.");
    } else {
        for info in agents {
            println!(
                "║  {} {} | tasks: {} | tokens: {} | cost: ${:.6}",
                info.type_icon(),
                info.name,
                info.task_count,
                info.tokens_used,
                info.cost_usd
            );
        }
    }
    println!("╚══════════════════════════════════════════════════╝");
}

/// Returns the type icon for a given agent type string.
pub fn agent_type_icon(agent_type: &str) -> &'static str {
    match agent_type {
        "claude-code" => "🤖",
        "opencode" => "✨",
        "pi" => "🥧",
        "gemini" => "✨",
        "shell" => "🔧",
        "custom" => "🔌",
        _ => "🔌",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::adapter::AgentConfig;
    use crate::agents::registry::AgentRegistry;

    fn make_registry_with_agents() -> AgentRegistry {
        let mut registry = AgentRegistry::new();
        registry.register(
            "arch",
            "claude-code",
            AgentConfig {
                binary: "claude".to_string(),
                model: "claude-sonnet-4-5".to_string(),
                cost_per_1k_tokens: 0.003,
                args: vec![],
            },
        );
        registry.register(
            "tests",
            "opencode",
            AgentConfig {
                binary: "opencode".to_string(),
                model: "gemini-2.5-pro".to_string(),
                cost_per_1k_tokens: 0.001,
                args: vec![],
            },
        );
        registry.record_usage("arch", 1000, 0.003).unwrap();
        registry
    }

    #[test]
    fn test_handle_list_empty() {
        let registry = AgentRegistry::new();
        // Should not panic
        handle_list(&registry);
    }

    #[test]
    fn test_handle_list_with_agents() {
        let registry = make_registry_with_agents();
        handle_list(&registry);
    }

    #[test]
    fn test_handle_status_existing_agent() {
        let registry = make_registry_with_agents();
        let result = handle_status(&registry, "arch");
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_status_nonexistent_returns_error() {
        let registry = AgentRegistry::new();
        let err = handle_status(&registry, "nonexistent").unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[tokio::test]
    async fn test_handle_kill_existing_agent() {
        let mut registry = make_registry_with_agents();
        let result = handle_kill(&mut registry, "arch").await;
        assert!(result.is_ok());
        assert!(registry.get("arch").is_none());
    }

    #[tokio::test]
    async fn test_handle_kill_nonexistent_returns_error() {
        let mut registry = AgentRegistry::new();
        let err = handle_kill(&mut registry, "nonexistent").await.unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn test_handle_dashboard_output() {
        let registry = make_registry_with_agents();
        // Should not panic
        handle_dashboard(&registry);
    }

    #[test]
    fn test_agent_type_icon() {
        assert_eq!(agent_type_icon("claude-code"), "🤖");
        assert_eq!(agent_type_icon("opencode"), "✨");
        assert_eq!(agent_type_icon("pi"), "🥧");
        assert_eq!(agent_type_icon("shell"), "🔧");
        assert_eq!(agent_type_icon("custom"), "🔌");
        assert_eq!(agent_type_icon("unknown"), "🔌");
    }

    #[tokio::test]
    async fn test_handle_spawn_duplicate_name_fails() {
        let mut registry = AgentRegistry::new();
        registry.register(
            "arch",
            "claude-code",
            AgentConfig {
                binary: "claude".to_string(),
                model: "claude-sonnet-4-5".to_string(),
                cost_per_1k_tokens: 0.003,
                args: vec![],
            },
        );
        let result = handle_spawn(&mut registry, "shell", "arch", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_handle_spawn_unknown_type_fails() {
        let mut registry = AgentRegistry::new();
        let config = AgentConfig {
            binary: "some-binary".to_string(),
            model: "some-model".to_string(),
            cost_per_1k_tokens: 0.0,
            args: vec![],
        };
        let result = spawn_agent("unknown-type", "test", &config).await;
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Unknown agent type"), "got: {}", err_msg);
    }
}

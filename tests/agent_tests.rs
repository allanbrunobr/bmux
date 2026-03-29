// Integration tests for the agent system (Stories 2.1 - 2.6)

use bmux::agents::adapter::{AgentAdapter, AgentConfig, AgentStatus, Task};
use bmux::agents::shell::ShellAdapter;
use bmux::agents::registry::{AgentRegistry, AgentInfo};
use bmux::agents::claude_code::ClaudeCodeAdapter;
use bmux::agents::opencode::OpenCodeAdapter;
use bmux::agents::pi::PiAdapter;
use bmux::agents::custom::CustomAdapter;

// ─── Story 2.1: AgentAdapter Trait & Shell Agent ───────────────────────────

#[test]
fn test_agent_adapter_trait_methods_exist() {
    // Verify ShellAdapter implements all required AgentAdapter methods:
    // id, agent_type, model_name, cost_per_1k_tokens, spawn, send_task, recv_output, status, kill
    let agent = ShellAdapter::new("test-shell", "/bin/sh");
    assert_eq!(agent.id(), "test-shell");
    assert_eq!(agent.agent_type(), "shell");
    assert_eq!(agent.model_name(), "shell");
    assert_eq!(agent.cost_per_1k_tokens(), 0.0);
}

#[tokio::test]
async fn test_shell_adapter_initial_status_is_idle() {
    let agent = ShellAdapter::new("test", "/bin/sh");
    assert_eq!(agent.status().await, AgentStatus::Idle);
}

#[tokio::test]
async fn test_shell_adapter_spawn_and_kill() {
    let mut agent = ShellAdapter::new("test-spawn", "/bin/sh");
    let config = AgentConfig {
        binary: "/bin/sh".to_string(),
        model: "shell".to_string(),
        cost_per_1k_tokens: 0.0,
        args: vec![],
    };
    agent.spawn(&config).await.expect("Shell agent should spawn");
    assert_eq!(agent.status().await, AgentStatus::Idle);
    agent.kill().await.expect("Shell agent should be killable");
}

#[tokio::test]
async fn test_shell_adapter_send_task() {
    let mut agent = ShellAdapter::new("test-task", "/bin/sh");
    let config = AgentConfig {
        binary: "/bin/sh".to_string(),
        model: "shell".to_string(),
        cost_per_1k_tokens: 0.0,
        args: vec![],
    };
    agent.spawn(&config).await.expect("spawn ok");

    let task = Task {
        id: "task-001".to_string(),
        content: "echo hello".to_string(),
        priority: 1,
        preferred_model: None,
        max_cost_usd: None,
    };
    agent.send_task(&task).await.expect("send_task ok");
    agent.kill().await.ok();
}

// ─── Story 2.1: AgentRegistry ──────────────────────────────────────────────

#[test]
fn test_agent_registry_new_is_empty() {
    let registry = AgentRegistry::new();
    assert_eq!(registry.list().len(), 0);
}

#[test]
fn test_agent_registry_register_and_list() {
    let mut registry = AgentRegistry::new();
    let config = AgentConfig {
        binary: "/bin/sh".to_string(),
        model: "shell".to_string(),
        cost_per_1k_tokens: 0.0,
        args: vec![],
    };
    registry.register("my-shell", "shell", config);
    let agents = registry.list();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].name, "my-shell");
    assert_eq!(agents[0].agent_type, "shell");
}

#[test]
fn test_agent_registry_get_existing() {
    let mut registry = AgentRegistry::new();
    let config = AgentConfig {
        binary: "/bin/sh".to_string(),
        model: "shell".to_string(),
        cost_per_1k_tokens: 0.0,
        args: vec![],
    };
    registry.register("arch", "shell", config);
    let info = registry.get("arch");
    assert!(info.is_some());
    assert_eq!(info.unwrap().name, "arch");
}

#[test]
fn test_agent_registry_get_nonexistent_returns_none() {
    let registry = AgentRegistry::new();
    assert!(registry.get("nonexistent").is_none());
}

#[tokio::test]
async fn test_agent_registry_remove() {
    let mut registry = AgentRegistry::new();
    let config = AgentConfig {
        binary: "/bin/sh".to_string(),
        model: "shell".to_string(),
        cost_per_1k_tokens: 0.0,
        args: vec![],
    };
    registry.register("to-remove", "shell", config);
    assert_eq!(registry.list().len(), 1);
    let result = registry.remove("to-remove");
    assert!(result.is_ok());
    assert_eq!(registry.list().len(), 0);
}

#[test]
fn test_agent_registry_remove_nonexistent_errors() {
    let mut registry = AgentRegistry::new();
    let result = registry.remove("nonexistent");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("nonexistent"));
}

// ─── Story 2.2: AI Agents ──────────────────────────────────────────────────

#[test]
fn test_claude_code_adapter_type() {
    let agent = ClaudeCodeAdapter::new("arch", "claude-sonnet-4-5");
    assert_eq!(agent.agent_type(), "claude-code");
    assert_eq!(agent.model_name(), "claude-sonnet-4-5");
    assert_eq!(agent.id(), "arch");
}

#[test]
fn test_opencode_adapter_type() {
    let agent = OpenCodeAdapter::new("tests", "gemini-2.5-pro");
    assert_eq!(agent.agent_type(), "opencode");
    assert_eq!(agent.model_name(), "gemini-2.5-pro");
    assert_eq!(agent.id(), "tests");
}

#[test]
fn test_pi_adapter_type() {
    let agent = PiAdapter::new("docs", "claude-haiku-4-5");
    assert_eq!(agent.agent_type(), "pi");
    assert_eq!(agent.model_name(), "claude-haiku-4-5");
    assert_eq!(agent.id(), "docs");
}

#[tokio::test]
async fn test_spawn_missing_binary_returns_error() {
    let mut agent = ClaudeCodeAdapter::new("arch", "claude-sonnet-4-5");
    let config = AgentConfig {
        binary: "definitely-not-a-real-binary-xyz-12345".to_string(),
        model: "claude-sonnet-4-5".to_string(),
        cost_per_1k_tokens: 0.003,
        args: vec![],
    };
    let result = agent.spawn(&config).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found in PATH") || err_msg.contains("No such file"));
}

// ─── Story 2.3: Status Bar Data ────────────────────────────────────────────

#[test]
fn test_agent_info_icon_for_types() {
    use bmux::agents::registry::AgentInfo;
    let shell_info = AgentInfo::new("sh", "shell", "shell", 0.0);
    assert_eq!(shell_info.type_icon(), "🔧");

    let claude_info = AgentInfo::new("arch", "claude-code", "claude-sonnet-4-5", 0.003);
    assert_eq!(claude_info.type_icon(), "🤖");

    let opencode_info = AgentInfo::new("tests", "opencode", "gemini-2.5-pro", 0.001);
    assert_eq!(opencode_info.type_icon(), "✨");

    let pi_info = AgentInfo::new("docs", "pi", "claude-haiku-4-5", 0.0002);
    assert_eq!(pi_info.type_icon(), "🥧");

    let custom_info = AgentInfo::new("custom", "custom", "llama-3.1-70b", 0.0001);
    assert_eq!(custom_info.type_icon(), "🔌");
}

#[test]
fn test_agent_info_status_indicator() {
    use bmux::agents::registry::AgentInfo;
    let mut info = AgentInfo::new("arch", "claude-code", "claude-sonnet-4-5", 0.003);
    assert_eq!(info.status_indicator(), "○"); // idle = ○
    info.status = AgentStatus::Working;
    assert_eq!(info.status_indicator(), "●"); // working = ●
}

#[test]
fn test_registry_aggregate_stats() {
    let mut registry = AgentRegistry::new();
    let config1 = AgentConfig { binary: "claude".to_string(), model: "claude-sonnet-4-5".to_string(), cost_per_1k_tokens: 0.003, args: vec![] };
    let config2 = AgentConfig { binary: "opencode".to_string(), model: "gemini-2.5-pro".to_string(), cost_per_1k_tokens: 0.001, args: vec![] };
    registry.register("arch", "claude-code", config1);
    registry.register("tests", "opencode", config2);

    // Record some token usage
    registry.record_usage("arch", 1000, 0.003).unwrap();
    registry.record_usage("tests", 500, 0.0005).unwrap();

    let stats = registry.aggregate_stats();
    assert_eq!(stats.total_agents, 2);
    assert_eq!(stats.total_tokens, 1500);
    assert!((stats.total_cost - 0.0035).abs() < 1e-6);
}

// ─── Story 2.4: Kill & Error Handling ──────────────────────────────────────

#[test]
fn test_kill_nonexistent_agent_error_message() {
    let mut registry = AgentRegistry::new();
    let err = registry.remove("nonexistent").unwrap_err();
    assert!(err.to_string().contains("nonexistent"));
}

// ─── Story 2.5: CustomAdapter ──────────────────────────────────────────────

#[test]
fn test_custom_adapter_type_and_icon() {
    use bmux::agents::registry::AgentInfo;
    let agent = CustomAdapter::new("meu-agente", "meu-agente-cli", "llama-3.1-70b", 0.0001, vec!["--mode".to_string(), "interactive".to_string()]);
    assert_eq!(agent.agent_type(), "custom");
    assert_eq!(agent.model_name(), "llama-3.1-70b");
    assert_eq!(agent.id(), "meu-agente");
    assert_eq!(agent.cost_per_1k_tokens(), 0.0001);

    let info = AgentInfo::new("meu-agente", "custom", "llama-3.1-70b", 0.0001);
    assert_eq!(info.type_icon(), "🔌");
}

#[tokio::test]
async fn test_custom_adapter_spawn_missing_binary_error() {
    let mut agent = CustomAdapter::new(
        "meu-agente",
        "totally-nonexistent-binary-xyz",
        "llama-3.1-70b",
        0.0001,
        vec!["--mode".to_string(), "interactive".to_string()],
    );
    let config = AgentConfig {
        binary: "totally-nonexistent-binary-xyz".to_string(),
        model: "llama-3.1-70b".to_string(),
        cost_per_1k_tokens: 0.0001,
        args: vec!["--mode".to_string(), "interactive".to_string()],
    };
    let result = agent.spawn(&config).await;
    assert!(result.is_err());
}

// ─── Story 2.6: Session Persistence ────────────────────────────────────────

#[test]
fn test_agent_registry_serialize_deserialize() {
    use bmux::agents::registry::AgentRegistry;
    let mut registry = AgentRegistry::new();
    let config = AgentConfig {
        binary: "claude".to_string(),
        model: "claude-sonnet-4-5".to_string(),
        cost_per_1k_tokens: 0.003,
        args: vec![],
    };
    registry.register("arch", "claude-code", config.clone());
    registry.record_usage("arch", 1000, 0.003).unwrap();

    let json = registry.to_json().expect("serialize ok");
    let restored = AgentRegistry::from_json(&json).expect("deserialize ok");

    assert_eq!(restored.list().len(), 1);
    assert_eq!(restored.list()[0].name, "arch");
    assert_eq!(restored.list()[0].tokens_used, 1000);
}

#[test]
fn test_agent_registry_save_and_load_file() {
    use bmux::agents::registry::AgentRegistry;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let path = dir.path().join("agents.json");

    let mut registry = AgentRegistry::new();
    let config = AgentConfig {
        binary: "claude".to_string(),
        model: "claude-sonnet-4-5".to_string(),
        cost_per_1k_tokens: 0.003,
        args: vec![],
    };
    registry.register("arch", "claude-code", config);
    registry.save_to_file(&path).expect("save ok");

    let loaded = AgentRegistry::load_from_file(&path).expect("load ok");
    assert_eq!(loaded.list().len(), 1);
    assert_eq!(loaded.list()[0].name, "arch");
}

// AgentRegistry — manages spawned agents and their runtime state.
//
// Stores agent metadata and stats. Process handles are managed by each adapter.
// Provides serialization for session persistence (Story 2.6).

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agents::adapter::{AgentConfig, AgentStatus};
use crate::tui::status_bar::{AgentStatusProvider, AgentStatusRow, AggregateStats as StatusAggregateStats};

/// Serializable snapshot of a single agent's state (for persistence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub name: String,
    pub agent_type: String,
    pub model: String,
    pub cost_per_1k_tokens: f64,
    pub binary: String,
    pub args: Vec<String>,
    pub tokens_used: u64,
    pub cost_usd: f64,
    pub task_count: u64,
    pub last_task: Option<String>,
    pub spawned_at: DateTime<Utc>,
}

/// Runtime info about a registered agent (not persisted to process handles).
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub name: String,
    pub agent_type: String,
    pub model: String,
    pub cost_per_1k_tokens: f64,
    pub binary: String,
    pub args: Vec<String>,
    pub status: AgentStatus,
    pub tokens_used: u64,
    pub cost_usd: f64,
    pub task_count: u64,
    pub last_task: Option<String>,
    pub spawned_at: Instant,
}

impl AgentInfo {
    pub fn new(name: &str, agent_type: &str, model: &str, cost_per_1k_tokens: f64) -> Self {
        AgentInfo {
            name: name.to_string(),
            agent_type: agent_type.to_string(),
            model: model.to_string(),
            cost_per_1k_tokens,
            binary: String::new(),
            args: vec![],
            status: AgentStatus::Idle,
            tokens_used: 0,
            cost_usd: 0.0,
            task_count: 0,
            last_task: None,
            spawned_at: Instant::now(),
        }
    }

    /// Returns the display icon for this agent's type.
    pub fn type_icon(&self) -> &'static str {
        match self.agent_type.as_str() {
            "claude-code" => "🤖",
            "opencode" => "✨",
            "pi" => "🥧",
            "gemini" => "✨",
            "shell" => "🔧",
            "custom" => "🔌",
            _ => "🔌",
        }
    }

    /// Returns the status indicator: ● for working, ○ for everything else.
    pub fn status_indicator(&self) -> &'static str {
        self.status.indicator()
    }

    /// Returns uptime in seconds.
    pub fn uptime_seconds(&self) -> u64 {
        self.spawned_at.elapsed().as_secs()
    }

    /// Converts to a serializable snapshot (for session persistence).
    pub fn to_snapshot(&self) -> AgentSnapshot {
        AgentSnapshot {
            name: self.name.clone(),
            agent_type: self.agent_type.clone(),
            model: self.model.clone(),
            cost_per_1k_tokens: self.cost_per_1k_tokens,
            binary: self.binary.clone(),
            args: self.args.clone(),
            tokens_used: self.tokens_used,
            cost_usd: self.cost_usd,
            task_count: self.task_count,
            last_task: self.last_task.clone(),
            spawned_at: Utc::now(),
        }
    }
}

/// Aggregate statistics across all registered agents.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AggregateStats {
    pub total_agents: usize,
    pub active_agents: usize,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub session_seconds: u64,
}

/// Session snapshot for persistence (Story 2.6).
#[derive(Debug, Serialize, Deserialize)]
struct RegistrySnapshot {
    agents: Vec<AgentSnapshot>,
    session_started: DateTime<Utc>,
}

/// Manages the set of registered agents and their runtime stats.
pub struct AgentRegistry {
    agents: HashMap<String, AgentInfo>,
    session_started: Instant,
}

impl AgentRegistry {
    pub fn new() -> Self {
        AgentRegistry {
            agents: HashMap::new(),
            session_started: Instant::now(),
        }
    }

    /// Register a new agent entry (does not spawn the process).
    pub fn register(&mut self, name: &str, agent_type: &str, config: AgentConfig) {
        let mut info = AgentInfo::new(name, agent_type, &config.model, config.cost_per_1k_tokens);
        info.binary = config.binary;
        info.args = config.args;
        self.agents.insert(name.to_string(), info);
        tracing::debug!(name = %name, agent_type = %agent_type, "Agent registered");
    }

    /// Return all registered agents as a sorted list (by name).
    pub fn list(&self) -> Vec<&AgentInfo> {
        let mut list: Vec<&AgentInfo> = self.agents.values().collect();
        list.sort_by_key(|a| &a.name);
        list
    }

    /// Get a specific agent by name.
    pub fn get(&self, name: &str) -> Option<&AgentInfo> {
        self.agents.get(name)
    }

    /// Get a mutable reference to a specific agent.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut AgentInfo> {
        self.agents.get_mut(name)
    }

    /// Remove an agent from the registry.
    /// Returns an error if the agent is not found.
    pub fn remove(&mut self, name: &str) -> anyhow::Result<AgentInfo> {
        self.agents
            .remove(name)
            .ok_or_else(|| anyhow::anyhow!("Agent '{}' not found", name))
    }

    /// Record token usage and cost for an agent.
    pub fn record_usage(&mut self, name: &str, tokens: u64, cost: f64) -> anyhow::Result<()> {
        let info = self.agents.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Agent '{}' not found", name))?;
        info.tokens_used += tokens;
        info.cost_usd += cost;
        info.task_count += 1;
        Ok(())
    }

    /// Update an agent's status.
    pub fn set_status(&mut self, name: &str, status: AgentStatus) -> anyhow::Result<()> {
        let info = self.agents.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Agent '{}' not found", name))?;
        info.status = status;
        Ok(())
    }

    /// Record the last task description for an agent.
    pub fn set_last_task(&mut self, name: &str, task: impl Into<String>) -> anyhow::Result<()> {
        let info = self.agents.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Agent '{}' not found", name))?;
        info.last_task = Some(task.into());
        Ok(())
    }

    /// Compute aggregate stats across all agents.
    pub fn aggregate_stats(&self) -> AggregateStats {
        let mut stats = AggregateStats {
            total_agents: self.agents.len(),
            session_seconds: self.session_started.elapsed().as_secs(),
            ..Default::default()
        };
        for info in self.agents.values() {
            if info.status == AgentStatus::Working {
                stats.active_agents += 1;
            }
            stats.total_tokens += info.tokens_used;
            stats.total_cost += info.cost_usd;
        }
        stats
    }

    /// Serialize the registry to JSON for persistence.
    pub fn to_json(&self) -> anyhow::Result<String> {
        let snapshot = RegistrySnapshot {
            agents: self.agents.values().map(|a| a.to_snapshot()).collect(),
            session_started: Utc::now(),
        };
        serde_json::to_string_pretty(&snapshot).context("Failed to serialize agent registry")
    }

    /// Deserialize a registry from JSON (process handles not restored).
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let snapshot: RegistrySnapshot =
            serde_json::from_str(json).context("Failed to parse agent registry JSON")?;

        let mut registry = AgentRegistry::new();
        for agent_snap in snapshot.agents {
            let mut info = AgentInfo::new(
                &agent_snap.name,
                &agent_snap.agent_type,
                &agent_snap.model,
                agent_snap.cost_per_1k_tokens,
            );
            info.binary = agent_snap.binary;
            info.args = agent_snap.args;
            info.tokens_used = agent_snap.tokens_used;
            info.cost_usd = agent_snap.cost_usd;
            info.task_count = agent_snap.task_count;
            info.last_task = agent_snap.last_task;
            // Note: status is reset to Idle on reattach (process handles not restored)
            registry.agents.insert(agent_snap.name, info);
        }
        Ok(registry)
    }

    /// Save registry state to a JSON file (for session persistence).
    pub fn save_to_file(&self, path: &Path) -> anyhow::Result<()> {
        let json = self.to_json()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)
            .with_context(|| format!("Failed to write agent state to {}", path.display()))
    }

    /// Load registry state from a JSON file (for session reattach).
    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read agent state from {}", path.display()))?;
        Self::from_json(&json)
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentStatusProvider for AgentRegistry {
    fn agent_status_rows(&self) -> Vec<AgentStatusRow> {
        self.list()
            .into_iter()
            .map(|info| AgentStatusRow {
                icon: info.type_icon().to_string(),
                name: info.name.clone(),
                status_indicator: info.status_indicator().to_string(),
                model: info.model.clone(),
                cost_usd: info.cost_usd,
            })
            .collect()
    }

    fn aggregate_stats(&self) -> StatusAggregateStats {
        let stats = self.aggregate_stats();
        StatusAggregateStats {
            total_agents: stats.total_agents,
            active_agents: stats.active_agents,
            total_tokens: stats.total_tokens,
            total_cost: stats.total_cost,
            session_seconds: stats.session_seconds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(binary: &str, model: &str) -> AgentConfig {
        AgentConfig {
            binary: binary.to_string(),
            model: model.to_string(),
            cost_per_1k_tokens: 0.003,
            args: vec![],
        }
    }

    #[test]
    fn test_registry_starts_empty() {
        let registry = AgentRegistry::new();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_register_and_list() {
        let mut registry = AgentRegistry::new();
        registry.register("arch", "claude-code", make_config("claude", "claude-sonnet-4-5"));
        let list = registry.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "arch");
    }

    #[test]
    fn test_list_sorted_by_name() {
        let mut registry = AgentRegistry::new();
        registry.register("zzz", "shell", make_config("/bin/sh", "shell"));
        registry.register("aaa", "shell", make_config("/bin/sh", "shell"));
        registry.register("mmm", "shell", make_config("/bin/sh", "shell"));
        let names: Vec<&str> = registry.list().iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["aaa", "mmm", "zzz"]);
    }

    #[test]
    fn test_get_existing() {
        let mut registry = AgentRegistry::new();
        registry.register("arch", "claude-code", make_config("claude", "claude-sonnet-4-5"));
        assert!(registry.get("arch").is_some());
    }

    #[test]
    fn test_get_nonexistent_returns_none() {
        let registry = AgentRegistry::new();
        assert!(registry.get("nope").is_none());
    }

    #[test]
    fn test_remove_existing() {
        let mut registry = AgentRegistry::new();
        registry.register("arch", "claude-code", make_config("claude", "claude-sonnet-4-5"));
        registry.remove("arch").unwrap();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_error_contains_name() {
        let mut registry = AgentRegistry::new();
        let err = registry.remove("nonexistent").unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn test_record_usage_accumulates() {
        let mut registry = AgentRegistry::new();
        registry.register("arch", "claude-code", make_config("claude", "claude-sonnet-4-5"));
        registry.record_usage("arch", 1000, 0.003).unwrap();
        registry.record_usage("arch", 500, 0.0015).unwrap();
        let info = registry.get("arch").unwrap();
        assert_eq!(info.tokens_used, 1500);
        assert!((info.cost_usd - 0.0045).abs() < 1e-9);
        assert_eq!(info.task_count, 2);
    }

    #[test]
    fn test_aggregate_stats_totals() {
        let mut registry = AgentRegistry::new();
        registry.register("a", "claude-code", make_config("claude", "claude-sonnet-4-5"));
        registry.register("b", "opencode", make_config("opencode", "gemini-2.5-pro"));
        registry.record_usage("a", 1000, 0.003).unwrap();
        registry.record_usage("b", 500, 0.0005).unwrap();

        let stats = registry.aggregate_stats();
        assert_eq!(stats.total_agents, 2);
        assert_eq!(stats.total_tokens, 1500);
        assert!((stats.total_cost - 0.0035).abs() < 1e-9);
    }

    #[test]
    fn test_aggregate_active_agents_count() {
        let mut registry = AgentRegistry::new();
        registry.register("a", "claude-code", make_config("claude", "claude-sonnet-4-5"));
        registry.register("b", "opencode", make_config("opencode", "gemini-2.5-pro"));
        registry.set_status("a", AgentStatus::Working).unwrap();
        let stats = registry.aggregate_stats();
        assert_eq!(stats.active_agents, 1);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let mut registry = AgentRegistry::new();
        registry.register("arch", "claude-code", make_config("claude", "claude-sonnet-4-5"));
        registry.record_usage("arch", 1000, 0.003).unwrap();

        let json = registry.to_json().unwrap();
        let restored = AgentRegistry::from_json(&json).unwrap();

        assert_eq!(restored.list().len(), 1);
        assert_eq!(restored.get("arch").unwrap().tokens_used, 1000);
    }

    #[test]
    fn test_save_and_load_file() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let path = dir.path().join("agents.json");

        let mut registry = AgentRegistry::new();
        registry.register("arch", "claude-code", make_config("claude", "claude-sonnet-4-5"));
        registry.save_to_file(&path).unwrap();

        let loaded = AgentRegistry::load_from_file(&path).unwrap();
        assert_eq!(loaded.list().len(), 1);
    }

    #[test]
    fn test_agent_info_type_icons() {
        let tests = vec![
            ("claude-code", "🤖"),
            ("opencode", "✨"),
            ("pi", "🥧"),
            ("gemini", "✨"),
            ("shell", "🔧"),
            ("custom", "🔌"),
        ];
        for (agent_type, expected_icon) in tests {
            let info = AgentInfo::new("test", agent_type, "model", 0.0);
            assert_eq!(info.type_icon(), expected_icon, "icon for {}", agent_type);
        }
    }

    #[test]
    fn test_agent_info_status_indicator() {
        let mut info = AgentInfo::new("a", "shell", "shell", 0.0);
        assert_eq!(info.status_indicator(), "○");
        info.status = AgentStatus::Working;
        assert_eq!(info.status_indicator(), "●");
    }

    #[test]
    fn test_status_provider_rows() {
        let mut registry = AgentRegistry::new();
        registry.register("arch", "claude-code", make_config("claude", "claude-sonnet-4-5"));
        let rows = registry.agent_status_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "arch");
        assert_eq!(rows[0].icon, "🤖");
    }
}

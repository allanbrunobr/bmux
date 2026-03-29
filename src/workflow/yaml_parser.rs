/// YAML workflow parser and validator.
///
/// Parses workflow YAML into a `WorkflowDefinition`, validates agent types,
/// detects circular dependencies, and produces dry-run summaries.
///
/// Stories: 5.1
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("agent type '{0}' not configured in config.toml")]
    UnknownAgentType(String),
    #[error("circular dependency detected: {0}")]
    CircularDependency(String),
    #[error("unknown step dependency '{dep}' referenced in step '{step}'")]
    UnknownDependency { step: String, dep: String },
    #[error("duplicate step id '{0}'")]
    DuplicateStep(String),
    #[error("workflow has no steps")]
    NoSteps,
}

pub type Result<T> = std::result::Result<T, ParseError>;

// ─── YAML schema types ────────────────────────────────────────────────────────

/// The top-level workflow YAML definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub version: String,
    /// Map of logical agent name → agent spec
    pub agents: HashMap<String, AgentSpec>,
    pub steps: Vec<StepDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    #[serde(rename = "type")]
    pub agent_type: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDefinition {
    pub id: String,
    pub agent: String,
    pub prompt: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Optional: explicit context key for storing output.
    pub context_key: Option<String>,
    /// Optional: timeout in seconds for this step.
    pub timeout_seconds: Option<u64>,
}

// ─── Computed DAG ─────────────────────────────────────────────────────────────

/// A validated, topologically-ordered DAG ready for execution.
#[derive(Debug, Clone)]
pub struct WorkflowDag {
    pub definition: WorkflowDefinition,
    /// Topological ordering of step IDs (for display/planning).
    pub topo_order: Vec<String>,
    /// Adjacency list: step_id → steps that depend on it.
    pub dependents: HashMap<String, Vec<String>>,
    /// Inverse: step_id → steps it depends on.
    pub dependencies: HashMap<String, Vec<String>>,
}

// ─── Parser ───────────────────────────────────────────────────────────────────

pub struct WorkflowParser;

impl WorkflowParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse a workflow YAML file.
    pub fn parse_file(&self, path: &str) -> Result<WorkflowDag> {
        let content = std::fs::read_to_string(path)?;
        self.parse_str(&content)
    }

    /// Parse a workflow YAML string.
    pub fn parse_str(&self, yaml: &str) -> Result<WorkflowDag> {
        let definition: WorkflowDefinition = serde_yaml::from_str(yaml)?;
        self.build_dag(definition)
    }

    /// Build and validate the DAG from a definition.
    fn build_dag(&self, definition: WorkflowDefinition) -> Result<WorkflowDag> {
        if definition.steps.is_empty() {
            return Err(ParseError::NoSteps);
        }

        // Check for duplicate step IDs
        let mut seen_ids = HashSet::new();
        for step in &definition.steps {
            if !seen_ids.insert(step.id.clone()) {
                return Err(ParseError::DuplicateStep(step.id.clone()));
            }
        }

        // Collect all step IDs as a set for dependency validation
        let all_ids: HashSet<String> = definition.steps.iter().map(|s| s.id.clone()).collect();

        // Build dependency maps
        let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

        for step in &definition.steps {
            dependencies.insert(step.id.clone(), step.depends_on.clone());
            dependents.entry(step.id.clone()).or_default();

            for dep in &step.depends_on {
                if !all_ids.contains(dep) {
                    return Err(ParseError::UnknownDependency {
                        step: step.id.clone(),
                        dep: dep.clone(),
                    });
                }
                dependents.entry(dep.clone()).or_default().push(step.id.clone());
            }
        }

        // Topological sort (Kahn's algorithm) — also detects cycles
        let topo_order = self.kahn_topo_sort(&definition.steps, &dependencies)?;

        Ok(WorkflowDag {
            definition,
            topo_order,
            dependents,
            dependencies,
        })
    }

    /// Kahn's algorithm for topological sort; returns Err on cycle.
    fn kahn_topo_sort(
        &self,
        steps: &[StepDefinition],
        dependencies: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<String>> {
        let mut in_degree: HashMap<&str, usize> = steps
            .iter()
            .map(|s| (s.id.as_str(), s.depends_on.len()))
            .collect();

        // Build reverse map: dep → list of steps that depend on it
        let mut reverse: HashMap<&str, Vec<&str>> = HashMap::new();
        for step in steps {
            for dep in &step.depends_on {
                reverse.entry(dep.as_str()).or_default().push(step.id.as_str());
            }
        }

        // Start with steps that have no dependencies
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::new();

        while let Some(id) = queue.pop_front() {
            order.push(id.to_string());
            if let Some(dependents) = reverse.get(id) {
                for &dep in dependents {
                    let count = in_degree.get_mut(dep).unwrap();
                    *count -= 1;
                    if *count == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }

        if order.len() != steps.len() {
            // Cycle detected — find which steps are in the cycle
            let in_order: HashSet<&str> = order.iter().map(String::as_str).collect();
            let cycle_nodes: Vec<_> = steps
                .iter()
                .filter(|s| !in_order.contains(s.id.as_str()))
                .map(|s| s.id.as_str())
                .collect();

            let cycle_str = self.describe_cycle(&cycle_nodes, dependencies);
            return Err(ParseError::CircularDependency(cycle_str));
        }

        Ok(order)
    }

    /// Build a human-readable cycle description (e.g. "A → B → A").
    fn describe_cycle(
        &self,
        cycle_nodes: &[&str],
        dependencies: &HashMap<String, Vec<String>>,
    ) -> String {
        if cycle_nodes.is_empty() {
            return "(unknown cycle)".to_string();
        }
        // Walk from the first cycle node to find a cycle path
        let start = cycle_nodes[0];
        let mut path = vec![start.to_string()];
        let mut current = start;
        let node_set: HashSet<&str> = cycle_nodes.iter().copied().collect();

        for _ in 0..cycle_nodes.len() {
            if let Some(deps) = dependencies.get(current) {
                if let Some(next) = deps.iter().find(|d| node_set.contains(d.as_str())) {
                    if next == start {
                        path.push(start.to_string());
                        break;
                    }
                    path.push(next.clone());
                    current = next.as_str();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        path.join(" → ")
    }

    // ─── Validation ───────────────────────────────────────────────────────────

    /// Validate agent types against configured settings.
    pub fn validate_agent_types(
        &self,
        dag: &WorkflowDag,
        settings: &crate::config::settings::Settings,
    ) -> Result<()> {
        for (agent_name, spec) in &dag.definition.agents {
            if !settings.agent_types().is_empty()
                && !settings.agent_types().contains_key(&spec.agent_type)
            {
                return Err(ParseError::UnknownAgentType(spec.agent_type.clone()));
            }
            let _ = agent_name;
        }
        Ok(())
    }

    // ─── Dry-run summary ──────────────────────────────────────────────────────

    /// Produce a dry-run summary string (Story 5.1).
    pub fn dry_run_summary(
        &self,
        dag: &WorkflowDag,
        settings: &crate::config::settings::Settings,
    ) -> String {
        let mut lines = Vec::new();
        let def = &dag.definition;

        lines.push(format!("Workflow: {} (v{})", def.name, def.version));
        lines.push(format!("Steps: {}", def.steps.len()));

        // Agent types needed
        let agent_types: HashSet<_> = def.agents.values().map(|a| &a.agent_type).collect();
        lines.push(format!(
            "Agents needed: {}",
            agent_types
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));

        // Validate agent types (warn if mismatch)
        if !settings.agent_types().is_empty() {
            for spec in def.agents.values() {
                if !settings.agent_types().contains_key(&spec.agent_type) {
                    lines.push(format!(
                        "ERROR: Agent type '{}' not configured in config.toml",
                        spec.agent_type
                    ));
                }
            }
        }

        // Execution order
        lines.push(String::from("Execution order:"));
        for (i, step_id) in dag.topo_order.iter().enumerate() {
            let step = def.steps.iter().find(|s| &s.id == step_id).unwrap();
            let deps = if step.depends_on.is_empty() {
                "(no deps)".to_string()
            } else {
                format!("after [{}]", step.depends_on.join(", "))
            };
            lines.push(format!("  {}. {} [{}] {}", i + 1, step_id, step.agent, deps));
        }

        // Estimated cost placeholder
        lines.push(String::from("Estimated cost: N/A (agents not configured)"));
        lines.push(String::from("(dry-run) No agents spawned, no tasks sent."));

        lines.join("\n")
    }

    // ─── Workflow list ────────────────────────────────────────────────────────

    /// List available workflows in a directory (Story 5.1).
    pub fn list_workflows(&self, dir: &str) -> Result<Vec<WorkflowSummary>> {
        let path = Path::new(dir);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let mut summaries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                match self.parse_file(&file_path.to_string_lossy()) {
                    Ok(dag) => {
                        let agent_types: HashSet<_> = dag
                            .definition
                            .agents
                            .values()
                            .map(|a| a.agent_type.clone())
                            .collect();
                        summaries.push(WorkflowSummary {
                            name: dag.definition.name.clone(),
                            step_count: dag.definition.steps.len(),
                            agent_types: agent_types.into_iter().collect(),
                            file_path: file_path.to_string_lossy().to_string(),
                        });
                    }
                    Err(_) => {
                        // Skip invalid files
                    }
                }
            }
        }
        Ok(summaries)
    }
}

impl Default for WorkflowParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of a workflow for the `list` command.
#[derive(Debug, Clone)]
pub struct WorkflowSummary {
    pub name: String,
    pub step_count: usize,
    pub agent_types: Vec<String>,
    pub file_path: String,
}

impl fmt::Display for WorkflowSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} steps, agents: [{}]) — {}",
            self.name,
            self.step_count,
            self.agent_types.join(", "),
            self.file_path,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_YAML: &str = r#"
name: full-feature
version: "1.0"
agents:
  architect:
    type: claude-code
    model: claude-sonnet-4-5
  tester:
    type: opencode
    model: gemini-2.5-pro
steps:
  - id: design
    agent: architect
    prompt: "Design: {{ input.feature }}"
  - id: implement
    agent: architect
    depends_on: [design]
    prompt: "Implement based on: {{ context.workflow:design:output }}"
  - id: test
    agent: tester
    depends_on: [implement]
    prompt: "Write tests for: {{ context.workflow:implement:output }}"
  - id: document
    agent: tester
    depends_on: [implement]
    prompt: "Document: {{ context.workflow:implement:output }}"
"#;

    const CIRCULAR_YAML: &str = r#"
name: circular
version: "1.0"
agents:
  a:
    type: claude-code
steps:
  - id: stepA
    agent: a
    prompt: "A"
    depends_on: [stepB]
  - id: stepB
    agent: a
    prompt: "B"
    depends_on: [stepA]
"#;

    const MISSING_DEP_YAML: &str = r#"
name: missing-dep
version: "1.0"
agents:
  a:
    type: claude-code
steps:
  - id: stepA
    agent: a
    prompt: "A"
    depends_on: [nonexistent]
"#;

    fn parser() -> WorkflowParser {
        WorkflowParser::new()
    }

    // ── Story 5.1: parsing ────────────────────────────────────────────────────

    #[test]
    fn test_parse_valid_workflow() {
        let dag = parser().parse_str(VALID_YAML).unwrap();
        assert_eq!(dag.definition.name, "full-feature");
        assert_eq!(dag.definition.steps.len(), 4);
    }

    #[test]
    fn test_topo_order_design_before_implement() {
        let dag = parser().parse_str(VALID_YAML).unwrap();
        let design_pos = dag.topo_order.iter().position(|s| s == "design").unwrap();
        let impl_pos = dag.topo_order.iter().position(|s| s == "implement").unwrap();
        assert!(design_pos < impl_pos);
    }

    #[test]
    fn test_parallel_steps_both_after_implement() {
        let dag = parser().parse_str(VALID_YAML).unwrap();
        let impl_pos = dag.topo_order.iter().position(|s| s == "implement").unwrap();
        let test_pos = dag.topo_order.iter().position(|s| s == "test").unwrap();
        let doc_pos = dag.topo_order.iter().position(|s| s == "document").unwrap();
        assert!(impl_pos < test_pos);
        assert!(impl_pos < doc_pos);
    }

    #[test]
    fn test_circular_dependency_detected() {
        let err = parser().parse_str(CIRCULAR_YAML).unwrap_err();
        match err {
            ParseError::CircularDependency(msg) => {
                assert!(msg.contains("stepA") || msg.contains("stepB"));
            }
            other => panic!("expected CircularDependency, got {other}"),
        }
    }

    #[test]
    fn test_unknown_dependency_detected() {
        let err = parser().parse_str(MISSING_DEP_YAML).unwrap_err();
        assert!(matches!(err, ParseError::UnknownDependency { .. }));
    }

    #[test]
    fn test_duplicate_step_id_detected() {
        let yaml = r#"
name: dup
version: "1.0"
agents:
  a:
    type: claude-code
steps:
  - id: stepA
    agent: a
    prompt: "A"
  - id: stepA
    agent: a
    prompt: "A again"
"#;
        let err = parser().parse_str(yaml).unwrap_err();
        assert!(matches!(err, ParseError::DuplicateStep(_)));
    }

    #[test]
    fn test_no_steps_error() {
        let yaml = r#"
name: empty
version: "1.0"
agents:
  a:
    type: claude-code
steps: []
"#;
        let err = parser().parse_str(yaml).unwrap_err();
        assert!(matches!(err, ParseError::NoSteps));
    }

    // ── Story 5.1: agent type validation ────────────────────────────────────

    #[test]
    fn test_unknown_agent_type_rejected() {
        // Default Settings knows claude-code, opencode, pi — but not "unknown-type"
        // VALID_YAML uses claude-code and opencode which are known, so we test with
        // a YAML that references an unknown type.
        let yaml = r#"
name: bad-workflow
version: "1.0"
steps:
  - id: step1
    agent: agent1
    prompt: "do something"
agents:
  agent1:
    type: totally-unknown-agent
"#;
        let settings = crate::config::settings::Settings::default();
        let dag = parser().parse_str(yaml).unwrap();
        let err = parser().validate_agent_types(&dag, &settings).unwrap_err();
        assert!(matches!(err, ParseError::UnknownAgentType(_)));
    }

    #[test]
    fn test_known_agent_types_pass_validation() {
        // Default Settings includes claude-code and opencode
        let settings = crate::config::settings::Settings::default();
        let dag = parser().parse_str(VALID_YAML).unwrap();
        parser().validate_agent_types(&dag, &settings).unwrap();
    }

    // ── Story 5.1: dry-run summary ───────────────────────────────────────────

    #[test]
    fn test_dry_run_summary_contains_step_count() {
        let dag = parser().parse_str(VALID_YAML).unwrap();
        let settings = crate::config::settings::Settings::default();
        let summary = parser().dry_run_summary(&dag, &settings);
        assert!(summary.contains("Steps: 4"));
        assert!(summary.contains("dry-run"));
    }

    #[test]
    fn test_dry_run_summary_mentions_agents_needed() {
        let dag = parser().parse_str(VALID_YAML).unwrap();
        let settings = crate::config::settings::Settings::default();
        let summary = parser().dry_run_summary(&dag, &settings);
        assert!(summary.contains("claude-code") || summary.contains("opencode"));
    }
}

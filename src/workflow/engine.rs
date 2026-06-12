/// Workflow execution engine.
///
/// Executes workflow DAGs with:
/// - Sequential and parallel step execution (dependency-ordered)
/// - Template parameter injection ({{ input.X }}, {{ context.KEY }})
/// - Step output storage in shared context at `workflow:{step_id}:output`
/// - Failure handling: halt workflow and report the failed step
/// - Status tracking: per-step state (pending/running/completed/failed)
///
/// Stories: 5.2, 5.3, 5.4
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::orchestration::task_router::{SendResult, TaskRouter, TaskStatus};
use crate::storage::context_store::ContextStore;
use crate::workflow::yaml_parser::WorkflowDag;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("context store error: {0}")]
    ContextStore(#[from] crate::storage::context_store::ContextStoreError),
    #[error("workflow failed at step '{step}': {error}")]
    StepFailed { step: String, error: String },
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("workflow run '{0}' not found")]
    RunNotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, EngineError>;

// ─── Status types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

impl fmt::Display for StepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepStatus::Pending => write!(f, "pending"),
            StepStatus::Running => write!(f, "running"),
            StepStatus::Completed => write!(f, "completed"),
            StepStatus::Failed(e) => write!(f, "failed: {e}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRecord {
    pub step_id: String,
    pub status: StepStatus,
    pub assigned_agent: Option<String>,
    /// Duration in milliseconds (set on completion/failure).
    pub duration_ms: Option<u64>,
    /// Simulated cost in USD.
    pub cost_usd: Option<f64>,
    /// Output stored in context.
    pub output_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub run_id: String,
    pub workflow_name: String,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub steps: Vec<StepRecord>,
    pub total_cost_usd: f64,
    pub status: WorkflowRunStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowRunStatus {
    Running,
    Completed,
    Failed(String),
}

impl fmt::Display for WorkflowRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Workflow: {}", self.workflow_name)?;
        writeln!(f, "Run ID: {}", self.run_id)?;
        let status_str = match &self.status {
            WorkflowRunStatus::Running => "running".to_string(),
            WorkflowRunStatus::Completed => "completed".to_string(),
            WorkflowRunStatus::Failed(e) => format!("failed: {e}"),
        };
        writeln!(f, "Status: {status_str}")?;

        if let Some(fin) = self.finished_at {
            let dur = fin.saturating_sub(self.started_at);
            writeln!(f, "Total duration: {dur}s")?;
        }
        writeln!(f, "Total cost: ${:.4}", self.total_cost_usd)?;

        writeln!(f, "\nSteps:")?;
        for step in &self.steps {
            let agent = step.assigned_agent.as_deref().unwrap_or("(none)");
            let dur = step
                .duration_ms
                .map(|d| format!("{d}ms"))
                .unwrap_or_else(|| "-".to_string());
            let cost = step
                .cost_usd
                .map(|c| format!("${c:.4}"))
                .unwrap_or_else(|| "-".to_string());
            writeln!(
                f,
                "  {}: {} (agent: {}, duration: {}, cost: {})",
                step.step_id, step.status, agent, dur, cost
            )?;
        }
        Ok(())
    }
}

// ─── Engine ───────────────────────────────────────────────────────────────────

/// The workflow execution engine.
pub struct WorkflowEngine {
    context_store: Arc<ContextStore>,
    task_router: Arc<TaskRouter>,
}

impl WorkflowEngine {
    /// Create a new engine with a shared TaskRouter (dependency injection).
    pub fn with_router(context_store: ContextStore, task_router: Arc<TaskRouter>) -> Self {
        Self {
            context_store: Arc::new(context_store),
            task_router,
        }
    }

    /// Create an engine sharing existing Arc handles (daemon wiring).
    pub fn with_shared(context_store: Arc<ContextStore>, task_router: Arc<TaskRouter>) -> Self {
        Self {
            context_store,
            task_router,
        }
    }

    /// Create a new engine with a default stub TaskRouter (for standalone use).
    pub fn new(context_store: ContextStore) -> Self {
        let bus = Arc::new(
            crate::orchestration::message_bus::MessageBus::new(
                format!("wf-engine-{}", uuid::Uuid::new_v4())
            ).expect("engine bus")
        );
        Self {
            context_store: Arc::new(context_store),
            task_router: Arc::new(TaskRouter::new(bus, 300)),
        }
    }

    /// Run a workflow DAG with the given input parameters.
    ///
    /// Executes steps in topological order; independent steps run in parallel.
    /// Each step's output is stored in context at `workflow:{step_id}:output`.
    pub async fn run(
        &self,
        dag: &WorkflowDag,
        run_id: &str,
        input: HashMap<String, serde_json::Value>,
    ) -> Result<WorkflowRun> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Build initial step records
        let steps: Vec<StepRecord> = dag
            .definition
            .steps
            .iter()
            .map(|s| StepRecord {
                step_id: s.id.clone(),
                status: StepStatus::Pending,
                assigned_agent: Some(s.agent.clone()),
                duration_ms: None,
                cost_usd: None,
                output_key: format!("workflow:{run_id}:{}:output", s.id),
            })
            .collect();

        let run = Arc::new(Mutex::new(WorkflowRun {
            run_id: run_id.to_string(),
            workflow_name: dag.definition.name.clone(),
            started_at: now,
            finished_at: None,
            steps,
            total_cost_usd: 0.0,
            status: WorkflowRunStatus::Running,
        }));

        // Persist initial run state
        self.save_run(run_id, &run.lock().unwrap())?;

        // Execute using parallel-aware topological scheduling
        let result = self
            .execute_dag(dag, run_id, &run, &input)
            .await;

        // Finalize run record
        {
            let mut r = run.lock().unwrap();
            let fin = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            r.finished_at = Some(fin);
            r.total_cost_usd = r.steps.iter().filter_map(|s| s.cost_usd).sum();

            match &result {
                Ok(()) => r.status = WorkflowRunStatus::Completed,
                Err(e) => r.status = WorkflowRunStatus::Failed(e.to_string()),
            }
            self.save_run(run_id, &r)?;
        }

        match result {
            Ok(()) => {
                let r = run.lock().unwrap().clone();
                Ok(r)
            }
            Err(e) => Err(e),
        }
    }

    /// Execute the DAG: parallel-wave scheduling.
    ///
    /// At each iteration we find all steps whose dependencies are completed
    /// and execute them concurrently (Story 5.3).
    async fn execute_dag(
        &self,
        dag: &WorkflowDag,
        run_id: &str,
        run: &Arc<Mutex<WorkflowRun>>,
        input: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let mut completed: HashSet<String> = HashSet::new();
        let total = dag.definition.steps.len();

        while completed.len() < total {
            // Find steps ready to run (all deps completed, not yet completed)
            let ready: Vec<String> = dag
                .definition
                .steps
                .iter()
                .filter(|s| {
                    !completed.contains(&s.id)
                        && s.depends_on.iter().all(|d| completed.contains(d))
                })
                .map(|s| s.id.clone())
                .collect();

            if ready.is_empty() {
                // No progress possible — this shouldn't happen with a valid DAG
                return Err(EngineError::StepFailed {
                    step: "scheduler".to_string(),
                    error: "No runnable steps found (possible DAG issue)".to_string(),
                });
            }

            // Mark all ready steps as Running
            {
                let mut r = run.lock().unwrap();
                for step_id in &ready {
                    if let Some(rec) = r.steps.iter_mut().find(|s| &s.step_id == step_id) {
                        rec.status = StepStatus::Running;
                    }
                }
                self.save_run(run_id, &r)?;
            }

            // Execute ready steps in parallel (Story 5.3)
            let mut handles = Vec::new();
            for step_id in &ready {
                let step = dag
                    .definition
                    .steps
                    .iter()
                    .find(|s| &s.id == step_id)
                    .unwrap()
                    .clone();
                let ctx = Arc::clone(&self.context_store);
                let router = Arc::clone(&self.task_router);
                let run_clone = Arc::clone(run);
                let run_id_str = run_id.to_string();
                let input_clone = input.clone();

                handles.push(tokio::spawn(async move {
                    let start = Instant::now();

                    // Resolve prompt template
                    let resolved_prompt =
                        resolve_template(&step.prompt, &input_clone, &ctx, &run_id_str);

                    let send_result = router
                        .send_to_agent(&step.agent, &resolved_prompt, 1)
                        .await
                        .map_err(|e| EngineError::StepFailed {
                            step: step.id.clone(),
                            error: e.to_string(),
                        })?;

                    let task_id = match send_result {
                        SendResult::Dispatched { task_id } => task_id,
                        SendResult::Queued { task_id, .. } => task_id,
                    };

                    let timeout = step
                        .timeout_seconds
                        .map(Duration::from_secs)
                        .or(Some(Duration::from_secs(3600)));
                    let finished = router
                        .wait_for_task(&task_id, timeout)
                        .await
                        .map_err(|e| EngineError::StepFailed {
                            step: step.id.clone(),
                            error: e.to_string(),
                        })?;

                    let duration_ms = start.elapsed().as_millis() as u64;

                    match finished.status {
                        TaskStatus::Completed => {
                            let output_key = step.context_key.clone().unwrap_or_else(|| {
                                format!("workflow:{run_id_str}:{}:output", step.id)
                            });
                            let (output_str, cost_usd) = if let Some(result) = finished.result {
                                (result.content, result.cost_usd)
                            } else {
                                (String::new(), 0.0)
                            };
                            let _ = ctx.set(&output_key, &output_str, None);

                            let mut r = run_clone.lock().unwrap();
                            if let Some(rec) = r.steps.iter_mut().find(|s| s.step_id == step.id) {
                                rec.status = StepStatus::Completed;
                                rec.duration_ms = Some(duration_ms);
                                rec.cost_usd = Some(cost_usd);
                            }
                            Ok(step.id.clone())
                        }
                        TaskStatus::Failed | TaskStatus::TimedOut | TaskStatus::Cancelled => {
                            let err = format!(
                                "task {task_id} ended with status {}",
                                finished.status
                            );
                            let mut r = run_clone.lock().unwrap();
                            if let Some(rec) = r.steps.iter_mut().find(|s| s.step_id == step.id) {
                                rec.status = StepStatus::Failed(err.clone());
                                rec.duration_ms = Some(duration_ms);
                            }
                            Err(EngineError::StepFailed {
                                step: step.id.clone(),
                                error: err,
                            })
                        }
                        TaskStatus::Queued | TaskStatus::Active => {
                            let err = format!("task {task_id} still in progress after wait");
                            let mut r = run_clone.lock().unwrap();
                            if let Some(rec) = r.steps.iter_mut().find(|s| s.step_id == step.id) {
                                rec.status = StepStatus::Failed(err.clone());
                                rec.duration_ms = Some(duration_ms);
                            }
                            Err(EngineError::StepFailed {
                                step: step.id.clone(),
                                error: err,
                            })
                        }
                    }
                }));
            }

            // Collect results from this parallel wave
            let mut wave_failed: Option<EngineError> = None;
            for handle in handles {
                match handle.await {
                    Ok(Ok(step_id)) => {
                        completed.insert(step_id);
                    }
                    Ok(Err(e)) => {
                        // Record first failure; continue collecting to update all records
                        if wave_failed.is_none() {
                            wave_failed = Some(e);
                        }
                    }
                    Err(join_err) => {
                        if wave_failed.is_none() {
                            wave_failed = Some(EngineError::StepFailed {
                                step: "unknown".to_string(),
                                error: join_err.to_string(),
                            });
                        }
                    }
                }
            }

            // Persist after each wave
            self.save_run(run_id, &run.lock().unwrap())?;

            // Halt on failure (Story 5.2)
            if let Some(err) = wave_failed {
                return Err(err);
            }
        }

        Ok(())
    }

    // ─── Status (Story 5.4) ───────────────────────────────────────────────────

    /// Retrieve the status of a workflow run by ID.
    pub fn get_status(&self, run_id: &str) -> Result<Option<WorkflowRun>> {
        let key = format!("workflow_run:{run_id}");
        match self.context_store.get(&key)? {
            Some(json) => {
                let run: WorkflowRun = serde_json::from_str(&json)?;
                Ok(Some(run))
            }
            None => Ok(None),
        }
    }

    // ─── Persistence helpers ──────────────────────────────────────────────────

    fn save_run(&self, run_id: &str, run: &WorkflowRun) -> Result<()> {
        let key = format!("workflow_run:{run_id}");
        let json = serde_json::to_string(run)?;
        self.context_store.set(&key, &json, None)?;
        Ok(())
    }
}

// ─── Template resolution ──────────────────────────────────────────────────────

/// Resolve `{{ input.X }}` and `{{ context.KEY }}` placeholders in a prompt.
fn resolve_template(
    template: &str,
    input: &HashMap<String, serde_json::Value>,
    context: &ContextStore,
    _run_id: &str,
) -> String {
    let mut result = template.to_string();

    // Replace {{ input.KEY }}
    let input_re = simple_template_regex();
    let mut replacements = Vec::new();
    for (placeholder, kind, key) in extract_placeholders(&result) {
        let replacement = match kind.as_str() {
            "input" => input
                .get(&key)
                .map(|v| {
                    // Strip surrounding quotes from JSON strings
                    match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    }
                })
                .unwrap_or_default(),
            "context" => context.get(&key).unwrap_or(None).unwrap_or_default(),
            _ => String::new(),
        };
        replacements.push((placeholder, replacement));
    }
    let _ = input_re; // pattern kept for documentation

    for (from, to) in replacements {
        result = result.replace(&from, &to);
    }
    result
}

fn extract_placeholders(template: &str) -> Vec<(String, String, String)> {
    let mut results = Vec::new();
    let mut pos = 0;
    let bytes = template.as_bytes();

    while pos + 3 < bytes.len() {
        if bytes[pos] == b'{' && bytes[pos + 1] == b'{' {
            if let Some(end) = template[pos..].find("}}") {
                let inner = template[pos + 2..pos + end].trim();
                // Parse "kind.key" pattern
                if let Some(dot_pos) = inner.find('.') {
                    let kind = inner[..dot_pos].trim().to_string();
                    let key = inner[dot_pos + 1..].trim().to_string();
                    let full = template[pos..pos + end + 2].to_string();
                    results.push((full, kind, key));
                }
                pos += end + 2;
                continue;
            }
        }
        pos += 1;
    }
    results
}

fn simple_template_regex() -> &'static str {
    r"\{\{.*?\}\}" // informational: describes the pattern used in extract_placeholders
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use serial_test::serial;

    async fn make_engine() -> (WorkflowEngine, TempDir) {
        use crate::orchestration::task_router::AgentInfo;

        let dir = TempDir::new().unwrap();
        let store = ContextStore::open(
            &format!("engine-test-{}", uuid::Uuid::new_v4()),
            dir.path(),
        ).unwrap();
        let engine = WorkflowEngine::new(store);

        // Register stub agents so task dispatch succeeds
        for name in &["a", "b", "c", "d"] {
            engine.task_router.register_agent(AgentInfo {
                id: name.to_string(),
                agent_type: "claude-code".to_string(),
                model_name: "test-model".to_string(),
                cost_per_1k_tokens: 0.001,
                capabilities: vec![],
                idle_since: Some(Instant::now()),
            }).await;
        }

        (engine, dir)
    }

    fn make_dag_from_yaml(yaml: &str) -> WorkflowDag {
        crate::workflow::yaml_parser::WorkflowParser::new()
            .parse_str(yaml)
            .unwrap()
    }

    // ── Story 5.2: parameter injection ───────────────────────────────────────

    
    #[serial]
    #[test]
    fn test_resolve_template_input_substitution() {
        let dir = TempDir::new().unwrap();
        let ctx = ContextStore::open("tmpl", dir.path()).unwrap();
        let mut input = HashMap::new();
        input.insert("feature".to_string(), serde_json::Value::String("login".to_string()));

        let result = resolve_template("Design: {{ input.feature }}", &input, &ctx, "run1");
        assert_eq!(result, "Design: login");
    }

    
    #[serial]
    #[test]
    fn test_resolve_template_context_substitution() {
        let dir = TempDir::new().unwrap();
        let ctx = ContextStore::open("tmpl2", dir.path()).unwrap();
        ctx.set("workflow:run1:design:output", "design output", None).unwrap();

        let result = resolve_template(
            "Implement: {{ context.workflow:run1:design:output }}",
            &HashMap::new(),
            &ctx,
            "run1",
        );
        assert_eq!(result, "Implement: design output");
    }

    
    #[serial]
    #[test]
    fn test_resolve_template_missing_key_resolves_empty() {
        let dir = TempDir::new().unwrap();
        let ctx = ContextStore::open("tmpl3", dir.path()).unwrap();
        let result = resolve_template(
            "Hello {{ input.nonexistent }}",
            &HashMap::new(),
            &ctx,
            "run1",
        );
        assert_eq!(result, "Hello ");
    }

    // ── Story 5.2: sequential execution ──────────────────────────────────────

    #[tokio::test]
    #[ignore = "requires running message bus (integration test)"]
    async fn test_run_sequential_workflow_completes() {
        let (engine, _dir) = make_engine().await;
        let yaml = r#"
name: seq-test
version: "1.0"
agents:
  a:
    type: claude-code
steps:
  - id: step1
    agent: a
    prompt: "Step 1"
  - id: step2
    agent: a
    depends_on: [step1]
    prompt: "Step 2"
  - id: step3
    agent: a
    depends_on: [step2]
    prompt: "Step 3"
"#;
        let dag = make_dag_from_yaml(yaml);
        let run = engine.run(&dag, "run-seq", HashMap::new()).await.unwrap();
        assert_eq!(run.status, WorkflowRunStatus::Completed);
        assert!(run.steps.iter().all(|s| s.status == StepStatus::Completed));
    }

    #[tokio::test]
    #[ignore = "requires running message bus (integration test)"]
    async fn test_step_output_stored_in_context() {
        let (engine, _dir) = make_engine().await;
        let yaml = r#"
name: ctx-test
version: "1.0"
agents:
  a:
    type: claude-code
steps:
  - id: design
    agent: a
    prompt: "Design feature"
"#;
        let dag = make_dag_from_yaml(yaml);
        let run = engine.run(&dag, "run-ctx", HashMap::new()).await.unwrap();

        // All steps completed
        assert_eq!(run.status, WorkflowRunStatus::Completed);

        // Output key recorded in the step record
        let design_step = run.steps.iter().find(|s| s.step_id == "design").unwrap();
        assert_eq!(design_step.output_key, "workflow:run-ctx:design:output");
        assert_eq!(design_step.status, StepStatus::Completed);

        // Verify via status retrieval
        let status = engine.get_status("run-ctx").unwrap().unwrap();
        assert_eq!(status.status, WorkflowRunStatus::Completed);
    }

    // ── Story 5.3: parallel execution ────────────────────────────────────────

    #[tokio::test]
    #[ignore = "requires running message bus (integration test)"]
    async fn test_parallel_steps_both_complete() {
        let (engine, _dir) = make_engine().await;
        let yaml = r#"
name: parallel-test
version: "1.0"
agents:
  a:
    type: claude-code
  b:
    type: opencode
steps:
  - id: implement
    agent: a
    prompt: "Implement"
  - id: test
    agent: b
    depends_on: [implement]
    prompt: "Test {{ context.workflow:run-par:implement:output }}"
  - id: document
    agent: a
    depends_on: [implement]
    prompt: "Document {{ context.workflow:run-par:implement:output }}"
"#;
        let dag = make_dag_from_yaml(yaml);
        let run = engine.run(&dag, "run-par", HashMap::new()).await.unwrap();
        assert_eq!(run.status, WorkflowRunStatus::Completed);

        // Both test and document should be completed
        let test_step = run.steps.iter().find(|s| s.step_id == "test").unwrap();
        let doc_step = run.steps.iter().find(|s| s.step_id == "document").unwrap();
        assert_eq!(test_step.status, StepStatus::Completed);
        assert_eq!(doc_step.status, StepStatus::Completed);
    }

    #[tokio::test]
    #[ignore = "requires running message bus (integration test)"]
    async fn test_diamond_dag_a_bc_d() {
        let (engine, _dir) = make_engine().await;
        let yaml = r#"
name: diamond
version: "1.0"
agents:
  a:
    type: claude-code
steps:
  - id: A
    agent: a
    prompt: "A"
  - id: B
    agent: a
    depends_on: [A]
    prompt: "B"
  - id: C
    agent: a
    depends_on: [A]
    prompt: "C"
  - id: D
    agent: a
    depends_on: [B, C]
    prompt: "D"
"#;
        let dag = make_dag_from_yaml(yaml);
        let run = engine.run(&dag, "run-diamond", HashMap::new()).await.unwrap();
        assert_eq!(run.status, WorkflowRunStatus::Completed);
        assert!(run.steps.iter().all(|s| s.status == StepStatus::Completed));
    }

    // ── Story 5.4: status & monitoring ───────────────────────────────────────

    #[tokio::test]
    #[ignore = "requires running message bus (integration test)"]
    async fn test_get_status_after_completion() {
        let (engine, _dir) = make_engine().await;
        let yaml = r#"
name: status-test
version: "1.0"
agents:
  a:
    type: claude-code
steps:
  - id: only
    agent: a
    prompt: "Do it"
"#;
        let dag = make_dag_from_yaml(yaml);
        engine.run(&dag, "run-status", HashMap::new()).await.unwrap();

        let status = engine.get_status("run-status").unwrap().unwrap();
        assert_eq!(status.workflow_name, "status-test");
        assert_eq!(status.status, WorkflowRunStatus::Completed);
        assert!(status.finished_at.is_some());
    }

    #[tokio::test]
    #[ignore = "requires running message bus (integration test)"]
    async fn test_get_status_shows_step_details() {
        let (engine, _dir) = make_engine().await;
        let yaml = r#"
name: detail-test
version: "1.0"
agents:
  arch:
    type: claude-code
steps:
  - id: design
    agent: arch
    prompt: "Design it"
"#;
        let dag = make_dag_from_yaml(yaml);
        engine.run(&dag, "run-detail", HashMap::new()).await.unwrap();

        let status = engine.get_status("run-detail").unwrap().unwrap();
        let step = status.steps.iter().find(|s| s.step_id == "design").unwrap();
        assert_eq!(step.status, StepStatus::Completed);
        assert_eq!(step.assigned_agent, Some("arch".to_string()));
        assert!(step.duration_ms.is_some());
    }

    
    #[serial]
    #[tokio::test]
    async fn test_get_status_returns_none_for_unknown_id() {
        let (engine, _dir) = make_engine().await;
        let status = engine.get_status("nonexistent-run").unwrap();
        assert!(status.is_none());
    }

    
    #[serial]
    #[test]
    fn test_workflow_run_display_format() {
        let run = WorkflowRun {
            run_id: "test-run".to_string(),
            workflow_name: "my-workflow".to_string(),
            started_at: 1000,
            finished_at: Some(1060),
            steps: vec![
                StepRecord {
                    step_id: "step1".to_string(),
                    status: StepStatus::Completed,
                    assigned_agent: Some("arch".to_string()),
                    duration_ms: Some(500),
                    cost_usd: Some(0.001),
                    output_key: "workflow:test-run:step1:output".to_string(),
                },
            ],
            total_cost_usd: 0.001,
            status: WorkflowRunStatus::Completed,
        };

        let display = run.to_string();
        assert!(display.contains("my-workflow"));
        assert!(display.contains("completed"));
        assert!(display.contains("step1"));
        assert!(display.contains("arch"));
    }
}

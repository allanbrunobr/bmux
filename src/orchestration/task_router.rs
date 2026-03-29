use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::orchestration::message_bus::{
    AgentState, MessageBus, MessageBuilder, MessageType, ResultPayload,
    TaskPayload,
};

// Canonical agent types from agents module

// ---------------------------------------------------------------------------
// Task & result data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Active,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Queued => write!(f, "queued"),
            TaskStatus::Active => write!(f, "active"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
            TaskStatus::TimedOut => write!(f, "timed_out"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Short task ID (8 hex chars from UUID).
    pub id: String,
    /// Full UUID of the task message.
    pub full_id: String,
    pub destination: String,
    pub content: String,
    pub priority: u8,
    pub preferred_model: Option<String>,
    pub max_cost_usd: Option<f64>,
    pub status: TaskStatus,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    pub queue_position: Option<usize>,
    pub result: Option<TaskResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub content: String,
    pub tokens_used: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// Agent info used by the router (decoupled from the full AgentAdapter trait)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: String,
    pub agent_type: String,
    pub model_name: String,
    pub cost_per_1k_tokens: f64,
    pub capabilities: Vec<String>,
    pub idle_since: Option<Instant>,
}

// ---------------------------------------------------------------------------
// TaskRouter
// ---------------------------------------------------------------------------

pub struct TaskRouter {
    bus: Arc<MessageBus>,
    /// All known agents registered with the router.
    agents: Arc<RwLock<HashMap<String, AgentInfo>>>,
    /// Task store keyed by short ID.
    tasks: Arc<RwLock<HashMap<String, Task>>>,
    /// Per-agent queues: agent_id → ordered list of queued full UUIDs.
    queues: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// How many seconds to wait before timing out a queued auto-route task.
    queue_timeout_seconds: u64,
}

impl TaskRouter {
    pub fn new(bus: Arc<MessageBus>, queue_timeout_seconds: u64) -> Self {
        Self {
            bus,
            agents: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            queues: Arc::new(RwLock::new(HashMap::new())),
            queue_timeout_seconds,
        }
    }

    /// Register a known agent with the router.
    pub async fn register_agent(&self, info: AgentInfo) {
        let mut agents = self.agents.write().await;
        let mut queues = self.queues.write().await;
        queues.entry(info.id.clone()).or_default();
        agents.insert(info.id.clone(), info);
    }

    // -----------------------------------------------------------------------
    // Story 3.2 — Send Task to Specific Agent
    // -----------------------------------------------------------------------

    /// Send a task to a specific agent by name.
    ///
    /// - If the agent is Idle: sends immediately, marks agent Working.
    /// - If the agent is Working: queues the task and returns queue position.
    pub async fn send_to_agent(
        &self,
        agent_id: &str,
        content: &str,
        priority: u8,
    ) -> Result<SendResult> {
        let agents = self.agents.read().await;
        if !agents.contains_key(agent_id) {
            return Err(anyhow!("Unknown agent '{agent_id}'"));
        }
        drop(agents);

        let agent_state = {
            let map = self.bus.agent_status().read().await.clone();
            map.get(agent_id).cloned().unwrap_or(AgentState::Idle)
        };

        let task = self.create_task(agent_id, content, priority, None, None);
        let task_id = task.id.clone();

        match agent_state {
            AgentState::Idle => {
                self.dispatch_task_now(task).await?;
                Ok(SendResult::Dispatched { task_id })
            }
            AgentState::Working | AgentState::WaitingInput => {
                let pos = self.enqueue_task(task).await;
                Ok(SendResult::Queued {
                    task_id,
                    agent_id: agent_id.to_string(),
                    position: pos,
                })
            }
            AgentState::Error(e) => {
                Err(anyhow!("Agent '{agent_id}' is in error state: {e}"))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Story 3.3 — Task Queue, List & Cancel
    // -----------------------------------------------------------------------

    /// List all tasks.
    pub async fn list_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        let mut list: Vec<Task> = tasks.values().cloned().collect();
        list.sort_by_key(|t| t.submitted_at);
        list
    }

    /// Cancel a queued task. Active or completed tasks cannot be cancelled.
    pub async fn cancel_task(&self, task_id: &str) -> Result<CancelResult> {
        let mut tasks = self.tasks.write().await;
        match tasks.get_mut(task_id) {
            None => Err(anyhow!("Task '{task_id}' not found")),
            Some(task) => match task.status {
                TaskStatus::Queued => {
                    task.status = TaskStatus::Cancelled;
                    // Remove from queue
                    let dest = task.destination.clone();
                    let full_id = task.full_id.clone();
                    drop(tasks);
                    let mut queues = self.queues.write().await;
                    if let Some(q) = queues.get_mut(&dest) {
                        q.retain(|id| id != &full_id);
                        // Recompute queue positions
                        self.recompute_positions_locked(&queues, &dest).await;
                    }
                    Ok(CancelResult::Cancelled)
                }
                TaskStatus::Active => Ok(CancelResult::AlreadyActive),
                _ => Err(anyhow!(
                    "Task '{task_id}' is in state '{}' and cannot be cancelled",
                    task.status
                )),
            },
        }
    }

    /// Get detailed status of a task.
    pub async fn task_status(&self, task_id: &str) -> Option<Task> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    // -----------------------------------------------------------------------
    // Story 3.4 — Auto-Route Tasks by Cost & Capability
    // -----------------------------------------------------------------------

    /// Auto-route a task to the best available agent using the scoring formula:
    ///   score = (capability_score × 0.5) + (cost_score × 0.3) + (idle_score × 0.2)
    ///
    /// If `preferred_model` is set, skip scoring and route directly to that
    /// model's agent (if idle).
    pub async fn send_auto(
        &self,
        content: &str,
        preferred_model: Option<&str>,
        max_cost_usd: Option<f64>,
    ) -> Result<AutoRouteResult> {
        if let Some(model) = preferred_model {
            return self.route_by_model(content, model).await;
        }

        let agent_status = self.bus.agent_status().read().await.clone();
        let agents = self.agents.read().await;

        let idle_agents: Vec<&AgentInfo> = agents
            .values()
            .filter(|a| {
                agent_status
                    .get(&a.id)
                    .map_or(true, |s| *s == AgentState::Idle)
            })
            .collect();

        if idle_agents.is_empty() {
            drop(agents);
            drop(agent_status);
            let task = self.create_task(
                "__auto__",
                content,
                1,
                preferred_model.map(str::to_string),
                max_cost_usd,
            );
            let task_id = task.id.clone();
            let timeout = self.queue_timeout_seconds;
            self.enqueue_task(task).await;

            // Spawn a timeout watcher
            let tasks = Arc::clone(&self.tasks);
            let watcher_task_id = task_id.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(timeout)).await;
                let mut t = tasks.write().await;
                if let Some(task) = t.get_mut(&watcher_task_id) {
                    if task.status == TaskStatus::Queued {
                        task.status = TaskStatus::TimedOut;
                        warn!(task_id = %watcher_task_id, "Auto-routed task timed out");
                    }
                }
            });

            return Ok(AutoRouteResult::AllBusy {
                task_id,
                timeout_seconds: timeout,
            });
        }

        // Compute scores
        let max_cost = idle_agents
            .iter()
            .map(|a| a.cost_per_1k_tokens)
            .fold(0.0_f64, f64::max);
        let now = Instant::now();
        let max_idle_secs: f64 = idle_agents
            .iter()
            .map(|a| {
                a.idle_since
                    .map(|t| now.duration_since(t).as_secs_f64())
                    .unwrap_or(0.0)
            })
            .fold(0.0_f64, f64::max);

        let best = idle_agents
            .iter()
            .max_by(|a, b| {
                let sa = compute_score(a, content, max_cost, max_idle_secs, now);
                let sb = compute_score(b, content, max_cost, max_idle_secs, now);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied();

        let best = best.ok_or_else(|| anyhow!("No idle agents available"))?;
        let estimated_cost = best.cost_per_1k_tokens * 0.001; // rough estimate for 1 token
        let agent_id = best.id.clone();
        drop(agents);
        drop(agent_status);

        let task = self.create_task(
            &agent_id,
            content,
            1,
            preferred_model.map(str::to_string),
            max_cost_usd,
        );
        let task_id = task.id.clone();
        self.dispatch_task_now(task).await?;

        Ok(AutoRouteResult::Dispatched {
            task_id,
            agent_id,
            estimated_cost,
        })
    }

    async fn route_by_model(&self, content: &str, model: &str) -> Result<AutoRouteResult> {
        let agent_status = self.bus.agent_status().read().await.clone();
        let agents = self.agents.read().await;

        let agent = agents
            .values()
            .find(|a| {
                a.model_name == model
                    && agent_status
                        .get(&a.id)
                        .map_or(true, |s| *s == AgentState::Idle)
            })
            .ok_or_else(|| anyhow!("No idle agent with model '{model}'"))?;

        let agent_id = agent.id.clone();
        let estimated_cost = agent.cost_per_1k_tokens * 0.001;
        drop(agents);
        drop(agent_status);

        let task = self.create_task(&agent_id, content, 1, Some(model.to_string()), None);
        let task_id = task.id.clone();
        self.dispatch_task_now(task).await?;

        Ok(AutoRouteResult::Dispatched {
            task_id,
            agent_id,
            estimated_cost,
        })
    }

    // -----------------------------------------------------------------------
    // Story 3.5 — Agent Publishes Task Results
    // -----------------------------------------------------------------------

    /// Handle a result published by an agent (via IPC Result message).
    pub async fn handle_result(
        &self,
        task_full_id: &str,
        result_payload: ResultPayload,
    ) -> Result<()> {
        let task_id = short_id(task_full_id);
        let mut tasks = self.tasks.write().await;

        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| anyhow!("Task '{task_id}' not found for result"))?;

        let task_result = TaskResult {
            content: result_payload.content,
            tokens_used: result_payload.tokens_used,
            cost_usd: result_payload.cost_usd,
            duration_ms: result_payload.duration_ms,
            completed_at: chrono::Utc::now(),
        };

        task.status = TaskStatus::Completed;
        task.result = Some(task_result);
        let agent_id = task.destination.clone();
        drop(tasks);

        // Return agent to idle
        {
            let status_arc = self.bus.agent_status();
            let mut status_map = status_arc.write().await;
            status_map.insert(agent_id.clone(), AgentState::Idle);
        }

        info!(task_id = %task_id, agent = %agent_id, "Task completed, agent returned to idle");

        // Dispatch next queued task for this agent if any
        self.dispatch_next_for_agent(&agent_id).await?;

        Ok(())
    }

    /// Mark an agent as crashed: sets status to Error, marks its active task failed.
    pub async fn handle_agent_crash(&self, agent_id: &str) {
        // Mark agent error
        {
            let status_arc = self.bus.agent_status();
            let mut status_map = status_arc.write().await;
            status_map.insert(
                agent_id.to_string(),
                AgentState::Error("process exited unexpectedly".into()),
            );
        }

        // Mark the active task as failed
        let mut tasks = self.tasks.write().await;
        for task in tasks.values_mut() {
            if task.destination == agent_id && task.status == TaskStatus::Active {
                task.status = TaskStatus::Failed;
                warn!(task_id = %task.id, agent = %agent_id, "Task failed due to agent crash");
            }
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn create_task(
        &self,
        destination: &str,
        content: &str,
        priority: u8,
        preferred_model: Option<String>,
        max_cost_usd: Option<f64>,
    ) -> Task {
        let full_id = uuid::Uuid::new_v4().to_string();
        Task {
            id: short_id(&full_id),
            full_id,
            destination: destination.to_string(),
            content: content.to_string(),
            priority,
            preferred_model,
            max_cost_usd,
            status: TaskStatus::Queued,
            submitted_at: chrono::Utc::now(),
            queue_position: None,
            result: None,
        }
    }

    async fn dispatch_task_now(&self, mut task: Task) -> Result<()> {
        task.status = TaskStatus::Active;
        task.queue_position = None;
        let task_id = task.id.clone();
        let agent_id = task.destination.clone();
        let content = task.content.clone();
        let priority = task.priority;

        // Store the task
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id.clone(), task);
        }

        // Mark agent as Working
        {
            let status_arc = self.bus.agent_status();
            let mut status_map = status_arc.write().await;
            status_map.insert(agent_id.clone(), AgentState::Working);
        }

        // Send via message bus (best-effort — bus may not be running for in-process use)
        let payload = serde_json::to_value(TaskPayload {
            content,
            priority,
            preferred_model: None,
            max_cost_usd: None,
        })?;
        let msg = MessageBuilder::new("orchestrator", &agent_id, MessageType::Task, payload)
            .build_signed(&self.bus.hmac_key())
            .map_err(|e| anyhow!("Failed to sign task message: {e}"))?;

        match self.bus.send_message(&msg).await {
            Ok(()) => info!(task_id = %task_id, agent = %agent_id, "Task dispatched via IPC"),
            Err(e) => {
                // Bus not running — task is still tracked in memory, just not delivered via socket.
                // The daemon can poll task state and deliver to agents through PTY stdin.
                info!(task_id = %task_id, agent = %agent_id, "Task dispatched (in-process, bus not active: {e})");
            }
        }
        Ok(())
    }

    async fn enqueue_task(&self, mut task: Task) -> usize {
        task.status = TaskStatus::Queued;
        let agent_id = task.destination.clone();
        let full_id = task.full_id.clone();
        let task_id = task.id.clone();

        let pos = {
            let mut queues = self.queues.write().await;
            let q = queues.entry(agent_id.clone()).or_default();
            q.push(full_id.clone());
            let pos = q.len();
            task.queue_position = Some(pos);
            pos
        };

        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id.clone(), task);
        }

        info!(task_id = %task_id, agent = %agent_id, position = pos, "Task queued");
        pos
    }

    async fn dispatch_next_for_agent(&self, agent_id: &str) -> Result<()> {
        let next_full_id = {
            let mut queues = self.queues.write().await;
            if let Some(q) = queues.get_mut(agent_id) {
                if q.is_empty() {
                    return Ok(());
                }
                let id = q.remove(0);
                self.recompute_positions_locked(&queues, agent_id).await;
                id
            } else {
                return Ok(());
            }
        };

        // Fetch and re-dispatch the task
        let task_opt = {
            let tasks = self.tasks.read().await;
            tasks
                .iter()
                .find(|(_, t)| t.full_id == next_full_id)
                .map(|(_, t)| t.clone())
        };

        if let Some(task) = task_opt {
            if task.status == TaskStatus::Queued {
                self.dispatch_task_now(task).await?;
            }
        }

        Ok(())
    }

    async fn recompute_positions_locked(
        &self,
        queues: &HashMap<String, Vec<String>>,
        agent_id: &str,
    ) {
        if let Some(q) = queues.get(agent_id) {
            let full_ids: Vec<String> = q.clone();
            let tasks_arc = Arc::clone(&self.tasks);
            tokio::spawn(async move {
                let mut tasks = tasks_arc.write().await;
                for (pos, full_id) in full_ids.iter().enumerate() {
                    for task in tasks.values_mut() {
                        if &task.full_id == full_id {
                            task.queue_position = Some(pos + 1);
                        }
                    }
                }
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Scoring algorithm (Story 3.4)
// ---------------------------------------------------------------------------

fn compute_score(
    agent: &AgentInfo,
    task_content: &str,
    max_cost: f64,
    max_idle_secs: f64,
    now: Instant,
) -> f64 {
    let capability_score = capability_score(agent, task_content);
    let cost_score = if max_cost > 0.0 {
        1.0 - (agent.cost_per_1k_tokens / max_cost)
    } else {
        1.0
    };
    let idle_secs = agent
        .idle_since
        .map(|t| now.duration_since(t).as_secs_f64())
        .unwrap_or(0.0);
    let idle_score = if max_idle_secs > 0.0 {
        idle_secs / max_idle_secs
    } else {
        1.0
    };

    (capability_score * 0.5) + (cost_score * 0.3) + (idle_score * 0.2)
}

fn capability_score(agent: &AgentInfo, task_content: &str) -> f64 {
    if agent.capabilities.is_empty() {
        return 0.5; // neutral
    }
    let task_lower = task_content.to_lowercase();
    let matches = agent
        .capabilities
        .iter()
        .filter(|cap| task_lower.contains(cap.as_str()))
        .count();
    (matches as f64 / agent.capabilities.len() as f64).min(1.0)
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum SendResult {
    Dispatched { task_id: String },
    Queued { task_id: String, agent_id: String, position: usize },
}

#[derive(Debug)]
pub enum AutoRouteResult {
    Dispatched { task_id: String, agent_id: String, estimated_cost: f64 },
    AllBusy { task_id: String, timeout_seconds: u64 },
}

#[derive(Debug, PartialEq)]
pub enum CancelResult {
    Cancelled,
    AlreadyActive,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn short_id(full_id: &str) -> String {
    full_id.replace('-', "")[..8].to_string()
}


// ---------------------------------------------------------------------------
// Tests — Stories 3.2, 3.3, 3.4, 3.5
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    async fn make_router() -> (Arc<MessageBus>, TaskRouter) {
        let session_id = uuid::Uuid::new_v4().to_string();
        let bus = Arc::new(
            MessageBus::new(&session_id).expect("bus creation failed"),
        );
        bus.start().await.expect("bus start failed");
        sleep(Duration::from_millis(20)).await;
        let router = TaskRouter::new(Arc::clone(&bus), 5);
        (bus, router)
    }

    fn make_agent(id: &str, model: &str, cost: f64, caps: Vec<&str>) -> AgentInfo {
        AgentInfo {
            id: id.to_string(),
            agent_type: "test".to_string(),
            model_name: model.to_string(),
            cost_per_1k_tokens: cost,
            capabilities: caps.into_iter().map(str::to_string).collect(),
            idle_since: Some(Instant::now()),
        }
    }

    // -----------------------------------------------------------------------
    // Story 3.2: Send Task to Specific Agent
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_send_task_to_idle_agent_dispatches_immediately() {
        let (bus, router) = make_router().await;
        router.register_agent(make_agent("arch", "claude-sonnet", 0.003, vec![])).await;
        // Agent starts as idle (no entry in status map = idle)

        let result = router.send_to_agent("arch", "Refactor auth module", 1).await.unwrap();
        sleep(Duration::from_millis(30)).await;

        match result {
            SendResult::Dispatched { task_id } => {
                let task = router.task_status(&task_id).await.unwrap();
                assert_eq!(task.status, TaskStatus::Active);
            }
            _ => panic!("Expected Dispatched, got {:?}", result),
        }

        // Agent status should be Working
        let _ast = bus.agent_status();
        let map = _ast.read().await;
        assert_eq!(map.get("arch"), Some(&AgentState::Working));

        bus.stop();
    }

    #[tokio::test]
    async fn test_send_task_to_busy_agent_queues_it() {
        let (bus, router) = make_router().await;
        router.register_agent(make_agent("arch", "claude-sonnet", 0.003, vec![])).await;

        // Mark agent as Working first
        {
            let _ast = bus.agent_status();
            let mut map = _ast.write().await;
            map.insert("arch".to_string(), AgentState::Working);
        }

        let result = router.send_to_agent("arch", "Do something else", 1).await.unwrap();

        match result {
            SendResult::Queued { task_id, agent_id, position } => {
                assert_eq!(agent_id, "arch");
                assert_eq!(position, 1);
                let task = router.task_status(&task_id).await.unwrap();
                assert_eq!(task.status, TaskStatus::Queued);
            }
            _ => panic!("Expected Queued, got {:?}", result),
        }

        bus.stop();
    }

    #[tokio::test]
    async fn test_send_task_unknown_agent_returns_error() {
        let (bus, router) = make_router().await;
        let err = router.send_to_agent("unknown-agent", "task", 1).await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("Unknown agent"));
        bus.stop();
    }

    // -----------------------------------------------------------------------
    // Story 3.3: Task Queue, List & Cancel
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_list_tasks_returns_all_tasks() {
        let (bus, router) = make_router().await;
        router.register_agent(make_agent("arch", "claude-sonnet", 0.003, vec![])).await;

        router.send_to_agent("arch", "task one", 1).await.unwrap();
        // Mark busy so second is queued
        {
            let _ast = bus.agent_status();
            let mut map = _ast.write().await;
            map.insert("arch".to_string(), AgentState::Working);
        }
        router.send_to_agent("arch", "task two", 1).await.unwrap();

        let tasks = router.list_tasks().await;
        assert_eq!(tasks.len(), 2);
        bus.stop();
    }

    #[tokio::test]
    async fn test_cancel_queued_task_succeeds() {
        let (bus, router) = make_router().await;
        router.register_agent(make_agent("arch", "claude-sonnet", 0.003, vec![])).await;

        // Force agent to Working so next task is queued
        {
            let _ast = bus.agent_status();
            let mut map = _ast.write().await;
            map.insert("arch".to_string(), AgentState::Working);
        }

        let result = router.send_to_agent("arch", "cancel me", 1).await.unwrap();
        let task_id = match result {
            SendResult::Queued { task_id, .. } => task_id,
            _ => panic!("Expected queued"),
        };

        let cancel = router.cancel_task(&task_id).await.unwrap();
        assert_eq!(cancel, CancelResult::Cancelled);

        let task = router.task_status(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);

        bus.stop();
    }

    #[tokio::test]
    async fn test_cancel_active_task_returns_already_active() {
        let (bus, router) = make_router().await;
        router.register_agent(make_agent("arch", "claude-sonnet", 0.003, vec![])).await;
        sleep(Duration::from_millis(20)).await;

        let result = router.send_to_agent("arch", "active task", 1).await.unwrap();
        sleep(Duration::from_millis(30)).await;

        let task_id = match result {
            SendResult::Dispatched { task_id } => task_id,
            _ => panic!("Expected dispatched"),
        };

        let cancel = router.cancel_task(&task_id).await.unwrap();
        assert_eq!(cancel, CancelResult::AlreadyActive);

        bus.stop();
    }

    #[tokio::test]
    async fn test_task_status_shows_queue_position() {
        let (bus, router) = make_router().await;
        router.register_agent(make_agent("arch", "claude-sonnet", 0.003, vec![])).await;

        {
            let _ast = bus.agent_status();
            let mut map = _ast.write().await;
            map.insert("arch".to_string(), AgentState::Working);
        }

        let r1 = router.send_to_agent("arch", "q1", 1).await.unwrap();
        let r2 = router.send_to_agent("arch", "q2", 1).await.unwrap();

        let (t1, t2) = match (r1, r2) {
            (SendResult::Queued { task_id: t1, .. }, SendResult::Queued { task_id: t2, .. }) => {
                (t1, t2)
            }
            _ => panic!("expected both queued"),
        };

        let task1 = router.task_status(&t1).await.unwrap();
        let task2 = router.task_status(&t2).await.unwrap();
        assert_eq!(task1.queue_position, Some(1));
        assert_eq!(task2.queue_position, Some(2));

        bus.stop();
    }

    // -----------------------------------------------------------------------
    // Story 3.4: Auto-Route by Cost & Capability
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_auto_route_picks_best_scoring_agent() {
        let (bus, router) = make_router().await;

        // agent-cheap: low cost, no caps
        router
            .register_agent(make_agent("agent-cheap", "haiku", 0.0005, vec![]))
            .await;
        // agent-capable: matching capability, medium cost (0.001)
        // score = cap(1.0)*0.5 + cost(0.5)*0.3 + idle(1.0)*0.2 = 0.85  <- wins
        router
            .register_agent(make_agent("agent-capable", "sonnet", 0.001, vec!["refactor"]))
            .await;
        // agent-mid: higher cost (0.002), no caps
        // score = cap(0.5)*0.5 + cost(0.0)*0.3 + idle(1.0)*0.2 = 0.45
        router
            .register_agent(make_agent("agent-mid", "sonnet-mini", 0.002, vec![]))
            .await;

        let result = router
            .send_auto("refactor the authentication module", None, None)
            .await
            .unwrap();

        match result {
            AutoRouteResult::Dispatched { agent_id, .. } => {
                // agent-capable has full capability match (0.5 * 1.0 = 0.5)
                // vs agent-cheap (0.5 * 0.0 = 0.0 cap_score + best cost_score)
                // capability weight=0.5 should win for exact match
                assert_eq!(agent_id, "agent-capable", "most capable agent should win");
            }
            AutoRouteResult::AllBusy { .. } => panic!("expected dispatch"),
        }

        bus.stop();
    }

    #[tokio::test]
    async fn test_auto_route_all_busy_queues_task() {
        let (bus, router) = make_router().await;
        router.register_agent(make_agent("agent-a", "sonnet", 0.003, vec![])).await;

        // Mark agent as busy
        {
            let _ast = bus.agent_status();
            let mut map = _ast.write().await;
            map.insert("agent-a".to_string(), AgentState::Working);
        }

        let result = router.send_auto("do something", None, None).await.unwrap();

        match result {
            AutoRouteResult::AllBusy { task_id, timeout_seconds } => {
                assert_eq!(timeout_seconds, 5);
                let task = router.task_status(&task_id).await.unwrap();
                assert_eq!(task.status, TaskStatus::Queued);
            }
            _ => panic!("expected AllBusy"),
        }

        bus.stop();
    }

    #[tokio::test]
    async fn test_auto_route_preferred_model_bypasses_scoring() {
        let (bus, router) = make_router().await;
        router
            .register_agent(make_agent("agent-opus", "claude-opus", 0.015, vec![]))
            .await;
        router
            .register_agent(make_agent("agent-haiku", "claude-haiku", 0.0005, vec![]))
            .await;

        let result = router
            .send_auto("complex refactor", Some("claude-opus"), None)
            .await
            .unwrap();

        match result {
            AutoRouteResult::Dispatched { agent_id, .. } => {
                assert_eq!(agent_id, "agent-opus");
            }
            _ => panic!("expected dispatch to opus"),
        }

        bus.stop();
    }

    #[tokio::test]
    async fn test_auto_route_timeout_marks_task_timed_out() {
        let (bus, _) = make_router().await;
        let router = TaskRouter::new(Arc::clone(&bus), 0); // 0-second timeout
        router.register_agent(make_agent("agent-a", "sonnet", 0.003, vec![])).await;

        {
            let _ast = bus.agent_status();
            let mut map = _ast.write().await;
            map.insert("agent-a".to_string(), AgentState::Working);
        }

        let result = router.send_auto("task", None, None).await.unwrap();
        let task_id = match result {
            AutoRouteResult::AllBusy { task_id, .. } => task_id,
            _ => panic!("expected AllBusy"),
        };

        // Wait for timeout watcher to fire (0-second timeout + task processing)
        sleep(Duration::from_millis(100)).await;

        let task = router.task_status(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::TimedOut);

        bus.stop();
    }

    // -----------------------------------------------------------------------
    // Story 3.5: Agent Publishes Task Results
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_result_marks_task_completed_with_details() {
        let (bus, router) = make_router().await;
        router.register_agent(make_agent("arch", "sonnet", 0.003, vec![])).await;
        sleep(Duration::from_millis(20)).await;

        let send_result = router.send_to_agent("arch", "write docs", 1).await.unwrap();
        sleep(Duration::from_millis(30)).await;
        let task_id = match send_result {
            SendResult::Dispatched { task_id } => task_id,
            _ => panic!("expected dispatch"),
        };

        // Retrieve full_id
        let full_id = {
            let tasks = router.tasks.read().await;
            tasks.get(&task_id).unwrap().full_id.clone()
        };

        let result_payload = ResultPayload {
            content: "Documentation written".into(),
            tokens_used: 1500,
            cost_usd: 0.0045,
            duration_ms: 3200,
            task_id: Some(full_id.clone()),
        };

        router.handle_result(&full_id, result_payload).await.unwrap();

        let task = router.task_status(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        let result = task.result.unwrap();
        assert_eq!(result.tokens_used, 1500);
        assert_eq!(result.cost_usd, 0.0045);
        assert_eq!(result.duration_ms, 3200);

        // Agent should be idle again
        let _ast = bus.agent_status();
        let map = _ast.read().await;
        assert_eq!(map.get("arch"), Some(&AgentState::Idle));

        bus.stop();
    }

    #[tokio::test]
    async fn test_agent_crash_marks_task_failed_and_agent_error() {
        let (bus, router) = make_router().await;
        router.register_agent(make_agent("arch", "sonnet", 0.003, vec![])).await;
        sleep(Duration::from_millis(20)).await;

        // Dispatch a task
        router.send_to_agent("arch", "risky task", 1).await.unwrap();
        sleep(Duration::from_millis(30)).await;

        // Simulate crash
        router.handle_agent_crash("arch").await;

        // Agent should be in Error state
        let _ast = bus.agent_status();
        let map = _ast.read().await;
        assert!(matches!(map.get("arch"), Some(AgentState::Error(_))));

        // Task should be Failed
        let tasks = router.list_tasks().await;
        let failed: Vec<_> = tasks.iter().filter(|t| t.status == TaskStatus::Failed).collect();
        assert!(!failed.is_empty(), "at least one task should be failed");

        bus.stop();
    }

    #[tokio::test]
    async fn test_result_returns_agent_to_idle_and_dispatches_queue() {
        let (bus, router) = make_router().await;
        router.register_agent(make_agent("arch", "sonnet", 0.003, vec![])).await;
        sleep(Duration::from_millis(20)).await;

        // Send first task (dispatched)
        let r1 = router.send_to_agent("arch", "task one", 1).await.unwrap();
        sleep(Duration::from_millis(30)).await;

        // Queue second task
        let r2 = router.send_to_agent("arch", "task two", 1).await.unwrap();
        sleep(Duration::from_millis(10)).await;

        let task1_id = match r1 { SendResult::Dispatched { task_id } => task_id, _ => panic!() };
        let task2_id = match r2 { SendResult::Queued { task_id, .. } => task_id, _ => panic!() };

        // Complete task 1
        let full_id = {
            let tasks = router.tasks.read().await;
            tasks.get(&task1_id).unwrap().full_id.clone()
        };
        router.handle_result(&full_id, ResultPayload {
            content: "done".into(),
            tokens_used: 100,
            cost_usd: 0.0003,
            duration_ms: 500,
            task_id: Some(full_id.clone()),
        }).await.unwrap();

        // Give the dispatcher time to pick up task 2
        sleep(Duration::from_millis(60)).await;

        let task2 = router.task_status(&task2_id).await.unwrap();
        assert_eq!(task2.status, TaskStatus::Active, "queued task should be dispatched after first completes");

        bus.stop();
    }
}

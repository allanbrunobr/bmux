use serde::{Deserialize, Serialize};

/// Configuration for a single adversarial sprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialConfig {
    pub prompt: String,
    pub generator_model: String,
    pub evaluator_model: String,
    pub max_retries: u32,
    /// PRD content provided by the user; when present a Planner agent
    /// breaks it into structured sprints before the build/evaluate loop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prd_content: Option<String>,
}

impl Default for AdversarialConfig {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            generator_model: "claude-opus-4-5".to_string(),
            evaluator_model: "claude-sonnet-4-5".to_string(),
            max_retries: 3,
            prd_content: None,
        }
    }
}

/// A single sprint produced by the Planner agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedSprint {
    pub number: u32,
    pub title: String,
    pub features: Vec<String>,
    pub criteria: Vec<Criterion>,
}

/// Full sprint plan output by the Planner agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintPlan {
    pub sprints: Vec<PlannedSprint>,
}

/// Current phase of the adversarial harness.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AdversarialStatus {
    Idle,
    Spawning,
    ContractNegotiation,
    Building,
    Evaluating,
    Retrying,
    Passed,
    Failed,
    Stopped,
    Error(String),
}

impl std::fmt::Display for AdversarialStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Spawning => write!(f, "spawning"),
            Self::ContractNegotiation => write!(f, "contract_negotiation"),
            Self::Building => write!(f, "building"),
            Self::Evaluating => write!(f, "evaluating"),
            Self::Retrying => write!(f, "retrying"),
            Self::Passed => write!(f, "passed"),
            Self::Failed => write!(f, "failed"),
            Self::Stopped => write!(f, "stopped"),
            Self::Error(e) => write!(f, "error:{e}"),
        }
    }
}

/// A single criterion in the sprint contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criterion {
    pub name: String,
    pub description: String,
    pub threshold: f64,
}

/// Sprint contract agreed upon during the negotiation phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintContract {
    pub features: Vec<String>,
    pub criteria: Vec<Criterion>,
}

impl Default for SprintContract {
    fn default() -> Self {
        Self {
            features: vec!["core feature".to_string()],
            criteria: vec![Criterion {
                name: "functionality".to_string(),
                description: "Core functionality works as specified".to_string(),
                threshold: 7.0,
            }],
        }
    }
}

/// Per-criterion score from the evaluator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionScore {
    pub name: String,
    pub score: f64,
    pub threshold: f64,
}

/// Full evaluation result returned by the evaluator agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub passed: bool,
    pub scores: Vec<CriterionScore>,
    pub feedback: Vec<String>,
    #[serde(rename = "overallSummary", alias = "overall_summary")]
    pub overall_summary: String,
}

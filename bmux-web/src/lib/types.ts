export type AgentStatus = 'idle' | 'working' | 'error';
export type AgentType = 'claude-code' | 'opencode' | 'pi' | 'gemini' | 'shell' | 'custom';
export type TaskStatus = 'queued' | 'active' | 'completed' | 'failed' | 'timed_out';

export interface AgentLocation {
  type: 'local' | 'remote';
  provider?: string;
  region?: string;
  lat?: number;
  lon?: number;
}

export interface Agent {
  id: string;
  name: string;
  agent_type: AgentType;
  model: string;
  status: AgentStatus;
  tokens_used: number;
  cost_usd: number;
  uptime_seconds: number;
  last_task?: string;
  spawned_at: string;
  pane_id?: number;
  pid?: number;
  location?: AgentLocation;
}

export interface ContextEntry {
  key: string;
  value: string;
  timestamp: string;
  session: string;
}

export interface Task {
  id: string;
  from_agent: string;
  to_agent: string;
  content: string;
  status: TaskStatus;
  submitted_at: string;
  completed_at?: string;
  cost_usd?: number;
}

export interface Session {
  name: string;
  agents: number;
  created_at: string;
}

export interface Metrics {
  total_tokens: number;
  total_cost_usd: number;
  active_agents: number;
  tasks_completed: number;
  tasks_failed: number;
  uptime_seconds: number;
}

export type AdversarialPhase =
  | 'negotiating'
  | 'building'
  | 'evaluating'
  | 'passed'
  | 'failed'
  | 'complete';

export interface EvaluationScore {
  criterion: string;
  score: number;
  threshold: number;
  passed: boolean;
  details?: string;
}

export interface SprintAttempt {
  sprint: number;
  attempt: number;
  scores: EvaluationScore[];
  passed: boolean;
  feedback: string[];
}

export interface AdversarialConfig {
  generator_model: string;
  evaluator_model: string;
  prompt: string;
  threshold: number;
  max_retries: number;
}

export interface AdversarialState {
  phase: AdversarialPhase;
  sprint: number;
  totalSprints: number;
  attempt: number;
  maxAttempts: number;
  scores: EvaluationScore[];
  history: SprintAttempt[];
  config: AdversarialConfig | null;
}

export type BmuxEvent =
  | { type: 'agent_spawned'; agent: Agent }
  | { type: 'agent_killed'; agent_id: string }
  | { type: 'agent_status_changed'; agent_id: string; status: AgentStatus }
  | { type: 'agent_tokens_updated'; agent_id: string; tokens: number; cost_usd: number }
  | { type: 'context_updated'; entry: ContextEntry }
  | { type: 'task_created'; task: Task }
  | { type: 'task_updated'; task: Task }
  | { type: 'metrics_updated'; metrics: Metrics }
  | { type: 'pane_output'; pane_id: number; data: string }
  | { type: 'audit_event'; entry: AuditEntry }
  | { type: 'adversarial_started'; config: AdversarialConfig; total_sprints?: number }
  | { type: 'adversarial_negotiating'; contract_proposal: Record<string, unknown> }
  | { type: 'adversarial_building'; sprint: number; attempt: number }
  | { type: 'adversarial_evaluating'; sprint: number }
  | { type: 'adversarial_scores'; sprint: number; attempt: number; scores: EvaluationScore[]; passed: boolean }
  | { type: 'adversarial_retry'; sprint: number; attempt: number; feedback: string[] }
  | { type: 'adversarial_sprint_passed'; sprint: number; total_attempts: number }
  | { type: 'adversarial_failed'; sprint: number; reason: string }
  | { type: 'adversarial_complete'; sprints_passed: number; total_duration_ms: number };

export interface AuditEntry {
  id: string;
  timestamp: string;
  event_type: string;
  agent_id?: string;
  session_id: string;
  metadata: Record<string, string>;
  lgpd_compliant: boolean;
}

export interface ConsultationCost {
  consultation_id: string;
  agents: { name: string; role: string; tokens: number; cost_usd: number }[];
  total_cost_usd: number;
  transcription_cost_usd: number;
  soap_note_cost_usd: number;
  date: string;
  timestamp: string;
  [key: string]: unknown;
}

export interface TokenDataPoint {
  timestamp: string;
  agent: string;
  tokens_per_minute: number;
  cumulative_tokens: number;
  cumulative_cost_usd: number;
  [key: string]: unknown;
}

// Adversarial Mode Types
export type AdversarialModel = 'claude-sonnet-4-20250514' | 'claude-opus-4-20250514';

export const ADVERSARIAL_MODELS: AdversarialModel[] = [
  'claude-sonnet-4-20250514',
  'claude-opus-4-20250514',
];

export interface AdversarialStartRequest {
  session: string;
  generator_model: AdversarialModel;
  evaluator_model: AdversarialModel;
  prompt: string;
}

export interface AdversarialState {
  isOn: boolean;
  isRunning: boolean;
  generatorModel: AdversarialModel;
  evaluatorModel: AdversarialModel;
  prompt: string;
}

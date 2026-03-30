export type AgentStatus = 'idle' | 'working' | 'error';
export type AgentType = 'claude' | 'tool' | 'custom' | 'pie' | 'plugin';
export type TaskStatus = 'queued' | 'active' | 'completed' | 'failed' | 'timed_out';

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

export type BmuxEvent =
  | { type: 'agent_spawned'; agent: Agent }
  | { type: 'agent_killed'; agent_id: string }
  | { type: 'agent_status_changed'; agent_id: string; status: AgentStatus }
  | { type: 'agent_tokens_updated'; agent_id: string; tokens: number; cost_usd: number }
  | { type: 'context_updated'; entry: ContextEntry }
  | { type: 'task_created'; task: Task }
  | { type: 'task_updated'; task: Task }
  | { type: 'metrics_updated'; metrics: Metrics };

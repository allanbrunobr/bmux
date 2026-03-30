// Core domain types for the BMUX web dashboard

export type AgentStatus = 'idle' | 'working' | 'error' | 'stopped'
export type TaskStatus = 'queued' | 'active' | 'completed' | 'failed' | 'timed_out'
export type AgentProvider = 'anthropic' | 'openai' | 'google' | 'local' | 'shell'

export interface Agent {
  name: string
  agent_type: string
  model: string
  status: AgentStatus
  provider: AgentProvider
  tokens_total: number
  cost_usd: number
  uptime_secs: number
  last_task_preview: string
  pane_id: string
  location?: AgentLocation
}

export interface AgentLocation {
  lat: number
  lon: number
  label: string
  region?: string
}

export interface Task {
  id: string
  from_agent: string
  to_agent: string
  content: string
  status: TaskStatus
  created_at: string
  completed_at?: string
  cost_usd?: number
}

export interface ContextEntry {
  key: string
  value: string
  updated_at: string
  updated_by?: string
}

export interface TokenDataPoint {
  timestamp: number
  agent: string
  tokens_per_minute: number
  cumulative_tokens: number
  cumulative_cost_usd: number
}

export interface AuditEntry {
  id: string
  timestamp: string
  event_type: string
  agent: string
  session: string
  metadata: Record<string, string>
  lgpd_compliant: boolean | null
}

export interface ConsultationCost {
  consultation_id: string
  transcription_cost_usd: number
  soap_note_cost_usd: number
  total_cost_usd: number
  timestamp: string
  agent: string
}

// WebSocket event types
export type BmuxEvent =
  | { type: 'agent_updated'; agent: Agent }
  | { type: 'agent_spawned'; agent: Agent }
  | { type: 'agent_stopped'; name: string }
  | { type: 'task_queued'; task: Task }
  | { type: 'task_started'; task: Task }
  | { type: 'task_completed'; task: Task }
  | { type: 'context_updated'; entry: ContextEntry }
  | { type: 'pane_output'; pane_id: string; data: string }
  | { type: 'agent_tokens_updated'; name: string; tokens_delta: number; cost_delta_usd: number }
  | { type: 'audit_event'; entry: AuditEntry }

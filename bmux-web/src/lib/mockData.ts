import type { Agent, Task, ContextEntry, TokenDataPoint, AuditEntry, ConsultationCost } from './types'

export const MOCK_AGENTS: Agent[] = [
  {
    name: 'arquiteto',
    agent_type: 'claude-code',
    model: 'claude-opus-4-6',
    status: 'working',
    provider: 'anthropic',
    tokens_total: 45230,
    cost_usd: 0.54,
    uptime_secs: 3600,
    last_task_preview: 'Design auth system with JWT RS256',
    pane_id: 'pane-0',
    location: { lat: -23.5505, lon: -46.6333, label: 'São Paulo, BR' },
  },
  {
    name: 'testador',
    agent_type: 'claude-code',
    model: 'claude-sonnet-4-6',
    status: 'idle',
    provider: 'anthropic',
    tokens_total: 12100,
    cost_usd: 0.09,
    uptime_secs: 2800,
    last_task_preview: 'All 12 auth tests passing',
    pane_id: 'pane-1',
    location: { lat: 37.7749, lon: -122.4194, label: 'us-west-2', region: 'us-west-2' },
  },
  {
    name: 'devops',
    agent_type: 'shell',
    model: 'claude-haiku-4-5',
    status: 'idle',
    provider: 'anthropic',
    tokens_total: 3200,
    cost_usd: 0.01,
    uptime_secs: 1200,
    last_task_preview: 'Docker compose up — all services healthy',
    pane_id: 'pane-2',
    location: { lat: 51.5074, lon: -0.1278, label: 'eu-west-1', region: 'eu-west-1' },
  },
]

export const MOCK_TASKS: Task[] = [
  {
    id: 'task-001',
    from_agent: 'arquiteto',
    to_agent: 'testador',
    content: 'Write unit tests for JWT auth module — 12 test cases minimum',
    status: 'completed',
    created_at: new Date(Date.now() - 3600000).toISOString(),
    completed_at: new Date(Date.now() - 1800000).toISOString(),
    cost_usd: 0.09,
  },
  {
    id: 'task-002',
    from_agent: 'arquiteto',
    to_agent: 'devops',
    content: 'Set up Docker compose with postgres + redis',
    status: 'completed',
    created_at: new Date(Date.now() - 2400000).toISOString(),
    completed_at: new Date(Date.now() - 900000).toISOString(),
    cost_usd: 0.01,
  },
  {
    id: 'task-003',
    from_agent: 'arquiteto',
    to_agent: 'testador',
    content: 'Implement integration tests for /api/auth/login endpoint',
    status: 'active',
    created_at: new Date(Date.now() - 300000).toISOString(),
    cost_usd: 0.03,
  },
]

export const MOCK_CONTEXT: ContextEntry[] = [
  { key: 'design:auth', value: 'JWT RS256, 1h expiry, refresh in sled KV', updated_at: new Date(Date.now() - 3600000).toISOString(), updated_by: 'arquiteto' },
  { key: 'result:tests', value: 'tests/auth_test.rs — 12 tests passing', updated_at: new Date(Date.now() - 1800000).toISOString(), updated_by: 'testador' },
  { key: 'status:arquiteto', value: 'working on API schema', updated_at: new Date(Date.now() - 600000).toISOString(), updated_by: 'arquiteto' },
  { key: 'status:testador', value: 'running integration tests', updated_at: new Date(Date.now() - 300000).toISOString(), updated_by: 'testador' },
]

// Generate mock time-series metrics
function generateMetrics(): TokenDataPoint[] {
  const now = Date.now()
  const points: TokenDataPoint[] = []
  const agents = ['arquiteto', 'testador', 'devops']
  for (let i = 60; i >= 0; i--) {
    for (const agent of agents) {
      const base = agent === 'arquiteto' ? 800 : agent === 'testador' ? 300 : 80
      points.push({
        timestamp: now - i * 60000,
        agent,
        tokens_per_minute: base + Math.floor(Math.random() * base * 0.4),
        cumulative_tokens: (60 - i) * base,
        cumulative_cost_usd: ((60 - i) * base * 0.000015),
      })
    }
  }
  return points
}

export const MOCK_METRICS: TokenDataPoint[] = generateMetrics()

export const MOCK_AUDIT: AuditEntry[] = [
  {
    id: 'audit-001',
    timestamp: new Date(Date.now() - 3700000).toISOString(),
    event_type: 'agent.spawned',
    agent: 'arquiteto',
    session: 'main',
    metadata: { model: 'claude-opus-4-6', type: 'claude-code' },
    lgpd_compliant: true,
  },
  {
    id: 'audit-002',
    timestamp: new Date(Date.now() - 3600000).toISOString(),
    event_type: 'task.sent',
    agent: 'arquiteto',
    session: 'main',
    metadata: { to: 'testador', task_id: 'task-001' },
    lgpd_compliant: true,
  },
  {
    id: 'audit-003',
    timestamp: new Date(Date.now() - 1800000).toISOString(),
    event_type: 'task.completed',
    agent: 'testador',
    session: 'main',
    metadata: { task_id: 'task-001', tokens: '12100' },
    lgpd_compliant: true,
  },
  {
    id: 'audit-004',
    timestamp: new Date(Date.now() - 900000).toISOString(),
    event_type: 'context.updated',
    agent: 'testador',
    session: 'main',
    metadata: { key: 'result:tests', value_length: '38' },
    lgpd_compliant: null,
  },
  {
    id: 'audit-005',
    timestamp: new Date(Date.now() - 300000).toISOString(),
    event_type: 'consultation.started',
    agent: 'arquiteto',
    session: 'afya-demo',
    metadata: { consultation_id: 'c-001', patient_id: '[REDACTED]' },
    lgpd_compliant: true,
  },
]

export const MOCK_CONSULTATIONS: ConsultationCost[] = [
  {
    consultation_id: 'c-001',
    transcription_cost_usd: 0.042,
    soap_note_cost_usd: 0.018,
    total_cost_usd: 0.06,
    timestamp: new Date(Date.now() - 86400000).toISOString(),
    agent: 'transcriber',
  },
  {
    consultation_id: 'c-002',
    transcription_cost_usd: 0.038,
    soap_note_cost_usd: 0.022,
    total_cost_usd: 0.06,
    timestamp: new Date(Date.now() - 72000000).toISOString(),
    agent: 'transcriber',
  },
  {
    consultation_id: 'c-003',
    transcription_cost_usd: 0.055,
    soap_note_cost_usd: 0.031,
    total_cost_usd: 0.086,
    timestamp: new Date(Date.now() - 43200000).toISOString(),
    agent: 'transcriber',
  },
  {
    consultation_id: 'c-004',
    transcription_cost_usd: 0.029,
    soap_note_cost_usd: 0.014,
    total_cost_usd: 0.043,
    timestamp: new Date(Date.now() - 18000000).toISOString(),
    agent: 'transcriber',
  },
]

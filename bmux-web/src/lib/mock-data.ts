import type { Agent, ContextEntry, Task, Session, Metrics } from './types';

export function getMockSessions(): Session[] {
  return [
    {
      name: 'dev-session',
      agents: 3,
      created_at: new Date(Date.now() - 3600 * 1000 * 2).toISOString(),
    },
    {
      name: 'prod-monitor',
      agents: 1,
      created_at: new Date(Date.now() - 3600 * 1000 * 24).toISOString(),
    },
    {
      name: 'research-agents',
      agents: 5,
      created_at: new Date(Date.now() - 3600 * 1000 * 6).toISOString(),
    },
  ];
}

export function getMockAgents(): Agent[] {
  return [
    {
      id: 'agent-001',
      name: 'arquiteto',
      agent_type: 'claude-code',
      model: 'claude-sonnet-4-6',
      status: 'working',
      tokens_used: 42850,
      cost_usd: 0.0642,
      uptime_seconds: 7245,
      last_task: 'Design the authentication system with JWT RS256 and refresh tokens stored in sled KV',
      spawned_at: new Date(Date.now() - 7245 * 1000).toISOString(),
    },
    {
      id: 'agent-002',
      name: 'testador',
      agent_type: 'claude-code',
      model: 'claude-haiku-3-5',
      status: 'idle',
      tokens_used: 18320,
      cost_usd: 0.0091,
      uptime_seconds: 5120,
      last_task: 'Write unit tests for the auth module — 12 tests, all passing',
      spawned_at: new Date(Date.now() - 5120 * 1000).toISOString(),
    },
    {
      id: 'agent-003',
      name: 'infra-bot',
      agent_type: 'shell',
      model: 'claude-opus-4',
      status: 'error',
      tokens_used: 8100,
      cost_usd: 0.1215,
      uptime_seconds: 920,
      last_task: 'Deploy service to staging environment — connection refused on port 8080',
      spawned_at: new Date(Date.now() - 920 * 1000).toISOString(),
    },
    {
      id: 'agent-004',
      name: 'researcher',
      agent_type: 'custom',
      model: 'claude-sonnet-4-6',
      status: 'idle',
      tokens_used: 31500,
      cost_usd: 0.0472,
      uptime_seconds: 12800,
      last_task: 'Summarize latest papers on multi-agent coordination and task delegation',
      spawned_at: new Date(Date.now() - 12800 * 1000).toISOString(),
    },
    {
      id: 'agent-005',
      name: 'pie-runner',
      agent_type: 'pi',
      model: 'claude-haiku-3-5',
      status: 'working',
      tokens_used: 5200,
      cost_usd: 0.0026,
      uptime_seconds: 340,
      last_task: 'Execute data pipeline for user analytics aggregation',
      spawned_at: new Date(Date.now() - 340 * 1000).toISOString(),
    },
  ];
}

export function getMockContextEntries(): ContextEntry[] {
  return [
    {
      key: 'design:auth',
      value: 'JWT RS256 with 1h expiry, refresh tokens in sled KV, POST /auth/login returns access+refresh pair',
      timestamp: new Date(Date.now() - 1200 * 1000).toISOString(),
      session: 'dev-session',
    },
    {
      key: 'status:arquiteto',
      value: 'done — waiting for testador to complete unit tests',
      timestamp: new Date(Date.now() - 900 * 1000).toISOString(),
      session: 'dev-session',
    },
    {
      key: 'result:tests',
      value: 'tests/auth_test.rs created — 12 tests covering login, token refresh, expiry, revocation — all passing',
      timestamp: new Date(Date.now() - 600 * 1000).toISOString(),
      session: 'dev-session',
    },
    {
      key: 'design:database',
      value: 'PostgreSQL 16 with sled KV for session storage. Schema migrations via sqlx. Connection pool size: 10',
      timestamp: new Date(Date.now() - 3600 * 1000).toISOString(),
      session: 'dev-session',
    },
    {
      key: 'status:infra-bot',
      value: 'error — staging deployment failed: connection refused on port 8080. Check docker-compose config.',
      timestamp: new Date(Date.now() - 180 * 1000).toISOString(),
      session: 'dev-session',
    },
    {
      key: 'question:researcher',
      value: 'Which multi-agent framework shows best performance for real-time coordination? AutoGen vs CrewAI vs custom?',
      timestamp: new Date(Date.now() - 7200 * 1000).toISOString(),
      session: 'dev-session',
    },
    {
      key: 'result:research',
      value: 'Custom coordination with shared context store outperforms AutoGen by 40% in low-latency scenarios',
      timestamp: new Date(Date.now() - 5400 * 1000).toISOString(),
      session: 'dev-session',
    },
  ];
}

export function getMockTasks(): Task[] {
  return [
    {
      id: 'task-a1b2c3',
      from_agent: 'arquiteto',
      to_agent: 'testador',
      content: 'Write unit tests for the auth design: JWT RS256, refresh tokens, POST /auth/login endpoint',
      status: 'completed',
      submitted_at: new Date(Date.now() - 1800 * 1000).toISOString(),
      completed_at: new Date(Date.now() - 600 * 1000).toISOString(),
      cost_usd: 0.0091,
    },
    {
      id: 'task-d4e5f6',
      from_agent: 'arquiteto',
      to_agent: 'infra-bot',
      content: 'Deploy the auth service to staging environment and verify health check endpoint responds',
      status: 'failed',
      submitted_at: new Date(Date.now() - 1200 * 1000).toISOString(),
      completed_at: new Date(Date.now() - 900 * 1000).toISOString(),
      cost_usd: 0.0215,
    },
    {
      id: 'task-g7h8i9',
      from_agent: 'researcher',
      to_agent: 'arquiteto',
      content: 'Review the multi-agent coordination findings and incorporate into system design document',
      status: 'active',
      submitted_at: new Date(Date.now() - 300 * 1000).toISOString(),
    },
    {
      id: 'task-j0k1l2',
      from_agent: 'arquiteto',
      to_agent: 'infra-bot',
      content: 'Retry staging deployment after fixing docker-compose port mapping configuration',
      status: 'queued',
      submitted_at: new Date(Date.now() - 60 * 1000).toISOString(),
    },
    {
      id: 'task-m3n4o5',
      from_agent: 'pie-runner',
      to_agent: 'testador',
      content: 'Validate output schema of analytics pipeline matches expected format for downstream consumers',
      status: 'queued',
      submitted_at: new Date(Date.now() - 30 * 1000).toISOString(),
    },
    {
      id: 'task-p6q7r8',
      from_agent: 'researcher',
      to_agent: 'arquiteto',
      content: 'Summarize CrewAI performance benchmarks — latency p50/p99, memory usage, coordination overhead',
      status: 'timed_out',
      submitted_at: new Date(Date.now() - 10800 * 1000).toISOString(),
      completed_at: new Date(Date.now() - 7200 * 1000).toISOString(),
    },
  ];
}

export function getMockMetrics(): Metrics {
  return {
    total_tokens: 105970,
    total_cost_usd: 0.2446,
    active_agents: 2,
    tasks_completed: 1,
    tasks_failed: 2,
    uptime_seconds: 12800,
  };
}

export function getMockTokensPerAgent(): Array<{ name: string; tokens: number; cost: number }> {
  return [
    { name: 'arquiteto', tokens: 42850, cost: 0.0642 },
    { name: 'testador', tokens: 18320, cost: 0.0091 },
    { name: 'infra-bot', tokens: 8100, cost: 0.1215 },
    { name: 'researcher', tokens: 31500, cost: 0.0472 },
    { name: 'pie-runner', tokens: 5200, cost: 0.0026 },
  ];
}

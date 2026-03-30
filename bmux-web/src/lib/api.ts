import type { Agent, Task, ContextEntry, AuditEntry } from './types'

const BASE = process.env.NEXT_PUBLIC_BMUX_API ?? 'http://localhost:7432'

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`)
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`)
  return res.json() as Promise<T>
}

async function post<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`)
  return res.json() as Promise<T>
}

export const api = {
  getSessions(): Promise<{ sessions: string[] }> {
    return get('/api/sessions')
  },

  getAgents(session: string): Promise<{ agents: Agent[] }> {
    return get(`/api/agents?session=${encodeURIComponent(session)}`)
  },

  getTasks(session: string): Promise<{ tasks: Task[] }> {
    return get(`/api/tasks?session=${encodeURIComponent(session)}`)
  },

  getContext(session: string): Promise<{ entries: ContextEntry[] }> {
    return get(`/api/context?session=${encodeURIComponent(session)}`)
  },

  getPaneOutput(session: string, paneId: string, lines = 100): Promise<{ lines: string[] }> {
    return get(
      `/api/pane-output?session=${encodeURIComponent(session)}&pane_id=${encodeURIComponent(paneId)}&lines=${lines}`
    )
  },

  getAuditLog(session: string): Promise<{ entries: AuditEntry[] }> {
    return get(`/api/audit?session=${encodeURIComponent(session)}`)
  },

  sendTask(session: string, agentName: string, content: string): Promise<{ task_id: string }> {
    return post('/api/tasks', { session, agent: agentName, content })
  },
}

export function wsUrl(session: string): string {
  const base = process.env.NEXT_PUBLIC_BMUX_API ?? 'http://localhost:7432'
  return base.replace(/^http/, 'ws') + `/ws?session=${encodeURIComponent(session)}`
}

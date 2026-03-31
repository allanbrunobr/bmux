import type { Agent, ContextEntry, Task, Session, Metrics, AdversarialStartRequest, AdversarialModel } from './types';

// Extended start request that includes prd_content for Planner agent (Story 1.1).
export interface AdversarialStartRequestV2 {
  session: string;
  generator_model: AdversarialModel;
  evaluator_model: AdversarialModel;
  prompt?: string;
  prd_content?: string;
}
import {
  getMockSessions,
  getMockAgents,
  getMockContextEntries,
  getMockTasks,
  getMockMetrics,
} from './mock-data';

const API_BASE = process.env.NEXT_PUBLIC_BMUX_API || 'http://localhost:7432';

async function fetchWithFallback<T>(url: string, fallback: () => T): Promise<T> {
  try {
    const res = await fetch(url, {
      signal: AbortSignal.timeout(5000),
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${res.statusText}`);
    }
    return (await res.json()) as T;
  } catch {
    console.warn(`[BmuxClient] Failed to fetch ${url}, using mock data`);
    return fallback();
  }
}

export class BmuxClient {
  async getSessions(): Promise<Session[]> {
    return fetchWithFallback<Session[]>(
      `${API_BASE}/api/sessions`,
      getMockSessions
    );
  }

  async getAgents(session: string): Promise<Agent[]> {
    return fetchWithFallback<Agent[]>(
      `${API_BASE}/api/agents?session=${encodeURIComponent(session)}`,
      getMockAgents
    );
  }

  async getContext(session: string): Promise<ContextEntry[]> {
    return fetchWithFallback<ContextEntry[]>(
      `${API_BASE}/api/context?session=${encodeURIComponent(session)}`,
      getMockContextEntries
    );
  }

  async getTasks(session: string): Promise<Task[]> {
    return fetchWithFallback<Task[]>(
      `${API_BASE}/api/tasks?session=${encodeURIComponent(session)}`,
      getMockTasks
    );
  }

  async getMetrics(session: string): Promise<Metrics> {
    return fetchWithFallback<Metrics>(
      `${API_BASE}/api/metrics?session=${encodeURIComponent(session)}`,
      getMockMetrics
    );
  }

  async sendTask(
    session: string,
    toAgent: string,
    content: string
  ): Promise<{ id: string }> {
    try {
      const res = await fetch(
        `${API_BASE}/api/tasks?session=${encodeURIComponent(session)}`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ session, agent: toAgent, content }),
          signal: AbortSignal.timeout(10000),
        }
      );
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}: ${res.statusText}`);
      }
      return (await res.json()) as { id: string };
    } catch (err) {
      // In mock mode, simulate a successful task creation
      const mockId = `task-${Math.random().toString(36).substring(2, 8)}`;
      console.warn(`[BmuxClient] sendTask failed, returning mock id ${mockId}`, err);
      return { id: mockId };
    }
  }

  async startAdversarialLoop(req: AdversarialStartRequest | AdversarialStartRequestV2): Promise<{ ok: boolean }> {
    try {
      const res = await fetch(`${API_BASE}/api/adversarial/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(req),
        signal: AbortSignal.timeout(10000),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return { ok: true };
    } catch (err) {
      console.warn('[BmuxClient] startAdversarialLoop failed', err);
      return { ok: false };
    }
  }

  async stopAdversarialLoop(session: string): Promise<{ ok: boolean }> {
    try {
      const res = await fetch(`${API_BASE}/api/adversarial/stop`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ session }),
        signal: AbortSignal.timeout(10000),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return { ok: true };
    } catch (err) {
      console.warn('[BmuxClient] stopAdversarialLoop failed', err);
      return { ok: false };
    }
  }
}

export const bmuxClient = new BmuxClient();

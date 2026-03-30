import { create } from 'zustand';
import type { Agent, ContextEntry, Task, Session, Metrics, BmuxEvent, AdversarialState, AdversarialModel } from './types';

interface BmuxStore {
  // Session
  sessions: Session[];
  activeSession: string | null;
  setActiveSession: (name: string) => void;
  setSessions: (sessions: Session[]) => void;

  // Connection
  isConnected: boolean;
  setConnected: (v: boolean) => void;

  // Agents
  agents: Agent[];
  setAgents: (agents: Agent[]) => void;

  // Context
  contextEntries: ContextEntry[];
  setContextEntries: (entries: ContextEntry[]) => void;

  // Tasks
  tasks: Task[];
  setTasks: (tasks: Task[]) => void;

  // Metrics
  metrics: Metrics | null;
  setMetrics: (m: Metrics) => void;

  // Adversarial
  adversarialRunning: boolean;
  setAdversarialRunning: (v: boolean) => void;  generatorModel: AdversarialModel;
  adversarialOn: boolean;
  setAdversarialOn: (v: boolean) => void;  evaluatorModel: AdversarialModel;
  setGeneratorModel: (m: AdversarialModel) => void;
  setEvaluatorModel: (m: AdversarialModel) => void;  // WebSocket event handler
  adversarialPrompt: string;
  setAdversarialPrompt: (p: string) => void;  handleEvent: (event: BmuxEvent) => void;

  // Adversarial
  adversarial: AdversarialState | null;
  setAdversarial: (state: AdversarialState | null) => void;

  // UI
  sendTaskTarget: string | null;
  setSendTaskTarget: (agentId: string | null) => void;
}

export const useBmuxStore = create<BmuxStore>((set, get) => ({
  // Session
  sessions: [],
  activeSession: null,
  setActiveSession: (name) => set({ activeSession: name }),
  setSessions: (sessions) => set({ sessions }),

  // Connection
  isConnected: false,
  setConnected: (v) => set({ isConnected: v }),

  // Agents
  agents: [],
  setAgents: (agents) => set({ agents }),

  // Context
  contextEntries: [],
  setContextEntries: (entries) => set({ contextEntries: entries }),

  // Tasks
  tasks: [],
  setTasks: (tasks) => set({ tasks }),

  // Metrics
  metrics: null,
  setMetrics: (m) => set({ metrics: m }),

  // Adversarial
  adversarial: null,
  adversarialRunning: false,  setAdversarial: (state) => set({ adversarial: state }),
  setAdversarialRunning: (v) => set({ adversarialRunning: v }),  generatorModel: "claude-sonnet-4-20250514",
  adversarialOn: false,
  setAdversarialOn: (v) => set({ adversarialOn: v }),  evaluatorModel: "claude-opus-4-20250514",
  setGeneratorModel: (m) => set({ generatorModel: m }),
  setEvaluatorModel: (m) => set({ evaluatorModel: m }),
  adversarialPrompt: "",
  setAdversarialPrompt: (p) => set({ adversarialPrompt: p }),  // WebSocket event handler
  handleEvent: (event) => {
    const state = get();

    switch (event.type) {
      case 'agent_spawned':
        set({ agents: [event.agent, ...state.agents] });
        break;

      case 'agent_killed':
        set({
          agents: state.agents.filter((a) => a.id !== event.agent_id),
        });
        break;

      case 'agent_status_changed':
        set({
          agents: state.agents.map((a) =>
            a.id === event.agent_id ? { ...a, status: event.status } : a
          ),
        });
        break;

      case 'agent_tokens_updated':
        set({
          agents: state.agents.map((a) =>
            a.id === event.agent_id
              ? { ...a, tokens_used: event.tokens, cost_usd: event.cost_usd }
              : a
          ),
        });
        break;

      case 'context_updated': {
        const newEntries = [event.entry, ...state.contextEntries];
        // Keep max 100 entries
        set({ contextEntries: newEntries.slice(0, 100) });
        break;
      }

      case 'task_created':
        set({ tasks: [event.task, ...state.tasks] });
        break;

      case 'task_updated':
        set({
          tasks: state.tasks.map((t) =>
            t.id === event.task.id ? event.task : t
          ),
        });
        break;

      case 'metrics_updated':
        set({ metrics: event.metrics });
        break;

      case 'adversarial_started': {
        set({ adversarialRunning: true });
        set({
          adversarial: {
            phase: 'negotiating',
            sprint: 1,
            totalSprints: event.total_sprints ?? 1,
            attempt: 1,
            maxAttempts: event.config.max_retries,
            scores: [],
            history: [],
            config: event.config,
          },
        });
        break;
      }

      case 'adversarial_negotiating': {
        const cur = get().adversarial;
        if (cur) set({ adversarial: { ...cur, phase: 'negotiating' } });
        break;
      }

      case 'adversarial_building': {
        const cur = get().adversarial;
        if (cur) set({ adversarial: { ...cur, phase: 'building', sprint: event.sprint, attempt: event.attempt } });
        break;
      }

      case 'adversarial_evaluating': {
        const cur = get().adversarial;
        if (cur) set({ adversarial: { ...cur, phase: 'evaluating', sprint: event.sprint } });
        break;
      }

      case 'adversarial_scores': {
        const cur = get().adversarial;
        if (cur) {
          const attempt: import('./types').SprintAttempt = {
            sprint: event.sprint,
            attempt: event.attempt,
            scores: event.scores,
            passed: event.passed,
            feedback: [],
          };
          set({
            adversarial: {
              ...cur,
              scores: event.scores,
              history: [...cur.history, attempt],
            },
          });
        }
        break;
      }

      case 'adversarial_retry': {
        const cur = get().adversarial;
        if (cur) {
          const updated = cur.history.map((h) =>
            h.sprint === event.sprint && h.attempt === event.attempt
              ? { ...h, feedback: event.feedback }
              : h
          );
          set({ adversarial: { ...cur, phase: 'building', attempt: event.attempt, history: updated } });
        }
        break;
      }

      case 'adversarial_sprint_passed': {
        const cur = get().adversarial;
        if (cur) set({ adversarial: { ...cur, phase: 'passed', sprint: event.sprint } });
        break;
      }

      case 'adversarial_failed': {
        const cur = get().adversarial;
        if (cur) set({ adversarial: { ...cur, phase: 'failed' } });
        break;
      }

      case 'adversarial_complete': {
        const cur = get().adversarial;
        if (cur) set({ adversarial: { ...cur, phase: 'complete' } });
        break;
      }

      default:
        break;
    }
  },

  // UI
  sendTaskTarget: null,
  setSendTaskTarget: (agentId) => set({ sendTaskTarget: agentId }),
}));

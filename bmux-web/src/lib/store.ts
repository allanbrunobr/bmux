import { create } from 'zustand';
import type { Agent, ContextEntry, Task, Session, Metrics, BmuxEvent, AdversarialModel } from './types';

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

  // WebSocket event handler
  handleEvent: (event: BmuxEvent) => void;

  // Adversarial Mode
  adversarialOn: boolean;
  adversarialRunning: boolean;
  generatorModel: AdversarialModel;
  evaluatorModel: AdversarialModel;
  adversarialPrompt: string;
  setAdversarialOn: (on: boolean) => void;
  setAdversarialRunning: (running: boolean) => void;
  setGeneratorModel: (model: AdversarialModel) => void;
  setEvaluatorModel: (model: AdversarialModel) => void;
  setAdversarialPrompt: (prompt: string) => void;

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

  // WebSocket event handler
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

      default:
        break;
    }
  },

  // Adversarial Mode
  adversarialOn: false,
  adversarialRunning: false,
  generatorModel: 'claude-sonnet-4-20250514',
  evaluatorModel: 'claude-opus-4-20250514',
  adversarialPrompt: '',
  setAdversarialOn: (on) => set({ adversarialOn: on }),
  setAdversarialRunning: (running) => set({ adversarialRunning: running }),
  setGeneratorModel: (model) => set({ generatorModel: model }),
  setEvaluatorModel: (model) => set({ evaluatorModel: model }),
  setAdversarialPrompt: (prompt) => set({ adversarialPrompt: prompt }),

  // UI
  sendTaskTarget: null,
  setSendTaskTarget: (agentId) => set({ sendTaskTarget: agentId }),
}));

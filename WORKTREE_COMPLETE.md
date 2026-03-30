# Worktree wt2 — COMPLETE

**Branch:** `bmux-web-wt2`
**Completed:** 2026-03-29

## Stories Delivered

| Story | Title | Status |
|-------|-------|--------|
| 3.1 | Next.js Project Scaffold & Layout | ✅ done |
| 3.2 | Session Selector & WebSocket Connection | ✅ done |
| 3.3 | Agent Cards Grid | ✅ done |
| 3.4 | Shared Context Live Feed | ✅ done |
| 3.5 | Task Queue Table | ✅ done |
| 3.6 | Send Task Modal | ✅ done |

## Build Verification

```
npm run build → static export in bmux-web/out/
✓ Compiled successfully (zero TypeScript errors)
✓ 6 static routes generated
```

## Key Files

```
bmux-web/
├── src/lib/types.ts          — Agent, Task, ContextEntry, Metrics, BmuxEvent
├── src/lib/bmux-client.ts    — REST client with mock fallback
├── src/lib/store.ts          — Zustand store + WebSocket event handler
├── src/lib/mock-data.ts      — Dev mock data
├── src/hooks/useBmuxWebSocket.ts  — WS hook, exponential backoff reconnect
├── src/hooks/useAgents.ts    — Agent data loader
├── src/components/
│   ├── AgentCard.tsx         — Agent card with type icon, status badge, tokens
│   ├── AgentsGrid.tsx        — Responsive 1/2/3-col grid
│   ├── ContextFeed.tsx       — Live feed, expandable values, highlight animation
│   ├── TaskQueue.tsx         — Task table with status filter tabs
│   ├── SendTaskModal.tsx     — POST /api/tasks modal with toast
│   ├── SessionSelector.tsx   — Auto-selects single session
│   ├── ConnectionStatus.tsx  — Green/red connection indicator
│   └── BmuxProvider.tsx      — Initializes data + WebSocket
└── src/app/                  — Dashboard, Agents, Context, Tasks, Metrics pages
```

## API Expected (Feature 4-5, wt1)

- `GET /api/sessions` → `Session[]`
- `GET /api/agents?session=<name>` → `Agent[]`
- `GET /api/context?session=<name>` → `ContextEntry[]`
- `GET /api/tasks?session=<name>` → `Task[]`
- `GET /api/metrics?session=<name>` → `Metrics`
- `POST /api/tasks` `{ session, to_agent, content }` → `{ id }`
- `WS /ws?session=<name>` → `BmuxEvent` JSON stream

Falls back to mock data automatically when backend is unavailable.

---

# Worktree wt3 — COMPLETE

**Branch:** `bmux-adv-wt3`
**Completed:** 2026-03-30
**Build:** ✅ `npm run build` passed (10/10 pages, zero errors)

## Stories Delivered

| Story | Feature | Title | Status |
|-------|---------|-------|--------|
| 3.1 | 9 | Adversarial Status Panel | ✅ done |
| 3.2 | 10 | Score Board & Evaluation History | ✅ done |
| 3.3 | 11 | Adversarial WebSocket Events | ✅ done |
| 4.1 | 12 | Planner Agent Integration | ✅ done |
| 4.2 | 13 | Sprint Progress Dashboard | ✅ done |

## Key Files

```
bmux-web/src/
├── lib/
│   ├── types.ts          — +AdversarialPhase, EvaluationScore, SprintAttempt,
│   │                         AdversarialConfig, AdversarialState, +9 BmuxEvent types
│   └── store.ts          — +adversarial slice, +9 handleEvent cases
├── components/
│   ├── AdversarialStatus.tsx   — phase badge, counters, progress bar, config info
│   ├── ScoreBoard.tsx          — criterion table, pass/fail badges, glow anim, history
│   ├── SprintTimeline.tsx      — horizontal timeline, click-to-expand ScoreBoard
│   └── layout/Sidebar.tsx      — +Adversarial nav link
└── app/adversarial/page.tsx    — full adversarial control page (models, prompt, start/stop,
                                   multi-sprint checkbox, live panels)
```

## WebSocket Events Handled

`adversarial_started`, `adversarial_negotiating`, `adversarial_building`, `adversarial_evaluating`, `adversarial_scores`, `adversarial_retry`, `adversarial_sprint_passed`, `adversarial_failed`, `adversarial_complete`

## File Ownership Respected

- No changes to `src/adversarial/`, `src/agents/`, `src/tui/` (Forbidden)
- No changes to `bmux-web/src/components/AdversarialPanel.tsx` (wt1 owned)
- No changes to `src/web/` (wt2 owned)

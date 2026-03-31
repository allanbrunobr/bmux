# Worktree wt1 — Complete

Branch: `bmux-adv2-wt1`
Date: 2026-03-30

## Stories Completed

### Story 1.1: PRD Input in Dashboard ✅
- `bmux-web/src/components/AdversarialPanel.tsx` — large PRD textarea + .md/.txt file upload button
- `bmux-web/src/lib/bmux-client.ts` — `AdversarialStartRequestV2` with `prd_content`
- `src/web/routes.rs` — `AdversarialStartBody.prd_content: Option<String>`
- `src/adversarial/types.rs` — `AdversarialConfig.prd_content: Option<String>`

### Story 1.2: Planner Agent Sprint Generation ✅
- `src/adversarial/types.rs` — `PlannedSprint`, `SprintPlan` types
- `src/adversarial/parser.rs` — `parse_sprint_plan()` with fallback
- `src/adversarial/harness.rs` — `run_planner()` spawns Sonnet planner, parses sprint plan, stores in `adversarial:sprint_plan`, emits `AdversarialStarted` with `total_sprints`

## Build & Test Status

- `cargo build` — success (warnings only, pre-existing)
- `cargo test` — all tests pass (parser + security suites)
- TypeScript — no new logic errors (env-level @types issues pre-existing)

## Features Updated

- Feature 1: PRD Input in Dashboard → done
- Feature 2: Planner Agent Sprint Generation → done

## Commits

<<<<<<< HEAD
- b1b8521 feat(story-1.1): PRD input in adversarial dashboard
- 119cb0a feat(story-1.2): Planner agent sprint generation
=======
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

---

# Worktree wt3 — Adversarial v2 COMPLETE

**Branch:** `bmux-adv2-wt3`
**Completed:** 2026-03-30
**Build:** ✅ `npm run build` passed (10/10 pages, zero errors)

## Stories Delivered (Epic 3: Dashboard Multi-Sprint Status)

| Story | Feature | Title | Status |
|-------|---------|-------|--------|
| v2-3.1 | F6 | Multi-Sprint Timeline & ScoreBoard | ✅ done |
| v2-3.2 | F7 | Live Agent Terminal in Dashboard | ✅ done |

## Key Changes

```
bmux-web/src/
├── lib/
│   ├── types.ts          — +SprintCriterion, SprintSpec, SprintPlan interfaces
│   │                         +sprintPlan? to AdversarialState
│   │                         +sprint_plan? to adversarial_started event
│   └── store.ts          — adversarial_started stores sprint_plan from event
├── components/
│   ├── SprintTimeline.tsx      — +sprintPlan prop, sprint titles displayed in nodes
│   └── AdversarialTerminals.tsx — NEW: embedded side-by-side xterm.js panels
│                                   Generator (left) + Evaluator (right)
│                                   reads history + live pane_output WebSocket
└── app/adversarial/page.tsx    — +sprintPlan passed to SprintTimeline
                                  +<AdversarialTerminals /> when loop active
```

## File Ownership Respected

- No changes to `src/adversarial/` (backend — forbidden)
- All changes within `bmux-web/src/` frontend only
>>>>>>> bmux-adv2-wt3

# Story 3.1: Adversarial Status Panel

Status: review

## Story

As a dashboard user,
I want to see the live adversarial loop status on the dashboard,
so that I can monitor Generator vs Evaluator progress in real time.

## Acceptance Criteria

1. Given an adversarial loop is running, the dashboard shows: phase badge (🤝 Negotiating | 🔨 Building | 🔍 Evaluating | ✅ Passed | ❌ Failed), sprint counter, attempt counter, progress bar
2. Given WebSocket event `adversarial_retry` is received, the attempt counter increments and phase returns to "Building", and a notification appears
3. Given WebSocket event `adversarial_sprint_passed` is received, a green success notification appears and the sprint counter advances
4. Updates via WebSocket events in real time

## Tasks / Subtasks

- [x] Task 1: Create AdversarialStatus component (AC: 1, 4)
  - [x] Subtask 1.1: Create `bmux-web/src/components/AdversarialStatus.tsx` with phase badge, sprint counter, attempt counter, progress bar
  - [x] Subtask 1.2: Component reads from Zustand adversarial state slice
- [x] Task 2: Create adversarial page route (AC: 1)
  - [x] Subtask 2.1: Create `bmux-web/src/app/adversarial/page.tsx` that renders AdversarialStatus
- [x] Task 3: Add adversarial link to Sidebar (AC: 1)
  - [x] Subtask 3.1: Add "Adversarial" nav item to Sidebar component

## Dev Notes

- Stack: Next.js 14+ App Router, shadcn/ui, Zustand, TypeScript
- Owned files: `bmux-web/src/components/AdversarialStatus.tsx` (new), `bmux-web/src/app/adversarial/page.tsx` (new)
- Read-only: `bmux-web/src/components/AdversarialPanel.tsx` (wt1)
- Phase badges: 🤝 Negotiating | 🔨 Building | 🔍 Evaluating | ✅ Passed | ❌ Failed
- Adversarial state comes from Zustand store slice (added in Story 3.3 — for 3.1 use local interface)
- Progress bar: (sprint / total_sprints) * 100
- shadcn/ui components: Card, Badge, Progress (or custom)

### References

- [Source: bmux-web/src/lib/store.ts] — Zustand store patterns
- [Source: bmux-web/src/components/AgentCard.tsx] — card component patterns
- [Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md#Story 3.1]

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

### Completion Notes List

- Created AdversarialStatus.tsx: phase badge with color coding, sprint/attempt counters, progress bar, config info panel
- Created app/adversarial/page.tsx: full adversarial control page with start form, model selectors, multi-sprint checkbox, live status, scoreboard, timeline
- Added Swords icon + "Adversarial" nav link to layout/Sidebar.tsx

### File List

- bmux-web/src/components/AdversarialStatus.tsx
- bmux-web/src/app/adversarial/page.tsx
- bmux-web/src/components/layout/Sidebar.tsx

### Change Log

- 2026-03-30: Story 3.1 implemented — AdversarialStatus component + adversarial page + sidebar link

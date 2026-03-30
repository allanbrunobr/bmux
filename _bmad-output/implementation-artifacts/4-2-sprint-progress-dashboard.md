# Story 4.2: Sprint Progress Dashboard

Status: review

## Story

As a dashboard user,
I want to see all sprints with their status on a horizontal timeline,
so that I can track the multi-sprint adversarial build progress.

## Acceptance Criteria

1. Given a multi-sprint adversarial session is running, the dashboard shows a horizontal timeline with each sprint's status: ✅ Passed | 🔨 Building | ⏳ Pending
2. Each sprint node shows: sprint number, status icon, attempt count (when applicable)
3. Clicking a sprint expands to show criteria scores and evaluation history for that sprint

## Tasks / Subtasks

- [x] Task 1: Create SprintTimeline component (AC: 1, 2)
  - [x] Subtask 1.1: Create `bmux-web/src/components/SprintTimeline.tsx`
  - [x] Subtask 1.2: Render horizontal list of sprint nodes with status icons
  - [x] Subtask 1.3: Active sprint gets visual highlight (blue/purple glow + animate-pulse)
  - [x] Subtask 1.4: Pending sprints shown with muted styling
- [x] Task 2: Add click-to-expand evaluation details (AC: 3)
  - [x] Subtask 2.1: Clicking a sprint node toggles an inline expansion
  - [x] Subtask 2.2: Expanded view shows ScoreBoard for that sprint's latest attempt
- [x] Task 3: Integrate SprintTimeline into adversarial page (AC: 1)
  - [x] Subtask 3.1: Render SprintTimeline in `bmux-web/src/app/adversarial/page.tsx` when totalSprints > 1

## Dev Notes

- Stack: Next.js 14+ App Router, shadcn/ui, Zustand, TypeScript
- Owned files: `bmux-web/src/components/SprintTimeline.tsx` (new)
- Data source: adversarial.history from Zustand store (SprintAttempt[])
- Total sprints derived from adversarial.totalSprints (set on adversarial_started event)
- Sprint status logic:
  - sprint < currentSprint → check if passed in history
  - sprint === currentSprint → 'building' or 'evaluating' per phase
  - sprint > currentSprint → 'pending'
- Use ScoreBoard component for expanded sprint details

### References

- [Source: bmux-web/src/components/ScoreBoard.tsx] — reuse for expanded details
- [Source: bmux-web/src/lib/types.ts] — AdversarialState, SprintAttempt
- [Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md#Story 4.2]

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

### Completion Notes List

- Created SprintTimeline.tsx: horizontal timeline with sprint nodes, status icons, attempt counts
- Status logic: past sprints check history for pass/fail; current sprint reflects live phase; future = pending
- Active building/evaluating phases use animate-pulse for live feedback
- Click-to-expand toggles inline ScoreBoard panel for that sprint's history
- Integrated into adversarial/page.tsx — shown when totalSprints > 1

### File List

- bmux-web/src/components/SprintTimeline.tsx
- bmux-web/src/app/adversarial/page.tsx

### Change Log

- 2026-03-30: Story 4.2 implemented — SprintTimeline component + adversarial page integration

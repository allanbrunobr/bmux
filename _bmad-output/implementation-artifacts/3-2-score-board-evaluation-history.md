# Story 3.2: Score Board & Evaluation History

Status: review

## Story

As a dashboard user,
I want to see scores per criterion with pass/fail badges and full evaluation history,
so that I can understand what the Evaluator found and track quality improvements across retries.

## Acceptance Criteria

1. Given an evaluation completes, a score table appears showing each criterion with score (X/10) and pass/fail badge
2. Given the user clicks "View History", a panel shows all attempts for the current sprint with criteria pass counts and feedback
3. Given scores update via WebSocket, the table animates — scores that improved glow green, scores that dropped glow red

## Tasks / Subtasks

- [x] Task 1: Create ScoreBoard component (AC: 1, 3)
  - [x] Subtask 1.1: Create `bmux-web/src/components/ScoreBoard.tsx` with criterion table
  - [x] Subtask 1.2: Each row shows: criterion name, score (X/10), status badge (✅ PASS / ❌ FAIL)
  - [x] Subtask 1.3: Animate score changes — green glow when improved, red glow when dropped
- [x] Task 2: Add evaluation history panel (AC: 2)
  - [x] Subtask 2.1: "View History" button toggles a history panel
  - [x] Subtask 2.2: History shows all attempts with criteria pass counts and feedback summaries

## Dev Notes

- Stack: Next.js 14+ App Router, shadcn/ui, Zustand, TypeScript
- Owned files: `bmux-web/src/components/ScoreBoard.tsx` (new)
- Score threshold default: 7/10 (pass)
- Glow animation: CSS transition on background-color — green (#16a34a) for improvement, red (#dc2626) for drop
- Data source: adversarial state from Zustand (scores: EvaluationScore[], history: SprintAttempt[])
- shadcn/ui: Table, Badge, Button, Card

### References

- [Source: bmux-web/src/components/ui/table.tsx] — table component
- [Source: bmux-web/src/components/ui/badge.tsx] — badge component
- [Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md#Story 3.2]

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

### Completion Notes List

- Created ScoreBoard.tsx with score table (criterion, score X/threshold, pass/fail badge)
- Glow animation: useEffect + DOM style transition on score delta (green=improved, red=dropped)
- View History toggle shows all SprintAttempts in reverse order with per-attempt pass counts and feedback
- Component is reused by SprintTimeline for expanded sprint details

### File List

- bmux-web/src/components/ScoreBoard.tsx

### Change Log

- 2026-03-30: Story 3.2 implemented — ScoreBoard + evaluation history

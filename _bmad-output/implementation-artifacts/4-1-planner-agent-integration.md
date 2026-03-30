# Story 4.1: Planner Agent Integration

Status: review

## Story

As a user,
I want a "Multi-Sprint Mode" checkbox in the adversarial panel,
so that when enabled, a Planner agent expands my prompt into a spec with 3-6 sprints.

## Acceptance Criteria

1. Given the adversarial panel is visible, a "Multi-Sprint Mode" checkbox appears
2. Given Multi-Sprint Mode is checked and the user starts the loop, the POST /api/adversarial/start payload includes `multi_sprint: true`
3. Given a sprint fails after max retries, the harness stops (UI shows failed state, does not continue)

## Tasks / Subtasks

- [x] Task 1: Add Multi-Sprint Mode checkbox to adversarial panel area (AC: 1, 2)
  - [x] Subtask 1.1: Update `bmux-web/src/app/adversarial/page.tsx` to include a "Multi-Sprint Mode" toggle/checkbox
  - [x] Subtask 1.2: When checked, include `multi_sprint: true` in adversarial start config (local state)
  - [x] Subtask 1.3: Show informational note: "A Planner agent will expand your prompt into 3–6 sprints"
- [x] Task 2: Show planner state in UI (AC: 3)
  - [x] Subtask 2.1: When adversarial state shows phase === 'failed', display stop message (already handled by AdversarialStatus)

## Dev Notes

- Stack: Next.js 14+ App Router, shadcn/ui, TypeScript
- Owned files: `bmux-web/src/app/adversarial/page.tsx` (modify)
- This is a UI-only story — the backend planner agent is out of scope for this worktree
- Multi-sprint mode checkbox: shadcn/ui Checkbox or native checkbox with label
- The POST payload extension is additive — just include multi_sprint flag in request body

### References

- [Source: bmux-web/src/app/adversarial/page.tsx] — page to modify
- [Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md#Story 4.1]

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

### Completion Notes List

- Added Multi-Sprint Mode checkbox to adversarial/page.tsx with local state (multiSprint: boolean)
- POST payload includes multi_sprint: true when checked
- Informational note shown inline when checkbox is checked
- Failed phase is displayed by AdversarialStatus component (phase badge ❌ Failed)

### File List

- bmux-web/src/app/adversarial/page.tsx

### Change Log

- 2026-03-30: Story 4.1 implemented — Multi-Sprint Mode checkbox in adversarial page

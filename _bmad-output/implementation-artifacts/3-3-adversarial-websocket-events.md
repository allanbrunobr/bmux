# Story 3.3: Adversarial WebSocket Events

Status: review

## Story

As a developer,
I want the Zustand store and WebSocket hook to handle adversarial events,
so that the dashboard updates in real time as the adversarial loop progresses.

## Acceptance Criteria

1. AdversarialState interface added to types.ts
2. All 9 adversarial event types added to BmuxEvent union in types.ts
3. Adversarial state slice added to store.ts with initial state and handleEvent cases
4. useBmuxWebSocket.ts handles adversarial events (via store.handleEvent — already wired)

## Tasks / Subtasks

- [x] Task 1: Add AdversarialState interface and event types to types.ts (AC: 1, 2)
  - [x] Subtask 1.1: Add EvaluationScore, SprintAttempt, AdversarialConfig interfaces
  - [x] Subtask 1.2: Add AdversarialState interface: phase, sprint, totalSprints, attempt, maxAttempts, scores, history, config
  - [x] Subtask 1.3: Add all 9 adversarial event types to BmuxEvent union:
    - adversarial_started, adversarial_negotiating, adversarial_building, adversarial_evaluating
    - adversarial_scores, adversarial_retry, adversarial_sprint_passed, adversarial_failed, adversarial_complete
- [x] Task 2: Add adversarial slice to store.ts (AC: 3)
  - [x] Subtask 2.1: Add `adversarial: AdversarialState | null` and `setAdversarial` to BmuxStore interface
  - [x] Subtask 2.2: Add initial state: adversarial: null
  - [x] Subtask 2.3: Add handleEvent cases for all 9 adversarial event types

## Dev Notes

- Stack: TypeScript, Zustand
- Owned files: `bmux-web/src/lib/store.ts` (add adversarial slice), `bmux-web/src/lib/types.ts` (add interfaces)
- useBmuxWebSocket already calls store.handleEvent for every message — no changes needed there
- EvaluationScore: { criterion: string; score: number; threshold: number; passed: boolean; details?: string }
- SprintAttempt: { sprint: number; attempt: number; scores: EvaluationScore[]; passed: boolean; feedback: string[] }
- AdversarialConfig: { generator_model: string; evaluator_model: string; prompt: string; threshold: number; max_retries: number }
- Phase type: 'negotiating' | 'building' | 'evaluating' | 'passed' | 'failed' | 'complete'

### References

- [Source: bmux-web/src/lib/types.ts] — existing BmuxEvent union pattern
- [Source: bmux-web/src/lib/store.ts] — existing handleEvent switch pattern
- [Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md#Story 3.3]

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

### Completion Notes List

- Added AdversarialPhase type, EvaluationScore, SprintAttempt, AdversarialConfig, AdversarialState interfaces to types.ts
- Added 9 adversarial event types to BmuxEvent union
- Added adversarial state slice to store.ts (interface, initial state, 9 handleEvent cases)
- useBmuxWebSocket already routes all events through store.handleEvent — no changes needed

### File List

- bmux-web/src/lib/types.ts
- bmux-web/src/lib/store.ts

### Change Log

- 2026-03-30: Story 3.3 implemented — adversarial types + store slice

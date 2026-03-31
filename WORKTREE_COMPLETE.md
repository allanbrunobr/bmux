# Worktree wt4 — COMPLETE

## Story 4.1: Calibrated System Prompts

**Status:** Done  
**Commit:** 884ef24  
**Branch:** bmux-adv2-wt4

## Files Owned

| File | Action |
|------|--------|
| `src/adversarial/prompts.rs` | Created (new) |
| `src/adversarial/mod.rs` | Modified (`pub mod prompts;` added) |

## Acceptance Criteria — All Satisfied

| Prompt | Required phrases | Status |
|--------|-----------------|--------|
| `PLANNER_SYSTEM_PROMPT` | 5/5 | ✅ |
| `GENERATOR_SYSTEM_PROMPT` | 5/5 + BMUX instructions | ✅ |
| `EVALUATOR_SYSTEM_PROMPT` | 6/6 + JSON spec + scoring scale + BMUX | ✅ |
| `CONTRACT_NEGOTIATION_GENERATOR_PROMPT` | criteria count + JSON spec | ✅ |
| `CONTRACT_NEGOTIATION_EVALUATOR_PROMPT` | review directive + APPROVED | ✅ |
| Context isolation (Generator ≠ Evaluator) | scoring rubric not in Generator | ✅ |

## Tests

6 unit tests in `src/adversarial/prompts.rs` — all pass.

## Build

`cargo build` — clean (only pre-existing warnings, no new issues).

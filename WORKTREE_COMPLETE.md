# Worktree wt2 — COMPLETE

Branch: `bmux-adv2-wt2`
Date: 2026-03-30

## Stories Implemented

### Story 2.1: Multi-Sprint Loop in Harness ✅
- `run_inner()` now loops through all sprints from `adversarial:sprint_plan` context key
- Falls back to single sprint from `config.prompt` when no plan exists
- Each sprint: negotiate contract → build → evaluate → retry (up to max_retries)
- Generator receives cumulative context from previous sprints
- Correct events per sprint: AdversarialSprintPassed, AdversarialFailed, AdversarialComplete
- Persists `adversarial:sprint` with current sprint number
- New types: `SprintPlan`, `SprintSpec` in `src/adversarial/types.rs`
- New helper: `parse_sprint_plan()` in `src/adversarial/parser.rs`

### Story 2.2: Evaluator with Real Tools ✅
- Evaluation prompt instructs Evaluator to RUN code, not just read it
- Run `cargo test` / `npm test`, report exact test names and errors
- Curl API endpoints, report URL/expected/actual on failures
- Grep for security issues: unwrap on user input, hardcoded secrets, missing auth, SQL injection
- All feedback includes file path and line number
- Kill background processes before outputting evaluation JSON

### Story 2.3: Git Commits per Feature ✅
- Generator build prompt includes: `git add -A && git commit -m 'feat(sprint-N): <description>'` after each feature
- Generator retry prompt includes: `git add -A && git commit -m 'fix(sprint-N): <what was fixed>'` after each fix
- Sprint number interpolated correctly in commit message templates

## Files Changed

| File | Change |
|------|--------|
| `src/adversarial/types.rs` | Added SprintPlan, SprintSpec types |
| `src/adversarial/parser.rs` | Added parse_sprint_plan() |
| `src/adversarial/harness.rs` | Full rewrite: multi-sprint loop, real-tools eval prompt, git commit instructions |

## Test Results

```
223 lib tests  — 0 failed
22 agent_tests — 0 failed
7 audit_log    — 0 failed
10 integration — 0 failed
5 hmac         — 0 failed
4 sandbox      — 0 failed
8 secrets      — 0 failed
Total: 279 passed, 0 failed
```

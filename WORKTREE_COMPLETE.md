# Worktree wt2 — Complete

Branch: `bmux-adv-wt2`

## Stories Implemented

### Story 2.1 — Spawn Generator & Evaluator Agents ✅
- `spawn_agent()` in `src/adversarial/harness.rs`
- Creates TUI pane via `session.split_and_run_command()` (interactive bash + BMUX env vars)
- Registers in `AgentRegistry` with role as `agent_type` ("adversarial-generator" / "adversarial-evaluator")
- Injects `BMUX_SESSION`, `BMUX_SOCKET`, `BMUX_AGENT_NAME`, `BMUX_ADV_ROLE`
- Emits `AgentSpawned` WebSocket event
- Evaluator isolation: separate PTY panes, no shared context

### Story 2.2 — Contract Negotiation Phase ✅
- `negotiate_contract()` in `src/adversarial/harness.rs`
- Generator proposes JSON `{features[], criteria[{name, description, threshold}]}`
- Evaluator outputs `APPROVED` or tougher revised JSON (up to 2 rounds)
- Fallback chain in `src/adversarial/parser.rs`:
  1. Fenced code block
  2. Brace-matched extraction
  3. Raw JSON parse
  4. Default contract
- Final contract stored in `adversarial:contract`

### Story 2.3 — Build → Evaluate → Feedback → Retry Loop ✅
- `build_evaluate_loop()` in `src/adversarial/harness.rs`
- Generator builds; Evaluator scores `{passed, scores, feedback[], overallSummary}`
- Pass: all `score >= threshold`; Retry: any < threshold AND retries < max; Fail: exhausted
- Runs via `tokio::spawn` (non-blocking)
- Emits `AdversarialPassed` / `AdversarialFailed` WS events

### Story 2.4 — Persist Adversarial State ✅
Context keys: `adversarial:status`, `adversarial:config`, `adversarial:contract`,
`adversarial:attempt`, `adversarial:scores`, `adversarial:feedback`

### Story 2.5 — Adversarial REST API Endpoints ✅
- `POST /api/adversarial/start` — spawns background harness task
- `POST /api/adversarial/stop` — signals stop via AtomicBool
- `GET /api/adversarial/status?session=X`
- `GET /api/adversarial/history?session=X`

## Build
- `cargo build` ✅
- `cargo build --release` ✅
- `cargo test adversarial` ✅ (5/5 pass)

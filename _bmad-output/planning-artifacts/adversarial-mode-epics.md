---
stepsCompleted: ['step-01-validate-prerequisites', 'step-02-design-epics', 'step-03-create-stories', 'step-04-final-validation']
inputDocuments:
  - 'https://github.com/coleam00/adversarial-dev.git (harness.ts, evaluator.ts, generator.ts, types.ts, prompts.ts)'
  - 'https://www.anthropic.com/engineering/harness-design-long-running-apps'
  - 'BMUX existing codebase (daemon, agents, task router, dashboard)'
  - 'Pi long-run skill (sequential feature loop pattern)'
mvpScope: [1, 2, 3]
v1Scope: [4]
---

# BMUX Adversarial Development Mode - Epic Breakdown

## Overview

This document provides the epic and story breakdown for adding an Adversarial Development Mode to BMUX. Inspired by the adversarial-dev harness (coleam00) and Anthropic's harness design article, this feature turns BMUX into a GAN-inspired development environment where one AI agent builds and another tries to break what was built.

## Requirements Inventory

### Functional Requirements

- FR-ADV-01: User can toggle "Adversarial Mode" on/off from the web dashboard
- FR-ADV-02: When Adversarial Mode is activated, user can select the Generator model (e.g. Claude Sonnet) and Evaluator model (e.g. Claude Opus) from a dropdown of available cloud models
- FR-ADV-03: Activating Adversarial Mode spawns two agents automatically: a Generator agent and an Evaluator agent, each in its own TUI pane
- FR-ADV-04: User can provide a task/prompt that triggers the adversarial loop: Generator implements → Evaluator verifies → feedback → retry
- FR-ADV-05: The Evaluator scores each criterion on a 1-10 scale with a configurable pass threshold (default: 7/10)
- FR-ADV-06: If the Evaluator fails any criterion, detailed feedback (file paths, line numbers, what failed) is sent back to the Generator automatically
- FR-ADV-07: The Generator retries up to a configurable max (default: 3) before the sprint is marked as failed
- FR-ADV-08: The web dashboard shows adversarial loop status in real time: current sprint, attempt number, scores per criterion, pass/fail badges
- FR-ADV-09: The web dashboard shows a contract negotiation phase where Generator proposes criteria and Evaluator toughens them
- FR-ADV-10: All adversarial loop state (sprints, contracts, scores, feedback) is stored in the BMUX context store for auditability
- FR-ADV-11: User can view the full evaluation history (all attempts, all scores) per sprint in the dashboard
- FR-ADV-12: The daemon orchestrates the adversarial loop without user intervention after initial prompt — fully autonomous
- FR-ADV-13: Each agent (Generator/Evaluator) has its own isolated context window — the Evaluator never sees the Generator's prompt (reduces self-evaluation bias)

### NonFunctional Requirements

- NFR-ADV-01: Adversarial loop latency: Generator → Evaluator handoff < 5 seconds
- NFR-ADV-02: Evaluation JSON parsing must be resilient — multiple extraction strategies (code blocks, brace matching, raw text)
- NFR-ADV-03: The loop must not block the TUI or web dashboard — runs as a background tokio task
- NFR-ADV-04: Memory usage: each agent's context window is independent (no shared context leaking prompts)
- NFR-ADV-05: The harness must handle agent crashes gracefully — if Generator or Evaluator dies, mark sprint as failed, don't hang

### Additional Requirements

- AR-ADV-01: Generator agent spawned with `claude --dangerously-skip-permissions --model <selected-model>`
- AR-ADV-02: Evaluator agent spawned with `claude --dangerously-skip-permissions --model <selected-model>` and tools: Read, Bash, Glob, Grep
- AR-ADV-03: Contract format: JSON `{ sprintNumber, features[], criteria[{ name, description, threshold }] }`
- AR-ADV-04: Evaluation result format: JSON `{ passed, scores{}, feedback[{ criterion, score, details }], overallSummary }`
- AR-ADV-05: Models available: claude-sonnet-4-20250514, claude-opus-4-20250514 (cloud API via CLI)
- AR-ADV-06: System prompts for Generator, Evaluator, and Contract Negotiation based on adversarial-dev patterns
- AR-ADV-07: The harness loop is implemented in Rust inside the daemon (not in TypeScript) — the adversarial-dev repo is a reference pattern, not a dependency

### UX Design Requirements

N/A — web dashboard toggle + existing shadcn/ui components.

### FR Coverage Map

- FR-ADV-01: Epic 1 — Story 1.1 (Dashboard toggle)
- FR-ADV-02: Epic 1 — Story 1.2 (Model selector)
- FR-ADV-03: Epic 2 — Story 2.1 (Auto-spawn agents)
- FR-ADV-04: Epic 2 — Story 2.2 (Adversarial loop trigger)
- FR-ADV-05: Epic 2 — Story 2.3 (Evaluation scoring)
- FR-ADV-06: Epic 2 — Story 2.3 (Feedback delivery)
- FR-ADV-07: Epic 2 — Story 2.3 (Retry loop)
- FR-ADV-08: Epic 3 — Story 3.1 (Dashboard real-time status)
- FR-ADV-09: Epic 2 — Story 2.2 (Contract negotiation)
- FR-ADV-10: Epic 2 — Story 2.4 (Context store persistence)
- FR-ADV-11: Epic 3 — Story 3.2 (Evaluation history view)
- FR-ADV-12: Epic 2 — Story 2.3 (Autonomous loop)
- FR-ADV-13: Epic 2 — Story 2.1 (Isolated context windows)

## Epic List

### Epic 1: Dashboard — Adversarial Mode Controls [MVP]
User can toggle Adversarial Mode on/off from the web dashboard, select models for Generator and Evaluator, and trigger the loop with a prompt.
**FRs covered:** FR-ADV-01, FR-ADV-02

### Epic 2: Backend — Adversarial Harness Loop [MVP]
The daemon orchestrates the full adversarial development cycle: spawn agents, negotiate contract, build → evaluate → feedback → retry, store results. Fully autonomous after initial prompt.
**FRs covered:** FR-ADV-03, FR-ADV-04, FR-ADV-05, FR-ADV-06, FR-ADV-07, FR-ADV-09, FR-ADV-10, FR-ADV-12, FR-ADV-13

### Epic 3: Dashboard — Real-Time Adversarial Status [MVP]
Dashboard shows live adversarial loop progress: sprint number, attempt count, scores per criterion with pass/fail badges, contract details, and full evaluation history.
**FRs covered:** FR-ADV-08, FR-ADV-11

### Epic 4: Advanced — Multi-Sprint Planning [v1.0]
Add a Planner agent (3rd agent) that expands a short prompt into a full spec with multiple sprints, enabling long-running adversarial development sessions.
**FRs covered:** Extension of FR-ADV-04

---

## Epic 1: Dashboard — Adversarial Mode Controls [MVP]

User can activate Adversarial Mode from the web dashboard, choose which AI models to pit against each other, and kick off the loop with a single prompt.

### Story 1.1: Adversarial Mode Toggle

As a dashboard user,
I want to toggle Adversarial Mode on/off from the dashboard header,
So that I can switch between normal agent management and adversarial development.

**Acceptance Criteria:**

**Given** the dashboard is loaded
**When** the user clicks the "Adversarial Mode" toggle in the header
**Then** the toggle switches to ON state with a visual indicator (amber glow)
**And** the dashboard shows the Adversarial Mode panel (model selectors + prompt input)

**Given** Adversarial Mode is ON
**When** the user toggles it OFF
**Then** the panel hides
**And** any running adversarial loop continues in background (not cancelled)

---

### Story 1.2: Model Selector for Generator & Evaluator

As a dashboard user,
I want to select different models for the Generator and Evaluator agents,
So that I can use Sonnet for speed (Generator) and Opus for rigor (Evaluator).

**Acceptance Criteria:**

**Given** Adversarial Mode is ON
**Then** two dropdown selectors appear: "Generator Model" and "Evaluator Model"
**And** each dropdown lists available cloud models: claude-sonnet-4-20250514, claude-opus-4-20250514

**Given** the user selects Sonnet for Generator and Opus for Evaluator
**When** the adversarial loop starts
**Then** Agent 1 is spawned with `--model claude-sonnet-4-20250514`
**And** Agent 2 is spawned with `--model claude-opus-4-20250514`

---

### Story 1.3: Adversarial Prompt Input & Start

As a dashboard user,
I want to type a task prompt and click "Start Adversarial Loop",
So that the Generator/Evaluator cycle begins autonomously.

**Acceptance Criteria:**

**Given** Adversarial Mode is ON and models are selected
**When** the user types a prompt ("Build a REST API with auth") and clicks [Start]
**Then** a `POST /api/adversarial/start` request is sent with `{ session, generator_model, evaluator_model, prompt }`
**And** the button changes to "Running..." with a spinner
**And** the dashboard begins showing real-time loop status via WebSocket

**Given** an adversarial loop is already running
**When** the user clicks [Stop]
**Then** a `POST /api/adversarial/stop` is sent
**And** the loop halts after the current phase completes

---

## Epic 2: Backend — Adversarial Harness Loop [MVP]

The daemon runs the full adversarial cycle: spawn two Claude agents with different models, negotiate a quality contract, loop build → evaluate → feedback → retry, and store all results in the context store. After the initial prompt, no human intervention is needed.

### Story 2.1: Spawn Generator & Evaluator Agents

As the daemon,
I want to spawn two Claude agents with different models when adversarial mode starts,
So that the Generator and Evaluator have separate PTY panes and isolated contexts.

**Acceptance Criteria:**

**Given** `POST /api/adversarial/start` is received with generator_model and evaluator_model
**When** the daemon processes the request
**Then** Agent "generator" is spawned: `claude --dangerously-skip-permissions --model <generator_model>`
**And** Agent "evaluator" is spawned: `claude --dangerously-skip-permissions --model <evaluator_model>`
**And** both agents appear in their own TUI panes
**And** both agents are registered in AgentRegistry with role metadata (generator/evaluator)
**And** BMUX_SESSION, BMUX_AGENT_NAME env vars are injected
**And** the Evaluator never receives the Generator's system prompt (isolated context)

---

### Story 2.2: Contract Negotiation Phase

As the daemon harness,
I want to run a contract negotiation between Generator and Evaluator before building starts,
So that both agents agree on specific, testable success criteria.

**Acceptance Criteria:**

**Given** both agents are spawned and ready
**When** the harness enters the negotiation phase
**Then** the Generator receives: "Propose a sprint contract for: <prompt>" and returns JSON `{ features, criteria }`
**And** the Evaluator receives: "Review this contract: <generator's proposal>" and returns either "APPROVED" or a toughened JSON
**And** the final contract is stored in context: `adversarial:contract` with JSON value

**Given** the Evaluator returns a revised contract
**Then** the harness uses the Evaluator's version (tougher criteria)

**Given** JSON parsing fails on either response
**Then** the harness uses fallback extraction strategies (code blocks, brace matching, raw text)
**And** if all strategies fail, a default contract with 3 basic criteria is used

---

### Story 2.3: Build → Evaluate → Feedback → Retry Loop

As the daemon harness,
I want to run the adversarial loop: Generator builds, Evaluator scores, feedback sent back on failure, retry up to max,
So that code quality is driven by adversarial tension.

**Acceptance Criteria:**

**Given** a contract is agreed
**When** the harness enters the build phase
**Then** the Generator receives: "Implement the features in the contract. Commit after each feature."
**And** the Generator's output appears in its TUI pane in real time

**Given** the Generator completes building
**When** the harness enters the evaluation phase
**Then** the Evaluator receives: "Evaluate the app/ directory against this contract: <JSON>. Score each criterion 1-10."
**And** the Evaluator reads code, runs the app, tests edge cases
**And** the Evaluator returns JSON `{ passed, scores, feedback[], overallSummary }`

**Given** the Evaluator returns `passed: true` (all criteria >= threshold)
**Then** the sprint is marked as PASSED
**And** a WebSocket event `adversarial_sprint_passed` is emitted
**And** the harness moves to the next sprint (or completes)

**Given** the Evaluator returns `passed: false`
**And** retry count < max (default: 3)
**Then** the feedback is sent to the Generator: "Evaluation failed. Fix these issues: <feedback>"
**And** retry count increments
**And** a WebSocket event `adversarial_retry` is emitted with attempt number

**Given** the Evaluator returns `passed: false`
**And** retry count >= max
**Then** the sprint is marked as FAILED
**And** the harness stops
**And** a WebSocket event `adversarial_failed` is emitted

---

### Story 2.4: Persist Adversarial State in Context Store

As the daemon,
I want to store all adversarial loop state in the BMUX context store,
So that the dashboard can display history and the state survives detach/reattach.

**Acceptance Criteria:**

**Given** any adversarial phase completes
**Then** the following keys are updated in the context store:
- `adversarial:status` → "negotiating" | "building" | "evaluating" | "passed" | "failed"
- `adversarial:contract` → JSON contract
- `adversarial:sprint` → current sprint number
- `adversarial:attempt` → current attempt number
- `adversarial:scores` → JSON array of all evaluation results
- `adversarial:feedback` → latest feedback from Evaluator
- `adversarial:config` → JSON `{ generator_model, evaluator_model, prompt, threshold, max_retries }`

---

### Story 2.5: Adversarial REST API Endpoints

As the web dashboard,
I want REST endpoints to start/stop/query the adversarial loop,
So that the frontend can control and display the harness state.

**Acceptance Criteria:**

**Given** the Axum server is running
**Then** these endpoints are available:
- `POST /api/adversarial/start` → `{ session, generator_model, evaluator_model, prompt, threshold?, max_retries? }`
- `POST /api/adversarial/stop` → stops the loop after current phase
- `GET /api/adversarial/status?session=X` → returns current harness state (status, sprint, attempt, scores)
- `GET /api/adversarial/history?session=X` → returns all sprints with all attempts and scores

---

## Epic 3: Dashboard — Real-Time Adversarial Status [MVP]

The dashboard shows the adversarial loop's live progress — which phase is active, scores per criterion with visual pass/fail, attempt counter, and full evaluation history.

### Story 3.1: Adversarial Status Panel

As a dashboard user,
I want to see the live adversarial loop status on the dashboard,
So that I can monitor Generator vs Evaluator progress in real time.

**Acceptance Criteria:**

**Given** an adversarial loop is running
**Then** the dashboard shows a panel with:
- Current phase badge: 🤝 Negotiating | 🔨 Building | 🔍 Evaluating | ✅ Passed | ❌ Failed
- Sprint counter: "Sprint 2/5"
- Attempt counter: "Attempt 1/3"
- Progress bar showing overall completion

**Given** WebSocket event `adversarial_retry` is received
**Then** the attempt counter increments and the phase returns to "Building"
**And** a notification appears: "Sprint 2 failed evaluation — retry 2/3"

**Given** WebSocket event `adversarial_sprint_passed` is received
**Then** a green success notification appears
**And** the sprint counter advances

---

### Story 3.2: Score Board & Evaluation History

As a dashboard user,
I want to see scores per criterion with pass/fail badges and full evaluation history,
So that I can understand what the Evaluator found and track quality improvements across retries.

**Acceptance Criteria:**

**Given** an evaluation completes
**Then** a score table appears showing each criterion:
  | Criterion | Score | Status |
  |-----------|-------|--------|
  | basic_functionality | 8/10 | ✅ PASS |
  | error_handling | 5/10 | ❌ FAIL |
  | code_quality | 9/10 | ✅ PASS |

**Given** the user clicks "View History"
**Then** a panel shows all attempts for the current sprint:
  - Attempt 1: 2/5 criteria passed (feedback shown)
  - Attempt 2: 4/5 criteria passed (feedback shown)
  - Attempt 3: 5/5 criteria passed ✅

**Given** scores update via WebSocket
**Then** the table animates — scores that improved glow green, scores that dropped glow red

---

### Story 3.3: Adversarial WebSocket Events

As a developer,
I want the daemon to emit typed WebSocket events for all adversarial state changes,
So that the dashboard updates in real time.

**Acceptance Criteria:**

**Given** the adversarial harness is running
**Then** these WebSocket events are emitted:
- `{ type: "adversarial_started", config: {...} }`
- `{ type: "adversarial_negotiating", contract_proposal: {...} }`
- `{ type: "adversarial_building", sprint: N, attempt: M }`
- `{ type: "adversarial_evaluating", sprint: N }`
- `{ type: "adversarial_scores", sprint: N, attempt: M, scores: [...], passed: bool }`
- `{ type: "adversarial_retry", sprint: N, attempt: M, feedback: [...] }`
- `{ type: "adversarial_sprint_passed", sprint: N, total_attempts: M }`
- `{ type: "adversarial_failed", sprint: N, reason: "..." }`
- `{ type: "adversarial_complete", sprints_passed: N, total_duration_ms: T }`

---

## Epic 4: Advanced — Multi-Sprint Planning [v1.0]

A third Planner agent expands a short prompt into a full product spec with multiple sprints, enabling long-running adversarial sessions that build entire applications.

### Story 4.1: Planner Agent Integration

As a user,
I want to provide a short 1-4 sentence prompt and have a Planner agent create a full spec with sprints,
So that the adversarial loop can build a complete application autonomously.

**Acceptance Criteria:**

**Given** the user enables "Multi-Sprint Mode" in the adversarial panel
**When** the user provides a prompt and clicks Start
**Then** a third agent "planner" is spawned
**And** the Planner receives: "Expand this into a product spec with sprints: <prompt>"
**And** the Planner writes a spec.md with features organized into 3-6 sprints
**And** the harness loops through each sprint with the Generator/Evaluator

**Given** a sprint fails after max retries
**Then** the harness stops (does not continue to the next sprint)

---

### Story 4.2: Sprint Progress Dashboard

As a dashboard user,
I want to see all sprints with their status on a timeline,
So that I can track the multi-sprint build progress.

**Acceptance Criteria:**

**Given** a multi-sprint adversarial session is running
**Then** the dashboard shows a horizontal timeline:
  Sprint 1: ✅ Passed (2 attempts)
  Sprint 2: 🔨 Building (attempt 1/3)
  Sprint 3: ⏳ Pending
  Sprint 4: ⏳ Pending

**And** clicking a sprint shows its criteria scores and evaluation history

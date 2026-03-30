---
stepsCompleted: ['step-01-validate-prerequisites', 'step-02-design-epics', 'step-03-create-stories', 'step-04-final-validation']
inputDocuments:
  - 'https://github.com/coleam00/adversarial-dev.git'
  - 'https://www.anthropic.com/engineering/harness-design-long-running-apps'
  - 'BMUX existing adversarial implementation (src/adversarial/, bmux-web components)'
  - 'User requirement: PRD → Planner → Generator → Evaluator adversarial loop'
mvpScope: [1, 2, 3, 4]
---

# BMUX Adversarial v2 — Full Harness Loop (Planner + Generator + Evaluator)

## Overview

Upgrade the adversarial mode from single-sprint to the full coleam00 harness pattern: a Planner agent creates sprints from a PRD, a Generator implements each sprint, and an Evaluator adversarially tests each sprint. The loop is fully autonomous after the user provides the PRD.

## What Already Exists (DO NOT rebuild)

- ✅ Adversarial toggle + model selectors in dashboard (Epic 1 v1 — done)
- ✅ Agent spawn with PTY panes (harness.rs spawn_agents)
- ✅ Single-sprint build → evaluate → retry loop (harness.rs build_evaluate_loop)
- ✅ Contract negotiation (harness.rs negotiate_contract)
- ✅ JSON evaluation parser (parser.rs)
- ✅ REST API: POST /api/adversarial/start|stop, GET status|history
- ✅ WebSocket events: all 9 adversarial event types
- ✅ Dashboard: AdversarialStatus, ScoreBoard, SprintTimeline components
- ✅ Context store persistence for adversarial state

## Requirements Inventory

### Functional Requirements

- FR-V2-01: User can paste/upload a PRD into the adversarial panel
- FR-V2-02: Planner agent (Sonnet) receives the PRD and creates epics + stories using BMAD workflow, producing a spec with N sprints (1 story = 1 sprint)
- FR-V2-03: The harness loops through ALL sprints sequentially — not just one
- FR-V2-04: Before each sprint, Generator and Evaluator negotiate a contract with specific testable criteria per sprint
- FR-V2-05: Generator (Sonnet) implements each sprint, committing to git after each feature
- FR-V2-06: Evaluator (Opus) actively tries to BREAK what the Generator built — reads code, runs tests, executes the app, probes edge cases
- FR-V2-07: Evaluator has access to real tools: Read, Bash, Grep, Glob — can run cargo test, curl endpoints, check for security issues
- FR-V2-08: If Evaluator fails any criterion (<7/10), detailed feedback with file paths and line numbers is sent back to Generator
- FR-V2-09: Generator retries up to 3 times per sprint with Evaluator's adversarial feedback
- FR-V2-10: If a sprint fails 3 retries, the entire harness STOPS (no continuing to next sprint)
- FR-V2-11: Dashboard SprintTimeline shows all sprints with real-time status (pending/building/evaluating/passed/failed)
- FR-V2-12: Dashboard ScoreBoard shows per-criterion scores with pass/fail badges, updating live across retries
- FR-V2-13: User can see each agent's live terminal output in the dashboard while the loop runs
- FR-V2-14: Each agent has isolated context — Evaluator never sees Generator's system prompt (reduces self-evaluation bias)

### NonFunctional Requirements

- NFR-V2-01: Planner must produce structured output (not free-form) — use BMAD epics template
- NFR-V2-02: Evaluator MUST actually execute code (cargo test, curl, etc.) — not just read and assume
- NFR-V2-03: JSON parsing resilient with multiple extraction strategies (code blocks, brace matching, raw text)
- NFR-V2-04: The harness loop runs as tokio::spawn — non-blocking to TUI and dashboard
- NFR-V2-05: Agent crashes handled gracefully — mark sprint as failed, don't hang

### Additional Requirements

- AR-V2-01: Planner model: claude-sonnet-4-20250514 (fast, good at planning)
- AR-V2-02: Generator model: claude-sonnet-4-20250514 (fast implementation)
- AR-V2-03: Evaluator model: claude-opus-4-20250514 (deep reasoning for adversarial testing)
- AR-V2-04: System prompts based on adversarial-dev repo (EVALUATOR_SYSTEM_PROMPT, GENERATOR_SYSTEM_PROMPT)
- AR-V2-05: Git commits after each feature (Generator) for atomic rollback

### FR Coverage Map

- FR-V2-01: Epic 1 — Story 1.1 (PRD input)
- FR-V2-02: Epic 1 — Story 1.2 (Planner agent)
- FR-V2-03: Epic 2 — Story 2.1 (Multi-sprint loop)
- FR-V2-04: Epic 2 — Story 2.1 (Contract per sprint — already exists, extend)
- FR-V2-05: Epic 2 — Story 2.1 (Generator build)
- FR-V2-06: Epic 2 — Story 2.2 (Evaluator with tools)
- FR-V2-07: Epic 2 — Story 2.2 (Evaluator tools)
- FR-V2-08: Epic 2 — Story 2.1 (Feedback delivery)
- FR-V2-09: Epic 2 — Story 2.1 (Retry loop — already exists, extend)
- FR-V2-10: Epic 2 — Story 2.1 (Stop on sprint failure)
- FR-V2-11: Epic 3 — Story 3.1 (SprintTimeline update)
- FR-V2-12: Epic 3 — Story 3.1 (ScoreBoard update)
- FR-V2-13: Epic 3 — Story 3.2 (Agent terminal in dashboard)
- FR-V2-14: Epic 2 — Story 2.2 (Isolated contexts)

## Epic List

### Epic 1: Planner Agent — PRD to Sprints [MVP]
User provides a PRD, Planner agent breaks it into structured sprints with stories and acceptance criteria.
**FRs covered:** FR-V2-01, FR-V2-02

### Epic 2: Multi-Sprint Adversarial Loop [MVP]
The harness loops through all sprints: contract → build → evaluate → retry, with Evaluator using real tools to test.
**FRs covered:** FR-V2-03, FR-V2-04, FR-V2-05, FR-V2-06, FR-V2-07, FR-V2-08, FR-V2-09, FR-V2-10, FR-V2-14

### Epic 3: Dashboard — Multi-Sprint Status & Agent Terminals [MVP]
Dashboard shows real-time progress across all sprints with live agent terminal output.
**FRs covered:** FR-V2-11, FR-V2-12, FR-V2-13

### Epic 4: System Prompts & Adversarial Tuning [MVP]
Calibrate system prompts for each role based on the adversarial-dev patterns to maximize adversarial tension.
**FRs covered:** Cross-cutting quality improvement

---

## Epic 1: Planner Agent — PRD to Sprints [MVP]

The Planner agent receives a PRD and produces structured sprints. This is the missing piece that enables the full adversarial loop.

### Story 1.1: PRD Input in Dashboard

As a dashboard user,
I want to paste a PRD or upload a markdown file into the adversarial panel,
So that the Planner agent can break it into implementable sprints.

**Acceptance Criteria:**

**Given** Adversarial Mode is ON
**When** the user pastes text or uploads a .md file into the PRD textarea
**Then** the content is stored and ready to send to the Planner

**Given** the PRD textarea has content
**When** the user clicks [Start Adversarial Loop]
**Then** the PRD is sent in the POST body: `{ prd_content, generator_model, evaluator_model }`

**Technical Notes:**
- Extend the adversarial panel to have a larger textarea or file upload
- Extend POST /api/adversarial/start body to accept `prd_content: String`

---

### Story 1.2: Planner Agent Sprint Generation

As the daemon harness,
I want a Planner agent that receives a PRD and outputs structured sprints,
So that the Generator and Evaluator have clear scope for each sprint.

**Acceptance Criteria:**

**Given** POST /api/adversarial/start is received with prd_content
**When** the harness starts
**Then** a Planner agent is spawned with model claude-sonnet-4-20250514

**Given** the Planner agent is spawned
**When** it receives the PRD
**Then** it outputs a JSON spec: `{ sprints: [{ number, title, features[], criteria[] }] }`
**And** each sprint has 3-8 testable criteria with name, description, threshold

**Given** the Planner output is parsed
**Then** the harness stores the sprint plan in context: `adversarial:sprint_plan`
**And** the SprintTimeline in the dashboard populates with all sprints as "pending"
**And** the Planner agent pane is closed (its job is done)

**Given** JSON parsing fails
**Then** fallback: create a single sprint with the full PRD as scope and 3 default criteria

**Technical Notes:**
- Planner system prompt tells it to output structured JSON, not free-form markdown
- Parse sprint plan from PTY output using the existing JSON parser (parser.rs)
- Emit AdversarialStarted event with total_sprints from the plan

---

## Epic 2: Multi-Sprint Adversarial Loop [MVP]

Extend the existing single-sprint harness to loop through all sprints from the Planner's spec.

### Story 2.1: Multi-Sprint Loop in Harness

As the daemon harness,
I want to iterate through all sprints from the Planner's spec,
So that the Generator implements and Evaluator tests each sprint in sequence.

**Acceptance Criteria:**

**Given** a sprint plan with N sprints exists
**When** the harness enters the sprint loop
**Then** for each sprint 1..N:
  1. Contract negotiation: Generator proposes, Evaluator toughens (existing — extend for sprint-specific criteria)
  2. Build phase: Generator receives sprint features + contract + cumulative context from previous sprints
  3. Evaluate phase: Evaluator tests against sprint contract
  4. If PASS: emit AdversarialSprintPassed, advance to sprint N+1
  5. If FAIL and retries < 3: emit AdversarialRetry, send feedback to Generator
  6. If FAIL and retries exhausted: emit AdversarialFailed, STOP (do not continue)

**Given** all N sprints pass
**Then** emit AdversarialComplete with total sprints passed and total duration

**Given** sprint 3 of 5 fails after 3 retries
**Then** the harness stops at sprint 3 — sprints 4 and 5 are NOT attempted

**Technical Notes:**
- Modify run_inner() in harness.rs: wrap the existing build_evaluate_loop in a for loop over sprints
- Each sprint gets its own contract from negotiate_contract()
- Persist adversarial:sprint with current sprint number
- Generator receives context from previous sprints: "Sprint 1 implemented auth. Sprint 2 implemented API. Now implement Sprint 3: dashboard."

---

### Story 2.2: Evaluator with Real Tools

As the Evaluator agent,
I want to actually RUN the code, tests, and endpoints,
So that my evaluation is based on real execution, not just reading code.

**Acceptance Criteria:**

**Given** the Evaluator enters evaluation phase
**When** it receives the evaluation prompt
**Then** the prompt instructs it to: read code, run `cargo test`, run `npm test`, curl API endpoints, check for security issues (SQL injection, missing auth, hardcoded secrets)

**Given** the Evaluator runs `cargo test`
**And** 3 of 12 tests fail
**Then** the score for "tests_passing" criterion reflects the failure (e.g. 6/10)
**And** feedback includes exact test names and error messages

**Given** the Evaluator curls an API endpoint
**And** the response is 500
**Then** the score reflects the broken endpoint
**And** feedback includes the URL, expected response, and actual response

**Given** the Evaluator finds `unwrap()` on user input
**Then** the score for "error_handling" reflects the risk
**And** feedback includes file path and line number

**Technical Notes:**
- The Evaluator agent already has a PTY with bash — it CAN run commands
- The system prompt must explicitly instruct: "Run the application. Test it. Do not just read code."
- Use the EVALUATOR_SYSTEM_PROMPT from adversarial-dev as the base
- Add BMUX-specific instructions: "The workspace is in the current directory. Use `cargo test` for Rust, `npm test` for TypeScript."

---

### Story 2.3: Git Commits per Feature

As the Generator agent,
I want to commit to git after implementing each feature in a sprint,
So that the Evaluator can see atomic changes and the team can review history.

**Acceptance Criteria:**

**Given** the Generator implements a feature in a sprint
**When** the feature is complete
**Then** the Generator runs `git add -A && git commit -m "feat(sprint-N): <description>"`

**Given** the Generator needs to retry after feedback
**When** it fixes issues
**Then** each fix is committed: `fix(sprint-N): <what was fixed>`

**Technical Notes:**
- The Generator system prompt includes: "After completing each feature, commit your work with a descriptive message."
- Git is already available in the PTY pane

---

## Epic 3: Dashboard — Multi-Sprint Status & Agent Terminals [MVP]

### Story 3.1: Multi-Sprint Timeline & ScoreBoard

As a dashboard user,
I want to see all sprints in a timeline with per-sprint scores,
So that I can track the adversarial loop across the full project.

**Acceptance Criteria:**

**Given** the Planner produces 5 sprints
**Then** the SprintTimeline shows:
  Sprint 1: ✅ Passed (2 attempts)
  Sprint 2: 🔨 Building (attempt 1/3)
  Sprint 3: ⏳ Pending
  Sprint 4: ⏳ Pending
  Sprint 5: ⏳ Pending

**Given** Sprint 2 is being evaluated
**Then** the ScoreBoard shows Sprint 2's criteria with live scores

**Given** the user clicks Sprint 1 in the timeline
**Then** Sprint 1's final scores and evaluation history expand

**Technical Notes:**
- The SprintTimeline and ScoreBoard components already exist — extend them to handle N sprints
- Use adversarial:sprint_plan from context to get total sprints
- WebSocket events already include sprint number — use it to route to correct timeline entry

---

### Story 3.2: Live Agent Terminal in Dashboard

As a dashboard user,
I want to see what the Generator and Evaluator are doing in real time,
So that I can watch the adversarial tension play out.

**Acceptance Criteria:**

**Given** the adversarial loop is running
**Then** the dashboard shows two embedded terminal panels:
  Left: Generator (what it's implementing)
  Right: Evaluator (what it's testing/breaking)

**Given** the Generator writes code
**Then** the text appears in the Generator terminal panel in real time

**Given** the Evaluator runs `cargo test`
**Then** the test output appears in the Evaluator terminal panel in real time

**Technical Notes:**
- Use the existing xterm.js AgentTerminal component
- Subscribe to pane_output WebSocket events filtered by generator/evaluator pane_id
- The adversarial status panel already knows the pane IDs from agent spawn

---

## Epic 4: System Prompts & Adversarial Tuning [MVP]

### Story 4.1: Calibrated System Prompts

As the harness,
I want system prompts that maximize adversarial tension,
So that the Evaluator truly tries to break things and the Generator truly fights to pass.

**Acceptance Criteria:**

**Given** the Generator enters build phase
**Then** its prompt includes:
- "You are an expert software engineer. Build production-quality code."
- "Commit after each feature with git."
- "If retrying after feedback, read every feedback item. Address ALL issues."
- Sprint context from previous sprints

**Given** the Evaluator enters evaluation phase
**Then** its prompt includes:
- "You are a skeptical QA engineer. Your job is to BREAK this code."
- "Do NOT be generous. Resist the urge to praise mediocre work."
- "Run the application. Do not just read code and assume it works."
- "Test EVERY criterion. Check edge cases, not just happy path."
- "When something fails, provide SPECIFIC details: file paths, line numbers, exact errors."
- "CRITICAL: Kill any background processes before outputting your evaluation."
- Evaluation output must be JSON: `{ passed, scores[], feedback[], overallSummary }`

**Given** the Planner enters planning phase
**Then** its prompt includes:
- "You are a product architect. Expand this PRD into sprints."
- "Each sprint must be independently testable."
- "Output structured JSON with features and criteria per sprint."
- "Stay high-level — do NOT specify implementation details."

**Technical Notes:**
- Store system prompts in src/adversarial/prompts.rs (new file)
- Based on adversarial-dev shared/prompts.ts but adapted for BMUX context
- Generator and Evaluator prompts are NEVER shared — this is the context isolation (FR-V2-14)

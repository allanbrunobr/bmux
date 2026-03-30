# Features

<!-- Generated from BMAD artifacts by /skill:hive-md-from-bmad -->
<!-- Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md -->
<!-- Date: 2026-03-30 -->

## Feature 1: Adversarial Mode Toggle
- Description: [Epic 1: Dashboard Controls, Story 1.1] As a dashboard user, I want to toggle Adversarial Mode on/off from the dashboard header, so that I can switch between normal agent management and adversarial development. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: none
- Status: pending

## Feature 2: Model Selector for Generator & Evaluator
- Description: [Epic 1: Dashboard Controls, Story 1.2] As a dashboard user, I want to select different models for Generator (e.g. Sonnet) and Evaluator (e.g. Opus), so that I can balance speed vs rigor. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 1
- Status: pending

## Feature 3: Adversarial Prompt Input & Start
- Description: [Epic 1: Dashboard Controls, Story 1.3] As a dashboard user, I want to type a task prompt and click Start to trigger the adversarial loop via POST /api/adversarial/start. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 2
- Status: pending

## Feature 4: Spawn Generator & Evaluator Agents
- Description: [Epic 2: Backend Harness, Story 2.1] As the daemon, I want to spawn two Claude agents with different models and isolated contexts when adversarial mode starts. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 3
- Status: pending

## Feature 5: Contract Negotiation Phase
- Description: [Epic 2: Backend Harness, Story 2.2] As the daemon harness, I want Generator to propose criteria and Evaluator to toughen them, producing an agreed JSON contract before building. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 4
- Status: pending

## Feature 6: Build → Evaluate → Feedback → Retry Loop
- Description: [Epic 2: Backend Harness, Story 2.3] As the daemon harness, I want the adversarial loop: Generator builds → Evaluator scores 1-10 → feedback on failure → retry up to 3 times. Fully autonomous. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 5
- Status: pending

## Feature 7: Persist Adversarial State in Context Store
- Description: [Epic 2: Backend Harness, Story 2.4] As the daemon, I want all adversarial state (status, contract, scores, feedback) stored in BMUX context store for auditability and dashboard display. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 6
- Status: pending

## Feature 8: Adversarial REST API Endpoints
- Description: [Epic 2: Backend Harness, Story 2.5] As the web dashboard, I want REST endpoints: POST /api/adversarial/start, POST /api/adversarial/stop, GET /api/adversarial/status, GET /api/adversarial/history. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 7
- Status: pending

## Feature 9: Adversarial Status Panel
- Description: [Epic 3: Dashboard Status, Story 3.1] As a dashboard user, I want to see live adversarial loop status: current phase badge, sprint counter, attempt counter, progress bar. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 8
- Status: pending

## Feature 10: Score Board & Evaluation History
- Description: [Epic 3: Dashboard Status, Story 3.2] As a dashboard user, I want to see scores per criterion with pass/fail badges and full evaluation history across all attempts. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 9
- Status: pending

## Feature 11: Adversarial WebSocket Events
- Description: [Epic 3: Dashboard Status, Story 3.3] As a developer, I want the daemon to emit typed WebSocket events for all adversarial state changes (started, negotiating, building, evaluating, scores, retry, passed, failed, complete). Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 10
- Status: pending

## Feature 12: Planner Agent Integration
- Description: [Epic 4: Multi-Sprint Planning, Story 4.1] [v1.0] As a user, I want a 3rd Planner agent that expands a short prompt into a full spec with multiple sprints for long-running adversarial sessions. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 8
- Status: pending

## Feature 13: Sprint Progress Dashboard
- Description: [Epic 4: Multi-Sprint Planning, Story 4.2] [v1.0] As a dashboard user, I want a horizontal timeline showing all sprints with pass/fail status and click-to-expand evaluation details. Source: _bmad-output/planning-artifacts/adversarial-mode-epics.md
- Dependencies: Feature 12
- Status: pending

# Features

<!-- Generated from BMAD artifacts by /skill:hive-md-from-bmad -->
<!-- Source: _bmad-output/planning-artifacts/adversarial-v2-epics.md -->
<!-- Date: 2026-03-30 -->

## Feature 1: PRD Input in Dashboard
- Description: [Epic 1: Planner Agent, Story 1.1] As a dashboard user, I want to paste a PRD or upload a markdown file into the adversarial panel, so that the Planner agent can break it into sprints. Source: _bmad-output/planning-artifacts/adversarial-v2-epics.md
- Dependencies: none
- Status: pending

## Feature 2: Planner Agent Sprint Generation
- Description: [Epic 1: Planner Agent, Story 1.2] As the daemon harness, I want a Planner agent (Sonnet) that receives a PRD and outputs structured JSON sprints with features and testable criteria per sprint. Source: _bmad-output/planning-artifacts/adversarial-v2-epics.md
- Dependencies: Feature 1
- Status: pending

## Feature 3: Multi-Sprint Loop in Harness
- Description: [Epic 2: Multi-Sprint Loop, Story 2.1] As the daemon harness, I want to iterate through all N sprints: contract negotiation → build → evaluate → retry per sprint, stopping if any sprint fails 3x. Source: _bmad-output/planning-artifacts/adversarial-v2-epics.md
- Dependencies: Feature 2
- Status: pending

## Feature 4: Evaluator with Real Tools
- Description: [Epic 2: Multi-Sprint Loop, Story 2.2] As the Evaluator agent (Opus), I want to RUN code, tests, and endpoints — not just read code. Uses cargo test, curl, grep for security issues. Feedback with file paths and line numbers. Source: _bmad-output/planning-artifacts/adversarial-v2-epics.md
- Dependencies: Feature 3
- Status: pending

## Feature 5: Git Commits per Feature
- Description: [Epic 2: Multi-Sprint Loop, Story 2.3] As the Generator agent, I want to commit to git after each feature for atomic history and rollback. Source: _bmad-output/planning-artifacts/adversarial-v2-epics.md
- Dependencies: Feature 4
- Status: pending

## Feature 6: Multi-Sprint Timeline & ScoreBoard
- Description: [Epic 3: Dashboard Status, Story 3.1] As a dashboard user, I want the SprintTimeline to show all N sprints with real-time status and the ScoreBoard to show per-sprint criteria scores. Source: _bmad-output/planning-artifacts/adversarial-v2-epics.md
- Dependencies: Feature 5
- Status: done

## Feature 7: Live Agent Terminal in Dashboard
- Description: [Epic 3: Dashboard Status, Story 3.2] As a dashboard user, I want to see Generator and Evaluator terminal output side by side in real time via xterm.js while the loop runs. Source: _bmad-output/planning-artifacts/adversarial-v2-epics.md
- Dependencies: Feature 6
- Status: done

## Feature 8: Calibrated System Prompts
- Description: [Epic 4: System Prompts, Story 4.1] As the harness, I want system prompts that maximize adversarial tension — Evaluator tries to BREAK, Generator fights to PASS. Based on adversarial-dev patterns. Source: _bmad-output/planning-artifacts/adversarial-v2-epics.md
- Dependencies: Feature 2
- Status: pending

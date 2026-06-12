# ADR-003: Workflow Execution via Daemon IPC

**Status:** Accepted  
**Date:** 2026-06-11

## Context

`bmux workflow run` was a stub; `WorkflowEngine` existed but was not owned by the session daemon.

## Decision

1. Add `ClientMessage::WorkflowRun` and `WorkflowStatus` to the TUI protocol.
2. `DaemonServer` owns `Arc<WorkflowEngine>` sharing `ContextStore` and `TaskRouter` with the session.
3. `WorkflowRun` parses YAML, spawns `engine.run()` in a background task, returns `run_id` immediately.
4. `WorkflowStatus` reads `workflow_run:{id}` from the shared context store.

## Consequences

- Positive: CLI and daemon share one task router and context store per session.
- Negative: workflow steps depend on agents publishing IPC `Result` messages; without a live agent the step times out.
- Update (2026-06-12): engine calls `wait_for_task` and stores `result.content` with real cost.

## Compliance

- agentic-design-patterns: orchestration loop owned by daemon
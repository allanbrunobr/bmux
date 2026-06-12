# ADR-002: Daemon Security Envelope and Agent Runtime

**Status:** Accepted  
**Date:** 2026-06-11

## Context

Production spawn bypassed sandbox, audit, and safe command building. `--dangerously-skip-permissions` was hardcoded. Task results never returned agents to Idle.

## Decision

1. **`SecurityEnvelope`** (`src/security/envelope.rs`) is the single entry for scrub, task sanitization, launch-command quoting, and audit events.
2. **`AgentRuntime`** (`src/agents/runtime.rs`) resolves config from `agent_catalog` and builds shell-safe launch commands.
3. **`DaemonServer`** wires:
   - `message_bus.start()` on boot
   - subscriber: `MessageType::Result` → `TaskRouter::handle_result`
   - `AgentSpawn` / `TaskSend` / `ContextSet` / `AgentKill` through envelope + runtime
4. Dangerous Claude flag is **opt-in** via `[security] allow_dangerous_permissions = true`.
5. Builtin agent defaults live in `src/config/agent_catalog.rs` (SSOT).

## Consequences

- Positive: CI green; task loop closes; audit on spawn/kill/task/context.
- Negative: Linux Landlock cannot apply to PTY-typed spawns; macOS uses `sandbox-exec` wrapper when enabled.
- Follow-up: `create_secret_fd` requires structured `spawn_command` (not shell line injection).
- Workflow `run` wired via `WorkflowRun` / `WorkflowStatus` IPC (ADR-003).

## Compliance

- agent-security: least privilege (dangerous flags opt-in), task sanitization
- software-architecture: subsystems wired to production path, not diagram-only
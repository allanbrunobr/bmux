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
- Negative: PTY-based spawn still runs inside `$SHELL`; full `pre_exec` sandbox requires adapter/PTY bridge (tracked separately).
- Negative: workflow `run` without `--dry-run` remains stub until WorkflowRun IPC lands.

## Compliance

- agent-security: least privilege (dangerous flags opt-in), task sanitization
- software-architecture: subsystems wired to production path, not diagram-only
# ADR-004: Structured PTY Agent Spawn

**Status:** Accepted  
**Date:** 2026-06-12

## Context

Phase 2 used `split_and_run_command` with a shell string built by `SecurityEnvelope::build_agent_launch_command`. That path is vulnerable to injection via model/args and cannot pass file descriptors (e.g. `create_secret_fd`).

`portable-pty` closes all fds ≥ 3 before exec. BMUX uses `tui::pty_spawn` with selective `close_fds_except` so `create_secret_fd` and `BMUX_IPC_HMAC_FD` survive into the child.

## Decision

1. **`build_agent_pty_command`** (`src/security/agent_spawn.rs`) builds a `CommandBuilder` with `env_clear`, BMUX allowlist env, optional sandbox wrap (`sandbox-exec` / `bwrap`).
2. **`Pane::spawn_with_command`** spawns directly; **`Session::split_and_spawn_agent`** is the daemon entry point.
3. **`AgentRuntime::build_pty_command`** delegates to `agent_spawn`; string launch remains for legacy shell panes only.
4. **`AgentRegistry.pid`** is set from `Pane::process_id()` after spawn.

## Consequences

- Positive: no shell line injection on agent spawn; PID tracked for kill.
- Negative: Landlock cannot apply inside the PTY child; macOS/Linux use wrapper binaries.
- Resolved: fd-based secrets via `pty_spawn` (see ADR-004 follow-up, 2026-06-12).

## Compliance

- agent-security: sanitize at boundary, least privilege (env -i)
- genai-integration-patterns: structured argv instead of PTY shell injection
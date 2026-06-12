# ADR-004: Structured PTY Agent Spawn

**Status:** Accepted  
**Date:** 2026-06-12

## Context

Phase 2 used `split_and_run_command` with a shell string built by `SecurityEnvelope::build_agent_launch_command`. That path is vulnerable to injection via model/args and cannot pass file descriptors (e.g. `create_secret_fd`).

`portable-pty` closes all fds ≥ 3 before exec, so secret fd injection must use `BMUX_SECRETS_PATH` until a non–close-fds spawn path exists.

## Decision

1. **`build_agent_pty_command`** (`src/security/agent_spawn.rs`) builds a `CommandBuilder` with `env_clear`, BMUX allowlist env, optional sandbox wrap (`sandbox-exec` / `bwrap`).
2. **`Pane::spawn_with_command`** spawns directly; **`Session::split_and_spawn_agent`** is the daemon entry point.
3. **`AgentRuntime::build_pty_command`** delegates to `agent_spawn`; string launch remains for legacy shell panes only.
4. **`AgentRegistry.pid`** is set from `Pane::process_id()` after spawn.

## Consequences

- Positive: no shell line injection on agent spawn; PID tracked for kill.
- Negative: Landlock cannot apply inside the PTY child; macOS/Linux use wrapper binaries.
- Follow-up: fd-based secrets when spawn API allows preserving fds.

## Compliance

- agent-security: sanitize at boundary, least privilege (env -i)
- genai-integration-patterns: structured argv instead of PTY shell injection
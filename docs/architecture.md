# BMUX Architecture

BMUX is a terminal multiplexer that orchestrates multiple AI agent CLIs in shared sessions.

## Runtime components

```
┌─────────────┐     Unix socket      ┌──────────────────┐
│  bmux CLI   │ ───────────────────► │  DaemonServer    │
│  bmux TUI   │   /tmp/bmux-{s}.sock │  (session+PTY)   │
└─────────────┘                      └────────┬─────────┘
                                            │
                    ┌───────────────────────┼───────────────────────┐
                    ▼                       ▼                       ▼
             AgentRegistry          ContextStore            TaskRouter
             (pane metadata)        (sled per session)      (queues + dispatch)
                                            │
                                            ▼
                                    MessageBus (IPC)
                              /tmp/bmux-{s}-ipc.sock (0600, HMAC)
```

## Production paths (wired)

| Operation | Path |
|-----------|------|
| Spawn agent | CLI/TUI → `DaemonServer` → `AgentRuntime` → `SecurityEnvelope::build_agent_launch_command` → PTY pane |
| Send task | `TaskRouter` dispatch + `SecurityEnvelope::sanitize_task_content` → PTY stdin |
| Task result | Agent IPC `Result` → `MessageBus` → subscriber → `TaskRouter::handle_result` → agent Idle + next queue |
| Context | `ContextStore` (per-session DB); `ContextSet` audited via envelope |
| Config | `agent_catalog` SSOT for builtins; `[agents.<name>]` for custom agents |

## Security layers

1. **Envelope** — scrub secrets, reject multiline/NUL tasks, shell-quote spawn env
2. **IPC bus** — HMAC on agent messages (socket 0600)
3. **Daemon socket** — 0600 after bind
4. **Secrets** — `~/.config/bmux/secrets.toml` (0600), fd injection API in `security/secrets.rs`
5. **Sandbox** — `security/sandbox.rs` (Landlock/Seatbelt/namespace); PTY spawn integration pending

## Module map

| Module | Role |
|--------|------|
| `tui/` | Daemon, panes, protocol, client |
| `agents/` | Adapters, registry, runtime, catalog |
| `orchestration/` | MessageBus, TaskRouter |
| `storage/` | Context, sessions, audit log |
| `security/` | HMAC, sandbox, secrets, envelope |
| `workflow/` | YAML DAG parser + engine (dry-run local; run via IPC planned) |
| `config/` | TOML settings + agent catalog |

## ADRs

See [docs/adr/](adr/).
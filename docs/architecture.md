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
| Spawn agent | CLI/TUI → `DaemonServer` → `AgentRuntime::build_pty_command` → `Pane::spawn_with_command` (structured argv) |
| Send task | `TaskRouter` dispatch → `TaskDispatchEvent` → sanitize → PTY stdin (audit on success) |
| Task result | Agent IPC `Result` → `MessageBus` → subscriber → `TaskRouter::handle_result` → agent Idle + next queue |
| Workflow step | `send_to_agent` → `wait_for_task` → store `result.content` in context (`author: workflow`) |
| Context | `ContextStore` (per-session DB); writes record `author`; agent namespacing enforced via daemon `ContextSet`; scrub + audit |
| Config | `agent_catalog` SSOT for builtins; `[agents.<name>]` for custom agents |

## Security layers

1. **Envelope** — scrub secrets, reject multiline/NUL tasks, shell-quote spawn env
2. **IPC bus** — HMAC on agent messages (socket 0600)
3. **Daemon socket** — 0600 after bind
4. **Secrets** — `~/.config/bmux/secrets.toml` (0600), fd injection API in `security/secrets.rs`
5. **Sandbox** — `security/sandbox.rs`; PTY spawn uses `sandbox-exec`/`bwrap` wrapper when enabled
6. **Resilience** — IPC query timeout (5s), task prune after completion, `max_cost_usd` on auto-route

## Module map

| Module | Role |
|--------|------|
| `tui/` | Daemon, panes, protocol, client (zoom/scroll/reload via snapshots) |
| `agents/` | Adapters, registry, runtime, catalog |
| `orchestration/` | MessageBus, TaskRouter |
| `storage/` | Context, sessions, audit log |
| `security/` | HMAC, sandbox, secrets, envelope |
| `workflow/` | YAML DAG parser + engine (run via `WorkflowRun` IPC; steps wait for task results) |
| `config/` | TOML settings + agent catalog |

## ADRs

See [docs/adr/](adr/).
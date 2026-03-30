# BMUX — Codebase Summary

## Project

- **Name:** BMUX
- **Language:** Rust 1.78+
- **Framework:** ratatui 0.28 (TUI), crossterm 0.27 (terminal), tokio 1.x (async)
- **Stack:** portable-pty, sled 0.34, ring 0.17, clap 4.x, serde
- **Root:** /Users/bruno/Desktop/BMUX/bmux

## Statistics

| Metric | Count |
|--------|-------|
| Total .rs files | 43 |
| Lines of Rust | 12,667 |
| Test files | 10 |
| Modules | 7 (tui, agents, config, orchestration, security, storage, workflow) |
| Hub files | 5 |

## Architecture

4-layer architecture:
1. **TUI** — ratatui rendering, pane/window/session management, client-server protocol
2. **Orchestration** — MessageBus (Unix socket + HMAC), TaskRouter (scoring), SharedContext
3. **Agents** — AgentAdapter trait, GenericCliAdapter (PTY-based), 6 adapter implementations
4. **Storage** — sled KV (context + sessions), JSONL audit logging

## Semantic Clusters

### Cluster 1: TUI & Rendering (12 files)
Core terminal multiplexer: panes, windows, sessions, keybindings, scroll, zoom, status bar, layout (BSP tree), protocol (client↔server JSON), client renderer, daemon server.

### Cluster 2: Agent System (11 files)
Agent lifecycle: AgentAdapter trait, GenericCliAdapter (PTY spawn), ShellAdapter, ClaudeCode/OpenCode/Pi/Gemini/Custom adapters, AgentRegistry, CLI commands.

### Cluster 3: Orchestration (4 files)
Inter-agent communication: MessageBus (Unix socket + HMAC-SHA256), TaskRouter (scoring algorithm, queue, auto-routing), SharedContextStore (IPC wrapper).

### Cluster 4: Storage (4 files)
Persistence: sled-backed ContextStore (TTL, namespaced), SessionStore (detach/reattach), AuditLogger (JSONL).

### Cluster 5: Security (4 files)
Sandboxing: Landlock/Seatbelt/namespace, HMAC-SHA256 signing, SecretsManager (sealed FD injection).

### Cluster 6: Workflow (3 files)
YAML DAG parser, parallel step execution engine, template parameter injection.

### Cluster 7: Config & Distribution (5 files)
TOML config with hot reload, CI/CD pipelines, Homebrew formula, build.rs version embedding.

## Hub Files (High Impact)

| File | Imported By | Impact Score | Intent |
|------|-------------|-------------|--------|
| `src/agents/adapter.rs` | 12 files | 24 | Core trait defining agent interface (spawn, send_task, kill, status) |
| `src/config/settings.rs` | 7 files | 35 | Configuration loading, defaults, agent settings, hot reload |
| `src/storage/context_store.rs` | 5 files | 37 | Sled-backed KV store with TTL, session namespacing, JSON dump |
| `src/orchestration/message_bus.rs` | 3 files | 10 | Unix socket IPC with HMAC, message routing, agent status map |
| `src/tui/server.rs` | 1 file (entry) | 8 | Daemon server owning all state (session, agents, context, router) |

## Key Dependencies

- `agents::adapter` → used by all 6 adapters + commands + task_router + tests
- `config::settings` → used by agents, server, TUI, workflow, tests
- `storage::context_store` → used by orchestration, workflow engine, session_store, server
- `orchestration::message_bus` → used by task_router, server
- `tui::protocol` → shared between client and server

# Worktree wt1 — COMPLETE

Branch: `bmux-web-wt1`
Completed: 2026-03-29

## Stories implemented

| Story | Title | Status |
|-------|-------|--------|
| 1.1 | Axum Server Scaffold & Sessions Endpoint | done |
| 1.2 | Agents Endpoint | done |
| 1.3 | Context, Tasks & Metrics Endpoints | done |
| 1.4 | Static File Serving & `bmux web` CLI | done |
| 1.5 | POST /api/tasks — Send Task from Browser | done |
| 2.1 | WebSocket Endpoint & Event Broadcaster | done |
| 2.2 | BmuxEvent Enum & Emission Points | done |
| 2.3 | Client Reconnection & Multi-Client Support | done |

## Files created / modified

### New files
- `src/web/mod.rs` — web server entry point, spawns Axum on port 7432
- `src/web/events.rs` — `BmuxEvent` enum (agent_spawned, agent_killed, context_updated, task_dispatched, task_completed)
- `src/web/routes.rs` — all REST handlers + Axum `AppState`
- `src/web/websocket.rs` — WS upgrade handler, snapshot-on-connect, broadcast fan-out

### Modified files
- `Cargo.toml` — added axum 0.7 (ws), tower-http 0.5 (cors+fs), tokio-tungstenite 0.21
- `src/lib.rs` — added `pub mod web;`
- `src/tui/server.rs` — made `DaemonState` pub(crate), added `web_events_tx`, spawns Axum, emits events
- `src/main.rs` — added `bmux web [--port N] [--no-open]` subcommand

## API surface

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/sessions` | List all active sessions |
| GET | `/api/agents?session=<name>` | List agents in session |
| GET | `/api/context?session=<name>` | List context entries |
| GET | `/api/tasks?session=<name>` | List tasks |
| GET | `/api/metrics?session=<name>` | Aggregate metrics |
| POST | `/api/tasks` | Send task to agent |
| GET | `/ws?session=<name>` | WebSocket stream |
| GET | `/` | Static Next.js build (web-dist/) |

## Test results

`cargo test` — all tests pass (32 unit + integration tests).

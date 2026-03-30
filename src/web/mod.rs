/// HTTP API & WebSocket server for the BMUX web dashboard.
///
/// Exposes:
///  - GET  /api/sessions          — list active sessions
///  - GET  /api/agents            — agents in a session
///  - GET  /api/tasks             — task queue
///  - GET  /api/context           — shared context entries
///  - GET  /api/pane-output       — last N lines of raw PTY output (Story 4.1)
///  - GET  /api/audit             — audit log entries
///  - POST /api/tasks             — send a task to an agent
///  - GET  /ws                    — WebSocket event stream
pub mod routes;
pub mod state;

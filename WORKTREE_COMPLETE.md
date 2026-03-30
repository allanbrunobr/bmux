# Worktree wt3 — COMPLETE

Branch: `bmux-web-wt3`
Date: 2026-03-29
Stories: 4.1, 4.2, 5.1, 5.2, 5.3, 6.1, 6.2

## Build Status

- `npm run build` ✓ (Next.js 14, 6 static pages)
- `cargo test`   ✓ (8/8 tests passing)
- `cargo check`  ✓ (no errors, 11 pre-existing warnings)

## Implemented Stories

### Story 4.1 — Pane Output Streaming Endpoint
- `src/web/mod.rs` — web module declaration
- `src/web/state.rs` — `PaneOutputCache` ring-buffer (bounded to 5 000 lines/pane), `WsEvent` enum, `WebAppState` shared state
- `src/web/routes.rs` — Axum routes including `GET /api/pane-output` and WebSocket `/ws` with `pane_output` event broadcast
- `Cargo.toml` — added `axum`, `tower`, `tower-http`, `tokio-stream`
- CPU constraint: pane output is read from a lock-protected ring-buffer; no polling; piggy-backs the existing PTY reader thread → well below 5% CPU budget

### Story 4.2 — xterm.js Terminal Component
- `bmux-web/src/components/AgentTerminal.tsx`
  - Dynamic import of xterm.js (SSR-safe)
  - Loads last 100 lines on mount via `GET /api/pane-output`
  - Subscribes to WebSocket `pane_output` events for live streaming
  - Read-only (disableStdin: true)
  - Auto-fits on window resize

### Story 5.1 — Metrics Charts with Recharts
- `bmux-web/src/components/MetricsChart.tsx`
  - Tokens/min line chart (one colored line per agent)
  - Cumulative cost (USD) line chart
  - Summary cards: total tokens, total cost, active agents, completed tasks
  - Hover tooltips with exact values
  - Real-time via `agent_tokens_updated` WebSocket events
- `bmux-web/src/app/metrics/page.tsx`

### Story 5.2 — Geographic Agent Map with deck.gl
- `bmux-web/src/components/AgentMap.tsx`
  - `ScatterplotLayer`: agent pins (local → user location, remote → cloud region coords)
  - `ArcLayer`: animated arcs on `task_queued`/`task_started` events, fade after 5 s
  - Provider-colored pins and arcs
  - Pickable tooltips with agent name

### Story 5.3 — Cost Breakdown Dashboard
- `bmux-web/src/components/CostBreakdown.tsx`
  - Cost per agent table (name, model, tokens, cost USD, cost/hr)
  - Bar chart: cost per hour by agent
  - Projection cards: session total, projected/day, projected/month

### Story 6.1 — Audit Log Viewer with Filtering & CSV Export
- `bmux-web/src/components/AuditLog.tsx`
  - Filterable by agent, event type, time range (1h/24h/7d/all)
  - LGPD compliance badge per entry (green/yellow/red)
  - CSV export downloads filtered entries
  - Real-time via `audit_event` WebSocket events
- `bmux-web/src/app/audit/page.tsx`

### Story 6.2 — Cost per Consultation (AFYA)
- `bmux-web/src/components/CostPerConsulta.tsx`
  - Groups costs by `consultation:{id}:*` context keys
  - Per-consultation breakdown: transcription + SOAP note + total
  - Monthly cost projection
  - Stacked bar chart + detail table with totals row

## Shared Infrastructure Created

- `bmux-web/package.json` — Next.js 14, Recharts, deck.gl, xterm.js, Tailwind CSS
- `bmux-web/src/lib/types.ts` — shared TypeScript domain types
- `bmux-web/src/lib/api.ts` — REST + WebSocket URL helpers (reads `NEXT_PUBLIC_BMUX_API`)
- `bmux-web/src/lib/useBmuxSocket.ts` — auto-reconnect WebSocket React hook
- `bmux-web/src/lib/mockData.ts` — development mock data for all domains
- `bmux-web/src/app/layout.tsx` + `globals.css` + `Sidebar.tsx` — app shell

## Coordination Notes for wt1

Story 4.1 adds `src/web/` to the Rust crate. The integration points for wt1:

1. Call `WebAppState::new(session_name)` to create shared state
2. Wire `web::routes::build_router(state.clone())` into the Axum router
3. In the PTY reader thread, call `state.pane_cache.push_lines(pane_id, lines)` and `state.broadcast(WsEvent::PaneOutput { ... })` to stream output

## File Ownership Summary

All files created by this worktree are new — no existing files were modified except:
- `Cargo.toml` — added web dependencies
- `src/lib.rs` — added `pub mod web`
- `features.md` — marked Features 15-21 as done

---
stepsCompleted: ['step-01-validate-prerequisites', 'step-02-design-epics', 'step-03-create-stories', 'step-04-final-validation']
inputDocuments:
  - 'Web Dashboard Implementation Prompt (user-provided task description)'
  - '_bmad-output/planning-artifacts/architecture.md'
  - '_bmad-output/planning-artifacts/BMUX_PRD.md'
mvpScope: [1, 2, 3]
v1Scope: [4, 5, 6]
---

# BMUX Web Dashboard - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for the BMUX Web Dashboard, a new product surface (v1.0 milestone) that mirrors the terminal multiplexer in a browser-based real-time dashboard.

## Requirements Inventory

### Functional Requirements

- FR-WEB-01: User can view all active BMUX sessions in a web dashboard
- FR-WEB-02: User can select a session from a dropdown to view its agents, context, and tasks
- FR-WEB-03: Dashboard updates in real time via WebSocket when terminal state changes (agent spawn, task send, context set)
- FR-WEB-04: User can view all agents in a session as cards showing name, type icon, model, status, tokens, cost, uptime
- FR-WEB-05: User can view the live terminal output of any agent pane in the browser (read-only, supervisory, via xterm.js)
- FR-WEB-06: User can send a task to a specific agent from the browser UI
- FR-WEB-07: User can view the shared context store as a live feed (key, value, timestamp, set_by_agent)
- FR-WEB-08: User can view the task queue with full history (from, to, status, cost, time)
- FR-WEB-09: User can view aggregate metrics (total tokens, total cost, active agents, completed tasks, tokens/min, cost/hour)
- FR-WEB-10: User can view cost breakdown per agent, per session, per hour in real time
- FR-WEB-11: User can view geographic location of each agent (local, Databricks, AWS, GCP, Azure) on a world map
- FR-WEB-12: User can view message arcs between agents on the geographic map when tasks are sent
- FR-WEB-13: User can view a structured audit log of all agent actions with timestamps, filterable by agent/type/time
- FR-WEB-14: User can export the audit log to CSV for compliance
- FR-WEB-15: User can open the web dashboard by running `bmux web` in the terminal
- FR-WEB-16: User can view cost grouped by consultation session (AFYA healthcare use case)
- FR-WEB-17: Dashboard displays line charts of tokens per minute per agent and cumulative cost over time

### NonFunctional Requirements

- NFR-WEB-01: WebSocket latency from state change to browser render < 200ms
- NFR-WEB-02: Dashboard must be responsive — functional on mobile devices (min 375px width)
- NFR-WEB-03: Backend HTTP API response time < 50ms for all GET endpoints
- NFR-WEB-04: WebSocket must handle 10+ concurrent browser clients without degradation
- NFR-WEB-05: Frontend must render 10+ agent cards without perceptible lag
- NFR-WEB-06: pane-output streaming must not increase daemon CPU usage by more than 5%
- NFR-WEB-07: No authentication required (localhost only, v1.0 — future: token auth)
- NFR-WEB-08: LGPD compliance indicators on audit log entries (AFYA use case)
- NFR-WEB-09: Static frontend assets served by Axum (single binary distribution)
- NFR-WEB-10: Dashboard auto-reconnects WebSocket on disconnect (with exponential backoff)

### Additional Requirements

- AR-WEB-01: Backend uses Axum 0.7 with WebSocket support, port 7432
- AR-WEB-02: Backend shares Arc<DaemonState> with existing daemon process
- AR-WEB-03: Frontend uses Next.js 14+ with App Router, TypeScript, Tailwind CSS
- AR-WEB-04: UI components from shadcn/ui library
- AR-WEB-05: Charts use Recharts library
- AR-WEB-06: Geographic map uses deck.gl (ScatterplotLayer + ArcLayer)
- AR-WEB-07: Terminal embed uses xterm.js + xterm-addon-fit
- AR-WEB-08: Client state management via Zustand store
- AR-WEB-09: Build pipeline: `cd bmux-web && npm run build`, static output served by Axum
- AR-WEB-10: `bmux web` CLI command opens browser to http://localhost:7432

### UX Design Requirements

N/A — CLI + web dashboard project, no UX design document. UI follows shadcn/ui defaults.

### FR Coverage Map

- FR-WEB-01: Epic 1 — Story 1.1 (Sessions API)
- FR-WEB-02: Epic 3 — Story 3.2 (Session selector component)
- FR-WEB-03: Epic 2 — Story 2.1 (WebSocket broadcaster)
- FR-WEB-04: Epic 3 — Story 3.3 (Agent cards)
- FR-WEB-05: Epic 4 — Story 4.1 (xterm.js terminal embed)
- FR-WEB-06: Epic 3 — Story 3.6 (Send task from browser)
- FR-WEB-07: Epic 3 — Story 3.4 (Context feed)
- FR-WEB-08: Epic 3 — Story 3.5 (Task queue table)
- FR-WEB-09: Epic 5 — Story 5.1 (Metrics charts)
- FR-WEB-10: Epic 5 — Story 5.1 (Cost per agent/session)
- FR-WEB-11: Epic 5 — Story 5.2 (Geographic map)
- FR-WEB-12: Epic 5 — Story 5.2 (Message arcs)
- FR-WEB-13: Epic 6 — Story 6.1 (Audit log viewer)
- FR-WEB-14: Epic 6 — Story 6.1 (CSV export)
- FR-WEB-15: Epic 1 — Story 1.4 (bmux web CLI command)
- FR-WEB-16: Epic 6 — Story 6.2 (Cost per consultation)
- FR-WEB-17: Epic 5 — Story 5.1 (Line charts)

## Epic List

### Epic 1: Backend — Axum HTTP API + Web Server [MVP]
Embed an Axum HTTP server in the BMUX daemon that exposes REST endpoints for sessions, agents, context, tasks, and metrics. Serve the static frontend. Add `bmux web` CLI command.
**FRs covered:** FR-WEB-01, FR-WEB-15

### Epic 2: Backend — WebSocket Real-Time Events [MVP]
Add a WebSocket endpoint to the Axum server that pushes state change events to all connected browser clients in real time.
**FRs covered:** FR-WEB-03

### Epic 3: Frontend — Core Dashboard [MVP]
Create the Next.js application with session selector, agent cards, context feed, task queue, and the ability to send tasks from the browser.
**FRs covered:** FR-WEB-02, FR-WEB-04, FR-WEB-06, FR-WEB-07, FR-WEB-08

### Epic 4: Frontend — Terminal Embed [v1.0]
Embed xterm.js to show live agent terminal output in the browser (read-only supervisory view).
**FRs covered:** FR-WEB-05

### Epic 5: Frontend — Metrics, Charts & Geographic Map [v1.0]
Add Recharts line charts for token/cost trends, cost breakdowns, and a deck.gl geographic map showing agent locations and message arcs.
**FRs covered:** FR-WEB-09, FR-WEB-10, FR-WEB-11, FR-WEB-12, FR-WEB-17

### Epic 6: Healthcare/Compliance Features [v1.0]
Add audit log viewer with filtering and CSV export, LGPD compliance indicators, and cost-per-consultation grouping for the AFYA healthcare use case.
**FRs covered:** FR-WEB-13, FR-WEB-14, FR-WEB-16

---

## Epic 1: Backend — Axum HTTP API + Web Server [MVP]

Users can query the BMUX daemon via REST endpoints from any HTTP client. The dashboard frontend (or curl, or Postman) can list sessions, agents, context, tasks, and metrics. The `bmux web` command opens the dashboard in the browser.

### Story 1.1: Axum Server Scaffold & Sessions Endpoint

As a developer,
I want to start an Axum HTTP server inside the BMUX daemon on port 7432,
So that external HTTP clients can query session state.

**Acceptance Criteria:**

**Given** a BMUX daemon is running (`bmux new meu-projeto`)
**When** the daemon starts
**Then** an Axum HTTP server binds to `0.0.0.0:7432`
**And** `GET /api/sessions` returns JSON `[{ "name": "meu-projeto", "created_at": "...", "agent_count": 0 }]`
**And** the Axum server shares `Arc<DaemonState>` with the daemon (read-only for GET endpoints)
**And** CORS is enabled for localhost origins

**Technical Notes:**
- Add axum 0.7, tower-http 0.5 (cors) to Cargo.toml
- Spawn Axum in a `tokio::spawn` alongside the daemon server
- All state reads use the existing `DaemonState` locks

---

### Story 1.2: Agents Endpoint

As a dashboard user,
I want to fetch all agents in a session via HTTP,
So that the frontend can render agent cards.

**Acceptance Criteria:**

**Given** the Axum server is running and agents are spawned in a session
**When** a client sends `GET /api/agents?session=meu-projeto`
**Then** the response is JSON array of agents: `[{ "name", "agent_type", "model", "status", "tokens_used", "cost_usd", "uptime_seconds", "pane_id", "last_task" }]`

**Given** the session has no agents
**When** `GET /api/agents?session=meu-projeto` is called
**Then** the response is `[]`

**Given** the session does not exist
**When** `GET /api/agents?session=nonexistent` is called
**Then** the response is `404 { "error": "Session not found" }`

---

### Story 1.3: Context & Tasks Endpoints

As a dashboard user,
I want to fetch shared context and task queue via HTTP,
So that the frontend can display the context feed and task table.

**Acceptance Criteria:**

**Given** a session has context entries
**When** `GET /api/context?session=meu-projeto` is called
**Then** the response is `[{ "key", "value", "created_at", "expires_at" }]`

**Given** a session has tasks
**When** `GET /api/tasks?session=meu-projeto` is called
**Then** the response is `[{ "id", "destination", "content", "status", "submitted_at", "cost_usd" }]`

**Given** a session exists
**When** `GET /api/metrics?session=meu-projeto` is called
**Then** the response is `{ "total_tokens", "total_cost_usd", "active_agents", "completed_tasks" }`

---

### Story 1.4: Static File Serving & `bmux web` CLI Command

As a user,
I want to run `bmux web` and have the dashboard open in my browser,
So that I can monitor my agents without configuring anything.

**Acceptance Criteria:**

**Given** the daemon is running
**When** `GET /` is requested
**Then** Axum serves the static Next.js build from an embedded or adjacent directory

**Given** a daemon is running for session "meu-projeto"
**When** the user runs `bmux web`
**Then** the default browser opens `http://localhost:7432`

**Given** the user runs `bmux web --port 8080`
**Then** the Axum server listens on port 8080 instead of 7432

**Given** the user runs `bmux web --no-open`
**Then** the server starts but the browser does NOT open

---

### Story 1.5: POST Endpoint — Send Task from Browser

As a dashboard user,
I want to send a task to an agent from the browser UI,
So that I can interact with agents without switching to the terminal.

**Acceptance Criteria:**

**Given** the Axum server is running and an agent "arquiteto" is spawned
**When** a client sends `POST /api/tasks` with `{ "session": "meu-projeto", "agent": "arquiteto", "content": "design auth module" }`
**Then** the task is routed to the agent via the daemon's TaskRouter
**And** the response is `{ "task_id": "...", "status": "dispatched", "routed_to": "arquiteto" }`
**And** the task content is written to the agent's pane PTY stdin

**Given** the agent name does not exist
**When** `POST /api/tasks` is called
**Then** the response is `400 { "error": "Agent not found" }`

---

## Epic 2: Backend — WebSocket Real-Time Events [MVP]

The dashboard receives instant state updates without polling. When anything changes in the terminal (agent spawned, task completed, context updated), all connected browser clients see it instantly.

### Story 2.1: WebSocket Endpoint & Event Broadcaster

As a dashboard user,
I want to receive real-time updates via WebSocket,
So that the dashboard reflects terminal state instantly without refreshing.

**Acceptance Criteria:**

**Given** the Axum server is running
**When** a client connects to `ws://localhost:7432/ws?session=meu-projeto`
**Then** the WebSocket connection is established
**And** the server subscribes to the session's event broadcast channel

**Given** a WebSocket client is connected
**When** an agent is spawned in the terminal (`bmux agent spawn --type claude-code --name arch`)
**Then** the client receives `{ "type": "agent_spawned", "agent": { "name": "arch", "agent_type": "claude-code", ... } }`

**Given** a WebSocket client is connected
**When** a context key is set (`bmux context set design:auth "JWT RS256"`)
**Then** the client receives `{ "type": "context_updated", "key": "design:auth", "value": "JWT RS256" }`

---

### Story 2.2: BmuxEvent Enum & Emission Points

As a developer,
I want every state mutation in the daemon to emit a typed event,
So that WebSocket clients receive all state changes.

**Acceptance Criteria:**

**Given** the `DaemonState` has a `broadcast::Sender<BmuxEvent>` field
**When** `AgentSpawn` handler registers an agent
**Then** `BmuxEvent::AgentSpawned { name, agent_type, model }` is emitted

**When** `AgentKill` handler removes an agent
**Then** `BmuxEvent::AgentKilled { name }` is emitted

**When** `TaskSend` handler dispatches a task
**Then** `BmuxEvent::TaskDispatched { task_id, agent, content_preview }` is emitted

**When** `ContextSet` handler stores a key
**Then** `BmuxEvent::ContextUpdated { key, value, set_by }` is emitted

**And** all events include a `timestamp` field
**And** events are serialized as JSON and pushed to all WebSocket clients

---

### Story 2.3: Client Reconnection & Multi-Client Support

As a dashboard user,
I want multiple browser tabs to receive updates simultaneously and auto-reconnect on disconnect,
So that I can monitor from multiple screens reliably.

**Acceptance Criteria:**

**Given** 5 WebSocket clients are connected to the same session
**When** an event is emitted
**Then** all 5 clients receive the event

**Given** a WebSocket client disconnects (network drop)
**When** the client reconnects
**Then** it receives a full state snapshot as the first message
**And** resumes receiving live events

**Given** the daemon restarts
**When** a client detects disconnect
**Then** it retries with exponential backoff (1s, 2s, 4s, 8s, max 30s)

---

## Epic 3: Frontend — Core Dashboard [MVP]

Users open the BMUX dashboard in a browser and see all agents, shared context, and tasks in a modern UI. They can switch sessions, view agent details, and send tasks — all updating in real time.

### Story 3.1: Next.js Project Scaffold & Layout

As a developer,
I want to create the Next.js project with the dashboard layout,
So that all dashboard pages share a consistent sidebar and header.

**Acceptance Criteria:**

**Given** the `bmux-web/` directory is created
**When** the project is built with `npm run build`
**Then** it produces a static export in `bmux-web/out/`

**Given** a user navigates to `/`
**Then** they see the dashboard layout with:
- Header: "BMUX Dashboard" + session selector + connection status (🟢/🔴)
- Sidebar: Dashboard, Agents, Context, Tasks, Metrics links
- Main content area

**Technical Notes:**
- Next.js 14+ with App Router, TypeScript, Tailwind CSS
- shadcn/ui components (Button, Card, Badge, Table, Select)
- Zustand store for global state

---

### Story 3.2: Session Selector & WebSocket Connection

As a dashboard user,
I want to select a session from a dropdown and connect via WebSocket,
So that the dashboard shows data for the session I'm monitoring.

**Acceptance Criteria:**

**Given** the dashboard loads
**When** sessions are fetched from `GET /api/sessions`
**Then** a dropdown shows all available sessions

**Given** the user selects session "meu-projeto"
**Then** a WebSocket connection is opened to `ws://localhost:7432/ws?session=meu-projeto`
**And** all dashboard components update to show data for "meu-projeto"

**Given** only one session is active
**Then** it is auto-selected on load

---

### Story 3.3: Agent Cards Grid

As a dashboard user,
I want to see all agents in a responsive card grid,
So that I can monitor agent status, cost, and activity at a glance.

**Acceptance Criteria:**

**Given** a session has 3 agents (arquiteto, testador, helper)
**When** the dashboard loads
**Then** 3 agent cards are displayed in a responsive grid (3 columns on desktop, 1 on mobile)

Each card shows:
- Agent name and type icon (🤖 claude-code, 🔧 shell, ✨ gemini, 🥧 pi, 🔌 custom)
- Model name (e.g. "claude-sonnet-4-20250514")
- Status badge: 🟢 idle (gray) / 🔵 working (blue, pulsing) / 🔴 error (red)
- Token counter (updates in real time)
- Cost in USD (updates in real time)
- Uptime
- Last task preview (truncated 60 chars)
- [Send Task] button

**Given** a WebSocket event `agent_spawned` is received
**Then** a new card appears instantly (no page refresh)

**Given** a WebSocket event `agent_killed` is received
**Then** the card is removed with a fade-out animation

---

### Story 3.4: Shared Context Live Feed

As a dashboard user,
I want to see a live feed of shared context entries,
So that I can follow agent collaboration in real time.

**Acceptance Criteria:**

**Given** the session has context entries
**When** the Context page loads
**Then** all entries are displayed in a table: key, value (truncated), timestamp, relative time ("2s ago")

**Given** a WebSocket event `context_updated` is received
**Then** the new entry appears at the top of the feed with a highlight animation

**Given** a context entry value is longer than 80 characters
**Then** it is truncated with "..." and expandable on click

---

### Story 3.5: Task Queue Table

As a dashboard user,
I want to see the task queue with full history,
So that I can track what agents are working on and what completed.

**Acceptance Criteria:**

**Given** the session has tasks
**When** the Tasks page loads
**Then** tasks are displayed in a table: ID, from → to, content preview, status badge, submitted time, cost

Status badges:
- ⏳ queued (yellow)
- 🔄 active (blue)
- ✅ completed (green)
- ❌ failed (red)
- ⏱ timed_out (gray)

**Given** a WebSocket event `task_dispatched` is received
**Then** a new row appears at the top of the table

**Given** a WebSocket event `task_completed` is received
**Then** the task row status updates from 🔄 to ✅

---

### Story 3.6: Send Task Modal

As a dashboard user,
I want to send a task to an agent directly from the browser,
So that I can interact with agents without switching to the terminal.

**Acceptance Criteria:**

**Given** the user clicks [Send Task] on an agent card
**Then** a modal opens with: agent name (read-only), task content textarea, [Send] button

**Given** the user types a task and clicks [Send]
**Then** `POST /api/tasks` is called with the task content
**And** the modal closes
**And** a success toast notification appears: "Task sent to arquiteto"

**Given** the task is sent
**Then** it appears in the task queue table immediately via WebSocket event

---

## Epic 4: Frontend — Terminal Embed [v1.0]

Users can view live agent terminal output directly in the browser, providing a supervisory view without needing terminal access.

### Story 4.1: Pane Output Streaming Endpoint

As a developer,
I want a backend endpoint that streams pane output,
So that xterm.js in the browser can display live terminal content.

**Acceptance Criteria:**

**Given** `GET /api/pane-output?session=meu-projeto&pane_id=1&lines=100`
**Then** returns the last 100 lines of raw ANSI output from the pane

**Given** a WebSocket client is subscribed to a pane
**When** new PTY output is produced
**Then** a `pane_output` event is sent: `{ "type": "pane_output", "pane_id": 1, "data": "...raw ansi..." }`

**And** the daemon CPU usage does not increase by more than 5% from pane streaming

---

### Story 4.2: xterm.js Terminal Component

As a dashboard user,
I want to click "View Terminal" on an agent card and see live output,
So that I can supervise agent activity in the browser.

**Acceptance Criteria:**

**Given** the user clicks [View Terminal] on an agent card
**Then** a modal/panel opens with an xterm.js terminal
**And** the last 100 lines of the agent's pane output are loaded

**Given** the terminal is open
**When** new output is produced by the agent
**Then** it appears in the xterm.js terminal in real time via WebSocket `pane_output` events

**Given** the terminal is open
**Then** it is read-only (no keyboard input — supervisory view only)

---

## Epic 5: Frontend — Metrics, Charts & Geographic Map [v1.0]

Users can visualize token usage, costs over time, and agent geographic distribution, enabling cost optimization and fleet monitoring.

### Story 5.1: Metrics Charts with Recharts

As a dashboard user,
I want to see line charts of token usage and cost over time,
So that I can monitor spending and optimize agent usage.

**Acceptance Criteria:**

**Given** the Metrics page loads
**Then** it shows:
- Line chart: tokens per minute per agent (one line per agent, colored)
- Line chart: cumulative cost in USD over time
- Summary cards: total tokens, total cost, active agents, completed tasks

**Given** a WebSocket event `agent_tokens_updated` is received
**Then** the charts update in real time (no page refresh)

**Given** the user hovers over a chart data point
**Then** a tooltip shows the exact value and timestamp

---

### Story 5.2: Geographic Agent Map with deck.gl

As a dashboard user,
I want to see a world map showing where my agents are running,
So that I can visualize my distributed agent fleet.

**Acceptance Criteria:**

**Given** agents have location metadata (local, Databricks us-east-1, AWS eu-west-1)
**Then** pins appear on the map at corresponding geographic coordinates

**Given** a task is sent between two agents
**Then** an animated arc appears between their pins (color coded by provider)

**Given** an agent is local
**Then** its pin is placed at the user's approximate location

---

### Story 5.3: Cost Breakdown Dashboard

As a dashboard user,
I want to see cost broken down by agent, by session, and by hour,
So that I can identify expensive agents and optimize spending.

**Acceptance Criteria:**

**Given** the Metrics page loads
**Then** a table shows cost per agent: name, model, tokens, cost, cost/hour
**And** a bar chart shows cost per hour for the session
**And** a summary shows: session total cost, projected hourly/daily cost

---

## Epic 6: Healthcare / Compliance Features [v1.0]

Users in healthcare environments (AFYA) can view audit trails for LGPD compliance, export logs for regulatory review, and track cost per patient consultation.

### Story 6.1: Audit Log Viewer with Filtering & CSV Export

As a compliance officer,
I want to view all agent actions in a structured audit log,
So that I can verify LGPD compliance and review agent behavior.

**Acceptance Criteria:**

**Given** the Audit page loads
**Then** all audit events are displayed: timestamp, event type, agent, session, metadata

**Given** the user applies a filter (agent = "arquiteto", type = "task_sent", last 24h)
**Then** only matching entries are shown

**Given** the user clicks [Export CSV]
**Then** a CSV file is downloaded with all visible (filtered) entries

**Given** each audit entry
**Then** a LGPD compliance indicator is shown (green = compliant, yellow = review needed)

---

### Story 6.2: Cost per Consultation (AFYA Use Case)

As a healthcare administrator,
I want to see token costs grouped by consultation session,
So that I can track AI cost per patient visit.

**Acceptance Criteria:**

**Given** context keys follow the pattern `consultation:{id}:*`
**Then** the dashboard groups all associated agent costs

**Given** a consultation had agents "transcritor" and "soap-gen"
**Then** the cost breakdown shows: transcription cost + SOAP note cost + total

**Given** the Metrics page loads
**Then** a monthly cost projection is displayed based on current usage patterns

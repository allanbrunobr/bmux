# Features

<!-- Generated from BMAD artifacts by /skill:hive-md-from-bmad -->
<!-- Source: _bmad-output/planning-artifacts/web-dashboard-epics.md -->
<!-- Date: 2026-03-29 -->

## Feature 1: Axum Server Scaffold & Sessions Endpoint
- Description: [Epic 1: Backend HTTP API, Story 1.1] As a developer, I want to start an Axum HTTP server inside the BMUX daemon on port 7432, so that external HTTP clients can query session state. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: none
- Status: done

## Feature 2: Agents Endpoint
- Description: [Epic 1: Backend HTTP API, Story 1.2] As a dashboard user, I want to fetch all agents in a session via HTTP, so that the frontend can render agent cards. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 1
- Status: done

## Feature 3: Context & Tasks Endpoints
- Description: [Epic 1: Backend HTTP API, Story 1.3] As a dashboard user, I want to fetch shared context and task queue via HTTP, so that the frontend can display the context feed and task table. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 2
- Status: done

## Feature 4: Static File Serving & bmux web CLI Command
- Description: [Epic 1: Backend HTTP API, Story 1.4] As a user, I want to run bmux web and have the dashboard open in my browser, so that I can monitor agents without configuring anything. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 3
- Status: done

## Feature 5: POST Endpoint — Send Task from Browser
- Description: [Epic 1: Backend HTTP API, Story 1.5] As a dashboard user, I want to send a task to an agent from the browser UI via POST /api/tasks. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 4
- Status: done

## Feature 6: WebSocket Endpoint & Event Broadcaster
- Description: [Epic 2: WebSocket Events, Story 2.1] As a dashboard user, I want to receive real-time updates via WebSocket, so that the dashboard reflects terminal state instantly. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 5
- Status: done

## Feature 7: BmuxEvent Enum & Emission Points
- Description: [Epic 2: WebSocket Events, Story 2.2] As a developer, I want every state mutation in the daemon to emit a typed event, so that WebSocket clients receive all state changes. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 6
- Status: done

## Feature 8: Client Reconnection & Multi-Client Support
- Description: [Epic 2: WebSocket Events, Story 2.3] As a dashboard user, I want multiple browser tabs to receive updates simultaneously and auto-reconnect on disconnect. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 7
- Status: done

## Feature 9: Next.js Project Scaffold & Layout
- Description: [Epic 3: Core Dashboard, Story 3.1] As a developer, I want to create the Next.js project with the dashboard layout (sidebar, header, session selector, content area). Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 8
- Status: pending

## Feature 10: Session Selector & WebSocket Connection
- Description: [Epic 3: Core Dashboard, Story 3.2] As a dashboard user, I want to select a session from a dropdown and connect via WebSocket, so that the dashboard shows live data for my session. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 9
- Status: pending

## Feature 11: Agent Cards Grid
- Description: [Epic 3: Core Dashboard, Story 3.3] As a dashboard user, I want to see all agents as responsive cards showing name, type icon, model, status badge (pulsing when working), tokens, cost, uptime, and last task preview. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 10
- Status: pending

## Feature 12: Shared Context Live Feed
- Description: [Epic 3: Core Dashboard, Story 3.4] As a dashboard user, I want to see a live feed of shared context entries with key, value, timestamp, and relative time, updating in real time via WebSocket. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 11
- Status: pending

## Feature 13: Task Queue Table
- Description: [Epic 3: Core Dashboard, Story 3.5] As a dashboard user, I want to see the task queue with status badges (queued/active/completed/failed/timed_out), from→to, content preview, and cost. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 12
- Status: pending

## Feature 14: Send Task Modal
- Description: [Epic 3: Core Dashboard, Story 3.6] As a dashboard user, I want to click Send Task on an agent card, type a prompt in a modal, and have it delivered to the agent via POST /api/tasks. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 13
- Status: pending

## Feature 15: Pane Output Streaming Endpoint
- Description: [Epic 4: Terminal Embed, Story 4.1] As a developer, I want a backend endpoint GET /api/pane-output that returns last N lines of raw ANSI output and streams live via WebSocket pane_output events. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 8
- Status: pending

## Feature 16: xterm.js Terminal Component
- Description: [Epic 4: Terminal Embed, Story 4.2] As a dashboard user, I want to click View Terminal on an agent card and see live read-only terminal output via xterm.js. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 15
- Status: pending

## Feature 17: Metrics Charts with Recharts
- Description: [Epic 5: Metrics & Maps, Story 5.1] As a dashboard user, I want line charts showing tokens per minute per agent and cumulative cost over time, updating in real time. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 14
- Status: pending

## Feature 18: Geographic Agent Map with deck.gl
- Description: [Epic 5: Metrics & Maps, Story 5.2] As a dashboard user, I want a world map showing agent pins at geographic locations with animated arcs when tasks are sent between agents. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 17
- Status: pending

## Feature 19: Cost Breakdown Dashboard
- Description: [Epic 5: Metrics & Maps, Story 5.3] As a dashboard user, I want cost per agent table, cost per hour bar chart, and projected daily/monthly cost. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 18
- Status: pending

## Feature 20: Audit Log Viewer with Filtering & CSV Export
- Description: [Epic 6: Healthcare/Compliance, Story 6.1] As a compliance officer, I want a filterable audit log with LGPD indicators and CSV export for regulatory review. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 14
- Status: pending

## Feature 21: Cost per Consultation (AFYA Use Case)
- Description: [Epic 6: Healthcare/Compliance, Story 6.2] As a healthcare administrator, I want token costs grouped by consultation session with transcription + SOAP note cost breakdown. Source: _bmad-output/planning-artifacts/web-dashboard-epics.md
- Dependencies: Feature 20
- Status: pending

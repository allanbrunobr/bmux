# ADR-005: TUI Zoom, Scroll, and Config Reload

**Status:** Accepted  
**Date:** 2026-06-12

## Context

Stories 1.6–1.8 defined pane zoom (`Ctrl-b z`), scroll/copy mode (`Ctrl-b [`), and config hot reload (`Ctrl-b r`). The daemon path needed these wired through the session protocol so remote TUI clients reflect state consistently.

## Decision

1. **Zoom** — `ZoomState` lives on `Window`; `Session::toggle_zoom` delegates; snapshots expose `zoomed: bool` and render only the zoomed pane.
2. **Scroll** — `ScrollState` on `Pane` (line-buffered history); `enter_scroll_mode` / `handle_scroll_input` consume `k`/`j`/`q`/ESC before PTY; snapshots expose `scroll_active`.
3. **Reload** — `BmuxConfig::hot_reload` updates daemon `StdMutex<BmuxConfig>`; `Session::status_flash` carries transient messages; snapshots expose `status_message`.
4. **Status bar** — `StatusBar` widget (local mode) and `client::build_status_bar` (daemon mode) show `[ZOOM]`, scroll indicator, and config messages.

## Consequences

- Positive: feature parity between local and daemon TUI; CLI context queries enforce namespacing at the daemon boundary.
- Negative: scroll history is per-pane in-memory only (not persisted across detach).
- Follow-up: HMAC on daemon socket (ADR-001 scope); E2E PTY scroll under load.

## Compliance

- software-architecture: TUI state owned by daemon `Session`, broadcast via snapshots
- genai-integration-patterns: reload scrubs via `SecurityEnvelope` on context writes
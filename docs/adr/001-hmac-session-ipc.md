# ADR-001: HMAC-Signed Session IPC Bus

**Status:** Accepted  
**Date:** 2026-06-11

## Context

Agents need a trusted channel to publish task results and status. A separate Unix socket (`/tmp/bmux-{session}-ipc.sock`) carries JSON messages signed with HMAC-SHA256 (ring). The TUI daemon socket is a different channel for CLI/TUI queries.

## Decision

1. `MessageBus` generates a per-session 32-byte HMAC key at creation.
2. `MessageBus::start()` binds the IPC socket with mode `0600`.
3. Inbound messages are verified before broadcast; invalid HMAC is dropped.
4. Cryptographic primitives live in `src/security/hmac.rs`; `message_bus` delegates signing/verification there (ADR-001 compliance comment in code).

## Consequences

- Positive: tamper-evident agent ↔ orchestrator messages on the IPC bus.
- Negative: agents must receive the session key to sign (future work: fd injection).
- Trade-off: TUI daemon socket uses a simpler protocol; task delivery to PTY uses sanitized single-line content via `SecurityEnvelope`.

## Compliance

- agent-security: separation of perception/action on the IPC bus
- genai-integration-patterns: authenticated cross-boundary actions
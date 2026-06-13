# ADR-006: HMAC on TUI Daemon Socket

**Status:** Accepted  
**Date:** 2026-06-12

## Context

ADR-001 covers HMAC on the agent IPC bus (`/tmp/bmux-{session}-ipc.sock`). The TUI daemon socket (`/tmp/bmux-{session}.sock`) carried unsigned JSON frames, relying only on filesystem permissions (0600).

## Decision

1. **`SessionAuth`** generates a per-session 32-byte key at daemon boot (`session_auth.rs`).
2. Key is stored as `auth_key_hex` in session metadata (`/tmp/bmux/{name}.json`, mode 0600).
3. All client frames are `SignedClientFrame { ..msg, hmac }`; server frames are `SignedServerFrame`.
4. CLI (`ipc_client::query`) and TUI client load the key from metadata and sign outbound messages; inbound frames are verified.
5. Invalid HMAC yields `ServerMessage::Error` and connection close.

## Consequences

- Positive: tamper-evident CLI/TUI ↔ daemon traffic on the same host.
- Negative: clients must read session metadata before connecting; unsigned legacy clients are rejected.
- Complements: `BMUX_IPC_HMAC_FD` fd injection passes the *IPC bus* key to agents separately.

## Compliance

- ADR-001: cryptographic primitives in `security::hmac`
- agent-security: authenticated cross-boundary actions
# ADR-000: Record Architecture Decisions

**Status:** Accepted  
**Date:** 2026-06-11

## Context

BMUX is a modular Rust monolith with security, orchestration, and TUI subsystems. Structural decisions were previously implicit in code and audits flagged missing ADRs and diagram-only architecture.

## Decision

All structural changes (module boundaries, IPC model, spawn path, security envelope) require an ADR under `docs/adr/` before merge.

Format: TITLE, STATUS, CONTEXT, DECISION, CONSEQUENCES, COMPLIANCE.

## Consequences

- Positive: decisions are searchable; agents and humans share the same "why".
- Negative: small overhead for cross-cutting changes.

## Compliance

- software-architecture spec: `05-decisions-and-adrs.md`
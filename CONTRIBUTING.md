# Contributing to BMUX

## Prerequisites

- Rust 1.78+ (see `rust-version` in `Cargo.toml`)
- `cargo fmt`, `cargo clippy`, `cargo test`

## Before you open a PR

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

CI runs the same checks on Ubuntu and macOS.

## Structural changes

Cross-cutting changes (IPC, spawn, security, orchestration) need an ADR in `docs/adr/`. Use ADR-000 as the template.

Consult the specs in `../.specs/` when touching:

- agent spawn, context store, task routing → `agent-security/`
- module boundaries → `software-architecture/`
- IPC and audit → `genai-integration-patterns/`
- task lifecycle and workflow → `agentic-design-patterns/`

## Code style

- Match existing module layout and error types (`anyhow` at boundaries, `thiserror` in libraries).
- Prefer wiring new subsystems through `SecurityEnvelope` and `AgentRuntime` in the daemon path.
- Do not add `--dangerously-skip-permissions` to defaults; use `[security] allow_dangerous_permissions`.

## Tests

- Unit tests live beside modules (`#[cfg(test)]`).
- Integration tests: `tests/` and `tests/integration/`.
- New daemon behavior should get an integration test when feasible.
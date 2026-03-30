# Repository Guidelines

## Project Structure & Module Organization
`bmux` is a Rust workspace root with the CLI, TUI, agent runtime, web server, and storage logic under [`src/`](/Users/bruno/Desktop/BMUX/bmux/src). Core areas are grouped by domain: `src/agents/`, `src/tui/`, `src/web/`, `src/workflow/`, `src/security/`, and `src/storage/`. Rust tests live in [`tests/`](/Users/bruno/Desktop/BMUX/bmux/tests), including integration coverage in [`tests/integration/`](/Users/bruno/Desktop/BMUX/bmux/tests/integration). The Next.js dashboard lives in [`bmux-web/`](/Users/bruno/Desktop/BMUX/bmux/bmux-web) with routes in `src/app/`, reusable UI in `src/components/`, and shared client code in `src/lib/`. Treat [`web-dist/`](/Users/bruno/Desktop/BMUX/bmux/web-dist) as generated output.

## Build, Test, and Development Commands
Use `cargo build` to compile the Rust binary and `cargo run -- new demo` to launch a local session. Run `cargo test` for the full Rust test suite. Before opening a PR, run `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings`.

For the web UI:

- `cd bmux-web && npm install` installs dependencies.
- `cd bmux-web && npm run dev` starts the Next.js dashboard.
- `cd bmux-web && npm run build` creates the production bundle.
- `cd bmux-web && npm run lint` runs the frontend linter.

## Coding Style & Naming Conventions
Follow standard Rust formatting: 4-space indentation, `snake_case` for functions/modules, `PascalCase` for types, and small domain-focused modules. Keep clap command names, config keys, and file names descriptive and consistent with existing patterns such as `task_router.rs` and `session_store.rs`.

Frontend code follows the current TypeScript style in `bmux-web`: no semicolons, single quotes, `PascalCase` component files, and camelCase helpers/hooks such as `useBmuxSocket`.

## Testing Guidelines
Add Rust tests beside the behavior they cover in [`tests/`](/Users/bruno/Desktop/BMUX/bmux/tests) or `tests/integration/` when validating end-to-end flows. Name files after the feature under test, for example `security_hmac_test.rs`. There is no dedicated frontend test suite yet; at minimum, run `npm run lint` and verify affected pages locally.

## Commit & Pull Request Guidelines
Recent history uses Conventional Commit prefixes such as `feat:` and `fix:`, with optional scoped merges like `merge(wt3): ...`. Keep subjects imperative and concise. PRs should describe the user-visible change, list validation steps, link related issues, and include screenshots or terminal captures when TUI or dashboard behavior changes.

## Security & Configuration Tips
Do not commit secrets or local config. Runtime settings belong in `~/.config/bmux/config.toml`; API keys belong in `~/.config/bmux/secrets.toml`. Changes touching sandboxing, HMAC, audit logs, or secret handling should include targeted tests.

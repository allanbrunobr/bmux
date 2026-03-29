# BMUX

**A terminal multiplexer with AI agent orchestration** — think tmux, but with Claude Code, OpenCode, and other AI agents as first-class citizens.

---

## Quickstart (3 steps)

```sh
# 1. Install
cargo install bmux

# 2. Start a session
bmux new my-project

# 3. Spawn your first agent
bmux agent spawn claude --name dev
```

That's it. The TUI launches, splits a pane for the agent, and the status bar starts tracking token cost in real time.

---

## TUI Screenshot

```
┌────────────────────────────────┬──────────────────────────────────┐
│  [pane 1] shell                │  [pane 2] dev (Claude Code)      │
│  ~/my-project $ _              │  > Analyzing codebase...         │
│                                │  ✓ Found 42 Rust files           │
│                                │  > Running tests...              │
│                                │                                  │
├────────────────────────────────┴──────────────────────────────────┤
│ ● dev (claude-3-7-sonnet) [idle]  $0.023  ● ci (shell) [running]  │
└────────────────────────────────────────────────────────────────────┘
```

---

## Keybindings

| Key | Action |
|-----|--------|
| `Ctrl-b \` | Split pane horizontally |
| `Ctrl-b -` | Split pane vertically |
| `Ctrl-b h/j/k/l` | Navigate panes (Vim-style) |
| `Ctrl-b H/J/K/L` | Resize pane |
| `Ctrl-b z` | Zoom pane (full-screen toggle) |
| `Ctrl-b [` | Enter scroll/copy mode |
| `Ctrl-b d` | Detach from session |
| `Ctrl-b c` | Create new window |
| `Ctrl-b n/p` | Next/previous window |
| `Ctrl-b ,` | Rename current window |
| `Ctrl-b t` | Send task to focused agent |
| `Ctrl-b T` | Send task (auto-route to best agent) |
| `Ctrl-b r` | Reload config |
| `Ctrl-b ?` | Show this help |

---

## Installation

### cargo install (recommended)

```sh
cargo install bmux
```

Requires Rust 1.78+. Pre-compiled binaries are available on the [Releases](https://github.com/brunobsc/bmux/releases) page for:
- `linux-x86_64`
- `linux-aarch64`
- `macos-x86_64` (Intel)
- `macos-aarch64` (Apple Silicon)

### Homebrew (macOS)

```sh
brew install bruno/tap/bmux
```

### From source

```sh
git clone https://github.com/brunobsc/bmux
cd bmux
cargo install --path .
```

---

## CLI Commands

```
bmux new [name]           Start a new named session
bmux attach [name]        Reattach to an existing session
bmux ls                   List all sessions
bmux kill [name]          Kill a session

bmux agent spawn <type> --name <name>   Spawn an AI agent (claude, opencode, pi, gemini, shell)
bmux agent ls                            List all agents
bmux agent status <name>                 Show agent detail
bmux agent kill <name>                  Terminate an agent

bmux task send <agent> "<prompt>"       Send a task to a specific agent
bmux task ls                             List pending tasks
bmux task cancel <id>                   Cancel a queued task

bmux context get <key>                  Read a context value
bmux context set <key> <value>          Write a context value
bmux context dump                       Export session context as JSON

bmux workflow run <file.yaml>           Execute a workflow YAML
bmux workflow status <id>               Show workflow progress

bmux config show                        Display effective configuration
bmux config reload                      Hot-reload config.toml

bmux data purge                         Delete all local BMUX data
```

---

## Configuration

BMUX reads `~/.config/bmux/config.toml` on startup.

```toml
[tui]
default_shell = "/bin/zsh"
prefix_key    = "ctrl-b"
status_bar    = true

[agents]
default_type  = "claude"

[security]
sandbox       = "landlock"   # landlock | namespace | seatbelt | none

[audit]
enabled       = true         # set false to disable (not recommended)
```

### Secrets

Store API keys in `~/.config/bmux/secrets.toml` (must be chmod 0600):

```toml
[keys]
anthropic = "sk-ant-..."
openai    = "sk-..."
```

Keys are injected into agents via file descriptor — **never via environment variables** — and are automatically redacted from all audit log entries.

---

## Documentation

- [Man page](docs/man/bmux.1) — `man bmux` after install
- [Architecture](docs/architecture.md)
- [Contributing](CONTRIBUTING.md)

---

## License

MIT OR Apache-2.0

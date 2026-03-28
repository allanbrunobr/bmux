# Features

<!-- Generated from BMAD artifacts by /skill:hive-md-from-bmad -->
<!-- Source: _bmad-output/planning-artifacts/epics.md -->
<!-- Date: 2026-03-28 -->

## Feature 1: Project Scaffold & Single-Pane TUI
- Description: [Epic 1: Core Terminal Multiplexer, Story 1.1] As a developer, I want to run `bmux` and see a functional terminal with a single pane running my default shell, so that I have a working foundation. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: none
- Status: pending

## Feature 2: Pane Splitting (Horizontal & Vertical)
- Description: [Epic 1: Core Terminal Multiplexer, Story 1.2] As a developer, I want to split the terminal into multiple panes horizontally and vertically, so that I can view multiple terminals side by side. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 1
- Status: pending

## Feature 3: Pane Navigation with Vim Keys
- Description: [Epic 1: Core Terminal Multiplexer, Story 1.3] As a developer, I want to navigate between panes using Vim-style keys (h/j/k/l) and resize with H/J/K/L, so that I can switch focus quickly. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 2
- Status: pending

## Feature 4: Window Management
- Description: [Epic 1: Core Terminal Multiplexer, Story 1.4] As a developer, I want to create, list, and switch between multiple windows, so that I can organize different workstreams in separate views. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 3
- Status: pending

## Feature 5: Session Management (Create, Detach, Reattach, Kill)
- Description: [Epic 1: Core Terminal Multiplexer, Story 1.5] As a developer, I want to create named sessions, detach from them, and reattach later, so that my terminal state persists. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 4
- Status: pending

## Feature 6: Pane Zoom (Full-Screen Toggle)
- Description: [Epic 1: Core Terminal Multiplexer, Story 1.6] As a developer, I want to zoom into a single pane to see it full-screen and toggle back, so that I can focus on one task. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 5
- Status: pending

## Feature 7: Scroll/Copy Mode
- Description: [Epic 1: Core Terminal Multiplexer, Story 1.7] As a developer, I want to scroll back through pane output history and copy text, so that I can review past command output. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 6
- Status: pending

## Feature 8: Configuration System & Hot Reload
- Description: [Epic 1: Core Terminal Multiplexer, Story 1.8] As a developer, I want to configure BMUX via a TOML file and reload config without restarting, so that I can customize behavior. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 7
- Status: pending

## Feature 9: AgentAdapter Trait & Shell Agent
- Description: [Epic 2: Agent Lifecycle & Monitoring, Story 2.1] As a developer, I want a common AgentAdapter trait and a basic Shell agent, so that the agent system has a working foundation. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 8
- Status: pending

## Feature 10: Spawn AI Agents by Type
- Description: [Epic 2: Agent Lifecycle & Monitoring, Story 2.2] As a developer, I want to spawn Claude Code, OpenCode, and Pi agents by specifying type and name, so that I can run AI agents in BMUX panes. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 9
- Status: pending

## Feature 11: Status Bar with Agent State & Cost
- Description: [Epic 2: Agent Lifecycle & Monitoring, Story 2.3] As a developer, I want a persistent status bar showing all agents' model, status, and token cost, so that I have real-time visibility. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 10
- Status: pending

## Feature 12: Agent List, Status & Kill
- Description: [Epic 2: Agent Lifecycle & Monitoring, Story 2.4] As a developer, I want to list all agents, see detailed status, and kill specific agents via CLI, so that I can manage my agent fleet. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 11
- Status: pending

## Feature 13: Custom Agent Support via Config
- Description: [Epic 2: Agent Lifecycle & Monitoring, Story 2.5] As a contributor, I want to spawn any CLI binary as a managed agent by defining it in config.toml, so that I can integrate custom agents. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 12
- Status: pending

## Feature 14: Session Detach Preserves Agents
- Description: [Epic 2: Agent Lifecycle & Monitoring, Story 2.6] As a developer, I want agents to keep running after I detach from a session, so that long-running tasks continue while I'm away. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 13
- Status: pending

## Feature 15: Message Bus (Unix Socket + HMAC)
- Description: [Epic 3: Task Routing & Communication, Story 3.1] As a developer, I want a secure message bus using Unix sockets with HMAC-SHA256 authentication for agent communication. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 14
- Status: pending

## Feature 16: Send Task to Specific Agent
- Description: [Epic 3: Task Routing & Communication, Story 3.2] As a developer, I want to send a task to a specific agent by name via CLI or keybinding, so that I can direct work to the right agent. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 15
- Status: pending

## Feature 17: Task Queue, List & Cancel
- Description: [Epic 3: Task Routing & Communication, Story 3.3] As a developer, I want to see pending tasks and cancel queued tasks, so that I can manage my work queue effectively. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 16
- Status: pending

## Feature 18: Auto-Route Tasks by Cost & Capability
- Description: [Epic 3: Task Routing & Communication, Story 3.4] As a tech lead, I want tasks auto-routed to the best agent by scoring (capability×0.5 + cost×0.3 + idle×0.2), so that work is cost-effective. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 17
- Status: pending

## Feature 19: Agent Publishes Task Results
- Description: [Epic 3: Task Routing & Communication, Story 3.5] As a developer, I want agents to publish task results (content, tokens, cost, duration) so other agents can access them. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 18
- Status: pending

## Feature 20: Context Store with CLI Read/Write
- Description: [Epic 4: Shared Context Store, Story 4.1] As a developer, I want to read and write key-value pairs to a shared context store (sled KV) via CLI. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 19
- Status: pending

## Feature 21: Cross-Agent Context Access via IPC
- Description: [Epic 4: Shared Context Store, Story 4.2] As an agent, I want to read other agents' outputs from the shared context via IPC, so that I can build on previous work. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 20
- Status: pending

## Feature 22: Context Session Persistence & Cleanup
- Description: [Epic 4: Shared Context Store, Story 4.3] As a developer, I want context to persist through detach/reattach and clean up when the session ends, with TTL support. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 21
- Status: pending

## Feature 23: Export Context as JSON
- Description: [Epic 4: Shared Context Store, Story 4.4] As a developer, I want to export the entire session context as a valid JSON file via `bmux context dump`. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 22
- Status: pending

## Feature 24: YAML Workflow Parser & Validator
- Description: [Epic 5: Workflow Orchestration, Story 5.1] As a developer, I want to write workflow YAML definitions and validate them with dry-run, catching errors before spending tokens. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 23
- Status: pending

## Feature 25: Workflow Execution with Step Sequencing
- Description: [Epic 5: Workflow Orchestration, Story 5.2] As a developer, I want to execute a workflow that runs steps in dependency order with parameter injection and automatic agent spawning. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 24
- Status: pending

## Feature 26: Parallel Step Execution
- Description: [Epic 5: Workflow Orchestration, Story 5.3] As a developer, I want workflow steps without dependencies to execute in parallel on different agents, so that independent work runs simultaneously. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 25
- Status: pending

## Feature 27: Workflow Status & Monitoring
- Description: [Epic 5: Workflow Orchestration, Story 5.4] As a developer, I want to see real-time workflow status showing each step's state, assigned agent, duration, and cost. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 26
- Status: pending

## Feature 28: Agent Filesystem Sandboxing
- Description: [Epic 6: Security & Sandboxing, Story 6.1] As a developer, I want agents sandboxed to the project directory using Landlock/Seatbelt, so that misbehaving agents cannot access sensitive files. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 14
- Status: pending

## Feature 29: Secrets Management
- Description: [Epic 6: Security & Sandboxing, Story 6.2] As a developer, I want API keys stored securely in secrets.toml (0600) and injected via sealed file descriptors, never in env vars or logs. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 28
- Status: pending

## Feature 30: Audit Logging
- Description: [Epic 6: Security & Sandboxing, Story 6.3] As a developer, I want all agent actions logged to structured JSONL audit files with 100% completeness (NFR-08). Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 29
- Status: pending

## Feature 31: Cargo Install & CI Build Pipeline
- Description: [Epic 7: Distribution & Developer Setup, Story 7.1] As a developer, I want to install BMUX with `cargo install bmux` on macOS and Linux (4 targets) with CI-built binaries. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 8
- Status: pending

## Feature 32: README Quickstart & Man Page
- Description: [Epic 7: Distribution & Developer Setup, Story 7.2] As a new user, I want clear README with 3-step quickstart, keybindings table, ASCII screenshot, and a man page. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 31
- Status: pending

## Feature 33: Homebrew Formula
- Description: [Epic 7: Distribution & Developer Setup, Story 7.3] [v1.0] As a macOS user, I want to install BMUX via `brew install bruno/tap/bmux` for standard package management. Source: _bmad-output/planning-artifacts/epics.md
- Dependencies: Feature 32
- Status: pending

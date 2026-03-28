---
stepsCompleted: ['step-01-validate-prerequisites', 'step-02-design-epics', 'step-03-create-stories']
inputDocuments:
  - '_bmad-output/planning-artifacts/BMUX_PRD.md'
  - '_bmad-output/planning-artifacts/architecture.md'
mvpScope: [1, 2, 3, 6, 7]
v1Scope: [4, 5]
---

# BMUX - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for BMUX, decomposing the requirements from the PRD and Architecture into implementable stories.

## Requirements Inventory

### Functional Requirements

- FR-TUI-01: Usuário pode dividir o terminal em panes horizontais e verticais
- FR-TUI-02: Usuário pode navegar entre panes usando teclas Vim (h/j/k/l)
- FR-TUI-03: Usuário pode criar, listar e alternar entre windows numeradas
- FR-TUI-04: Usuário pode criar, listar e reatachar sessions nomeadas
- FR-TUI-05: Usuário pode dar zoom em um pane ocupando a tela inteira
- FR-TUI-06: Usuário pode entrar em scroll/copy mode para revisar histórico
- FR-TUI-07: Usuário vê status bar permanente com estado de todos os agentes
- FR-TUI-08: Usuário pode recarregar a configuração sem reiniciar a session
- FR-AGT-01: Usuário pode spawnar um agente especificando tipo e nome
- FR-AGT-02: Usuário pode spawnar qualquer agente CLI como processo filho gerenciado
- FR-AGT-03: Usuário pode ver status detalhado de um agente específico
- FR-AGT-04: Usuário pode encerrar um agente sem afetar outros agentes ou a session
- FR-AGT-05: Usuário pode listar todos os agentes ativos com seus status
- FR-AGT-06: Sistema mantém agentes rodando após o usuário fazer detach da session
- FR-TASK-01: Usuário pode enviar uma tarefa a um agente específico pelo nome
- FR-TASK-02: Usuário pode enviar uma tarefa sem especificar agente e o sistema roteia automaticamente
- FR-TASK-03: Sistema seleciona agente idle com base em capacidade, custo e disponibilidade
- FR-TASK-04: Usuário pode ver a fila de tarefas pendentes
- FR-TASK-05: Usuário pode cancelar uma tarefa que ainda não foi processada
- FR-TASK-06: Agente pode publicar resultado de tarefa acessível por outros agentes
- FR-CTX-01: Usuário pode ler e escrever valores no shared context via CLI
- FR-CTX-02: Agente pode ler o output de outro agente via shared context
- FR-CTX-03: Contexto persiste durante toda a session e é limpo ao encerrar
- FR-CTX-04: Usuário pode exportar todo o contexto da session como JSON
- FR-WFL-01: Usuário pode executar um workflow YAML que orquestra múltiplos agentes
- FR-WFL-02: Usuário pode passar parâmetros de entrada para um workflow
- FR-WFL-03: Usuário pode ver o status de um workflow em execução
- FR-WFL-04: Sistema executa steps sem dependências em paralelo
- FR-SEC-01: Sistema restringe acesso de agentes ao filesystem do projeto
- FR-SEC-02: Sistema armazena API keys sem expô-las em logs ou variáveis de ambiente
- FR-SEC-03: Sistema registra log de auditoria de todas as ações dos agentes

### NonFunctional Requirements

- NFR-01: Latência de renderização TUI < 16ms (critical: < 33ms) — criterion benchmark, 4 panes 80x24
- NFR-02: Throughput do message bus > 1.000 msg/s (critical: > 100 msg/s) — stress benchmark, 4 agentes, 1KB msgs
- NFR-03: Latência IPC ponta-a-ponta < 1ms (critical: < 10ms) — timestamp p99, Unix socket local
- NFR-04: Tempo de startup < 200ms (critical: < 1s) — cold start, sem agentes, macOS e Linux
- NFR-05: RAM em idle com 4 agentes < 50MB (critical: < 128MB) — heaptrack/smaps
- NFR-06: Disponibilidade durante session > 99.9% — crash de 1 agente não derruba session
- NFR-07: Tempo de recuperação após crash de agente < 2s (critical: < 10s)
- NFR-08: Auditoria: 100% das ações registradas

### Additional Requirements

- AR-01: Greenfield Rust project with Cargo.toml, clap entrypoint, ratatui + crossterm + tokio runtime
- AR-02: Project structure with modules (tui/, orchestration/, agents/, storage/, workflow/, security/, config/)
- AR-03: IPC protocol: JSON over Unix socket with HMAC-SHA256 authentication
- AR-04: Storage via sled KV store for context/sessions/logs
- AR-05: Each agent as isolated child process via portable-pty
- AR-06: Testing strategy: Unit, Integration (tokio-test), E2E (assert_cmd + rexpect), Fuzzing (cargo-fuzz), Performance (criterion)
- AR-07: cargo install bmux on macOS + Linux (4 architectures)
- AR-08: Config: ~/.config/bmux/config.toml (TOML) + ~/.config/bmux/workflows/*.yaml (YAML)
- AR-09: Distribution: README quickstart, man page, Homebrew formula (v1.0)

### UX Design Requirements

N/A — CLI project, no UX design document.

### FR Coverage Map

- FR-TUI-01: Epic 1 — Story 1.2 (Pane splitting)
- FR-TUI-02: Epic 1 — Story 1.3 (Pane navigation)
- FR-TUI-03: Epic 1 — Story 1.4 (Window management)
- FR-TUI-04: Epic 1 — Story 1.5 (Session management)
- FR-TUI-05: Epic 1 — Story 1.6 (Pane zoom)
- FR-TUI-06: Epic 1 — Story 1.7 (Scroll/copy mode)
- FR-TUI-07: Epic 2 — Story 2.3 (Status bar)
- FR-TUI-08: Epic 1 — Story 1.8 (Config hot reload)
- FR-AGT-01: Epic 2 — Story 2.2 (Spawn agent by type)
- FR-AGT-02: Epic 2 — Story 2.5 (Custom agent support)
- FR-AGT-03: Epic 2 — Story 2.4 (Agent status CLI)
- FR-AGT-04: Epic 2 — Story 2.4 (Agent kill)
- FR-AGT-05: Epic 2 — Story 2.4 (Agent list)
- FR-AGT-06: Epic 2 — Story 2.6 (Detach preserves agents)
- FR-TASK-01: Epic 3 — Story 3.2 (Send task to specific agent)
- FR-TASK-02: Epic 3 — Story 3.4 (Auto-route tasks)
- FR-TASK-03: Epic 3 — Story 3.4 (Scoring algorithm)
- FR-TASK-04: Epic 3 — Story 3.3 (Task queue + list)
- FR-TASK-05: Epic 3 — Story 3.3 (Task cancel)
- FR-TASK-06: Epic 3 — Story 3.5 (Publish task results)
- FR-CTX-01: Epic 4 — Story 4.1 (Context CLI read/write)
- FR-CTX-02: Epic 4 — Story 4.2 (Cross-agent context access)
- FR-CTX-03: Epic 4 — Story 4.3 (Session persistence + cleanup)
- FR-CTX-04: Epic 4 — Story 4.4 (Context dump as JSON)
- FR-WFL-01: Epic 5 — Story 5.2 (Workflow execution)
- FR-WFL-02: Epic 5 — Story 5.2 (Parameter passing)
- FR-WFL-03: Epic 5 — Story 5.4 (Workflow status)
- FR-WFL-04: Epic 5 — Story 5.3 (Parallel step execution)
- FR-SEC-01: Epic 6 — Story 6.1 (Filesystem sandboxing)
- FR-SEC-02: Epic 6 — Story 6.2 (Secrets management)
- FR-SEC-03: Epic 6 — Story 6.3 (Audit logging)

## Epic List

### Epic 1: Core Terminal Multiplexer [MVP]
Users can use BMUX as a tmux-compatible terminal multiplexer with panes, windows, sessions, and familiar keybindings.
**FRs covered:** FR-TUI-01, FR-TUI-02, FR-TUI-03, FR-TUI-04, FR-TUI-05, FR-TUI-06, FR-TUI-08

### Epic 2: Agent Lifecycle & Monitoring [MVP]
Users can spawn AI agents in terminal panes, see real-time status/cost in the status bar, and manage agent lifecycle.
**FRs covered:** FR-AGT-01, FR-AGT-02, FR-AGT-03, FR-AGT-04, FR-AGT-05, FR-AGT-06, FR-TUI-07

### Epic 3: Task Routing & Communication [MVP]
Users can send tasks to specific agents or let the system auto-route to the best agent by cost/capability/availability.
**FRs covered:** FR-TASK-01, FR-TASK-02, FR-TASK-03, FR-TASK-04, FR-TASK-05, FR-TASK-06

### Epic 4: Shared Context Store [v1.0]
Agents can share discoveries, outputs, and data through a persistent context store accessible by all agents and the user via CLI.
**FRs covered:** FR-CTX-01, FR-CTX-02, FR-CTX-03, FR-CTX-04

### Epic 5: Workflow Orchestration — Minion Engine [v1.0]
Users can define multi-agent workflows as YAML DAGs and execute them with automatic step sequencing and parallel execution.
**FRs covered:** FR-WFL-01, FR-WFL-02, FR-WFL-03, FR-WFL-04

### Epic 6: Security & Sandboxing [MVP]
Agents are sandboxed to the project directory, IPC is authenticated, secrets are protected, and all actions are audited.
**FRs covered:** FR-SEC-01, FR-SEC-02, FR-SEC-03

### Epic 7: Distribution & Developer Setup [MVP]
Users can install BMUX via cargo install on macOS + Linux, get started in < 5 minutes with a README quickstart, and access a man page.
**FRs covered:** AR-07, AR-09

---

## Epic 1: Core Terminal Multiplexer [MVP]

Users can use BMUX as a tmux-compatible terminal multiplexer with panes, windows, sessions, and familiar keybindings. This is the foundation — a fully usable terminal multiplexer before any agent features are added.

### Story 1.1: Project Scaffold & Single-Pane TUI

As a developer,
I want to run `bmux` and see a functional terminal with a single pane running my default shell,
So that I have a working foundation to build the multiplexer on.

**Acceptance Criteria:**

**Given** the user has installed bmux via `cargo install`
**When** the user runs `bmux` with no arguments
**Then** a full-screen TUI launches with a single pane running `$SHELL` (or config default_shell)
**And** the terminal renders at < 33ms latency (NFR-01 critical threshold)
**And** startup completes in < 1s (NFR-04 critical threshold)
**And** the user can type commands and see output in the pane
**And** Ctrl-b is captured as the prefix key (does not pass through to shell)

**Technical Notes:**
- Initialize Cargo.toml with dependencies: ratatui 0.28, crossterm 0.27, tokio 1.x, clap 4.x, portable-pty 0.8, tracing, tracing-subscriber
- Project structure per AR-02: src/main.rs (clap entrypoint), src/tui/mod.rs, src/tui/pane.rs
- Each pane owns a PTY (portable-pty) connected to a child shell process
- TUI app loop: crossterm events → state update → ratatui render

---

### Story 1.2: Pane Splitting (Horizontal & Vertical)

As a developer,
I want to split the terminal into multiple panes horizontally and vertically,
So that I can view multiple terminals side by side.

**Acceptance Criteria:**

**Given** the user is in a bmux session with at least one pane
**When** the user presses Ctrl-b \
**Then** the current pane splits horizontally into two equal panes, each running a new shell instance
**And** focus moves to the new pane

**Given** the user is in a bmux session with at least one pane
**When** the user presses Ctrl-b -
**Then** the current pane splits vertically into two equal panes, each running a new shell instance
**And** focus moves to the new pane

**Given** the user has split into multiple panes
**When** the user views the TUI
**Then** each pane has a visible border separating it from adjacent panes
**And** the active pane border is visually distinct (highlighted)
**And** each pane operates independently (input goes only to the focused pane)

---

### Story 1.3: Pane Navigation with Vim Keys

As a developer,
I want to navigate between panes using Vim-style keys (h/j/k/l),
So that I can switch focus quickly without using the mouse.

**Acceptance Criteria:**

**Given** the user has 2+ panes in a layout
**When** the user presses Ctrl-b h
**Then** focus moves to the pane to the left (if one exists)

**Given** the user has 2+ panes in a layout
**When** the user presses Ctrl-b j
**Then** focus moves to the pane below (if one exists)

**Given** the user has 2+ panes in a layout
**When** the user presses Ctrl-b k
**Then** focus moves to the pane above (if one exists)

**Given** the user has 2+ panes in a layout
**When** the user presses Ctrl-b l
**Then** focus moves to the pane to the right (if one exists)

**Given** the user presses Ctrl-b H/J/K/L (uppercase)
**When** the key is received
**Then** the active pane resizes in the corresponding direction by a fixed increment
**And** adjacent panes adjust to fill remaining space

**Given** focus is on an edge pane and the user navigates toward the edge
**When** no pane exists in that direction
**Then** focus remains on the current pane (no wrap, no error)

---

### Story 1.4: Window Management

As a developer,
I want to create, list, and switch between multiple windows,
So that I can organize different workstreams in separate views.

**Acceptance Criteria:**

**Given** the user is in a bmux session
**When** the user presses Ctrl-b c
**Then** a new window is created with a single pane running the default shell
**And** focus switches to the new window
**And** the status bar shows the new window in the window list

**Given** the user has multiple windows
**When** the user presses Ctrl-b 1 (or 2-9)
**Then** the view switches to the corresponding window by index

**Given** the user has multiple windows
**When** the user presses Ctrl-b n
**Then** the view switches to the next window in order

**Given** the user has multiple windows
**When** the user presses Ctrl-b p
**Then** the view switches to the previous window in order

**Given** the user views the status bar
**When** multiple windows exist
**Then** all windows are listed with their index and name
**And** the active window is visually highlighted

---

### Story 1.5: Session Management (Create, Detach, Reattach, Kill)

As a developer,
I want to create named sessions, detach from them, and reattach later,
So that my terminal state persists even when I disconnect.

**Acceptance Criteria:**

**Given** the user runs `bmux new my-project`
**When** the command executes
**Then** a new session named "my-project" is created with one window and one pane
**And** the user is attached to the session

**Given** the user is attached to a session
**When** the user presses Ctrl-b d
**Then** the user detaches from the session
**And** the session continues running in the background (all panes/shells alive)
**And** the user returns to their original terminal

**Given** a session named "my-project" is running in the background
**When** the user runs `bmux attach -s my-project`
**Then** the user reattaches to the session and sees all windows/panes as they were

**Given** one or more sessions are running
**When** the user runs `bmux ls`
**Then** a list of all active sessions is displayed with: name, window count, creation time

**Given** a session named "my-project" is running
**When** the user runs `bmux kill -s my-project`
**Then** all windows, panes, and child processes in the session are terminated
**And** the session is removed from `bmux ls`

---

### Story 1.6: Pane Zoom (Full-Screen Toggle)

As a developer,
I want to zoom into a single pane to see it full-screen,
So that I can focus on one task without distraction.

**Acceptance Criteria:**

**Given** the user has multiple panes in a window
**When** the user presses Ctrl-b z
**Then** the active pane expands to fill the entire window area
**And** other panes are hidden (not killed)

**Given** the user is in zoomed mode
**When** the user presses Ctrl-b z again
**Then** the layout returns to the original multi-pane arrangement
**And** all pane content is preserved

---

### Story 1.7: Scroll/Copy Mode

As a developer,
I want to scroll back through pane output history and copy text,
So that I can review past command output without losing my place.

**Acceptance Criteria:**

**Given** the user is in a pane with scrollback content
**When** the user presses Ctrl-b [
**Then** the pane enters scroll mode
**And** a visual indicator shows "[SCROLL]" or similar

**Given** the user is in scroll mode
**When** the user presses k/j (or arrow keys)
**Then** the view scrolls up/down through the scrollback buffer
**And** the buffer supports at least 10,000 lines (config: history_limit)

**Given** the user is in scroll mode
**When** the user presses q or Escape
**Then** scroll mode exits and the pane returns to live output

---

### Story 1.8: Configuration System & Hot Reload

As a developer,
I want to configure BMUX via a TOML file and reload config without restarting,
So that I can customize behavior without losing my session.

**Acceptance Criteria:**

**Given** `~/.config/bmux/config.toml` exists with valid configuration
**When** bmux starts
**Then** it loads and applies all settings from the config file (prefix key, history_limit, default_shell, mouse_support)

**Given** `~/.config/bmux/config.toml` does not exist
**When** bmux starts
**Then** it uses sensible defaults (prefix=Ctrl-b, history_limit=10000, default_shell=$SHELL)

**Given** the user is in an active session
**When** the user presses Ctrl-b r
**Then** config.toml is reloaded and applied without killing any panes or agents
**And** a confirmation message appears briefly in the status bar: "Config reloaded"

**Given** the config file contains invalid TOML
**When** the user attempts hot reload
**Then** the current config is retained
**And** an error message appears in the status bar: "Config error: {details}"

**Given** the user runs `bmux config show`
**When** the command executes
**Then** the current active configuration is displayed

**Given** the user runs `bmux config validate`
**When** the command executes
**Then** the config file is parsed and validated without applying changes
**And** any errors are reported

---

## Epic 2: Agent Lifecycle & Monitoring [MVP]

Users can spawn AI agents in terminal panes, see real-time status and cost in the status bar, and manage agent lifecycle (list, status, kill, detach). This epic transforms BMUX from a terminal multiplexer into an agent orchestration environment.

### Story 2.1: AgentAdapter Trait & Shell Agent

As a developer,
I want a common interface for all agents and a basic Shell agent to test with,
So that the agent system has a working foundation before integrating real AI agents.

**Acceptance Criteria:**

**Given** the agent module exists
**When** a new agent type is implemented
**Then** it implements the AgentAdapter trait (id, agent_type, model_name, cost_per_1k_tokens, spawn, send_task, recv_output, status, kill)

**Given** the ShellAdapter is available
**When** the user spawns a shell agent: `bmux agent spawn --type shell --name test`
**Then** a new pane opens with a shell process managed as an agent
**And** the agent appears in `bmux agent list` with status "idle"
**And** the agent responds to `bmux agent status test` with uptime and basic info

**Technical Notes:**
- ShellAdapter wraps a PTY child process (same as regular pane but tracked as agent)
- This validates the full agent lifecycle before AI-specific adapters are built

---

### Story 2.2: Spawn AI Agents by Type

As a developer,
I want to spawn Claude Code, OpenCode, and Pi agents by specifying their type and name,
So that I can run AI agents as managed processes in BMUX panes.

**Acceptance Criteria:**

**Given** the user runs `bmux agent spawn --type claude-code --name arch`
**When** the command executes
**Then** BMUX launches the `claude` binary (from config agents.claude-code.binary) as a child process
**And** a new pane opens showing the agent's terminal output
**And** the pane header shows 🤖 [arch] with the agent type indicator
**And** the agent registers with the system as status "idle"

**Given** the user runs `bmux agent spawn --type opencode --model gemini-2.5-pro --name tests`
**When** the command executes
**Then** BMUX launches the `opencode` binary with the specified model
**And** a new pane opens for the agent

**Given** the user runs `bmux agent spawn --type pi --model claude-haiku-4-5 --name docs`
**When** the command executes
**Then** BMUX launches the `pi` binary with the specified model
**And** a new pane opens for the agent

**Given** the specified agent binary is not found in PATH
**When** the user attempts to spawn
**Then** an error message is displayed: "Agent binary '{binary}' not found in PATH"
**And** no pane is created

---

### Story 2.3: Status Bar with Agent State & Cost

As a developer,
I want to see a persistent status bar showing all agents' model, status, and token cost,
So that I have real-time visibility into my agent fleet without switching panes.

**Acceptance Criteria:**

**Given** one or more agents are spawned
**When** the user views the status bar
**Then** it displays for each agent: type icon (🤖/🔧/🥧/✨/🔌), name, status indicator (● working / ○ idle), model name, cumulative cost
**And** the bar also shows aggregate: total agent count, active count, total tokens, total cost, session duration

**Given** an agent transitions from idle to working
**When** the status changes
**Then** the status bar updates within 1 render cycle (< 33ms)
**And** the indicator changes from ○ to ●

**Given** an agent completes a task and reports token usage
**When** the result is received
**Then** the cost display updates in the status bar for that agent
**And** the aggregate totals update

**Given** the user presses Ctrl-b $
**When** the cost dashboard opens
**Then** it shows a detailed breakdown: per-agent task count, tokens used, cost, and total session cost

---

### Story 2.4: Agent List, Status & Kill

As a developer,
I want to list all agents, see detailed status, and kill specific agents,
So that I can manage my agent fleet from the CLI.

**Acceptance Criteria:**

**Given** multiple agents are spawned
**When** the user runs `bmux agent list`
**Then** a table is displayed with columns: name, type, model, status, cost
**And** all active agents are listed

**Given** an agent named "arch" is running
**When** the user runs `bmux agent status arch`
**Then** detailed info is displayed: uptime, tokens used, cost, last task, current status

**Given** an agent named "tests" is running alongside other agents
**When** the user runs `bmux agent kill tests`
**Then** the "tests" agent process is terminated
**And** its pane is removed from the layout
**And** remaining agents and panes are unaffected
**And** the status bar updates to reflect the removed agent

**Given** the user tries to kill a non-existent agent
**When** `bmux agent kill nonexistent` is run
**Then** an error message is displayed: "Agent 'nonexistent' not found"

---

### Story 2.5: Custom Agent Support via Config

As a contributor,
I want to spawn any CLI binary as a BMUX-managed agent by defining it in config.toml,
So that I can integrate proprietary or custom agents without modifying BMUX code.

**Acceptance Criteria:**

**Given** the user has defined a custom agent in config.toml:
```
[agents.meu-agente]
binary = "meu-agente-cli"
model = "llama-3.1-70b"
cost_per_1k_tokens = 0.0001
args = ["--mode", "interactive"]
```
**When** the user runs `bmux agent spawn --type custom --name meu-agente`
**Then** BMUX launches "meu-agente-cli" with args ["--mode", "interactive"] as a child process
**And** a pane opens with the 🔌 indicator and name "meu-agente"
**And** the agent is tracked in the status bar with the configured cost_per_1k_tokens

**Given** the custom agent binary writes to stdout
**When** output is produced
**Then** it renders in the agent's pane in real time

---

### Story 2.6: Session Detach Preserves Agents

As a developer,
I want agents to keep running after I detach from a session,
So that long-running agent tasks continue while I'm away.

**Acceptance Criteria:**

**Given** the user has a session with 3 agents running (one with an active task)
**When** the user presses Ctrl-b d to detach
**Then** the user returns to their original terminal
**And** all 3 agent processes continue running

**Given** the user has detached from a session with active agents
**When** the user runs `bmux attach -s <session>`
**Then** the user reattaches and sees all panes and agents as they were
**And** any agent output produced during detach is visible in the scrollback
**And** the status bar shows current agent states (may have changed during detach)

---

## Epic 3: Task Routing & Communication [MVP]

Users can send tasks to specific agents or let the system auto-route to the best agent by cost, capability, and availability. This epic builds the communication layer that makes agents collaborative rather than isolated.

### Story 3.1: Message Bus (Unix Socket + HMAC)

As a developer,
I want a secure message bus for agent communication,
So that messages between agents and the system are reliable and authenticated.

**Acceptance Criteria:**

**Given** a bmux session starts
**When** the session initializes
**Then** a Unix socket is created at `/tmp/bmux-{session-id}.sock` with permissions 0600
**And** a session HMAC key is generated for message signing

**Given** the message bus is running
**When** a message is sent (task, result, status, context_update)
**Then** the message follows the JSON protocol: id (uuid), from, to, type, payload, timestamp, hmac
**And** the HMAC-SHA256 signature is verified on receipt
**And** messages with invalid HMAC are rejected and logged

**Given** an agent connects to the socket
**When** the agent sends a status heartbeat
**Then** the message bus routes it to the orchestration layer
**And** the agent's status is updated in the system

**Technical Notes:**
- Use tokio::net::UnixListener for the socket server
- ring crate for HMAC-SHA256
- Each spawned agent receives the socket path and session key at spawn time

---

### Story 3.2: Send Task to Specific Agent

As a developer,
I want to send a task to a specific agent by name,
So that I can direct work to the right agent for the job.

**Acceptance Criteria:**

**Given** an agent named "arch" is spawned and idle
**When** the user runs `bmux task send arch "Refatore o módulo de autenticação"`
**Then** a task message is created with type "task" and delivered to the "arch" agent via IPC
**And** the agent's status changes to "working" in the status bar

**Given** the user is focused on an agent pane
**When** the user presses Ctrl-b t
**Then** a prompt appears for the task content
**And** the task is sent to the agent in the focused pane

**Given** the user presses Ctrl-b T (uppercase)
**When** the agent chooser appears
**Then** a list of all agents with their status is shown
**And** the user can select an agent and type a task prompt

**Given** the user sends a task to an agent that is already "working"
**When** the task is submitted
**Then** the task is queued and the user is informed: "Agent 'arch' is busy — task queued (position: 1)"

---

### Story 3.3: Task Queue, List & Cancel

As a developer,
I want to see pending tasks and cancel tasks that haven't started,
So that I can manage my work queue effectively.

**Acceptance Criteria:**

**Given** multiple tasks have been submitted (some queued, some active)
**When** the user runs `bmux task list`
**Then** a table is displayed with: id (short), destination agent, status (queued/active/completed), time in queue

**Given** a task with id "abc123" is queued (not yet started)
**When** the user runs `bmux task cancel abc123`
**Then** the task is removed from the queue
**And** confirmation: "Task abc123 cancelled"

**Given** a task with id "def456" is currently active (agent is working on it)
**When** the user runs `bmux task cancel def456`
**Then** an error is displayed: "Task def456 is already in progress — cannot cancel"

**Given** the user runs `bmux task status abc123`
**When** the task exists
**Then** detailed status is shown: destination, submitted time, status, position in queue (if queued)

---

### Story 3.4: Auto-Route Tasks by Cost & Capability

As a tech lead,
I want to send tasks without specifying an agent and have the system choose the best one,
So that tasks are automatically routed to the most cost-effective capable agent.

**Acceptance Criteria:**

**Given** 3 agents are spawned with different costs and at least 1 is idle
**When** the user runs `bmux task send --auto "Adicione docstrings em src/router.rs"`
**Then** the task router evaluates all idle agents using the scoring formula:
  score = (capability_score × 0.5) + (cost_score × 0.3) + (idle_score × 0.2)
**And** the task is assigned to the agent with the highest score
**And** the user sees: "[auto-routed → {agent_name} | estimated cost: ${cost}]"

**Given** all agents are busy (none idle)
**When** the user sends an auto-routed task
**Then** the task is queued with a configurable timeout
**And** the user sees: "All agents busy — task queued (timeout: {config.queue_timeout_seconds}s)"

**Given** the user specifies a preferred model: `bmux task send --model sonnet "complex refactor"`
**When** an agent with that model exists and is idle
**Then** the task is sent directly to that agent (bypasses scoring)

**Given** the queue timeout expires with no agent becoming available
**When** the timeout fires
**Then** the task is marked as "timed out"
**And** the user is notified

---

### Story 3.5: Agent Publishes Task Results

As a developer,
I want agents to publish their task results so other agents can access them,
So that agents can collaborate on multi-step workflows.

**Acceptance Criteria:**

**Given** an agent completes a task
**When** the result message is sent via IPC
**Then** the result includes: content, tokens_used, cost_usd, duration_ms
**And** the agent's status returns to "idle"
**And** the status bar updates with new token/cost totals

**Given** an agent publishes a result with task_id "abc123"
**When** the result is received by the message bus
**Then** it is stored and accessible via the task system
**And** `bmux task status abc123` shows: completed, output summary, tokens, cost, duration

**Given** an agent crashes during a task
**When** the process exits unexpectedly
**Then** the agent's status changes to "error" within < 2s (NFR-07)
**And** the task is marked as "failed"
**And** the session remains stable (NFR-06)

---

## Epic 4: Shared Context Store [v1.0]

Agents can share discoveries, outputs, and data through a persistent context store accessible by all agents and the user via CLI. This enables true multi-agent collaboration where agents build on each other's work.

### Story 4.1: Context Store with CLI Read/Write

As a developer,
I want to read and write key-value pairs to a shared context store via CLI,
So that I can manually share data between agents.

**Acceptance Criteria:**

**Given** a bmux session is active
**When** the user runs `bmux context set project:tech-stack "Rust + ratatui + tokio"`
**Then** the key-value pair is stored in the sled KV database
**And** `bmux context get project:tech-stack` returns "Rust + ratatui + tokio"

**Given** the user runs `bmux context list`
**When** context entries exist
**Then** all keys are listed with their value preview (truncated to 80 chars)

**Given** the user runs `bmux context get nonexistent:key`
**When** the key does not exist
**Then** an appropriate message is returned: "Key not found: nonexistent:key"

---

### Story 4.2: Cross-Agent Context Access via IPC

As an agent,
I want to read other agents' outputs from the shared context,
So that I can build on previous work without human intervention.

**Acceptance Criteria:**

**Given** agent "arch" has produced output that was stored as `agent:arch:last_output`
**When** agent "tests" sends a context_update read request via IPC
**Then** agent "tests" receives the value of `agent:arch:last_output`

**Given** an agent completes a task
**When** the result is processed
**Then** the agent's last output is automatically stored at `agent:{id}:last_output` with TTL of 1 hour

**Given** the user presses Ctrl-b m
**When** the shared context viewer opens
**Then** all context keys and values are displayed in a navigable view

---

### Story 4.3: Context Session Persistence & Cleanup

As a developer,
I want context to persist through detach/reattach and clean up when the session ends,
So that shared data survives disconnections but doesn't accumulate indefinitely.

**Acceptance Criteria:**

**Given** a session has context entries and the user detaches
**When** the user reattaches
**Then** all context entries are still available

**Given** a session is terminated with `bmux kill -s <session>`
**When** the session ends
**Then** all context entries for that session are deleted from the sled database

**Given** a context entry has a TTL configured
**When** the TTL expires
**Then** the entry is automatically removed by the background cleanup task (60s sweep interval)

---

### Story 4.4: Export Context as JSON

As a developer,
I want to export the entire session context as a JSON file,
So that I can save, share, or analyze the collaborative output.

**Acceptance Criteria:**

**Given** a session has context entries
**When** the user runs `bmux context dump`
**Then** a JSON object is output to stdout with all key-value pairs
**And** the output is valid JSON parseable by `jq`

**Given** the user runs `bmux context dump > context.json`
**When** the file is written
**Then** the file contains all context entries with keys, values, and metadata (TTL, timestamp)

---

## Epic 5: Workflow Orchestration — Minion Engine [v1.0]

Users can define multi-agent workflows as YAML DAGs and execute them with automatic step sequencing, parameter injection, and parallel execution of independent steps.

### Story 5.1: YAML Workflow Parser & Validator

As a developer,
I want to write workflow definitions in YAML and validate them before running,
So that I catch errors in my workflow definition without wasting agent tokens.

**Acceptance Criteria:**

**Given** a valid workflow YAML file exists at `./my-workflow.yaml`
**When** the user runs `bmux workflow run ./my-workflow.yaml --dry-run`
**Then** the workflow is parsed, validated, and a summary is displayed: agents needed, step count, dependency graph, estimated cost
**And** no agents are spawned or tasks sent

**Given** a workflow YAML references an agent type not configured
**When** the user runs dry-run
**Then** an error is displayed: "Agent type '{type}' not configured in config.toml"

**Given** the workflow YAML has a circular dependency (step A depends on B, B depends on A)
**When** the user runs dry-run
**Then** an error is displayed: "Circular dependency detected: A → B → A"

**Given** the user runs `bmux workflow list`
**When** workflows exist in the configured workflows_dir
**Then** available workflows are listed with: name, step count, agent types required

---

### Story 5.2: Workflow Execution with Step Sequencing

As a developer,
I want to execute a workflow that runs steps in dependency order with parameter injection,
So that agents collaborate automatically on complex multi-step tasks.

**Acceptance Criteria:**

**Given** a valid workflow YAML with sequential steps (design → implement → test)
**When** the user runs `bmux workflow run ./feature.yaml --input '{"feature":"login"}'`
**Then** the workflow engine spawns required agents (if not already running)
**And** step "design" executes first with `{{ input.feature }}` resolved to "login"
**And** step "implement" starts only after "design" completes
**And** step "test" starts only after "implement" completes
**And** each step's output is stored in shared context at `workflow:{step_id}:output`

**Given** a step fails (agent error or timeout)
**When** the failure is detected
**Then** the workflow halts
**And** the user is notified: "Workflow failed at step '{step_id}': {error}"
**And** completed step outputs remain in shared context

---

### Story 5.3: Parallel Step Execution

As a developer,
I want workflow steps without dependencies to execute in parallel,
So that independent work happens simultaneously for faster completion.

**Acceptance Criteria:**

**Given** a workflow where steps "test" and "document" both depend only on "implement"
**When** "implement" completes
**Then** "test" and "document" start simultaneously on different agents
**And** the workflow completes when both parallel steps finish

**Given** a workflow with 4 steps: A (no deps), B (depends A), C (depends A), D (depends B+C)
**When** the workflow runs
**Then** A runs first, then B and C run in parallel, then D runs after both B and C complete

---

### Story 5.4: Workflow Status & Monitoring

As a developer,
I want to see the status of a running workflow,
So that I can monitor progress and identify bottlenecks.

**Acceptance Criteria:**

**Given** a workflow is running
**When** the user runs `bmux workflow status {id}`
**Then** the status shows: workflow name, each step with status (pending/running/completed/failed), assigned agent, duration, cost

**Given** a workflow has completed
**When** the user runs `bmux workflow status {id}`
**Then** the final status shows: total duration, total cost, all step outcomes

---

## Epic 6: Security & Sandboxing [MVP]

Agents are sandboxed to the project directory, IPC is authenticated, secrets are protected, and all actions are audited. This epic hardens BMUX for production use before the first release.

### Story 6.1: Agent Filesystem Sandboxing

As a developer,
I want agents to be restricted to reading/writing only within the project directory,
So that a misbehaving or compromised agent cannot access sensitive files on my system.

**Acceptance Criteria:**

**Given** an agent is spawned with project_dir set to `/home/bruno/my-project`
**When** the agent attempts to read a file outside project_dir (e.g., `~/.ssh/id_rsa`)
**Then** the read is blocked by the sandbox (Landlock on Linux 5.13+, Seatbelt on macOS)
**And** an audit log entry is created: "Access denied: agent '{name}' attempted to read '{path}'"

**Given** an agent is spawned
**When** the agent writes to a file inside project_dir
**Then** the write succeeds normally

**Given** an agent is spawned
**When** the agent writes to `/tmp/bmux-*` (temporary IPC files)
**Then** the write succeeds (allowed by sandbox policy)

**Given** the system is Linux < 5.13 (no Landlock)
**When** an agent is spawned
**Then** namespace-based isolation is used as fallback
**And** a warning is logged: "Landlock unavailable — using namespace isolation (reduced protection)"

**Given** sandbox configuration is set to "none" in config.toml
**When** an agent is spawned
**Then** no sandbox is applied
**And** a warning is logged: "Agent sandboxing disabled — running without filesystem restrictions"

---

### Story 6.2: Secrets Management

As a developer,
I want my API keys stored securely and never exposed in logs or environment variables,
So that my credentials are protected from accidental leakage.

**Acceptance Criteria:**

**Given** API keys are stored in `~/.config/bmux/secrets.toml` with permissions 0600
**When** an agent is spawned
**Then** the API key is injected via a sealed file descriptor, not via environment variable
**And** `env` output inside the agent process does NOT contain any API key

**Given** an agent is running and producing output
**When** the output is logged to the audit log
**Then** no API key appears in the log entries

**Given** an agent is running and publishing to shared context
**When** context entries are stored
**Then** no API key appears in any context value

**Given** `secrets.toml` has permissions other than 0600
**When** bmux starts
**Then** a warning is displayed: "secrets.toml has insecure permissions ({actual}), expected 0600"
**And** bmux offers to fix permissions automatically

---

### Story 6.3: Audit Logging

As a developer,
I want all agent actions logged to a structured audit trail,
So that I can review what agents did and debug issues after the fact.

**Acceptance Criteria:**

**Given** the audit log is enabled (default: true)
**When** any of these events occur:
- Agent spawned
- Agent killed
- Task sent to agent
- Task completed by agent
- Agent status change
- Sandbox access denied
- HMAC verification failure
**Then** a structured JSON entry is appended to `~/.local/share/bmux/audit/{date}.jsonl`
**And** each entry contains: timestamp, event type, agent_id, session_id, and event-specific metadata

**Given** 50 tasks are sent in sequence
**When** the audit log is checked
**Then** exactly 50 "task_sent" entries exist (NFR-08: 100% audit completeness)

**Given** the user runs `bmux data purge`
**When** the command executes
**Then** all audit logs are deleted
**And** all shared context data is deleted
**And** all session data is deleted
**And** confirmation is shown: "All BMUX local data purged"

**Given** audit_log is set to false in config.toml
**When** bmux runs
**Then** no audit entries are written
**And** a one-time notice is shown: "Audit logging is disabled"

---

## Epic 7: Distribution & Developer Setup [MVP]

Users can install BMUX easily and get started in under 5 minutes. This epic ensures the product is shippable and accessible.

### Story 7.1: Cargo Install & CI Build Pipeline

As a developer,
I want to install BMUX with `cargo install bmux` on macOS and Linux,
So that I can start using it immediately without compiling from source.

**Acceptance Criteria:**

**Given** the user has Rust installed
**When** the user runs `cargo install bmux`
**Then** the binary compiles and installs successfully on:
- linux-x86_64
- linux-aarch64
- macos-x86_64 (Intel)
- macos-aarch64 (Apple Silicon)

**Given** a CI pipeline (GitHub Actions)
**When** a new tag is pushed
**Then** pre-compiled binaries are built for all 4 targets
**And** binaries are attached to the GitHub Release
**And** SHA256 checksums are published alongside binaries

**Given** the installed binary
**When** the user runs `bmux --version`
**Then** the version, commit hash, and build target are displayed

---

### Story 7.2: README Quickstart & Man Page

As a new user,
I want clear documentation to get started in under 5 minutes,
So that I can evaluate BMUX quickly without reading extensive docs.

**Acceptance Criteria:**

**Given** the README.md exists in the repository root
**When** a new user reads it
**Then** they find:
- A one-line description of what BMUX is
- A 3-step quickstart (install → run → spawn first agent)
- An ASCII screenshot of the TUI with agents running
- A keybindings reference table
- Links to detailed documentation

**Given** the user runs `man bmux`
**When** the man page is installed
**Then** it displays: synopsis, description, keybindings, CLI commands, configuration file location, and examples

**Given** the user runs `bmux --help`
**When** the help output renders
**Then** all subcommands are listed with descriptions (new, attach, ls, kill, agent, task, context, workflow, config)

---

### Story 7.3: Homebrew Formula [v1.0]

As a macOS user,
I want to install BMUX via Homebrew,
So that I can install and update it using my standard package manager.

**Acceptance Criteria:**

**Given** the Homebrew tap exists at `bruno/tap`
**When** the user runs `brew install bruno/tap/bmux`
**Then** the latest release binary is downloaded and installed
**And** the man page is installed to the correct Homebrew prefix

**Given** a new BMUX version is released
**When** the formula is updated
**Then** `brew upgrade bmux` installs the new version

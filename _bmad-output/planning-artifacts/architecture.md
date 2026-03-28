# BMUX — Architecture Document

> Status: Draft v0.1  
> Autor: Bruno Oliveira Silva · Recife, BR · 2026  
> Referência: prd.md (BMUX PRD v0.1)

---

## 1. Visão Geral da Arquitetura

O BMUX é construído em **Rust** com uma arquitetura em 4 camadas desacopladas: TUI, Orquestração, Agentes e Storage. A comunicação entre camadas é assíncrona via `tokio`. Agentes são processos filhos isolados que se comunicam com o core via Unix sockets IPC.

```
┌─────────────────────────────────────────────────────────┐
│                    CAMADA TUI                           │
│         ratatui + crossterm (panes/windows/sessions)    │
└────────────────────────┬────────────────────────────────┘
                         │ eventos / render commands
┌────────────────────────▼────────────────────────────────┐
│              CAMADA DE ORQUESTRAÇÃO                     │
│   MessageBus │ TaskRouter │ SharedContextStore          │
│   (tokio channels + Unix socket server)                 │
└────────────────────────┬────────────────────────────────┘
                         │ AgentAdapter trait (IPC JSON)
┌────────────────────────▼────────────────────────────────┐
│               CAMADA DE AGENTES                         │
│   ClaudeCodeAdapter │ OpenCodeAdapter │ PiAdapter       │
│   GeminiAdapter     │ ShellAdapter    │ CustomAdapter   │
│   (processos filhos isolados via PTY + Unix socket)     │
└────────────────────────┬────────────────────────────────┘
                         │ sled KV + serde_yaml
┌────────────────────────▼────────────────────────────────┐
│                  CAMADA STORAGE                         │
│   sled (context/sessions/logs) + YAML (workflows)       │
└─────────────────────────────────────────────────────────┘
```

---

## 2. Stack Técnica

### 2.1 Linguagem e Runtime

| Componente | Tecnologia | Versão | Justificativa |
|---|---|---|---|
| Linguagem | Rust | 1.78+ (stable) | Zero GC pauses, segurança de memória, ecossistema TUI maduro |
| Runtime async | tokio | 1.x | Padrão de facto para Rust async, suporte a Unix sockets |
| Build | Cargo | bundled | Gerenciamento de deps e build |

### 2.2 TUI Layer

| Componente | Crate | Versão | Uso |
|---|---|---|---|
| Framework TUI | `ratatui` | 0.28 | Renderização de widgets, panes, status bar |
| Backend terminal | `crossterm` | 0.27 | Input de teclado, controle de terminal cross-platform |
| PTY management | `portable-pty` | 0.8 | Criar e gerenciar pseudoterminais para agentes |

### 2.3 Orquestração

| Componente | Crate | Versão | Uso |
|---|---|---|---|
| IPC server | `tokio::net::UnixListener` | — | Socket Unix para comunicação com agentes |
| Serialização | `serde` + `serde_json` | 1.x | Protocolo de mensagens JSON |
| Autenticação | `ring` | 0.17 | HMAC-SHA256 para assinatura de mensagens IPC |
| UUID | `uuid` | 1.x | IDs únicos de mensagens e tarefas |

### 2.4 Storage

| Componente | Crate | Versão | Uso |
|---|---|---|---|
| KV store | `sled` | 0.34 | Shared context, sessions, logs (embedded, sem deps C) |
| YAML parser | `serde_yaml` | 0.9 | Leitura de workflows Minion Engine |
| Config | `toml` + `serde` | — | Arquivo `~/.config/bmux/config.toml` |

### 2.5 CLI e Logging

| Componente | Crate | Versão | Uso |
|---|---|---|---|
| CLI parser | `clap` | 4.x | Subcomandos, flags, help automático |
| Logging | `tracing` | 0.1 | Structured logging async-aware |
| Log output | `tracing-subscriber` | 0.3 | Formatação e filtro de logs |

### 2.6 Segurança / Sandboxing

| Componente | Crate / Mecanismo | Uso |
|---|---|---|
| Landlock (Linux 5.13+) | `landlock` crate | Restrição de acesso ao filesystem por agente |
| Namespaces Linux | syscalls via `nix` | Isolamento de processos (fallback) |
| Seatbelt (macOS) | `sandbox-exec` | Sandboxing em macOS 13+ |

---

## 3. Estrutura de Diretórios do Projeto

```
bmux/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs                  # entrypoint CLI (clap)
│   ├── tui/
│   │   ├── mod.rs               # TUI app loop
│   │   ├── pane.rs              # gerenciamento de panes
│   │   ├── window.rs            # gerenciamento de windows
│   │   ├── session.rs           # gerenciamento de sessions
│   │   ├── status_bar.rs        # status bar (agentes, tokens, custo)
│   │   └── keybindings.rs       # mapeamento de teclas
│   ├── orchestration/
│   │   ├── mod.rs
│   │   ├── message_bus.rs       # pub/sub via tokio channels + Unix socket
│   │   ├── task_router.rs       # algoritmo de roteamento de tarefas
│   │   └── context_store.rs     # shared context via sled
│   ├── agents/
│   │   ├── mod.rs
│   │   ├── adapter.rs           # AgentAdapter trait
│   │   ├── claude_code.rs       # ClaudeCodeAdapter
│   │   ├── opencode.rs          # OpenCodeAdapter
│   │   ├── pi.rs                # PiAdapter
│   │   ├── gemini.rs            # GeminiAdapter
│   │   ├── shell.rs             # ShellAdapter (fallback)
│   │   └── custom.rs            # CustomAdapter (plugin)
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── session_store.rs     # persistência de sessions
│   │   ├── context_store.rs     # shared context KV
│   │   └── audit_log.rs         # logs de auditoria
│   ├── workflow/
│   │   ├── mod.rs
│   │   ├── engine.rs            # Minion Engine runner
│   │   └── yaml_parser.rs       # parser de workflows YAML
│   ├── security/
│   │   ├── mod.rs
│   │   ├── sandbox.rs           # Landlock / namespaces / seatbelt
│   │   ├── hmac.rs              # autenticação IPC
│   │   └── secrets.rs           # gerenciamento de API keys
│   └── config/
│       ├── mod.rs
│       └── settings.rs          # config.toml deserializer
├── tests/
│   ├── integration/
│   └── e2e/
├── docs/
│   ├── prd.md                   # (symlink ou cópia)
│   └── architecture.md          # este documento
└── .config/
    └── bmux/
        └── config.toml.example
```

---

## 4. Protocolo de Comunicação IPC

### 4.1 Socket

Cada sessão BMUX cria um socket em `/tmp/bmux-{session-id}.sock` com permissão `0600`.

### 4.2 Formato de Mensagem

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "from": "claude-code-arch",
  "to": "opencode-tests",
  "type": "task",
  "payload": {
    "content": "Escreva testes unitários para src/task_router.rs",
    "priority": 1,
    "preferred_model": null,
    "max_cost_usd": 0.05
  },
  "timestamp": "2026-03-28T10:00:00Z",
  "hmac": "sha256-abc123..."
}
```

### 4.3 Tipos de Mensagem

| Tipo | Direção | Payload |
|---|---|---|
| `task` | any → agent | `content`, `priority`, `preferred_model`, `max_cost_usd` |
| `result` | agent → any | `content`, `tokens_used`, `cost_usd`, `duration_ms` |
| `status` | agent → bus | `state: idle\|working\|waiting_input\|error` |
| `context_update` | agent → bus | `key`, `value`, `ttl_seconds` |
| `heartbeat` | agent → bus | `agent_id`, `uptime_seconds` |

---

## 5. AgentAdapter Trait

```rust
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,
    Working,
    WaitingInput,
    Error(String),
}

pub struct AgentOutput {
    pub content: String,
    pub tokens_used: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
}

#[async_trait]
pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn agent_type(&self) -> &str;
    fn model_name(&self) -> &str;
    fn cost_per_1k_tokens(&self) -> f64;

    async fn spawn(&mut self, config: &AgentConfig) -> anyhow::Result<()>;
    async fn send_task(&self, task: &Task) -> anyhow::Result<()>;
    async fn recv_output(&mut self) -> anyhow::Result<AgentOutput>;
    async fn status(&self) -> AgentStatus;
    async fn kill(&mut self) -> anyhow::Result<()>;
}
```

---

## 6. Task Router — Algoritmo

```
fn route(task: Task, agents: Vec<&dyn AgentAdapter>) -> Option<&dyn AgentAdapter>:

1. Filtrar: agentes com status == Idle
2. Se task.preferred_model != None:
     → selecionar agente cujo model_name == preferred_model
3. Senão, calcular score para cada agente idle:
     capability_score = match_keywords(task.content, agent.capabilities) → 0.0..1.0
     cost_score       = 1.0 - normalize(agent.cost_per_1k_tokens)        → 0.0..1.0
     idle_score       = normalize(agent.idle_since_seconds)               → 0.0..1.0
     final_score      = (capability_score * 0.5)
                      + (cost_score       * 0.3)
                      + (idle_score       * 0.2)
4. Retornar agente com maior final_score
5. Se nenhum disponível → enfileirar task com timeout = config.queue_timeout_seconds
6. Fallback após timeout → ShellAgent para tarefas simples (bash, file ops)
```

---

## 7. Shared Context Store

```rust
// Chaves padronizadas no sled KV:
"session:{id}:summary"           → resumo da sessão atual
"agent:{id}:last_output"         → último output do agente (TTL: 1h)
"agent:{id}:status"              → status atual do agente
"task:{id}:result"               → resultado de tarefa específica (TTL: 24h)
"project:structure"              → estrutura do projeto detectada (TTL: session)
"workflow:{id}:state"            → estado do workflow Minion Engine em execução
```

TTLs são aplicados via background task do tokio que varre o sled a cada 60s.

---

## 8. Integração Minion Engine

O BMUX executa workflows YAML do Minion Engine como DAGs:

```yaml
# Exemplo: ~/.config/bmux/workflows/feature.yaml
name: full-feature
version: "1.0"
agents:
  architect: { type: claude-code, model: claude-sonnet-4-5 }
  tester:    { type: opencode,    model: gemini-2.5-pro }
  documenter:{ type: pi,          model: claude-haiku-4-5 }

steps:
  - id: design
    agent: architect
    prompt: "Projete: {{ input.feature }}"
    outputs: [design_doc]
    context_key: "workflow:design:output"

  - id: implement
    agent: architect
    depends_on: [design]
    prompt: "Implemente baseado em: {{ context.workflow:design:output }}"

  - id: test
    agent: tester
    depends_on: [implement]
    prompt: "Escreva testes para: {{ context.workflow:implement:output }}"

  - id: document
    agent: documenter
    depends_on: [implement]
    prompt: "Documente: {{ context.workflow:implement:output }}"
```

O engine resolve dependências, executa steps em paralelo quando possível e persiste o estado no `SharedContextStore`.

---

## 9. Modelo de Segurança

### 9.1 Sandboxing por Camada

| Camada | Mecanismo | Escopo |
|---|---|---|
| Processo filho | Landlock (Linux) / Seatbelt (macOS) | Filesystem restrito ao project_dir |
| Rede | Network namespace (Linux) | Sem acesso à rede por padrão |
| IPC | Unix socket 0600 + HMAC-SHA256 | Apenas processos do mesmo usuário |
| Secrets | File descriptor selado no spawn | API keys nunca em env vars |

### 9.2 Capabilities por Agente (padrão)

| Capability | Default | Configurável |
|---|---|---|
| Read/Write project_dir | ✅ | Sim |
| Execute bash em project_dir | ✅ | Sim |
| Network access | ✅ | Sim |
| Read fora do project_dir | ✗ | Não |
| Write fora do project_dir | ✗ | Não |

---

## 10. Requisitos Não-Funcionais

| Requisito | Alvo | Crítico |
|---|---|---|
| Latência de renderização TUI | < 16ms (60fps) | < 33ms |
| Throughput do message bus | > 1.000 msg/s | > 100 msg/s |
| Latência IPC ponta-a-ponta | < 1ms | < 10ms |
| Tempo de startup | < 200ms | < 1s |
| RAM em idle (4 agentes ativos) | < 50MB | < 128MB |
| Tempo de build (CI) | < 3min | < 10min |

---

## 11. Estratégia de Testes

| Nível | Ferramenta | O que testa |
|---|---|---|
| Unit | `#[test]` nativo Rust | Lógica de TaskRouter, MessageBus, parsers |
| Integration | `tokio-test` | Comunicação IPC, fluxo agent→bus→agent |
| E2E CLI | `assert_cmd` + `rexpect` | Comandos CLI completos com TTY simulado |
| Fuzzing | `cargo-fuzz` | Parser YAML, parser de mensagens IPC |
| Performance | `criterion` | Throughput do message bus, latência de render |

---

## 12. Decisões de Arquitetura (ADRs)

### ADR-001: Rust em vez de Go
- **Decisão:** Usar Rust
- **Motivo:** Zero GC pauses para TUI sem jank; segurança de memória em código PTY de baixo nível; reutilização do ecossistema ratatui/crossterm já maduro em Rust; compilação para binário único sem runtime
- **Trade-off:** Compile time mais lento (~60s vs ~5s Go); curva de aprendizado maior para contribuidores

### ADR-002: Unix sockets em vez de TCP para IPC
- **Decisão:** `/tmp/bmux-{session}.sock`
- **Motivo:** Zero overhead de TCP stack; autenticação gratuita via permissões de arquivo (0600); sem exposição de porta de rede
- **Trade-off:** Não funciona para agentes remotos (SSH) — resolvido na v2.0 com túnel SSH

### ADR-003: `sled` em vez de SQLite
- **Decisão:** `sled` KV store
- **Motivo:** API idiomática Rust async; sem dependência C (SQLite requer unsafe FFI); melhor para acesso KV com TTL; ACID por transação
- **Trade-off:** Sem suporte a queries SQL; menos ferramentas de inspeção

### ADR-004: Processos filhos em vez de threads para agentes
- **Decisão:** Cada agente é um processo filho isolado
- **Motivo:** Crash de um agente não derruba o BMUX; sandbox via Landlock/namespaces aplica por processo; alinhado com modelo do tmux
- **Trade-off:** Overhead de IPC vs chamada de função; mais lento para mensagens de alta frequência

### ADR-005: TOML para config, YAML para workflows
- **Decisão:** Dois formatos distintos
- **Motivo:** TOML é idiomático em Rust e melhor para config estática hierárquica; YAML é necessário para compatibilidade com Minion Engine existente do autor
- **Trade-off:** Dois parsers de configuração no projeto

### ADR-006: `ratatui` em vez de `cursive` ou TUI própria
- **Decisão:** `ratatui` 0.28
- **Motivo:** Projeto mais ativo (3k+ stars, releases mensais); modelo de render imediato (immediate mode) mais simples para panes dinâmicos; compatível com crossterm e termion
- **Trade-off:** API de baixo nível — mais código boilerplate que cursive

---

## 13. Plano de Release

| Milestone | Versão | Foco | Prazo estimado |
|---|---|---|---|
| MVP | 0.1.0 | TUI básica + 3 agentes + IPC | 3 meses |
| Beta | 0.5.0 | Task router + shared context + Minion Engine | 5 meses |
| v1.0 | 1.0.0 | Estabilização + Homebrew + docs completos | 6 meses |
| v2.0 | 2.0.0 | SSH remoto + plugin WASM + BMUX Cloud | 12 meses |

Versionamento: **SemVer** estrito. Breaking changes apenas em major versions.  
Binários pré-compilados: `linux-x86_64`, `linux-aarch64`, `macos-x86_64`, `macos-aarch64`.

---

*BMUX Architecture Document v0.1 · Bruno Oliveira Silva · Recife, BR · Março 2026*

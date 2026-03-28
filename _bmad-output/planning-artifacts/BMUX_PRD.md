# BMUX — Product Requirements Document

> Terminal multiplexer de nova geração para orquestração de agentes de IA  
> Autor: Bruno Oliveira Silva · Recife, BR · 2026  
> Status: Draft v0.1

---

## 🧭 Seção 1 — Visão, Objetivos e Métricas de Sucesso

### Vision Statement

> O BMUX é o terminal nativo da era dos agentes de IA — um multiplexer que trata humanos e agentes como cidadãos de primeira classe, permitindo que equipes híbridas (humano + múltiplos agentes) trabalhem no mesmo espaço, com visibilidade total, custo controlado e orquestração inteligente.

### O Problema

Desenvolvedores que usam agentes de IA hoje enfrentam três dores críticas:

**Dor 1 — Fragmentação.** Claude Code, OpenCode, Pi e Gemini CLI rodam em terminais separados, sem visibilidade unificada. Trocar de janela para supervisionar agentes quebra o fluxo de trabalho.

**Dor 2 — Sem comunicação entre agentes.** Não existe mecanismo nativo para que um agente delegue uma subtarefa a outro. O desenvolvedor vira o intermediário manual.

**Dor 3 — Custo invisível.** Nenhuma ferramenta atual mostra em tempo real o custo acumulado de tokens por agente, tornando impossível otimizar o uso de modelos caros vs baratos.

### Personas

**Persona A — Bruno, Senior Engineer com IA (primária)**
- 35 anos, Recife. 17+ anos de experiência, heavy user de Claude Code e ferramentas CLI.
- Roda 3–5 agentes em paralelo para projetos diferentes.
- Dor principal: perde contexto ao alternar entre terminais.
- Desejo: um painel unificado onde possa supervisionar tudo sem sair do terminal.

**Persona B — Tech Lead de startup (primária)**
- Quer delegar tarefas de baixo valor (testes, docs, refactors) a agentes baratos (OpenCode + Gemini) e usar Claude Code apenas para arquitetura.
- Dor: não consegue rotear tarefas por custo/capacidade automaticamente.

**Persona C — Contribuidor open-source (secundária)**
- Quer plugar seu próprio agente CLI no ecossistema.
- Dor: cada ferramenta tem sua própria interface de integração.

### Proposta de Valor Única

| Dimensão | tmux | Zellij | amux | **BMUX** |
|---|---|---|---|---|
| Suporte a agentes | Nenhum | Nenhum | Parcial (Claude Code) | Multi-agente nativo |
| Comunicação entre agentes | Nenhuma | Nenhuma | Via screen capture | Message bus real |
| Task routing | Manual | Manual | Manual | Automático por custo/modelo |
| Shared context | Nenhum | Nenhum | Nenhum | Store persistente |
| Linguagem | C | Rust | Go | **Rust** |
| Integração Minion Engine | Nenhuma | Nenhuma | Nenhuma | **Nativa** |

### OKRs

**6 meses — MVP**
- O1: Lançar o BMUX com suporte estável a 3 agentes
  - KR1: 500+ GitHub stars no mês do lançamento
  - KR2: 3 agentes integrados (Claude Code, OpenCode, Pi)
  - KR3: zero bugs críticos por 30 dias após release

**12 meses — v1.0**
- O2: Tornar-se a referência para workflows multi-agente no terminal
  - KR1: 5.000+ GitHub stars
  - KR2: 50+ contribuidores externos
  - KR3: Menção oficial em docs da Anthropic como ferramenta parceira

**24 meses — v2.0**
- O3: Base para monetização sustentável
  - KR1: 20.000+ GitHub stars
  - KR2: 500+ usuários pagantes (cloud sync / hosted sessions)
  - KR3: Parceria formal com Anthropic fechada

### Definition of Done — MVP

- [ ] TUI funcional com panes, windows e sessions (compatível com keybindings tmux)
- [ ] Suporte a Claude Code, OpenCode e Pi como agentes gerenciados
- [ ] Message bus via Unix socket com entrega garantida
- [ ] Shared context store básico (in-memory + flush para disco)
- [ ] Task router com roteamento manual e por custo estimado
- [ ] Status bar com modelo, status e tokens consumidos por agente
- [ ] `cargo install bmux` funcionando em macOS e Linux
- [ ] Documentação: README, quickstart e man page

### Hipóteses a Validar

1. Devs preferem um terminal unificado a abas separadas — validar via survey pós-lançamento
2. Task routing por custo reduz gastos em 30%+ — validar com telemetria opt-in
3. O Minion Engine como DSL de orquestração é suficiente para a maioria dos workflows — validar com primeiros 100 usuários

### Riscos Estratégicos

| Risco | Probabilidade | Impacto | Mitigação |
|---|---|---|---|
| Anthropic muda API do Claude Code | Média | Alto | Agent Adapter desacopla a integração |
| Outro projeto lança feature similar | Alta | Médio | Velocidade de execução + comunidade |
| Complexidade do Rust afasta contribuidores | Média | Médio | Boa documentação + boas primeiras issues |
| Custo de API maior que esperado para usuários | Baixa | Alto | Task router por custo + alertas configuráveis |

---

## User Journeys

### Journey 1 — Bruno: Orquestrar 3 agentes em paralelo (Persona A)

**Goal:** Implementar uma feature completa usando agentes especializados sem sair do terminal

**Preconditions:** BMUX instalado, projeto aberto, 3 API keys configuradas

```
1. Bruno abre o terminal e roda `bmux new feature-login`
   → BMUX cria nova session com 1 pane humano

2. Bruno spawna 3 agentes especializados:
   `bmux agent spawn --type claude-code --name arch`
   `bmux agent spawn --type opencode --model gemini-2.5-pro --name tests`
   `bmux agent spawn --type pi --model claude-haiku-4-5 --name docs`
   → TUI divide automaticamente em 4 panes (1 humano + 3 agentes)
   → Status bar mostra: agents: 3 | ativo: 0 | custo: $0.000

3. Bruno envia tarefa ao agente de arquitetura:
   Ctrl-b t (focado no pane arch)
   → prompt: "Projete o módulo de autenticação JWT para o bmux CLI"
   → pane arch muda para: ● working | claude-sonnet | $0.000

4. Bruno monitora o progresso sem interagir:
   → Vê o agente arch escrevendo a proposta em tempo real no pane
   → Status bar atualiza: tokens: 1.2k | custo: $0.004

5. Agente arch conclui e publica resultado no shared context:
   → pane arch volta para: ○ idle | claude-sonnet | $0.012
   → Bruno lê o output no pane

6. Bruno delega testes e docs em paralelo:
   `bmux task send tests "Escreva testes para o design do arch"`
   `bmux task send docs "Documente a API do módulo de auth"`
   → panes tests e docs mudam para ● working simultaneamente

7. Bruno acompanha os dois agentes trabalhando em paralelo nos panes
   → Status bar: agents: 3 | ativo: 2 | tokens: 4.8k | custo: $0.021

8. Ambos concluem. Bruno revisa os outputs nos panes
   → Faz Ctrl-b [ no pane tests para entrar em scroll mode e ler o código gerado

9. Bruno detacha a session: Ctrl-b d
   → Agentes continuam rodando em background
   → Bruno pode reatachar com `bmux attach -s feature-login`
```

**Success:** Feature implementada com 3 agentes colaborando, custo visível, Bruno nunca saiu do terminal.

---

### Journey 2 — Tech Lead: Rotear tarefas por custo (Persona B)

**Goal:** Reduzir custo de API usando o task router automático para direcionar tarefas ao agente mais barato

**Preconditions:** BMUX com task router habilitado (`auto_route = true`), 3 agentes configurados com custos diferentes

```
1. Tech Lead abre config e verifica custos configurados:
   → claude-code: $0.003/1k tokens (prioridade 1 — mais caro)
   → opencode+gemini: $0.0007/1k tokens (prioridade 2)
   → pi+haiku: $0.00025/1k tokens (prioridade 3 — mais barato)

2. Tech Lead envia tarefa simples sem especificar agente:
   `bmux task send --auto "Adicione docstrings em todos os métodos de src/router.rs"`
   → Task router avalia: tarefa simples (file ops + docstrings) → baixo capability_score para claude-code
   → Router seleciona pi+haiku (score mais alto: custo baixo + tarefa adequada)
   → Output: [auto-routed → pi-docs | estimated cost: $0.001]

3. Tech Lead envia tarefa complexa sem especificar agente:
   `bmux task send --auto "Refatore o TaskRouter para suportar weighted round-robin"`
   → Task router avalia: refactoring complexo → alto capability_score para claude-code
   → Router seleciona claude-code (score mais alto: capacidade > custo para esta tarefa)
   → Output: [auto-routed → claude-code-arch | estimated cost: $0.018]

4. Ao final da sessão, Tech Lead verifica o custo total:
   Ctrl-b $
   → Dashboard mostra:
     claude-code-arch: 12 tarefas | $0.087
     opencode-tests:    8 tarefas | $0.009
     pi-docs:          15 tarefas | $0.004
     TOTAL SESSION:    35 tarefas | $0.100
     SAVED vs all-claude: ~$0.312 (75% economia)
```

**Success:** Tech Lead reduziu custo em ~75% sem configurar manualmente cada tarefa.

---

### Journey 3 — Contribuidor: Plugar agente customizado (Persona C)

**Goal:** Integrar um agente CLI proprietário ao BMUX sem modificar o core

**Preconditions:** BMUX instalado, agente customizado disponível como binário CLI

```
1. Contribuidor cria configuração do agente customizado em config.toml:
   [agents.meu-agente]
   binary = "meu-agente-cli"
   model = "llama-3.1-70b"
   cost_per_1k_tokens = 0.0001
   args = ["--mode", "interactive"]

2. Contribuidor spawna o agente customizado:
   `bmux agent spawn --type custom --name meu-agente`
   → BMUX inicializa o binário como processo filho
   → Pane aparece na TUI com indicador 🔌 [meu-agente]
   → Status bar inclui o agente na contagem

3. Contribuidor envia uma tarefa para testar:
   `bmux task send meu-agente "Liste os arquivos no projeto"`
   → BMUX envia via IPC socket (JSON + HMAC)
   → Output aparece no pane em tempo real

4. Contribuidor verifica que o agente participa do shared context:
   `bmux context get "agent:meu-agente:last_output"`
   → Retorna o último output do agente — disponível para outros agentes

5. Contribuidor publica o adapter no GitHub como exemplo
   → Outros usuários podem reutilizar a config
```

**Success:** Agente customizado integrado em < 5 minutos, sem modificar código do BMUX.

---

## ⚙️ Seção 2 — Requisitos de Sistema e Performance

> Arquitetura técnica, stack, protocolos, ADRs e decisões de design estão documentados em [architecture.md](architecture.md). Esta seção define apenas os requisitos de sistema mensuráveis.

### Non-Functional Requirements

| ID | NFR | Target | Critical | Measurement Method | Conditions |
|---|---|---|---|---|---|
| NFR-01 | Latência de renderização TUI | < 16ms | < 33ms | `criterion` benchmark no render loop | 4 panes ativos, terminal 80x24 |
| NFR-02 | Throughput do message bus | > 1.000 msg/s | > 100 msg/s | Benchmark de stress com producer/consumer tokio tasks | 4 agentes simultâneos, mensagens de 1KB |
| NFR-03 | Latência IPC ponta-a-ponta | < 1ms | < 10ms | Timestamp no envio e recebimento, p99 | Máquina local, Unix socket, mensagem 1KB |
| NFR-04 | Tempo de startup | < 200ms | < 1s | `time bmux new test && bmux kill -s test` | Cold start, sem agentes, macOS e Linux |
| NFR-05 | RAM em idle com 4 agentes | < 50MB | < 128MB | `heaptrack` ou `/proc/PID/smaps` | 4 agentes spawned, nenhuma tarefa ativa |
| NFR-06 | Disponibilidade durante session | > 99.9% uptime | > 99% | Crash de 1 agente não derruba session | Teste com `kill -9` no processo filho |
| NFR-07 | Tempo de recuperação após crash de agente | < 2s | < 10s | Tempo entre kill do agente e pane atualizar status para "error" | Agente com tarefa ativa |
| NFR-08 | Auditoria: 100% das ações registradas | 100% | 100% | Comparar contagem de ações executadas vs entradas no audit log | 50 tasks enviadas em sequência |

### Sistemas Operacionais Suportados

| OS | Suporte | Sandboxing |
|---|---|---|
| Linux 5.13+ | Completo | Landlock + namespaces |
| macOS 13+ | Completo | Seatbelt (sandbox-exec) |
| Linux < 5.13 | Parcial | Namespaces apenas |
| Windows (WSL2) | Experimental | Via WSL2 |

---

## 🎨 Seção 3 — Developer Experience, Interface e Keybindings

### Princípios de Design

1. **Compatibilidade com tmux** — qualquer usuário de tmux deve se sentir em casa em < 5 minutos
2. **Agentes são cidadãos do terminal** — não são "features", são participantes do workflow
3. **Zero surpresas** — comportamento previsível e determinístico
4. **Informação contextual, não verborrágica** — mostrar o que importa, esconder o resto
5. **O humano sempre no controle** — agentes nunca executam ações irreversíveis sem confirmação

### Keybindings Completos

| Tecla | Ação | Contexto |
|---|---|---|
| `Ctrl-b \` | Split horizontal | Qualquer pane |
| `Ctrl-b -` | Split vertical | Qualquer pane |
| `Ctrl-b h/j/k/l` | Navegar entre panes | Qualquer pane |
| `Ctrl-b H/J/K/L` | Redimensionar pane | Qualquer pane |
| `Ctrl-b z` | Zoom no pane ativo | Qualquer pane |
| `Ctrl-b c` | Novo window | Session |
| `Ctrl-b n/p` | Próximo/anterior window | Session |
| `Ctrl-b 1-9` | Ir para window por número | Session |
| `Ctrl-b d` | Detach da session | Session |
| `Ctrl-b [` | Modo scroll/copy | Qualquer pane |
| `Ctrl-b a` | Abrir painel de agentes | Global |
| `Ctrl-b t` | Enviar tarefa ao agente focado | Pane de agente |
| `Ctrl-b T` | Enviar tarefa (escolher agente) | Global |
| `Ctrl-b s` | Painel de status de todos os agentes | Global |
| `Ctrl-b $` | Ver custo acumulado da session | Global |
| `Ctrl-b m` | Ver shared context | Global |
| `Ctrl-b w` | Executar workflow Minion Engine | Global |
| `Ctrl-b r` | Hot reload config | Global |
| `Ctrl-b ?` | Help inline | Global |

### CLI Commands Completos

```bash
# Sessões
bmux                              # iniciar ou reatachar última session
bmux new [nome]                   # nova session nomeada
bmux attach -s [nome]             # reatachar session específica
bmux ls                           # listar sessions ativas
bmux kill -s [nome]               # encerrar session

# Agentes
bmux agent spawn --type claude-code --name "arch"
bmux agent spawn --type opencode --model gemini-2.5-pro --name "tests"
bmux agent spawn --type pi --model claude-haiku-4-5 --name "docs"
bmux agent spawn --type custom --cmd "meu-agente" --name "custom"
bmux agent list                   # listar agentes e status
bmux agent status [nome]          # status detalhado + tokens/custo
bmux agent kill [nome]            # encerrar agente

# Tarefas
bmux task send [agente] "prompt"  # enviar tarefa a agente específico
bmux task send --auto "prompt"    # task router escolhe o melhor agente
bmux task send --model sonnet "prompt"  # forçar modelo específico
bmux task list                    # fila de tarefas pendentes
bmux task status [id]             # status de tarefa específica
bmux task cancel [id]             # cancelar tarefa

# Context compartilhado
bmux context get [chave]          # ler valor do shared context
bmux context set [chave] [valor]  # escrever no shared context
bmux context list                 # listar todas as chaves
bmux context dump                 # exportar context completo como JSON

# Workflows Minion Engine
bmux workflow run ./meu-workflow.yaml --input '{"feature":"login"}'
bmux workflow run ./meu-workflow.yaml --dry-run
bmux workflow list                # listar workflows disponíveis
bmux workflow status [id]         # status de workflow em execução

# Config
bmux config show                  # mostrar config atual
bmux config edit                  # abrir config no $EDITOR
bmux config validate              # validar config sem aplicar
```

### Wireframe ASCII dos Panes

```
┌─────────────────────────────────────────────────────────────────┐
│ bmux  [main]  w1:arch  w2:tests  w3:docs          Ctrl-b ?:help │
├─────────────────────────────┬───────────────────────────────────┤
│ 🧑 [você]                   │ 🤖 Claude Code [arch]             │
│                             │ ● working  claude-sonnet  $0.042  │
│ $ git diff src/router.rs    │ ─────────────────────────────────  │
│                             │ Analisando o task router...       │
│                             │ > Identificando 3 edge cases      │
│                             │ > Propondo solução com priority   │
│                             │                                   │
├─────────────────────────────┼───────────────────────────────────┤
│ 🤖 OpenCode [tests]         │ 🤖 Pi Agent [docs]                │
│ ○ idle  gemini-2.5  $0.001  │ ○ idle  claude-haiku  $0.000      │
│ ─────────────────────────── │ ─────────────────────────────────  │
│ Aguardando próxima tarefa   │ Aguardando próxima tarefa         │
│                             │                                   │
├─────────────────────────────┴───────────────────────────────────┤
│ agents: 3  ativo: 1  tokens: 12.4k  custo: $0.043  session: 23m │
└─────────────────────────────────────────────────────────────────┘
```

### Indicadores Visuais por Tipo de Agente

| Agente | Ícone | Cor da borda | Status indicator |
|---|---|---|---|
| Humano | 🧑 | Sem borda especial | — |
| Claude Code | 🤖 | Roxo | ● working / ○ idle |
| OpenCode | 🔧 | Verde | ● working / ○ idle |
| Pi Agent | 🥧 | Azul | ● working / ○ idle |
| Gemini CLI | ✨ | Amarelo | ● working / ○ idle |
| Custom Agent | 🔌 | Cinza | ● working / ○ idle |

### Schema Completo `~/.config/bmux/config.toml`

```toml
[general]
prefix = "Ctrl-b"              # prefixo de keybindings
history_limit = 10000          # linhas de scrollback por pane
default_shell = "/bin/zsh"     # shell padrão para panes humanos
mouse_support = true           # suporte a mouse

[agents.defaults]
timeout_seconds = 300          # timeout padrão por tarefa
max_cost_per_session = 5.00    # limite de custo em USD por sessão
auto_route = true              # usar task router automático
confirm_destructive = true     # confirmar ações irreversíveis

[agents.claude-code]
binary = "claude"
model = "claude-sonnet-4-5"
cost_per_1k_tokens = 0.003
priority = 1                   # maior prioridade (menor número)

[agents.opencode]
binary = "opencode"
model = "gemini-2.5-pro"
cost_per_1k_tokens = 0.0007
priority = 2

[agents.pi]
binary = "pi"
model = "claude-haiku-4-5"
cost_per_1k_tokens = 0.00025
priority = 3

[ui]
theme = "catppuccin-mocha"     # tema de cores
status_bar_position = "bottom" # "top" ou "bottom"
show_token_count = true
show_cost = true
show_model = true
show_agent_type = true

[security]
sandbox = "landlock"           # "landlock", "namespace", "none"
allow_network = false
allow_filesystem = "project_dir_only"
audit_log = true
audit_log_path = "~/.local/share/bmux/audit/"

[minion_engine]
workflows_dir = "~/.config/bmux/workflows"
docker_enabled = true
```

---

## 🔒 Seção 4 — Requisitos de Segurança e Privacidade

> Threat model, superfícies de ataque, implementação de sandboxing, código de autenticação IPC e detalhes técnicos de segurança estão documentados em [architecture.md](architecture.md). Esta seção define os requisitos de segurança e privacidade que o sistema deve atender.

### Security Requirements

- **SR-01:** Agentes devem ser sandboxed ao diretório do projeto — nenhum acesso de leitura ou escrita fora do `project_dir` e `/tmp/bmux-*`
- **SR-02:** Toda mensagem IPC deve ser autenticada via HMAC-SHA256 com chave de sessão
- **SR-03:** Unix socket do message bus deve usar permissões `0600` (apenas o dono)
- **SR-04:** API keys nunca devem aparecer em logs, shared context, message bus ou core dumps
- **SR-05:** Output de agente encaminhado a outro agente deve passar por sanitização contra prompt injection
- **SR-06:** Workflows YAML devem ser validados antes da execução para prevenir comandos arbitrários
- **SR-07:** `cargo audit` deve passar sem vulnerabilidades críticas antes de cada release
- **SR-08:** Fuzzing do parser YAML e do parser de mensagens IPC (mínimo 1h cada) antes de release

### Privacy Requirements

- **PR-01:** Por padrão, apenas metadados são logados (timestamp, agente, tokens, custo) — conteúdo de prompts é opt-in explícito
- **PR-02:** BMUX nunca envia dados a servidores externos
- **PR-03:** Usuários podem exportar e deletar todos os dados locais com `bmux data purge`
- **PR-04:** Audit logs retidos por 30 dias por padrão em `~/.local/share/bmux/audit/`

---

## Functional Requirements

### FR-TUI: Interface de Terminal

| ID | Functional Requirement | Acceptance Criteria |
|---|---|---|
| FR-TUI-01 | Usuário pode dividir o terminal em panes horizontais e verticais | Ctrl-b \ e Ctrl-b - criam splits; panes são independentes |
| FR-TUI-02 | Usuário pode navegar entre panes usando teclas Vim (h/j/k/l) | Foco muda instantaneamente; pane ativo tem borda destacada |
| FR-TUI-03 | Usuário pode criar, listar e alternar entre windows numeradas | Ctrl-b c cria; Ctrl-b 1-9 navega; status bar mostra todas |
| FR-TUI-04 | Usuário pode criar, listar e reatachar sessions nomeadas | `bmux new <nome>`, `bmux ls`, `bmux attach -s <nome>` funcionam |
| FR-TUI-05 | Usuário pode dar zoom em um pane ocupando a tela inteira | Ctrl-b z alterna entre zoom e layout normal |
| FR-TUI-06 | Usuário pode entrar em scroll/copy mode para revisar histórico | Ctrl-b [ entra em modo scroll; q sai; suporte a 10.000 linhas |
| FR-TUI-07 | Usuário vê status bar permanente com estado de todos os agentes | Status bar mostra: tipo, modelo, status (●/○), tokens, custo por agente |
| FR-TUI-08 | Usuário pode recarregar a configuração sem reiniciar a session | Ctrl-b r recarrega config.toml sem matar agentes ativos |

### FR-AGT: Gerenciamento de Agentes

| ID | Functional Requirement | Acceptance Criteria |
|---|---|---|
| FR-AGT-01 | Usuário pode spawnar um agente especificando tipo e nome | `bmux agent spawn --type <tipo> --name <nome>` cria pane e processo |
| FR-AGT-02 | Usuário pode spawnar qualquer agente CLI como processo filho gerenciado | Qualquer binário configurado em config.toml pode ser spawnado |
| FR-AGT-03 | Usuário pode ver status detalhado de um agente específico | `bmux agent status <nome>` retorna: uptime, tokens usados, custo, última tarefa |
| FR-AGT-04 | Usuário pode encerrar um agente sem afetar outros agentes ou a session | `bmux agent kill <nome>` termina processo e remove pane |
| FR-AGT-05 | Usuário pode listar todos os agentes ativos com seus status | `bmux agent list` retorna tabela com: nome, tipo, modelo, status, custo |
| FR-AGT-06 | Sistema mantém agentes rodando após o usuário fazer detach da session | `bmux detach` (Ctrl-b d) não mata processos filhos |

### FR-TASK: Envio e Roteamento de Tarefas

| ID | Functional Requirement | Acceptance Criteria |
|---|---|---|
| FR-TASK-01 | Usuário pode enviar uma tarefa a um agente específico pelo nome | `bmux task send <nome> "<prompt>"` entrega tarefa ao agente correto |
| FR-TASK-02 | Usuário pode enviar uma tarefa sem especificar agente e o sistema roteia automaticamente | `bmux task send --auto "<prompt>"` seleciona agente com maior score |
| FR-TASK-03 | Sistema seleciona agente idle com base em capacidade, custo e disponibilidade | Task router avalia: capability_score (0.5) + cost_score (0.3) + idle_score (0.2) |
| FR-TASK-04 | Usuário pode ver a fila de tarefas pendentes | `bmux task list` mostra: id, destino, status, tempo na fila |
| FR-TASK-05 | Usuário pode cancelar uma tarefa que ainda não foi processada | `bmux task cancel <id>` remove da fila se não iniciada |
| FR-TASK-06 | Agente pode publicar resultado de tarefa acessível por outros agentes | Resultados ficam disponíveis no shared context com chave `task:{id}:result` |

### FR-CTX: Contexto Compartilhado

| ID | Functional Requirement | Acceptance Criteria |
|---|---|---|
| FR-CTX-01 | Usuário pode ler e escrever valores no shared context via CLI | `bmux context get <chave>` e `bmux context set <chave> <valor>` funcionam |
| FR-CTX-02 | Agente pode ler o output de outro agente via shared context | Qualquer agente acessa `agent:{id}:last_output` via IPC |
| FR-CTX-03 | Contexto persiste durante toda a session e é limpo ao encerrar | Valores sobrevivem a detach/reattach; são removidos com `bmux kill -s` |
| FR-CTX-04 | Usuário pode exportar todo o contexto da session como JSON | `bmux context dump` gera arquivo JSON com todos os pares chave-valor |

### FR-WFL: Workflows Minion Engine

| ID | Functional Requirement | Acceptance Criteria |
|---|---|---|
| FR-WFL-01 | Usuário pode executar um workflow YAML que orquestra múltiplos agentes | `bmux workflow run <arquivo.yaml>` executa todos os steps definidos |
| FR-WFL-02 | Usuário pode passar parâmetros de entrada para um workflow | `bmux workflow run <arquivo.yaml> --input '{"feature":"login"}'` injeta variáveis |
| FR-WFL-03 | Usuário pode ver o status de um workflow em execução | `bmux workflow status <id>` mostra: steps concluídos, em progresso, pendentes |
| FR-WFL-04 | Sistema executa steps sem dependências em paralelo | Steps com `depends_on` vazios rodam simultâneos; steps com deps aguardam |

### FR-SEC: Segurança

| ID | Functional Requirement | Acceptance Criteria |
|---|---|---|
| FR-SEC-01 | Sistema restringe acesso de agentes ao filesystem do projeto | Agente não consegue ler/escrever fora do `project_dir` configurado |
| FR-SEC-02 | Sistema armazena API keys sem expô-las em logs ou variáveis de ambiente | API keys não aparecem em `env`, logs de auditoria ou shared context |
| FR-SEC-03 | Sistema registra log de auditoria de todas as ações dos agentes | Cada spawn, task e acesso a arquivo gera entrada em `~/.local/share/bmux/audit/` |

---

## 📊 Seção 5 — Análise de Mercado, Roadmap e Go-to-Market

### TAM / SAM / SOM

| Mercado | Tamanho | Base de cálculo |
|---|---|---|
| **TAM** | $28B | Mercado global de ferramentas de desenvolvimento (2024) |
| **SAM** | $2.4B | ~8M devs usando AI coding tools regularmente |
| **SOM (ano 1)** | $480k | 2.000 usuários pagantes × $20/mês |
| **SOM (ano 3)** | $6M | 25.000 usuários pagantes × $20/mês |

### Análise Competitiva Detalhada

| Feature | tmux | Zellij | amux | Cursor | OpenCode | **BMUX** |
|---|---|---|---|---|---|---|
| Multi-agente nativo | ✗ | ✗ | Parcial | ✗ | ✗ | ✅ |
| Message bus real | ✗ | ✗ | ✗ | ✗ | ✗ | ✅ |
| Task routing automático | ✗ | ✗ | ✗ | ✗ | ✗ | ✅ |
| Shared context | ✗ | ✗ | ✗ | ✗ | ✗ | ✅ |
| Suporte a qualquer agente | ✗ | ✗ | ✗ | ✗ | ✗ | ✅ |
| Open source (MIT) | ✅ | ✅ | ✅ | ✗ | ✅ | ✅ |
| Rust | ✗ | ✅ | ✗ | ✗ | ✗ | ✅ |
| Terminal nativo | ✅ | ✅ | ✅ | ✗ | ✅ | ✅ |
| Compatível com tmux keys | ✅ | Parcial | ✅ | ✗ | ✗ | ✅ |
| Workflow DSL | ✗ | ✗ | ✗ | ✗ | ✗ | ✅ (Minion Engine) |
| Dashboard de custo | ✗ | ✗ | ✗ | Parcial | ✗ | ✅ |
| Subscription necessária | ✗ | ✗ | ✗ | $20/mês | ✗ | ✗ |

### Positioning Statement

> Para desenvolvedores que usam agentes de IA no terminal, o BMUX é o multiplexer que transforma ferramentas isoladas em uma equipe orquestrada — ao contrário do tmux e do amux, o BMUX oferece comunicação real entre agentes, roteamento inteligente por custo e visibilidade unificada, sem abrir mão da velocidade e familiaridade do terminal.

### Roadmap Detalhado

**MVP — mês 3 (scope fechado)**

IN:
- TUI básica (panes, windows, sessions) compatível com tmux
- Suporte a Claude Code, OpenCode, Pi como agentes gerenciados
- Message bus via Unix socket (JSON + HMAC)
- Task routing manual (usuário escolhe agente)
- Status bar com modelo, status e tokens por agente
- `cargo install bmux` no macOS e Linux
- Config via TOML
- README + quickstart

OUT do MVP:
- Task router automático (v1.0)
- Shared context persistente (v1.0)
- Minion Engine integration (v1.0)
- SSH remoto (v2.0)
- Plugin WASM (v2.0)
- GUI companion (v2.0)

**v1.0 — mês 6**
- Task router automático (score por custo + modelo)
- Shared context store persistente (sled)
- Integração Minion Engine (YAML workflows)
- Suporte a Gemini CLI e agentes customizados
- Homebrew + AUR + nixpkgs
- Dashboard de custo em tempo real
- Modo supervisão (ver todos os agentes simultaneamente)

**v2.0 — mês 12**
- SSH remoto (rodar agentes em servidores remotos com GPU)
- Plugin system (agentes como plugins WASM)
- BMUX Cloud (sync de sessions entre máquinas)
- Integração com GitHub Actions
- Modo headless para CI/CD

### Estratégia de Distribuição

```bash
# Dia 1 — Cargo (zero fricção para devs Rust)
cargo install bmux

# Semana 2 — Homebrew (macOS mainstream)
brew install bruno/tap/bmux
# ou fórmula oficial após 500+ stars:
brew install bmux

# Mês 1 — AUR (Arch Linux community)
paru -S bmux

# Mês 2 — nixpkgs
nix run nixpkgs#bmux

# Mês 3 — Binários pré-compilados no GitHub Releases
# (linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64)
curl -fsSL https://bmux.dev/install | bash
```

### Go-to-Market — Semana do Lançamento

**Dia 1 — Hacker News**
Título: *"Show HN: BMUX – tmux for the AI agent era, written in Rust"*
- Demo GIF mostrando Claude Code + OpenCode + Pi colaborando em um feature
- Link para README com quickstart de 60 segundos
- Benchmarks vs tmux (startup time, memory)

**Dia 1 — X/Twitter**
- Thread técnica com 10 tweets explicando o problema e a solução
- GIF animado do status bar em tempo real com custo por agente
- Mencionar @AnthropicAI, @rustlang

**Dia 2 — Dev.to / Medium**
Artigo: *"Como construí um terminal que faz agentes de IA se comunicarem (e porque precisava ser em Rust)"*

**Dia 3 — Subreddits**
- r/rust, r/programming, r/commandline, r/MachineLearning

**Semana 2 — YouTube**
- Vídeo de 10 min: "BMUX: rodando 4 agentes de IA em paralelo no terminal"

### Proposta para Parceria Anthropic

**Por que o BMUX beneficia a Anthropic:**

1. **Aumenta o uso do Claude Code** — ao ser o ambiente nativo de orquestração, desenvolvedores usam mais Claude Code, não menos. É amplificador, não substituto.

2. **Caso de uso concreto para multi-agent** — demonstra o valor das capacidades multi-agent em desenvolvimento de software real.

3. **Base de usuários heavy** — 5k–20k desenvolvedores heavy users de Claude como usuários diretos, com feedback de alta qualidade.

4. **Open source com MIT** — pode ser recomendado oficialmente sem lock-in.

**O que pedir à Anthropic:**
- Menção no blog/docs como "ambiente recomendado para workflows multi-agente com Claude Code"
- Acesso antecipado a features multi-agent do Claude Code (hooks, eventos, streaming de status)
- Suporte técnico para integração profunda
- Co-marketing em lançamento da v1.0

### Métricas de Adoção

| Métrica | MVP (mês 3) | v1.0 (mês 6) | v2.0 (mês 12) |
|---|---|---|---|
| GitHub Stars | 500 | 5.000 | 20.000 |
| Downloads/mês | 1.000 | 10.000 | 50.000 |
| Contribuidores | 5 | 50 | 200 |
| Issues abertas/fechadas | 20/15 | 100/80 | 300/250 |
| Discord membros | 100 | 1.000 | 5.000 |

### Modelo de Monetização Futuro

| Tier | Preço | Inclui |
|---|---|---|
| **Open Source** | Gratuito | 100% das features — sempre free |
| **BMUX Cloud** | $9/mês | Sync de sessions entre máquinas, histórico em nuvem, backup de context |
| **Enterprise** | $29/seat/mês | SSO, audit logs centralizados, suporte prioritário, SLA 99.9% |

**Princípio:** O produto open source nunca será degradado para forçar upgrade. Monetização via conveniência (cloud sync) e compliance (enterprise), não via feature gating.

---

## Apêndice — Glossário

| Termo | Definição |
|---|---|
| Agent Adapter | Interface Rust que abstrai qualquer agente CLI no ecossistema BMUX |
| Message Bus | Sistema de mensagens assíncrono via Unix socket para comunicação entre agentes |
| Shared Context | Store KV persistente (sled) acessível por todos os agentes na sessão |
| Task Router | Algoritmo que seleciona o agente mais adequado para uma tarefa baseado em custo, modelo e disponibilidade |
| Minion Engine | Motor de workflows YAML do projeto irmão do autor, integrado nativamente ao BMUX |
| Pane | Divisão da janela do terminal, pode conter um humano ou um agente |
| Session | Grupo de windows/panes que persiste mesmo após detach |
| Landlock | Mecanismo de sandboxing do Linux kernel 5.13+ baseado em capabilities de filesystem |

---

*BMUX PRD v0.1 — gerado colaborativamente por time de agentes especializados*  
*Bruno Oliveira Silva · Recife, BR · Março 2026*  
*Licença do documento: CC BY 4.0*

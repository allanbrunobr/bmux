# Afya Realtime Insights - Factory Droids Configuration

Este documento explica a configuração dos Factory Droids e Hooks para o Afya Realtime Insights.

## 📋 Sumário

- [Estrutura](#estrutura)
- [Hooks Automáticos](#hooks-automáticos)
- [Droids Disponíveis](#droids-disponíveis)
- [Workflows](#workflows)

## 📁 Estrutura

```
.factory/
├── settings.json              # ← CONFIGURAÇÃO DOS HOOKS
├── droids/                    # Droids especializados
├── workflows/                 # Workflows automatizados
├── hooks/                     # Scripts de validação
│   ├── pre-edit-safety.py     # Bloqueia secrets/PHI
│   ├── validate-bash.py       # Bloqueia comandos perigosos
│   ├── post-edit-checks.py    # Checa após edição
│   └── on-stop-summary.sh     # Resumo ao finalizar
└── logs/                      # Logs de mudanças
```

## 🪝 Hooks Automáticos

Os hooks são executados **automaticamente** pelo Droid em eventos específicos.

### Configuração (`.factory/settings.json`)

```json
{
  "hooks": {
    "PreToolUse": [...],   // Antes de usar ferramenta
    "PostToolUse": [...],  // Depois de usar ferramenta
    "Stop": [...]          // Quando Droid termina
  }
}
```

### Eventos e Ações

| Evento | Matcher | Script | Ação |
|--------|---------|--------|------|
| **PreToolUse** | `Edit\|Create` | `pre-edit-safety.py` | Bloqueia secrets, PHI, arquivos sensíveis |
| **PreToolUse** | `Bash` | `validate-bash.py` | Bloqueia comandos destrutivos |
| **PostToolUse** | `Edit\|Create` | `post-edit-checks.py` | Loga mudanças, lembra de testes |
| **Stop** | - | `on-stop-summary.sh` | Mostra resumo de arquivos críticos |

### O Que É Bloqueado (Exit 2)

**pre-edit-safety.py:**
- 🔐 Secrets (API keys, tokens, senhas)
- 🔐 Credenciais de banco de dados
- 🔐 Chaves privadas (PEM, RSA)
- 📁 Arquivos sensíveis (.env, .pem, .key)

**validate-bash.py:**
- 🚫 `rm -rf /` e variantes
- 🚫 `DROP DATABASE`, `TRUNCATE TABLE`
- 🚫 `cat .env`, `cat *.pem`
- 🚫 Fork bombs, mkfs, dd perigoso

### O Que Gera Aviso (Não bloqueia)

- ⚠️ PHI (CPF, nomes de pacientes)
- 🏥 Mudanças em código médico AI
- ⚡ Mudanças em código realtime/WebSocket
- 🎙️ Mudanças em transcrição ao vivo
- ⚠️ `git push --force`
- ⚠️ Migrações de banco

### Exemplo de Bloqueio

```
🔐 BLOQUEADO: Possível API Key detectado no conteúdo!
   Arquivo: backend/config.py
   Match: OPENAI_API_KEY = "sk-abc123..."
   
   Use variáveis de ambiente ou .env em vez de hardcode.
```

### Exemplo de Aviso Médico

```
🏥 CÓDIGO MÉDICO AI ══════════════════════════════════════════
🏥 CÓDIGO MÉDICO AI  CÓDIGO CRÍTICO SENDO MODIFICADO
🏥 CÓDIGO MÉDICO AI ══════════════════════════════════════════

   Arquivo: backend/app/agents/specialists/safety.py

   LEMBRETE:
   • Mudanças podem afetar segurança do paciente
   • Teste exaustivamente antes de deploy
   • PR requer review cuidadoso
```

### Exemplo de Aviso Realtime

```
⚡ CÓDIGO REALTIME ══════════════════════════════════════════
⚡ CÓDIGO REALTIME  CÓDIGO CRÍTICO SENDO MODIFICADO
⚡ CÓDIGO REALTIME ══════════════════════════════════════════

   Arquivo: backend/app/realtime/websocket_handler.py

   LEMBRETE:
   • Mudanças podem afetar segurança do paciente
   • Teste exaustivamente antes de deploy
   • PR requer review cuidadoso
```

## 📋 Resumo ao Parar (Stop)

Quando o Droid termina uma resposta:

```
🏥 ══════════════════════════════════════════════════
🏥  ARQUIVOS MÉDICOS MODIFICADOS NESTA SESSÃO
🏥 ══════════════════════════════════════════════════

   • backend/app/agents/coordinator.py
   • backend/app/agents/specialists/safety.py

   ANTES DE COMMITAR:
   Rode os testes de segurança médica

⚡ ══════════════════════════════════════════════════
⚡  ARQUIVOS REALTIME MODIFICADOS NESTA SESSÃO
⚡ ══════════════════════════════════════════════════

   • backend/app/realtime/gemini_live.py

   ANTES DE COMMITAR:
   Teste conexões WebSocket e reconexão
```

## 🤖 Droids Disponíveis

| Droid | Função |
|-------|--------|
| `security-auditor` | Segurança, LGPD/HIPAA |
| `medical-prompt-auditor` | Auditoria de prompts médicos |
| `test-writer-evo` | Geração de testes |
| `code-reviewer-evo` | Review de código |
| `python-bug-fixer-evo` | Correção de bugs Python |
| `prompt-engineer-evo` | Otimização de prompts |
| `api-designer-evo` | Design de APIs |
| `react-specialist-evo` | Frontend React |
| `typescript-bug-fixer-evo` | Bugs TypeScript |
| `documentation-writer-evo` | Documentação |

## 🔧 Desabilitar Hooks Temporariamente

Edite `.factory/settings.json` e mude para:

```json
{
  "hooks": {}
}
```

## 📊 Logs

Mudanças são logadas em `.factory/logs/changes.log`:

```json
{"timestamp": "2024-01-15T10:30:00", "session_id": "abc123", "file": "backend/app/agents/coordinator.py", "category": "medical"}
{"timestamp": "2024-01-15T10:31:00", "session_id": "abc123", "file": "backend/app/realtime/handler.py", "category": "realtime"}
```

---

**Maintainer**: Afya Team

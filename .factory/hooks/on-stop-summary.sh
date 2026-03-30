#!/bin/bash
# Afya Realtime Insights - On Stop Summary Hook
# Runs when Droid finishes a response.
# Shows summary of critical files changed in session.

LOG_FILE="${FACTORY_PROJECT_DIR}/.factory/logs/changes.log"

# Check if log file exists
if [ ! -f "$LOG_FILE" ]; then
    exit 0
fi

# Get medical changes from recent entries
MEDICAL_CHANGES=$(tail -50 "$LOG_FILE" | grep '"category": "medical"' | jq -r '.file' 2>/dev/null | sort -u)

# Get realtime changes from recent entries
REALTIME_CHANGES=$(tail -50 "$LOG_FILE" | grep '"category": "realtime"' | jq -r '.file' 2>/dev/null | sort -u)

# Show medical changes if any
if [ -n "$MEDICAL_CHANGES" ]; then
    echo "" >&2
    echo "🏥 ══════════════════════════════════════════════════" >&2
    echo "🏥  ARQUIVOS MÉDICOS MODIFICADOS NESTA SESSÃO" >&2
    echo "🏥 ══════════════════════════════════════════════════" >&2
    echo "" >&2
    echo "$MEDICAL_CHANGES" | while read -r file; do
        [ -n "$file" ] && echo "   • $file" >&2
    done
    echo "" >&2
    echo "   ANTES DE COMMITAR:" >&2
    echo "   Rode os testes de segurança médica" >&2
    echo "" >&2
fi

# Show realtime changes if any
if [ -n "$REALTIME_CHANGES" ]; then
    echo "" >&2
    echo "⚡ ══════════════════════════════════════════════════" >&2
    echo "⚡  ARQUIVOS REALTIME MODIFICADOS NESTA SESSÃO" >&2
    echo "⚡ ══════════════════════════════════════════════════" >&2
    echo "" >&2
    echo "$REALTIME_CHANGES" | while read -r file; do
        [ -n "$file" ] && echo "   • $file" >&2
    done
    echo "" >&2
    echo "   ANTES DE COMMITAR:" >&2
    echo "   Teste conexões WebSocket e reconexão" >&2
    echo "" >&2
fi

exit 0

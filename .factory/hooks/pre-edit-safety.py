#!/usr/bin/env python3
"""
Afya Realtime Insights - Pre-Edit Safety Hook
Runs BEFORE Droid edits or creates files.

Blocks:
- Secrets in new content
- PHI in new content
- Edits to sensitive files (.env, .pem, .key)

Warns:
- Medical AI code changes
- Realtime/WebSocket code changes
"""

import json
import sys
import re
from pathlib import Path

# ═══════════════════════════════════════════════════════════════
# CONFIGURATION
# ═══════════════════════════════════════════════════════════════

# Files that should NEVER be edited by Droid
BLOCKED_FILES = [
    r"\.env$",
    r"\.env\..*$",
    r"\.pem$",
    r"\.key$",
    r"id_rsa",
    r"\.secret",
    r"credentials\.json$",
    r"service-account.*\.json$",
]

# Patterns that indicate secrets (BLOCK)
SECRET_PATTERNS = [
    (r"(api[_-]?key|apikey)\s*[=:]\s*['\"][a-zA-Z0-9_\-]{20,}['\"]", "API Key"),
    (r"(secret|password|passwd|pwd)\s*[=:]\s*['\"][^'\"]{8,}['\"]", "Password/Secret"),
    (r"(token)\s*[=:]\s*['\"][a-zA-Z0-9_\-\.]{20,}['\"]", "Token"),
    (r"-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----", "Private Key"),
    (r"-----BEGIN\s+CERTIFICATE-----", "Certificate"),
    (r"(postgres|mysql|mongodb)://[^@]+:[^@]+@", "Database URL with credentials"),
    (r"sk-[a-zA-Z0-9]{48}", "OpenAI API Key"),
    (r"AIza[a-zA-Z0-9_-]{35}", "Google API Key"),
    (r"ghp_[a-zA-Z0-9]{36}", "GitHub Personal Access Token"),
    (r"xox[baprs]-[a-zA-Z0-9-]+", "Slack Token"),
]

# Patterns that indicate PHI (WARN)
PHI_PATTERNS = [
    (r"\b\d{3}\.\d{3}\.\d{3}-\d{2}\b", "CPF (Brazilian ID)"),
    (r"\b\d{3}-\d{2}-\d{4}\b", "SSN (US Social Security)"),
    (r"(paciente|patient)\s*[=:]\s*['\"]?[A-Z][a-z]+\s+[A-Z][a-z]+", "Patient Name"),
    (r"prontuário\s*[=:#]\s*\d+", "Medical Record Number"),
]

# Medical/Realtime AI paths (WARN)
CRITICAL_PATHS = [
    (r"backend/app/agents/", "🏥 CÓDIGO MÉDICO AI"),
    (r"agents/specialists/", "🏥 CÓDIGO MÉDICO AI"),
    (r"agents/coordinator", "🏥 CÓDIGO MÉDICO AI"),
    (r"agents/critic", "🏥 CÓDIGO MÉDICO AI"),
    (r"clinical_tools", "🏥 CÓDIGO MÉDICO AI"),
    (r"realtime/", "⚡ CÓDIGO REALTIME"),
    (r"websocket", "⚡ CÓDIGO WEBSOCKET"),
    (r"live.*transcri", "🎙️ TRANSCRIÇÃO AO VIVO"),
    (r"gemini.*live", "🎙️ GEMINI LIVE"),
]

# ═══════════════════════════════════════════════════════════════
# MAIN LOGIC
# ═══════════════════════════════════════════════════════════════

def main():
    # Read input from stdin
    try:
        input_data = json.load(sys.stdin)
    except json.JSONDecodeError:
        sys.exit(0)  # No input, allow
    
    tool_input = input_data.get("tool_input", {})
    
    # Get file path (different field names for Edit vs Create)
    file_path = tool_input.get("file_path") or tool_input.get("path", "")
    
    # Get new content
    new_content = tool_input.get("new_str") or tool_input.get("content", "")
    
    if not file_path:
        sys.exit(0)  # No file, allow
    
    # ─────────────────────────────────────────────────────────────
    # CHECK 1: Blocked files
    # ─────────────────────────────────────────────────────────────
    for pattern in BLOCKED_FILES:
        if re.search(pattern, file_path, re.IGNORECASE):
            print(f"🚫 BLOQUEADO: Arquivo sensível não pode ser editado: {file_path}", file=sys.stderr)
            print(f"   Padrão bloqueado: {pattern}", file=sys.stderr)
            sys.exit(2)  # Exit 2 = block
    
    # ─────────────────────────────────────────────────────────────
    # CHECK 2: Secrets in new content
    # ─────────────────────────────────────────────────────────────
    if new_content:
        for pattern, name in SECRET_PATTERNS:
            match = re.search(pattern, new_content, re.IGNORECASE)
            if match:
                # Show truncated match for context
                matched_text = match.group(0)
                truncated = matched_text[:50] + "..." if len(matched_text) > 50 else matched_text
                
                print(f"🔐 BLOQUEADO: Possível {name} detectado no conteúdo!", file=sys.stderr)
                print(f"   Arquivo: {file_path}", file=sys.stderr)
                print(f"   Match: {truncated}", file=sys.stderr)
                print(f"", file=sys.stderr)
                print(f"   Use variáveis de ambiente ou .env em vez de hardcode.", file=sys.stderr)
                sys.exit(2)  # Exit 2 = block
    
    # ─────────────────────────────────────────────────────────────
    # CHECK 3: PHI in new content (warn only)
    # ─────────────────────────────────────────────────────────────
    if new_content:
        for pattern, name in PHI_PATTERNS:
            match = re.search(pattern, new_content, re.IGNORECASE)
            if match:
                matched_text = match.group(0)
                print(f"⚠️ AVISO: Possível {name} detectado!", file=sys.stderr)
                print(f"   Arquivo: {file_path}", file=sys.stderr)
                print(f"   Verifique se são dados mock, não reais.", file=sys.stderr)
                # Don't block, just warn (exit 0)
    
    # ─────────────────────────────────────────────────────────────
    # CHECK 4: Critical code paths (warn only)
    # ─────────────────────────────────────────────────────────────
    for pattern, label in CRITICAL_PATHS:
        if re.search(pattern, file_path, re.IGNORECASE):
            print(f"", file=sys.stderr)
            print(f"{label} ══════════════════════════════════════════", file=sys.stderr)
            print(f"{label}  CÓDIGO CRÍTICO SENDO MODIFICADO", file=sys.stderr)
            print(f"{label} ══════════════════════════════════════════", file=sys.stderr)
            print(f"", file=sys.stderr)
            print(f"   Arquivo: {file_path}", file=sys.stderr)
            print(f"", file=sys.stderr)
            print(f"   LEMBRETE:", file=sys.stderr)
            print(f"   • Mudanças podem afetar segurança do paciente", file=sys.stderr)
            print(f"   • Teste exaustivamente antes de deploy", file=sys.stderr)
            print(f"   • PR requer review cuidadoso", file=sys.stderr)
            print(f"", file=sys.stderr)
            break  # Only warn once
    
    # All checks passed
    sys.exit(0)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
Afya Realtime Insights - Bash Command Validator Hook
Runs BEFORE Droid executes bash commands.

Blocks:
- Destructive commands (rm -rf /, etc.)
- Commands that might expose secrets
- Database drop commands

Warns:
- Git push to protected branches
- Database migrations
"""

import json
import sys
import re

# ═══════════════════════════════════════════════════════════════
# CONFIGURATION
# ═══════════════════════════════════════════════════════════════

# Commands that are ALWAYS blocked
BLOCKED_COMMANDS = [
    (r"rm\s+-rf\s+/(?!\w)", "rm -rf / (root deletion)"),
    (r"rm\s+-rf\s+~", "rm -rf ~ (home deletion)"),
    (r"rm\s+-rf\s+\*", "rm -rf * (wildcard deletion)"),
    (r"rm\s+-rf\s+\.\.", "rm -rf .. (parent deletion)"),
    (r"mkfs\.", "mkfs (format filesystem)"),
    (r"dd\s+if=.*of=/dev/", "dd to device"),
    (r">\s*/dev/sd", "overwrite disk device"),
    (r"chmod\s+-R\s+777\s+/", "chmod 777 on root"),
    (r":()\s*{\s*:\s*\|\s*:\s*&\s*}", "fork bomb"),
]

# Commands that expose secrets (BLOCKED)
SECRET_EXPOSING = [
    (r"cat\s+.*\.env", "cat .env (exposes secrets)"),
    (r"cat\s+.*\.pem", "cat .pem (exposes keys)"),
    (r"cat\s+.*\.key", "cat .key (exposes keys)"),
    (r"cat\s+.*id_rsa", "cat id_rsa (exposes SSH key)"),
    (r"echo\s+.*\$\{?[A-Z_]*KEY", "echo API key variable"),
    (r"echo\s+.*\$\{?[A-Z_]*SECRET", "echo secret variable"),
    (r"echo\s+.*\$\{?[A-Z_]*PASSWORD", "echo password variable"),
    (r"printenv\s+(.*KEY|.*SECRET|.*PASSWORD)", "printenv secrets"),
]

# Database destructive commands (BLOCKED)
DB_DESTRUCTIVE = [
    (r"DROP\s+DATABASE", "DROP DATABASE"),
    (r"DROP\s+TABLE", "DROP TABLE"),
    (r"TRUNCATE\s+TABLE", "TRUNCATE TABLE"),
    (r"DELETE\s+FROM\s+\w+\s*;?\s*$", "DELETE without WHERE"),
    (r"dropdb\s+", "dropdb command"),
]

# Warning patterns (not blocked)
WARNING_PATTERNS = [
    (r"git\s+push.*(-f|--force)", "Force push detected"),
    (r"git\s+push.*(main|master|prod)", "Push to protected branch"),
    (r"alembic\s+upgrade", "Database migration"),
    (r"npm\s+publish", "npm publish"),
    (r"pip\s+install.*--break-system-packages", "System pip install"),
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
    command = tool_input.get("command", "")
    
    if not command:
        sys.exit(0)  # No command, allow
    
    # ─────────────────────────────────────────────────────────────
    # CHECK 1: Blocked destructive commands
    # ─────────────────────────────────────────────────────────────
    for pattern, name in BLOCKED_COMMANDS:
        if re.search(pattern, command, re.IGNORECASE):
            print(f"🚫 BLOQUEADO: Comando destrutivo detectado!", file=sys.stderr)
            print(f"   Comando: {command[:100]}", file=sys.stderr)
            print(f"   Motivo: {name}", file=sys.stderr)
            sys.exit(2)  # Exit 2 = block
    
    # ─────────────────────────────────────────────────────────────
    # CHECK 2: Commands that expose secrets
    # ─────────────────────────────────────────────────────────────
    for pattern, name in SECRET_EXPOSING:
        if re.search(pattern, command, re.IGNORECASE):
            print(f"🔐 BLOQUEADO: Comando pode expor segredos!", file=sys.stderr)
            print(f"   Comando: {command[:100]}", file=sys.stderr)
            print(f"   Motivo: {name}", file=sys.stderr)
            print(f"", file=sys.stderr)
            print(f"   Use variáveis de ambiente diretamente no código.", file=sys.stderr)
            sys.exit(2)  # Exit 2 = block
    
    # ─────────────────────────────────────────────────────────────
    # CHECK 3: Database destructive commands
    # ─────────────────────────────────────────────────────────────
    for pattern, name in DB_DESTRUCTIVE:
        if re.search(pattern, command, re.IGNORECASE):
            print(f"💾 BLOQUEADO: Comando destrutivo de banco de dados!", file=sys.stderr)
            print(f"   Comando: {command[:100]}", file=sys.stderr)
            print(f"   Motivo: {name}", file=sys.stderr)
            print(f"", file=sys.stderr)
            print(f"   Execute manualmente se realmente necessário.", file=sys.stderr)
            sys.exit(2)  # Exit 2 = block
    
    # ─────────────────────────────────────────────────────────────
    # CHECK 4: Warning patterns (don't block)
    # ─────────────────────────────────────────────────────────────
    for pattern, name in WARNING_PATTERNS:
        if re.search(pattern, command, re.IGNORECASE):
            print(f"⚠️ AVISO: {name}", file=sys.stderr)
            print(f"   Comando: {command[:100]}", file=sys.stderr)
            # Don't block, just warn
    
    # All checks passed
    sys.exit(0)


if __name__ == "__main__":
    main()

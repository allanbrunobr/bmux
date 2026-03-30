#!/usr/bin/env python3
"""
Afya Realtime Insights - Post-Edit Checks Hook
Runs AFTER Droid edits or creates files.

Actions:
- Log critical code changes
- Remind about tests for medical/realtime files
- Track files modified in session
"""

import json
import sys
import re
import os
from pathlib import Path
from datetime import datetime

# ═══════════════════════════════════════════════════════════════
# CONFIGURATION
# ═══════════════════════════════════════════════════════════════

# Critical paths that need special attention
CRITICAL_PATHS = [
    (r"backend/app/agents/", "medical", "🏥 Arquivo médico"),
    (r"agents/specialists/", "medical", "🏥 Arquivo médico"),
    (r"agents/coordinator", "medical", "🏥 Arquivo médico"),
    (r"agents/critic", "medical", "🏥 Arquivo médico"),
    (r"clinical_tools", "medical", "🏥 Arquivo médico"),
    (r"realtime/", "realtime", "⚡ Arquivo realtime"),
    (r"websocket", "realtime", "⚡ Arquivo websocket"),
    (r"live.*transcri", "realtime", "🎙️ Arquivo transcrição"),
    (r"gemini.*live", "realtime", "🎙️ Arquivo Gemini Live"),
]

# Log file for tracking changes
LOG_DIR = Path(os.environ.get("FACTORY_PROJECT_DIR", ".")) / ".factory" / "logs"

# ═══════════════════════════════════════════════════════════════
# MAIN LOGIC
# ═══════════════════════════════════════════════════════════════

def log_change(file_path: str, session_id: str, category: str):
    """Log the file change for tracking."""
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    
    log_file = LOG_DIR / "changes.log"
    timestamp = datetime.now().isoformat()
    
    entry = {
        "timestamp": timestamp,
        "session_id": session_id,
        "file": file_path,
        "category": category,
    }
    
    with open(log_file, "a") as f:
        f.write(json.dumps(entry) + "\n")


def main():
    # Read input from stdin
    try:
        input_data = json.load(sys.stdin)
    except json.JSONDecodeError:
        sys.exit(0)
    
    session_id = input_data.get("session_id", "unknown")
    tool_input = input_data.get("tool_input", {})
    
    # Get file path
    file_path = tool_input.get("file_path") or tool_input.get("path", "")
    
    if not file_path:
        sys.exit(0)
    
    # ─────────────────────────────────────────────────────────────
    # CHECK: Is this critical code?
    # ─────────────────────────────────────────────────────────────
    category = "other"
    label = None
    
    for pattern, cat, lbl in CRITICAL_PATHS:
        if re.search(pattern, file_path, re.IGNORECASE):
            category = cat
            label = lbl
            break
    
    # Log the change
    try:
        log_change(file_path, session_id, category)
    except Exception:
        pass  # Don't fail on logging errors
    
    # ─────────────────────────────────────────────────────────────
    # REMINDER: Critical code changed
    # ─────────────────────────────────────────────────────────────
    if label:
        print(f"", file=sys.stderr)
        print(f"✅ {label} modificado: {Path(file_path).name}", file=sys.stderr)
        print(f"", file=sys.stderr)
        
        if category == "medical":
            print(f"📋 CHECKLIST MÉDICO:", file=sys.stderr)
            print(f"   □ Rodar testes de segurança", file=sys.stderr)
            print(f"   □ Verificar validação do critic", file=sys.stderr)
            print(f"   □ Testar detecção de red flags", file=sys.stderr)
        elif category == "realtime":
            print(f"📋 CHECKLIST REALTIME:", file=sys.stderr)
            print(f"   □ Testar conexão WebSocket", file=sys.stderr)
            print(f"   □ Verificar handling de reconexão", file=sys.stderr)
            print(f"   □ Testar com latência alta", file=sys.stderr)
        
        print(f"", file=sys.stderr)
    
    sys.exit(0)


if __name__ == "__main__":
    main()

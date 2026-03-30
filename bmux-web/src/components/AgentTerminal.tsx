'use client'

/**
 * AgentTerminal — Story 4.2
 *
 * Read-only xterm.js terminal modal for supervisory viewing of agent PTY output.
 * - Loads last 100 lines on mount via GET /api/pane-output
 * - Subscribes to WebSocket `pane_output` events for live streaming
 * - Keyboard input is disabled (supervisory view only)
 */

import { useEffect, useRef, useCallback } from 'react'
import { api, wsUrl } from '@/lib/api'
import type { BmuxEvent } from '@/lib/types'

interface Props {
  session: string
  paneId: string
  agentName: string
  onClose: () => void
}

export function AgentTerminal({ session, paneId, agentName, onClose }: Props) {
  const containerRef = useRef<HTMLDivElement>(null)
  const termRef = useRef<import('xterm').Terminal | null>(null)
  const fitRef = useRef<import('xterm-addon-fit').FitAddon | null>(null)
  const wsRef = useRef<WebSocket | null>(null)

  // Initialise xterm.js (dynamic import — SSR safe)
  const initTerminal = useCallback(async () => {
    if (!containerRef.current) return

    const { Terminal } = await import('xterm')
    const { FitAddon } = await import('xterm-addon-fit')
    const { WebLinksAddon } = await import('xterm-addon-web-links')

    const term = new Terminal({
      disableStdin: true,       // read-only: no keyboard input forwarded
      cursorBlink: false,
      theme: {
        background: '#000000',
        foreground: '#e6edf3',
        cursor: '#58a6ff',
        selectionBackground: '#264f78',
      },
      fontSize: 13,
      fontFamily: "'SF Mono', 'Fira Code', monospace",
      scrollback: 5000,
    })

    const fitAddon = new FitAddon()
    term.loadAddon(fitAddon)
    term.loadAddon(new WebLinksAddon())
    term.open(containerRef.current)
    fitAddon.fit()

    termRef.current = term
    fitRef.current = fitAddon

    // Load historical lines
    try {
      const { lines } = await api.getPaneOutput(session, paneId, 100)
      for (const line of lines) {
        term.writeln(line)
      }
    } catch {
      term.writeln('\x1b[33mCould not load pane history — daemon not reachable\x1b[0m')
    }

    // Live streaming via WebSocket
    const ws = new WebSocket(wsUrl(session))
    wsRef.current = ws

    ws.onmessage = (msg) => {
      try {
        const event = JSON.parse(msg.data as string) as BmuxEvent
        if (event.type === 'pane_output' && event.pane_id === paneId) {
          term.write(event.data)
        }
      } catch {
        // ignore malformed frames
      }
    }

    ws.onerror = () => {
      term.writeln('\r\n\x1b[31m[WebSocket error — live stream disconnected]\x1b[0m')
    }
  }, [session, paneId])

  useEffect(() => {
    initTerminal()

    const onResize = () => fitRef.current?.fit()
    window.addEventListener('resize', onResize)

    return () => {
      window.removeEventListener('resize', onResize)
      wsRef.current?.close()
      termRef.current?.dispose()
    }
  }, [initTerminal])

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70">
      <div className="flex flex-col w-[900px] max-w-[95vw] h-[600px] max-h-[90vh] bg-[#0d1117] border border-[#30363d] rounded-lg shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 bg-[#161b22] border-b border-[#30363d] shrink-0">
          <div className="flex items-center gap-2">
            <span className="text-[#58a6ff] font-mono text-sm">▶</span>
            <span className="text-[#e6edf3] font-mono text-sm font-semibold">{agentName}</span>
            <span className="text-[#8b949e] font-mono text-xs">/{paneId}</span>
            <span className="ml-2 px-2 py-0.5 text-xs bg-[#21262d] text-[#8b949e] rounded border border-[#30363d]">
              read-only
            </span>
          </div>
          <button
            onClick={onClose}
            className="text-[#8b949e] hover:text-[#e6edf3] transition-colors text-lg font-mono"
            aria-label="Close terminal"
          >
            ✕
          </button>
        </div>

        {/* xterm.js mount point */}
        <div
          ref={containerRef}
          className="flex-1 xterm-container"
          style={{ background: '#000' }}
        />
      </div>
    </div>
  )
}

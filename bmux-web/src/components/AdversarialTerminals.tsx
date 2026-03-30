'use client'

import { useEffect, useRef, useCallback } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { useBmuxStore } from '@/lib/store'
import { api, wsUrl } from '@/lib/api'
import type { BmuxEvent } from '@/lib/types'

interface EmbeddedTerminalProps {
  session: string
  paneId: number
  agentName: string
}

function EmbeddedTerminal({ session, paneId, agentName }: EmbeddedTerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const termRef = useRef<import('xterm').Terminal | null>(null)
  const fitRef = useRef<import('xterm-addon-fit').FitAddon | null>(null)
  const wsRef = useRef<WebSocket | null>(null)

  const initTerminal = useCallback(async () => {
    if (!containerRef.current) return

    const { Terminal } = await import('xterm')
    const { FitAddon } = await import('xterm-addon-fit')
    const { WebLinksAddon } = await import('xterm-addon-web-links')

    const term = new Terminal({
      disableStdin: true,
      cursorBlink: false,
      theme: {
        background: '#000000',
        foreground: '#e6edf3',
        cursor: '#58a6ff',
        selectionBackground: '#264f78',
      },
      fontSize: 12,
      fontFamily: "'SF Mono', 'Fira Code', monospace",
      scrollback: 2000,
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
      const { lines } = await api.getPaneOutput(session, String(paneId), 100)
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
    <div className="flex flex-col h-full">
      {/* Terminal header */}
      <div className="flex items-center gap-2 px-3 py-2 bg-[#161b22] border-b border-[#30363d] shrink-0">
        <span className="text-[#58a6ff] font-mono text-xs">▶</span>
        <span className="text-[#e6edf3] font-mono text-xs font-semibold">{agentName}</span>
        <span className="text-[#8b949e] font-mono text-xs">/{paneId}</span>
        <span className="ml-auto px-1.5 py-0.5 text-[10px] bg-[#21262d] text-[#8b949e] rounded border border-[#30363d]">
          read-only
        </span>
      </div>
      {/* xterm.js mount point */}
      <div
        ref={containerRef}
        className="flex-1 xterm-container"
        style={{ background: '#000000' }}
      />
    </div>
  )
}

function TerminalPlaceholder({ label }: { label: string }) {
  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 px-3 py-2 bg-[#161b22] border-b border-[#30363d] shrink-0">
        <span className="text-[#8b949e] font-mono text-xs font-semibold">{label}</span>
        <span className="ml-auto px-1.5 py-0.5 text-[10px] bg-[#21262d] text-[#8b949e] rounded border border-[#30363d]">
          waiting
        </span>
      </div>
      <div className="flex-1 flex items-center justify-center bg-[#0d1117]">
        <span className="text-[#8b949e] font-mono text-xs">Agent not spawned yet…</span>
      </div>
    </div>
  )
}

export function AdversarialTerminals() {
  const agents = useBmuxStore((s) => s.agents)
  const adversarial = useBmuxStore((s) => s.adversarial)
  const activeSession = useBmuxStore((s) => s.activeSession)

  if (!adversarial || !activeSession) return null

  const isActive = !['failed', 'complete'].includes(adversarial.phase)
  if (!isActive) return null

  const generator = agents.find(
    (a) => a.name.toLowerCase().includes('generator') && a.pane_id != null
  )
  const evaluator = agents.find(
    (a) => a.name.toLowerCase().includes('evaluator') && a.pane_id != null
  )

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base font-semibold text-foreground">Agent Terminals</CardTitle>
      </CardHeader>
      <CardContent className="p-0 pb-0">
        <div
          className="grid grid-cols-2 divide-x divide-[#30363d] bg-[#0d1117] rounded-b-lg overflow-hidden"
          style={{ minHeight: '280px' }}
        >
          {generator?.pane_id != null ? (
            <EmbeddedTerminal
              session={activeSession}
              paneId={generator.pane_id}
              agentName={generator.name}
            />
          ) : (
            <TerminalPlaceholder label="Generator" />
          )}
          {evaluator?.pane_id != null ? (
            <EmbeddedTerminal
              session={activeSession}
              paneId={evaluator.pane_id}
              agentName={evaluator.name}
            />
          ) : (
            <TerminalPlaceholder label="Evaluator" />
          )}
        </div>
      </CardContent>
    </Card>
  )
}

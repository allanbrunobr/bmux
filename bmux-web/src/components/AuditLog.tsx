'use client'

/**
 * AuditLog — Story 6.1
 *
 * Filterable audit log viewer for LGPD compliance:
 *   • Columns: timestamp, event type, agent, session, metadata
 *   • Filters: agent, event type, time range
 *   • LGPD compliance indicator per entry (green = compliant, yellow = unknown, red = non-compliant)
 *   • [Export CSV] button downloads filtered entries
 *   • Real-time updates via WebSocket `audit_event` events
 */

import { useState, useCallback, useMemo } from 'react'
import type { AuditEntry, BmuxEvent } from '@/lib/types'
import { MOCK_AUDIT } from '@/lib/mockData'
import { useBmuxSocket } from '@/lib/useBmuxSocket'

interface Props {
  session: string | null
}

type TimeRange = 'all' | '1h' | '24h' | '7d'

const TIME_RANGE_MS: Record<TimeRange, number> = {
  all: Infinity,
  '1h':  3_600_000,
  '24h': 86_400_000,
  '7d':  604_800_000,
}

function lgpdBadge(compliant: boolean | null) {
  if (compliant === true)  return <span className="px-2 py-0.5 text-xs rounded-full bg-[#0f3b1a] text-[#3fb950] border border-[#1a7f37]">LGPD ✓</span>
  if (compliant === false) return <span className="px-2 py-0.5 text-xs rounded-full bg-[#3b1a1a] text-[#f85149] border border-[#7f1a1a]">LGPD ✗</span>
  return <span className="px-2 py-0.5 text-xs rounded-full bg-[#2d2a1a] text-[#d29922] border border-[#7a6a1a]">LGPD ?</span>
}

function fmtTs(ts: string) {
  return new Date(ts).toLocaleString([], {
    month: 'short', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit',
  })
}

function entriesToCsv(entries: AuditEntry[]): string {
  const header = 'id,timestamp,event_type,agent,session,lgpd_compliant,metadata'
  const rows = entries.map((e) =>
    [
      e.id, e.timestamp, e.event_type, e.agent, e.session,
      e.lgpd_compliant === null ? 'unknown' : String(e.lgpd_compliant),
      `"${JSON.stringify(e.metadata).replace(/"/g, '""')}"`,
    ].join(',')
  )
  return [header, ...rows].join('\n')
}

export function AuditLog({ session }: Props) {
  const [entries, setEntries]         = useState<AuditEntry[]>(MOCK_AUDIT)
  const [filterAgent, setFilterAgent] = useState('')
  const [filterType, setFilterType]   = useState('')
  const [timeRange, setTimeRange]     = useState<TimeRange>('all')

  const handleEvent = useCallback((event: BmuxEvent) => {
    if (event.type === 'audit_event') {
      setEntries((prev) => [event.entry as AuditEntry, ...prev].slice(0, 1000))
    }
  }, [])

  useBmuxSocket(session, handleEvent)

  // Unique filter options
  const agentOptions = useMemo(() => Array.from(new Set(entries.map((e) => e.agent))), [entries])
  const typeOptions  = useMemo(() => Array.from(new Set(entries.map((e) => e.event_type))), [entries])

  // Apply filters
  const filtered = useMemo(() => {
    const cutoff = Date.now() - TIME_RANGE_MS[timeRange]
    return entries.filter((e) => {
      if (filterAgent && e.agent !== filterAgent) return false
      if (filterType  && e.event_type !== filterType) return false
      if (new Date(e.timestamp).getTime() < cutoff) return false
      return true
    })
  }, [entries, filterAgent, filterType, timeRange])

  const handleExport = () => {
    const csv  = entriesToCsv(filtered)
    const blob = new Blob([csv], { type: 'text/csv' })
    const url  = URL.createObjectURL(blob)
    const a    = document.createElement('a')
    a.href     = url
    a.download = `bmux-audit-${new Date().toISOString().slice(0, 10)}.csv`
    a.click()
    URL.revokeObjectURL(url)
  }

  return (
    <div className="flex flex-col gap-4">
      {/* Filter bar */}
      <div className="flex flex-wrap items-center gap-3 bg-[#161b22] border border-[#30363d] rounded-lg px-4 py-3">
        <select
          value={filterAgent}
          onChange={(e) => setFilterAgent(e.target.value)}
          className="bg-[#21262d] border border-[#30363d] text-[#e6edf3] text-sm rounded px-3 py-1.5 focus:outline-none focus:border-[#58a6ff]"
        >
          <option value="">All agents</option>
          {agentOptions.map((a) => <option key={a} value={a}>{a}</option>)}
        </select>

        <select
          value={filterType}
          onChange={(e) => setFilterType(e.target.value)}
          className="bg-[#21262d] border border-[#30363d] text-[#e6edf3] text-sm rounded px-3 py-1.5 focus:outline-none focus:border-[#58a6ff]"
        >
          <option value="">All event types</option>
          {typeOptions.map((t) => <option key={t} value={t}>{t}</option>)}
        </select>

        <select
          value={timeRange}
          onChange={(e) => setTimeRange(e.target.value as TimeRange)}
          className="bg-[#21262d] border border-[#30363d] text-[#e6edf3] text-sm rounded px-3 py-1.5 focus:outline-none focus:border-[#58a6ff]"
        >
          <option value="all">All time</option>
          <option value="1h">Last 1 hour</option>
          <option value="24h">Last 24 hours</option>
          <option value="7d">Last 7 days</option>
        </select>

        <span className="text-xs text-[#8b949e] ml-auto">{filtered.length} entries</span>

        <button
          onClick={handleExport}
          className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-[#21262d] border border-[#30363d] text-[#58a6ff] rounded hover:bg-[#30363d] transition-colors"
        >
          ↓ Export CSV
        </button>
      </div>

      {/* Table */}
      <div className="bg-[#161b22] border border-[#30363d] rounded-lg overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-[#8b949e] text-xs border-b border-[#30363d]">
                <th className="text-left px-4 py-2 whitespace-nowrap">Timestamp</th>
                <th className="text-left px-4 py-2">Event Type</th>
                <th className="text-left px-4 py-2">Agent</th>
                <th className="text-left px-4 py-2">Session</th>
                <th className="text-left px-4 py-2">Metadata</th>
                <th className="text-left px-4 py-2">LGPD</th>
              </tr>
            </thead>
            <tbody>
              {filtered.length === 0 ? (
                <tr>
                  <td colSpan={6} className="px-4 py-8 text-center text-[#8b949e]">No entries match current filters.</td>
                </tr>
              ) : filtered.map((entry) => (
                <tr key={entry.id} className="border-b border-[#21262d] hover:bg-[#21262d] transition-colors">
                  <td className="px-4 py-2 font-mono text-xs text-[#8b949e] whitespace-nowrap">
                    {fmtTs(entry.timestamp)}
                  </td>
                  <td className="px-4 py-2">
                    <span className="font-mono text-xs text-[#bc8cff]">{entry.event_type}</span>
                  </td>
                  <td className="px-4 py-2 font-mono text-xs text-[#58a6ff]">{entry.agent}</td>
                  <td className="px-4 py-2 font-mono text-xs text-[#8b949e]">{entry.session}</td>
                  <td className="px-4 py-2 max-w-xs truncate">
                    <span className="font-mono text-xs text-[#8b949e]" title={JSON.stringify(entry.metadata)}>
                      {Object.entries(entry.metadata).map(([k, v]) => `${k}=${v}`).join(' ')}
                    </span>
                  </td>
                  <td className="px-4 py-2">{lgpdBadge(entry.lgpd_compliant)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}

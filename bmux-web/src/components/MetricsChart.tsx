'use client'

/**
 * MetricsChart — Story 5.1
 *
 * Real-time metrics visualisation using Recharts:
 *   • Line chart: tokens per minute per agent (one colored line per agent)
 *   • Line chart: cumulative cost (USD) over time
 *   • Summary cards: total tokens, total cost, active agents, completed tasks
 *   • Tooltip on hover with exact values
 *   • Updates via WebSocket `agent_tokens_updated` events
 */

import { useState, useEffect, useCallback } from 'react'
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip,
  Legend, ResponsiveContainer,
} from 'recharts'
import type { TokenDataPoint, BmuxEvent } from '@/lib/types'
import { MOCK_METRICS, MOCK_AGENTS, MOCK_TASKS } from '@/lib/mockData'
import { useBmuxSocket } from '@/lib/useBmuxSocket'

// Colour palette per agent (cycles if >8 agents)
const AGENT_COLORS = [
  '#58a6ff', '#3fb950', '#bc8cff', '#d29922',
  '#f85149', '#39d353', '#ff7b72', '#79c0ff',
]

interface Props {
  session: string | null
}

// ── Helpers ────────────────────────────────────────────────────────────────────

function fmtTime(ts: number) {
  return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function fmtCost(v: number) {
  return `$${v.toFixed(4)}`
}

function fmtTokens(v: number) {
  return v >= 1000 ? `${(v / 1000).toFixed(1)}k` : String(v)
}

// ── Component ──────────────────────────────────────────────────────────────────

export function MetricsChart({ session }: Props) {
  const [dataPoints, setDataPoints] = useState<TokenDataPoint[]>(MOCK_METRICS)
  const [agentNames, setAgentNames] = useState<string[]>(
    ['arquiteto', 'testador'] as string[]
  )

  // Accumulate per-agent cost deltas from WebSocket events
  const handleEvent = useCallback((event: BmuxEvent) => {
    if (event.type !== 'agent_tokens_updated') return
    const now = Date.now()
    setDataPoints((prev) => {
      const last = prev.filter((d) => d.agent === event.agent_id).at(-1)
      const newPoint: TokenDataPoint = {
        timestamp:          new Date(now).toISOString(),
        agent:              event.agent_id,
        tokens_per_minute:  event.tokens,
        cumulative_tokens:  (Number(last?.cumulative_tokens) || 0) + event.tokens,
        cumulative_cost_usd: (Number(last?.cumulative_cost_usd) || 0) + event.cost_usd,
      }
      // Keep last 120 data points per agent
      const pruned = prev.filter((d) => d.agent !== event.agent_id).concat(
        [...prev.filter((d) => d.agent === event.agent_id), newPoint].slice(-120)
      )
      return pruned
    })
    setAgentNames((prev) => prev.includes(event.agent_id) ? prev : [...prev, event.agent_id])
  }, [])

  useBmuxSocket(session, handleEvent)

  // ── Reshape for Recharts ────────────────────────────────────────────────────
  // Recharts expects one row per timestamp with all agents as keys.
  const timestamps = Array.from(new Set(dataPoints.map((d) => d.timestamp))).sort()

  const tpmRows = timestamps.map((ts) => {
    const row: Record<string, number | string> = { ts }
    for (const name of agentNames) {
      const pt = dataPoints.find((d) => d.timestamp === ts && d.agent === name)
      row[name] = pt?.tokens_per_minute ?? 0
    }
    return row
  })

  const costRows = timestamps.map((ts) => {
    const row: Record<string, number | string> = { ts, total: 0 }
    for (const name of agentNames) {
      const pt = dataPoints.find((d) => d.timestamp === ts && d.agent === name)
      row[name] = pt?.cumulative_cost_usd ?? 0
      row['total'] = (Number(row['total']) || 0) + (Number(row[name]) || 0)
    }
    return row
  })

  // ── Summary totals ──────────────────────────────────────────────────────────
  const totalTokens = MOCK_AGENTS.reduce((s, a) => s + a.tokens_used, 0)
  const totalCost   = MOCK_AGENTS.reduce((s, a) => s + a.cost_usd, 0)
  const activeAgents = MOCK_AGENTS.filter((a) => a.status === 'working').length
  const completedTasks = MOCK_TASKS.filter((t) => t.status === 'completed').length

  return (
    <div className="flex flex-col gap-6">
      {/* Summary cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <SummaryCard label="Total Tokens"    value={fmtTokens(totalTokens)} color="#58a6ff" />
        <SummaryCard label="Total Cost"      value={fmtCost(totalCost)}     color="#3fb950" />
        <SummaryCard label="Active Agents"   value={String(activeAgents)}   color="#bc8cff" />
        <SummaryCard label="Completed Tasks" value={String(completedTasks)} color="#d29922" />
      </div>

      {/* Tokens / min chart */}
      <ChartCard title="Tokens per Minute (by Agent)">
        <ResponsiveContainer width="100%" height={260}>
          <LineChart data={tpmRows} margin={{ top: 5, right: 20, left: 0, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#30363d" />
            <XAxis
              dataKey="ts"
              tickFormatter={fmtTime}
              tick={{ fill: '#8b949e', fontSize: 11 }}
              stroke="#30363d"
            />
            <YAxis
              tickFormatter={fmtTokens}
              tick={{ fill: '#8b949e', fontSize: 11 }}
              stroke="#30363d"
            />
            <Tooltip
              contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6 }}
              labelStyle={{ color: '#8b949e', fontSize: 11 }}
              labelFormatter={(v) => fmtTime(v as number)}
              formatter={(v, name) => [fmtTokens(v as number), name]}
            />
            <Legend wrapperStyle={{ color: '#8b949e', fontSize: 12 }} />
            {agentNames.map((name, i) => (
              <Line
                key={name}
                type="monotone"
                dataKey={name}
                stroke={AGENT_COLORS[i % AGENT_COLORS.length]}
                dot={false}
                strokeWidth={2}
              />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </ChartCard>

      {/* Cumulative cost chart */}
      <ChartCard title="Cumulative Cost (USD)">
        <ResponsiveContainer width="100%" height={260}>
          <LineChart data={costRows} margin={{ top: 5, right: 20, left: 0, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#30363d" />
            <XAxis
              dataKey="ts"
              tickFormatter={fmtTime}
              tick={{ fill: '#8b949e', fontSize: 11 }}
              stroke="#30363d"
            />
            <YAxis
              tickFormatter={fmtCost}
              tick={{ fill: '#8b949e', fontSize: 11 }}
              stroke="#30363d"
            />
            <Tooltip
              contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6 }}
              labelStyle={{ color: '#8b949e', fontSize: 11 }}
              labelFormatter={(v) => fmtTime(v as number)}
              formatter={(v, name) => [fmtCost(v as number), name]}
            />
            <Legend wrapperStyle={{ color: '#8b949e', fontSize: 12 }} />
            {agentNames.map((name, i) => (
              <Line
                key={name}
                type="monotone"
                dataKey={name}
                stroke={AGENT_COLORS[i % AGENT_COLORS.length]}
                dot={false}
                strokeWidth={2}
              />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </ChartCard>
    </div>
  )
}

// ── Sub-components ─────────────────────────────────────────────────────────────

function SummaryCard({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="bg-[#161b22] border border-[#30363d] rounded-lg p-4">
      <div className="text-xs text-[#8b949e] mb-1">{label}</div>
      <div className="text-2xl font-bold font-mono" style={{ color }}>{value}</div>
    </div>
  )
}

function ChartCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="bg-[#161b22] border border-[#30363d] rounded-lg p-4">
      <h3 className="text-sm font-semibold text-[#e6edf3] mb-4">{title}</h3>
      {children}
    </div>
  )
}

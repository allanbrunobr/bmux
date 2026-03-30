'use client'

/**
 * CostBreakdown — Story 5.3
 *
 * Cost analytics dashboard:
 *   • Cost per agent table (name, model, tokens, cost USD, cost/hour)
 *   • Bar chart: cost per hour by agent
 *   • Session total + projected daily / monthly cost
 */

import { useState, useCallback } from 'react'
import {
  BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip,
  ResponsiveContainer, Cell,
} from 'recharts'
import type { Agent, BmuxEvent } from '@/lib/types'
import { MOCK_AGENTS } from '@/lib/mockData'
import { useBmuxSocket } from '@/lib/useBmuxSocket'

const AGENT_COLORS = [
  '#58a6ff', '#3fb950', '#bc8cff', '#d29922',
  '#f85149', '#39d353', '#ff7b72', '#79c0ff',
]

function fmtCost(v: number) { return `$${v.toFixed(4)}` }
function fmtTokens(v: number) { return v >= 1000 ? `${(v / 1000).toFixed(1)}k` : String(v) }

interface Props {
  session: string | null
}

export function CostBreakdown({ session }: Props) {
  const [agents, setAgents] = useState<Agent[]>(MOCK_AGENTS)

  const handleEvent = useCallback((event: BmuxEvent) => {
    switch (event.type) {
      case 'agent_updated':
      case 'agent_spawned':
        setAgents((prev) => {
          const idx = prev.findIndex((a) => a.name === event.agent.name)
          if (idx >= 0) {
            const next = [...prev]
            next[idx] = event.agent
            return next
          }
          return [...prev, event.agent]
        })
        break
      case 'agent_tokens_updated':
        setAgents((prev) =>
          prev.map((a) =>
            a.name === event.name
              ? { ...a, tokens_total: a.tokens_total + event.tokens_delta, cost_usd: a.cost_usd + event.cost_delta_usd }
              : a
          )
        )
        break
    }
  }, [])

  useBmuxSocket(session, handleEvent)

  // Derived totals
  const sessionTotal = agents.reduce((s, a) => s + a.cost_usd, 0)
  const sessionHours = agents.reduce((s, a) => s + a.uptime_secs / 3600, 0) / agents.length || 1
  const costPerHour  = sessionTotal / sessionHours
  const projectedDay   = costPerHour * 24
  const projectedMonth = projectedDay * 30

  // Per-agent cost/hour
  const agentRows = agents.map((a) => {
    const hours = a.uptime_secs / 3600 || 0.001
    return { ...a, cost_per_hour: a.cost_usd / hours }
  })

  const barData = agentRows.map((a) => ({
    name:      a.name,
    cost_hour: parseFloat(a.cost_per_hour.toFixed(5)),
  }))

  return (
    <div className="flex flex-col gap-6">
      {/* Projection cards */}
      <div className="grid grid-cols-3 gap-4">
        <ProjectionCard label="Session Total"     value={fmtCost(sessionTotal)}   color="#58a6ff" />
        <ProjectionCard label="Projected / Day"   value={fmtCost(projectedDay)}   color="#d29922" />
        <ProjectionCard label="Projected / Month" value={fmtCost(projectedMonth)} color="#f85149" />
      </div>

      {/* Cost per agent table */}
      <div className="bg-[#161b22] border border-[#30363d] rounded-lg overflow-hidden">
        <div className="px-4 py-3 border-b border-[#30363d]">
          <h3 className="text-sm font-semibold text-[#e6edf3]">Cost per Agent</h3>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-[#8b949e] text-xs border-b border-[#30363d]">
                <th className="text-left px-4 py-2">Agent</th>
                <th className="text-left px-4 py-2">Model</th>
                <th className="text-right px-4 py-2">Tokens</th>
                <th className="text-right px-4 py-2">Cost (USD)</th>
                <th className="text-right px-4 py-2">Cost / hr</th>
              </tr>
            </thead>
            <tbody>
              {agentRows.map((a, i) => (
                <tr key={a.name} className="border-b border-[#21262d] hover:bg-[#21262d] transition-colors">
                  <td className="px-4 py-2">
                    <div className="flex items-center gap-2">
                      <span
                        className="w-2 h-2 rounded-full"
                        style={{ background: AGENT_COLORS[i % AGENT_COLORS.length] }}
                      />
                      <span className="font-mono text-[#58a6ff]">{a.name}</span>
                    </div>
                  </td>
                  <td className="px-4 py-2 text-[#8b949e] font-mono text-xs">{a.model}</td>
                  <td className="px-4 py-2 text-right font-mono">{fmtTokens(a.tokens_total)}</td>
                  <td className="px-4 py-2 text-right font-mono text-[#3fb950]">{fmtCost(a.cost_usd)}</td>
                  <td className="px-4 py-2 text-right font-mono text-[#d29922]">{fmtCost(a.cost_per_hour)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Cost/hour bar chart */}
      <div className="bg-[#161b22] border border-[#30363d] rounded-lg p-4">
        <h3 className="text-sm font-semibold text-[#e6edf3] mb-4">Cost per Hour by Agent</h3>
        <ResponsiveContainer width="100%" height={220}>
          <BarChart data={barData} margin={{ top: 5, right: 20, left: 0, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#30363d" vertical={false} />
            <XAxis dataKey="name" tick={{ fill: '#8b949e', fontSize: 11 }} stroke="#30363d" />
            <YAxis tickFormatter={fmtCost} tick={{ fill: '#8b949e', fontSize: 11 }} stroke="#30363d" />
            <Tooltip
              contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6 }}
              labelStyle={{ color: '#e6edf3' }}
              formatter={(v) => [fmtCost(v as number), 'Cost/hr']}
            />
            <Bar dataKey="cost_hour" radius={[4, 4, 0, 0]}>
              {barData.map((_entry, i) => (
                <Cell key={i} fill={AGENT_COLORS[i % AGENT_COLORS.length]} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  )
}

function ProjectionCard({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="bg-[#161b22] border border-[#30363d] rounded-lg p-4">
      <div className="text-xs text-[#8b949e] mb-1">{label}</div>
      <div className="text-xl font-bold font-mono" style={{ color }}>{value}</div>
    </div>
  )
}

'use client'

/**
 * CostPerConsulta — Story 6.2 (AFYA Healthcare Use Case)
 *
 * Groups agent costs by consultation session using context keys:
 *   consultation:{id}:transcription_cost
 *   consultation:{id}:soap_note_cost
 *
 * Displays:
 *   • Per-consultation breakdown: transcription cost + SOAP note cost + total
 *   • Monthly cost projection based on observed daily average
 *   • Real-time updates from context_updated WebSocket events
 */

import { useState, useCallback, useMemo } from 'react'
import {
  BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip,
  ResponsiveContainer, Legend,
} from 'recharts'
import type { BmuxEvent, ContextEntry, ConsultationCost } from '@/lib/types'
import { MOCK_CONSULTATIONS } from '@/lib/mockData'
import { useBmuxSocket } from '@/lib/useBmuxSocket'

function fmtCost(v: number) { return `$${v.toFixed(4)}` }
function fmtDate(ts: string) {
  return new Date(ts).toLocaleDateString([], { month: 'short', day: '2-digit' })
}

interface Props {
  session: string | null
}

// Parse consultation context entries into ConsultationCost records.
function applyContextEntry(
  prev: ConsultationCost[],
  entry: ContextEntry
): ConsultationCost[] {
  // Match keys: consultation:{id}:transcription_cost or consultation:{id}:soap_note_cost
  const match = entry.key.match(/^consultation:([^:]+):(transcription_cost|soap_note_cost)$/)
  if (!match) return prev

  const [, id, field] = match
  const value = parseFloat(entry.value) || 0

  const existing = prev.find((c) => c.consultation_id === id)
  if (existing) {
    const updated = {
      ...existing,
      [field === 'transcription_cost' ? 'transcription_cost_usd' : 'soap_note_cost_usd']: value,
    }
    updated.total_cost_usd = updated.transcription_cost_usd + updated.soap_note_cost_usd
    return prev.map((c) => c.consultation_id === id ? updated : c)
  }

  const newEntry: ConsultationCost = {
    consultation_id:       id,
    transcription_cost_usd: field === 'transcription_cost' ? value : 0,
    soap_note_cost_usd:     field === 'soap_note_cost'     ? value : 0,
    total_cost_usd:         value,
    timestamp:              entry.updated_at,
    agent:                  entry.updated_by ?? 'unknown',
  }
  return [...prev, newEntry]
}

export function CostPerConsulta({ session }: Props) {
  const [consultations, setConsultations] = useState<ConsultationCost[]>(MOCK_CONSULTATIONS)

  const handleEvent = useCallback((event: BmuxEvent) => {
    if (event.type === 'context_updated') {
      setConsultations((prev) => applyContextEntry(prev, event.entry as ContextEntry))
    }
  }, [])

  useBmuxSocket(session, handleEvent)

  // Totals
  const totalCost  = consultations.reduce((s, c) => s + c.total_cost_usd, 0)
  const totalTx    = consultations.reduce((s, c) => s + c.transcription_cost_usd, 0)
  const totalSoap  = consultations.reduce((s, c) => s + c.soap_note_cost_usd, 0)
  const avgPerDay  = consultations.length > 0
    ? totalCost / Math.max(uniqueDays(consultations), 1)
    : 0
  const projMonth  = avgPerDay * 30

  // Bar chart data
  const barData = consultations.map((c) => ({
    id:            c.consultation_id,
    transcription: parseFloat(c.transcription_cost_usd.toFixed(5)),
    soap_note:     parseFloat(c.soap_note_cost_usd.toFixed(5)),
  }))

  return (
    <div className="flex flex-col gap-6">
      {/* Summary */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <SummaryCard label="Total Cost"          value={fmtCost(totalCost)}  color="#58a6ff" />
        <SummaryCard label="Transcription Total" value={fmtCost(totalTx)}    color="#3fb950" />
        <SummaryCard label="SOAP Note Total"     value={fmtCost(totalSoap)}  color="#bc8cff" />
        <SummaryCard label="Projected / Month"   value={fmtCost(projMonth)}  color="#d29922" />
      </div>

      {/* Bar chart */}
      <div className="bg-[#161b22] border border-[#30363d] rounded-lg p-4">
        <h3 className="text-sm font-semibold text-[#e6edf3] mb-4">Cost Breakdown per Consultation</h3>
        <ResponsiveContainer width="100%" height={240}>
          <BarChart data={barData} margin={{ top: 5, right: 20, left: 0, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#30363d" vertical={false} />
            <XAxis dataKey="id" tick={{ fill: '#8b949e', fontSize: 10 }} stroke="#30363d" />
            <YAxis tickFormatter={fmtCost} tick={{ fill: '#8b949e', fontSize: 11 }} stroke="#30363d" />
            <Tooltip
              contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6 }}
              labelStyle={{ color: '#e6edf3' }}
              formatter={(v, name) => [fmtCost(v as number), name === 'transcription' ? 'Transcription' : 'SOAP Note']}
            />
            <Legend
              wrapperStyle={{ color: '#8b949e', fontSize: 12 }}
              formatter={(value) => value === 'transcription' ? 'Transcription' : 'SOAP Note'}
            />
            <Bar dataKey="transcription" stackId="a" fill="#3fb950" radius={[0, 0, 0, 0]} />
            <Bar dataKey="soap_note"     stackId="a" fill="#58a6ff" radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>

      {/* Detail table */}
      <div className="bg-[#161b22] border border-[#30363d] rounded-lg overflow-hidden">
        <div className="px-4 py-3 border-b border-[#30363d] flex items-center justify-between">
          <h3 className="text-sm font-semibold text-[#e6edf3]">Per-Consultation Detail</h3>
          <span className="text-xs text-[#8b949e]">{consultations.length} consultations</span>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-[#8b949e] text-xs border-b border-[#30363d]">
                <th className="text-left px-4 py-2">Consultation ID</th>
                <th className="text-left px-4 py-2">Date</th>
                <th className="text-right px-4 py-2">Transcription</th>
                <th className="text-right px-4 py-2">SOAP Note</th>
                <th className="text-right px-4 py-2">Total</th>
              </tr>
            </thead>
            <tbody>
              {consultations.map((c) => (
                <tr key={c.consultation_id} className="border-b border-[#21262d] hover:bg-[#21262d] transition-colors">
                  <td className="px-4 py-2 font-mono text-xs text-[#bc8cff]">{c.consultation_id}</td>
                  <td className="px-4 py-2 text-xs text-[#8b949e]">{fmtDate(c.timestamp)}</td>
                  <td className="px-4 py-2 text-right font-mono text-[#3fb950]">{fmtCost(c.transcription_cost_usd)}</td>
                  <td className="px-4 py-2 text-right font-mono text-[#58a6ff]">{fmtCost(c.soap_note_cost_usd)}</td>
                  <td className="px-4 py-2 text-right font-mono font-semibold">{fmtCost(c.total_cost_usd)}</td>
                </tr>
              ))}
            </tbody>
            <tfoot>
              <tr className="border-t border-[#30363d] bg-[#0d1117]">
                <td colSpan={2} className="px-4 py-2 text-xs text-[#8b949e] font-semibold">TOTAL</td>
                <td className="px-4 py-2 text-right font-mono text-[#3fb950] font-semibold">{fmtCost(totalTx)}</td>
                <td className="px-4 py-2 text-right font-mono text-[#58a6ff] font-semibold">{fmtCost(totalSoap)}</td>
                <td className="px-4 py-2 text-right font-mono font-bold text-[#e6edf3]">{fmtCost(totalCost)}</td>
              </tr>
            </tfoot>
          </table>
        </div>
      </div>
    </div>
  )
}

function SummaryCard({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="bg-[#161b22] border border-[#30363d] rounded-lg p-4">
      <div className="text-xs text-[#8b949e] mb-1">{label}</div>
      <div className="text-xl font-bold font-mono" style={{ color }}>{value}</div>
    </div>
  )
}

function uniqueDays(consultations: ConsultationCost[]): number {
  const days = new Set(consultations.map((c) => c.timestamp.slice(0, 10)))
  return days.size
}

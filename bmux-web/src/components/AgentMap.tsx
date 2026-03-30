'use client'

/**
 * AgentMap — Story 5.2
 *
 * World map using deck.gl:
 *   • ScatterplotLayer: pins at agent geographic locations
 *     - Local agents → user's approximate location
 *     - Remote agents → cloud region coordinates
 *   • ArcLayer: animated arcs when tasks are sent between agents
 *     - Arc color derived from the source agent's provider
 */

import { useState, useCallback, useEffect } from 'react'
import DeckGL from '@deck.gl/react'
import { ScatterplotLayer, ArcLayer } from '@deck.gl/layers'
import type { Agent, Task, BmuxEvent } from '@/lib/types'
import { MOCK_AGENTS, MOCK_TASKS } from '@/lib/mockData'
import { useBmuxSocket } from '@/lib/useBmuxSocket'

// Cloud region → approximate coordinates
const REGION_COORDS: Record<string, [number, number]> = {
  'us-east-1':      [-78.45,  38.13],
  'us-east-2':      [-83.00,  39.96],
  'us-west-1':      [-121.96, 37.35],
  'us-west-2':      [-120.50, 46.15],
  'eu-west-1':      [ -8.24,  53.33],
  'eu-west-2':      [ -0.13,  51.51],
  'eu-central-1':   [ 8.68,   50.11],
  'ap-southeast-1': [103.82,   1.35],
  'ap-northeast-1': [139.69,  35.69],
  'sa-east-1':      [-46.63, -23.55],
}

const PROVIDER_COLORS: Record<string, [number, number, number]> = {
  anthropic: [88,  166, 255],
  openai:    [ 26, 127,  55],
  google:    [188,  40, 255],
  local:     [ 63, 185,  80],
  shell:     [210, 153,  34],
}

// Default fallback if user location unknown
const DEFAULT_LOCAL: [number, number] = [-46.63, -23.55]

// View state — centred on the world
const INITIAL_VIEW = {
  longitude: -10,
  latitude:  20,
  zoom:       1.5,
  pitch:      30,
  bearing:    0,
}

interface ScatterData {
  coordinates: [number, number]
  name: string
  color: [number, number, number]
  radius: number
}

interface ArcData {
  source: [number, number]
  target: [number, number]
  color: [number, number, number]
  id: string
}

// ── Helper ─────────────────────────────────────────────────────────────────────

function agentToCoords(agent: Agent): [number, number] {
  if (!agent.location) return DEFAULT_LOCAL
  if (agent.location.region && REGION_COORDS[agent.location.region]) {
    return REGION_COORDS[agent.location.region]
  }
  return [agent.location.lon ?? 0, agent.location.lat ?? 0]
}

function agentColor(agent: Agent): [number, number, number] {
  return PROVIDER_COLORS[agent.location?.provider ?? ''] ?? [100, 100, 100]
}

// ── Component ──────────────────────────────────────────────────────────────────

interface Props {
  session: string | null
}

export function AgentMap({ session }: Props) {
  const [agents, setAgents]           = useState<Agent[]>(MOCK_AGENTS)
  const [activeArcs, setActiveArcs]   = useState<ArcData[]>([])

  const handleEvent = useCallback((event: BmuxEvent) => {
    switch (event.type) {
      case 'agent_spawned':
        setAgents((prev) => event.type === 'agent_spawned' ? [...prev, event.agent] : prev)
        break
      case 'agent_tokens_updated':
        setAgents((prev) => prev.map((a) => a.name === event.agent_id ? { ...a, tokens_used: event.tokens, cost_usd: event.cost_usd } : a))
        break
      case 'agent_killed':
        setAgents((prev) => prev.filter((a) => a.name !== event.agent_id))
        break
      case 'task_created':
      case 'task_updated': {
        // Draw an arc from source agent to target agent
        const task = event.task as Task
        const src = agents.find((a) => a.name === task.from_agent)
        const dst = agents.find((a) => a.name === task.to_agent)
        if (src && dst) {
          const arc: ArcData = {
            id:     task.id,
            source: agentToCoords(src),
            target: agentToCoords(dst),
            color:  agentColor(src),
          }
          setActiveArcs((prev) => [...prev.slice(-19), arc]) // keep last 20 arcs
        }
        break
      }
    }
  }, [agents])

  useBmuxSocket(session, handleEvent)

  // Fade out arcs after 5 s
  useEffect(() => {
    if (activeArcs.length === 0) return
    const t = setTimeout(() => {
      setActiveArcs((prev) => prev.slice(1))
    }, 5000)
    return () => clearTimeout(t)
  }, [activeArcs])

  // Build layer data
  const scatterData: ScatterData[] = agents.map((a) => ({
    coordinates: agentToCoords(a),
    name:        a.name,
    color:       agentColor(a),
    radius:      a.status === 'working' ? 80000 : 50000,
  }))

  const layers = [
    new ScatterplotLayer<ScatterData>({
      id:                'agents',
      data:              scatterData,
      getPosition:       (d) => d.coordinates,
      getFillColor:      (d) => [...d.color, 200] as [number,number,number,number],
      getRadius:         (d) => d.radius,
      radiusMinPixels:   6,
      radiusMaxPixels:   20,
      pickable:          true,
    }),
    new ArcLayer<ArcData>({
      id:             'task-arcs',
      data:           activeArcs,
      getSourcePosition: (d) => d.source,
      getTargetPosition: (d) => d.target,
      getSourceColor:    (d) => [...d.color, 180] as [number,number,number,number],
      getTargetColor:    (d) => [...d.color,  80] as [number,number,number,number],
      getWidth:          2,
    }),
  ]

  return (
    <div className="bg-[#161b22] border border-[#30363d] rounded-lg overflow-hidden">
      <div className="px-4 py-3 border-b border-[#30363d]">
        <h3 className="text-sm font-semibold text-[#e6edf3]">Geographic Agent Distribution</h3>
        <p className="text-xs text-[#8b949e] mt-1">Arcs animate when tasks are dispatched between agents</p>
      </div>
      <div style={{ height: 420, position: 'relative' }}>
        <DeckGL
          initialViewState={INITIAL_VIEW}
          controller
          layers={layers}
          style={{ background: '#0d1117' }}
          getTooltip={({ object }) => {
            const d = object as ScatterData | undefined
            return d ? { html: `<b>${d.name}</b>`, style: { background: '#161b22', border: '1px solid #30363d', borderRadius: '4px', color: '#e6edf3', fontSize: '12px' } } : null
          }}
        />
      </div>
      {/* Legend */}
      <div className="flex flex-wrap gap-3 px-4 py-3 border-t border-[#30363d]">
        {Object.entries(PROVIDER_COLORS).map(([provider, [r, g, b]]) => (
          <div key={provider} className="flex items-center gap-1.5">
            <span className="w-3 h-3 rounded-full inline-block" style={{ background: `rgb(${r},${g},${b})` }} />
            <span className="text-xs text-[#8b949e] capitalize">{provider}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

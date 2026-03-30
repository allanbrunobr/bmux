"use client"

import { useBmuxStore } from '@/lib/store'
import { AgentCard } from '@/components/AgentCard'
import { Bot } from 'lucide-react'

export function AgentsGrid() {
  const agents = useBmuxStore((s) => s.agents)

  if (agents.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-20 text-center">
        <Bot className="h-12 w-12 text-muted-foreground/40 mb-4" />
        <h3 className="text-sm font-medium text-muted-foreground">No agents running</h3>
        <p className="text-xs text-muted-foreground/60 mt-1">
          Agents will appear here when spawned in the active session.
        </p>
      </div>
    )
  }

  return (
    <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
      {agents.map((agent) => (
        <AgentCard key={agent.id} agent={agent} />
      ))}
    </div>
  )
}

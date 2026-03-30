"use client"

import { AgentsGrid } from '@/components/AgentsGrid'
import { useBmuxStore } from '@/lib/store'

export default function AgentsPage() {
  const agents = useBmuxStore((s) => s.agents)
  const activeSession = useBmuxStore((s) => s.activeSession)

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-foreground">Agents</h2>
          {activeSession && (
            <p className="text-sm text-muted-foreground mt-0.5">
              {agents.length} agent{agents.length !== 1 ? 's' : ''} in{' '}
              <span className="font-medium text-foreground">{activeSession}</span>
            </p>
          )}
        </div>
      </div>
      <AgentsGrid />
    </div>
  )
}

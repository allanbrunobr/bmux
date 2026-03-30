"use client"

import { useEffect } from 'react'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useBmuxStore } from '@/lib/store'

export function SessionSelector() {
  const sessions = useBmuxStore((s) => s.sessions)
  const activeSession = useBmuxStore((s) => s.activeSession)
  const setActiveSession = useBmuxStore((s) => s.setActiveSession)

  // Auto-select if only one session available
  useEffect(() => {
    if (sessions.length === 1 && !activeSession) {
      setActiveSession(sessions[0].name)
    }
  }, [sessions, activeSession, setActiveSession])

  if (sessions.length === 0) {
    return (
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <span>No sessions</span>
      </div>
    )
  }

  return (
    <Select
      value={activeSession ?? undefined}
      onValueChange={setActiveSession}
    >
      <SelectTrigger className="w-48 h-8 text-xs">
        <SelectValue placeholder="Select session..." />
      </SelectTrigger>
      <SelectContent>
        {sessions.map((session) => (
          <SelectItem key={session.name} value={session.name}>
            <span className="font-medium">{session.name}</span>
            <span className="ml-2 text-muted-foreground">
              ({session.agents} agents)
            </span>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

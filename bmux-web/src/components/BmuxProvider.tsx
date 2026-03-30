"use client"

import { useEffect, useCallback } from 'react'
import { useBmuxStore } from '@/lib/store'
import { bmuxClient } from '@/lib/bmux-client'
import { useBmuxWebSocket } from '@/hooks/useBmuxWebSocket'
import type { BmuxEvent } from '@/lib/types'

export function BmuxProvider({ children }: { children: React.ReactNode }) {
  const setSessions = useBmuxStore((s) => s.setSessions)
  const setActiveSession = useBmuxStore((s) => s.setActiveSession)
  const activeSession = useBmuxStore((s) => s.activeSession)
  const sessions = useBmuxStore((s) => s.sessions)
  const setAgents = useBmuxStore((s) => s.setAgents)
  const setContextEntries = useBmuxStore((s) => s.setContextEntries)
  const setTasks = useBmuxStore((s) => s.setTasks)
  const setMetrics = useBmuxStore((s) => s.setMetrics)
  const setConnected = useBmuxStore((s) => s.setConnected)
  const handleEvent = useBmuxStore((s) => s.handleEvent)

  // Load sessions on mount
  useEffect(() => {
    let cancelled = false
    async function loadSessions() {
      try {
        const data = await bmuxClient.getSessions()
        if (!cancelled) {
          setSessions(data)
          // Auto-select if only one session
          if (data.length === 1) {
            setActiveSession(data[0].name)
          }
        }
      } catch (err) {
        console.error('[BmuxProvider] Failed to load sessions:', err)
      }
    }
    void loadSessions()
    return () => { cancelled = true }
  }, [setSessions, setActiveSession])

  // Load session data when active session changes
  useEffect(() => {
    if (!activeSession) return
    let cancelled = false

    async function loadSessionData() {
      try {
        const [agents, context, tasks, metrics] = await Promise.all([
          bmuxClient.getAgents(activeSession!),
          bmuxClient.getContext(activeSession!),
          bmuxClient.getTasks(activeSession!),
          bmuxClient.getMetrics(activeSession!),
        ])
        if (!cancelled) {
          setAgents(agents)
          setContextEntries(context)
          setTasks(tasks)
          setMetrics(metrics)
        }
      } catch (err) {
        console.error('[BmuxProvider] Failed to load session data:', err)
      }
    }

    void loadSessionData()
    return () => { cancelled = true }
  }, [activeSession, setAgents, setContextEntries, setTasks, setMetrics])

  const onEvent = useCallback(
    (event: BmuxEvent) => {
      handleEvent(event)
    },
    [handleEvent]
  )

  const onConnected = useCallback(
    (connected: boolean) => {
      setConnected(connected)
    },
    [setConnected]
  )

  // WebSocket connection
  useBmuxWebSocket({
    session: activeSession,
    onEvent,
    onConnected,
  })

  return <>{children}</>
}

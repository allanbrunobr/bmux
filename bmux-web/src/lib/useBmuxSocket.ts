'use client'

import { useEffect, useRef, useCallback } from 'react'
import type { BmuxEvent } from './types'
import { wsUrl } from './api'

type Handler = (event: BmuxEvent) => void

export function useBmuxSocket(session: string | null, onEvent: Handler) {
  const wsRef = useRef<WebSocket | null>(null)
  const handlerRef = useRef(onEvent)
  handlerRef.current = onEvent

  const connect = useCallback(() => {
    if (!session) return
    const ws = new WebSocket(wsUrl(session))
    wsRef.current = ws

    ws.onmessage = (msg) => {
      try {
        const event = JSON.parse(msg.data as string) as BmuxEvent
        handlerRef.current(event)
      } catch {
        // ignore malformed frames
      }
    }

    ws.onclose = () => {
      // Auto-reconnect after 2 s
      setTimeout(() => {
        if (wsRef.current === ws) connect()
      }, 2000)
    }
  }, [session])

  useEffect(() => {
    connect()
    return () => {
      wsRef.current?.close()
      wsRef.current = null
    }
  }, [connect])
}

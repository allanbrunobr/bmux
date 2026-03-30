"use client";

import { useEffect, useRef, useCallback } from 'react';
import type { BmuxEvent } from '@/lib/types';

const API_BASE = process.env.NEXT_PUBLIC_BMUX_API || 'http://localhost:7432';

interface UseBmuxWebSocketOptions {
  session: string | null;
  onEvent: (event: BmuxEvent) => void;
  onConnected: (connected: boolean) => void;
}

interface UseBmuxWebSocketReturn {
  isConnected: boolean;
}

export function useBmuxWebSocket({
  session,
  onEvent,
  onConnected,
}: UseBmuxWebSocketOptions): UseBmuxWebSocketReturn {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const backoffRef = useRef<number>(1000);
  const isMountedRef = useRef<boolean>(true);
  const isConnectedRef = useRef<boolean>(false);

  const clearReconnectTimeout = useCallback(() => {
    if (reconnectTimeoutRef.current !== null) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
  }, []);

  const connect = useCallback(() => {
    if (!session || !isMountedRef.current) return;

    // Derive WebSocket URL from API base
    const wsBase = API_BASE.replace(/^http/, 'ws');
    const url = `${wsBase}/ws?session=${encodeURIComponent(session)}`;

    try {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        if (!isMountedRef.current) {
          ws.close();
          return;
        }
        backoffRef.current = 1000;
        isConnectedRef.current = true;
        onConnected(true);
      };

      ws.onmessage = (event) => {
        if (!isMountedRef.current) return;
        try {
          const parsed = JSON.parse(event.data as string) as BmuxEvent;
          onEvent(parsed);
        } catch (err) {
          console.warn('[useBmuxWebSocket] Failed to parse message:', err);
        }
      };

      ws.onerror = () => {
        // onclose will fire after onerror, handle there
      };

      ws.onclose = () => {
        if (!isMountedRef.current) return;

        if (isConnectedRef.current) {
          isConnectedRef.current = false;
          onConnected(false);
        }

        // Exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s max
        const delay = Math.min(backoffRef.current, 30000);
        backoffRef.current = Math.min(backoffRef.current * 2, 30000);

        reconnectTimeoutRef.current = setTimeout(() => {
          if (isMountedRef.current) {
            connect();
          }
        }, delay);
      };
    } catch (err) {
      console.warn('[useBmuxWebSocket] Failed to create WebSocket:', err);
      // Schedule reconnect
      const delay = Math.min(backoffRef.current, 30000);
      backoffRef.current = Math.min(backoffRef.current * 2, 30000);
      reconnectTimeoutRef.current = setTimeout(() => {
        if (isMountedRef.current) {
          connect();
        }
      }, delay);
    }
  }, [session, onEvent, onConnected]);

  useEffect(() => {
    isMountedRef.current = true;

    if (!session) {
      return;
    }

    connect();

    return () => {
      isMountedRef.current = false;
      clearReconnectTimeout();

      if (wsRef.current) {
        wsRef.current.onclose = null; // Prevent reconnect on intentional close
        wsRef.current.close();
        wsRef.current = null;
      }

      if (isConnectedRef.current) {
        isConnectedRef.current = false;
        onConnected(false);
      }
    };
  }, [session, connect, clearReconnectTimeout, onConnected]);

  return { isConnected: isConnectedRef.current };
}

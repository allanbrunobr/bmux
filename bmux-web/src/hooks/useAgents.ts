"use client";

import { useEffect } from 'react';
import { useBmuxStore } from '@/lib/store';
import { bmuxClient } from '@/lib/bmux-client';

export function useAgents() {
  const activeSession = useBmuxStore((s) => s.activeSession);
  const agents = useBmuxStore((s) => s.agents);
  const setAgents = useBmuxStore((s) => s.setAgents);

  useEffect(() => {
    if (!activeSession) return;

    let cancelled = false;

    async function loadAgents() {
      try {
        const data = await bmuxClient.getAgents(activeSession!);
        if (!cancelled) {
          setAgents(data);
        }
      } catch (err) {
        console.error('[useAgents] Failed to load agents:', err);
      }
    }

    void loadAgents();

    return () => {
      cancelled = true;
    };
  }, [activeSession, setAgents]);

  return { agents };
}

"use client"

import { useBmuxStore } from '@/lib/store'
import { AdversarialPanel } from '@/components/AdversarialPanel'

export function AdversarialPanelWrapper() {
  const adversarialOn = useBmuxStore((s) => s.adversarialOn)
  if (!adversarialOn) return null
  return <AdversarialPanel />
}

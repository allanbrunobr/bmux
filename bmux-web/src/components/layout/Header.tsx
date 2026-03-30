"use client"

import { Swords } from 'lucide-react'
import { SessionSelector } from '@/components/SessionSelector'
import { ConnectionStatus } from '@/components/ConnectionStatus'
import { Button } from '@/components/ui/button'
import { useBmuxStore } from '@/lib/store'
import { cn } from '@/lib/utils'

export function Header() {
  const { adversarialOn, setAdversarialOn } = useBmuxStore()

  return (
    <header className="flex h-14 items-center justify-between border-b border-border bg-background px-6 flex-shrink-0">
      {/* Left */}
      <div className="flex items-center gap-2">
        <h1 className="text-sm font-semibold text-foreground">BMUX Dashboard</h1>
      </div>

      {/* Center */}
      <div className="flex items-center">
        <SessionSelector />
      </div>

      {/* Right */}
      <div className="flex items-center gap-3">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setAdversarialOn(!adversarialOn)}
          className={cn(
            'flex items-center gap-1.5 text-xs h-8 px-3 transition-all',
            adversarialOn
              ? 'text-amber-400 border border-amber-500/50 bg-amber-500/10 shadow-[0_0_8px_rgba(245,158,11,0.4)]'
              : 'text-muted-foreground hover:text-foreground'
          )}
          aria-pressed={adversarialOn}
        >
          <Swords className="h-3.5 w-3.5" />
          Adversarial Mode
        </Button>
        <ConnectionStatus />
      </div>
    </header>
  )
}

"use client"

import { SessionSelector } from '@/components/SessionSelector'
import { ConnectionStatus } from '@/components/ConnectionStatus'

export function Header() {
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
        <ConnectionStatus />
      </div>
    </header>
  )
}

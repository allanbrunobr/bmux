"use client"

import { useBmuxStore } from '@/lib/store'
import { cn } from '@/lib/utils'

export function ConnectionStatus() {
  const isConnected = useBmuxStore((s) => s.isConnected)

  return (
    <div className="flex items-center gap-2">
      <div
        className={cn(
          'h-2 w-2 rounded-full',
          isConnected
            ? 'bg-green-500 shadow-[0_0_6px_rgba(34,197,94,0.6)]'
            : 'bg-red-500 shadow-[0_0_6px_rgba(239,68,68,0.6)]'
        )}
      />
      <span
        className={cn(
          'text-xs font-medium',
          isConnected ? 'text-green-400' : 'text-red-400'
        )}
      >
        {isConnected ? 'Connected' : 'Disconnected'}
      </span>
    </div>
  )
}

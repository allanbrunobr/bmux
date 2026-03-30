"use client"

import { useState } from 'react'
import { ContextFeed } from '@/components/ContextFeed'
import { Input } from '@/components/ui/input'
import { useBmuxStore } from '@/lib/store'
import { Search } from 'lucide-react'

export default function ContextPage() {
  const [search, setSearch] = useState('')
  const contextEntries = useBmuxStore((s) => s.contextEntries)

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-semibold text-foreground">Context Store</h2>
          <p className="text-sm text-muted-foreground mt-0.5">
            {contextEntries.length} entr{contextEntries.length !== 1 ? 'ies' : 'y'} in the shared context
          </p>
        </div>
        <div className="relative w-72">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            type="search"
            placeholder="Filter by key or value..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-9 h-9 text-sm"
          />
        </div>
      </div>

      <ContextFeed filter={search || undefined} />
    </div>
  )
}

"use client"

import { useState, useRef, useEffect } from 'react'
import { formatDistanceToNow } from 'date-fns'
import { useBmuxStore } from '@/lib/store'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { ChevronDown, ChevronUp } from 'lucide-react'

const PAGE_SIZE = 50

function getKeyBadgeVariant(key: string): 'default' | 'secondary' | 'destructive' | 'outline' {
  if (key.startsWith('design:')) return 'default'
  if (key.startsWith('status:')) return 'outline'
  if (key.startsWith('result:')) return 'secondary'
  if (key.startsWith('question:')) return 'outline'
  return 'outline'
}

interface ExpandableValueProps {
  value: string
  isNew: boolean
}

function ExpandableValue({ value, isNew }: ExpandableValueProps) {
  const [expanded, setExpanded] = useState(false)
  const truncated = value.length > 80

  return (
    <div
      className={isNew ? 'animate-new-entry rounded px-1 -mx-1' : ''}
    >
      <p
        className="text-xs text-foreground/80 leading-relaxed cursor-pointer"
        onClick={() => truncated && setExpanded(!expanded)}
        title={truncated ? (expanded ? 'Click to collapse' : 'Click to expand') : undefined}
      >
        {expanded || !truncated ? value : `${value.slice(0, 80)}...`}
        {truncated && (
          <span className="ml-1 inline-flex items-center text-muted-foreground hover:text-foreground transition-colors">
            {expanded
              ? <ChevronUp className="h-3 w-3" />
              : <ChevronDown className="h-3 w-3" />}
          </span>
        )}
      </p>
    </div>
  )
}

interface ContextFeedProps {
  filter?: string
}

export function ContextFeed({ filter }: ContextFeedProps) {
  const contextEntries = useBmuxStore((s) => s.contextEntries)
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE)
  const newEntryKeysRef = useRef<Set<string>>(new Set())
  const prevLengthRef = useRef(contextEntries.length)

  // Track newly added entries (prepended to front)
  useEffect(() => {
    if (contextEntries.length > prevLengthRef.current) {
      const newCount = contextEntries.length - prevLengthRef.current
      for (let i = 0; i < newCount; i++) {
        const entry = contextEntries[i]
        const entryKey = `${entry.key}:${entry.timestamp}`
        newEntryKeysRef.current.add(entryKey)
      }
      // Clear highlight after animation
      setTimeout(() => {
        newEntryKeysRef.current.clear()
      }, 3500)
    }
    prevLengthRef.current = contextEntries.length
  }, [contextEntries])

  const filtered = filter
    ? contextEntries.filter(
        (e) =>
          e.key.toLowerCase().includes(filter.toLowerCase()) ||
          e.value.toLowerCase().includes(filter.toLowerCase())
      )
    : contextEntries

  const visible = filtered.slice(0, visibleCount)
  const hasMore = filtered.length > visibleCount

  if (contextEntries.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center">
        <p className="text-sm text-muted-foreground">No context entries yet.</p>
        <p className="text-xs text-muted-foreground/60 mt-1">
          Context will appear here as agents publish data.
        </p>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-48">Key</TableHead>
            <TableHead>Value</TableHead>
            <TableHead className="w-36 text-right">Time</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {visible.map((entry) => {
            const entryKey = `${entry.key}:${entry.timestamp}`
            const isNew = newEntryKeysRef.current.has(entryKey)
            return (
              <TableRow key={entryKey}>
                <TableCell>
                  <Badge
                    variant={getKeyBadgeVariant(entry.key)}
                    className="font-mono text-[10px] max-w-full truncate"
                  >
                    {entry.key}
                  </Badge>
                </TableCell>
                <TableCell>
                  <ExpandableValue value={entry.value} isNew={isNew} />
                </TableCell>
                <TableCell className="text-right">
                  <div className="text-xs text-muted-foreground">
                    {formatDistanceToNow(new Date(entry.timestamp), { addSuffix: true })}
                  </div>
                  <div className="text-[10px] text-muted-foreground/50 mt-0.5">
                    {new Date(entry.timestamp).toLocaleTimeString()}
                  </div>
                </TableCell>
              </TableRow>
            )
          })}
        </TableBody>
      </Table>

      {hasMore && (
        <div className="flex justify-center pt-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setVisibleCount((c) => c + PAGE_SIZE)}
          >
            Load more ({filtered.length - visibleCount} remaining)
          </Button>
        </div>
      )}
    </div>
  )
}

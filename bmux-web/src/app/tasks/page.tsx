"use client"

import { useState } from 'react'
import { TaskQueue } from '@/components/TaskQueue'
import { useBmuxStore } from '@/lib/store'
import type { TaskStatus } from '@/lib/types'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

type FilterTab = TaskStatus | 'all'

const TABS: { value: FilterTab; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'active', label: 'Active' },
  { value: 'queued', label: 'Queued' },
  { value: 'completed', label: 'Completed' },
  { value: 'failed', label: 'Failed' },
  { value: 'timed_out', label: 'Timed Out' },
]

export default function TasksPage() {
  const [activeTab, setActiveTab] = useState<FilterTab>('all')
  const tasks = useBmuxStore((s) => s.tasks)

  const countByStatus = (status: FilterTab) => {
    if (status === 'all') return tasks.length
    return tasks.filter((t) => t.status === status).length
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-foreground">Task Queue</h2>
        <p className="text-sm text-muted-foreground mt-0.5">
          {tasks.length} total task{tasks.length !== 1 ? 's' : ''}
        </p>
      </div>

      {/* Filter tabs */}
      <div className="flex items-center gap-1 flex-wrap">
        {TABS.map((tab) => {
          const count = countByStatus(tab.value)
          return (
            <Button
              key={tab.value}
              variant={activeTab === tab.value ? 'default' : 'ghost'}
              size="sm"
              className={cn(
                'h-8 text-xs gap-1.5',
                activeTab !== tab.value && 'text-muted-foreground'
              )}
              onClick={() => setActiveTab(tab.value)}
            >
              {tab.label}
              {count > 0 && (
                <span
                  className={cn(
                    'inline-flex items-center justify-center rounded-full px-1.5 min-w-[18px] h-[18px] text-[10px] font-medium',
                    activeTab === tab.value
                      ? 'bg-primary-foreground/20 text-primary-foreground'
                      : 'bg-muted text-muted-foreground'
                  )}
                >
                  {count}
                </span>
              )}
            </Button>
          )
        })}
      </div>

      <TaskQueue statusFilter={activeTab} />
    </div>
  )
}

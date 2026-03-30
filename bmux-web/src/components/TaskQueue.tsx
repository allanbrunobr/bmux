"use client"

import { formatDistanceToNow } from 'date-fns'
import { useBmuxStore } from '@/lib/store'
import type { Task, TaskStatus } from '@/lib/types'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { ArrowRight, ListTodo } from 'lucide-react'
import { cn } from '@/lib/utils'

interface StatusBadgeProps {
  status: TaskStatus
}

function TaskStatusBadge({ status }: StatusBadgeProps) {
  switch (status) {
    case 'queued':
      return (
        <Badge variant="secondary" className="gap-1 text-[10px]">
          <span>Queued</span>
        </Badge>
      )
    case 'active':
      return (
        <Badge className="gap-1 text-[10px] bg-blue-500/20 text-blue-400 border-blue-500/30 animate-pulse">
          <span>Active</span>
        </Badge>
      )
    case 'completed':
      return (
        <Badge className="gap-1 text-[10px] bg-green-500/20 text-green-400 border-green-500/30">
          <span>Completed</span>
        </Badge>
      )
    case 'failed':
      return (
        <Badge variant="destructive" className="gap-1 text-[10px]">
          <span>Failed</span>
        </Badge>
      )
    case 'timed_out':
      return (
        <Badge className="gap-1 text-[10px] bg-yellow-500/20 text-yellow-400 border-yellow-500/30">
          <span>Timed Out</span>
        </Badge>
      )
  }
}

interface TaskQueueProps {
  statusFilter?: TaskStatus | 'all'
}

export function TaskQueue({ statusFilter = 'all' }: TaskQueueProps) {
  const tasks = useBmuxStore((s) => s.tasks)

  const filtered =
    statusFilter === 'all'
      ? tasks
      : tasks.filter((t) => t.status === statusFilter)

  if (tasks.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center">
        <ListTodo className="h-12 w-12 text-muted-foreground/40 mb-4" />
        <p className="text-sm text-muted-foreground">No tasks yet.</p>
        <p className="text-xs text-muted-foreground/60 mt-1">
          Tasks will appear here as agents delegate work.
        </p>
      </div>
    )
  }

  if (filtered.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-10 text-center">
        <p className="text-sm text-muted-foreground">
          No {statusFilter} tasks.
        </p>
      </div>
    )
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead className="w-20">ID</TableHead>
          <TableHead className="w-48">Route</TableHead>
          <TableHead>Content</TableHead>
          <TableHead className="w-28">Status</TableHead>
          <TableHead className="w-36">Submitted</TableHead>
          <TableHead className="w-20 text-right">Cost</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {filtered.map((task) => (
          <TaskRow key={task.id} task={task} />
        ))}
      </TableBody>
    </Table>
  )
}

function TaskRow({ task }: { task: Task }) {
  const shortId = task.id.slice(-6)
  const contentPreview =
    task.content.length > 40
      ? `${task.content.slice(0, 40)}...`
      : task.content

  return (
    <TableRow
      className={cn(
        task.status === 'active' && 'bg-blue-500/5',
        task.status === 'failed' && 'bg-destructive/5',
        task.status === 'timed_out' && 'bg-yellow-500/5'
      )}
    >
      <TableCell>
        <code className="text-xs font-mono text-muted-foreground">
          ...{shortId}
        </code>
      </TableCell>
      <TableCell>
        <div className="flex items-center gap-1.5 text-xs">
          <span className="font-medium text-foreground truncate max-w-[60px]">
            {task.from_agent}
          </span>
          <ArrowRight className="h-3 w-3 text-muted-foreground flex-shrink-0" />
          <span className="font-medium text-foreground truncate max-w-[60px]">
            {task.to_agent}
          </span>
        </div>
      </TableCell>
      <TableCell>
        <p
          className="text-xs text-foreground/80"
          title={task.content}
        >
          {contentPreview}
        </p>
      </TableCell>
      <TableCell>
        <TaskStatusBadge status={task.status} />
      </TableCell>
      <TableCell>
        <span className="text-xs text-muted-foreground">
          {formatDistanceToNow(new Date(task.submitted_at), { addSuffix: true })}
        </span>
      </TableCell>
      <TableCell className="text-right">
        {task.cost_usd !== undefined ? (
          <span className="text-xs text-muted-foreground font-mono">
            ${task.cost_usd.toFixed(4)}
          </span>
        ) : (
          <span className="text-xs text-muted-foreground/40">—</span>
        )}
      </TableCell>
    </TableRow>
  )
}

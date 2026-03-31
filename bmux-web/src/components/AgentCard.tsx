"use client"

import { Card, CardHeader, CardContent, CardFooter } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { useBmuxStore } from '@/lib/store'
import type { Agent, AgentType, AgentStatus } from '@/lib/types'
import { cn } from '@/lib/utils'

function getAgentTypeIcon(type: AgentType): string {
  switch (type) {
    case 'claude-code': return '🤖'
    case 'opencode': return '✨'
    case 'pi': return '🥧'
    case 'gemini': return '✨'
    case 'shell': return '🔧'
    case 'custom': return '🔌'
    default: return '🤖'
  }
}

function getAgentTypeEmoji(type: AgentType): string {
  switch (type) {
    case 'claude-code': return '\u{1F916}' // 🤖
    case 'opencode': return '\u{2728}' // ✨
    case 'pi': return '\u{1F967}' // 🥧
    case 'gemini': return '\u{2728}' // ✨
    case 'shell': return '\u{1F527}' // 🔧
    case 'custom': return '\u{1F50C}' // 🔌
    default: return '\u{1F916}'
  }
}

function formatCost(cost: number): string {
  if ((cost ?? 0) < 0.001) return `$${((cost ?? 0) * 1000).toFixed(2)}m`
  return `$${(cost ?? 0).toFixed(4)}`
}

function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`
  if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}K`
  return tokens.toString()
}

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`
  if (seconds < 3600) {
    const m = Math.floor(seconds / 60)
    const s = seconds % 60
    return s > 0 ? `${m}m ${s}s` : `${m}m`
  }
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return m > 0 ? `${h}h ${m}m` : `${h}h`
}

interface StatusBadgeProps {
  status: AgentStatus
}

function StatusBadge({ status }: StatusBadgeProps) {
  switch (status) {
    case 'working':
      return (
        <Badge className="bg-blue-500/20 text-blue-400 border-blue-500/30 animate-pulse">
          Working
        </Badge>
      )
    case 'idle':
      return (
        <Badge variant="secondary" className="text-muted-foreground">
          Idle
        </Badge>
      )
    case 'error':
      return (
        <Badge variant="destructive">
          Error
        </Badge>
      )
  }
}

interface AgentCardProps {
  agent: Agent
  className?: string
}

export function AgentCard({ agent, className }: AgentCardProps) {
  const setSendTaskTarget = useBmuxStore((s) => s.setSendTaskTarget)

  return (
    <Card className={cn('flex flex-col transition-all duration-300', className)}>
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0">
            <span className="text-lg" role="img" aria-label={getAgentTypeIcon(agent.agent_type)}>
              {getAgentTypeEmoji(agent.agent_type)}
            </span>
            <div className="min-w-0">
              <h3 className="font-semibold text-sm text-foreground truncate">
                {agent.name}
              </h3>
              <Badge
                variant="outline"
                className="text-[10px] mt-0.5 px-1.5 py-0 h-4 font-normal text-muted-foreground"
              >
                {agent.model}
              </Badge>
            </div>
          </div>
          <StatusBadge status={agent.status} />
        </div>
      </CardHeader>

      <CardContent className="flex-1 space-y-3 pb-3">
        {/* Tokens & Cost */}
        <div className="grid grid-cols-2 gap-3">
          <div className="rounded-md bg-muted/50 p-2">
            <p className="text-[10px] text-muted-foreground mb-0.5">Tokens</p>
            <p className="text-sm font-semibold text-foreground">
              {formatTokens(agent.tokens_used)}
            </p>
          </div>
          <div className="rounded-md bg-muted/50 p-2">
            <p className="text-[10px] text-muted-foreground mb-0.5">Cost</p>
            <p className="text-sm font-semibold text-foreground">
              {formatCost(agent.cost_usd)}
            </p>
          </div>
        </div>

        {/* Uptime */}
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground">Uptime</span>
          <span className="text-foreground font-medium">
            {formatUptime(agent.uptime_seconds)}
          </span>
        </div>

        {/* Last task */}
        {agent.last_task && (
          <div>
            <p className="text-[10px] text-muted-foreground mb-1">Last task</p>
            <p className="text-xs text-foreground/80 leading-relaxed line-clamp-2">
              {agent.last_task.length > 60
                ? `${agent.last_task.slice(0, 60)}...`
                : agent.last_task}
            </p>
          </div>
        )}
      </CardContent>

      <CardFooter className="pt-0">
        <Button
          size="sm"
          variant="outline"
          className="w-full text-xs h-8"
          onClick={() => setSendTaskTarget(agent.id)}
        >
          Send Task
        </Button>
      </CardFooter>
    </Card>
  )
}

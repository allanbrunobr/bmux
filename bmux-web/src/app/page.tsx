"use client"

import { useBmuxStore } from '@/lib/store'
import { AgentsGrid } from '@/components/AgentsGrid'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Bot, DollarSign, Hash, CheckCircle, XCircle, Clock } from 'lucide-react'

function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(2)}M`
  if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}K`
  return tokens.toString()
}

function formatCost(cost: number): string {
  return `$${(cost ?? 0).toFixed(4)}`
}

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return m > 0 ? `${h}h ${m}m` : `${h}h`
}

interface MetricCardProps {
  title: string
  value: string
  icon: React.ReactNode
  description?: string
  highlight?: 'green' | 'red' | 'blue' | 'default'
}

function MetricCard({ title, value, icon, description, highlight = 'default' }: MetricCardProps) {
  const iconColorMap = {
    green: 'text-green-400',
    red: 'text-red-400',
    blue: 'text-blue-400',
    default: 'text-muted-foreground',
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">
          {title}
        </CardTitle>
        <div className={iconColorMap[highlight]}>{icon}</div>
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold text-foreground">{value}</div>
        {description && (
          <p className="text-xs text-muted-foreground mt-1">{description}</p>
        )}
      </CardContent>
    </Card>
  )
}

export default function DashboardPage() {
  const metrics = useBmuxStore((s) => s.metrics)
  const activeSession = useBmuxStore((s) => s.activeSession)

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-foreground">Overview</h2>
        {activeSession && (
          <p className="text-sm text-muted-foreground mt-0.5">
            Session: <span className="font-medium text-foreground">{activeSession}</span>
          </p>
        )}
      </div>

      {/* Metrics grid */}
      {metrics && (
        <div className="grid grid-cols-2 gap-4 lg:grid-cols-3 xl:grid-cols-6">
          <MetricCard
            title="Total Tokens"
            value={formatTokens(metrics.total_tokens)}
            icon={<Hash className="h-4 w-4" />}
          />
          <MetricCard
            title="Total Cost"
            value={formatCost(metrics.total_cost_usd)}
            icon={<DollarSign className="h-4 w-4" />}
          />
          <MetricCard
            title="Active Agents"
            value={metrics.active_agents.toString()}
            icon={<Bot className="h-4 w-4" />}
            highlight="blue"
          />
          <MetricCard
            title="Tasks Completed"
            value={metrics.tasks_completed.toString()}
            icon={<CheckCircle className="h-4 w-4" />}
            highlight="green"
          />
          <MetricCard
            title="Tasks Failed"
            value={metrics.tasks_failed.toString()}
            icon={<XCircle className="h-4 w-4" />}
            highlight={metrics.tasks_failed > 0 ? 'red' : 'default'}
          />
          <MetricCard
            title="Uptime"
            value={formatUptime(metrics.uptime_seconds)}
            icon={<Clock className="h-4 w-4" />}
          />
        </div>
      )}

      {/* Agents section */}
      <div>
        <h3 className="text-base font-medium text-foreground mb-4">Agents</h3>
        <AgentsGrid />
      </div>
    </div>
  )
}

"use client"

import { useBmuxStore } from '@/lib/store'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { getMockTokensPerAgent } from '@/lib/mock-data'
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts'
import { Hash, DollarSign, Bot, CheckCircle, XCircle, Clock } from 'lucide-react'

function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(2)}M`
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

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function CustomTooltip({ active, payload, label }: any) {
  if (active && payload && payload.length) {
    return (
      <div className="rounded-lg border border-border bg-background p-3 shadow-lg text-xs">
        <p className="font-semibold text-foreground mb-2">{label}</p>
        {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
        {payload.map((p: any) => (
          <div key={p.name} className="flex items-center gap-2">
            <div
              className="h-2 w-2 rounded-full"
              style={{ backgroundColor: p.fill }}
            />
            <span className="text-muted-foreground">{p.name}:</span>
            <span className="font-medium text-foreground">
              {p.name === 'Tokens'
                ? formatTokens(p.value as number)
                : `$${(p.value as number).toFixed(4)}`}
            </span>
          </div>
        ))}
      </div>
    )
  }
  return null
}

export default function MetricsPage() {
  const metrics = useBmuxStore((s) => s.metrics)
  const agents = useBmuxStore((s) => s.agents)

  // Build chart data from real agents or fall back to mock
  const chartData =
    agents.length > 0
      ? agents.map((a) => ({
          name: a.name,
          tokens: a.tokens_used,
          cost: a.cost_usd,
        }))
      : getMockTokensPerAgent().map((d) => ({
          name: d.name,
          tokens: d.tokens,
          cost: d.cost,
        }))

  const summaryItems = metrics
    ? [
        {
          icon: <Hash className="h-5 w-5 text-blue-400" />,
          label: 'Total Tokens',
          value: formatTokens(metrics.total_tokens),
        },
        {
          icon: <DollarSign className="h-5 w-5 text-yellow-400" />,
          label: 'Total Cost',
          value: `$${metrics.total_cost_usd.toFixed(4)}`,
        },
        {
          icon: <Bot className="h-5 w-5 text-purple-400" />,
          label: 'Active Agents',
          value: metrics.active_agents.toString(),
        },
        {
          icon: <CheckCircle className="h-5 w-5 text-green-400" />,
          label: 'Tasks Completed',
          value: metrics.tasks_completed.toString(),
        },
        {
          icon: <XCircle className="h-5 w-5 text-red-400" />,
          label: 'Tasks Failed',
          value: metrics.tasks_failed.toString(),
        },
        {
          icon: <Clock className="h-5 w-5 text-muted-foreground" />,
          label: 'Session Uptime',
          value: formatUptime(metrics.uptime_seconds),
        },
      ]
    : []

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-foreground">Metrics</h2>
        <p className="text-sm text-muted-foreground mt-0.5">
          Session performance and resource usage
        </p>
      </div>

      {/* Summary cards */}
      {summaryItems.length > 0 && (
        <div className="grid grid-cols-2 gap-4 lg:grid-cols-3 xl:grid-cols-6">
          {summaryItems.map((item) => (
            <Card key={item.label}>
              <CardHeader className="pb-2 pt-4 px-4">
                <div className="flex items-center gap-2">
                  {item.icon}
                  <CardDescription className="text-xs">{item.label}</CardDescription>
                </div>
              </CardHeader>
              <CardContent className="pb-4 px-4 pt-0">
                <p className="text-xl font-bold text-foreground">{item.value}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Token usage by agent */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Token Usage by Agent</CardTitle>
          <CardDescription>
            Tokens consumed per agent in the current session
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="h-72">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart
                data={chartData}
                margin={{ top: 8, right: 16, left: 0, bottom: 8 }}
              >
                <CartesianGrid
                  strokeDasharray="3 3"
                  stroke="hsl(217.2 32.6% 17.5%)"
                  vertical={false}
                />
                <XAxis
                  dataKey="name"
                  tick={{ fontSize: 11, fill: 'hsl(215 20.2% 65.1%)' }}
                  axisLine={false}
                  tickLine={false}
                />
                <YAxis
                  tickFormatter={(v: number) => formatTokens(v)}
                  tick={{ fontSize: 11, fill: 'hsl(215 20.2% 65.1%)' }}
                  axisLine={false}
                  tickLine={false}
                  width={48}
                />
                <Tooltip content={<CustomTooltip />} cursor={{ fill: 'hsl(217.2 32.6% 17.5% / 0.5)' }} />
                <Legend
                  wrapperStyle={{ fontSize: '11px', color: 'hsl(215 20.2% 65.1%)' }}
                />
                <Bar
                  dataKey="tokens"
                  name="Tokens"
                  fill="hsl(217 91% 60%)"
                  radius={[4, 4, 0, 0]}
                />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </CardContent>
      </Card>

      {/* Cost by agent */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Cost by Agent (USD)</CardTitle>
          <CardDescription>
            API spend per agent in the current session
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="h-72">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart
                data={chartData}
                margin={{ top: 8, right: 16, left: 0, bottom: 8 }}
              >
                <CartesianGrid
                  strokeDasharray="3 3"
                  stroke="hsl(217.2 32.6% 17.5%)"
                  vertical={false}
                />
                <XAxis
                  dataKey="name"
                  tick={{ fontSize: 11, fill: 'hsl(215 20.2% 65.1%)' }}
                  axisLine={false}
                  tickLine={false}
                />
                <YAxis
                  tickFormatter={(v: number) => `$${v.toFixed(3)}`}
                  tick={{ fontSize: 11, fill: 'hsl(215 20.2% 65.1%)' }}
                  axisLine={false}
                  tickLine={false}
                  width={56}
                />
                <Tooltip content={<CustomTooltip />} cursor={{ fill: 'hsl(217.2 32.6% 17.5% / 0.5)' }} />
                <Legend
                  wrapperStyle={{ fontSize: '11px', color: 'hsl(215 20.2% 65.1%)' }}
                />
                <Bar
                  dataKey="cost"
                  name="Cost (USD)"
                  fill="hsl(142 71% 45%)"
                  radius={[4, 4, 0, 0]}
                />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}

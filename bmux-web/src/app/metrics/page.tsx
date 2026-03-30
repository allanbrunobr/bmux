import { MetricsChart } from '@/components/MetricsChart'

export const metadata = { title: 'Metrics — BMUX Dashboard' }

export default function MetricsPage() {
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-bold text-[#e6edf3]">Metrics</h1>
      <p className="text-sm text-[#8b949e]">Real-time token usage and cost tracking per agent.</p>
      <MetricsChart session="main" />
    </div>
  )
}

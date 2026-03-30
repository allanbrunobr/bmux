import Link from 'next/link'

export default function Home() {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold text-[#58a6ff]">BMUX Web Dashboard</h1>
      <p className="text-[#8b949e]">Select a section from the sidebar to get started.</p>
      <div className="grid grid-cols-2 gap-4 max-w-lg">
        <Link href="/metrics" className="block p-4 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-[#58a6ff] transition-colors">
          <div className="text-lg font-semibold">Metrics</div>
          <div className="text-sm text-[#8b949e]">Token usage &amp; costs</div>
        </Link>
        <Link href="/audit" className="block p-4 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-[#58a6ff] transition-colors">
          <div className="text-lg font-semibold">Audit Log</div>
          <div className="text-sm text-[#8b949e]">Compliance &amp; events</div>
        </Link>
      </div>
    </div>
  )
}

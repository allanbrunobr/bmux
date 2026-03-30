'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import clsx from 'clsx'

const NAV = [
  { href: '/', label: 'Dashboard', icon: '⬡' },
  { href: '/metrics', label: 'Metrics', icon: '📈' },
  { href: '/audit', label: 'Audit Log', icon: '📋' },
]

export function Sidebar() {
  const path = usePathname()
  return (
    <aside className="w-48 shrink-0 bg-[#161b22] border-r border-[#30363d] flex flex-col py-4">
      <div className="px-4 mb-6">
        <span className="text-[#58a6ff] font-bold text-lg tracking-wide">BMUX</span>
      </div>
      <nav className="flex flex-col gap-1 px-2">
        {NAV.map((item) => (
          <Link
            key={item.href}
            href={item.href}
            className={clsx(
              'flex items-center gap-2 px-3 py-2 rounded-md text-sm transition-colors',
              path === item.href
                ? 'bg-[#0d1117] text-[#58a6ff]'
                : 'text-[#8b949e] hover:text-[#e6edf3] hover:bg-[#21262d]'
            )}
          >
            <span>{item.icon}</span>
            {item.label}
          </Link>
        ))}
      </nav>
    </aside>
  )
}

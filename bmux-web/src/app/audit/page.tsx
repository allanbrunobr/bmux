import { AuditLog } from '@/components/AuditLog'

export const metadata = { title: 'Audit Log — BMUX Dashboard' }

export default function AuditPage() {
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-bold text-[#e6edf3]">Audit Log</h1>
      <p className="text-sm text-[#8b949e]">
        Compliance event log with LGPD indicators. Filter and export for regulatory review.
      </p>
      <AuditLog session="main" />
    </div>
  )
}

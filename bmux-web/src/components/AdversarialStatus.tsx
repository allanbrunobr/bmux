"use client"

import { useBmuxStore } from '@/lib/store'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import type { AdversarialPhase } from '@/lib/types'

const PHASE_CONFIG: Record<AdversarialPhase, { label: string; emoji: string; color: string }> = {
  negotiating: { label: 'Negotiating', emoji: '🤝', color: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' },
  building:    { label: 'Building',    emoji: '🔨', color: 'bg-blue-500/20 text-blue-400 border-blue-500/30' },
  evaluating:  { label: 'Evaluating', emoji: '🔍', color: 'bg-purple-500/20 text-purple-400 border-purple-500/30' },
  passed:      { label: 'Passed',      emoji: '✅', color: 'bg-green-500/20 text-green-400 border-green-500/30' },
  failed:      { label: 'Failed',      emoji: '❌', color: 'bg-red-500/20 text-red-400 border-red-500/30' },
  complete:    { label: 'Complete',    emoji: '✅', color: 'bg-green-500/20 text-green-400 border-green-500/30' },
}

export function AdversarialStatus() {
  const adversarial = useBmuxStore((s) => s.adversarial)

  if (!adversarial) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium text-muted-foreground">Adversarial Loop</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">No adversarial session active.</p>
        </CardContent>
      </Card>
    )
  }

  const phaseConfig = PHASE_CONFIG[adversarial.phase]
  const progressPct = adversarial.totalSprints > 0
    ? Math.round(((adversarial.sprint - 1) / adversarial.totalSprints) * 100)
    : 0

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base font-semibold text-foreground">Adversarial Loop</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Phase badge */}
        <div className="flex items-center gap-3">
          <span className="text-xs text-muted-foreground font-medium uppercase tracking-wider">Phase</span>
          <span className={`inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1 text-sm font-medium ${phaseConfig.color}`}>
            <span>{phaseConfig.emoji}</span>
            {phaseConfig.label}
          </span>
        </div>

        {/* Counters */}
        <div className="grid grid-cols-2 gap-3">
          <div className="rounded-md bg-muted/40 px-3 py-2">
            <p className="text-xs text-muted-foreground">Sprint</p>
            <p className="text-lg font-bold text-foreground">
              {adversarial.sprint}
              <span className="text-sm font-normal text-muted-foreground">/{adversarial.totalSprints}</span>
            </p>
          </div>
          <div className="rounded-md bg-muted/40 px-3 py-2">
            <p className="text-xs text-muted-foreground">Attempt</p>
            <p className="text-lg font-bold text-foreground">
              {adversarial.attempt}
              <span className="text-sm font-normal text-muted-foreground">/{adversarial.maxAttempts}</span>
            </p>
          </div>
        </div>

        {/* Progress bar */}
        <div>
          <div className="flex justify-between text-xs text-muted-foreground mb-1.5">
            <span>Overall progress</span>
            <span>{progressPct}%</span>
          </div>
          <div className="h-2 w-full rounded-full bg-muted/50 overflow-hidden">
            <div
              className="h-full rounded-full bg-primary transition-all duration-500"
              style={{ width: `${progressPct}%` }}
            />
          </div>
        </div>

        {/* Config info */}
        {adversarial.config && (
          <div className="rounded-md bg-muted/20 px-3 py-2 space-y-1">
            <div className="flex justify-between text-xs">
              <span className="text-muted-foreground">Generator</span>
              <span className="text-foreground font-mono text-[11px]">{adversarial.config.generator_model}</span>
            </div>
            <div className="flex justify-between text-xs">
              <span className="text-muted-foreground">Evaluator</span>
              <span className="text-foreground font-mono text-[11px]">{adversarial.config.evaluator_model}</span>
            </div>
            <div className="flex justify-between text-xs">
              <span className="text-muted-foreground">Pass threshold</span>
              <Badge variant="outline" className="text-[10px] h-4 px-1">{adversarial.config.threshold}/10</Badge>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

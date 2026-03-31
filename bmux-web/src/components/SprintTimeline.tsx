"use client"

import { useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { ScoreBoard } from '@/components/ScoreBoard'
import type { AdversarialPhase, SprintAttempt, SprintPlan } from '@/lib/types'

interface SprintTimelineProps {
  currentSprint: number
  totalSprints: number
  phase: AdversarialPhase
  history: SprintAttempt[]
  threshold: number
  sprintPlan?: SprintPlan
}

type SprintStatus = 'passed' | 'failed' | 'building' | 'evaluating' | 'pending'

function getSprintStatus(
  sprintNum: number,
  currentSprint: number,
  phase: AdversarialPhase,
  history: SprintAttempt[]
): SprintStatus {
  // Past sprints — check history
  if (sprintNum < currentSprint) {
    const sprintAttempts = history.filter((h) => h.sprint === sprintNum)
    if (sprintAttempts.length > 0) {
      return sprintAttempts.some((a) => a.passed) ? 'passed' : 'failed'
    }
    return 'passed' // assume passed if we moved on
  }
  // Current sprint
  if (sprintNum === currentSprint) {
    if (phase === 'evaluating') return 'evaluating'
    if (phase === 'building' || phase === 'negotiating') return 'building'
    if (phase === 'passed' || phase === 'complete') return 'passed'
    if (phase === 'failed') return 'failed'
  }
  // Future sprints
  return 'pending'
}

const STATUS_CONFIG: Record<SprintStatus, { emoji: string; label: string; ringClass: string }> = {
  passed:     { emoji: '✅', label: 'Passed',     ringClass: 'ring-green-500/50 bg-green-500/10' },
  failed:     { emoji: '❌', label: 'Failed',     ringClass: 'ring-red-500/50 bg-red-500/10' },
  building:   { emoji: '🔨', label: 'Building',   ringClass: 'ring-blue-500/50 bg-blue-500/20 animate-pulse' },
  evaluating: { emoji: '🔍', label: 'Evaluating', ringClass: 'ring-purple-500/50 bg-purple-500/20 animate-pulse' },
  pending:    { emoji: '⏳', label: 'Pending',    ringClass: 'ring-border/50 bg-muted/20' },
}

export function SprintTimeline({
  currentSprint,
  totalSprints,
  phase,
  history,
  threshold,
  sprintPlan,
}: SprintTimelineProps) {
  const [expandedSprint, setExpandedSprint] = useState<number | null>(null)

  const sprints = Array.from({ length: totalSprints }, (_, i) => i + 1)

  function getSprintTitle(sprintNum: number): string | null {
    return sprintPlan?.sprints.find((s) => s.number === sprintNum)?.title ?? null
  }

  function getAttemptCount(sprintNum: number): number {
    return history.filter((h) => h.sprint === sprintNum).length
  }

  function getSprintHistory(sprintNum: number): SprintAttempt[] {
    return history.filter((h) => h.sprint === sprintNum)
  }

  function getSprintScores(sprintNum: number) {
    const attempts = getSprintHistory(sprintNum)
    if (attempts.length === 0) return []
    return attempts[attempts.length - 1].scores
  }

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base font-semibold text-foreground">Sprint Timeline</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Horizontal timeline */}
        <div className="flex items-start gap-0 overflow-x-auto pb-2">
          {sprints.map((sprintNum, idx) => {
            const status = getSprintStatus(sprintNum, currentSprint, phase, history)
            const config = STATUS_CONFIG[status]
            const attemptCount = getAttemptCount(sprintNum)
            const isExpanded = expandedSprint === sprintNum
            const isLast = idx === sprints.length - 1

            const title = getSprintTitle(sprintNum)

            return (
              <div key={sprintNum} className="flex items-center">
                {/* Sprint node */}
                <button
                  onClick={() => setExpandedSprint(isExpanded ? null : sprintNum)}
                  className={`flex flex-col items-center gap-1 px-3 py-2 rounded-lg ring-1 transition-all min-w-[80px] ${config.ringClass} ${
                    isExpanded ? 'ring-2' : ''
                  } ${status !== 'pending' ? 'cursor-pointer hover:opacity-80' : 'cursor-default'}`}
                  disabled={status === 'pending'}
                >
                  <span className="text-xl leading-none">{config.emoji}</span>
                  <span className="text-xs font-medium text-foreground">Sprint {sprintNum}</span>
                  {title && (
                    <span className="text-[10px] text-muted-foreground text-center leading-tight max-w-[72px] truncate" title={title}>{title}</span>
                  )}
                  <span className="text-[10px] text-muted-foreground">{config.label}</span>
                  {attemptCount > 0 && (
                    <span className="text-[10px] text-muted-foreground">{attemptCount} attempt{attemptCount !== 1 ? 's' : ''}</span>
                  )}
                </button>

                {/* Connector line */}
                {!isLast && (
                  <div className="flex-shrink-0 w-8 h-px bg-border/50 mx-1" />
                )}
              </div>
            )
          })}
        </div>

        {/* Expanded sprint details */}
        {expandedSprint !== null && (
          <div className="border-t border-border/50 pt-4">
            <p className="text-sm font-medium text-foreground mb-3">
              Sprint {expandedSprint} — Details
            </p>
            {getSprintScores(expandedSprint).length > 0 ? (
              <ScoreBoard
                scores={getSprintScores(expandedSprint)}
                history={getSprintHistory(expandedSprint)}
                threshold={threshold}
              />
            ) : (
              <p className="text-sm text-muted-foreground">No evaluation data yet.</p>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

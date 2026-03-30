"use client"

import { useState, useRef, useEffect } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import type { EvaluationScore, SprintAttempt } from '@/lib/types'

interface ScoreBoardProps {
  scores: EvaluationScore[]
  history: SprintAttempt[]
  threshold: number
}

function ScoreRow({ score, prevScore }: { score: EvaluationScore; prevScore?: EvaluationScore }) {
  const glowRef = useRef<HTMLTableRowElement>(null)

  useEffect(() => {
    if (!prevScore || !glowRef.current) return
    const delta = score.score - prevScore.score
    if (delta === 0) return

    const el = glowRef.current
    el.style.transition = 'background-color 0.3s ease'
    el.style.backgroundColor = delta > 0 ? 'rgba(22,163,74,0.25)' : 'rgba(220,38,38,0.25)'
    const timeout = setTimeout(() => {
      el.style.backgroundColor = ''
    }, 1200)
    return () => clearTimeout(timeout)
  }, [score.score, prevScore])

  return (
    <tr ref={glowRef} className="border-b border-border/50 last:border-0 transition-colors">
      <td className="py-2 pr-4 text-sm text-foreground font-mono">{score.criterion}</td>
      <td className="py-2 pr-4 text-sm text-center">
        <span className={score.passed ? 'text-green-400 font-semibold' : 'text-red-400 font-semibold'}>
          {score.score}
        </span>
        <span className="text-muted-foreground text-xs">/{score.threshold}</span>
      </td>
      <td className="py-2 text-sm text-center">
        {score.passed ? (
          <span className="inline-flex items-center gap-1 rounded border border-green-500/30 bg-green-500/10 px-2 py-0.5 text-xs text-green-400">
            ✅ PASS
          </span>
        ) : (
          <span className="inline-flex items-center gap-1 rounded border border-red-500/30 bg-red-500/10 px-2 py-0.5 text-xs text-red-400">
            ❌ FAIL
          </span>
        )}
      </td>
      {score.details && (
        <td className="py-2 pl-4 text-xs text-muted-foreground max-w-xs truncate">{score.details}</td>
      )}
    </tr>
  )
}

function HistoryAttempt({ attempt }: { attempt: SprintAttempt }) {
  const passed = attempt.scores.filter((s) => s.passed).length
  const total = attempt.scores.length

  return (
    <div className="rounded-md border border-border/50 bg-muted/20 p-3 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-foreground">
          Sprint {attempt.sprint} · Attempt {attempt.attempt}
        </span>
        <span className={`text-xs px-2 py-0.5 rounded border ${
          attempt.passed
            ? 'text-green-400 bg-green-500/10 border-green-500/30'
            : 'text-red-400 bg-red-500/10 border-red-500/30'
        }`}>
          {attempt.passed ? '✅ Passed' : `❌ ${passed}/${total} criteria`}
        </span>
      </div>
      {attempt.feedback.length > 0 && (
        <ul className="space-y-1">
          {attempt.feedback.map((f, i) => (
            <li key={i} className="text-xs text-muted-foreground pl-3 border-l-2 border-border/50">{f}</li>
          ))}
        </ul>
      )}
    </div>
  )
}

export function ScoreBoard({ scores, history, threshold }: ScoreBoardProps) {
  const [showHistory, setShowHistory] = useState(false)

  // Previous scores for glow animation (last history entry scores)
  const prevAttempt = history.length > 1 ? history[history.length - 2] : undefined
  const prevScoreMap = new Map<string, EvaluationScore>(
    prevAttempt?.scores.map((s) => [s.criterion, s]) ?? []
  )

  if (scores.length === 0) return null

  const passCount = scores.filter((s) => s.passed).length

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base font-semibold text-foreground">
            Evaluation Scores
            <span className="ml-2 text-sm font-normal text-muted-foreground">
              ({passCount}/{scores.length} passed)
            </span>
          </CardTitle>
          {history.length > 0 && (
            <button
              onClick={() => setShowHistory((v) => !v)}
              className="text-xs text-primary hover:text-primary/80 transition-colors"
            >
              {showHistory ? 'Hide History' : 'View History'}
            </button>
          )}
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Score table */}
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-border text-left">
                <th className="pb-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">Criterion</th>
                <th className="pb-2 text-xs font-medium text-muted-foreground uppercase tracking-wider text-center">Score</th>
                <th className="pb-2 text-xs font-medium text-muted-foreground uppercase tracking-wider text-center">Status</th>
                <th className="pb-2 text-xs font-medium text-muted-foreground uppercase tracking-wider pl-4">Details</th>
              </tr>
            </thead>
            <tbody>
              {scores.map((score) => (
                <ScoreRow
                  key={score.criterion}
                  score={score}
                  prevScore={prevScoreMap.get(score.criterion)}
                />
              ))}
            </tbody>
          </table>
        </div>

        {/* Threshold note */}
        <p className="text-xs text-muted-foreground">Pass threshold: {threshold}/10</p>

        {/* History panel */}
        {showHistory && history.length > 0 && (
          <div className="space-y-2 pt-2 border-t border-border/50">
            <p className="text-sm font-medium text-foreground">All Attempts</p>
            {[...history].reverse().map((attempt, i) => (
              <HistoryAttempt key={`${attempt.sprint}-${attempt.attempt}-${i}`} attempt={attempt} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

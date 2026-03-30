"use client"

import { AdversarialStatus } from '@/components/AdversarialStatus'
import { ScoreBoard } from '@/components/ScoreBoard'
import { SprintTimeline } from '@/components/SprintTimeline'
import { useBmuxStore } from '@/lib/store'
import { useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

const API_BASE = process.env.NEXT_PUBLIC_BMUX_API || 'http://localhost:7432'

export default function AdversarialPage() {
  const adversarial = useBmuxStore((s) => s.adversarial)
  const activeSession = useBmuxStore((s) => s.activeSession)

  const [generatorModel, setGeneratorModel] = useState('claude-sonnet-4-20250514')
  const [evaluatorModel, setEvaluatorModel] = useState('claude-opus-4-20250514')
  const [prompt, setPrompt] = useState('')
  const [multiSprint, setMultiSprint] = useState(false)
  const [isStarting, setIsStarting] = useState(false)

  const models = [
    'claude-sonnet-4-20250514',
    'claude-opus-4-20250514',
  ]

  async function handleStart() {
    if (!prompt.trim() || !activeSession) return
    setIsStarting(true)
    try {
      await fetch(`${API_BASE}/api/adversarial/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          session: activeSession,
          generator_model: generatorModel,
          evaluator_model: evaluatorModel,
          prompt,
          multi_sprint: multiSprint,
        }),
      })
    } catch {
      // Connection errors are expected in dev
    } finally {
      setIsStarting(false)
    }
  }

  async function handleStop() {
    if (!activeSession) return
    await fetch(`${API_BASE}/api/adversarial/stop`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session: activeSession }),
    }).catch(() => {})
  }

  const isRunning = adversarial && !['failed', 'complete'].includes(adversarial.phase)

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-foreground">Adversarial Mode</h2>
        <p className="text-sm text-muted-foreground mt-0.5">
          Generator builds, Evaluator breaks — driven by adversarial tension.
        </p>
      </div>

      {/* Start panel */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium text-foreground">Start Adversarial Loop</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <label className="text-xs text-muted-foreground font-medium">Generator Model</label>
              <select
                className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm text-foreground"
                value={generatorModel}
                onChange={(e) => setGeneratorModel(e.target.value)}
                disabled={!!isRunning}
              >
                {models.map((m) => <option key={m} value={m}>{m}</option>)}
              </select>
            </div>
            <div className="space-y-1.5">
              <label className="text-xs text-muted-foreground font-medium">Evaluator Model</label>
              <select
                className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm text-foreground"
                value={evaluatorModel}
                onChange={(e) => setEvaluatorModel(e.target.value)}
                disabled={!!isRunning}
              >
                {models.map((m) => <option key={m} value={m}>{m}</option>)}
              </select>
            </div>
          </div>

          <div className="space-y-1.5">
            <label className="text-xs text-muted-foreground font-medium">Task Prompt</label>
            <textarea
              className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground resize-none"
              rows={3}
              placeholder="Build a REST API with authentication and rate limiting..."
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              disabled={!!isRunning}
            />
          </div>

          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="multi-sprint"
              className="h-4 w-4 rounded border-border"
              checked={multiSprint}
              onChange={(e) => setMultiSprint(e.target.checked)}
              disabled={!!isRunning}
            />
            <label htmlFor="multi-sprint" className="text-sm text-foreground cursor-pointer">
              Multi-Sprint Mode
            </label>
            {multiSprint && (
              <span className="text-xs text-muted-foreground">— a Planner agent will expand your prompt into 3–6 sprints</span>
            )}
          </div>

          <div className="flex gap-3">
            <button
              onClick={handleStart}
              disabled={!!isRunning || isStarting || !prompt.trim()}
              className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-50 disabled:cursor-not-allowed hover:bg-primary/90 transition-colors"
            >
              {isStarting ? 'Starting…' : isRunning ? 'Running…' : 'Start Adversarial Loop'}
            </button>
            {isRunning && (
              <button
                onClick={handleStop}
                className="rounded-md border border-border px-4 py-2 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
              >
                Stop
              </button>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Live status */}
      <AdversarialStatus />

      {/* Score board */}
      {adversarial && adversarial.scores.length > 0 && (
        <ScoreBoard scores={adversarial.scores} history={adversarial.history} threshold={adversarial.config?.threshold ?? 7} />
      )}

      {/* Sprint timeline (multi-sprint) */}
      {adversarial && adversarial.totalSprints > 1 && (
        <SprintTimeline
          currentSprint={adversarial.sprint}
          totalSprints={adversarial.totalSprints}
          phase={adversarial.phase}
          history={adversarial.history}
          threshold={adversarial.config?.threshold ?? 7}
        />
      )}
    </div>
  )
}

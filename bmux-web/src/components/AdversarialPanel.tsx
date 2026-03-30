"use client"

import { Loader2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { ModelSelector } from '@/components/ModelSelector'
import { useBmuxStore } from '@/lib/store'
import { bmuxClient } from '@/lib/bmux-client'

export function AdversarialPanel() {
  const {
    activeSession,
    adversarialRunning,
    generatorModel,
    evaluatorModel,
    adversarialPrompt,
    setAdversarialRunning,
    setGeneratorModel,
    setEvaluatorModel,
    setAdversarialPrompt,
  } = useBmuxStore()

  async function handleStart() {
    if (!activeSession || !adversarialPrompt.trim()) return
    setAdversarialRunning(true)
    await bmuxClient.startAdversarialLoop({
      session: activeSession,
      generator_model: generatorModel,
      evaluator_model: evaluatorModel,
      prompt: adversarialPrompt,
    })
  }

  async function handleStop() {
    if (!activeSession) return
    await bmuxClient.stopAdversarialLoop(activeSession)
    setAdversarialRunning(false)
  }

  return (
    <div className="border-b border-amber-500/30 bg-amber-500/5 px-6 py-3">
      <div className="flex flex-wrap items-end gap-4">
        {/* Model selectors */}
        <ModelSelector
          label="Generator Model"
          value={generatorModel}
          onChange={setGeneratorModel}
        />
        <ModelSelector
          label="Evaluator Model"
          value={evaluatorModel}
          onChange={setEvaluatorModel}
        />

        {/* Prompt input */}
        <div className="flex flex-col gap-1 flex-1 min-w-64">
          <span className="text-xs text-muted-foreground">Task Prompt</span>
          <Textarea
            placeholder="e.g. Build a REST API with JWT auth"
            className="h-8 min-h-0 resize-none text-xs py-1.5"
            value={adversarialPrompt}
            onChange={(e) => setAdversarialPrompt(e.target.value)}
            disabled={adversarialRunning}
            rows={1}
          />
        </div>

        {/* Start / Stop */}
        {adversarialRunning ? (
          <Button
            size="sm"
            variant="destructive"
            onClick={handleStop}
            className="h-8 text-xs"
          >
            Stop
          </Button>
        ) : (
          <Button
            size="sm"
            onClick={handleStart}
            disabled={!adversarialPrompt.trim()}
            className="h-8 text-xs bg-amber-600 hover:bg-amber-500 text-white"
          >
            {adversarialRunning ? (
              <>
                <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                Running...
              </>
            ) : (
              'Start Adversarial Loop'
            )}
          </Button>
        )}
      </div>
    </div>
  )
}

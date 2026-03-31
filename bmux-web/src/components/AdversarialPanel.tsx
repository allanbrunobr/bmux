"use client"

import { useRef, useState, type ChangeEvent } from 'react'
import { Loader2, Upload } from 'lucide-react'
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
    setAdversarialRunning,
    setGeneratorModel,
    setEvaluatorModel,
  } = useBmuxStore()

  const [prdContent, setPrdContent] = useState('')
  const fileInputRef = useRef<HTMLInputElement>(null)

  function handleFileUpload(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = (ev) => {
      const text = ev.target?.result
      if (typeof text === 'string') setPrdContent(text)
    }
    reader.readAsText(file)
    // Reset so same file can be re-uploaded
    e.target.value = ''
  }

  async function handleStart() {
    if (!activeSession || !prdContent.trim()) return
    setAdversarialRunning(true)
    await bmuxClient.startAdversarialLoop({
      session: activeSession,
      generator_model: generatorModel,
      evaluator_model: evaluatorModel,
      prd_content: prdContent,
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

        {/* PRD input */}
        <div className="flex flex-col gap-1 flex-1 min-w-64">
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">Paste PRD here</span>
            <Button
              type="button"
              size="sm"
              variant="ghost"
              className="h-5 px-1.5 text-xs gap-1"
              onClick={() => fileInputRef.current?.click()}
              disabled={adversarialRunning}
            >
              <Upload className="h-3 w-3" />
              Upload .md
            </Button>
          </div>
          <input
            ref={fileInputRef}
            type="file"
            accept=".md,.txt"
            className="hidden"
            onChange={handleFileUpload}
          />
          <Textarea
            placeholder="Paste your PRD here or upload a .md file…"
            className="min-h-[80px] resize-y text-xs py-1.5"
            value={prdContent}
            onChange={(e) => setPrdContent(e.target.value)}
            disabled={adversarialRunning}
            rows={4}
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
            disabled={!prdContent.trim()}
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

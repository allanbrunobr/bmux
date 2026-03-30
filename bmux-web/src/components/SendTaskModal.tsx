"use client"

import { useState, useCallback } from 'react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { useBmuxStore } from '@/lib/store'
import { bmuxClient } from '@/lib/bmux-client'
import { toast } from '@/components/ui/use-toast'
import { Loader2 } from 'lucide-react'

export function SendTaskModal() {
  const sendTaskTarget = useBmuxStore((s) => s.sendTaskTarget)
  const setSendTaskTarget = useBmuxStore((s) => s.setSendTaskTarget)
  const activeSession = useBmuxStore((s) => s.activeSession)
  const agents = useBmuxStore((s) => s.agents)
  const handleEvent = useBmuxStore((s) => s.handleEvent)

  const [content, setContent] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)

  const targetAgent = agents.find((a) => a.id === sendTaskTarget)

  const handleClose = useCallback(() => {
    setSendTaskTarget(null)
    setContent('')
  }, [setSendTaskTarget])

  const handleSend = useCallback(async () => {
    if (!content.trim() || !sendTaskTarget || !activeSession) return

    setIsSubmitting(true)
    try {
      const result = await bmuxClient.sendTask(
        activeSession,
        sendTaskTarget,
        content.trim()
      )

      // Optimistically add task to store
      handleEvent({
        type: 'task_created',
        task: {
          id: result.id,
          from_agent: 'dashboard',
          to_agent: targetAgent?.name ?? sendTaskTarget,
          content: content.trim(),
          status: 'queued',
          submitted_at: new Date().toISOString(),
        },
      })

      toast({
        title: 'Task sent',
        description: `Task dispatched to ${targetAgent?.name ?? sendTaskTarget}`,
      })

      handleClose()
    } catch (err) {
      toast({
        title: 'Failed to send task',
        description:
          err instanceof Error ? err.message : 'An unknown error occurred',
        variant: 'destructive',
      })
    } finally {
      setIsSubmitting(false)
    }
  }, [
    content,
    sendTaskTarget,
    activeSession,
    targetAgent,
    handleEvent,
    handleClose,
  ])

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        void handleSend()
      }
    },
    [handleSend]
  )

  return (
    <Dialog open={!!sendTaskTarget} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Send Task</DialogTitle>
          <DialogDescription>
            {targetAgent ? (
              <>
                Send a task to{' '}
                <span className="font-semibold text-foreground">
                  {targetAgent.name}
                </span>{' '}
                ({targetAgent.model})
              </>
            ) : (
              'Send a task to the selected agent.'
            )}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          <Textarea
            placeholder="Describe the task for this agent..."
            value={content}
            onChange={(e) => setContent(e.target.value)}
            onKeyDown={handleKeyDown}
            rows={5}
            className="resize-none"
            autoFocus
            disabled={isSubmitting}
          />
          <p className="text-[10px] text-muted-foreground">
            Tip: Press Cmd+Enter (or Ctrl+Enter) to send quickly.
          </p>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={handleClose}
            disabled={isSubmitting}
          >
            Cancel
          </Button>
          <Button
            onClick={() => void handleSend()}
            disabled={!content.trim() || isSubmitting}
          >
            {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {isSubmitting ? 'Sending...' : 'Send Task'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

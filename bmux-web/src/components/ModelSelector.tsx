"use client"

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { AdversarialModel } from '@/lib/types'
import { ADVERSARIAL_MODELS } from '@/lib/types'

interface ModelSelectorProps {
  label: string
  value: AdversarialModel
  onChange: (model: AdversarialModel) => void
}

export function ModelSelector({ label, value, onChange }: ModelSelectorProps) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs text-muted-foreground">{label}</span>
      <Select value={value} onValueChange={(v) => onChange(v as AdversarialModel)}>
        <SelectTrigger className="h-8 text-xs w-56">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {ADVERSARIAL_MODELS.map((model) => (
            <SelectItem key={model} value={model} className="text-xs">
              {model}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}

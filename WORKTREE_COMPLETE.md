# Worktree wt1 — COMPLETE

**Branch:** `bmux-adv-wt1`
**Date:** 2026-03-30

## Stories Implemented

### Story 1.1 — Adversarial Mode Toggle ✅
- Toggle button in Header with amber glow when active
- `adversarialOn` state in Zustand store
- `AdversarialPanelWrapper` conditionally renders panel in layout

### Story 1.2 — Model Selector for Generator & Evaluator ✅
- `AdversarialModel` type + `ADVERSARIAL_MODELS` constant in `types.ts`
- `ModelSelector.tsx` — reusable shadcn/ui Select wrapper
- Generator and Evaluator dropdowns in `AdversarialPanel.tsx`

### Story 1.3 — Adversarial Prompt Input & Start ✅
- `AdversarialStartRequest` type in `types.ts`
- `startAdversarialLoop` / `stopAdversarialLoop` in `BmuxClient`
- Prompt textarea + Start/Stop button wired to `POST /api/adversarial/start|stop`

## Files Changed

| File | Change |
|------|--------|
| `bmux-web/src/lib/types.ts` | Added adversarial types |
| `bmux-web/src/lib/store.ts` | Added adversarial state |
| `bmux-web/src/lib/bmux-client.ts` | Added adversarial API methods |
| `bmux-web/src/app/layout.tsx` | Added AdversarialPanelWrapper |
| `bmux-web/src/components/layout/Header.tsx` | Added toggle button |
| `bmux-web/src/components/AdversarialPanel.tsx` | New — panel with selectors + prompt + start/stop |
| `bmux-web/src/components/AdversarialPanelWrapper.tsx` | New — client boundary wrapper |
| `bmux-web/src/components/ModelSelector.tsx` | New — model dropdown component |
| `features.md` | Features 1, 2, 3 marked done |

## Build
`npm run build` — ✅ passes (0 errors, 0 type errors)

# Tasks: add-gameplay-options-auto-processing

## 1. Options state + panel

- [ ] 1.1 Add persisted `gameplayOptions` slice to `uiStore` (all toggles + defaults: automation on, confirm off, animations on, rotation on) following the resolution-preset persistence pattern
- [ ] 1.2 Build `GameplayOptionsPanel` component (toggle list with short descriptions, DCGO-style grouping) and mount it on the settings page
- [ ] 1.3 Add in-game access (gear affordance near the action bar / pause surface) opening the same panel

## 2. Auto-resolution engine

- [ ] 2.1 Implement `utils/autoResolve.ts` with `classifyAutoResolve(pendingSelection, mask, options)` covering: single-legal-action, order-only allowlist (bottom-of-deck order), min digivolve cost, auto-hatch — returning the action(s) to submit or null
- [ ] 2.2 Unit tests per rule, including the negative cases: two-legal-action selections stay manual, unlisted ordering kinds stay manual, optional-keyword decline-only vs use-only handling, over-exposed-PASS selections stay manual
- [ ] 2.3 Wire the driver in `GamePage`: classify on each new pending selection, submit through `sendAction` after a short visible beat, one classification per response cycle, depth guard, on engine rejection disable automation for the session + notify
- [ ] 2.4 Mark auto-submitted actions in `GameLog`/`ActionTraceTicker` with an "(auto)" marker

## 3. Presentation toggles

- [ ] 3.1 Confirm-before-end: two-step submit inside `SelectionPanel` showing the chosen cards
- [ ] 3.2 Animation toggle consumed by `PhaseBanner`, `DigivolveBanner`, `BattleEffect`, `SecurityRevealOverlay` (skip/instant modes; keep event-seq tracking intact; security reveals must still show card + result)
- [ ] 3.3 Suspend-rotation toggle in `Card.tsx`/`PermanentSlot.tsx` with upright + "SUSPENDED" tag fallback rendering

## 4. Verification

- [ ] 4.1 Frontend test pass (`npm test`) including new unit/component tests
- [ ] 4.2 Playwright scenario spec (via the scenario substrate) staging a forced single-option selection and asserting auto-submit + log marker; a 2-option selection asserting manual prompt
- [ ] 4.3 Manual desktop run (`/run-desktop`): play a bot game with all automation on and confirm reduced click count, audit-log markers, and that no strategic prompt was bypassed

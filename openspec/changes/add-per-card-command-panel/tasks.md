# Tasks: add-per-card-command-panel

## 1. Menu model

- [ ] 1.1 Extend `useActionMask`/`utils` as needed so each per-card capability can be mapped back to its concrete action id(s) (play, digivolve per target+cost, attack per target, effect per index, DNA, trash, breeding moves)
- [ ] 1.2 Implement `utils/commandMenu.ts` (`buildCommandMenu(cardRef, parsedMask, gameState)`) producing labeled entries with action ids or target-pick descriptors; labels per design D4
- [ ] 1.3 Unit tests: menu derivation for hand cards, permanents with effects/attacks, breeding, empty state, suspended permanents, inline-target threshold

## 2. Panel component + wiring

- [ ] 2.1 Build `components/game/CommandPanel.tsx` (anchored popover, canvas-clamped, click-away/Esc dismiss, empty state)
- [ ] 2.2 Centralize left-click routing in `GamePage`: idle state opens panel; pending-selection/attack/block/counter flows keep direct click meaning (unit tests per flow state)
- [ ] 2.3 Wire click → panel in `HandZone`, `PermanentSlot`/`BattleArea`, `BreedingArea`; verify no regression to right-click inspect, hover preview, or drag (interaction test matrix)
- [ ] 2.4 Target-pick mode: entry activation highlights legal slots via existing highlight machinery, slot click composes + submits the action, Esc cancels
- [ ] 2.5 Close/rebuild panel on mask refresh (subscribe to state version in `gameStore`)

## 3. ActionBar slimming

- [ ] 3.1 Remove generic per-source effect buttons from `ActionBar` (panel now owns effect activation); keep Pass, Hatch, Move, Mulligan, Surrender
- [ ] 3.2 Update any tests/specs referencing the removed buttons

## 4. Verification

- [ ] 4.1 Component/unit test pass (`npm test`)
- [ ] 4.2 Playwright scenario specs: staged board asserting menu contents for a known card (play+digivolve), effect entry activation submitting the correct action, selection-flow click unaffected
- [ ] 4.3 Manual desktop run (`/run-desktop`): play a bot game using only the command panel (no drag) end-to-end; confirm parity of outcomes and touch-input behavior

# Proposal: add-per-card-command-panel

## Why

In DCGO, clicking any card opens a contextual command panel listing exactly what that card can do right now (Play, Digivolve, Attack, each activatable effect by name). In our UI the action space is illegible: effects are global `ActionBar` buttons labeled generically, digivolve targets are discoverable only by dragging, and a player cannot ask "what can this card do?" This is the single largest interaction gap vs DCGO for players, and it also hurts QA (verifying that a card's effect is offered requires decoding the action mask by hand).

## What Changes

- Add a **per-card command panel**: left-clicking an own hand card or own field permanent opens a small anchored menu of that card's currently-legal actions, decoded from the existing action mask parse (`useActionMask` already produces per-card capability maps: `canPlayFromHand`, `canDigivolve`, `canAttack`, `canActivateEffect`, `canDnaDigivolve`, `canTrashFromHand`).
  - **Hand card**: "Play (cost N)", "Digivolve onto <target>" (one entry per legal target, hover-highlights the target slot), DNA digivolve, trash/discard when legal.
  - **Field permanent**: "Attack <target/security>" (per legal target), "Activate <effect>" (per legal effect index), move-to/from breeding when legal.
  - Suspended permanents render the panel rotated/repositioned appropriately (DCGO parity); panel is dismissed by click-away / `Esc`.
- **Effect entries get real labels** where available: use the engine-provided effect choice labels when present, falling back to timing-tagged generic labels (`[Main] Effect 1`) — and upgrade as the engine's effect-text serialization lands (`add-permanent-stack-inspector`).
- **Drag-and-drop is preserved** as the fast path; the command panel is the discoverable path. Click→menu→target-pick and drag→drop submit identical actions.
- **Target-pick mode**: choosing a multi-target entry (e.g. "Digivolve onto…") enters the existing slot-highlight flow (reuse `fieldSelectionHighlights` machinery) instead of listing slots textually when more than ~3 targets exist.
- Right-click inspection (`CardOverlay` stack inspector) and hover preview behavior are unchanged.

## Capabilities

### New Capabilities
- `per-card-command-panel`: A contextual, mask-derived action menu on own hand cards and own permanents, listing every currently-legal action for that card with human-readable labels, submitting through the same action path as existing gestures.

### Modified Capabilities
<!-- None. The action mask, submission path, and existing gestures are unchanged; this adds a parallel affordance. -->

## Impact

- **Frontend only**: new `components/game/CommandPanel.tsx`; wiring in `HandZone.tsx`, `PermanentSlot.tsx`, `BattleArea.tsx`, `GamePage.tsx` (click routing — today's bare `onSlotClick` field-index action becomes panel-mediated where ambiguous); `hooks/useActionMask.ts` (may need small extensions, e.g. exposing action ids alongside capability sets); `utils/constants.ts` action encoding helpers.
- **Engine/server**: no changes; reads existing mask + state.
- **Conflicts/ordering**: builds on the same click surface as the in-flight `add-permanent-stack-inspector` (right-click) — left-click panel must not regress right-click inspect. Benefits from `add-ingame-card-preview` and effect-text serialization for richer labels but does not depend on them.
- **Tests**: component tests for mask→menu derivation; Playwright scenario specs (via the completed `add-ui-scenario-test-substrate`) asserting menu contents for staged board states.

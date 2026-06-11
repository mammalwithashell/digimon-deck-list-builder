# Design: add-per-card-command-panel

## Context

DCGO's `CommandPanel.cs` anchors a contextual button list to a clicked card: every legal action for that card, by name, sized to button count, rotated when the permanent is suspended. Our equivalent information already exists in the frontend: `useActionMask` parses the 2192-action mask into per-card capability maps (`canPlayFromHand`, `canDigivolve: Map<handIdx, Set<slot>>`, `canAttack: Map<slot, Set<target>>`, `canActivateEffect: Map<slot, Set<effectIdx>>`, `canDnaDigivolve`, `canTrashFromHand`, `canAttackSecurity`, `canHatch`/`canMove`) — but it is consumed only by drag-and-drop validity, the global `ActionBar` (generic "Effect N" buttons), and bare `onSlotClick` routing. There is no per-card affordance. Today left-click on a permanent triggers a context-dependent field-index action; right-click opens the stack inspector (`add-permanent-stack-inspector`, in flight).

## Goals / Non-Goals

**Goals:**
- Click any own card → see every currently-legal action for it, named, and execute one.
- Keep drag-and-drop as the fast path; identical action submission either way.
- Make effect activation legible (named where the engine provides labels).
- DCGO-parity ergonomics: anchored panel, suspended-card handling, click-away/Esc dismiss.

**Non-Goals:**
- Opponent-card command panels (no legal actions on opponent cards outside selections; right-click inspect already covers information).
- Replacing `SelectionPanel`/pending-selection flows — the panel covers *initiating* actions, not answering engine prompts.
- Engine/server changes; richer effect labels arrive via the in-flight serialization work and are consumed opportunistically.

## Decisions

### D1: Mask-derived menu model, one builder module

`utils/commandMenu.ts` exports `buildCommandMenu(cardRef, parsedMask, gameState) -> CommandMenuEntry[]`, where `cardRef` is `{ zone: 'hand'|'field'|'breeding', index }` and each entry is `{ label, kind, actionId | targetPick }`. Pure function over already-parsed mask + state (names, costs from state DTO), unit-testable without rendering. The component is a thin renderer.

### D2: Disambiguation between click-to-act and click-for-menu

Current `onSlotClick` submits a context-dependent action directly (e.g. attacker pick during attack flow, block target). Rule: **when the game is in a targeting/interrupt flow (pending selection, attack declaration, block/counter window), left-click keeps its current direct meaning; otherwise left-click opens the command panel.** A card with exactly zero legal actions opens the panel in "no actions" state showing a hint (and the hover preview continues to exist). This keeps selection flows unambiguous and makes the panel the default idle-state gesture. Alternative (always open panel, panel handles targeting) rejected: would add a click to every selection answer and fight the existing highlight-driven flows.

### D3: Target-pick entries

Multi-target actions ("Digivolve onto…", "Attack…") render as a single entry that, on activation, enters target-pick mode: the panel closes and legal targets highlight via the existing `fieldSelectionHighlights`/slot-glow machinery; clicking a highlighted slot submits the composed action; Esc cancels. When ≤3 targets exist, the panel MAY list them inline ("Attack: Security", "Attack: <name>") for one-click submission. This reuses the highlight system rather than inventing a second targeting UI.

### D4: Labels

- Play: "Play — <cost> memory" (cost from state DTO).
- Digivolve: "Digivolve onto <top-card name> (<cost>)" inline, or "Digivolve…" for target-pick.
- Attack: "Attack Security" / "Attack <name>".
- Effects: engine-provided label when present (`effectChoices[].label` pattern), else `[Main] Effect N` with timing tag; upgrade path: once `add-permanent-stack-inspector`'s effect-text serialization lands, derive labels from effect text headers.
- DNA digivolve, trash-from-hand, move-to/from breeding as applicable.

### D5: Panel presentation

Anchored popover near the clicked card (hand: above the card; field: beside the slot), clamped to the canvas, rendered above the board but below modal selection overlays. Suspended permanents: anchor accounts for rotated bounds (no rotated buttons — DCGO rotates because its panel is board-space; ours is screen-space). Dismiss on click-away, Esc, or any state change that invalidates the menu (mask refresh closes/rebuilds).

### D6: ActionBar slimming

Once effect activation moves into the panel, `ActionBar` keeps only global actions (Pass, Hatch shortcut, Mulligan, Surrender) — the generic per-source effect buttons are removed in this change to avoid duplicate affordances. Pass/turn-end stays global (DCGO parity: NextPhaseButton).

## Risks / Trade-offs

- [Gesture conflict with drag] → dnd-kit `PointerSensor` already uses an 8px activation distance; sub-threshold pointer-up is a click → panel. Test on touch (150ms delay sensor).
- [Gesture conflict with right-click inspector / hover preview] → left-click panel, right-click inspect, hover preview unchanged; add an explicit interaction test matrix in component tests.
- [Click-meaning ambiguity during interrupt windows] → D2 rule is centralized in one routing function in `GamePage` with unit tests per game-flow state.
- [Stale menu after state change (paced bot games)] → menu derives from current mask version; any mask refresh closes or rebuilds the panel.
- [Generic effect labels remain unhelpful until serialization lands] → acceptable; timing tags + source card name already beat today's global "Effect N", and labels upgrade automatically.

## Open Questions

- Inline-target threshold (≤3) vs always target-pick — tune during implementation with real boards.

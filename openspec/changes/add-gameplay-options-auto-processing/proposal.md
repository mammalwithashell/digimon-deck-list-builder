# Proposal: add-gameplay-options-auto-processing

## Why

DCGO ships a Gameplay Options panel whose toggles remove most of the per-click toil of playing a faithful TCG client — auto-resolving trivial choices (deck-bottom/top placements, min/max digivolve cost, auto-hatch), confirmation before committing selections, and animation skipping. Our UI has none of this: every forced or trivial choice the engine surfaces (correctly, per the no-approximations policy) is a manual click, which makes real play tedious for the players the UI is for. We copy DCGO's option set deliberately — it is the battle-tested spec for which choices players want automated.

## What Changes

- Add a **Gameplay Options panel** (settings page section + in-game access), with DCGO's toggle set adapted to our client:
  - **Auto-order trivial effects / deck operations**: auto-submit when a pending selection has exactly one legal action; auto-resolve order-only selections that have no hidden information or strategic impact (e.g. bottom-of-deck placement order) with a default order.
  - **Auto min/max digivolve cost**: when a digivolve cost choice exists, auto-pick the minimum (toggle).
  - **Auto-hatch**: automatically hatch at breeding phase when it is the only meaningful action (toggle).
  - **Check before ending selection**: confirmation prompt before submitting a multi-select (toggle, default off).
  - **Show cut-in / animations** (toggle): skip transient animations (PhaseBanner, DigivolveBanner, security reveal dwell times).
  - **Rotate suspended cards** (toggle): visual-only suspend rotation on/off.
- All automation is **strictly UI-side auto-submit**: the engine continues to surface every choice through `pending_selection` / the action mask (no-approximations rule untouched; RL action space unchanged). The UI merely answers some prompts automatically per the user's standing instruction.
- Persist all toggles in `uiStore` (localStorage), defaulting to DCGO-like behavior (automation on, confirmation off, animations on).
- Every auto-submitted choice is **visible**: it still produces its log line / trace entry so the player can audit what was auto-resolved.

## Capabilities

### New Capabilities
- `gameplay-options`: A persisted set of player-facing gameplay toggles (DCGO-parity: auto-order, auto min/max digivolve cost, auto-hatch, confirm-before-end-selection, animation skip, suspend-rotation) controlling UI-side automation and presentation.
- `ui-auto-resolution`: The UI-side auto-submit engine — rules for which pending selections are safe to answer automatically (single-legal-action, order-only-without-strategic-impact, cost-min/max), with the guarantee that automation never hides information or changes the engine-visible action space.

### Modified Capabilities
<!-- None. Engine surfaces and action space are untouched; this is a frontend-only layer. -->

## Impact

- **Frontend only**: `stores/uiStore.ts` (new persisted options slice), `pages/GamePage.tsx` (auto-submit hook on pending-selection changes), `components/game/SelectionPanel.tsx` (confirm-before-end), animation components (`PhaseBanner`, `DigivolveBanner`, `SecurityRevealOverlay`) honor the animation toggle, `Card.tsx`/`PermanentSlot.tsx` honor suspend-rotation, new `GameplayOptionsPanel` component reachable from settings and in-game.
- **Engine/server**: no changes. Auto-submitted actions go through the normal action submission path.
- **Risk surface**: misclassifying a strategic selection as auto-resolvable. The classification rules live in one auditable module (`utils/autoResolve.ts`) with unit tests per rule; default-on automation is limited to provably trivial cases (single legal action), everything else ships default-off.
- **Interaction with `add-bot-action-pacing`**: animation-skip and pacing compose (skip affects transient overlays, pacing affects bot step cadence); no shared code beyond `uiStore`.

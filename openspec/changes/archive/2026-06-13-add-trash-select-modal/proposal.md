## Why

Selecting card(s) from a trash (e.g. EX11-012 Medusamon, ST6-010 Skullsatamon, BT18 Millenniumon, promo Kotone, EX10-074 Beelzemon) is awkward today. Single-select trash opens a generic centered grid, while **multi-select trash effects render no selection surface at all** — the engine installs them in the `SelectBudgeted` phase, which the `SelectionPanel`'s `PANEL_PHASES` excludes. The only way to even see the cards is the read-only `TrashViewer` (opened from the board), which cannot select and visually blocks the affordance. The fix is to make the trash modal itself the real selector for both single- and multi-select, from either player's trash.

Worse, there was a confirmed **production soft-lock** (shipped desktop 0.3.2): on ST6-13 CresGarurumon's `[Main] <Digi-Burst 2>` forced "play 1 purple Lv.3 Digimon from trash" pick, the modal renders the trash cards but no click resolves the selection — the player's only recourse is to restart the game. Root-caused live (browser-dev repro + `document.elementFromPoint`): it is an **action-ID range mismatch**, not an overlay. The Rust engine encodes `SelectionKind::Trash` at `TRASH_EFFECT_START..END` (`1150..1195`), but the front-end's `SELECTION.TRASH_START` constant is the stale value `130`, so `SelectionPanel` gates every trash card on `actionMask[130+i]` (always 0) → all cards render `cursor-not-allowed` with no handler → unclickable. A forced trash pick (no PASS) then has no escape. **Fixed + verified** by correcting the constant to the engine's `1150/1194` range; this change folds that fix in and hardens the new modal to derive action IDs from the engine rather than a hardcoded base.

## What Changes

- Add a dedicated **`TrashSelectModal`** front-end component that becomes the selection surface for all trash selections (single and multi), targeting the `zone_owner`'s trash. The read-only board `TrashViewer` is retained for casual browsing.
- Support **single-select** (`SelectionKind::Trash`): click a card to dispatch immediately; `Decline` when optional.
- Support **multi-select** (`SelectionKind::CountCappedMultiSelect`) with a **true toggle + Done** interaction: clicks toggle a local ordered selection (real deselect, capped at `max`), and `Done` is enabled once the floor (`min`) is met. On confirm, picks are dispatched in order followed by a stop/`PASS` (deferred dispatch). A `distinct`-constrained selection (candidates that shrink between picks) falls back to immediate per-click commit with identical visuals, so the engine re-filters each step.
- **BREAKING (engine, internal only)**: extend the `CountCappedMultiSelect` selection-kind variant from `{ max, picked }` to `{ min, max, picked, distinct }` so the selection floor and distinct-constraint flag reach the UI. Both wires already serialize the kind via `format!("{:?}", kind)`, so no DTO struct changes are required; the new fields ride the kind string on both the browser (`serialization.rs`) and desktop (`engine_commands.rs`) DTOs.
- Remove `SelectTrash` (and its trash branch + `opponentTrashIds` prop) from `SelectionPanel`; it keeps hand / security / effect-choice / source-select.
- Restyle single-select trash to a unified trash-styled modal (shared look with multi-select), rather than the current generic centered grid.
- **Diagnose and fix the production soft-lock** so a forced trash selection (e.g. CresGarurumon ST6-13) is always resolvable from within the modal: every legal selection action must dispatch on the desktop build, no full-screen overlay may sit above the modal and capture clicks, and the player must never be left with "restart the game" as the only option.

## Capabilities

### New Capabilities
- `trash-selection-ui`: How the front-end exposes trash-card selection (single and capped multi-select, own or opponent trash) as an interactive modal, and the engine metadata (`min`, `distinct`) the multi-select interaction depends on.

### Modified Capabilities
<!-- None: no existing spec governs the selection UI; the engine variant change is captured by the new capability's requirements. -->

## Impact

- **Engine (Rust)**: `code/digimon-engine/src/selection.rs` (`CountCappedMultiSelect` variant), `code/digimon-engine/src/effect_context/selections.rs` (install site at ~3218 builds `min`/`distinct`). Any test that string-matches the old `CountCappedMultiSelect { max:` Debug form must be updated.
- **Front-end (React/TS)**: new `code/frontend/src/components/game/TrashSelectModal.tsx`; new parser util (`parseCountCappedKind`); edits to `SelectionPanel.tsx` (drop trash) and `GamePage.tsx` (mount the new modal). New/updated Vitest specs.
- **No DTO/serialization struct changes**; desktop and browser wires inherit the new kind fields automatically. No RL action-space change — every pick remains a real per-pick engine action (no-approximations preserved).
- **Soft-lock diagnosis** spans the desktop dispatch path (`code/src-tauri/src/engine_commands.rs` `rust_submit_action`), the front-end input gating (`GamePage.tsx` `sendAction` / `agentPending` / `actionPendingRef`), and overlay z-index/pointer-events layering (`SecurityRevealOverlay` z-50 and any other full-screen layer vs the modal's z-40). Reproduction uses the scenario MCP / desktop-dev to stage CresGarurumon ST6-13's Digi-Burst trash pick.

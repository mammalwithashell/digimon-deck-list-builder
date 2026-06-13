# trash-selection-ui Specification

## Purpose

Defines how the front-end exposes trash-card selection — single-select and capped multi-select, from either player's trash — as an interactive modal (`TrashSelectModal`), and the engine metadata (`min`, `distinct`) the multi-select interaction depends on. Selectability is gated on the engine action mask / `validIndices` (action id `TRASH_START + i`, where `TRASH_START` mirrors the engine's `TRASH_EFFECT_START`) rather than a hardcoded base, so an action-space shift can't silently soft-lock trash selection.

## Requirements

### Requirement: Interactive trash-selection modal

The front-end SHALL present an interactive modal (`TrashSelectModal`) as the selection surface for every engine selection whose candidates are trash cards, for the local player. The modal SHALL render the cards of the `zone_owner`'s trash (the opponent's trash when `zone_owner` is set and differs from the local player; otherwise the local player's trash) and SHALL map the card at trash index `i` to action id `TRASH_START + i` (the engine's trash-selection range; `TRASH_START` mirrors `TRASH_EFFECT_START`). A card SHALL be presented as selectable only when its action id is currently legal in the action mask. The modal SHALL allow right-clicking a card to open the enlarged card detail.

The modal SHALL open when the pending selection's `selecting_player` is the local player, the selection is not a keyword prompt, and either:
- the selection `kind` is `Trash` (single-select), or
- the selection `kind` is `CountCappedMultiSelect` and every valid action id falls within the trash action range `[TRASH_START, TRASH_END]`.

The read-only board trash viewer (`TrashViewer`) SHALL remain available for browsing and is independent of this modal.

#### Scenario: Single-select trash modal opens and dispatches

- **WHEN** the engine installs a `SelectionKind::Trash` pending selection for the local player (e.g. EX11-012 Medusamon returning a card from the opponent's trash)
- **THEN** the `TrashSelectModal` opens against the `zone_owner`'s trash, the legal cards are selectable, and clicking a selectable card dispatches action `TRASH_START + i` immediately

#### Scenario: Opponent trash is rendered when zone_owner differs

- **WHEN** a trash selection for the local player has `zone_owner` set to the opponent
- **THEN** the modal renders the opponent's trash list (not the local player's) so the action ids align with the displayed cards

#### Scenario: Optional single-select can be declined

- **WHEN** a single-select trash selection is optional (`is_optional` true, `DECLINE`/`PASS` legal in the mask)
- **THEN** the modal shows a decline control that dispatches the decline/pass action (62)

#### Scenario: Trash selection no longer routes through SelectionPanel

- **WHEN** any trash selection (single or multi) is pending for the local player
- **THEN** the `SelectionPanel` does not render a trash surface (it no longer handles `SelectTrash`), and the `TrashSelectModal` is the sole interactive trash selector

### Requirement: Capped multi-select with deferred toggle

For a `CountCappedMultiSelect` trash selection that is not `distinct`-constrained, the modal SHALL support a true toggle interaction with deferred dispatch. Clicking a selectable card SHALL toggle it within a local, ordered selection set (selecting and deselecting freely), capped so the number of locally-selected cards never exceeds `max`. The modal SHALL display the running count against `max` and SHALL indicate the required floor when `min > 0`.

A confirm ("Done") control SHALL be enabled only when the number of locally-selected cards is at least `min`. On confirm, the modal SHALL dispatch the selected cards' actions in selection order, and then — only when fewer than `max` cards were selected — dispatch the stop/`PASS` action (62); when exactly `max` cards were selected the final pick auto-commits and no stop action is sent. While dispatching, the modal SHALL freeze its grid and preserve the local selection until the engine clears the pending selection. The dispatch sequence SHALL function in both the awaited HTTP/desktop path and the fire-in-order WebSocket path.

#### Scenario: Toggle, deselect, and confirm an "up to N" trash selection

- **WHEN** the engine installs a `CountCappedMultiSelect { min, max, picked: 0, distinct: false }` trash selection (e.g. "return up to 3 cards from your trash") and the local player toggles three cards on, then toggles one back off, leaving two selected
- **THEN** the count shows 2 of `max`, Done is enabled (2 ≥ `min`), and confirming dispatches the two picks in order followed by the stop/`PASS` action

#### Scenario: Selecting the maximum auto-commits without a stop action

- **WHEN** the local player selects exactly `max` cards and confirms
- **THEN** the modal dispatches all `max` picks in order and does not dispatch a separate stop/`PASS` action, because the final pick auto-commits the selection

#### Scenario: Done gated by the minimum floor

- **WHEN** a `CountCappedMultiSelect` trash selection has `min > 0` and the local player has selected fewer than `min` cards
- **THEN** the Done control is disabled until at least `min` cards are selected

#### Scenario: Selection cap prevents over-selection

- **WHEN** the local player has `max` cards locally selected
- **THEN** further unselected cards are not addable (only deselection is permitted) until a selected card is toggled off

### Requirement: Engine exposes multi-select floor and distinct constraint

The engine's `CountCappedMultiSelect` selection-kind variant SHALL carry the effective minimum (`min`), the maximum (`max`), the running picked count (`picked`), and a boolean (`distinct`) indicating whether the selection applies a distinct-by constraint that removes candidates between picks. Because both the browser and desktop pending-selection wires serialize the selection kind via its debug representation, these fields SHALL be available to the front-end through the serialized `kind` string without additional DTO struct fields. The front-end SHALL parse `min`, `max`, `picked`, and `distinct` from the `kind` string.

#### Scenario: Multi-select metadata reaches the front-end on both wires

- **WHEN** a `CountCappedMultiSelect` trash selection is serialized for the browser (hosted-API serialization) or the desktop (Tauri engine command)
- **THEN** the emitted `kind` string includes `min`, `max`, `picked`, and `distinct`, and the front-end parses them to drive the modal's floor gating and dispatch mode

### Requirement: Trash selection must never soft-lock the client

A trash selection SHALL always be resolvable from within the modal on every build (browser and desktop/production). When a trash selection is pending for the local player, every legal action for that selection (each selectable card, and decline/`PASS` when legal) MUST dispatch and advance the game when invoked. The selectability and dispatch of each card MUST be derived from the engine action mask / `validIndices` (not a hardcoded action-id base), so an engine action-space change cannot leave the cards rendered-but-dead. No full-screen overlay SHALL sit above the trash-selection modal and capture pointer events: the modal's stacking context MUST be at least as high as any concurrent overlay (e.g. `SecurityRevealOverlay`), and animation/banner overlays that span the viewport MUST remain non-interactive (`pointer-events: none`). For a forced (non-optional) trash selection with at least one legal target, the player MUST be able to complete it by clicking a card without relying on the (modal-covered) action bar. The client MUST NOT reach a state where restarting the game is the only way to proceed.

#### Scenario: CresGarurumon forced trash pick resolves in the desktop build

- **WHEN** ST6-13 CresGarurumon's `[Main] <Digi-Burst 2>` resolves and installs the forced "play 1 purple Lv.3 Digimon card from your trash" selection in the desktop/production build
- **THEN** the highlighted cards are clickable and clicking one dispatches the pick and advances the game (no dimmed dead screen, no required restart)

#### Scenario: No overlay covers the selection modal

- **WHEN** a trash selection modal is open at the same time as any viewport-spanning overlay (security reveal, digivolve banner, effect popup, battle effect)
- **THEN** the selection modal receives clicks (it is not occluded by a click-capturing overlay), and non-interactive overlays do not intercept pointer events

#### Scenario: Optional trash selection is declinable from the modal

- **WHEN** an optional trash selection is pending and the action bar is hidden behind the modal backdrop
- **THEN** a decline/skip control inside the modal dispatches the decline/`PASS` action, so the player is never dependent on the covered action bar to back out

### Requirement: Distinct-constrained multi-select falls back to immediate commit

When a `CountCappedMultiSelect` trash selection is `distinct`-constrained, deferred dispatch cannot safely precompute the pick sequence because the candidate set shrinks between picks. In that case the modal SHALL commit each click immediately (dispatching one pick per click so the engine re-filters candidates for the next step) while presenting the same selected-card marking, running count, and Done control. The Done control SHALL dispatch the stop/`PASS` action when the engine reports the selection is optional, and the modal SHALL close automatically when the engine reaches `max` and auto-commits.

#### Scenario: Distinct selection commits per click and re-filters

- **WHEN** a `CountCappedMultiSelect { distinct: true }` trash selection is pending and the local player clicks a card
- **THEN** the pick is dispatched immediately, the engine re-installs the selection with the distinct-excluded candidates removed from the mask, and the clicked card is shown as selected without being toggleable off

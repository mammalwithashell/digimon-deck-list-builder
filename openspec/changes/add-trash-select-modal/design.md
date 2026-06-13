## Context

Trash selection is split across two front-end surfaces today:

- `SelectionPanel` (`code/frontend/src/components/game/SelectionPanel.tsx`) is a centered full-screen modal that opens only for phases in `PANEL_PHASES = {SelectHand, SelectTrash, SelectSecurity, SelectEffectChoice}`. It renders single-select trash (`SelectionKind::Trash`, phase `SelectTrash`) as a clickable grid, dispatching one action per click.
- `TrashViewer` (`code/frontend/src/components/game/TrashViewer.tsx`) is a separate **read-only** modal opened from the board trash pile.

The engine models trash selection in two ways:

- **Single-select** — `SelectionKind::Trash`, phase `SelectTrash`. Action ids are `TRASH_START + i` (130 + i) into the `zone_owner`'s trash; `zone_owner` already distinguishes own vs opponent (EX11-012 Medusamon picks from the opponent's trash).
- **Multi-select** — `SelectionKind::CountCappedMultiSelect { max, picked }`, phase **`SelectBudgeted`**, installed at `code/digimon-engine/src/effect_context/selections.rs:~3215`. `SelectBudgeted` is **not** in `PANEL_PHASES`, so no selection modal renders — the player can only open the read-only `TrashViewer`. This is the broken case the change targets.

Key engine behaviors (verified in `selections.rs`, `install_count_capped_step`):

- Multi-select is **iterative**: each pick dispatches one action, the engine re-installs the prompt with `picked` incremented and that candidate removed, and the player `PASS`es to stop (or it auto-commits at `max`).
- Picked cards **stay physically in the trash** during the selection — only `candidate_indices` shrinks — so a trash card's action id (`TRASH_START + trash_index`) is **stable** across re-prompts; picking it just drops the id from `valid_action_ids` (see the comment at `selections.rs:~3267`).
- The **effective floor** `effective_min = min.max(is_optional_zero ? 0 : 1)` is computed at install time and is only reflected per-step via `is_optional`. Neither `min` nor an "unpick" action is exposed.
- A `distinct_by` constraint removes remaining candidates by attribute after each pick (`selections.rs:~3296`), so the candidate set shrinks in ways the front-end cannot predict from the initial prompt alone.

Both pending-selection wires serialize the kind via its debug form: browser `serialization.rs` (`PendingSelectionView::kind_str()` → `format!("{:?}", kind)`) and desktop `code/src-tauri/src/engine_commands.rs:666` (`format!("{:?}", sel.kind)`). The front-end already treats `kind` as a string it pattern-matches (`utils/selectionTargets.ts`).

## Goals / Non-Goals

**Goals:**
- A single interactive modal that is the real selector for all trash selections (single + capped multi), for either player's trash.
- True toggle-with-deselect for the common capped multi-select case, with a Done/confirm gated by the real floor.
- Faithful to the engine: every pick remains a real per-pick engine action (no-approximations / RL parity preserved).
- No DTO struct churn; desktop and browser stay in sync automatically.

**Non-Goals:**
- Changing how the engine resolves multi-select (still iterative pick → re-prompt → PASS).
- Adding a native "unpick" action to the engine.
- Touching non-trash selections (hand/security/effect-choice/source stay in `SelectionPanel`; field/board routing unchanged).
- Reworking the read-only `TrashViewer` (kept for browsing).

## Decisions

### D1 — Dedicated `TrashSelectModal`, gated on selection kind, not phase
A new component owns all trash selection; `SelectionPanel` drops `SelectTrash`. Gating on `kind` (`Trash`, or `CountCappedMultiSelect` with all valid ids in `[130,179]`) rather than phase is robust to the engine putting multi-select in `SelectBudgeted`, and matches the existing kind-based routing pattern in `selectionTargets.ts`.

*Alternatives:* extend `SelectionPanel` to also open for `SelectBudgeted` (rejected — overloads a component that already branches across four phases, and `SelectBudgeted` also carries non-trash budget kinds); unify `TrashViewer` + selection into one component (rejected per product choice — keep browsing separate).

### D2 — Expose `min` and `distinct` by widening the variant
Change `CountCappedMultiSelect { max, picked }` → `CountCappedMultiSelect { min, max, picked, distinct }`, populated at the single install site with `min = effective_min` and `distinct = distinct_by.is_some()`. Because both wires emit `format!("{:?}", kind)`, the new fields reach the front-end for free; the front-end parses them with a small regex (`parseCountCappedKind`).

*Why:* deferred dispatch only ever observes the initial prompt (`picked = 0`), so it cannot learn the floor from per-step `is_optional`. The variant is matched in exactly two places (`selection.rs:156` definition, `selections.rs:3218` install), so widening is cheap.

*Alternatives:* add structured `select_min`/`select_max`/`distinct` fields to `PendingSelection`/`View` and both DTOs (rejected — more surface, touches both serializers); keep `min` off the wire and enable Done at ≥1 (rejected — a `PASS` below the floor is illegal and would dangle the pending selection).

### D3 — Deferred toggle for the common case; immediate-commit fallback for `distinct`
For `distinct = false`, the modal holds a local ordered selection (true toggle, capped at `max`), and on Done dispatches the picks in order then `PASS` (skipping `PASS` when exactly `max`, which auto-commits). Action ids are stable and picked cards stay in the zone, so the precomputed sequence stays valid.

For `distinct = true`, candidates shrink unpredictably between picks, so deferred precomputation could dispatch a now-illegal pick. The modal instead commits each click immediately (engine re-filters per step) with identical visuals (marked card, running count, Done = `PASS` when `is_optional`, auto-close at `max`).

*Alternatives:* always immediate-commit (rejected per product choice — no pre-confirm deselect); always deferred + handle mid-drain rejection by skipping (rejected — silently alters the user's intended set).

### D4 — Dispatch sequencing reuses existing `handleAction`
The drain loop does `for (id of picked) await onAction(id)` then conditionally `await onAction(62)`. In HTTP/desktop mode `sendAction` awaits the state round-trip (and its `actionPendingRef` guard would otherwise drop a second un-awaited action), so awaiting is required; in WebSocket mode `ws.sendAction` is fire-and-forget but ordered, so the same awaited loop dispatches in order. A `committing` guard freezes the grid and prevents local state reset while intermediate `CountCappedMultiSelect` re-prompts arrive; the modal closes when the engine clears the pending selection. Per-pick `humanStillDeciding` (selecting_player stays local) keeps the agent step from firing mid-batch.

### D5 — Soft-lock root cause: stale trash action-ID constant (DIAGNOSED + FIXED + VERIFIED)

The soft-lock was reproduced live (browser-dev + scenario MCP staging CresGarurumon ST6-13's Digi-Burst → forced "play 1 purple Lv3 from trash") and root-caused with `document.elementFromPoint`. It is **not** an overlay/stacking occlusion (my earlier hypothesis — recorded here as retired so the wrong trail isn't re-walked). `elementFromPoint` on a trash card returned the card image itself (`topSame: true`) — nothing occludes it.

The real cause is an **action-ID range mismatch**:
- The Rust engine encodes `SelectionKind::Trash` selections in the `TRASH_EFFECT_START..TRASH_EFFECT_END` range (`code/digimon-engine/src/action/space.rs` → `1150..1195`). For the repro, `pendingSelection.validIndices = [1150,1151,1152,1153,1154]` and `actionMask[1150..1154] = 1`.
- The front-end `SelectionPanel` maps trash card `i` to `SELECTION.TRASH_START + i` = **`130 + i`** (a stale constant), and gates clickability on `actionMask[130+i]` — which is **always 0**. Every trash card therefore renders `cursor-not-allowed` with **no `onClick`** → unclickable. For a *forced* trash pick (no PASS in the mask) the only legal actions are the (dead) cards + concede → the player must restart. Hand/security selections work because their constants still match the engine ranges.

**Fix (applied + verified):** correct `SELECTION.TRASH_START/END` in `code/frontend/src/utils/constants.ts` from `130/179` to the engine's `1150/1194`. This fixes both consumers — `SelectionPanel.tsx:104-105` (the soft-lock) and `useActionMask.ts:141` (which built `validSelections` from the dead range). Verified live: after the fix the 5 valid Lv3 cards became `cursor-pointer`, the Lv5 card stayed correctly greyed, and clicking a card resolved the selection (DemiDevimon played from trash, modal closed, back to Main).

**Hardening (carried into the new modal):** the only branches that *didn't* drift are the ones that consume engine-provided IDs directly (effect-choice uses `choice.actionId`; source-select uses `validIndices`). The new `TrashSelectModal` SHALL likewise derive each card's action ID from the engine — map the `zone_owner`'s trash card at index `j` to the `validIndices`/mask entry for that zone slot rather than trusting a hardcoded base — so a future action-space shift can't silently soft-lock trash selection again.

## Risks / Trade-offs

- **Stale Rust tests string-matching the old kind Debug form** → grep for `CountCappedMultiSelect { max` in `code/digimon-engine` and update to the new field set as part of the change.
- **Opponent-trash multi-select**: the install site currently sets `zone_owner: None` for `CountCappedMultiSelect`, so the modal defaults to the local player's trash. This matches today's cards (multi-select recovery/play targets the owner's own trash). → If a future card needs capped multi-select from the *opponent's* trash, the install site must set `zone_owner`; the modal already honors it.
- **WebSocket ordering**: deferred dispatch relies on the socket preserving message order; picks are validated server-side against the evolving state. → Action ids are stable, so ordered delivery is sufficient; no per-step await is needed in WS mode.
- **Regex-parsing the Debug string** is mildly brittle if the variant's debug shape changes → keep `parseCountCappedKind` tolerant (named-field regex) and unit-tested; both wires share the one `format!("{:?}", kind)` source.
- **`distinct` fallback loses pre-confirm deselect** for those specific cards → acceptable; the visual contract is identical and these selections are rare.

## Why

PR #533 (commit `e303dc43`, 2026-05-23) closed exactly **one** of many code paths that leave a "zombie permanent" — a `Permanent` whose `card_sources` Vec is empty but whose slot is still in `Player::battle_area`. Within hours of the fix landing, a new generalist training run (`pilot_ppo_20260523_215003`) captured 10 fresh `Permanent must have at least one card` panics from the **same** panic family (`G-PERMANENT-EMPTY-DURING-BATCH-DELETION`), at the same order-of-magnitude rate as before the fix. Static audit confirms the gaps.md prediction: at least 6 other production paths empty `card_sources` without the carrier-slot cleanup, and several unguarded read-side callers still panic on zombie carriers. The training run is the forcing function — every panic costs one game's worth of training samples, and the panic family blocks long-running training to convergence.

## What Changes

- Close 6 remaining mutation sibling sites that empty a Permanent's `card_sources` without removing the slot, using the existing `Game::soft_remove_if_emptied` cleanup pattern landed in PR #533:
  1. `EffectContext::play_from_materials_suppress_on_play` (effect_context/mod.rs:3329) — explicit sibling flagged in engine-gaps.md
  2. `Game::place_as_bottom_source_observed` (game_actions.rs:4426) — Save / Stash / BottomReturn flow
  3. Replacement→Trash redirect branch (game_actions.rs:6141)
  4. Place-into-security from material (game_actions.rs:6192)
  5. `Game::trash_source_ref` (game.rs:1058) — self-trash-source effects (Rocks archetype)
  6. `EffectContext::trash_card_source` (effect_context/mod.rs:4028) — targeted source-trash
- Extend Layer 2 (read-side) zombie-tolerance guards to the two remaining unguarded battle-area-iterating callers in the effect queue:
  1. `find_event_gated_delay_permanent` (effect_queue.rs:2361) — likely dominant production panic site
  2. `event_gated_delay_source` (effect_queue.rs:2327)
- Add behavioral regression tests mirroring the existing digivolve-from-material zombie tests, one per closed mutation sibling.
- Update tracking artifacts: revert the premature `RESOLVED` strike-through on the `G-PERMANENT-EMPTY` entry in `qa/archetype-qa/engine-gaps.md` (or split the family into a resolved digivolve-from-material entry + an open siblings entry), and bring `qa/archetype-qa/panic-families.json` in sync.

Non-goals (deferred to a separate change):
- The architectural refactor of `Permanent::top_card()` to return `Option<&CardSource>` (DCGO's null-check pattern). This change keeps the per-site cleanup pattern PR #533 established; the architectural refactor is a parallel track captured in engine-gaps.md "Family-wide note".

## Capabilities

### New Capabilities
- `zombie-permanent-cleanup`: The engine's contract that any code path which empties a permanent's `card_sources` must remove the carrier slot from `battle_area` before any trigger fan-out can observe it, plus the read-side defensive guards on trigger-queue callers that iterate `battle_area`.

### Modified Capabilities
<!-- None. `permanent-deletion-semantics` is about batched deletion (PR #525); the zombie-permanent class addressed here is about material extraction (non-deletion), so it is its own capability. -->

## Impact

- **Code**: `code/digimon-engine/src/effect_context/mod.rs`, `code/digimon-engine/src/game.rs`, `code/digimon-engine/src/game_actions.rs`, `code/digimon-engine/src/effect_queue.rs`.
- **Tests**: new behavioral regressions in `code/digimon-engine/tests/effect_context/play_from_materials.rs`, `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs` (extend), plus a new file for the trash / replacement / place-as-bottom-source paths.
- **Docs**: `qa/archetype-qa/engine-gaps.md` (revert premature RESOLVED), `qa/archetype-qa/panic-families.json` (keep `status: "open"` until siblings close, then resolve).
- **Cards affected** (no card-script changes needed; engine fix transparently restores correctness):
  - `play_from_materials` users: BT22-015 Omnimon, BT13-110, BT13-112, BT20-083, BT23-072, EX4-060, EX9-021 (8 YAML cards)
  - Save / Stash / BottomReturn cards (Save is a common keyword across the format)
  - Self-trash-source effects in the Rocks archetype (currently triggering this in training)
- **Behavior change for callers**: an effect that previously left a zombie carrier silently in `battle_area` now removes the carrier slot. Linked cards on the removed carrier flow to trash and fire `OnLinkedCardTrashed` per the established `soft_remove_if_emptied` contract. In-flight `PermanentHandle` indices into the same player's battle area may shift down by 1 — callers that hold such handles past the mutation must use `Game::shift_handle_after_soft_remove` (as the digivolve fix already does).
- **Risk**: medium. The cleanup runs in well-trafficked code paths (Save, Stash, self-trash-source). Existing zombie regression tests, plus new ones per sibling, gate the change. The rollback path in `play_from_materials_suppress_on_play` needs special handling: soft-remove must not run before play success is confirmed, since the `PlayFromHandCostResult::Failed` branch re-inserts the source back into the carrier's stack.

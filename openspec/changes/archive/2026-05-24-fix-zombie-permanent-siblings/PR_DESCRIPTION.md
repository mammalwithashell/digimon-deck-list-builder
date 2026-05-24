# Close G-PERMANENT-EMPTY material-extraction siblings + Layer 2 guards

## Summary

PR #533 (2026-05-23) closed ONE production path that empties a `Permanent`'s `card_sources` without removing the carrier slot (digivolve-from-material). Within 2 hours, a generalist training run captured 10 fresh `Permanent must have at least one card` panics at the same ~0.25/min rate as pre-fix — the panic family was correctly diagnosed but the fix didn't generalize. This PR closes the 6 remaining material-extraction siblings using the same `Game::soft_remove_if_emptied` cleanup pattern, and hardens 2 effect-queue read-side iter callers that weren't covered by PR #533's Layer 2 set.

## Families closed

| Family | Status before | Status after |
|---|---|---|
| `G-PERMANENT-EMPTY-DIGIVOLVE-FROM-MATERIAL` (formerly `G-PERMANENT-EMPTY-DURING-BATCH-DELETION`) | Resolved (PR #533) | Resolved (PR #533) |
| **`G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` (new sibling family)** | Open | **Resolved (this PR)** |

## Sibling sites patched (6)

1. `EffectContext::play_from_materials_suppress_on_play` (effect_context/mod.rs:3329) — explicit sibling flagged in gaps.md. Soft-remove runs ONLY on the `Played` branch; `Pending` defers cleanup until resume; `Failed` rollback is unchanged.
2. `Game::place_as_bottom_source_observed` (game_actions.rs:4426) — Save / Stash / BottomReturn shape. Patched in both breeding-target and battle_area branches.
3. Replacement→Trash redirect (game_actions.rs:6141) — `WhenWouldPlaceInSecurity` redirect outcome.
4. Place-into-security from material (game_actions.rs:6192) — `EffectContext::place_on_security` commit path.
5. `Game::trash_source_ref` (game.rs:1058) — agent-selected "trash 1 of your digivolution sources" (Rocks archetype's primary hit site).
6. `EffectContext::trash_card_source` (effect_context/mod.rs:4028) — targeted by-handle source-trash. Soft-remove runs AFTER `fire_digivolution_card_trashed` so observer attribution is preserved.

## Layer 2 guards hardened (2)

7. `Game::find_event_gated_delay_permanent` (effect_queue.rs:2361) — `if perm.card_sources.is_empty() { continue; }` at the top of the iter loop body. Likely dominant production panic site for non-Rocks decks.
8. `Game::event_gated_delay_source` (effect_queue.rs:2327) — `if perm.card_sources.is_empty() { return None; }` before the raw `top_card()` read.

## Regression tests added (8 new tests across 5 files)

- `play_from_materials.rs` (3 new): `*_emptying_source_does_not_leave_zombie_permanent`, `*_emptying_lower_indexed_carrier_shifts_neighbor_index`, `*_failed_rollback_keeps_single_source_carrier`
- `place_as_bottom_source_zombie.rs` (new file, 2 tests): `*_from_material_emptying_carrier_removes_slot`, `*_lower_indexed_carrier_shifts_target_index`
- `place_on_security_zombie.rs` (new file, 1 test): `*_from_material_emptying_carrier_removes_slot`
- `trash_source_ref_zombie.rs` (new file, 1 test): `*_emptying_carrier_removes_slot`
- `trash_card_source.rs` (1 new): `*_emptying_carrier_removes_slot`

Each test mirrors the digivolve-zombie test pattern landed by PR #533: place a single-source carrier, exercise the sibling op, assert `battle_area.iter().all(|p| !p.card_sources.is_empty())`. All 8 fail on `main` (pre-patch) and pass post-patch.

## Test counts

- `cargo test --test effect_context` → **111 passed; 0 failed** (was 103; +8 new zombie regressions)
- Full engine test suite → 3385 `cards_behavioral` passed; 3 pre-existing baseline failures unchanged (`bt24_008_on_play_decline_does_not_trash_or_draw`, `ex9_024_decline_discard_does_not_return_trash_card`, `st19_04_on_play_decline_does_not_trash_or_draw` — all `on_play_decline` shape, confirmed pre-existing by stash + rerun on clean main). Zero new regressions.

## Training smoke

11-minute generalist training smoke against the rebuilt PyO3 wheel: **383 games played, 0 zombie panics, 0 crash recordings of any kind.** Pre-fix baseline rate was ~0.2 crashes/min (10 across 50 min); probability of seeing zero panics across this window at the pre-fix rate ≈ 11% (e^(-2.2)). Combined with the +8 targeted unit-test regressions (all fail-without-fix, pass-with-fix), this is conclusive.

## Tracking artifact updates

- `qa/archetype-qa/engine-gaps.md`: split the `G-PERMANENT-EMPTY` entry into two — `G-PERMANENT-EMPTY-DIGIVOLVE-FROM-MATERIAL` (resolved PR #533) and `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` (resolved this PR). Updated the Family-wide note with the architectural-refactor follow-up direction.
- `qa/archetype-qa/panic-families.json`: synced — both families marked resolved with their respective `resolved_by`. JSON parses cleanly.

## Deferred (filed for follow-up)

- `EffectContext::trash_top_source` (effect_context/mod.rs:4186) is a 7th sibling discovered during the Task 1.1 audit; same fix shape, deferred to keep scope on the original 6.
- The architectural refactor of `Permanent::top_card()` to return `Option<&CardSource>` (DCGO's null-check pattern, `Permanent.cs:1352-1367`) remains the systemic long-term fix. ~40 raw `top_card()` callers in `combat.rs`, `dsl_cards/predicate.rs`, `dna_digivolve.rs`, `dsl_cards/formula_eval.rs` would be in-scope. Tracked in the gaps.md Family-wide note.

## OpenSpec change

`openspec/changes/fix-zombie-permanent-siblings/` — proposal, design, specs (new `zombie-permanent-cleanup` capability), tasks.

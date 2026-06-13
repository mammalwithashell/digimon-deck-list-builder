# Archetype DSL Implementation: aegiomon-2 (BT25 Olympos XII Lv.6 / BEATBREAK slice)
Date: 2026-06-06
Total cards in slice: 6
Processed this run: 6
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 3 (BT25-044, BT25-028, BT25-059)
- PARTIAL: 3 (BT25-043, BT25-077, BT25-084)
- BLOCKED: 0
- SKIPPED (prior verdict): 0

All 6 cards ship YAML + behavioral tests; 42 tests pass
(`cargo test --test cards_behavioral -- bt25_028 bt25_043 bt25_044 bt25_059 bt25_077 bt25_084`).

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| BT25-043 | Habakirimon | IMPLEMENT | PARTIAL (dsl) | 8 | Recovery+1 + most-sec trash→unsuspend + Glowing-Dawn leave-prevention ship; BEATBREAK Option [Main]/Arts side omitted |
| BT25-044 | Junomon | IMPLEMENT | IMPLEMENTED | 6 | cost-reduce(sec-sum) + place other Digimon as sec top→trash both + on_lose_security play-free |
| BT25-028 | Dianamon | IMPLEMENT | IMPLEMENTED | 7 | cost-reduce + mass CannotSuspend(≤1 src)+delete unsuspended + OPT DNA-into-hand + inherited WA lock |
| BT25-059 | Ceresmon | IMPLEMENT | IMPLEMENTED | 7 | cost-reduce(≥2 susp) + suspend up-to-2 + effect-immunity susp Veg/TS + on_suspend -3000×susp debuff |
| BT25-077 | Bacchusmon | IMPLEMENT | PARTIAL (dsl) | 7 | play TS≤6000 free + OPT suspend + effect-gated delete-lowest; cost-reduce(board level-sum) omitted |
| BT25-084 | Titamon | IMPLEMENT | PARTIAL (engine) | 7 | trash-1-hand→delete all highest DP (+effect sec trash) + leave-prevent trash-2-hand; on-hand-trashed clause omitted |

## Engine-Gap Blocked / Partial Cards
### BT25-084 Titamon — G-ENGINE-ON-DISCARD-HAND
- Effect: "[All Turns] When your hand is trashed from, delete 1 of your opponent's lowest DP Digimon."
- Missing engine API: no `OnDiscardHand`/`OnTrashFromHand` EffectTiming observer (only `OnTrash` exists, no hand-specific trigger).
- Suggested addition: an `OnDiscardHand` EffectTiming fired by the hand-trash code path + matching DSL `on_discard_hand` timing. (Logged qa/archetype-qa/engine-gaps.md.)

## DSL-Vocab-Gap Partial Cards
### BT25-043 Habakirimon — G-DSL-BEATBREAK-ARTS-OPTION
- No dual Digimon+Option identity / `arts_digivolve` alt-path. Option [Main] DP-debuffs + Arts Digivolve omitted (BT25-041 precedent).
### BT25-077 Bacchusmon — G-DSL-BOARD-LEVEL-SUM
- No board-wide level-sum predicate; the "12+ total levels" cost-reduction condition omitted rather than approximated.
### BT25-084 Titamon — G-DSL-SELF-COLOR-COUNT-LTE
- No "distinct colors <= N" / "without 3 colors" base filter; the "[Titamon] w/o 3 colors: Cost 2" alt-path omitted (standard Lv.5 Purple + Lv.5 [TS] cost-4 ship).

## Documented Modelling Nuances (shipped IMPLEMENTED)
### BT25-028 Dianamon / BT25-059 Ceresmon — G-DSL-PLAYER-CANNOT-SUSPEND-FILTER
- DCGO installs a player-level CannotSuspend / effect-immunity with a dynamically re-evaluated permanent condition. Modelled as a `for_each` snapshot applying the per-target modifier at install time (standard DSL "all of them" idiom). Practical per-turn outcome matches; the late-entrant re-check nuance is the only divergence.

## Substrate notes (reused, no new vocabulary authored)
- `recover`, `trash_top_security`, `select_effect_choice` security-count gated branches (Habakirimon).
- `place_permanent_on_security` (field permanent → security top) as a cost, `select_union_zone`+`play_union_bound_free`, `cost_reduction` with `card_count_in_zone` formula in a `DpConstraint` (Junomon, Dianamon, Ceresmon).
- `for_each` over an aggregate (`highest_dp`/`materials_count_lte`/suspended/Veg-TS) + `add_modifier`/`grant_effect_immunity`/`delete_permanent`; `may_dna_digivolve_now` (DNA into a hand card); `select_count_capped_multi` over `of: any` battle area + `per_selected suspend`; `suspended_count` formula in an `add_dp_modifier`.
- `select_opponent_sources` (0..4 distributed) + `trash_selected_sources`; `select_hand` with `cost: true` (decline-aborts) + `trash_from_hand_by_index`; `when_would_leave_battle_area` replacement + `cancel_replacement`.

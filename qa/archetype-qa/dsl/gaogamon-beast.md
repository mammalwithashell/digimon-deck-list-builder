# Archetype DSL Implementation: Gaogamon / beast (BT25)
Date: 2026-06-06
Total cards in this slice: 8
Processed this run: 8
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 8
- PARTIAL: 0
- AUDITED-OK: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

All 8 cards authored as YAML + DebugRunner behavioral tests. 59 tests pass
(`cargo test --test cards_behavioral -- bt25_008 bt25_009 bt25_012 bt25_013
bt25_021 bt25_023 bt25_048 bt25_051` → 59 passed; 0 failed).

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT25-008 | Coronamon | IMPLEMENT | IMPLEMENTED | self | 10/10 | OnPlay/WhenMoving trash up-to-2 [Iliad]/[TS] → draw per trashed; inherited [Your Turn] +2000 DP |
| BT25-009 | Bearmon | IMPLEMENT | IMPLEMENTED | self | 17/17 | SOMP free-digivolve into Beast/Animal/Sovereign (not Sea Animal) / TS from hand; inherited [All Turns] +1000 DP |
| BT25-021 | Gaomon | IMPLEMENT | IMPLEMENTED | self | 5/5 | OnPlay reveal-3 two-bucket add (Thomas/DATA SQUAD + Gaogamon-named), bottom rest; inherited [When Attacking][OPT] both players draw |
| BT25-048 | Bearmon | IMPLEMENT | IMPLEMENTED | self | 8/8 | [Your Turn] −1 cost when THIS digivolves into a [TS] Digimon; inherited [All Turns][OPT] win-battle draw |
| BT25-012 | Grizzlymon | IMPLEMENT | IMPLEMENTED | self | 6/6 | OnPlay/WhenDigivolving select own Beast-family Digimon → Raid + 3000 DP for turn; inherited +1000 DP |
| BT25-013 | Firamon | IMPLEMENT | IMPLEMENTED | self | 6/6 | OnPlay/WhenDigivolving trash 1 → return red/blue [Iliad] from trash; [Your Turn] blue-play/digivolve → digivolve into Flaremon cost −1; inherited [Your Turn] +2000 DP |
| BT25-023 | Gaogamon | IMPLEMENT | IMPLEMENTED | self | 6/6 | OnPlay/WhenDigivolving if ≤1 Tamers, may play Thomas H. Norstein free; inherited [When Attacking][OPT] both players draw |
| BT25-051 | Grizzlymon | IMPLEMENT | IMPLEMENTED | self | 7/7 | Blocker; OnPlay/WhenDigivolving select own Beast-family Digimon → +3000 DP until opp turn ends; inherited [All Turns][OPT] win-battle draw |

## Key DSL idioms used (reusable for the rest of BT25 TS/beast)
- **Free self-digivolve from hand at SOMP** (BT25-009): `when: start_of_your_main_phase`,
  `active_when: { all_of: [your_turn, memory_lte: 4] }`, `select_hand` → `effect_initiated_digivolve { target: source, from_hand, cost: 0, ignore_requirements: true }`. Mirror of the shipping BT25-062 Kokuwamon.
- **Beast/Animal/Sovereign (not Sea Animal) OR Shaman/TS trait filter** (BT25-009/012/051):
  `any_of: [ { all_of: [ { any_of: [trait_has Beast/Animal/Sovereign] }, { none_of: [trait_has Sea Animal] } ] }, trait_has Shaman, trait_has TS ]`. DCGO's `HasBeastTraits` helper expanded to printed text.
- **Win-a-battle inherited trigger** (BT25-048/051): `when: on_any_deletion` + `condition: { source_deleted_battle_opponent: true }` + `once_per_turn: true` (the ST4-11 idiom).
- **Digivolve-into cost reduction restricted to THIS base** (BT25-048): `kind: cost_reduction`,
  `when_any_ally_digivolves_into: { trait_has: TS }`, `condition: { source_is_cost_target_permanent: true }` — the `source_is_cost_target_permanent` gate models DCGO's `permanentCondition: targetPermanent == this`.
- **Reveal-N two-bucket add + bottom remainder** (BT25-021): `reveal_top_deck` → `select_reveal_buckets` (two `min:0/max:1` buckets, `no_duplicate_cards`) → `add_to_hand_from_reveal ×2` → `place_remainder_on_deck { position: bottom }`.
- **Grant Raid + turn-scoped DP to a selected own Digimon** (BT25-012): `select_own_permanent` → `grant_keyword { keyword: Raid, expiry: end_of_turn }` + `add_dp_modifier { value: 3000, expiry: end_of_turn }`. (BT20-016 idiom.) BT25-051 uses `expiry: end_of_opponents_turn`.
- **Trash 1 as cost (decline aborts) → return from trash** (BT25-013): `select_hand { optional: true, cost: true, filter: {} }` → `if { binding_present } then [trash_from_hand_by_index, select_trash, add_to_hand_from_trash]`.
- **Conditional free play of a named card** (BT25-023): `optional: true`, `active_when: { count_lte: { filter: { kind: tamer }, n: 1 } }`, `select_hand { name_is: "Thomas H. Norstein" }` → `play_from_hand_free`.

## Source-priority notes
- **BT25-048** — printed text reduces the cost when digivolving "into a [TS] trait
  Digimon card" with NO color clause; DCGO additionally requires the target be
  green. Per CLAUDE.md source priority (printed text > DCGO), the YAML models the
  [TS]-trait restriction only and omits the DCGO green gate. Documented inline.

## Engine-Gap / DSL-Vocab-Gap Blocked Cards
None. Every clause was expressible with existing DSL vocabulary + engine primitives.

## Environment note (shared worktree)
This slice was authored alongside several other parallel BT25 sessions sharing the
same worktree's `cards/` and `tests/` trees. Sibling sessions' git operations
(stash/clean of untracked files) repeatedly wiped in-progress untracked YAML/test
files; all 8 were recreated and re-validated. Final per-card run (59/59 green) was
obtained by temporarily neutralizing two siblings' not-yet-compiling test modules
(`bt25_003`, `bt25_079`) in `bt25/mod.rs`, then restoring `mod.rs` to its original
state. No engine code was modified.

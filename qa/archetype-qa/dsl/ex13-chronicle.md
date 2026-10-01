# Archetype DSL Implementation: EX13 — Chronicle
Date: 2026-10-01
Total cards in pool: 5
Processed this run: 5
Pipeline: batch-implement-cards-rust-dsl (EX13 author-set slice "Chronicle")

## Summary
- IMPLEMENTED: 5
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| EX13-006 | Dorimon | IMPLEMENT | IMPLEMENTED | self-review | 9/9 | Inherited [End of Your Turn][OPT]: pay 1 (outer confirm), then optional unsuspend of a suspended [X Antibody]/[Chronicle] Digimon. Matches DCGO EX13_006.cs. |
| EX13-049 | Dorumon | IMPLEMENT | IMPLEMENTED | self-review | 16/16 | Reveal 3 / add 1 trait card / rest top-or-bottom; inherited [When Attacking][OPT] -2000; [Dorimon] + Black Lv.2 w/[X Antibody] cost-0 routes (engine prompts the route choice against the cost-1 circle). |
| EX13-055 | Raptordramon | IMPLEMENT | IMPLEMENTED | self-review | 15/15 | [OP][WD] -3000; [When Attacking] optional digivolve into a [Chronicle] card from hand ∪ trash at printed cost; inherited ＜Barrier＞. |
| EX13-057 | Grademon | IMPLEMENT | IMPLEMENTED | self-review | 22/22 | Reboot + Blocker until opp turn ends; during an attack also opp-Digimon-effect immunity + +5000 (BT20-053 DCGO precedent); [End of Attack][OPT] digivolve; inherited [All Turns][OPT] [Chronicle] leave-prevention by trashing top security. |
| EX13-072 | Kota Domoto | IMPLEMENT | IMPLEMENTED | self-review | 13/13 | Start-of-main trash 1 [Chronicle] → Draw 1 + 1 memory; [Chronicle] ally attacks → suspend to use [X Antibody] (by name) or a [Chronicle] Option at cost −1; [Security] play free. |

Shared fixtures: `code/digimon-engine/tests/cards_behavioral/ex13/chronicle_support.rs`.
DCGO: only EX13-006 has a script at b9a0638cd; the other four follow printed text +
the closest DCGO-backed siblings (BT20-053, BT24-101, BT25-092, EX12-066, BT18-092).

## Engine-Gap Blocked Cards
None.

## DSL-Vocab-Gap Blocked Cards
None. Two small vocabulary additions were made instead (substrate widened, see
`qa/dsl-vocab-gaps.md` "slice Chronicle"):
- `during_attack:` game-level predicate (G-DSL-DURING-ATTACK) — EX13-057.
- `can_digivolve_onto: source` magic name (G-DSL-CAN-DIGIVOLVE-ONTO-SOURCE) — EX13-055, EX13-057.

## Findings logged (not blocking)
- G-DSL-COST-SELECT-NO-CANDIDATES-RUNS-TAIL (`qa/archetype-qa/engine-gaps.md`): an
  optional `select_hand { cost: true }` with zero candidates runs the paid tail.
  EX13-072 gates its offer on a payable hand; BT18-092/093-shaped cards are exposed.
- `effect_initiated_digivolve { cost: printed }` takes the cheapest legal route without a
  prompt (noted under the same entry).

## New Patterns Discovered
- Turn-passing in DebugRunner tests: `end_turn()` with the gauge on the ending
  player's side (memory ≥ 0) makes the next player start on the wrong side and the
  harness rotates straight back. Set `game.memory = -3` first (`next_turn` helper in
  `chronicle_support.rs`) to land in the opponent's turn with 3 memory.

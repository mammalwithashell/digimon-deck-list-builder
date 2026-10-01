# Archetype DSL Implementation: EX13 slice — Guilmon / reptile
Date: 2026-10-01
Total cards in pool: 3
Processed this run: 3
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 2
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 1
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| EX13-007 | Guilmon | IMPLEMENT | IMPLEMENTED | self-review | 11/11 | WM/OP trash 1 → return [Gallantmon]/red Tamer; inherited +2000 DP-deletion max (G-ENGINE-DP-DELETION-MAX-MODIFIER resolved 2026-10-01) |
| EX13-048 | Kotemon | IMPLEMENT | IMPLEMENTED | self-review | 19/19 | Two-bucket reveal ([Knightmon] text / name) + inherited OPT leave-prevention by deleting another own [Knightmon]-text Digimon |
| EX13-068 | Takato Matsuki | IMPLEMENT | IMPLEMENTED | self-review | 13/13 | SoT memory→3; SoMP optional return-self → free [Takato Matsuki] from hand → if no Digimon, free [Guilmon] from trash; Security play self |

Per-card JSONs `code/digimon-engine/cards/ex13/EX13-{007,048,068}.json` were missing and were written from `data/cards.json`
(EX13-068's [Security] line sits in `inherited_effect_description_eng`, same corpus quirk as EX13-067). No DCGO file exists for EX13-048 / EX13-068 at b9a0638cd; EX13-007 has one.

## Engine-Gap Blocked Cards
### EX13-007 Guilmon
- Effect text: "[All Turns] Add 2000 to this Digimon's DP deletion effects' maximums." (inherited)
- Missing engine API: a continuous modifier consulted by DP-capped *deletion* effects whose source is this permanent (DCGO `ChangeDPDeleteEffectMaxDPClass`); `dp_lte` caps are static and context-blind.
- Suggested addition: `ModifierType::DpDeletionMaxDelta(i32)` + a deletion-context marker on `dp_lte`. Also unblocks BT17-008 / BT17-010 / BT19-007 / BT19-009.

## DSL-Vocab-Gap Blocked Cards
(none)

## New Patterns Discovered
- Optional `select_hand` / `select_trash` prompts expose decline via `is_optional` (PASS accepted) without listing PASS in `valid_action_ids` — tests should assert `view.is_optional`, not `valid_action_ids.contains(&PASS)`.

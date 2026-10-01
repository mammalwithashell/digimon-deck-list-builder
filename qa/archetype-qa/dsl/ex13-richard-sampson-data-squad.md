# Archetype DSL Implementation: EX13 — Richard Sampson / DATA SQUAD
Date: 2026-10-01
Total cards in pool: 5
Processed this run: 5
Pipeline: batch-implement-cards-rust-dsl (author-set EX13 slice)

## Summary
- IMPLEMENTED: 3
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 1
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 1
- SKIPPED (prior verdict): 0

None of the five cards has a DCGO script at the submodule (b9a0638cd). The
official Bandai DB bundles (`data/card_bundles/`) and existing DATA SQUAD /
Glowing Dawn prior art (ST23-06, ST24-01, ST24-09, BT25-027, BT25-087,
BT23-035) were the references. Per-card JSONs were added from
`data/cards.json` (which already folds in `card_overrides.json`). The two
BLOCKED cards ship partial YAML: the expressible clauses are live and tested,
and the blocked clauses are left out (no approximation), with ignored tests
named by gap id.

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| EX13-003 | Kyaromon | IMPLEMENT | IMPLEMENTED | self-reviewed | 10/10 | Inherited [Your Turn][OPT] on_own_security_removed → may digivolve the carrier into a [Kentaurosmon]-name / [Holy Beast] hand card, printed cost −1 |
| EX13-026 | Kudamon | IMPLEMENT | IMPLEMENTED | self-reviewed | 13/13 | [When Moving][On Play] reveal 3 → add 1 + place 1 face down at the bottom of a chosen [DATA SQUAD] Tamer; inherited [WA][OPT] Security A. −1 until end of opponent's turn |
| EX13-030 | Reppamon | IMPLEMENT | IMPLEMENTED | self-reviewed | 16/16 | <Barrier> own + inherited; [OP][WD][WA][OPT] optional trash top security → may play [Richard Sampson] from hand or trash free |
| EX13-032 | Chirinmon | IMPLEMENT | BLOCKED (engine) | self-reviewed | 13/15 (2 ignored) | [WD][WA][OPT] two-cost optional processing condition → unsuspend + opp Digimon can't activate [When Digivolving] is green; both would-leave "top stacked card → top security" saves blocked (G-TOP-STACKED-CARD-TO-SECURITY) |
| EX13-071 | Richard Sampson | IMPLEMENT | BLOCKED (hybrid) | self-reviewed | 10/12 (2 ignored) | [SoYMP][On Play] optional face-down placement + memory, and [Security] play self, are green; [Main][OPT] Kudamon → Kentaurosmon "ignoring level, cost −1" blocked (G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST) |

## Engine-Gap Blocked Cards
### EX13-032 Chirinmon
- Effect text: "[All Turns] When this Digimon would leave the battle area, by placing its top stacked card as the top security card, it doesn't leave." (+ the inherited [Kentaurosmon]-carrier OPT version)
- Missing engine API: moving a permanent's TOP card (DCGO `Permanent.TopCard`) into security while the permanent stays topped by the next card. `security_place_top_stacked_card` extracts `card_sources[len-2]` (the card under the top).
- Suggested addition: `EffectContext::security_place_top_card(carrier, player, position, face_up)` + DSL `security_place_top_card`, usable in a would-leave replacement body before `cancel_replacement`. See `qa/archetype-qa/engine-gaps.md` §G-TOP-STACKED-CARD-TO-SECURITY. That entry also flags a suspected divergence in shipped BT20-084 / BT23-008 / BT23-018, which use the same `len-2` reading.

## Hybrid-Gap Blocked Cards
### EX13-071 Richard Sampson
- Effect text: "[Main] [Once Per Turn] By trashing 3 bottom face-down cards from under any of your Tamers and placing 1 each of level 4 and level 5 [Holy Beast] trait yellow Digimon cards from your trash as 1 of your [Kudamon]'s bottom digivolution cards, it may digivolve into [Kentaurosmon] in the hand or trash, ignoring level and with the cost reduced by 1."
- Missing engine API: effect digivolve that ignores only level (keeps the colour circle) and pays the printed cost (DCGO `IgnoreRequirement.Level`). Today `ignore_requirements` drops colour and zeroes the base cost.
- Missing DSL verb: `effect_initiated_digivolve { ignore_level: true }`.
- See `qa/archetype-qa/engine-gaps.md` + `qa/dsl-vocab-gaps.md` §G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST.

## New Patterns Discovered
- There is no `GameEvent` for a security-card trash, so §5 Section 4's "cost firing" check for a security-trash cost is pinned through a real observer. In EX13-030's test, EX13-003's inherited `on_own_security_removed` hand pick surfaces after Reppamon pays.
- Two alternative "By trashing X or Y" costs: gate the clause on `any_of` payable, offer `select_effect_choice` only when both are payable, and keep the effect steps inside each cost branch. `trash_bottom_face_down_source_under_tamer` skips its tail when unpaid, so the "After" part is gated on payment (EX13-032, Official Q&A).

# Archetype DSL Implementation: EX13 — orphan staples
Date: 2026-10-01
Total cards in pool: 21
Processed this run: 21
Pipeline: batch-implement-cards-rust-dsl (EX13 author-set, slice "orphan staples")

Notes on this run: the orchestrating session had no sub-agent dispatch, so the
scout / implementer / reviewer waves ran inline in one session. Tests were
iterated in a throw-away scratch test binary (deleted) and then registered in
`tests/cards_behavioral/ex13/mod.rs`. Shared fixtures for the slice live in
`tests/cards_behavioral/ex13/orphan_support.rs`. Per-card JSON was split from
`data/cards.json` with `code/tools/split_cards_json.py --card`.

## Summary
- IMPLEMENTED: 19
- PARTIAL: 0
- BLOCKED (engine): 1 (EX13-010)
- BLOCKED (hybrid): 1 (EX13-076)
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| EX13-002 | DemiVeemon | IMPLEMENT | IMPLEMENTED | 10 | inherited OPT optional self-unsuspend on own blue Tamer play; [Veedramon]-name + suspended gates |
| EX13-004 | DemiMeramon | IMPLEMENT | IMPLEMENTED | 7 | inherited WA OPT digivolve into [Witchelny]-text card −1; `effect_digivolved_any_digimon` → trash own top security; decline refunds OPT |
| EX13-005 | Bebydomon | IMPLEMENT | IMPLEMENTED | 7 | inherited WA OPT play-or-use [Dracomon]/[Examon]-text card −1; decline refunds OPT |
| EX13-009 | Huckmon | IMPLEMENT | IMPLEMENTED | 11 | reveal 3 two buckets (Digimon / Tamer-or-Option w/ text); inherited OPT +1 memory on own white Digimon play |
| EX13-046 | Kokuwamon | IMPLEMENT | IMPLEMENTED | 8 | reveal 3 ([Mamemon] text / [Mutant] trait); inherited On Deletion De-Digivolve 1 |
| EX13-047 | Gotsumon | IMPLEMENT | IMPLEMENTED | 9 | Blocker; reveal ([Royal Knight] / printed <Blocker>, inherited <Blocker> excluded per Q&A); WA lose 2 memory; inherited opp-turn +2000 |
| EX13-010 | Growlmon | — | BLOCKED (engine) | 0 | inherited "add 2000 to this Digimon's DP deletion effects' maximums" — G-ENGINE-DP-DELETION-MAX-MODIFIER |
| EX13-011 | BaoHuckmon | IMPLEMENT | IMPLEMENTED | 12 | Raid; OP/WD ≤1 Tamer → optional free [Mon]; text-gated special digivolve; inherited your-turn +2000 |
| EX13-051 | Guardromon | IMPLEMENT | IMPLEMENTED | 16 | Blocker; would-leave replacement for other <Blocker> allies (suspend this), batch-saves all leaving per Q&A; inherited opp-turn OPT unsuspend |
| EX13-052 | Gladimon | IMPLEMENT | IMPLEMENTED | 11 | <Guard>; OP/OD De-Digivolve 1 (Guard self-deletion fires OD); inherited OPT replacement by deleting other [Knightmon]-text Digimon |
| EX13-012 | SaviorHuckmon | IMPLEMENT | IMPLEMENTED | 9 | Alliance (face-up + inherited); WD/WA shared OPT play-or-use white [Huckmon]-text −3, RemoveUse refund |
| EX13-013 | WarGrowlmon | IMPLEMENT | IMPLEMENTED | 14 | Engage; WD/WA mandatory ≤5000 delete else Piercing +3000; EoA/OD free [Guilmon]-text Tamer from hand/trash; inherited OPT [Gallantmon] security trash |
| EX13-034 | Wisemon | IMPLEMENT | IMPLEMENTED | 14 | Barrier; OP/WD/WA OPT Reboot+Blocker+CannotBeDeDigivolved to 1 ally; security-removed OPT De-Digivolve then ≤3 sec CannotDigivolve; inherited OPT unsuspend |
| EX13-042 | Bastemon | IMPLEMENT | IMPLEMENTED | 6 | Alliance (both); WD/WA OPT free ≤4 cost Beast/Animal/Sovereign (not Sea Animal) Digimon |
| EX13-056 | Giromon | IMPLEMENT | IMPLEMENTED | 10 | Collision, Blocker, Rule [Machine]; self-suspend OPT reveal-3 play Lv.4- black printed-Blocker, trash rest; inherited opp-turn OPT Lv.5- play from hand |
| EX13-058 | Knightmon | IMPLEMENT | IMPLEMENTED | 10 | WA/OD free play-or-use [Knightmon]-text ≤4; opp-turn Reboot+Blocker aura for [Knightmon]-text Digimon; inherited OPT De-Digivolve on [Knightmon]-text play |
| EX13-024 | Slayerdramon | IMPLEMENT | IMPLEMENTED | 13 | Raid, Blocker, Assembly -5, named special digivolve; OP/WD trash N opp sources (N = own sources, clamped) then optional fewest-source deck-bottom; face-up + inherited OPT replacements |
| EX13-076 | Imperialdramon: Paladin Mode | — | BLOCKED (hybrid) | 0 | battle comparing digivolution-card counts (G-ENGINE-BATTLE-COMPARE-SOURCE-COUNT) + Assembly w/different names (G-ASSEMBLY-NO-DISTINCT-BY) |
| EX13-070 | Davis Motomiya & Ken Ichijoji | IMPLEMENT | IMPLEMENTED | 11 | memory floor; EoT suspend-self → choose-one: digivolve (−1 per opp Digimon) / DNA into [Free] (printed DNA cost); security play |
| EX13-073 | Tai Kamiya & Matt Ishida | IMPLEMENT | IMPLEMENTED | 13 | SoMP +1 with ADVENTURE Digimon; ally-played suspend-self draw 1 + trash 1 (own play triggers per Q&A); Lv.5+ ADVENTURE Rush/Blocker; security play |
| EX13-075 | Mon | IMPLEMENT | IMPLEMENTED | 6 | memory floor; reveal 3 add [Huckmon]-text; security play |

## Engine-Gap Blocked Cards
### EX13-010 Growlmon
- Effect text: "[All Turns] Add 2000 to this Digimon's DP deletion effects' maximums." (inherited)
- Missing engine API: a continuous DP-deletion-cap modifier consulted by `dp_lte` caps of deletion effects whose source is in the carrier's stack.
- Tracker: `qa/archetype-qa/engine-gaps.md` §G-ENGINE-DP-DELETION-MAX-MODIFIER (consumer line added).

### EX13-076 Imperialdramon: Paladin Mode (hybrid)
- Effect text: "... have this Digimon battle it. Compare the number of digivolution cards instead of DP in this battle." + "Assembly -8: 6 [Free]/[Royal Knight] trait Digimon cards w/different names"
- Missing engine API: battle-scoped comparator (`BattleCompare::SourceCount`); Assembly `distinct_by: name`.
- Trackers: `qa/archetype-qa/engine-gaps.md` §G-ENGINE-BATTLE-COMPARE-SOURCE-COUNT, `qa/dsl-vocab-gaps.md` (same id), §G-ASSEMBLY-NO-DISTINCT-BY.

## Interpretation notes (printed text vs DCGO)
- EX13-051 "your other Digimon with <Blocker>": modelled as the leaving Digimon's live keyword (`has_keyword: Blocker`, includes granted Blocker such as EX13-058's opponent-turn aura). DCGO reads `TopCard.HasBlocker` (printed only). The "card with <Blocker>" Q&A (EX13-047/056) is about cards, not Digimon on the field.
- EX13-056: DCGO's candidate filters omit the printed "black"; the colour gate is kept (printed text governs).
- EX13-024: DCGO limits the fewest-sources deck-bottom to opponent Digimon with ≥1 source; printed "all of their Digimon with the fewest digivolution cards" governs (0-source Digimon count).
- "Declining a 'you may play/use' pick refunds the [Once Per Turn]" follows DCGO's `RemoveUse()` (EX13-012 / EX13-056) and the shipped EX13-043 convention; applied to 004, 005, 012, 042, 056.

## New Patterns Discovered
- Engine-timed tests that cross a turn must keep the turn alive: `r.game.memory` is turn-player-relative, and a negative value ends the opponent's turn the moment the queue drains (which made early "accept" assertions vacuous). `orphan_support::end_turn_declining` / `cycle_to_my_turn_with_memory` encode the safe pattern, including passing an <Engage> `EndOfTurnAction` window.
- Memory-cost assertions on effects that resolve at [End of Your Turn] should read the `MemoryChange` event, not `memory()` — the turn passes right after.

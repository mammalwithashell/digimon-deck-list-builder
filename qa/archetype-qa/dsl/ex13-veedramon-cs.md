# Archetype DSL Implementation: EX13 — Veedramon / CS
Date: 2026-10-01
Total cards in pool: 6
Processed this run: 6
Pipeline: batch-implement-cards-rust-dsl (author-set EX13 slice)

## Summary
- IMPLEMENTED: 5
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
| EX13-017 | Veemon | IMPLEMENT | IMPLEMENTED | self-reviewed (+ mutation check) | 16/16 | [OP]/[When Moving] reveal 3 → [Veedramon]-text/[Royal Knight] to hand; Lv.2 [CS] cost-0 alt; inherited OPT would-leave (opp effect) → suspend, doesn't leave |
| EX13-019 | Veedramon | IMPLEMENT | IMPLEMENTED | self-reviewed | 10/10 | <Jamming> face + inherited; [WA][OPT] [Veedramon]-text Tamer from hand −2, refund_opt on decline |
| EX13-022 | AeroVeedramon | IMPLEMENT | IMPLEMENTED | self-reviewed (+ mutation check) | 19/19 | [OP][WD][WA] shared OPT free Tamer play; own Tamer played → opp Digimon/Tamer can't suspend; inherited OPT self-suspend → may unsuspend |
| EX13-067 | Nokia Shiramine | IMPLEMENT | IMPLEMENTED | self-reviewed (+ mutation check) | 15/15 | No DCGO script. Start-of-main +1 memory; on_digivolve ≤1 Digimon, suspend → [Gabumon]/[Agumon] from hand/trash free; Q&A two-copy re-check pinned; [Security] play |
| EX13-069 | Rina Shinomiya | IMPLEMENT | IMPLEMENTED | self-reviewed (+ mutation check) | 13/13 | Start-of-main +1 memory; own Digimon unsuspend → suspend, Draw 1, may digivolve into [Veedramon]-name −2; [Security] play |
| EX13-074 | Rie Kishibe | IMPLEMENT | BLOCKED (engine) | — | 0 | Tamer digivolving into [LordKnightmon] (fixed cost 3, ignore reqs, hand/trash) — no Tamer-base digivolve primitive |

Mutation check: dropping the `on_move` self gate (017), the inherited name gate
(022), the `≤1 Digimon` gate (067) and the own-Digimon event gate (069) each
turned the matching negative test red, so those tests are not vacuous.

## Engine-Gap Blocked Cards
### EX13-074 Rie Kishibe
- Effect text: "[Main] [Once Per Turn] If this Tamer has 3 or more [Knightmon] text cards under it, it may digivolve into [LordKnightmon] in the hand or trash for a digivolution cost of 3, ignoring digivolution requirements."
- Missing engine API: a digivolve onto a TAMER permanent. `effect_initiated_digivolve_from_source_inner` rejects targets with no level, and the route machinery assumes a Digimon base.
- Suggested addition: `Game::effect_initiated_digivolve_tamer(player, source_ref, tamer, cost)` (Tamer + under-cards become digivolution cards; fixed cost; digivolution draw + [When Digivolving]). Then `effect_initiated_digivolve { target: source, source: <union>, cost: { fixed: 3 }, ignore_requirements: true }` lowers to it.
- Also needed (clause 2): a "place a hand/trash card under this Tamer" cost step over a `select_union_zone` binding.
- Tracked: `qa/archetype-qa/engine-gaps.md` §G-TAMER-DIGIVOLVE-INTO-DIGIMON.

## DSL-Vocab-Gap Blocked Cards
(none)

## New Patterns Discovered
- Inherited "would leave by opponent's effects → by suspending it, it doesn't leave": `scope: inherited` + `kind: replacement` with NO `replacement_subject_is_mine` (the engine restricts the subject to the carrier), `source_name_contains` + `source_is_unsuspended` in `active_when`, then `suspend {source}` + `cancel_replacement`. Neither source-relative predicate counts as a subject read, so the carrier-only restriction holds.
- A `condition` on an observer clause is checked again at resolution. EX13-067's official Q&A (the second copy fails "1 or fewer Digimon" after the first copy resolves) passes with no extra `if` guard.

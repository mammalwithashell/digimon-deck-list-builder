# Archetype DSL Implementation: EX13 slice — Puppet (Sistermon Awakened DUALs)
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
| EX13-065 | Sistermon Blanc (Awakened) | IMPLEMENT | IMPLEMENTED | self-reviewed | 18/18 | DUAL. Guard → own Decode nesting (G-NESTED-PARKED-REPLACEMENT RESOLVED 2026-10-01); every other clause green (name/Lv.2-text digivolve, Decode, Guard, Option free play + per-ally -3000, Arts) |
| EX13-066 | Sistermon Noir (Awakened) | IMPLEMENT | IMPLEMENTED | self-reviewed | 18/18 | DUAL. name_in / Lv.3-text digivolve, also_treated_as, Decode, [WD] delete cost<=4, Option free play + De-Digivolve N per ally, Arts |
| EX13-035 | KingEtemon | IMPLEMENT | IMPLEMENTED | self-reviewed | 18/18 | [OP][WD] optional return-10 (+6) then up to 2 free plays under a running budget; [All Turns] both-fields count → opp -3000 DP + Security A. -1 |

Per-card JSONs `code/digimon-engine/cards/ex13/EX13-0{35,65,66}.json` were
missing and were materialized from `data/cards.json` (which already carries the
`card_overrides.json` DUAL blocks / evo circles / Data attribute; the official
Bandai DB bundles agree). DCGO has no script for any of the three at
`b9a0638cd`; printed text, the official Q&A, and `general_rule.pdf` govern.

Review note: this run had no sub-agent dispatch tool, so the scout,
implementer and reviewer waves ran in a single agent (no separate Opus review).

## Engine-Gap Blocked Cards
### EX13-065 Sistermon Blanc (Awakened)
- Effect text: "<Guard> (When any of your other Digimon would leave the battle area by your opponent's effects, by deleting this Digimon, they don't leave.)" together with "<Decode ([Sistermon Blanc])>".
- Missing engine API: nested replacement parking. Guard deletes its carrier inside its own replacement process; the carrier's Decode window parks a second replacement → `replacement.rs` nested-park debug_assert (G-NESTED-PARKED-REPLACEMENT, qa/archetype-qa/engine-gaps.md).
- Suggested addition: make `Game::parked_replacement` a stack and defer Guard's cancel until the carrier deletion commits.

## DSL-Vocab-Gap Blocked Cards
(none)

## Engine substrate widened this run
- G-ENGINE-ARTS-ALT-PATH-BASE — `<Arts Digivolve>` base check now uses the full digivolve route set (alt paths), not only standard circles.
- G-FORMULA-BINDING-PLAY-COST-UNION — `binding_play_cost` reads `select_union_zone` picks.

## Verification
- `cards_behavioral` ex13_035 / ex13_065 / ex13_066: 51 passed, 1 ignored (gap-pinned).
- Full `cards_behavioral` run set by set (the single-process run got OOM-killed at about 4800 tests on this 15 GB box): everything green except `ex13_044` (8 failures). Those failures come from another slice's uncommitted EX13-044.yaml edit and happen with or without this slice's engine changes.
- `st24_07`: two tests updated because Arts is now offered onto the alt-path base, and one new test pins that.
- `--lib` (338), `dsl` (926), `dual_cards`, `rules_faq`, `keyword_semantics_matrix`, `effect_source_kind`: all green.

## New Patterns Discovered
- Running play-cost budget across sequential union picks: `play_cost_lte: { subtract: [B, { binding_play_cost: <pick 1> }] }` — propose adding to RUST_DSL_TEST_API.md.
- "While there are N or more Digimon" (no "your") → `count_gte.filter.owner: any`; the default owner is `you`.

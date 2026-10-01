# Archetype DSL Implementation: EX13 slice — Sukamon / beast
Date: 2026-10-01
Total cards in pool: 4
Processed this run: 4
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 4
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
| EX13-027 | Chuumon | IMPLEMENT | IMPLEMENTED | APPROVED | 16/16 | [When Moving][On Play] `reveal_search` hand bucket then trash bucket (lone match -> hand, Official Q&A); inherited [All Turns][OPT] self-scoped leave-prevention by deleting another [Sukamon]-named Digimon |
| EX13-028 | Sukamon | IMPLEMENT | IMPLEMENTED | APPROVED | 13/13 | <Blocker>; [On Deletion] reveal 3 / may play cost<=3 [Chuumon]/[Sukamon] Digimon free / trash rest; inherited = EX13-027 |
| EX13-038 | Salamon | IMPLEMENT | IMPLEMENTED | APPROVED | 12/12 | [On Play] two hand buckets: `in_text_contains: Leopardmon` + Digimon with `trait_contains` Beast/Sovereign/(Animal && !Sea Animal); (Rule) Beast; inherited +1000 DP to own suspended Digimon |
| EX13-040 | Mikemon | IMPLEMENT | IMPLEMENTED | APPROVED | 13/13 | [On Play][When Digivolving] CannotUnsuspend (until end of their turn); [All Turns] self `on_suspend` -> suspend 1 opp Digimon/Tamer; inherited suspended-Digimon aura |

Per-card JSONs `code/digimon-engine/cards/ex13/EX13-0{27,28,38,40}.json` were
missing and were materialized from `data/cards.json` (which already carries the
`card_overrides.json` evo-cost / Rule-trait fixes; official Bandai DB bundles
agree). DCGO has no script for any of the four at `b9a0638cd`.

## Engine-Gap Blocked Cards
None.

## DSL-Vocab-Gap Blocked Cards
None.

## New Patterns Discovered
- Self-scoped inherited leave-prevention: keep `active_when` free of
  subject-reading predicates (`kind`, `replacement_subject_is_mine`, ...).
  `lower_replacement.rs::predicate_reads_replacement_subject` treats ANY such
  leaf as "may match a cross-permanent subject", which drops the
  subject == source guard. Only `replacement_cause` is used here, so the
  replacement protects only the carrier ("this Digimon").
- SUSPECTED DRIFT (not fixed, outside this slice): EX11-022 Karakurumon's
  inherited "When THIS Digimon would leave" replacement lists
  `replacement_subject_is_mine` + `kind: digimon` in `active_when`, which by
  the rule above widens it to protect any of the owner's Digimon. Route to
  audit.
- Test note: flipping `is_suspended` directly does not re-tick declarative
  auras; use `game.suspend(h)` + `game.tick_declarative_effects()`.

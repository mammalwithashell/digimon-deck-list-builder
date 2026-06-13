# Archetype DSL Implementation: mammal (TS / Iliad)
Date: 2026-06-06
Total cards in slice this run: 4
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
| BT25-022 | Lunamon | IMPLEMENT | IMPLEMENTED | self | 6/6 | [On Play] reveal-3 Iliad+TS two-bucket; inherited Jamming; alt-digi Lv.2 Blue/TS |
| BT25-030 | Elecmon | IMPLEMENT | IMPLEMENTED | self | 9/9 | [Start of Main] add top sec → gain 1 memory (optional, gated sec≥1); inherited [When Attacking][OPT] may add sec then Recovery+1 if 0; alt-digi Lv.2 Yellow/TS |
| BT25-031 | Patamon | IMPLEMENT | IMPLEMENTED | self | 6/6 | [On Play] reveal-3 Angel-group+TS two-bucket; inherited Barrier; alt-digi Lv.2 Yellow/TS |
| BT25-078 | Gazimon | IMPLEMENT | IMPLEMENTED | self | 7/7 | [When Moving][On Play] optional reveal-3 branch: add [Three Musketeers]-in-text to hand OR place [Three Musketeers]-trait as bottom source; inherited Retaliation; alt-digi Lv.2 Black/TS/TM-text |

## Engine-Gap Blocked Cards
None.

## DSL-Vocab-Gap Blocked Cards
None.

## New Patterns Discovered
None — all four cards expressed with existing DSL vocabulary:
- `select_reveal_buckets` + `no_duplicate_cards` for "Add 1 [X] and 1 [Y]" two-pick reveals
  (canonical BT24-031 / EX8-047 idiom).
- `choose_from_reveal` with `destination: hand` and
  `destination: { bottom_source_of: { target: this } }` plus a `select_effect_choice`
  branch for "add to hand OR place as bottom digivolution card" (P-167 idiom). BT25-078's
  two branches use different eligibility filters (`effect_text_contains` vs `trait_has`),
  faithfully matching DCGO's "TM-in-text → hand; TM-trait → may place as source".
- `alt_paths { kind: digivolve, from: { trait_has: TS } }` for the [Digivolve] Lv.2 w/[TS]
  trait Cost 0 alternate recipe (BT24-031 idiom); BT25-078 also adds an
  `effect_text_contains: "Three Musketeers"` source branch.
- `add_top_security_to_hand` + `gain_memory` for the cost-for-memory start-of-main effect;
  `may_add_top_security_to_hand` + conditional `recover` for the inherited When-Attacking
  Recovery clause.

## Test infrastructure note
The shared worktree was under heavy concurrent BT25 authoring during this run, which
repeatedly (a) deleted `cards/bt25/*.yaml` out from under the writer and (b) left sibling
test files / YAMLs that did not compile, blocking the shared `cards_behavioral` binary.
The 4 cards' YAMLs were validated for parse+compile via a standalone `digimon-dsl`
`CardRegistry::from_specs` checker, and the 28 behavioral tests were executed green via an
isolated test crate depending on `digimon-engine` (path). The in-tree files
(`cards/bt25/BT25-0{22,30,31,78}.yaml`, `tests/cards_behavioral/bt25/bt25_0{22,30,31,78}.rs`,
registered in `bt25/mod.rs`) are the deliverables; they will run under the normal
`cargo test --test cards_behavioral` once the concurrent sibling churn settles.

# Archetype DSL Implementation: orphan-c (BT25 slice)
Date: 2026-06-06
Total cards in slice: 6
Processed this run: 6
Pipeline: batch-implement-cards-rust-dsl

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
| BT25-067 | Sealsdramon | IMPLEMENT | IMPLEMENTED | self | 10/10 | Alt-digi D-Brigade/ACCEL; [Your Turn] own D-Brigade/ACCEL play → may self-digivolve into hand -2; inherited [All Turns] +1000 DP |
| BT25-070 | Logamon | IMPLEMENT | BLOCKED (engine) | self | 0 | Entire kit on the [Link] subsystem (link Digimon card from trash/sources to self; WhenLinked/[When Linking] triggers). No faithful partial. |
| BT25-015 | Garudamon | IMPLEMENT | IMPLEMENTED | self | 7/7 | <Raid>+<Fortitude>; [OP][WD] delete opp DP<=6000; inherited [All Turns][OPT] battle-delete → trash opp top security |
| BT25-026 | Crescemon | IMPLEMENT | IMPLEMENTED | self | 9/9 | [OP][WD] trash bottom 3 sources + CannotSuspend lock; [Your Turn] red-played → Dianamon-in-trash -2; inherited [Your Turn] CanNotSwitchAttackTarget |
| BT25-038 | Shakkoumon | IMPLEMENT | IMPLEMENTED | self | 7/7 | TS alt-digi + DNA digivolve; [OP][WD] place traited Digimon from hand OR sources top/bottom sec; on_dna_digivolve trash both top sec; [All Turns][OPT] sec-added De-Digivolve 1; inherited [All Turns][OPT] sec-removed -4000 DP |
| BT25-040 | MagnaAngemon | IMPLEMENT | IMPLEMENTED | self | 8/8 | Security-trash self → play Lv<=4 Angel/Iliad free; <Ascension>; [OP][WD] trash top/bottom sec → opp -8000 DP; inherited [All Turns][OPT] sec-removed -4000 DP |

All 41 behavioral tests for the 5 implemented cards pass (isolated `cargo test --test cards_behavioral` subset run, 2026-06-06).

## Source-priority corrections applied
- **BT25-026 Crescemon inherited effect**: `data/cards.json` says `<Security A. +1>`. The printed card FACE (image BT25-026.webp) and DCGO `BT25_026.cs` both say "[Your Turn] This Digimon's attack target can't change" (`CanNotSwitchAttackTargetClass`). Implemented the image/DCGO reading (`CanNotSwitchAttackTarget` self-aura) per CLAUDE.md source priority.
- **BT25-024-style trash-source digivolve**: BT25-026's [Your Turn] digivolves into [Dianamon] in the TRASH (DCGO `isHand:false`), matching the BT25-024 precedent.

## DSL substrate widened (rule 28)
- **`Ascension` keyword** added to the DSL validator keyword allowlist (`code/digimon-dsl/src/validator.rs`). The engine `Keyword::Ascension` enum variant + `grant_keyword` lowering already existed and the build accepts it; only the lint-time allowlist was missing it (also unblocks the previously-shipped BT25-034 Angemon, which carries `<Ascension>`).

## Engine-Gap Blocked Cards
### BT25-070 Logamon (Lv.4 Black/Purple, Logoff Sup.)
- Effect text:
  - "[Main] [Once Per Turn] You may link 1 [Social], [Tool] or [Game] trait Digimon card from your trash or this Digimon's digivolution cards to this Digimon with the cost reduced by 1."
  - "[Your Turn] [Once Per Turn] When this Digimon gets linked, delete 1 of your opponent's Digimon with a play cost of 4 or less."
  - Inherited: "1 of your opponent's Digimon or Tamers can't unsuspend until their turn ends." (DCGO: `[When Linking]` consumer)
  - Plus `AddAppfuseMethodByName(Offmon, Hackmon)` App-Fusion (separate App Fuse gap).
- Missing engine primitives (all in the standing `[Link]` subsystem gap):
  - A `WhenLinked` / `[When Linking]` **triggered** timing — `EffectTiming` has only `WhenWouldLink` (a replacement); the DSL→engine timing map (`timing_map.rs`) has no link-trigger timing.
  - An "actively link a Digimon card from trash or this Digimon's digivolution cards to this Digimon" step (the DSL `link_to_own_digimon` links the *current Option* to a host; there is no link-a-Digimon-card-from-non-hand-zone step).
- Logged to `docs/RUST_ENGINE_GAPS.md` → "`[Link]` keyword subsystem" entry (facets #6/#9/#11). Only Logamon's standard `Logoff`-trait alt-digivolve *requirement* is expressible — which is not an effect — so the card is BLOCKED in full (no faithful partial; stubbing the link clauses would violate the no-approximations policy).

## Notes on test execution
The shared `cards_behavioral` Cargo binary was contended by several concurrent
authoring sessions during this run (sibling slices had non-compiling WIP test
files, and at least one session quarantined in-progress files to keep the
aggregate binary green). The orphan-c cards were validated by:
1. `cargo build --lib` (build.rs scans all `cards/`) — all 5 YAMLs parse,
   compile, and lower (exit 0).
2. An isolated `cards_behavioral` run scoped to only the 5 orphan-c modules —
   41 tests, all passing.
The 5 YAMLs + 5 test files + the validator change are git-staged so they survive
concurrent `git clean`/quarantine churn on the shared tree.

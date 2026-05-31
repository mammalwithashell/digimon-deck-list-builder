## 1. Discovery and Baseline

- [x] 1.1 Confirm the current minimal YAML shape for a no-effect/vanilla card by inspecting existing cards and compiling one ST-4 vanilla candidate locally
- [x] 1.2 Resolve the exact ST-4 Giga Green deck recipe counts from the canonical source and identify the repo's preferred starter-deck fixture location
- [x] 1.3 Inspect `fix-outer-optional-prompt-trigger-ctx` or its merged state before authoring ST4-14 optional tamer-cost behavior
- [x] 1.4 Search current DSL predicates and trigger context for any existing battle-opponent/survival helper before adding new vocabulary

## 2. ST4-11 DSL Vocabulary Slice

- [x] 2.1 Write failing DSL/engine tests for the battle-deletion-survivor predicate/helper: qualifying carrier battle, unrelated friendly battle deletion, mutual destruction, and non-battle deletion
- [x] 2.2 Add the predicate/helper to the DSL parse/compile/lowering path if no existing surface covers the behavior
- [x] 2.3 Wire predicate evaluation to engine battle context so inherited-source carrier identity and survival are checked exactly
- [x] 2.4 Run the targeted DSL/engine tests and document the reusable gap closure in `qa/resolved-gaps.md` or the active gap tracker

## 3. Card YAML Authoring

- [x] 3.1 Add YAML specs for vanilla/no-effect ST-4 cards (`ST4-02`, `ST4-05`, `ST4-07`, `ST4-09`) with no extra gameplay behavior
- [x] 3.2 Add YAML specs for simple inherited/keyword cards (`ST4-01`, `ST4-04`, `ST4-06`, `ST4-08`)
- [x] 3.3 Add YAML specs for reveal/search cards (`ST4-03`, `ST4-10`) with correct eligible-card filters and bottom-deck remainder handling
- [x] 3.4 Add YAML specs for suppression, Digi-Burst, and battle-deletion cards (`ST4-11`, `ST4-12`, `ST4-13`)
- [x] 3.5 Add YAML specs for tamer and option cards (`ST4-14`, `ST4-15`, `ST4-16`) including security effects
- [x] 3.6 Compile the Rust engine card pack and confirm all `ST4-01` through `ST4-16` card IDs load as implemented

## 4. Behavioral Tests

- [x] 4.1 Add behavioral tests for ST4-01 inherited level-gated DP and ST4-04/ST4-06 inherited attack-target DP
- [x] 4.2 Add behavioral tests for ST4-03/ST4-10 reveal-search positive and no-hit cases
- [x] 4.3 Add behavioral tests for ST4-08 Blocker and attack memory loss
- [x] 4.4 Add behavioral tests for ST4-11 qualifying battle, non-survival, unrelated battle deletion, and once-per-turn suppression
- [x] 4.5 Add behavioral tests for ST4-12 attack/block suppression and expiry
- [x] 4.6 Add behavioral tests for ST4-13 Piercing and Digi-Burst suspend behavior
- [x] 4.7 Add behavioral tests for ST4-14 optional tamer suspend-as-cost memory gain and security play
- [x] 4.8 Add behavioral tests for ST4-15/ST4-16 main and security option behavior, including ST4-15 add-to-hand and ST4-16 no add-to-hand

## 5. Starter Deck Recipe

- [x] 5.1 Add the canonical ST-4 Giga Green deck recipe using the verified 4-card Digitama and 50-card main-deck composition
- [x] 5.2 Add or update a deck-library/fixture validation test confirming the recipe size and card counts
- [x] 5.3 Add a Rust-backed headless reset smoke test using ST-4 Giga Green as a player deck

## 6. Tracker Reconciliation and Verification

- [x] 6.1 Update ST-4 entries in the current tested-card or validated-card ledger only after the related behavioral tests pass
- [x] 6.2 Move any newly closed ST4-11 DSL gap note from `qa/dsl-vocab-gaps.md` or `qa/archetype-qa/engine-gaps.md` to `qa/resolved-gaps.md`
- [x] 6.3 Run targeted verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl`
- [x] 6.4 Run targeted verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st4`
- [x] 6.5 Run the broader Rust engine regression suite required by the changed modules before marking the change complete

## 1. Resolver Baseline

- [x] 1.1 Re-run `python code/tools/resolve_deck.py "Mastemon (Tribal)" --json` with UTF-8 output and confirm the resolved pool and best deck match `qa/archetype-qa/mastemon-tribal/deck_pool.json`.
- [x] 1.2 Produce a Rust coverage snapshot for the resolved pool: YAML count, behavioral-test count, best-deck YAML/test count, and high-frequency missing card list.
- [x] 1.3 Record the starting values for `ACTION_SPACE_SIZE`, `standard_lite_v2`, and `standard_compact_v1` so card unlock work can prove it did not change active RL contracts.

## 2. Owner-Routed Security Placement Substrate

- [x] 2.1 Add failing DSL/engine tests for placing a selected own permanent into its owner's security at top and bottom positions.
- [x] 2.2 Add failing DSL/engine tests for placing a selected opponent permanent into the opponent's security, proving static `of: you` routing is not used.
- [x] 2.3 Implement the owner-routed permanent-to-security helper and DSL vocabulary.
- [x] 2.4 Verify replacement handling, security-add observers, and action-mask target exposure for owner-routed placement.

## 3. Security Cost And Result Gates

- [x] 3.1 Add failing tests for a top-security trash cost that gates a triggered effect body.
- [x] 3.2 Add failing tests for a permanent-to-security placement cost that gates a follow-up tail only on successful placement.
- [x] 3.3 Implement the DSL/engine support for security-stack costs and success-gated placement tails.
- [x] 3.4 Verify declined, unpayable, prevented, and successful cost paths do not hide player choices.

## 4. Selected Security Play And Digivolve Confirmation

- [x] 4.1 Add a focused `BT14-033` test proving selected-security effect digivolve can use a chosen security card, preserve remaining security order, and fire effect-initiated digivolve provenance.
- [x] 4.2 Add or extend selected-security play tests to prove play-success bindings and card-local tails work for Mastemon support shapes.
- [x] 4.3 Implement only the missing security-card substrate discovered by the tests, if current `select_security` plus `effect_initiated_digivolve` and `play_security_card` are insufficient.
- [x] 4.4 Verify searched security stacks shuffle after both selected and declined optional security search effects.

## 5. Mastemon Boss Line

- [x] 5.1 Complete `EX6-029` YAML and tests for optional hand/trash level 5 or lower Angel, Archangel, or Fallen Angel play plus DNA-origin bottom-security/trash-to-4 branch.
- [x] 5.2 Add `P-187` YAML and tests for DNA placement, top/bottom security choice, successful-placement security trash, and security-trash-cost hand/trash play branch.
- [x] 5.3 Add `BT23-102` YAML and tests for alt paths, Barrier, Partition, level-matching security trash to 3, and all-turns security-loss bottom-security trigger.
- [x] 5.4 Run the relevant boss-line behavioral tests and resolve any card-local gaps without raw-Rust escapes.

## 6. High-Frequency Support Core

- [x] 6.1 Implement and test `BT14-033`, `ST10-04`, `EX6-020`, `BT23-031`, and `BT23-067`.
- [x] 6.2 Implement and test `BT11-042`, `BT11-083`, `BT11-094`, and `EX6-074`.
- [x] 6.3 Implement and test `BT15-037`, `EX6-022`, `BT9-082`, and `BT7-107`.
- [x] 6.4 Confirm all support-card searches, cost reductions, inherited auras, end-of-turn DNA effects, and security movement choices are surfaced through pending selections and masks.

## 7. Best-Deck Readiness And Gap Hygiene

- [x] 7.1 Verify every unique card in the resolved best deck has production YAML and a behavioral test.
- [x] 7.2 Remove or replace any no-op stubs, stale raw-Rust examples, or comments claiming unimplemented printed text for best-deck cards.
- [x] 7.3 Update `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, and Mastemon QA notes for every substrate gap closed or newly discovered.
- [x] 7.4 Document any remaining unimplemented low-frequency cards from the full 93-card pool as follow-up coverage, not best-deck blockers.

## 8. Verification

- [x] 8.1 Run focused DSL tests for owner-routed security placement, security costs, selected-security digivolve/play, and trash-until-threshold formulas.
- [x] 8.2 Run focused card behavioral tests for all implemented Mastemon best-deck cards.
- [x] 8.3 Run broader regression suites that cover `cards_behavioral`, `dsl`, `dna_digivolve`, and `digivolve` areas touched by this change.
- [x] 8.4 Re-check `ACTION_SPACE_SIZE` and active tensor layout metadata against the baseline from task 1.3.

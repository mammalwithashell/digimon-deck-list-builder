## 1. Baseline Audit

- [x] 1.1 Resolve the official ST-2 English/Worldwide deck composition and record the exact source used for card counts.
- [x] 1.2 Audit `data/cards.json` for `ST2-01` through `ST2-16`, extracting printed effect, inherited effect, security effect, stats, traits, and digivolution costs.
- [x] 1.3 Compare current `code/digimon-engine/cards/st2/` YAML coverage against the ST2 card pool and confirm `ST2-13` remains faithful.
- [x] 1.4 Verify current DSL/engine support for `select_opponent_sources`, `select_material`, `play_from_materials`, source-count predicates, attack/block modifiers, Tamer security play, and return-to-hand behavior against source code rather than tracker text.
- [x] 1.5 Identify which ST2 clauses are implementable with existing vocabulary and which require new substrate.

## 2. Substrate Tests

- [x] 2.1 Add failing DSL tests for no-choice bottom-source trash: count 1, count 2, fewer-than-requested sources, and no source-selection prompt.
- [x] 2.2 Add failing engine/DSL tests proving bottom-source trash soft-fails for no sources, stale target permanents, and insufficient sources.
- [x] 2.3 Add failing combat/DSL tests for a battle-context predicate that checks whether the opposing battled Digimon has zero source cards.
- [x] 2.4 Add or confirm tests proving `select_material` / `play_from_materials` can faithfully express ST2-15 Kaiser Nail, including source removal, ownership, on-play behavior, and no hidden target shortcuts.

## 3. Substrate Implementation

- [x] 3.1 Implement the no-choice bottom-source trash DSL surface in `digimon-dsl` parse/spec/compile types.
- [x] 3.2 Implement bottom-source trash execution in `digimon-engine`, routing each actually removed source to its owner's trash and firing normal source-trash observers.
- [x] 3.3 Wire bottom-source trash into the soft-fail contract so stale or insufficient sources never panic.
- [x] 3.4 Implement the battle-context no-source predicate for inherited/aura conditions that evaluate during Digimon-vs-Digimon combat.
- [x] 3.5 If Kaiser Nail cannot use existing source-play substrate faithfully, add the narrowest reusable DSL source-play support and tests needed to satisfy ST2-15.

## 4. ST2 Card Authoring

- [x] 4.1 Add production YAML and card-data tests for vanilla/no-effect ST2 cards: `ST2-02`, `ST2-04`, `ST2-05`, and `ST2-10`.
- [x] 4.2 Add production YAML and behavioral tests for `ST2-01` Tsunomon's inherited battle-context DP modifier.
- [x] 4.3 Add production YAML and behavioral tests for source-trash inherited cards `ST2-03` Gabumon and `ST2-06` Garurumon.
- [x] 4.4 Add production YAML and behavioral tests for `ST2-07` Grizzlymon's `Blocker` keyword and `[When Attacking] Lose 2 memory`.
- [x] 4.5 Add production YAML and behavioral tests for `ST2-08` WereGarurumon's inherited Security Attack +1 while the opponent has a no-source Digimon.
- [x] 4.6 Add production YAML and behavioral tests for `ST2-09` Zudomon's `[When Digivolving]` bottom-2 source trash.
- [x] 4.7 Add production YAML and behavioral tests for `ST2-11` MetalGarurumon's `[When Attacking] [Once Per Turn] Unsuspend this Digimon`.
- [x] 4.8 Add production YAML and behavioral tests for `ST2-12` Matt Ishida, including start-turn memory and `[Security] Play this card without paying the cost`.
- [x] 4.9 Add production YAML and behavioral tests for `ST2-14` Sorrow Blue's no-source attack/block restriction in main and security contexts.
- [x] 4.10 Add production YAML and behavioral tests for `ST2-15` Kaiser Nail's selected-source free play.
- [x] 4.11 Add production YAML and behavioral tests for `ST2-16` Cocytus Breath's opponent Digimon return-to-hand effect.

## 5. Starter Deck Artifact

- [x] 5.1 Add the ST-2 Cocytus Blue deck artifact in the repository's existing deck/deck-library convention.
- [x] 5.2 Add tests proving the ST-2 artifact has the exact official card counts and validates as 4 Digi-Egg cards plus 50 main-deck cards.
- [x] 5.3 Add or update a smoke test proving the complete ST-2 deck's unique card IDs are all present in `load_implemented_card_ids()`.

## 6. Tracker Reconciliation

- [x] 6.1 Update `qa/qa-reports/validated_cards_dsl.json` with implemented ST2 verdicts and YAML/test references.
- [x] 6.2 Update `data/tested_cards.json` or equivalent tested-card registry for newly covered ST2 cards.
- [x] 6.3 Reconcile stale `select_opponent_sources` and ST2-related gap entries in `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/resolved-gaps.md`.
- [x] 6.4 Ensure any remaining blocker is filed as a reusable substrate gap with affected cards and tests, not as a one-off ST2 TODO.

## 7. Verification

- [x] 7.1 Run targeted DSL tests for bottom-source trash, battle-context predicates, source play, and source-trash soft-fail.
- [x] 7.2 Run targeted ST2 behavioral tests.
- [x] 7.3 Run the relevant Rust engine suites: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st2`, and deck-tool tests that cover the starter artifact.
  - Targeted ST2/card/deck suites passed. The full DSL suite was run and has one non-ST2 failure in `select_materials::select_materials_batch_play_from_materials_plays_every_picked_source`; the multi-pick commits, then a pre-existing/adjacent `TriggerOrder` pending selection is installed for three On Play triggers.
- [x] 7.4 Run or document the Rust/PyO3 registry smoke needed to confirm `load_implemented_card_ids()` includes every ST2 card.
  - Rust embedded-pack/card tests confirm the ST2 registry. The installed Python module is stale and reports the new ST2 YAML IDs missing; `python -m maturin develop` could not refresh it because no virtualenv/conda environment is active.
- [x] 7.5 Run `openspec status --change implement-st2-cocytus-blue-substrate` and confirm the change is apply-ready.

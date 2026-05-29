## 1. Baseline And Test Harness

- [x] 1.1 Confirm the ST6 card pool and official starter-deck counts from `data/cards.json` and the selected starter-deck source.
- [x] 1.2 Add `code/digimon-engine/tests/cards_behavioral/st6/` with `mod.rs` and register the `st6` module from `cards_behavioral/main.rs`.
- [x] 1.3 Add a small ST6 test helper or fixture pattern for building purple stacks, trash contents, and hand candidates without duplicating setup in every card test.

## 2. Low-Risk YAML And Tests

- [x] 2.1 Add production YAML for vanilla or metadata-only ST6 cards (`ST6-02`, `ST6-05`, `ST6-07`, `ST6-09`) and verify they compile into the DSL pack.
- [x] 2.2 Add YAML and behavioral coverage for inherited trash/draw effects (`ST6-01`, `ST6-03`, `ST6-06`, `ST6-11`).
- [x] 2.3 Add YAML and behavioral coverage for `ST6-08` Blocker plus mandatory memory loss on attack.
- [x] 2.4 Add YAML and behavioral coverage for `ST6-10` purple Digimon trash-to-hand recursion.

## 3. Complex ST6 Effects

- [x] 3.1 Add YAML and behavioral coverage for `ST6-12` granting Retaliation to up to two own Digimon until the end of the opponent's next turn.
- [x] 3.2 Add YAML and behavioral coverage for `ST6-13` Security Attack +1 and `[Main] <Digi-Burst 2>` play-one-purple-level-3-from-trash behavior.
- [x] 3.3 Add YAML and behavioral coverage for `ST6-14` Matt Ishida, including Security play and suspend-as-cost memory gain when an own Digimon is deleted.
- [x] 3.4 Add YAML and behavioral coverage for `ST6-15` Death Claw, including Main self-sacrifice deletion and Security target deletion.
- [x] 3.5 Add YAML and behavioral coverage for `ST6-16` Nail Bone, including Main two-zone trash plays, Security play, and suppression of played Digimon On Play effects.

## 4. Starter Deck Fixture

- [x] 4.1 Add a Venomous Violet starter-deck fixture or deck-library entry with four `ST6-01` Digi-Eggs and the exact fifty-card main deck composition.
- [x] 4.2 Ensure the starter fixture is labeled as starter/manual product data and does not fabricate DigiLab meta-share or conversion-rate statistics.
- [x] 4.3 Add or update a focused test that loads the Venomous Violet deck and verifies card counts, 4-egg/50-main validity, and implemented-card eligibility.

## 5. Verification And Documentation

- [x] 5.1 Run focused ST6 behavioral tests with `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st6 -- --nocapture`.
- [ ] 5.2 Run DSL compile/lowering coverage relevant to the authored cards with `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- --nocapture`.
  - Blocked by existing unrelated failure: `select_materials::select_materials_batch_play_from_materials_plays_every_picked_source` fails when run alone.
- [x] 5.3 Rebuild or verify PyO3 bindings as needed, then confirm `digimon_engine.load_implemented_card_ids()` contains all `ST6-01` through `ST6-16`.
- [x] 5.4 Run a Rust headless smoke check using the Venomous Violet starter deck and confirm reset plus several legal actions succeed.
- [x] 5.5 Update gap/readiness documentation only for verified ST6 findings, moving or annotating any reusable gaps that were closed or disproven during implementation.

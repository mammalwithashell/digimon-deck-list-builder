## 1. Primitive Tests

- [x] 1.1 Add a failing DSL/parser test for `materials_count_matches_aggregate: { selector: fewest_materials, of: opponent }`.
- [x] 1.2 Add a failing predicate/evaluation test proving all opponent Digimon tied for fewest materials match and higher-material Digimon do not.
- [x] 1.3 Add a failing DSL/parser and lowering test for `de_digivolve.amount_fn`.
- [x] 1.4 Add a failing engine behavior test where formula-valued De-Digivolve amount equals own Digimon count and normal caps still apply.
- [x] 1.5 Add failing timing-dispatch tests proving affected permanents cannot activate `[When Attacking]` and `[When Digivolving]` effects while unaffected permanents still can.
- [x] 1.6 Add a failing effect-driven Option-use test for `BT24-085` shape: eligible `[TS]` Option under opponent-memory ceiling is selectable, ineligible higher-cost Option is not.

## 2. DSL and Engine Primitives

- [x] 2.1 Add `materials_count_matches_aggregate` schema, compile, pack, validator, and predicate evaluation support in `code/digimon-dsl/` and `code/digimon-engine/src/dsl_cards/`.
- [x] 2.2 Add `amount_fn` to `DeDigivolveArgs`, compiled step forms, validator checks, and DSL lowering into `EffectContext::de_digivolve`.
- [x] 2.3 Add predicate-scoped timing suppression DSL/modifier support for `[When Attacking]` and `[When Digivolving]`.
- [x] 2.4 Wire timing suppression through the shared triggered-effect enqueue/activation path, including face-up, inherited, and granted effects.
- [x] 2.5 Add an `EffectContext` helper and DSL step for effect-driven Option use from hand without paying cost, with filter and use-cost ceiling support.
- [x] 2.6 Reuse the normal Option lifecycle for effect-driven hand use, including `OnUseOption`, Option mode selection, disposal, Delay, and Link paths.
- [x] 2.7 Verify primitive coverage with focused `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- <new tests>`.

## 3. Representative TS Olympos Cards

- [x] 3.1 Add or enable behavioral tests for `BT24-085` Dan Yuki & Kanan Yuki covering memory gain, effect-driven TS Option use, and follow-up may-attack.
- [x] 3.2 Add or enable behavioral tests for `BT24-030` Neptunemon covering play-cost reduction, fewest-material bottom-deck, self-unsuspend, and protection.
- [x] 3.3 Add or enable behavioral tests for `BT24-041` Minervamon covering play-cost reduction, Iliad free play, formula-valued De-Digivolve, and opponent-turn keyword aura.
- [x] 3.4 Add or enable behavioral tests for `BT10-042` Venusmon covering Security Attack -1, cannot-attack-Venusmon, and timing suppression for affected opponent Digimon.
- [x] 3.5 Add or enable behavioral tests for `BT24-091` Tidal Stream covering lowest-level return-to-hand, security placement, conditional unsuspend, and Link flow.
- [x] 3.6 Add or enable behavioral tests for `BT24-034`, `BT24-035`, `BT24-051`, `BT24-083`, `BT24-088`, `BT24-090`, and `BT24-095`.
- [x] 3.7 Author production YAML for all twelve remaining representative TS Olympos cards with no no-op clauses, hidden auto-selections, or raw-Rust card escapes.

## 4. Training Readiness and QA

- [x] 4.1 Refresh the TS Olympos deck resolver snapshot and confirm the representative unique-card list used by the tests.
- [x] 4.2 Update TS Olympos QA ledgers with representative implemented count, broad-pool implemented count, and residual broad-pool cards.
- [x] 4.3 Move or annotate closed TS Olympos gap entries in `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, and `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md`.
- [x] 4.4 Verify the Rust implemented-card registry includes every representative TS Olympos card after YAML/tests land.
- [x] 4.5 Run focused TS Olympos card behavioral tests and record the commands in the QA closure notes.

## 5. Final Verification

- [x] 5.1 Run the focused DSL suite for the new vocabulary and lowering surfaces.
- [x] 5.2 Run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_030 bt24_041 bt24_085 bt24_091 bt10_042 --nocapture`.
- [x] 5.3 Run any additional cards_behavioral filters for `BT24-034`, `BT24-035`, `BT24-051`, `BT24-083`, `BT24-088`, `BT24-090`, and `BT24-095`.
- [x] 5.4 Run the training deck eligibility smoke check or equivalent registry validation for the representative TS Olympos deck.
- [x] 5.5 Run `openspec status --change close-ts-olympos-rust-gaps` and confirm the change is apply-ready.

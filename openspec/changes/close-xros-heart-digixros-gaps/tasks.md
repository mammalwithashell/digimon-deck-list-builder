## 1. Acceptance Tests and Baseline

- [x] 1.1 Add failing Rust behavioral tests for BT10-009 DigiXros material selection, cost reduction, source attachment, and `digixros_count`.
- [x] 1.2 Add failing Rust behavioral tests for BT10-087 granting under-Tamer DigiXros material access for one pending play.
- [x] 1.3 Add failing Rust behavioral tests for BT12-112 pre-attaching a selected `Shoutmon`, reducing cost, and unlocking trash materials before pay cost.
- [x] 1.4 Add failing Rust behavioral tests for BT10-013 `<Material Save>` deletion timing, optionality, recipe filtering, and Tamer placement.
- [x] 1.5 Record the current ignored/partial Xros Heart fixtures and tracker entries that these tests are expected to close.

## 2. DigiXros Transaction Engine

- [x] 2.1 Add a `DigiXrosTransaction` data model for played card, controller, recipe slots, selected material origins, pre-attached materials, zone allowances, cost deltas, and `digixros_count`.
- [x] 2.2 Wire DigiXros alt-path selection into the play-from-hand flow without changing normal play behavior.
- [x] 2.3 Implement recipe-slot validation for selected materials across hand, battle area, trash, and under-Tamer origins.
- [x] 2.4 Compute final play cost from selected/pre-attached material count and per-material cost deltas before payment.
- [x] 2.5 Commit selected materials as sources only after successful cost payment and permanent creation.
- [x] 2.6 Expose transaction-local context to resolving effects that need `was_digixros` and `digixros_count`.
- [x] 2.7 Verify `ACTION_SPACE_SIZE` and existing action IDs remain unchanged.

## 3. Cast-Time Transaction Modifiers

- [x] 3.1 Add effect-context helpers for before-pay-cost handlers to detect and mutate a pending DigiXros transaction.
- [x] 3.2 Implement one-play material-origin extension for under-Tamer cards.
- [x] 3.3 Implement one-play material-origin extension for trash cards.
- [x] 3.4 Implement pre-attached material selection and one-shot transaction cost deltas.
- [x] 3.5 Ensure declined optional transaction modifiers leave the transaction unchanged.
- [x] 3.6 Add regression tests proving transaction modifiers expire after the current play resolves or aborts.

## 4. Material Save and Leave-Battle-Area Source Flows

- [x] 4.1 Remove the current `[Main]` activated-effect lowering for `Keyword::MaterialSave`.
- [x] 4.2 Reimplement `<Material Save X>` as an optional deletion/removal-timed keyword using deletion snapshots.
- [x] 4.3 Filter Material Save source choices through the carrier's printed DigiXros recipe.
- [x] 4.4 Add pending selections for Material Save Tamer destination and count-capped eligible source picks.
- [x] 4.5 Add no-op paths for declined Material Save, no eligible source, and no legal Tamer destination.
- [x] 4.6 Add reusable helpers for leave-battle-area source rescue and source replay effects to consume pre-removal snapshots.

## 5. DSL Schema and Lowering

- [x] 5.1 Add YAML schema support for `kind: digixros` paths with recipe material filters, material zones, and per-material cost deltas.
- [x] 5.2 Lower DigiXros DSL paths into `DigiXrosTransaction` setup rather than normal alternate digivolution.
- [x] 5.3 Add DSL support for transaction modifiers that grant material zones/counts before pay cost.
- [x] 5.4 Add DSL support for pre-attaching selected materials, one-shot cost deltas, and transaction-scoped trash access.
- [x] 5.5 Add DSL support for Material Save keyword lowering from a card's DigiXros recipe.
- [x] 5.6 Reject unsupported DigiXros DSL fields with explicit compile errors.

## 6. Xros Heart Card Authoring

- [x] 6.1 Promote or author production YAML for BT10-009 with pure DSL DigiXros behavior and passing tests.
- [x] 6.2 Promote or author production YAML for BT10-087 with pure DSL under-Tamer transaction modification and passing tests.
- [x] 6.3 Promote or author production YAML for BT12-112 with pure DSL pre-attach, cost reduction, and trash-access behavior and passing tests.
- [x] 6.4 Promote or author production YAML for BT10-013 with pure DSL DigiXros and Material Save behavior and passing tests.
- [x] 6.5 Remove or update `_examples` comments that no longer describe true gaps after production authoring lands.

## 7. Verification and Documentation

- [x] 7.1 Run focused Rust tests for DigiXros transaction, Material Save, and the four acceptance cards.
- [x] 7.2 Run relevant DSL parser/lowering tests for the new vocabulary.
- [x] 7.3 Run the broader `code/digimon-engine` test suite or document any pre-existing unrelated failures.
  - Focused coverage passed for `digixros`, Material Save, DSL DigiXros lowering, and BT10-009/BT10-013/BT10-087/BT12-112 behavioral acceptance tests.
  - `cargo test -p digimon-dsl` is currently blocked by unrelated stale `parse_source_selection_steps.rs` expectations for `CompiledStep::SelectOpponentDpBudget` (`filter` field and `CompiledFormula` payload).
  - `cargo test -p digimon-engine` is currently blocked by unrelated stale selection tests using the old `select_union_zone` signature in `tests/selection/union_zone.rs` and `tests/selection/behavioral_end_to_end.rs`.
- [x] 7.4 Update `docs/RUST_ENGINE_GAPS.md` for closed DigiXros, transaction-hook, and Material Save gaps.
- [x] 7.5 Update `qa/archetype-qa/engine-gaps.md` and `qa/dsl-vocab-gaps.md` with closure notes and remaining Xros Heart blockers.
- [x] 7.6 Update the Xros Heart archetype readiness/QA report with the post-change verdict and any next-batch card gaps.

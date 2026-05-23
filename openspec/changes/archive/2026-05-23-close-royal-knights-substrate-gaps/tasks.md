## 1. Reconcile Current State

- [x] 1.1 Resolve the Royal Knights card pool and list all YAML files, raw_rust uses, ignored tests, and gap comments for the affected cards.
- [x] 1.2 Verify each cited Royal Knights gap against current `code/digimon-engine/src/` and `code/digimon-dsl/src/`, classifying it as open substrate, closed substrate, card-authoring backlog, or out of scope.
- [x] 1.3 Record the reconciliation result in the Royal Knights gap rollup before changing card bodies.

## 2. Close Reusable Substrate Gaps

- [x] 2.1 Add failing engine and DSL tests for optional breeding-permanent selection accept, decline, mandatory, and no-candidate behavior.
- [x] 2.2 Implement optional breeding-permanent selection through the DSL and engine pending-selection path.
- [x] 2.3 Add or verify tests proving event-bound keyword grants can target the triggering played Digimon without over-targeting unrelated Digimon.
- [x] 2.4 Add or verify DSL tests for `select_materials` / `play_from_materials` against breeding carriers with name uniqueness and On Play suppression.
- [x] 2.5 Add or verify card-shaped tests proving `select_opponent_dp_budget` can replace BT17-018's raw_rust budgeted delete approximation.

## 3. Migrate Royal Knights Cards

- [x] 3.1 Migrate BT17-018 Gallantmon: Crimson Mode to native `select_opponent_dp_budget` plus bound deletion, and remove the single-pick raw_rust approximation.
- [x] 3.2 Author BT13-112 Omnimon's delete/source-play modal branches with different-name Royal Knight breeding-source selection, On Play suppression, breeding trash, and Rush grant.
- [x] 3.3 Author BT13-110 Royal Knights of the Purge's optional hand-to-King-Drasil source placement and Delay source-play/Rush flow.
- [x] 3.4 Author BT20-083 Omekamon's optional On Deletion placement under breeding King Drasil and inherited security-removed source-play flow where substrate permits.
- [x] 3.5 Author BT20-017 Jesmon's token play and other-Digimon-play observer with delete and may-attack choice.
- [x] 3.6 Author BT23-072 King Drasil_7D6's hand-main source placement, played-Digimon keyword grant, and inherited breeding-source play where substrate permits.
- [x] 3.7 Revisit BT23-013, BT13-019, EX11-053, BT20-021, BT23-057, and BT23-058 after reconciliation and migrate any now-unblocked clauses.

## 4. Behavioral Coverage

- [x] 4.1 Add or re-enable active behavioral tests for each migrated card's accept path, decline path, and negative cases.
- [x] 4.2 Assert action masks or pending selections for every player-visible Royal Knights choice introduced by this change.
- [x] 4.3 Ensure remaining ignored tests cite only code-verified open primitives and include tracker references.

## 5. Documentation And Verification

- [x] 5.1 Update `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, and the Royal Knights rollup with closures, residual blockers, and card-authoring status.
- [x] 5.2 Run focused DSL, selection, and card behavioral tests for the changed primitives and cards.
- [x] 5.3 Run the broader Rust engine verification suite needed for Royal Knights card coverage.
- [x] 5.4 Confirm no `ACTION_SPACE_SIZE` or tensor contract changed; if it did, update all required action/tensor docs, exports, wrappers, and metadata in the same change.

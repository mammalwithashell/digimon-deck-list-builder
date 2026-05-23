## 1. Reconciliation

- [x] 1.1 Re-read printed text for the targeted Royal Knights cards and confirm which clauses belong to event-context coverage, union/source flows, or out-of-scope aggregate/formula work.
- [x] 1.2 Reconcile current ignored Royal Knights tests, YAML gap comments, and tracker entries against current engine/DSL capabilities.
- [x] 1.3 Update the implementation slice list so `BT13-019`, `BT20-021`, `EX11-053`, and related observer cards each name one current reusable blocker.

## 2. Event-Context Test Coverage

- [x] 2.1 Add failing Rust behavioral tests for played-Digimon event payload predicates, including controller, level, cause, and played permanent binding.
- [x] 2.2 Add failing Rust behavioral tests for security removed, added, and trashed event payload predicates.
- [x] 2.3 Add failing Rust behavioral tests for self-scoped event-target predicates and negative cases where another Digimon caused the event.
- [x] 2.4 Add failing Rust behavioral tests for same-level X digivolution event payloads and observer filtering.
- [x] 2.5 Add failing Rust behavioral tests that prove optional and mandatory event-context selections expose the correct PASS legality.

## 3. Event-Context Implementation

- [x] 3.1 Extend engine event payloads or trigger context accessors for played, digivolved, security removed, security added, and security trashed events.
- [x] 3.2 Add DSL predicates for event-target self comparison, event-target level branching, played-Digimon cause/origin, and same-level X digivolution checks.
- [x] 3.3 Add DSL effect binding for event participants so keyword grants, attack permission, deletion, and attach clauses target the triggering permanent.
- [x] 3.4 Verify event-context choices route through pending selections and action masks with no hidden auto-selection.

## 4. Union/Source Test Coverage

- [x] 4.1 Add failing Rust behavioral tests for trash-or-breeding-source union play with fixed name exclusions.
- [x] 4.2 Add failing Rust behavioral tests for hand-or-trash source-placement costs, including unpayable and optional decline cases.
- [x] 4.3 Add failing Rust behavioral tests for hand-or-source play effects that bind the played permanent for attach-self follow-up.
- [ ] 4.4 Add failing Rust behavioral tests for different-name constraints spanning heterogeneous union candidate zones.
- [x] 4.5 Add a contract guard test proving the scoped union/source implementation does not change `ACTION_SPACE_SIZE` unless a separate contract change exists.

## 5. Union/Source Implementation

- [ ] 5.1 Add a reusable pending-selection candidate model for heterogeneous hand, trash, breeding-source, and battle-source candidates.
- [ ] 5.2 Implement source-zone validation and resolution so selected candidate handles remain legal at selection time.
- [x] 5.3 Implement source-placement costs from hand or trash as atomic gates before effect success bodies resolve.
- [x] 5.4 Implement source-play operations that optionally suppress On Play effects according to printed text.
- [x] 5.5 Bind cards played or placed by union/source operations for follow-up attach, keyword, Rush, or cleanup clauses.

## 6. Royal Knights Card Migration

- [ ] 6.1 Migrate `BT13-019` to native YAML/tests using trash-or-breeding-source union play.
- [ ] 6.2 Migrate `BT20-021` to native YAML/tests for hand-or-trash source-cost, source-bound follow-up effects, and any still-open non-scope blockers documented separately.
- [ ] 6.3 Complete `EX11-053` On Deletion YAML/tests for hand-or-source play plus attach-self binding.
- [ ] 6.4 Complete scoped event-observer cards whose only remaining blocker is event-context coverage: `BT15-084`, `BT20-060`, `BT23-035`, `BT23-047`, `BT8-090`, `BT9-092`, `BT13-095`, `BT21-086`, and `RB1-035`.
- [ ] 6.5 Complete scoped union-adjacent cards whose only remaining blocker is union/source coverage, including `BT20-056` and `EX11-069` where printed text fits this scope.

## 7. Tracker and Validation Closeout

- [ ] 7.1 Update `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`, and the Royal Knights QA rollup with closed primitives and remaining blockers.
- [ ] 7.2 Re-enable or replace ignored Royal Knights tests whose blockers are closed by this change.
- [ ] 7.3 Run targeted Rust engine card tests for the migrated Royal Knights cards.
- [ ] 7.4 Run the relevant DSL lowering/tests for the new vocabulary.
- [ ] 7.5 Run `openspec validate complete-royal-knights-event-union-gaps --strict`.

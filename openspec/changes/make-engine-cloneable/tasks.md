# Tasks

Incremental, parity-guarded. The data-VM and legacy closure executor coexist behind a per-effect switch until the pool is fully migrated; every batch stays green against `cards_behavioral` + archetype interaction tests + the DCGO recording parity harness before proceeding.

## 1. Foundations (no behavior change)

- [ ] 1.1 `Arc`-share the immutable registries on the clone path: `card_data`, `effect_registry`, `formula_extensions`, `token_registry`, `alt_path_registry`, `rules`, `logger` (matches `reset_for_replay`'s immutable set).
- [ ] 1.2 Convert Category-A behavior closures (modifier predicates/effects) to `Arc<dyn Fn>` shareable handles.
- [ ] 1.3 `derive(Clone)`/ensure `Clone` on all plain per-game data (`Player`, zones, `ModifierRegistry` data, `EffectQueue` data, counters, `rng`).
- [ ] 1.4 Add a migration tracker doc and a CI job that runs both execution paths during the transition.

## 2. Resumable effect VM

- [ ] 2.1 Design the interpreter state: instruction pointer, binding/value stack, typed frame stack; decide bytecode vs tree-walking (resolve design Open Question).
- [ ] 2.2 Implement the interpreter over the compiled DSL AST (`digimon-dsl` `compiled.rs`/`step.rs`) behind a per-effect switch, alongside the legacy closure path.
- [ ] 2.3 Convert the ~30 `select_*` helpers (`effect_context/selections.rs`) into data yields: halt with a `PendingSelection` record; resume pushes the choice and continues at the saved IP.
- [ ] 2.4 Represent pay-cost continuations, parked replacements, granted-effect bodies, and the effect queue as typed VM frames (replace the `Box<dyn FnOnce>` slots on `Game`).
- [ ] 2.5 Validate the no-approximations invariant: every choice still surfaces via `pending_selection`.

## 3. Card-pool migration (parity-gated)

- [ ] 3.1 Migrate cards to the VM in batches by set/archetype; gate each batch on `cards_behavioral`, archetype interaction tests, and DCGO parity.
- [ ] 3.2 Migrate the nastiest multi-pick / pay-cost / replacement archetypes early to validate the frame-stack design (or simplest-first per the resolved Open Question).
- [ ] 3.3 Port or constrain the single `raw_rust` effect to be clone-safe.

## 4. Make Game cloneable

- [ ] 4.1 Once all parked slots are data and the pool is on the VM, `derive(Clone)` on `Game`; share immutables via `Arc`, deep-copy mutable data.
- [ ] 4.2 Add the clone-independence and clone-replays-identically guard tests (clone at a decision point, drive both with identical inputs, assert identical outcome; assert mutating the clone leaves the original unchanged).
- [ ] 4.3 (Optional, profiled) introduce structural sharing / COW for zones to keep clone near-O(1) for untouched state.
- [ ] 4.4 (Optional) add `Serialize`/`Deserialize` for game state if save/load is in scope (resolve Open Question).

## 5. raw_rust clone-safety enforcement

- [ ] 5.1 Add a guard test/lint that fails if a `raw_rust` effect parks a non-`Clone` continuation on `Game`.
- [ ] 5.2 Amend CLAUDE.md rule 28 with the clone-safety constraint (atomic or resume-state-providing); decide whether `formula_extensions` need the same treatment.

## 6. Cutover & docs

- [ ] 6.1 Delete the legacy closure executor once the pool is fully migrated and both-path CI is green.
- [ ] 6.2 Update `docs/RUST_ENGINE_API.md`: the VM execution model and that the reset-and-replay contract now has a snapshot alternative (note replay-stepper conversion is a separate follow-on).
- [ ] 6.3 Note in `add-model-evaluation-harness` that the equilibrium-methods horizon (Deep CFR / ReBeL / Player of Games) is now unblocked.

## 7. Validation

- [ ] 7.1 `openspec validate make-engine-cloneable --strict` passes.
- [ ] 7.2 Full `cargo test --manifest-path code/digimon-engine/Cargo.toml` green, including `--test cards_behavioral` and the archetype tests.
- [ ] 7.3 DCGO recording parity harness green across the migrated pool.
- [ ] 7.4 Clone guard tests green; raw_rust clone-safety guard green.

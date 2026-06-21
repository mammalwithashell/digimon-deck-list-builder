# Tasks

Incremental, parity-guarded. The data-VM and legacy closure executor coexist behind a per-effect switch until the pool is fully migrated; every batch stays green against `cards_behavioral` + archetype interaction tests + the DCGO recording parity harness before proceeding.

## 0. Prerequisites & de-risking spike

- [x] 0.1 **Sequence after the in-flight DSL consolidations — LANDED (archived 2026-06-20).** `collapse-dsl-step-idioms`, `unify-dsl-scalar-and-comparators`, and `fix-dsl-substrate-rot-and-bugs` have all merged to main, so the `dsl_cards/step/` surface is now stable to start on (no concurrent-rewrite / rule-31 contamination risk). **Caveat (verified post-merge 2026-06-18):** the two Tier-0-relevant simplifications those proposals described did **not** materialize — the dp/play-cost budget verbs were NOT merged (still **7** trampolines), and active `raw_rust` is **4**, not 0 (capped at <3% of the pool by `raw_rust_budget_status`). So Tier-0 scope is essentially unchanged from the inventory; the win is a stable, conflict-free surface, not a smaller one.
- [ ] 0.2 **De-risking spike:** defunctionalize ONE recursive trampoline (`count_capped` — exercises the multi-pick accumulator, composition, and the decline path) behind the coexistence switch (`PendingSelection.callback` stays; add `resume: Option<ResumeStack>`; `resolve_generic_selection` runs `resume` if `Some`, else the closure). **Success criterion (corrected 2026-06-18):** `Game` cannot be cloned until the whole pool is on data frames (task 4.1), so the spike does NOT clone a live Game. Instead prove: (a) **behavioral differential** — count_capped cards routed through the data-frame path pass their `cards_behavioral` tests identically to the closure path; and (b) **round-trip** — the parked `ResumeStack` for count_capped is `Clone` + serializable in isolation. The whole-Game clone-then-replay-equals-original assertion lives in task 4.2, after the pool is migrated. Validates the frame-stack design before committing to the full-pool migration.

## 1. Foundations (no behavior change)

- [x] 1.1 `Arc`-share the immutable registries on the clone path. *(Verified 2026-06-18: `effect_registry`, `formula_extensions`, `token_registry`, `rules`, `alt_path_registry` ALREADY derive `Clone` — effects are internally `Arc`-shared — so they do NOT block `Game: Clone`. `card_data` is `Vec<CardData>` (`Clone`); wrapping it in `Arc` for cheap clone is the D5 optimization, deferred to 4.3. `logger` is NOT `Arc`-shareable — its trait methods take `&mut self` — so it is handled by the manual `impl Clone` instead (see 4.1), not by Arc-sharing.)*
- [x] 1.2 Convert Category-A behavior closures (modifier predicates/effects) to `Arc<dyn Fn>` shareable handles. *(Done 2026-06-18: modifier predicates + granted bodies were already `Arc`; converted the last straggler `replacement::ReplacementConditionFn` from `Box<dyn Fn>` → `Arc<dyn Fn>`.)*
- [x] 1.3 `derive(Clone)`/ensure `Clone` on all plain per-game data. *(Done 2026-06-18: `Player`, zones, `EffectQueue` (`= VecDeque<QueuedEffect>`, effects stored by `card_id`+slot), counters, and `rng: StdRng` were already `Clone`; added `Clone` to the `ModifierRegistry` family (1.2) and `ParkedReplacement`. A throwaway `derive(Clone)` on `Game` confirms ALL plain per-game data is now `Clone` — the only residual blockers are the 3 non-data fields in 4.1.)*
- [ ] 1.4 Add a migration tracker doc and a CI job that runs both execution paths during the transition.

## 2. Resumable effect VM

- [x] 2.1 Design the interpreter state: typed frame stack. *(Done 2026-06-18: `code/digimon-engine/src/resume.rs` defines `ResumeStack` + `ResumeFrame` {`RunTail`, `MultiPickStep`} + `ResumeProvenance` — all `Clone`, compiling against the real engine types (`CompiledStep`/`Bindings`/`StepRuntime`/`CompiledPredicate`/`TriggerContext`). Bytecode-vs-tree-walking Open Question resolved to **tree-walking**: the executor is already a tree-walking interpreter over `CompiledStep`, so we defunctionalize the continuation rather than rewrite to bytecode. Wiring this into `PendingSelection` (coexistence `resume` field) + porting `count_capped` is the 0.2 spike — next.)*
- [ ] 2.2 Implement the interpreter over the compiled DSL AST (`digimon-dsl` `compiled.rs`/`step.rs`) behind a per-effect switch, alongside the legacy closure path.
- [ ] 2.3 Convert the ~30 `select_*` helpers (`effect_context/selections.rs`) into data yields: halt with a `PendingSelection` record; resume pushes the choice and continues at the saved IP.
- [ ] 2.4 Represent pay-cost continuations, parked replacements, granted-effect bodies, and the effect queue as typed VM frames (replace the `Box<dyn FnOnce>` slots on `Game`).
- [ ] 2.5 Validate the no-approximations invariant: every choice still surfaces via `pending_selection`.

## 3. Card-pool migration (parity-gated)

- [ ] 3.1 Migrate cards to the VM in batches by set/archetype; gate each batch on `cards_behavioral`, archetype interaction tests, and DCGO parity.
- [ ] 3.2 Migrate the nastiest multi-pick / pay-cost / replacement archetypes early to validate the frame-stack design (or simplest-first per the resolved Open Question).
- [ ] 3.3 Port or constrain the remaining `raw_rust` effects to be clone-safe (4 as of 2026-06-18: `bt24_012`/`lm_027`/`bt21_093`/`bt13_040`; `bt24_012` is already a no-op placeholder).

## 4. Make Game cloneable

- [ ] 4.1 Once all parked slots are data and the pool is on the VM, implement `Clone` on `Game`; share immutables via `Arc`, deep-copy mutable data. **MILESTONE (verified 2026-06-18 via a throwaway `derive(Clone)` on `Game`):** after Phase-1 foundations, exactly THREE fields block `Game: Clone` — (1) `pending_selection: Option<PendingSelection>` (the `Box<dyn FnOnce>` callbacks — gated on the VM/spike), (2) `logger: Box<dyn GameLogger>`, (3) `reveal_source: Option<Box<dyn RevealSource>>`. Use a **manual `impl Clone`** (not `derive`): clone all data fields, and on the clone set `logger = Box::new(SilentLogger)` and `reveal_source = None` (both are external/diagnostic state a forked search node should not inherit). So once `pending_selection` is data, `Game: Clone` is a small manual impl away.
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

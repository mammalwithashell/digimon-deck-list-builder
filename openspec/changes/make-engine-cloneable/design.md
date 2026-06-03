## Context

`Game` (`code/digimon-engine/src/game.rs`) derives only `Debug`. Its non-`Clone` surface splits into two categories:

- **Category A — behavior closures (`Fn`):** `ConditionFn`, `ProcessFn`, `PayCostFn`, modifier predicates. Created from card definitions, immutable "code." `effect_registry` is built once (`build_registry()`) and treated as immutable by `reset_for_replay`. Some are already `Arc<dyn Fn>`. Fix is mechanical (Arc-share, don't deep-clone).
- **Category B — continuation closures (`FnOnce`):** `SelectionCallback = Box<dyn FnOnce(&mut Game, choice)>` and ~30 `select_*` sites in `effect_context/selections.rs`, plus pay-cost continuations, parked replacements (`parked_replacement`), and granted-effect bodies. These capture "the rest of this effect after the player picks" — a paused call stack. **This is the real blocker**; a `FnOnce` cannot be `Arc`-shared and represents unique mid-computation state.

Everything else in `Game` (zones, memory, counters, `rng: StdRng`, `card_data`) is plain data that is trivially `Clone`. The card pool is 477 DSL YAML specs vs 1 `raw_rust` effect, and `digimon-dsl` already produces a compiled data AST (`compiled.rs`/`step.rs`) — today wrapped into closures at registry-build time. The DSL-first migration (rule 28) has already done the hard part: the cards are data; only the executor is closures.

## Goals / Non-Goals

**Goals:**
- `Game: Clone` producing an independent copy that replays identically; ideally `Serialize`/`Deserialize` too.
- Cheap clone (Arc-shared immutables + structural sharing where it pays).
- Exact behavioral parity with the current closure executor across the full card pool.
- A single, centralized executor rewrite — not a 477-card per-card rewrite.

**Non-Goals:**
- Implementing MCTS / Deep CFR / ReBeL / Player of Games (this change only unblocks them).
- Converting the debugger's reset-and-replay to snapshot back-step (a deliberate follow-on, enabled but not delivered here).
- Changing card *authoring* (YAML/DSL surface stays the same).
- Removing the `raw_rust` escape hatch (it is constrained, not deleted).

## Decisions

**D1 — Defunctionalize the executor, not the cards.** Re-express effect execution as a resumable interpreter over the existing compiled AST. In-flight state = `(card, effect-slot, instruction pointer, binding/value stack, frame stack)`, all plain data. This covers all 477 DSL cards at once because they share one executor. Alternative (make each closure `Clone` via `dyn-clone`) rejected: `FnOnce` continuations capturing the stack cannot be meaningfully cloned, and it would not yield serializability.

**D2 — Selections become data yields, not callbacks.** `select_*` halts the VM with a `PendingSelection` data record; `resolve_selection` pushes the choice onto the VM's binding stack and resumes at the saved instruction pointer. The ~30 `select_*` helpers become VM opcodes/yield points. The no-approximations rule is preserved: every choice still surfaces through `pending_selection`. Alternative (keep `FnOnce` but snapshot its captured env) rejected: Rust closures are not introspectable/serializable.

**D3 — Nested/parked computations become an explicit frame stack.** Pay-cost continuations, parked replacements, granted-effect bodies, and the effect queue become typed VM frames pushed/popped by the interpreter, so a paused multi-pick / pay-cost / replacement flow is fully described by data. This is the hardest part and is where parity risk concentrates. Alternative (special-case each parked slot) rejected: a uniform frame stack is simpler to clone and reason about than N bespoke data encodings.

**D4 — Arc-share immutable code and registries; deep-clone only mutable data.** Category-A closures become `Arc<dyn Fn>`; `card_data`, `effect_registry`, `formula_extensions`, `token_registry`, `alt_path_registry`, `rules`, `logger` are `Arc`-shared on clone (matching how `reset_for_replay` already treats them as immutable). Only the mutable per-game data is deep-copied. Alternative (deep-clone everything) rejected: `card_data` and registries are large and never mutated during play.

**D5 — Optional structural sharing / copy-on-write for hot clone paths.** For search, clone happens at every decision node. Persistent/COW data structures (e.g. `im` or hand-rolled `Arc`-backed zones) keep clone near-O(1) for untouched state. Treated as an optimization layered after correctness, gated on profiling.

**D6 — Incremental, parity-guarded cutover.** The VM and the legacy closure path coexist behind a per-effect switch; cards move over in batches, each batch kept green against `cards_behavioral`, the archetype interaction tests, and the **DCGO recording parity harness** (differential oracle). The legacy path is deleted only when the pool is fully migrated. Alternative (big-bang switch) rejected: unacceptable parity risk for the engine's core.

**D7 — raw_rust becomes clone-safe by policy.** A `raw_rust` effect must be either atomic (no mid-effect `select_*`) or provide an explicit, `Clone`-able resume-state implementing the VM frame contract. Enforced by a guard test / lint and CLAUDE.md rule 28. Trivial today (n=1); standing constraint going forward.

**D8 — `reset_for_replay` stays, and gains a sibling.** Once `Game: Clone`, snapshot back-step becomes possible, but converting the replay stepper is out of scope; `reset_for_replay` remains the supported path and the new `Clone` is validated by a "clone-then-replay-equals-original" guard test.

## Risks / Trade-offs

- **Parity regressions in nested resolution (multi-pick, pay-cost, replacement)** → Mitigation: uniform frame stack + incremental cutover + DCGO differential oracle + the per-set behavioral suite as a gate on every batch.
- **Interpreter slower per-op than native closures** → Mitigation: the engine is already fast (release build constructs <100ms); for search, clonability outweighs per-step cost; opcode dispatch can be optimized; structural sharing keeps clone cheap.
- **Hidden non-determinism breaking "clone replays identically"** → Mitigation: RNG is `StdRng` (clonable, deterministic); guard test asserts a clone diverges only by injected inputs, not by internal state.
- **Scope creep into the replay/back-step rewrite** → Mitigation: explicit non-goal; only a `Clone` guard test here.
- **Long migration window with two execution paths** → Mitigation: per-effect switch + a migration tracker; CI runs both paths until cutover completes.
- **`raw_rust` future cards needing mid-effect choice** → Mitigation: such a card must be expressed in the DSL (widen the substrate, rule 28) or implement the resume-state contract; the guard makes the requirement explicit at author time.

## Migration Plan

1. Land the data-VM executor alongside the closure path (feature-switched), plus Arc-sharing of registries and Category-A closures; `Game` not yet `Clone`.
2. Migrate cards to the VM in batches by set/archetype, gating each batch on `cards_behavioral` + archetype interaction tests + DCGO parity.
3. Once the pool is fully on the VM and all parked slots are data, `derive(Clone)` (+ `Serialize`) on `Game`; add the clone-replays-identically guard test.
4. Add the raw_rust clone-safety guard/lint and amend CLAUDE.md rule 28.
5. Delete the legacy closure executor. Rollback at any step is reverting the in-progress batch; the legacy path remains until step 5.

## Open Questions

- VM shape: a flat bytecode + operand stack, or a tree-walking resumable interpreter with an explicit continuation stack? (Bytecode serializes/clones more cheaply; tree-walking is closer to the current AST.)
- Serialization format and stability guarantees — is on-disk save/load in scope, or only in-memory `Clone`?
- Do we adopt a persistent-data-structure crate (`im`) for zones, or hand-roll `Arc`-backed COW?
- Does `formula_extensions` (raw_rust DSL formula callbacks) need the same clone-safety treatment as raw_rust effects?
- Migration ordering: simplest sets first to de-risk the VM, or the nastiest multi-pick archetypes first to validate the frame-stack design early?

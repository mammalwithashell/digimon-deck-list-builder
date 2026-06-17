## Why

The Rust engine's `Game` is **not `Clone`** (it derives only `Debug`). It is a mutable, closure-bearing graph: modifiers hold `Arc/Box<dyn Fn>`, and in-flight effect resolution is held as `Box<dyn FnOnce>` continuations (`SelectionCallback`, pay-cost callbacks, parked replacements, granted-effect bodies). A `Game` therefore cannot be cheaply forked or snapshotted, only reconstructed by reset-and-replay.

This single property blocks the entire search/equilibrium frontier — MCTS/AlphaZero-style search, Deep CFR, ReBeL, Player of Games all require cheap state forking to build/traverse a game tree — and forces the debugger's back-step to replay-from-turn-1 instead of restoring a snapshot. The surprising leverage: the card pool is already **477 DSL YAML specs vs 1 hand-written `raw_rust` effect**, and the DSL already lowers to a compiled **data** AST. The closures are an implementation choice of the *executor*, not inherent to the cards. So "make `Game` cloneable" reduces to "execute the DSL as a resumable data-VM instead of compiling it to closures."

## What Changes

- **Resumable, data-driven effect VM** — **BREAKING (internal execution model):** replace the closure-based effect executor with an interpreter over the compiled DSL AST whose entire in-flight state is plain data: an instruction pointer, a binding/value stack, and an explicit frame stack for nested effects. A player choice becomes "halt with a pending-selection record; on resume, push the choice and continue at the saved instruction pointer" — no `Box<dyn FnOnce>` continuation captured.
- **Defunctionalize all parked-computation slots** — `pending_selection`, pay-cost continuations, parked replacements, granted-effect bodies, and the effect queue become VM frames (data), not boxed closures.
- **Arc-share immutable behavior and registries** — modifier predicate/effect closures become `Arc<dyn Fn>` (shallow-shareable on clone), and the already-immutable shared registries (`card_data`, `effect_registry`, `formula_extensions`, `token_registry`, `alt_path_registry`, `rules`) are `Arc`-shared rather than deep-cloned.
- **`derive(Clone)` (and `Serialize`/`Deserialize`) on `Game`** and all per-game data, producing an independent, faithful copy and a serializable state.
- **rule-28 amendment: raw_rust clone-safety** — the hand-written escape hatch must be clone-safe (atomic, i.e. no mid-effect player selection, OR provide an explicit resume-state); enforced by a guard/lint. Update CLAUDE.md rule 28.
- **Parity preserved throughout** via incremental, card-by-card cutover with the `cards_behavioral` suite and the DCGO recording parity harness as differential oracles.

## Capabilities

### New Capabilities
- `resumable-effect-vm`: DSL effects execute as a resumable, data-state interpreter; all in-flight resolution state (instruction pointer, bindings, frame stack, pending selection, pay-cost, replacement, granted bodies) is plain `Clone`/serializable data rather than boxed closures.
- `cloneable-game-state`: `Game` is `Clone` and (de)serializable, yielding an independent copy that replays identically; immutable registries are `Arc`-shared so cloning is cheap.
- `raw-rust-clone-safety`: the `raw_rust` escape hatch must be clone-safe (atomic or resume-state-providing), enforced by a guard and documented in rule 28.

### Modified Capabilities
<!-- None: card authoring (dsl-card-scripting-vocabulary) is unchanged; replay/back-step snapshot optimization is a deliberate follow-on, not part of this change. -->

## Impact

- **Code**: `code/digimon-dsl/` (the executor → VM; `compile.rs`/`compiled.rs`/`step.rs`), `code/digimon-engine/src/effect.rs`, `effect_context/selections.rs` (the ~30 `select_*` helpers), `effect_queue.rs`, `modifiers.rs`, `selection.rs`, and `game.rs` (`Game` fields, `derive(Clone)`, parked-state slots, `reset_for_replay` interplay). The single `code/digimon-engine/src/cards/raw_rust/` effect.
- **Policy/docs**: CLAUDE.md rule 28 (raw_rust clone-safety constraint); `docs/RUST_ENGINE_API.md` (reset-and-replay contract gains a snapshot alternative; VM execution model).
- **Performance**: a bytecode interpreter may cost more per op than native closures; mitigated by structural sharing / copy-on-write for large immutable state (`card_data`, untouched zones).
- **Unlocks**: `make-engine-cloneable` is the precondition for the equilibrium-methods horizon in `add-model-evaluation-harness`, plus snapshot back-step, game save/load, and network state resync.
- **Unlocks (session persistence)**: faithful durable PvP session state — restart-survival of the hosted API's in-memory `active_games`, cross-process game migration, and mid-effect snapshot/restore — depends on `Game: Clone`/`Serialize` delivered here. `to_scenario()` is **not** a substitute: it is a lossy static-board capture that drops `pending_selection` callbacks, modifiers, the effect queue, RNG, and `pending_attack`/option/pay-cost/replacement state, so restoring a session paused mid-resolution would silently corrupt it. The non-blocked **interim** for restart-survival only is *replay-from-seed*: persist `(deck1, deck2, seed, action_history)` per session and rebuild via the existing `reset_for_replay`/undo machinery (accepting replay cost + the existing divergence guard). A future session-management-hardening change is gated on this one for anything beyond that interim.
- **Risk surface**: deepest rewrite of the engine's execution core; guarded by the existing behavioral + DCGO-parity test suites and a coexisting-paths incremental migration.

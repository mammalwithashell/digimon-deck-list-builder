## Why

A bare-engine benchmark (`tests/bench_engine_throughput.rs` — greedy/random ST-1
self-play, release, no Python/PyO3/NN) shows the Rust engine plays only ~14-20
games/sec (~475-645 steps/sec), ~150× slower per game than peer TCG simulators.
A per-phase breakdown proves the engine **step** (`game.decode_action`) is
**73-92%** of all time at **1.4-3.5 ms/step**; mask-build (~3%), policy (~2%), and
per-game construction (~22%, separately cacheable) are minor. The engine — not the
Python/NN training harness (only ~6× on top) — is the dominant bottleneck, and the
real lever toward higher RL throughput and the many-rollout speed MCTS/AlphaZero
search needs.

Root cause: `decode_action` calls `tick_declarative_effects()` 2-3× per action,
and each call **clears all materialized declarative modifiers and rebuilds them
for the entire board from scratch** — scanning every permanent, stack source,
breeding, and face-up security card (a `String` heap alloc per card) and re-running
every continuous-effect condition+process closure. The cost is O(board) (greedy's
33-step games 1.4 ms/step vs random's fuller 69-step boards 3.5 ms/step),
confirming this function is the hot path.

## Measured outcome (post-implementation correction)

The implementation is complete and behavior-preserving, but **measurement
invalidated the premise above**. Instrumentation shows `tick_declarative_effects`
is **~1.1–1.5% of total runtime**, NOT 73–92% — that figure conflated "the engine
step is 70–90% of the run" (true) with "the declarative rebuild dominates the step"
(false). A same-binary A/B (incremental skip ON vs `force_full` OFF, interleaved to
cancel machine noise) shows **no measurable difference** (GREEDY 495–505 vs
503–505 steps/s). So this change is **perf-neutral for linear play**.

The real bottleneck is `effects_for_card` — **56–72% of total runtime**, of which
**90% is `impl_.effects(handle)` re-boxing per-instance closures** (~742–1715
calls/step, 100% registry hits). Because `CardEffect::effects(handle)` is a *pure*
function of `(card_id, handle)`, it is memoizable; that is the real ≥2× lever and
is scoped as a **separate** change, `cache-effects-for-card`.

**This change is kept (not reverted)** as a down-payment on the cloneable-engine /
DSL data-VM roadmap: incremental, allocation-light, byte-identical materialization
plus the differential oracle are exactly what cheap tree-search clones need (see
design.md §"Alignment upside"). It makes no throughput claim.

## What Changes

- **Make declarative-effect materialization incremental/cheaper, preserving EXACT
  correctness.** This is a pure performance optimization: byte-identical
  materialized modifier state, no behavioral, DSL-vocabulary, card-script,
  tensor, or action change; all behavioral/card/archetype/parity tests stay green.
- **Dirty-flag the declarative state** so `tick_declarative_effects` re-materializes
  only when the board changed in a way that can affect declarative sources
  (zone/stack/breeding/face-up-security mutations), skipping the full rebuild on
  the majority of actions that don't touch continuous sources.
- **Collapse the redundant 2-3× per-action tick** to a single tick per action.
- **Eliminate the per-card `String` allocation** (intern IDs / carry `&str`).
- *(Optional, deeper)* fully incremental install/remove of a source's declaratives
  on enter/leave instead of clear-and-rebuild.
- **Guard with a correctness oracle** (a debug invariant / differential test
  asserting the optimized path yields identical materialized modifiers vs the
  always-rebuild path) and **track `bench_engine_throughput.rs` as the regression
  meter** with a measured speedup target.

## Capabilities

### New Capabilities
- `declarative-effect-materialization`: the engine SHALL materialize continuous /
  declarative effects incrementally (re-materializing only when the board changed
  in a relevant way), producing materialized modifier state identical to the
  always-rebuild baseline, and SHALL guard that equivalence with a correctness
  oracle. Adds a benchmark-backed engine-step throughput target.

### Modified Capabilities
<!-- none — engine behavior is unchanged; this is an internal performance change -->

## Impact

- **Code:** `code/digimon-engine/src/game/triggers.rs` (`tick_declarative_effects`),
  `code/digimon-engine/src/action/decode.rs` (the 2-3× call sites),
  `code/digimon-engine/src/modifiers.rs` (clear/materialize + dirty flag), and the
  per-step game-mutation sites that must set the dirty flag.
- **No behavior change:** no DSL vocabulary, card-script, tensor, or action-space
  change; the full behavioral/card/archetype/parity suites must stay green.
- **Regression meter:** `code/digimon-engine/tests/bench_engine_throughput.rs`
  (already added) measures engine steps/sec before/after.
- **Risk:** the correctness hinges on invalidating exactly when a declarative
  source or a condition-relevant input changes; condition-dependent declaratives
  may need conservative (always re-evaluate) handling. The oracle de-risks this.
- **Out of scope:** harness levers (batched opponent inference, leaner tensor),
  multi-threading, and per-game registry construction caching — smaller, tracked
  separately.
- **Alignment:** incremental, allocation-light, byte-identical state is also what
  the cloneable-engine / DSL data-VM roadmap needs for cheap tree-search clones.

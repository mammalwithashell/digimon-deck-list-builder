## Why

A bare-engine profile (`tests/bench_engine_throughput.rs`, release, ST-1 self-play)
shows `Game::effects_for_card` is **56% (greedy) / 72% (random) of total runtime**,
and **~90% of that is the registry `impl_.effects(handle)` call** re-boxing
per-instance `Box<dyn Fn>` closures — ~742 (greedy) / ~1715 (random) calls/step,
**100% registry hits**. It is called from ~70 sites (mask-build, cost/DP calc, the
trigger drain in `effect_queue.rs`, replacement scans, the declarative tick, …),
re-deriving the same card's effect list from scratch every time.

This is the engine's true per-step bottleneck. (The prior
`optimize-declarative-effect-materialization` change targeted the *declarative
rebuild*, which a later measurement proved is only ~1.5% of runtime — the wrong
lever. This change targets the real one.)

The key enabler: `CardEffect::effects(&self, card: CardHandle)` takes **only the
handle and no game state**, so the registry effect list is a **pure function of
`(card_id, handle)`**. The only game-state input in `effects_for_card` itself is
`card_handle_is_under_top(handle)` (it gates the inherited-keyword auto-effects).
So the *entire* result is a pure function of `(card_id, handle, under_top)` and can
be **memoized for a game's lifetime with zero invalidation**.

## What Changes

- **Memoize `effects_for_card` per `(card_id, handle, under_top)` on `Game`.** Add a
  per-`Game` cache; on a hit, hand back the already-built effect list instead of
  re-running `impl_.effects(handle)` + the keyword synthesis.
- **Return a shared handle, not an owned `Vec`** (full ref-return — no per-call
  clone). Because `Game` must be `Send` (`RustHeadlessGame` is a non-`unsendable`
  `#[pyclass]`), the cache value is **`Arc<Vec<Effect>>`** (the existing `Box`
  closures live once inside the shared `Arc`'d vec — **no `Box`→`Arc` change to the
  closures is needed**; a cache hit is an `Arc` refcount bump). `effects_for_card`
  returns `Option<Arc<Vec<Effect>>>`.
- **Ripple the return-type change to the ~70 callers** (mechanical: `for e in
  effects` → `effects.iter()`). Migrate the hot per-step callers first
  (mask/cost/dp/effect_queue/triggers); each migration is a compiling checkpoint.
- **Handle the 2 mutating callers** (`game_actions/cost.rs:439`,
  `game_actions/options.rs:595`, which take `let Some(mut effects)`) by iterating
  the shared slice and collecting what they need, or routing them through an
  owned-returning builder.
- **No behavior change.** The cached value is byte-identical to the freshly-built
  list (pure key); all behavioral/card/archetype suites stay green. Guard with the
  same kind of debug oracle the declarative change uses (a cache-vs-fresh
  differential assert), and re-measure on `bench_engine_throughput.rs`.

## Capabilities

### New Capabilities
- `effects-for-card-memo`: the engine SHALL memoize per-card effect derivation by
  `(card_id, handle, under_top)`, returning a shared `Arc<Vec<Effect>>` identical to
  a fresh build, guarded by a debug differential oracle, with a benchmark-backed
  engine-step throughput target (≥2×).

### Modified Capabilities
<!-- none — engine behavior is unchanged; this is an internal performance change -->

## Impact

- **Code:** `code/digimon-engine/src/game/mod.rs` (`effects_for_card` + the cache
  field + construction), the ~70 caller sites (return-type ripple), and the 2
  mutating callers. No DSL/card-script/tensor/action change.
- **Regression meter:** `code/digimon-engine/tests/bench_engine_throughput.rs`
  (target ≥2× engine steps/sec; the profile predicts the boxing is ~50–65% of run).
- **Risk:** (1) cache key completeness — `under_top` is the only game-state input,
  but the oracle proves equivalence; (2) `Send` — keep `Arc<Vec<Effect>>` values so
  `Game` stays `Send` (verify the binding crate compiles); (3) the large caller
  ripple — migrate incrementally with compiling checkpoints.
- **Verification:** full `cards_behavioral` in **release** (the debug-oracle full
  run trips the known flaky stack abort) + the oracle on a declarative-machinery
  subset + the PyO3/headless smoke (a real speedup must carry through the harness).
- **Alignment:** allocation-light, shared-immutable effect lists also reduce the
  per-clone cost on the cloneable-engine / MCTS roadmap.

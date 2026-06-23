## ADDED Requirements

### Requirement: Memoized per-card effect derivation

The engine SHALL memoize `effects_for_card` per `(card_id, handle, under_top)` for a
game's lifetime, returning a shared `Arc<Vec<Effect>>` that is byte-identical to a
fresh build from the registry and keyword synthesis. A cache hit MUST NOT re-run
`CardEffect::effects(handle)` or re-synthesize keyword auto-effects, and MUST NOT
clone the effect vector (only the `Arc`). The memo MUST require no invalidation —
because `handle` is unique-and-stable per instance, the registry and card data are
immutable for a game, and `under_top` is part of the key.

#### Scenario: Repeated query returns the cached list
- **WHEN** `effects_for_card` is called more than once for the same
  `(card_id, handle, under_top)` within a game
- **THEN** every call after the first returns the same memoized `Arc<Vec<Effect>>`
  (a refcount bump, no rebuild) and the contents equal a fresh build

#### Scenario: Under-top change is a distinct key
- **WHEN** a card's `under_top` status changes (e.g. it is digivolved over)
- **THEN** the query keyed on the new `under_top` builds (and caches) the correct
  effect list, and does not return the entry cached for the other `under_top` value

### Requirement: Cache equivalence is oracle-guarded and Send-safe

Debug/test builds SHALL verify, on each cache hit, that the memoized effect list is
equivalent to a fresh build, and SHALL fail loudly on divergence; release builds run
only the fast path. The cache value SHALL be `Arc<Vec<Effect>>` so that `Game`
remains `Send` (the PyO3 binding `RustHeadlessGame` requires it). The full
behavioral, card, and archetype suites SHALL pass while running under the oracle
(on the declarative-machinery subset and in release for the full card binary).

#### Scenario: Oracle catches a wrong cache key
- **WHEN** a game mutation that should change a card's effect list fails to change
  the cache key
- **THEN** the debug oracle detects the mismatch between the cached and freshly-built
  lists and fails a test, rather than silently serving stale effects

#### Scenario: Binding crate stays Send
- **WHEN** `digimon-engine-py` is compiled with the cache field present on `Game`
- **THEN** `RustHeadlessGame` (a non-`unsendable` `#[pyclass]` holding a `Game`)
  compiles without a `Send` error

### Requirement: Engine-step throughput improves without behavior change

The optimization SHALL improve engine steps/sec on the bare-engine benchmark
(target ≥2×) without changing any behavioral test outcome, and SHALL NOT ship unless
the behavioral suites are green and the benchmark shows the improvement.

#### Scenario: Benchmark shows the speedup
- **WHEN** `bench_engine_throughput.rs` is run in release before and after the change
- **THEN** engine steps/sec is materially higher (target ≥2×) and the per-phase
  breakdown shows `effects_for_card`'s share dropped

#### Scenario: Behavior is preserved
- **WHEN** the behavioral / card / archetype suites run after the change
- **THEN** every test produces the same result as before (verified under the debug
  oracle on the declarative-machinery subset and in release for the full card suite)

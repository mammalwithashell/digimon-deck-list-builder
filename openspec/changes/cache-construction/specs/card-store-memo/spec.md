## ADDED Requirements

### Requirement: Memoized shared card store

The engine SHALL memoize the enriched card store — `card_data` (`Vec<CardData>`
with DSL alt-path enrichment and absorbed global token rows) and its
`card_id_index` — per a content fingerprint of `all_card_data`, and share it across
games via `Arc` so that `Game::new` does an `Arc` clone rather than re-cloning and
re-enriching the ~4000-card store. The shared store MUST be byte-identical to a
fresh per-game build, and `Game.card_data` MUST NOT be mutated after construction.

#### Scenario: Second game reuses the shared store
- **WHEN** a second `Game` is constructed from the same `all_card_data` within a
  process
- **THEN** it reuses the memoized `Arc` store (no re-clone, no re-enrichment) and
  the store contents equal a fresh build

#### Scenario: Different card data rebuilds
- **WHEN** a `Game` is constructed from an `all_card_data` whose content differs
  (a different DB or an override change)
- **THEN** the fingerprint differs and the engine builds (and caches) the correct
  store for it, never serving the other DB's store

### Requirement: Sharing preserves Send and behavior

The card store SHALL be shared via `Arc` such that `Game` remains `Send` (the PyO3
binding `RustHeadlessGame` requires it), and the `CardStore` field SHALL be
`Deref`-transparent so existing `card_data` reads are unchanged. The full
behavioral, card, and archetype suites SHALL pass with the shared store.

#### Scenario: Binding stays Send
- **WHEN** `digimon-engine-py` is compiled with the shared `CardStore` on `Game`
- **THEN** `RustHeadlessGame` (a non-`unsendable` `#[pyclass]`) compiles without a
  `Send` error

#### Scenario: Behavior is preserved
- **WHEN** the behavioral / card / archetype suites run with the shared store
- **THEN** every test produces the same result as before (verified in release for
  the full card suite + the oracle subset)

### Requirement: Construction throughput improves, measured

`Game::new`'s card-store cost SHALL drop from a clone-and-enrich to an `Arc` clone,
improving construction throughput on the bare-engine benchmark without changing any
behavioral outcome, and SHALL NOT ship unless the behavioral suites are green and
the benchmark shows the improvement.

#### Scenario: Benchmark shows the construction speedup
- **WHEN** `bench_engine_throughput.rs` is run in release before and after
- **THEN** the construct phase's share of the run drops materially and overall
  steps/sec increases, with the per-card-store build no longer dominating

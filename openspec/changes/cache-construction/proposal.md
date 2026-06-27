## Why

After the `effects_for_card` cache, a training-loop profile shows **`Game`
construction is now ~61% of the per-step cost** (the env builds a fresh `Game`
every episode via `reset()`). Inside construction, `CardRegistry::from_cards` is
9% and **`Game::new` is 91% (~14 ms/game)** — dominated by re-deriving the
immutable card store on every game:

1. `all_card_data.clone()` — clones the whole ~4000-card `HashMap`;
2. `enrich_card_data_with_dsl_alt_paths(...)` — DSL alt-path compilation over it;
3. building `card_data_store: Vec<CardData>` with a **second** per-card `.clone()`
   + the `card_id_index`, then absorbing the (global) token rows.

All of this is a **pure function of `all_card_data`** — identical for every game in
a process (the binding passes a stable `&'static` card DB) — yet redone per game.

## What Changes

- **Memoize the enriched card store** `(card_data: Vec<CardData>, card_id_index:
  HashMap<String,usize>, alt_path_registry)` in a process-global cache keyed by a
  content fingerprint of `all_card_data`, behind `Arc`. `Game::new` does a
  fingerprint + `Arc` clone (refcount bump) + deck deal instead of clone+enrich.
- **Share, don't clone.** `Game.card_data` becomes a `CardStore` newtype wrapping
  `Arc<Vec<CardData>>` with `Deref<Target=Vec<CardData>>` + `Clone`, so the ~3126
  `.card_data` accesses across the workspace (engine/tests/binding/tauri) stay
  transparent (indexing, `.iter()`, `.len()`, `&card_data` as `&[CardData]` all go
  through `Deref`); `card_id_index` becomes `Arc<HashMap<…>>`. Only a handful of
  breakers (explicit `Vec<CardData>` bindings, deep-clone expectations) need edits.
- **No behavior change.** The shared store is byte-identical to the per-game build
  (the global token registry + the same enrichment); `card_data` is immutable
  post-construction, so sharing is safe. All behavioral/card/archetype suites stay
  green; re-measure on `bench_engine_throughput.rs`.

## Capabilities

### New Capabilities
- `card-store-memo`: the engine SHALL memoize the enriched card store + index per
  `all_card_data` fingerprint and share it across games via `Arc`, producing a
  store identical to the per-game build, with a benchmark-backed construction
  throughput target.

### Modified Capabilities
<!-- none — engine behavior is unchanged; internal performance change -->

## Impact

- **Code:** `code/digimon-engine/src/game/setup.rs` (`Game::new` build → cache
  lookup), `game/mod.rs` (`card_data`/`card_id_index` field types + `CardStore`),
  a new `card_store` cache module, and the breaker `.card_data` sites the compiler
  flags. No DSL/card-script/tensor/action change.
- **Regression meter:** `bench_engine_throughput.rs` — construction is 61% of the
  per-step training cost; the target is to collapse `Game::new` toward an `Arc`
  clone (estimate ~2.3× on the engine-side of training).
- **Risk:** (1) cache-key completeness — fingerprint must capture all of
  `all_card_data` (content, not just ids); the per-game build stays available as
  the reference. (2) the `Deref` newtype must keep `Game: Send` (the cache value is
  `Arc<…>`, `Send`); verify `digimon-engine-py` compiles. (3) large `.card_data`
  ripple — `Deref` keeps it mostly transparent; the compiler enumerates breakers.
- **Verification:** full `cards_behavioral` in **release** + oracle subset +
  `digimon-engine-py` (Send) + PyO3/headless smoke.

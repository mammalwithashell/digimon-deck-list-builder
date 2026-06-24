## Context

`Game::effects_for_card(card_id, handle) -> Option<Vec<Effect>>` ([game/mod.rs:2463])
is called ~742–1715×/step (56–72% of runtime; 90% of that is the registry
`impl_.effects(handle)` re-boxing per-instance `Box<dyn Fn>` closures). It is
re-derived from scratch every call even though the result is a pure function of
`(card_id, handle, under_top)`:

```
registry_effects = effect_registry.get(card_id).map(|i| i.effects(handle))  // PURE(card_id,handle), ~90% cost
auto_effects     = cd.keywords → keyword_to_auto_effect(kw, handle)         // PURE(card_id,handle)
                 + IF card_handle_is_under_top(handle): inherited keywords   // game-state gate (the only one)
                 + registry grant_keyword auto-effects                       // PURE(card_id,handle)
combine(registry_effects, auto_effects)
```

`CardEffect::effects(&self, handle)` ([effect.rs:1225]) takes only the handle — no
`&Game`. So the entire result depends on game state *only* through `under_top`.

## Goals / Non-Goals

**Goals**
- Eliminate the repeated `impl_.effects(handle)` rebuild via a per-`Game` memo;
  target **≥2× engine steps/sec** on `bench_engine_throughput.rs`.
- **Byte-identical behavior** + a debug oracle proving cache == fresh build.
- Keep `Game: Send` (binding crate compiles).

**Non-Goals**
- No `Box`→`Arc` on the effect closures (not needed — see Decisions).
- No DSL/card-script/tensor/action change. No declarative-tick change (that was the
  separate, perf-neutral `optimize-declarative-effect-materialization`).

## Decisions

1. **Cache value = `Arc<Vec<Effect>>`, key = `(String card_id, CardHandle, bool under_top)`.**
   `Arc` (not `Rc`) because `Game` must be `Send` (`RustHeadlessGame` is a
   non-`unsendable` `#[pyclass]`). The `Box<dyn Fn + Send + Sync>` closures live
   once inside the shared vec — a cache hit clones the `Arc` (refcount bump), never
   the vec, so **no `Box`→`Arc` change to closures is required**.

2. **Interior mutability = `RefCell<HashMap<…, Arc<Vec<Effect>>>>`.** `effects_for_card`
   is `&self` (70 callers depend on that); the memo fills lazily on a `&self` call.
   `RefCell<HashMap<…, Arc<…>>>` is `Send` (so `Game` stays `Send`). It is not
   `Sync`, which is fine — each `Game` is owned/accessed single-threaded (under the
   GIL on the PyO3 path). If a `Game: Sync` bound surfaces at compile time, swap to
   `Mutex` (≈2% per-call overhead) — but it is not expected.

3. **No invalidation.** The key is the full set of inputs, and `handle` (card_index)
   is unique-and-stable per instance, `card_id`/registry/`card_data` are immutable
   for a game, and `under_top` is in the key. So an entry is never stale. The cache
   is per-`Game` (not shared across games) and bounded by
   `#instances × {top, under_top}`. Cleared only on construction (empty).

4. **Return `Option<Arc<Vec<Effect>>>`; migrate callers incrementally.** Changing the
   return type breaks all ~70 callers at once, so instead add the cache behind the
   existing signature path and migrate caller-by-caller (each a compiling
   checkpoint): hot per-step first (mask.rs, cost.rs, combat/dp.rs, effect_queue.rs,
   game/triggers.rs), then the rest. Read-only callers: `effects.iter()`. The 2
   mutating callers (cost.rs:439, options.rs:595) iterate the shared slice and
   collect, or call an owned-returning `build_effects_for_card` (the un-memoized
   body) — they are low-frequency.

5. **`under_top` cost.** `card_handle_is_under_top(handle)` scans the battle area
   (O(board)); it is already computed in the current hot path and is far cheaper
   than the `impl_.effects(handle)` rebuild it gates. Acceptable as the residual
   per-call cost; a later change can memoize position too if it shows up.

6. **Debug oracle.** Under `cfg(debug_assertions)`, on a cache *hit*, rebuild fresh
   and `debug_assert!` the two effect lists are equivalent (compare a stable
   projection — timing/flags/keyword/inherited per slot, since `Effect` holds
   un-comparable closures). Run the declarative-machinery subset + behavioral suites
   under it. Release runs only the fast path.

## Risks / Trade-offs

- **Cache-key incompleteness → stale effects.** Mitigated by the pure-key argument +
  the debug oracle across the corpus + the owned-build reference.
- **`Send` regression** from the cache field. Mitigated by `Arc` values; verified by
  compiling `digimon-engine-py`.
- **Large caller ripple.** Mitigated by incremental migration with checkpoints; the
  hot callers alone bank most of the win, so partial migration is still shippable.
- **Memory:** the cache holds every distinct `(card_id, handle, under_top)` effect
  list for the game's life — bounded and small (tens of entries), freed with the
  `Game`.

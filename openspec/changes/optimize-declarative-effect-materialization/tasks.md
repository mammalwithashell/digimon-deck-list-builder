## 1. Baseline + correctness oracle (do FIRST — the safety net)

- [ ] 1.1 Record the pre-change benchmark baseline: run `bench_engine_throughput.rs` (release, greedy + random) and capture games/sec, steps/sec, and the construct/mask/policy/engine-step split as the regression reference.
- [ ] 1.2 Add a `force_full_rebuild` switch (config/env or const) that keeps the existing always-rebuild path selectable, so it survives as the oracle baseline + a fallback.
- [ ] 1.3 Add the correctness oracle: under `cfg(debug_assertions)` (or a test feature), each `tick_declarative_effects` runs BOTH the (eventual) fast path and a fresh full rebuild and `debug_assert!`s the materialized modifier sets are identical. Wire it so the behavioral/card/archetype/parity suites exercise it.

## 2. Cheap win — remove the per-card `String` allocation

- [ ] 2.1 Replace the `sources` Vec's `String` card-id with a `Copy` interned card id / registry index (preserving the borrow-break the owned collection currently provides); thread it through `effects_for_card` / the de-dup `card_data_by_id` lookups.
- [ ] 2.2 Re-run behavioral/card suites (green) + the benchmark; record the steps/sec delta.

## 3. Cheap win — collapse the 2-3× per-action tick

- [ ] 3.1 Determine the minimum set of `tick_declarative_effects` calls in `decode_action` (currently `:46` pre, `:49` post-selection, `:86` post) that preserves identical behavior (some actions read materialized state mid-resolution); reduce to that minimum.
- [ ] 3.2 Run the full behavioral/card/archetype/parity suites under the oracle (no divergence) + benchmark; record the delta.

## 4. Dirty-flag the declarative state (the big win)

- [ ] 4.1 Add a `declaratives_dirty` flag (on `Game`/`ModifierRegistry`); make `tick_declarative_effects` early-return when not dirty and clear the flag after a rebuild.
- [ ] 4.2 Set the flag at the curated, deliberately-broad invalidation chokepoints: battle-area / stack / breeding / face-up-security mutations, plus the dynamic inputs declarative conditions read (turn/phase transitions, memory, suspend, DP, board counts). Start conservative — correctness over precision.
- [ ] 4.3 Run the full behavioral/card/archetype/parity suites under the oracle. Any missed invalidation must surface as a failing assert; fix the chokepoint set until the corpus is clean.

## 5. Verify + measure

- [ ] 5.1 Full suites green under the oracle (`cargo test` engine + `cards_behavioral` + `archetypes`), confirming byte-identical behavior.
- [ ] 5.2 Re-run `bench_engine_throughput.rs`; record the engine-step steps/sec speedup vs the 1.1 baseline and confirm the target (≥2× engine steps/sec; note actual). If a lever underperforms, log it (no silent cap).
- [ ] 5.3 Spot-check the PyO3/training path (run a short `pilot_training`/headless smoke) to confirm the speedup carries through the harness and nothing regressed.

## 6. Docs + follow-ups

- [ ] 6.1 Note the engine-step optimization + the oracle in `docs/RUST_ENGINE_API.md` (or the engine perf notes) and add a memory capturing the bottleneck + result.
- [ ] 6.2 If the conservative invalidation leaves measurable wins on the table, file a follow-up for static- vs dynamic-condition declarative classification (narrower invalidation) — do NOT scope-creep it here.

## 1. Cache infra + owned-build split (foundation, compiling checkpoint)

- [ ] 1.1 Record the pre-change benchmark baseline (`bench_engine_throughput.rs`, release, greedy + random) — reuse the `optimize-declarative-effect-materialization` baseline (~456–505 greedy steps/s) or re-capture on the current HEAD.
- [ ] 1.2 Split `effects_for_card` into a private `build_effects_for_card(card_id, handle) -> Option<Vec<Effect>>` (the current body, owned, un-memoized — the oracle/owned-caller reference) and the public memoizing wrapper. Compute `under_top` once and thread it so the body needn't recompute.
- [ ] 1.3 Add the cache field to `Game`: `effects_cache: std::cell::RefCell<std::collections::HashMap<(String, CardHandle, bool), Option<std::sync::Arc<Vec<Effect>>>>>` (default empty); wire it through every `Game` constructor / `Default`. Confirm `digimon-engine-py` still compiles (`Game: Send` preserved).

## 2. Memoizing wrapper + debug oracle

- [ ] 2.1 Implement the public `effects_for_card(...) -> Option<Arc<Vec<Effect>>>`: compute the key, return the cached `Arc` on hit; on miss call `build_effects_for_card`, wrap in `Arc`, insert, return the clone.
- [ ] 2.2 Add the debug oracle (`cfg(debug_assertions)`): on a cache hit, rebuild via `build_effects_for_card` and `debug_assert!` a stable projection (per-slot timing/inherited/declarative/granted_keyword/materializes flags) matches. Release runs only the fast path.

## 3. Migrate callers to the `Arc` return (incremental — each a checkpoint)

- [ ] 3.1 Migrate the HOT per-step callers first: `action/mask.rs` (3), `game_actions/cost.rs`, `combat/dp.rs` (2), `effect_queue.rs` (~20), `game/triggers.rs` (2), `action/main_effect_select.rs`. Read-only sites: `effects.iter()`.
- [ ] 3.2 Handle the 2 mutating callers (`game_actions/cost.rs:439`, `game_actions/options.rs:595`, `let Some(mut effects)`): iterate the shared slice and collect, or call `build_effects_for_card` (owned) — pick per site.
- [ ] 3.3 Migrate the remaining cold callers (replacement.rs, game_actions/*, dna_digivolve.rs, game_phases.rs, option_lifecycle.rs, dsl_cards/predicate.rs, game/queries.rs, effect.rs, effect_context/*). Engine `cargo check` green after each batch.

## 4. Verify + measure

- [ ] 4.1 Build green: `cargo check` engine lib + `digimon-engine-py` (Send) + Tauri layer.
- [ ] 4.2 Behavior green: oracle on the declarative-machinery subset (effects/flood_gates/replacements/combat/archetypes/keyword_phase_*) + full `cards_behavioral` in **release** (the full debug-oracle run trips the known flaky stack abort).
- [ ] 4.3 Re-run `bench_engine_throughput.rs` (release); record engine steps/sec vs the 1.1 baseline and confirm the target (≥2×; note actual). If it underperforms, log why (no silent cap) — e.g. residual `under_top`/auto-effect cost.
- [ ] 4.4 PyO3/headless smoke (a short `pilot_training` or headless run) to confirm the speedup carries through the harness and nothing regressed.

## 5. Docs + follow-ups

- [ ] 5.1 Note the memo + oracle in `docs/RUST_ENGINE_API.md` (effects_for_card section) and update the `project_engine_perf_effects_for_card` memory with the measured result.
- [ ] 5.2 If `under_top` recomputation or the auto-effect synthesis is now the residual hot spot, file a follow-up (memoize card position; or cache the two `under_top` variants together) — do NOT scope-creep here.

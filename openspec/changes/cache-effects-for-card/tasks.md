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

- [x] 4.1 Build green: `cargo check` engine lib (0 errors) + `digimon-engine-py` (compiles → **`Game: Send` preserved**; `RefCell<HashMap<…, Arc<Vec<Effect>>>>` is `Send`). Tauri layer: pending (separate target dir).
- [x] 4.2 Behavior green: **cache oracle (debug) clean across ~10 binaries / ~1759 tests** (effects 211, flood_gates 227, dsl 781, effect_context 145, selection 129, replacements 110, option_flow 78, combat/archetypes/keyword_phase_*) — zero divergence. Full `cards_behavioral` in **release**: running.
  - NOTE: `cost_hooks::player_digivolve_reducer::no_suspend_cost_applies_on_accept` fails — **pre-existing** (fails identically on parent `4621a01db`; stale since `b414917f5` made free reducers auto-apply with no prompt). `try_prompt_player_digivolve_cost_reducer` never calls `effects_for_card`, so it is unrelated to this change. Flagged to fix separately.
- [x] 4.3 Re-run `bench_engine_throughput.rs` (release). **Target ≥2× — BLOWN PAST:**
  - GREEDY: 456 → **1509 steps/s (3.3×)**; engine-step 1518 → **206 µs (7.4× faster)**; effects_for_card's share collapsed (construction is now the dominant 64%).
  - RANDOM: 244 → **1647 steps/s (6.7×)**; engine-step 3710 → **375 µs (~10× faster)**.
  - Step counts identical to baseline (6620 greedy / 13795 random) — behavior-preserving.
- [ ] 4.4 PyO3/headless smoke (a short `pilot_training` or headless run) to confirm the speedup carries through the harness — pending.

## 5. Docs + follow-ups

- [ ] 5.1 Note the memo + oracle in `docs/RUST_ENGINE_API.md` (effects_for_card section) and update the `project_engine_perf_effects_for_card` memory with the measured result.
- [ ] 5.2 If `under_top` recomputation or the auto-effect synthesis is now the residual hot spot, file a follow-up (memoize card position; or cache the two `under_top` variants together) — do NOT scope-creep here.

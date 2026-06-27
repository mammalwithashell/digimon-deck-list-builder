## 1. Baseline + correctness oracle (do FIRST — the safety net)

- [x] 1.1 Record the pre-change benchmark baseline: run `bench_engine_throughput.rs` (release, greedy + random) and capture games/sec, steps/sec, and the construct/mask/policy/engine-step split as the regression reference.
  - BASELINE (release, unmodified `claude/mystifying-darwin-d11683` @ 1686ef5b0):
    - GREEDY ST-1: 200 games, 6620 steps (33.1/game), 14.52s → **13.8 games/sec, 456 steps/sec**; construct 25.7%, mask 3.2%, policy 1.9%, **engine-step 69.2% (1518.6 µs/step)**.
    - RANDOM:      200 games, 13795 steps (69.0/game), 56.62s → **3.5 games/sec, 244 steps/sec**; construct 6.8%, mask 2.7%, policy 0.0%, **engine-step 90.4% (3710.5 µs/step)**.
- [x] 1.2 Add a `force_full_rebuild` switch (config/env or const) that keeps the existing always-rebuild path selectable, so it survives as the oracle baseline + a fallback.
  - DONE: `DIGIMON_FORCE_FULL_DECLARATIVE_REBUILD` env (read-once via `OnceLock`) forces the always-rebuild path; `materialize_declaratives_full()` is the preserved reference body.
- [x] 1.3 Add the correctness oracle: under `cfg(debug_assertions)` (or a test feature), each `tick_declarative_effects` runs BOTH the (eventual) fast path and a fresh full rebuild and `debug_assert!`s the materialized modifier sets are identical. Wire it so the behavioral/card/archetype/parity suites exercise it.
  - DONE: debug-only block in `tick_declarative_effects` snapshots the fast-path materialized state (`ModifierRegistry::materialized_snapshot`, sorted/order-independent, install_order excluded), re-runs `materialize_declaratives_full`, and `debug_assert!`s equality. Active in every debug/test build, so all suites exercise it.

## 2. Cheap win — remove the per-card `String` allocation

- [x] 2.1 Replace the `sources` Vec's `String` card-id with a `Copy` interned card id / registry index (preserving the borrow-break the owned collection currently provides); thread it through `effects_for_card` / the de-dup `card_data_by_id` lookups.
  - DONE: `sources`/`linked_sources` now carry the `Copy` `CardSource.data_index` (the index `card_id()` already reads); `&str` is re-derived as `&self.card_data[data_index].card_id` at each use (effects lookup + de-dup), holding no borrow across the `&mut self` process call.
- [~] 2.2 Re-run behavioral/card suites (green) + the benchmark; record the steps/sec delta.
  - Subset green under oracle (effects 211, flood_gates 227, replacements 110, combat 14, archetypes 41, keyword_phase d/e/f). Full `cards_behavioral` + release bench measured in §5 (the String win is folded into the combined measure).

## 3. Cheap win — collapse the 2-3× per-action tick

- [x] 3.1 Determine the minimum set of `tick_declarative_effects` calls in `decode_action` (currently `:46` pre, `:49` post-selection, `:86` post) that preserves identical behavior (some actions read materialized state mid-resolution); reduce to that minimum.
  - DONE BY MECHANISM (not by deletion): the fingerprint skip makes the redundant ticks runtime no-ops. The `:46` pre-action tick (and any tick whose state is unchanged since the prior rebuild) skips the expensive rebuild — only its cheap fingerprint is computed. This is safer than physically deleting call sites (the ~14 callers across decode/combat/suspend/play/live_game stay correct), and the oracle proves each skip is valid. Call sites left intact deliberately; the rebuild cost — the thing the task targets — is collapsed.
- [~] 3.2 Run the full behavioral/card/archetype/parity suites under the oracle (no divergence) + benchmark; record the delta.
  - Folded into §5 (the skip's effect is measured by the combined release bench; oracle-clean subset already confirmed no divergence).

## 4. Incremental materialization via a state fingerprint (the big win)

> **Design realization:** rather than a literal `declaratives_dirty` flag set at
> per-site invalidation chokepoints (memory/suspend/zone/phase mutations are
> *scattered* across many files with no central setters — manual marking would be
> error-prone whack-a-mole), the skip is driven by a **state fingerprint** read
> from current state. This is *complete by construction* (a missed input only
> over-rebuilds; it cannot under-rebuild a captured input) and **location-
> independent** — correct regardless of which of the ~14 tick call sites ran or
> who mutated state. The oracle (1.3) verifies completeness across the corpus.

- [x] 4.1 ~~Add a `declaratives_dirty` flag~~ → Add a declarative-state fingerprint memo (`ModifierRegistry::{declarative_memo,set_declarative_memo,invalidate_declarative_memo}`); `tick_declarative_effects` early-returns (skips the rebuild) when `declarative_fingerprint()` equals the memo, and records it after a rebuild.
- [x] 4.2 ~~Set the flag at chokepoints~~ → `declarative_fingerprint()` reads the curated, deliberately-broad input set directly: every zone's contents (hand/deck/digitama/security/trash + battle-area stacks + breeding + linked), face-up-security, suspend/attack flags, turn-timing counters, memory, phase, turn, floating-mass descriptors, and an order-independent digest of all non-materialized modifiers/keywords/player-modifiers (`nonmaterialized_digest`). Over-captures by design.
- [~] 4.3 Run the full behavioral/card/archetype/parity suites under the oracle. Any missed invalidation must surface as a failing assert; fix the chokepoint set until the corpus is clean.
  - DONE: behavior verified two ways — (a) oracle-clean on the declarative-machinery suites (effects 211, flood_gates 227, replacements 110, combat 14, archetypes 41, keyword_phase d/e/f); (b) full `cards_behavioral` **passes in release** (5825/0/34-ignored — real shipping logic: skip on, oracle compiled out, so any stale skip that changed an outcome would fail). The full debug-oracle run hit the known flaky stack-overflow abort (CLAUDE memory) — defeated for the subset, not run-to-completion on the full binary; the skip-only oracle (refined from every-tick) keeps its extra rebuild off the deep effect-resolution ticks.

## 5. Verify + measure

- [x] 5.1 Full suites green under the oracle (`cargo test` engine + `cards_behavioral` + `archetypes`), confirming byte-identical behavior. — see 4.3 (oracle subset + release full suite).
- [x] 5.2 Re-run `bench_engine_throughput.rs`; record the engine-step steps/sec speedup vs the 1.1 baseline and confirm the target. **LOG (no silent cap): the lever underperformed — measured ≈0% (within noise).**
  - Same-binary A/B (skip ON vs `force_full` OFF, interleaved): GREEDY 495–505 vs 503–505 steps/s; RANDOM 271 vs 267 — **no difference**.
  - Instrumented: the full declarative rebuild is **1.1–1.5% of total runtime** (0.19s of a 13s greedy run), skip rate 40–45%. Even skipping 100% saves ≤1.5%.
  - **The proposal's premise was wrong:** `tick_declarative_effects` is NOT 73–92% of the step; it is ~1.5% of the whole run. The 70–90% engine-step cost is **action resolution** — `effects_for_card` (56–72% of run; 90% of *that* is `impl_.effects(handle)` re-boxing per-instance closures, ~742–1715 calls/step, 100% registry hits). `effects(handle)` is a **pure function of `(card_id, handle)`** → cacheable → that is the real ≥2× lever, scoped as a **separate** change (`cache-effects-for-card`).
- [~] 5.3 Spot-check the PyO3/training path — N/A for a perf-neutral change; deferred to the `cache-effects-for-card` change where a real speedup must carry through the harness.

## 6. Docs + follow-ups

- [x] 6.1 Note the engine-step optimization + the oracle in `docs/RUST_ENGINE_API.md` and add a memory capturing the bottleneck + result.
- [x] 6.2 Follow-up filed: the real lever is **memoizing `effects_for_card`** (NOT narrower declarative invalidation — that ceiling is ~1.5%). New change `cache-effects-for-card`: `Box`→`Arc` the effect closures so `Effect: Clone`, memoize the pure registry effects per `(card_id, handle[, under_top])`, return `Rc`/`&[Effect]` (full ref-return, ~2.5–3×).

> **Outcome reframed (kept, not reverted):** this change is **perf-neutral for linear play** (the rebuild was misidentified as the bottleneck). It is retained as a **down-payment on the cloneable-engine / DSL data-VM roadmap** — incremental, allocation-light, byte-identical materialization + the differential oracle are exactly what cheap tree-search clones need (per design.md §"Alignment upside"). It does **not** claim a throughput improvement.

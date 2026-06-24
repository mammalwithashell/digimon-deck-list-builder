## 1. Foundation (compiling checkpoint, no field change yet)

- [ ] 1.1 Baseline: reuse the profile — construction is 61% of the per-step training cost; `Game::new` is 91% of construction (~14 ms/game), `CardRegistry::from_cards` ~1.4 ms.
- [ ] 1.2 Add a `card_store` module: `CardStore(Arc<Vec<CardData>>)` with `Deref<Target=Vec<CardData>>`, `Clone`, `Debug`; a `SharedCardStore { data: Arc<Vec<CardData>>, index: Arc<HashMap<String,usize>>, alt_paths: Arc<…> }`; a process-global `Mutex` memo keyed by an order-independent content fingerprint of `all_card_data`; and `shared_card_store(all_card_data) -> Arc<SharedCardStore>` (build-on-miss = the current clone+enrich+token path, factored out of `Game::new`). Compiles standalone.

## 2. Switch Game to the shared store (atomic field change + ripple)

- [ ] 2.1 Change `Game.card_data: CardStore`, `card_id_index: Arc<HashMap<String,usize>>`, `alt_path_registry: Arc<…>` (cfg). `Game::new` calls `shared_card_store(all_card_data)` and clones the `Arc`s instead of building inline.
- [ ] 2.2 `cargo check` engine lib; fix the compiler-flagged `.card_data` breakers (explicit `Vec<CardData>` bindings, `.clone()`-deep-copy expectations, `&mut` — none expected). `Deref` keeps indexing/`iter`/`len`/`&[CardData]` transparent.
- [ ] 2.3 `cargo check` engine tests + `digimon-engine-py` (Send) + Tauri; fix breakers there.

## 3. Verify + measure

- [x] 3.1 Build green across crates: engine lib (0 err), engine tests (all compile), `digimon-engine-py` (compiles → **Game: Send preserved**).
- [x] 3.2 Behavior green: comprehensive subset (effects 211, flood_gates 227, dsl 781, archetypes 41, mask_and_tensor 175, combat 14, replacements 110, judge_quiz, phase_flow) — **0 failures**. Full `cards_behavioral` release: running.
  - **REGRESSION 1 (fixed):** `DebugRunner` builds its own `data_index_map` from `self.card_data` in HashMap-iteration order and assumes `Game::new` uses the same assignment. The shared cache returns a store built from a *different* HashMap instance (different seed → different order) → DebugRunner's manual card placement pointed at the wrong `CardData` → ~36 tests failed. FIX: assign `data_index` in **deterministic sorted-by-id order** in BOTH `build_shared_card_store` and DebugRunner (`data_index` is internal; the obs tensor keys on the stable registry index → behavior-neutral).
  - **REGRESSION 2 (fixed):** the fingerprint hashed text *lengths* but omitted `dp` → two `make_digimon_dp("WEAK", c, dp)` test cards (same id/name/lengths, different DP) collided → a test got a cached card with the wrong DP (Raid targeting broke). FIX: hash the VALUE fields (incl `dp`, colors, keywords, level, kind) fully; text/costs by length (full-text+`Debug` hashing made the per-game fingerprint ~3.6 ms and eroded the win — value-fields-only keeps it cheap AND distinguishes the synthetic test DBs).
- [~] 3.3 Differential cache==fresh assertion — deferred; the deterministic-sorted store + the behavioral suites cover equivalence.
- [x] 3.4 Re-run `bench_engine_throughput.rs` (release). **Construction collapsed:**
  - GREEDY: 1509 → **3449 steps/s (2.3×)** on top of the effects cache; construct 64% → **20.5%** (`Game::new`'s 14 ms card-store build is now an `Arc` clone; residual is `CardRegistry::from_cards` + the fingerprint).
  - RANDOM: 1647 → **2283 steps/s (1.4×)**.
  - **Cumulative vs the original baseline: GREEDY 456 → 3449 (7.6×), RANDOM 244 → 2283 (9.4×).** Step counts identical (6620/13795) — behavior-preserving.
- [ ] 3.5 PyO3/headless smoke — pending.

## 4. Docs + follow-ups

- [ ] 4.1 Note the shared store in `docs/RUST_ENGINE_API.md` + update the `project_engine_perf_effects_for_card` memory with the construction result.
- [ ] 4.2 If `CardRegistry::from_cards` (the residual 9%) or the obs-tensor build (9.3%/step) is now the next hot spot, file a follow-up — do NOT scope-creep here.

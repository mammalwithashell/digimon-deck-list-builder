## 1. Foundation (compiling checkpoint, no field change yet)

- [ ] 1.1 Baseline: reuse the profile — construction is 61% of the per-step training cost; `Game::new` is 91% of construction (~14 ms/game), `CardRegistry::from_cards` ~1.4 ms.
- [ ] 1.2 Add a `card_store` module: `CardStore(Arc<Vec<CardData>>)` with `Deref<Target=Vec<CardData>>`, `Clone`, `Debug`; a `SharedCardStore { data: Arc<Vec<CardData>>, index: Arc<HashMap<String,usize>>, alt_paths: Arc<…> }`; a process-global `Mutex` memo keyed by an order-independent content fingerprint of `all_card_data`; and `shared_card_store(all_card_data) -> Arc<SharedCardStore>` (build-on-miss = the current clone+enrich+token path, factored out of `Game::new`). Compiles standalone.

## 2. Switch Game to the shared store (atomic field change + ripple)

- [ ] 2.1 Change `Game.card_data: CardStore`, `card_id_index: Arc<HashMap<String,usize>>`, `alt_path_registry: Arc<…>` (cfg). `Game::new` calls `shared_card_store(all_card_data)` and clones the `Arc`s instead of building inline.
- [ ] 2.2 `cargo check` engine lib; fix the compiler-flagged `.card_data` breakers (explicit `Vec<CardData>` bindings, `.clone()`-deep-copy expectations, `&mut` — none expected). `Deref` keeps indexing/`iter`/`len`/`&[CardData]` transparent.
- [ ] 2.3 `cargo check` engine tests + `digimon-engine-py` (Send) + Tauri; fix breakers there.

## 3. Verify + measure

- [ ] 3.1 Build green across crates; `digimon-engine-py` compiles (Game: Send).
- [ ] 3.2 Behavior green: oracle subset (effects/flood_gates/replacements/combat/archetypes/dsl) + full `cards_behavioral` in **release**.
- [ ] 3.3 (debug) assert the shared store equals a fresh build on cache miss (cheap differential), exercised by the suites.
- [ ] 3.4 Re-run `bench_engine_throughput.rs` (release); record the construct-phase share drop + steps/sec (target: construction no longer dominates; ~2.3× engine-side). Log if it underperforms (no silent cap).
- [ ] 3.5 PyO3/headless smoke — confirm the speedup carries through the harness.

## 4. Docs + follow-ups

- [ ] 4.1 Note the shared store in `docs/RUST_ENGINE_API.md` + update the `project_engine_perf_effects_for_card` memory with the construction result.
- [ ] 4.2 If `CardRegistry::from_cards` (the residual 9%) or the obs-tensor build (9.3%/step) is now the next hot spot, file a follow-up — do NOT scope-creep here.

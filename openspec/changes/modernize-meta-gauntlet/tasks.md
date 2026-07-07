# Tasks

Phased per design Migration Plan; each phase independently landable and flag-gated. Phases 1–2 are pure Python (gauntlet-internal), Phase 4 touches Rust (D7), Phase 5 is doctrine + tooling.

## 1. Threat Index + format scoping + pool hygiene (D1–D4)

- [ ] 1.1 Replace TI with `share × exp(c·(wr_adj − 0.5))` using k=25 empirical-Bayes shrinkage on DigiLab win rate; drop conversion from weighting (keep in telemetry/summary); add `--ti-legacy` flag preserving the old formula for one A/B release. Unit tests: the two proposal scenarios (popular-even vs fringe-fluke; equal-share win-rate tilt).
- [ ] 1.2 Re-key the sleeper rule on shrunk `wr_adj > sleeper_threshold`; retire `confidence_min_appearances`. Unit test: 6-game 70%-wr fluke does not trigger the floor.
- [ ] 1.3 Dedup decklists by `stable_deck_id` at alias-merge time (highest-preference source wins, all sources recorded). Unit test: identical list under two sources/aliases appears once.
- [ ] 1.4 Implement weighted within-archetype deck routing (`source_pref × recency_decay(event_date)`, normalized); delete the dead source-preference sort or make it feed the weights. Unit test: recent digimonmeta list sampled more than stale low-pref list.
- [ ] 1.5 Add `--gauntlet-window` (date range) → `digilab_client.get_scoped_meta` at run start → inject scoped share/wr before weight computation; fail loudly on query failure unless `--gauntlet-window-optional`. Confirm win/game counts are recoverable for shrinkage (add count columns to the scoped query if `win_rate × times_played` reconstruction is insufficient).
- [ ] 1.6 MetaGauntlet snapshot: write hash-verified snapshot (window, scoped stats, TI params, deck records + weights, pilot bindings, final sampling weights) at run start; restore + verify on resume; record path/hash in run metadata. Tests: round-trip, tamper detection, library-rebuild-mid-run scenario.
- [ ] 1.7 Calibration pass: run the new TI on BT25-window DigiLab data and sanity-check the induced distribution against the retrospective's top-cut table; tune c/k defaults and record the calibration in the design doc.

## 2. Adaptive sampling + bounty retirement (D5, D9)

- [ ] 2.1 Add `MetaGauntlet.reweight(wr_by_arch)` applying `w ∝ TI × (1 − wr_agent)^τ` with k=25 shrinkage on `wr_agent`, per-archetype weight floor, τ knob (τ=0 disables). Unit tests: mastered-matchup scenario; floor holds; τ=0 is a no-op.
- [ ] 2.2 Wire the training loop's eval callback to call `reweight` each eval window from the existing per-archetype win-rate telemetry; log the effective sampling distribution per window.
- [ ] 2.3 Remove the GauntletWrapper bounty (keep `--legacy-bounty` for one release); update rule-26-adjacent docs to note the composition change (magnitudes untouched).
- [ ] 2.4 A/B: one adaptive+no-bounty run vs one legacy run on the anchored frame; record the comparison; delete `--ti-legacy` and `--legacy-bounty` if the new path is non-inferior.

## 3. Joint (deck, pilot) opponents (D6)

- [ ] 3.1 Add pilot bindings to pool entries (`model_path`, `tensor_profile`, `decks_trained_on`); construction-time validation: coherence (exact deck_id or pool-trained archetype match) and tensor-profile match vs env. Unit tests: incoherent binding rejected; profile mismatch fails at construction.
- [ ] 3.2 Thread the sampled pilot through GauntletWrapper → OpponentWrapper (reuse the frozen-ONNX opponent path); greedy fallback for unbound decks, recorded in episode info.
- [ ] 3.3 Telemetry: per-eval-window pilot-class episode fractions (greedy vs frozen) in TB scalars + evals sidecar.
- [ ] 3.4 Bind the existing validated frozen policies (league specialists, AZ generation checkpoints) to their trained decks/archetypes; document the binding file format in the training runbook.

## 4. Meta-weighted self-play (D7)

- [ ] 4.1 `MetaGauntlet.export_selfplay_pool(path)`: deduplicated decks + normalized weights + snapshot hash, in the driver's pool schema.
- [ ] 4.2 Rust driver: accept optional per-deck weights in the pool file; weighted per-seat pair sampling from the master-seeded RNG (uniform when absent); record weights + pool hash + seed in `manifest.json`. Tests: weightless pool byte-identical behavior; golden determinism test (same seed+pool → identical pair sequence); frequency test on a skewed pool.
- [ ] 4.3 `run_selfplay_generation.py --deck-pool <exported.json>` pass-through; generation summary records the pool snapshot hash.
- [ ] 4.4 Switch the standing AZ recipe to a gauntlet-exported, format-windowed pool; run one generation and confirm shard diagnostics + anchored panel look sane.

## 5. Warm-start lineage doctrine (D8)

- [ ] 5.1 Document the lineage policy (warm-start default, non-inherited promotion gates, freeze-with-provenance, N=3 rebase check, embedding NN-init option) in `docs/TRAINING_RUNBOOK.md` + `docs/MODEL_EVALUATION.md`.
- [ ] 5.2 Promotion tooling: champion-registry entries gain format window + gauntlet snapshot hash provenance; promotion path asserts the format-scoped anchored panel was run against the format pool (rule 30).
- [ ] 5.3 Optional export-path flag: initialize new-card embedding rows from attribute-nearest-neighbor cards (off by default); unit test on a synthetic new card.
- [ ] 5.4 Schedule/record the first rebase A/B (lineage vs equal-compute from-scratch) template in the runbook's standing cadence section.

## 6. Validation

- [ ] 6.1 `openspec validate modernize-meta-gauntlet --strict` passes.
- [ ] 6.2 Gauntlet unit suite green (TI, sleeper, dedup, routing, snapshot, reweight, bindings).
- [ ] 6.3 Driver determinism + frequency tests green; weightless path regression-identical.
- [ ] 6.4 End-to-end: one format-windowed, adaptive, pilot-bound PPO run and one weighted-pool AZ generation both complete with telemetry showing scoped weights, pilot mix, and pool provenance.

# Tasks

Phased; each phase independently testable. **Gate analysis REVISED 2026-07-01** (see design "Readiness & sequencing"): `Game: Clone` already exists and the whole DSL selection surface is on the resumable VM — the real gate for the *search* phases (2–5) is `make-engine-cloneable` task 6.1's **non-DSL cutover** (combat interrupts, Overclock, TriggerOrder, replacement-accept, Delay, play-order, pay-cost cohort), because rollouts step through those and a closure-based park clones to a panic-stub. Phases 0 and 1 and the deck-prior phase are **clone-independent** and can start now.

## 0. Clone-independent pre-work (can start NOW)

- [x] 0.1 Throughput go/no-go spike — DONE 2026-07-01, verdict **GO** for AlphaZero-style search: clone 115–184 µs (73–243× cheaper than reset-replay), step 283 µs, step+mask 721 µs → ~1,100 sims/sec/core (clone + 1 step, NN excluded); rollout-style MCTS confirmed non-viable. **New finding: mask build (~60% of a sim) is the top engine-side lever, ahead of `effects_for_card` memoization** (which is unimplemented, so its A/B is deferred to the perf change). Harness: `code/digimon-engine/examples/search_throughput_spike.rs`; full numbers + levers: `spike-throughput.md` in this change dir. Re-run on cloud hardware before sizing a self-play generation.
- [x] 0.2 Value-head ONNX export — DONE 2026-07-01: MLP graph gains an inline `value` output (backward compatible — all consumers fetch by name, detect by presence; verified against `policy.predict_values`); LSTM keeps its main-graph signature and emits a companion `<stem>.value.onnx` threading the CRITIC's own LSTM state (`enable_critic_lstm=True` is the training default, so inline outputs would have added required inputs and broken existing consumers). Metadata gains `value_head: "inline" | "<companion filename>"`. Tests: `code/tests/rl/test_onnx_value_head.py` (2) + existing roundtrip/profile suites (12) green.
- [x] 0.3 Batched ONNX evaluator in Rust — DONE 2026-07-01: `BatchedPolicyValueEvaluator` + `masked_softmax` in `code/digimon-engine/src/inference/evaluator.rs` (loads a 0.2 inline-value export, one `ort` run per batch → per-row masked policy with exactly-zero illegal mass + value; rejects policy-only graphs; LSTM intentionally unsupported — per-node recurrent state is a separate design problem). Parity-tested against a Python-baselined fixture (`mlp_tiny_value.onnx` via `generate_fixtures.py`, new `onnx_parity` tests) + 4 masked-softmax unit tests. Benchmark (`examples/evaluator_bench.rs`, real-shape 8410→2192 model): 128 µs/eval single, 30–43 µs batched → **NN eval is ~5% of a sim; the engine dominates** (numbers in `spike-throughput.md`).
- [x] 0.4 Pre-register robustness baselines — DONE 2026-07-02, full numbers + protocol in **`baselines.md`** (this change dir). Anchored (n=300): starter-MLP 61.0% / v022 61.3% / v020 59.0% vs greedy; starter beats both champions head-to-head. Exploitability at fixed 1M budget (lower bounds): **0.70 / 0.75 / 0.75** — every current agent is farmable by a cheap best-response; skill and robustness measurably decouple. Ran on Hetzner cpx62 boxes (image `training-v0.46` + `exploiter_cli.py` patched in — the CLI postdates the image); RunPod was unusable (8 pods across 3 GPU types + a canary never scheduled, funded account, status page green). Both boxes harvested + deleted.
- [x] 0.5 Review v0 — win-probability bar — DONE 2026-07-01: new `digimon-engine-cli winprob <recording> --model <pv.onnx>` subcommand (`code/digimon-engine-cli/src/winprob.rs`) replays a native GameRecorder recording (or an eval-sidecar envelope — auto-unwrapped), builds the mover's obs tensor + action mask at every decision, batch-evaluates with the 0.3 `BatchedPolicyValueEvaluator`, and emits a `<stem>.winprob.json` sidecar (`schema: winprob-annotations-v1`: per-decision value + played-action prior — the pre-search half of the 5.1 fields). **Gotcha encoded: mulligan rows are NOT replayable (`replayable_action_indices` filters `phase=="Mulligan"`) — iterate the same filter or every obs/mask desyncs from the applied state** (found via all-zero priors; post-fix a real game shows coherent winner/loser value trends + ~1.0 priors on forced picks). Value semantics documented in the sidecar (discounted shaped return, NOT calibrated P(win)). Verified e2e on a complete cloud eval game + the in-repo test recording, using the trained `starter_pool_single_v1` policy+value export. Known gap: the `replay` viewer subcommand does not unwrap eval envelopes (winprob does).

## 1. Game-state determinization (clone-independent EXCEPT search-time use; see design Readiness Correction 2)

- [x] 1.1 (DONE 2026-07-03, `src/determinize/infoset.rs`) `Infoset` data type: public state verbatim + viewer own-hand (exact) + own hidden model as a **joint unseen pool (deck ∪ face-down security) with a count partition** (own security is hidden from its owner too — see design D2 sharpening) + opponent `HiddenModel` (per-zone counts, pinned cards, `DeckPrior`).
- [x] 1.2 (DONE 2026-07-03 — pins = `Player::face_up_security`, the engine's ONLY per-card revelation state; past peeks/reveals are lost at extraction — documented belief-staleness input to Phase 6) `Infoset` extractor = inverse of `server/state_filter.py`; split security into known `face_up_security` vs hidden face-down; compute "pinned" opponent cards proven to be in a hidden zone.
- [x] 1.3 (DONE 2026-07-03, `src/determinize/sample.rs`; D8 clone semantics documented) `SamplingRevealSource` implementing `opaque_deck::RevealSource` over a multiset + seeded `StdRng`.
- [x] 1.4 (DONE 2026-07-03, `src/determinize/materialize.rs` — clone → single Fisher-Yates re-deal of the physical CardSource instances per player → reseed world RNG (covert-channel hygiene); no-X-ray argument in module docs; mid-selection materialization defined, playing THROUGH an opponent-owned re-dealt prompt documented out of scope) World materializer: honor pins, shuffle known multisets into concrete order, draw opponent hidden cards from the `DeckPrior` respecting per-zone counts + copy limits + already-seen exclusions; produce a concrete cloneable `Game` with `reveal_source = None`.
- [x] 1.5 (DONE 2026-07-03, `src/determinize/invariants.rs` + 18/18 tests in `tests/determinization/`: round-trip law over random playouts, statistical no-leak guard, playable-to-completion worlds, tamper detection, opaque two-books reconciliation) Sampler invariant checker (counts balance, ≤copy-limit, no card in two zones; reconcile BOTH `OpaqueDeckState` books — `restore()` never errors and placeholder accounting desyncs `total_remaining` from the per-card multiset by design) + round-trip property test (re-extract infoset from a sampled world == source infoset for the viewer).
- [x] 1.6 (DONE — `extract`/`materialize` assert 2-player non-opaque; `DeckPrior` is an enum with `Exact` only, shaped for the Phase-6 inferred variant) Self-play regime only here (exact `DeckPrior` from the training pool); PvP prior deferred to Phase 6.

## 2. Neural MCTS search core (gate LIFTED 2026-07-02 — the non-DSL surface is resume-backed with zero live-closure production call paths per make-engine-cloneable 6.1; open with a clone-fuzz spike: clone at every decision of random real-deck playouts, resolve both copies, assert identical outcomes)

- [x] 2.1 (DONE 2026-07-03, `src/search/mcts.rs` — arena tree, identity-keyed sign backup (tested vs consecutive same-player decisions), root-only Dirichlet from a hand-rolled seeded Gamma sampler, forced single-action nodes auto-advance (design Open Question resolved), `SearchConfig.observation_profile` must match the evaluator's export) PUCT-MCTS over a determinized world: node = (`current_player_id`, action mask), edge = masked `action_id`, expand = clone + `step`, backup = leaf value. Dirichlet root noise + temperature schedule.
- [x] 2.2 (DONE 2026-07-03, `src/search/evaluator.rs` — batch-shaped `PolicyValueFn` + `UniformEvaluator`; ONNX adapter isolated in `onnx_adapter.rs` so the core never touches ort) Evaluator trait: `(observation tensor) -> (policy over 2192 masked, value)`; uniform/stub evaluator first.
- [ ] 2.3 Aggregation: PIMC (K independent worlds, sum root visits) and IS-MCTS (per-iteration re-determinization, infoset-keyed tree) behind one config knob.
- [x] 2.4 (DONE 2026-07-03, `tests/search/` 5/5 — DebugRunner-built forced-win found with UniformEvaluator incl. discovering the opponent's punish down the losing line; determinism per seed; mid-game robustness; profile plumbing) Validate on perfect-information unit games (pre-determinized) with a known optimal line; assert search finds it. Steps cross-checked against the DCGO parity oracle.
- [ ] 2.5 No-X-ray-vision guard: assert the search never reads opponent-hidden zone identities outside the materialized world.

## 3. AlphaZero self-play training

- [ ] 3.1 Self-play driver: MCTS-driven games producing `(infoset-obs, search-policy π, outcome z)` records; deck pair sampled per game/match (decide vs BO3 `MatchEnv` parallel).
- [ ] 3.2 Policy+value network + training loop on the `(obs, π, z)` buffer (Python side, `code/digimon_gym/`); reuse replay-buffer / loop skeleton patterns from AlphaZero-General-style references.
- [ ] 3.3 Per-generation evaluation: anchored frame (seat-balanced vs greedy + frozen champions), Elo ladder, `exploiter.py` exploitability. Promote only on the anchored frame (rule 30); never on an in-run / self-play win rate.
- [ ] 3.4 Throughput spike: measure sims/sec on a real deck; size a generation's wall-clock before scaling. (Bounded from above by the 0.1 pre-gate proxy — re-measure here with real fork + real evaluator.)

## 4. Productionize the evaluator

- [ ] 4.1 Embedded ONNX evaluator in Rust (`ort`, mirroring `code/src-tauri/src/inference_state.rs`) with leaf batching + virtual loss.
- [ ] 4.2 (Profiled) structural-sharing / COW clone if clone cost dominates (depends on `make-engine-cloneable` D5).

## 5. Game-review annotations

- [ ] 5.1 Recording-format extension: per-decision annotation fields (grade, played-value, best-value, best-action, prior). (Schema lands with 0.5, which populates the value fields pre-search; grades fill in here.)
- [ ] 5.2 Annotator: per recorded decision, search at that node and compute win-probability delta played-vs-best.
- [ ] 5.3 Decision-time (belief-aware over K sampled worlds) + hindsight (true recorded world) grades; brilliant/great/best/inaccuracy/mistake/blunder mapping incl. the brilliant heuristic (best + low prior + only-winning-line).
- [ ] 5.4 Surface annotations in the replay viewer.

## 6. Opponent deck prior (PvP review; parallel, non-blocking)

- [ ] 6.1 `DeckPrior` representation (decklist point estimate vs archetype distribution vs per-card inclusion probabilities) — resolve the design Open Question.
- [ ] 6.2 Deck classifier: infer the prior from revealed cards + `data/deck_library.json` meta + color/turn signals; training labels from the recording corpus' eventual full decklists.
- [ ] 6.3 Wire the inferred prior into the materializer for PvP recordings; enable fair decision-time review grades.
- [ ] 6.4 Surface prior uncertainty in the review UI.

## 7. Validation

- [ ] 7.1 `openspec validate add-determinized-search --strict` passes.
- [ ] 7.2 Determinization round-trip + sampler-invariant tests green.
- [ ] 7.3 Search finds the known optimal line on perfect-information unit games.
- [ ] 7.4 A self-play generation beats the PPO baseline on the anchored frame and shows ≤ baseline exploitability at equal exploiter budget (documented, not asserted as Nash).
- [ ] 7.5 Review grades reproduce on a held-out set of recorded games (decision-time + hindsight).

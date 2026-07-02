# Tasks

Phased; each phase independently testable. **Gate analysis REVISED 2026-07-01** (see design "Readiness & sequencing"): `Game: Clone` already exists and the whole DSL selection surface is on the resumable VM — the real gate for the *search* phases (2–5) is `make-engine-cloneable` task 6.1's **non-DSL cutover** (combat interrupts, Overclock, TriggerOrder, replacement-accept, Delay, play-order, pay-cost cohort), because rollouts step through those and a closure-based park clones to a panic-stub. Phases 0 and 1 and the deck-prior phase are **clone-independent** and can start now.

## 0. Clone-independent pre-work (can start NOW)

- [ ] 0.1 Throughput go/no-go spike: raw `step` throughput + reset-replay cost (upper bound on fork cost) on a real deck pair; A/B the `effects_for_card` memoization lever. Combined with 0.3's evals/sec, price "sims per decision per second" before committing to Phase 3 scale.
- [ ] 0.2 Value-head ONNX export: extend `code/tools/export_onnx.py` (currently exports policy logits ONLY, both MLP + LSTM wrappers) to emit the SB3 value head alongside. Unblocks 0.5 and every future leaf evaluator.
- [ ] 0.3 Batched ONNX evaluator in Rust (`ort`, mirroring `src-tauri/inference_state.rs`): batch-of-observations → (masked policy, value); benchmark standalone on synthetic batches.
- [ ] 0.4 Pre-register robustness baselines: `exploiter_cli.py` (fixed budget) + `anchored_eval_cli.py` against current champions, recorded before any search agent exists (D6 discipline).
- [ ] 0.5 Review v0 — win-probability bar, no search: replay a recording's true trajectory (`runners/replay.rs` already derives opponent decklists + reveal streams for opaque PvP) through the 0.2 value head; emit the per-decision win-prob series using the 5.1 annotation fields. Ships user-visible value pre-gate.

## 1. Game-state determinization (clone-independent EXCEPT search-time use; see design Readiness Correction 2)

- [ ] 1.1 `Infoset` data type: public state verbatim + viewer own-hand (exact) + own hidden model as a **joint unseen pool (deck ∪ face-down security) with a count partition** (own security is hidden from its owner too — see design D2 sharpening) + opponent `HiddenModel` (per-zone counts, pinned cards, `DeckPrior`).
- [ ] 1.2 `Infoset` extractor = inverse of `server/state_filter.py`; split security into known `face_up_security` vs hidden face-down; compute "pinned" opponent cards proven to be in a hidden zone.
- [ ] 1.3 `SamplingRevealSource` implementing `opaque_deck::RevealSource` over a multiset + seeded `StdRng`.
- [ ] 1.4 World materializer: honor pins, shuffle known multisets into concrete order, draw opponent hidden cards from the `DeckPrior` respecting per-zone counts + copy limits + already-seen exclusions; produce a concrete cloneable `Game` with `reveal_source = None`.
- [ ] 1.5 Sampler invariant checker (counts balance, ≤copy-limit, no card in two zones; reconcile BOTH `OpaqueDeckState` books — `restore()` never errors and placeholder accounting desyncs `total_remaining` from the per-card multiset by design) + round-trip property test (re-extract infoset from a sampled world == source infoset for the viewer).
- [ ] 1.6 Self-play regime only here (exact `DeckPrior` from the training pool); PvP prior deferred to Phase 6.

## 2. Neural MCTS search core (gated on the make-engine-cloneable 6.1 non-DSL cutover — rollouts panic at closure-based interrupts until then)

- [ ] 2.1 PUCT-MCTS over a determinized world: node = (`current_player_id`, action mask), edge = masked `action_id`, expand = clone + `step`, backup = leaf value. Dirichlet root noise + temperature schedule.
- [ ] 2.2 Evaluator trait: `(observation tensor) -> (policy over 2192 masked, value)`; uniform/stub evaluator first.
- [ ] 2.3 Aggregation: PIMC (K independent worlds, sum root visits) and IS-MCTS (per-iteration re-determinization, infoset-keyed tree) behind one config knob.
- [ ] 2.4 Validate on perfect-information unit games (pre-determinized) with a known optimal line; assert search finds it. Steps cross-checked against the DCGO parity oracle.
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

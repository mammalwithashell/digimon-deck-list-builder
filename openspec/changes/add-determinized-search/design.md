## Context

This change adds test-time search and self-play training on top of a cloneable engine. It is the realization of the "robustness & equilibrium methods (horizon)" section of `docs/MODEL_EVALUATION.md` §6, narrowed to the **tractable** member of that family: determinized / Information-Set MCTS with a neural policy+value guide (the imperfect-information dialect of AlphaZero).

Two engine facts make it feasible without new hidden-information plumbing:

1. **`opaque_deck::RevealSource`** is a trait the engine already consults *"whenever it would draw, mill, or pop security from an opaque pile"*, and its doc lists *"a sampler over the multiset (for RL inference)"* as an intended implementation. `OpaqueDeckState` (the per-player composition multiset) is `Clone`. This is the world-sampling seam.
2. **`server/state_filter.py`** already computes, for PvP, exactly what a player may observe of the opponent (own hand visible; opponent hand/security redacted to counts; board/trash/memory public). That redaction *is* the infoset boundary; determinization is its inverse.

The hard prerequisite is `make-engine-cloneable`. Prior exploration confirmed Tier-0 carries **no representability risk**: across all ~50 `Box<dyn FnOnce>` selection-callback sites and the 7 recursive multi-pick trampolines, every capture is `Copy`/`Clone` data, every filter is a `CompiledPredicate`, and every continuation is either already data (`dsl_outer_tail`, `parked_replacement`, …) or a nestable frame. The one `raw_rust` effect is governed by rule 28's clone-safety constraint. So this change can assume a faithful, cheap `Game::clone()`.

## Goals / Non-Goals

**Goals:**
- Materially stronger play than the PPO baseline, verified by anchored eval (seat-balanced vs greedy + frozen champions) and Elo ladder.
- A determinization layer that never leaks hidden information into the search (the no-X-ray-vision invariant).
- A measurable robustness improvement, quantified by `exploiter.py` (lower exploitability than the PPO baseline at equal exploiter budget).
- A chess-style review tool that annotates recorded games with per-move quality.

**Non-Goals:**
- Certified Nash / low-exploitability via Deep CFR / ReBeL / Player of Games (still the horizon; the 50-card belief space may make the full versions infeasible). This change measures robustness, it does not certify it.
- Changing card authoring, the engine's rules, or the existing PPO pipeline (PPO stays as baseline + exploiter).
- Real-time search inside live PvP / desktop play (latency budget out of scope; search is for training + offline review first).
- Solving the PvP opponent-deck-inference problem to high accuracy (a v1 meta-classifier suffices; the review's hindsight grade needs no prior).

## Decisions

**D1 — Determinize-then-materialize, root-level (PIMC) by default; IS-MCTS as an option.** Sample K worlds at the search root, each a concrete fully-known `Game` (no live `RevealSource`); run independent PUCT searches; sum root visit counts. Within a materialized world, draws/security are deterministic pops off committed piles, so chance never appears as a tree node — it is "frozen" by world choice. IS-MCTS (re-determinize per iteration, one tree keyed by infoset) is a configurable alternative that reduces strategy fusion at higher cost. Alternative (explicit chance nodes in one tree over the true state) rejected: it either leaks hidden info or explodes the branching factor with 50-card draws.

**D2 — `Infoset` is the inverse of `state_filter`.** Extract a viewer-relative `Infoset` carrying: the public state verbatim; the viewer's own hand (exact) and own deck/security as *composition multisets with unknown order*; and a `HiddenModel` for the opponent = per-zone counts + "pinned" cards proven to be in a hidden zone + a `DeckPrior` over the rest. The materializer honors pins, shuffles known multisets into concrete order, and draws opponent hidden cards from the prior respecting per-zone counts, copy limits, and already-seen exclusions. Alternative (search directly on the true `Game` and just "don't look") rejected: the engine resolves opponent-hidden cards during rollouts (draws, security flips), so the hidden identities must be committed up front.

**D3 — Search node = decision point, edge = masked action id.** The engine already surfaces every choice as a `step(u16)` gated by `pending_selection` + the 2192 action mask, so the tree is naturally over decision points (including mid-effect selections), not turns. The policy head is a masked distribution over 2192 actions; the value head predicts the game outcome for the player to move. No board-symmetry data augmentation exists (unlike Go) — drop it.

**D4 — NN evaluation: prototype in Python, productionize with ONNX-in-Rust.** Stage 1 prototype: search core callable from Python, NN eval in torch (simplest, slowest, FFI per node). Stage 2 production: search in Rust with the evaluator running embedded ONNX (`ort`, mirroring `code/src-tauri/src/inference_state.rs`) so self-play throughput stays in-process with leaf batching + virtual loss. Alternative (Rust search + batched callback into Python) kept as a fallback only if ONNX batching is insufficient.

**D5 — Two review grades; only one needs a deck prior.** The *decision-time* grade is belief-aware: average the value of the played move vs the search-best move over the K sampled worlds — the fair "given what you knew" grade, which requires a `DeckPrior`. The *hindsight* grade uses the single true recorded world (fully known post-hoc) — free, no prior. Brilliant = search-best AND low NN prior (non-obvious) AND only-winning-line; blunder/mistake/inaccuracy = win-probability-drop thresholds. Recording format gains per-decision annotation fields.

**D6 — Robustness is measured, not claimed.** Every agent produced here is run through `exploiter.py` (forward-only best-response PPO) and the anchored frame. We report exploitability as a lower bound and never assert Nash. Strategy fusion (acting differently in indistinguishable worlds) and belief staleness (ignoring info leaked by the opponent's past actions) are documented limitations of PIMC; IS-MCTS partially mitigates the former, policy-conditioned belief updating (horizon) mitigates the latter.

**D7 — Self-play opponent-deck regime is the easy case; PvP review is the hard case.** In self-play both decks are drawn from the known training pool, so the `DeckPrior` is the exact decklist multiset minus revealed cards — determinization is clean and AlphaZero self-play is straightforward. PvP review must infer the opponent's decklist; this is isolated in the `opponent-deck-prior` capability and does not block the search/self-play/training capabilities.

## Risks / Trade-offs

- **Strategy fusion / belief staleness** → can be exploited; mitigated by IS-MCTS option and measured by `exploiter.py`. Honest framing in all docs: stronger + measurably-more-robust, not certified.
- **Throughput** → MCTS is far costlier than PPO per game; mitigated by cheap clone (structural sharing from `make-engine-cloneable` D5), ONNX leaf batching, playout/visit caps. Risk that wall-clock makes large self-play impractical on available hardware — de-risk early with a throughput spike (sims/sec on a real deck).
- **World-sampler correctness (the novel piece)** → a buggy sampler that deals impossible worlds (over-copied cards, cards in two zones) corrupts search silently. Mitigated by a sampler invariant-checker (counts balance, copy limits, no double-placement) and property tests that re-extract the infoset from a sampled world and assert it matches the source.
- **Search-engine faithfulness** → MCTS clones and steps the real engine, so it inherits engine correctness; checkable by running search on fully-known (already-determinized) unit games where the optimal line is known, and by the DCGO parity oracle on the underlying steps.
- **Deck-prior accuracy (PvP review)** → a wrong prior makes the decision-time grade unfair; mitigated by leaning on the hindsight grade where the prior is irrelevant, and by surfacing prior uncertainty in the review UI.
- **Scope** → four-to-five capabilities is a program; mitigated by phasing (determinize → search → self-play → review, with deck-prior parallel) and the option to split into separate changes.

## Phasing / Migration Plan

1. **Determinization** (gated on `make-engine-cloneable`): `Infoset` extraction + `SamplingRevealSource` + world materializer + invariant checker + round-trip property tests. Self-play regime only (exact deck prior).
2. **Search core**: PUCT-MCTS over a determinized world with a stub/uniform evaluator; validate on perfect-information unit games. Add the NN evaluator (Python prototype).
3. **Self-play training**: self-play driver → `(obs, π, z)` buffer → train policy+value → iterate; anchored-eval + Elo + exploiter at each generation. Promote only on the anchored frame (rule 30).
4. **Productionize**: ONNX-in-Rust evaluator + batching for throughput; structural-sharing clone if profiling demands.
5. **Review tool**: recording-format annotation fields; decision-time (self-play prior) + hindsight grades; UI surfacing.
6. **PvP review**: `opponent-deck-prior` classifier; enable fair decision-time grades for real PvP recordings.

## Readiness & sequencing

This lane cannot start its own code until `make-engine-cloneable` lands (`Game: Clone`). Beyond that hard gate, three readiness items shape the start:

- **DSL consolidations have LANDED (archived 2026-06-20); Tier 0 remains the gate.** `make-engine-cloneable` was sequenced after the in-flight DSL changes (`collapse-dsl-step-idioms`, `unify-dsl-scalar-and-comparators`, `fix-dsl-substrate-rot-and-bugs`); all three are now merged, so that transitive prerequisite is satisfied. (Note: those changes did not deliver the Tier-0 simplifications once hoped for — budget verbs unmerged, `raw_rust` at 4 not 0 — so Tier-0 scope is unchanged; see `make-engine-cloneable` task 0.1.) `Game: Clone` is still the hard gate for this lane.
- **Throughput feasibility is a go/no-go for the *search* payoff, measurable early.** MCTS needs hundreds of NN-evaluated sims per decision over long games. A proxy — raw `step` throughput + reset-replay cost as an upper bound on clone cost — is measurable *before* Tier 0 lands and bounds whether self-play MCTS is viable at the intended scale. Tier 0 is justified independently (save/load, session persistence, snapshot back-step), so a weak throughput number does not kill Tier 0 — only this lane's self-play ambition.
- **First shippable slice = review on the hindsight grade, on the existing PPO value head.** The earliest user-visible payoff needs the *least*: `Game: Clone` + determinization (Phase 1) + a value estimate, but NOT self-play training (Phase 3), NOT a trained AlphaZero net, and NOT the deck classifier (Phase 6). Reuse the existing PPO value head as the v0 evaluator and the recording's `GameStart`-captured true decklist for the hindsight world. Sequence this ahead of self-play so the lane delivers value before its most expensive phase.

## Open Questions

- **Deck prior / classifier (the active exploration thread):** representation (point estimate decklist vs distribution over archetypes vs per-card inclusion probabilities), inputs (revealed cards + meta `deck_library.json` + turn/color signals), update cadence (per-reveal Bayesian update vs one classification at game end for review), and where training labels come from (the recording corpus has eventual full decklists). How wrong can the prior be before the decision-time grade misleads?
- **PIMC vs IS-MCTS as the default** — does strategy fusion measurably hurt anchored-eval / exploitability enough to justify IS-MCTS's cost? Decide empirically in Phase 3.
- **Search granularity** — mid-effect `pending_selection` nodes make trees deep; do we search every decision point or collapse forced/near-forced ones? Interaction with the long effect-resolution chains.
- **Self-play deck sampling** — fixed deck pair per game (parallels BO3 `MatchEnv`) vs sampled per game; how the deck pool interacts with search-policy targets.
- **Throughput target** — what sims/sec on what hardware makes a generation of self-play complete in a tolerable wall-clock? Spike before committing to Phase 3 scale.
- **Could this graft onto PPO first?** Use the existing PPO policy/value as MCTS priors and add search at decision time *before* committing to full self-play retraining (a cheaper intermediate that still requires only Phases 1–2).

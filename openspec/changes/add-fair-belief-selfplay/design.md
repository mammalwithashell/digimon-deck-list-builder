## Context

Baseline inspected: remote main `3b6a2d8f4` (2026-10-22). `DeckPrior` currently has only `Exact`; extraction derives the opponent unseen multiset from the actual hand/deck/face-down security. `materialize` re-deals those physical cards. This is sound for a known decklist but leaks composition in closed-list play. Existing no-X-ray tests preserve the hidden multiset, so they cannot catch this kind of advantage. PIMC and IS-MCTS, self-play shards, the AZ trainer, generation runner, and exploiters already exist.

The default engine observation includes the observer's original list, not the opponent's. Consequently the immediate problem is privileged search targets, not evidence that the policy tensor directly exposes the opponent list. Policy-only robustness and search-assisted robustness are different outcomes and must be measured separately.

The active `add-determinized-search/specs/opponent-deck-prior/spec.md` currently requires exact priors for all self-play. Implementation of this change must amend that active requirement and its design/tasks to distinguish regimes; no duplicate owner of Phase 6 is intended.

## Goals / Non-Goals

**Goals:** fair closed-list search supervision; current public copy counting and remembered legitimate information; legal and diverse hypotheses for unfamiliar lists; demonstrably stronger policy-only deployment against a named held-out population; reproducible promotion evidence.

**Non-Goals:** changing game rules, action masks, action-space size, or active tensor layouts; certifying Nash equilibrium; implementing a replay review UI; immediately learning a sophisticated action-conditioned Bayesian opponent model; assuming public trash implies public knowledge of a submitted decklist.

## Decisions

### 1. Make the information regime explicit

Add `closed_decklist` and `open_decklist` to self-play, evaluation, shard/run/model provenance, and resume validation. New robustness runs explicitly select closed mode; legacy artifacts without a tag are treated as legacy exact-prior data, never relabeled fair. Open mode preserves current behavior for legitimate open-list events and controlled comparisons. Mixing regimes requires explicit recorded configuration and separate reporting. Keep checkpoint progress distinct from promotion status.

### 2. Build beliefs from viewer knowledge, not a privileged Game

Introduce a viewer-scoped knowledge input: own list/hand, public zone state and counts, format policy, plus entitled reveal/peek events. Construct it at the engine boundary. The belief provider receives neither the opponent's submitted list nor hidden card identities. Prefer a typed restricted input over a convention telling callers not to inspect `Game`.

Track current locations of known cards and invalidate positional knowledge on shuffle or hidden randomization. Moving a public card back to hand preserves the identity fact when rules allow tracking it, but not an unknowable slot. Repeated observation of the same recycled card is not evidence of another physical copy. Distinguish current public accounting from historical sightings. Each seat owns its private knowledge; self-play driver omniscience is not shared between seats. Snapshot/resume retains knowledge deterministically. Legacy replays without sufficient history are marked degraded, never reconstructed using hidden truth.

Knowledge schema 2 also separates anonymous physical movement references from observed identities. A concealed security-to-deck-bottom move has a public destination, but neither player learns its printed identity merely from the engine's native handle. Sample that identity at the retained destination; preserve the reference through subsequent determined movements. For the owner, sample from the entitled unseen deck/security joint multiset and replace unobserved native aliases. For the opponent, use the hypothesis pool. Reserve exact positions before membership-only observations. Cached printed-card metadata still requires an identified source; a sampleable anonymous event reference does not authorize copying identity-derived caches. Concealed in-flight security destinations without a complete allocator remain explicit unsupported states.

### 3. Start with a weighted legal candidate mixture and a constrained fallback

Freeze a training prior snapshot with card population, candidate list hashes, weights, legality region/date/format, restriction revision, and source provenance. Initial weights describe the declared population, not inferred certainty. Deduplicate lists and avoid weighting one list repeatedly merely because several sources copied it. Filter candidates through existing deck legality, including default and card-specific copy limits, restrictions and combinations, eggs, deck size, and implemented-pool admission. An unsatisfiable format/pool is a configuration error, not permission to relax constraints.

Condition candidates on evidence that must be physically consistent with their composition and current knowledge. Hard-exclude impossible lists; v1 need not infer likelihood from the opponent declining a play. Sample a candidate according to normalized weights, subtract current accounted-for cards, then sample remaining hidden assignments without replacement.

Reserve configurable positive mass for unseen legal lists even while candidates remain compatible. The fallback uses only the frozen legal card population, public facts and own knowledge; sample constrained legal list completions with disclosed smoothing weights. Bound attempts and report exhaustion explicitly. Never restore a candidate contradicted by evidence, silently relax legality, consult the real list, or emit a zero-information uniform action target on failure. A failed generation job is recorded for diagnosis. Report fallback mass/use, candidate count, entropy, exclusions, and failures; entropy is diagnostic, not proof of calibration. A learned classifier is a later replacement behind this interface.

### 4. Construct hypothetical cards instead of shuffling the true hidden pool

The closed-mode materializer accepts the public snapshot, viewer knowledge, and sampled list/composition. Preserve public/known physical instances and resume data; instantiate hypothetical hidden `CardSource`s with coherent data indices and unique instance IDs. Avoid hidden original-deck metadata, RNG, cached hand filters, or continuations that carry true identities into the world. The hypothetical opponent `original_deck` must match the sampled list so later own-deck observations within that world are coherent. Original true-list data remains available only to the isolated game runner and post-game evaluation scorer.

World invariants compare against the sampled hypothesis plus viewer evidence, not against the true opponent deck. Check counts, conservation, pins, legality, public-state equality, root legal mask, and playable continuation. Root search remains at the viewer's decision. Audit continuation/prompt references before supporting each installation path; rebuild derived hidden-zone state from hypothetical cards. Unsupported states fail explicitly with context and cannot silently drop training examples. Reuse PIMC/IS-MCTS interfaces and retain their documented strategy-fusion limitations; this change is not an equilibrium solver.

### 5. Keep fair search targets usable by the existing policy

Both seats search from their own knowledge. Shards retain the current observation, sparse visit distribution and game outcome shapes. Belief diagnostics live in metadata/sidecars, not additional neural input. Audit whether finite observation windows truncate relevant trash/count facts or whether the chosen model can remember entitled history. Report that limitation; if a contract extension is necessary, create a separate synchronized tensor proposal instead of slipping it into this work. Include recurrent reset/state handling in this audit. Search-only gains do not demonstrate policy learning.

### 6. Measure the deployment we want

Before experiments, freeze a protocol with deck/variant splits, seeds, sample sizes, budgets, metrics, confidence procedure and thresholds. Compare an unchanged seed, exact-prior-trained baseline, and closed-prior-trained candidate under identical fair deployment inputs. Include policy-only and equal-budget search-assisted panels separately. Hold out entire decklists and variants across prior construction, training and tuning; add archetype holdouts where the implemented pool supports them and label narrower scopes honestly.

Use seat-balanced greedy/frozen champion anchors, legal unseen variants, rare tech/restriction-sensitive lists, and late-game counting/recovery probes. Score beliefs after the game using true hidden information in an isolated scorer (support coverage and proper probability scores); never feed those scores' truth back into action selection. Test with multiple training seeds.

Train fresh equal-budget exploiters against frozen candidates and baselines, using the same fair information regime and held-out evaluation games. Report best-response win rate as a budget-limited lower bound on exploitability, with uncertainty; inconclusive differences cannot establish robustness. Promotion requires the preregistered held-out policy-only improvement and anchored non-regression criteria plus exploiter non-regression, not mirror win rate. Failed, missing, or inconclusive required evidence blocks promotion while preserving resumable checkpoints. Thresholds are committed before compute; their numerical values are an experiment-design task, not an unsupported claim in this proposal.

## Risks / Trade-offs

- Candidate bias and implausible fallback lists → held-out splits, positive unseen mass, legality and coverage diagnostics; do not equate a legal list with a plausible list.
- Hidden data embedded in clones/continuations → composition-changing equivalence tests, typed inputs, instance/metadata audits, and continuation fixtures.
- Public history lost or stale after movement → engine event ledger with explicit forgetting semantics and recycled-card tests.
- Existing tensor/model cannot express the teacher's knowledge → policy-only probes and a separate contract proposal if needed.
- Added sampling cost → measure acceptance rate, wall time, decisions/sec and equal-budget performance before scaling; do not assume batching fixes prior costs.
- Search optimizes sampled opponents too strongly → retain PIMC/IS-MCTS caveats and measure with fresh exploiters; no robustness certificate.

## Migration Plan

1. Reconcile the parent search proposal and freeze regime/knowledge/legality APIs and protocol metadata.
2. Implement knowledge and priors with failing invariance/legality/counting tests first.
3. Add closed-world construction and engine continuation guards; retain open mode unchanged.
4. Wire self-play/runner/shard/resume provenance and complete small multi-generation smoke runs.
5. Run the preregistered held-out and exploiter comparison; promote only if gates pass.

Rollback selects explicit open mode and its compatible artifacts. Never reinterpret closed-mode shards as exact or vice versa. This proposal itself changes no runtime behavior.

## Event-context liveness

Broad-cohort continuation support also requires event-context liveness. A queued
security-removal observer can outlive the membership of an anonymous card added
to hand. The DSL compiler certifies a deliberately small set of clauses whose
gates, selections and bodies do not read event-card identity; unaudited forms
and raw Rust default to strict. Hypothetical worlds may omit only unused
unresolved event-card references in queued and parked optional-trigger contexts,
including unused attribution and moved-card payloads. They retain timing,
owner/cause scalars and entitled references, preserve the source game and reject
unresolved cached identity snapshots. Tests cover source-composition changes, actual continuation
choices and rejection of identity-dependent gates/bindings.

## Open Questions

- Which exact legal format snapshot, candidate population and held-out archetypes cover the first experiment? Resolve before producing training data.
- What positive fallback mass and constrained completion sampler meet coverage/throughput targets? Select using training/validation only, then freeze.
- What minimum held-out uplift, anchor/exploiter non-regression margins and confidence/sample budgets are affordable? Preregister before runs.
- Can the existing policy architecture retain sufficient counting/history information without a tensor change? Measure before making a deployment-readiness claim.

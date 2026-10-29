## 1. Contracts and experiment preparation

- [x] 1.1 Reconcile `add-determinized-search` proposal/design/tasks and its active opponent-deck-prior requirement: exact priors are open-list mode; this change owns closed-list prior/training implementation, while replay UI remains separate.
- [x] 1.2 Define versioned regime, restricted viewer-knowledge, belief-provider and world-builder APIs; audit current tensor fields, history truncation, recurrent state, and hidden data in clone metadata/continuations.
- [x] 1.3 Freeze the initial format/region/date restriction snapshot and implemented card population; identify existing legality services and add missing format-policy plumbing without duplicating legality rules.
- [x] 1.4 Commit candidate source/dedup/weight policy and training/validation/test splits; reserve held-out lists/variants and name the exact covered archetype population.
- [x] 1.5 Preregister seeds, game/sample and exploiter budgets, confidence methods, policy-only uplift threshold, anchor and exploiter non-regression margins, and equal-budget search comparisons before generating experiment data.

## 2. Viewer knowledge and legal beliefs

- [x] 2.1 Write failing per-seat knowledge tests for public zones, private peeks, return-to-hand, shuffle invalidation, recycled sightings, face-up security, and deterministic snapshot/resume.
- [x] 2.2 Implement knowledge event/state lifecycle and restricted belief inputs; mark insufficient legacy history explicitly rather than reconstructing from hidden truth.
- [x] 2.3 Write failing candidate tests for copy exhaustion, restricted and combination-banned cards, card-specific copy exceptions, egg/main separation, impossible evidence, and legal but unfamiliar variants.
- [x] 2.4 Implement frozen weighted candidate loading, existing legality validation, hard evidence conditioning, and current public-card subtraction with diagnostic hashes and counts.
- [x] 2.5 Implement positive unseen-list mass and bounded constrained legal completion fallback; test all-candidates-eliminated, impossible format/pool, normalization, deterministic seeds, and explicit exhaustion without hidden-truth access.

## 3. Composition-independent world construction

- [x] 3.1 Add failing paired-game guards with identical viewer knowledge/root masks but different concealed deck compositions; compare beliefs and sampled worlds under fixed seeds, beyond existing hidden-permutation tests.
- [x] 3.2 Build hypothetical hidden cards and sampled original-deck metadata from restricted input; preserve public instances, counts, pins, own knowledge, unique IDs, and data-index coherence.
- [x] 3.3 Audit/rebuild hidden-dependent caches, prompt indices and continuation state; add mid-effect continuation fixtures and explicit unsupported-state failures with diagnostics.
- [x] 3.4 Add hypothesis-based invariants for legal composition, counts, conservation, root masks, revealed facts and playable continuations; demonstrate resolution of a sampled legal card absent from the true list.
- [x] 3.5 Wire closed-world construction into both PIMC and IS-MCTS and extend paired-game guards through search outputs; retain open-mode regression fixtures.

## 4. Training integration and provenance

- [x] 4.1 Add explicit regime/prior configuration to Rust CLI, both-seat self-play and generation runner; verify the private game runner never supplies privileged data to closed search.
- [x] 4.2 Tag shards, run state, exported models and diagnostics with regime, prior/legality/split hashes, knowledge schema, seeds and search budgets; test incompatible resume rejection and explicit mixed-buffer policy.
- [x] 4.3 Keep existing sparse policy/outcome and tensor/action contracts intact; audit policy inputs and recurrent resets and add policy-only counting/history probes.
- [x] 4.4 Run small multi-generation end-to-end jobs including cold resume, unfamiliar-list fallback, and failed sampler handling; verify checkpoint completion never implies promotion.
- [x] 4.5 Benchmark candidate conditioning, fallback acceptance/failure rate, decisions/sec, wall time and memory at declared search budgets; record scaling limits before large runs.

## 5. Independent robustness evidence

- [x] 5.1 Add frozen held-out evaluation panels for seed, exact-prior-trained baseline and closed-prior-trained candidate; separate policy-only from equal-budget search-assisted play and balance seats/seeds.
- [x] 5.2 Implement long-game exhaustion, recovery/recycling, rare-tech and restriction-sensitive scenario probes; identify observation/memory failures separately from sampler failures.
- [x] 5.3 Add isolated post-game belief scoring for true-state support coverage and proper probability scores, fallback use and calibration summaries; test scorer truth cannot enter action selection.
- [x] 5.4 Run fresh budget-matched fair-information exploiters against frozen baselines/candidates; evaluate on held-out games and report uncertainty and lower-bound scope.
- [x] 5.5 Execute the preregistered multi-seed comparison and report exact deck/game denominators, truncated/failed games and all required metrics without dropping failures from aggregate evidence.
- [x] 5.6 Enforce promotion gates for held-out policy-only uplift, anchor non-regression and exploiter non-regression; test missing, skipped, failed and inconclusive evidence blocks promotion but preserves resumability.

## 6. Documentation and final validation

- [x] 6.1 Update training runbook, model-evaluation guidance and model metadata documentation with regime selection, legacy-data treatment, prior snapshots, reproduction commands, limitations and rollback.
- [x] 6.2 Run focused Rust determinization/search/self-play/knowledge and Python trainer/runner/exploiter checks with an isolated Cargo target, then the applicable repository verification ladder; preserve aggregate failures and state their scope.
- [x] 6.3 Run `openspec validate add-fair-belief-selfplay --strict` and verify coordinated active search specs are consistent; attach the final evidence report without claiming certified low exploitability.

## Progress evidence

All 29 implementation and evidence tasks are complete. See `evidence-report.md`
and the committed `evidence/` JSON for the full local three-seed comparison,
validation, uncertainty, artifact hashes and explicit population limits.

The comparison completed 384 training games, 6,144 baseline/candidate panel
games and 768 unchanged-initializer games, with no failed/truncated evaluation
games. Six fresh 2048-step exploiters completed held-out evaluation. The
candidate regressed in policy-only play and exploiter resistance; promotion
failed and no champion was promoted. Completing this change does not certify
stronger policies or low exploitability.

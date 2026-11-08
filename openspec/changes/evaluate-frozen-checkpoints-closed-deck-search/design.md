## Context

On 2026-10-29, fetched `origin/main` is `77f4702f9` (PR #757); this checkout contains it at merge `6a4eb5f56`. The ongoing `runs/ex13-model-evaluation-20261029` campaign is frozen to older engine `d0d0520262c6d8a4df0216ca6b5743052fe24789` plus its diagnostic harness. It must finish on its original inputs.

Current [closed-search qualification](../add-fair-belief-selfplay/cloud-search-qualification.md) records 127 completed probes: 63 training, 23 source-held-out, 36 supplemental and five pet probes, with no failures/truncations. These low-budget uniform-evaluator checks supersede the original 1/21 smoke, but do not prove exhaustive continuation coverage or model strength. Explicit unsupported states remain. The current two-model `checkpoint_search_eval.rs` invokes exact-list search and needs a separately frozen closed-mode implementation.

The four existing ONNX models remain fixed; their hashes are in `runs/ex13-model-evaluation-20261029/campaign-plan.json`. These experiments evaluate inference-time belief search on existing weights, not models trained with the new belief pipeline. The independent broader belief-training experiment documented in the qualification is a separate workload and must be included in capacity checks.

## Goals / Non-Goals

**Goals:** Determine checkpoint rankings under uncertain opponent composition; measure exact-list information advantage within each checkpoint; test generalization when the actual list is absent from the prior; identify failures in belief coverage and search cost.

**Non-Goals:** Retraining, champion promotion, repairing rules in a scored experiment, automatically resuming failed studies, claiming human strength, or replacing the running campaign. This turn creates a plan only.

## Decisions

### 1. Same-runtime paired controls

Build one new frozen runtime from current main plus the reviewed two-model diagnostic adapter. Use it for both modes, with identical card data, model hashes, action/observation layouts, search implementation and model-routing convention. Gate model metadata compatibility before gameplay; never reshape or relabel old weights. Preserve the old campaign as historical evidence rather than subtracting its win rates from new closed-mode rates: that would confound engine and information changes.

Use 320 total simulations, four worlds, batch four, every non-forced decision, zero root noise, and the existing deterministic selection/plan execution convention. Assert actual simulation counts and log exact dispatched legal actions. Equal simulation budgets are the primary comparison; record belief-construction and total decision cost because equal budgets are not equal wall time.

### 2. Honest information boundary and prior

Closed actors receive their own legitimate deck knowledge and private observations, public game history, and a frozen `WeightedDeckPrior` derived only from the 21 distinct training lists. Start with equal mass per distinct list, matching the existing prepared prior; the 26 sampling slots are not 26 prior candidates. This is a declared prior choice, not a claim of calibrated tournament frequencies. No oracle archetype label, opponent list ID, true hidden composition, held-out decklist, future observation or truth-derived cache enters the actor. Public revelations may update beliefs. Keep per-seat knowledge separate, including remembered private information and shuffle invalidation. Reuse production extraction/materialization rather than building a parallel sampler.

Use the admitted population fallback already defined by prior preparation, fixed before test data is scored. Log posterior support/effective sample size, contradictions and fallback usage; never silently fall back to exact-list search. The game driver necessarily owns both decks, but the belief provider cannot access that truth. Offline scoring can access truth only after decisions are committed and must never feed it back. A closed actor may legitimately narrow to one list from public evidence; that is different from supplying the list at launch.

### 3. Staged test matrix

All primary schedules use fresh seeds, frozen before any results, with common blocks across paired modes. Record explicit game/search RNG seeds and role/seat/opener mapping in the manifest. Do not reuse the already-inspected position audit as a strength test.

| Stage | Panels | Games/panel | Total | Purpose |
| --- | ---: | ---: | ---: | --- |
| Qualification | Diagnostic only | Bounded and declared | Excluded | Contract, information and runtime gates |
| A: Closed vs closed | All six CP8/12/16/20 pairs | 520 | 3,120 | Ranking with uncertain opponent lists |
| B: Open vs open | Same six pairs, same new runtime and paired blocks | 520 | 3,120 | Engine-matched information-regime controls |
| C: Open vs closed | Each checkpoint against itself, modes swapped across seats/openers | 520 | 2,080 | Within-model exact-list advantage |
| D: Source-held-out closed vs closed | Same six checkpoint pairs | 480 | 2,880 | Generalization beyond prior candidate lists |

Stages A-C total **8,320 scored games**; the separate held-out extension brings the planned total to **11,200**. Stage A is the first deliverable. Stages B/C answer why rankings might differ; neither stage alone identifies a training-regression cause. Stage D uses all 23 source-held-out lists across 12 archetypes, 120 four-game blocks, 40 candidate appearances per archetype with balanced seats/openers. Verify coverage by list as well as archetype; counts need not be equal for every list. Opponent archetype exposure must also balance. Light Hybrid lacks an independent held-out list in this source and must remain explicitly unmeasured there.

Stages A-C use the unchanged 21 lists/13 archetypes and existing four-game block construction: 130 blocks, 40 appearances per archetype, 20 per seat, all lists covered. These lists occur in the prior and are familiar training lists; call this closed-list inference on a known population, not unseen-deck generalization. Both actors in Stage D use held-out decklists while the opponent prior remains training-only. Supplemental and overlapping pet lists remain diagnostic populations outside these totals. Before Stage D verify the held-out partition was not used to fit these weights; previous evaluation exposure must be disclosed, so it is not a fresh blind test.

Independent-seed confirmations, prior-weight ablations, 1280-simulation sweeps and equal-wall-time comparisons are follow-ups, not unbounded additions to the initial compute budget. Select and freeze their protocols before running them; label any selection based on primary results exploratory.

### 4. Qualification before scored execution

Validate all frozen assets, full model contract and admission without modifying source lists. Build isolated Windows and Linux executables/private bindings and hash source, binaries, native dependencies, card data, prior and model metadata. Extend tests to change actual hidden opponent composition while holding entitled information and RNG fixed: evidence, observation, sampled worlds, root plans and selected actions must be invariant. Include pending reveal/cost choices, concealed sources, remembered cards and shuffle cases. Test per-seat isolation and the new harness's own decision path, not only the underlying library.

Run bounded full-budget preflights covering all 21 training lists and all four models, plus selected held-out/fallback states; no scored launch until every required preflight completes without failures/truncation. Confirm exact Windows/Linux action and outcome parity on a frozen reference set covering both regimes, two-model seat swaps and continuation states. The 127 existing probes are useful evidence but do not replace these adapter/full-budget checks. A failed check blocks the affected stage; preserve the failure rather than reducing coverage or raising caps post hoc.

### 5. Reporting and failure semantics

Every completed 520-game panel requires 260 verified two-game receipts; held-out panels require 240. Validate request/output hashes, game completion, legal executed traces, seeds, model/mode assignment, prior hash and balance. Keep games, worlds, decisions and paired blocks as distinct denominators.

Report W/L/D and score `(wins + 0.5 * draws) / games`, four-game-block bootstrap 95% intervals, first/second splits and per-archetype results. Forty games per archetype imply wide uncertainty; use block-aware intervals and display denominators. Compare open/open and closed/closed scores with a paired bootstrap of aligned block differences, not independent interval subtraction. Pairing aligns random starts; subsequent trajectories can differ. Multiple pairwise findings are exploratory; any formal family-wide claim needs a declared multiplicity procedure. Do not pool shared-seed panels as independent evidence or force a transitive champion ranking.

Report decision time (including belief work), action/plan agreement on matched diagnostic positions, posterior/fallback statistics and optional offline card log-loss/Brier/support coverage. These belief metrics are not calibrated game-win probabilities. Preserve raising evolution, promotion, battle evolution and DNA separately with opportunity denominators where measured; pending effect-sequence opportunities remain unmeasured unless implemented explicitly. Role reports describe the same games.

Any rejected, unsupported, crashed, timed-out or truncated game leaves the panel incomplete. Stop its queue, preserve checkpoint hashes, request, seeds, decks, recording, exact executed trace and error. No retry, substitution, draw assignment or complete-panel score. A new corrected runtime requires a new campaign identity; old failures remain visible.

### 6. Resource plan and artifact isolation

Use three independently resumable queues with the existing local/Hetzner split as a starting template, not a resource entitlement. As capacity becomes available, cloud-A can handle pairs involving CP8, cloud-B the remaining closed pairs; assign matched controls to the same host to reduce timing confounds. Local handles within-model mode comparisons and diagnostics. Queue the held-out extension after primary results, retaining its frozen protocol.

Before launching, inspect actual process roots, CPU/memory load and disk on both hosts, including the independent eight-worker belief experiment mentioned in source documentation. Reserve only idle capacity; never stop or alter another job. Preserve the currently frozen evaluation until terminal. Pilot throughput at full budget determines worker counts and ETA; no ETA is justified from the old open-search rate alone. Freeze timeout and decision caps before scored games, using diagnostics to establish feasibility. Each campaign has separate roots, bindings, source bundles, logs and monitoring records. No new automation or service is created by this proposal.

## Risks / Trade-offs

- Prior misspecification -> declare weighting and fallback, report support, retain held-out testing; a known candidate library remains privileged population knowledge.
- Unsupported rare histories -> retain strict errors and exact evidence; software smoke is not exhaustive coverage.
- Engine upgrades -> same-runtime open controls; report historical results separately.
- Runtime contention and belief overhead -> live capacity checks and a bounded throughput pilot before worker/ETA decisions.
- Existing models learned under older regimes -> inference-only conclusions; fair-search retraining is a separate experiment.
- CP16's preserved policy no-progress failure -> do not resume it or add broad policy-only lift panels until separately diagnosed.

## Migration Plan

Implement and test the adapter in an isolated checkout/target, freeze a new manifest, pass qualification, then launch only within an authorized execution step and available resources. Monitor committed receipts, with bounded logs after 30 minutes without progress and respect for the declared worker deadline. Rollback means stopping new launches and preserving evidence, never overwriting the running campaign or automatically restarting failed controllers.

## Open Questions

- What full-budget throughput and memory cost does the new adapter achieve on each host? Resolve in diagnostics, before allocating workers.
- Are the documented independent belief-training processes still active? Verify at launch; a source launch receipt is not current process state.
- Can current receipt/behavior telemetry represent the required per-actor belief fields without large live snapshots? Resolve in implementation tests.

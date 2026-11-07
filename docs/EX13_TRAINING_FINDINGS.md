# EX13 training runs and evaluation findings

Updated November 7, 2026. The latest evaluation evidence completed on November 6. The [review summary](../openspec/changes/evaluate-frozen-checkpoints-closed-deck-search/execution-status.md) and [machine-readable evidence index](../openspec/changes/evaluate-frozen-checkpoints-closed-deck-search/validation.json) record the qualified source, separate runtime cohorts and remaining study gaps.

The fresh EX13 learner completed 20 training generations and improved substantially against fixed policy-only opponents. CP8 subsequently beat later checkpoints in several completed known-decklist search tests. In the latest held-out closed-list search panels, CP20 scored 48.125% against CP12 and 52.708% against CP16 across 480 games each. Both four-game-block 95% intervals include 50%, leaving their ordering uncertain.

Model selection depends on the intended play setting. The sections below retain the policy-only, search, mulligan and runtime findings with their own populations and provenance. The held-out lists had prior evaluation exposure, and Light Hybrid held-out-list coverage remains unmeasured.

## What was trained

This account focuses on the **fresh-history EX13 lineage**, followed by its frozen CP8, CP12, CP16 and CP20 evaluations. Earlier EX13 campaigns are separate experiments, including runs affected by engine failures. Their replay and outcomes are not added to this lineage's totals.

The learner began with fresh weights and a mixed heuristic bootstrap using greedy/development behavior: **416 games and 53,248 rows**. It inherited no previous learner weights or search replay. Historical opponent networks influenced the games it subsequently played; their weights were not copied into the learner.

| Training segment | New search-collection games | Cumulative collection games |
|---|---:|---:|
| Generations 1–4 | 5,200 | 5,200 |
| Generations 5–8 | 5,200 | 10,400 |
| Generations 9–12 | 5,200 | 15,600 |
| Generations 13–16 | 5,200 | 20,800 |
| Generations 17–20 | 5,200 | 26,000 |

Across these segments, the lineage audit verified **6,500 collection chunks and 1,666,678 distinct collected rows**. Bootstrap rows are separate. Replay windows reuse collected rows, so neither repeated fitter input rows nor inherited replay copies count as new games or independent examples.

The recorded collection recipe used 1,300 games per generation, eight workers, four sampled worlds and leaf batches of four. Its configured search budget mixed 320-simulation searches with 64-simulation searches, with a full-search fraction of 0.25. It used a three-generation replay window and a replay cap setting of 1,300 games. This differs from the later scored evaluation protocol, which requires 320 simulations at every nonforced decision.

The initial opponent configuration assigned a 0.5 self-play probability and a fixed historical pool. The continuation segments enabled a three-generation history window. These runs do not isolate the causal effect of historical-opponent mixing: there is no otherwise-identical control establishing that this particular choice caused the gains.

“CP4” needs a qualifier: the **fresh learner's CP4**, the older **parent-checkpoint4 evaluation anchor**, and the older **page-search CP2 training opponent** are different models.

Evidence: [fresh campaign plan](../runs/ex13-fresh-history-rollout/campaign-plan.json), [continuation configuration](../runs/ex13-history-continued-dashboard/snapshot.json), [lineage audit](../runs/ex13-heldout-lineage-audit-20261030/reported-results.txt).

## What the deck pools measure

| Pool | Coverage | Interpretation |
|---|---|---|
| Training | 21 distinct lists, 13 archetypes, 26 sampling slots | Two slots per archetype; slots can repeat the same exact list |
| Combined diagnostic | 24 distinct lists, 13 archetypes | Includes a same-list Light Hybrid fallback |
| Weight-training holdout | 23 distinct lists, 12 archetypes | Exact lists excluded from bootstrap and collection inputs |
| Original pet regression | Five lists, five archetypes | Separate regression view; overlaps the combined diagnostic |

The held-out lists are **held out from weight-training data, not blind evaluation lists**. All 23 had already appeared in training-era diagnostic evaluation and later qualification. Cards and archetypes overlap training, and Light Hybrid has no measured held-out-list coverage. The pet and combined panels must not be pooled as independent evidence.

The [lineage proof](../runs/ex13-heldout-lineage-audit-20261030/completion-verified.json) follows collection shards, replay selections, fitter inputs, warm starts and exported checkpoint hashes. It supports exact-list training separation within the retained provenance; it does not turn repeated evaluation exposure into a blind test.

## Policy-only learning improved against fixed anchors

Policy-only evaluation selects legal actions from the model without tree search. The combined diagnostic supplies **2,600 games per anchor per checkpoint**, with deterministic play and balanced archetype coverage. The percentages below are wins out of 2,600; these selected checkpoints have no draws or incomplete outcomes in that panel.

| Fresh checkpoint | Greedy | Older parent CP4 | Development fixed |
|---|---:|---:|---:|
| CP0 after bootstrap | 67.81% | 22.54% | 38.62% |
| CP4 | 94.08% | 42.04% | 60.92% |
| CP8 | 96.77% | 48.00% | 65.15% |
| CP12 | 97.69% | 51.27% | 67.15% |
| CP16 | 97.65% | 50.38% | 67.81% |
| CP20 | 98.08% | 52.15% | 70.65% |

The large early gains show learning on this benchmark. By the later checkpoints, greedy is close to saturated and contributes little discrimination. Results against learned anchors remain more informative, and improvement is not monotonic against every anchor: CP16 trails CP12 against the older parent.

These are comparisons against fixed opponents, not direct matches between the rows. The same seeds and deck schedules recur across checkpoints. Small changes need paired analysis rather than a ranking based solely on point estimates.

The early behavior analysis also found increased raising evolution, battle evolution and attacking, with fewer direct-play choices. Promotion was already selected at nearly every legal opportunity. That identifies useful decisions to study—such as promote versus wait—but does not prove that the individual choices were good.

Evidence: [CP0–4 analysis](../runs/ex13-fresh-history-analysis/checkpoint-4-analysis.md), [CP8 snapshot](../runs/ex13-history-continued-dashboard/snapshot.json), [CP12 snapshot](../runs/ex13-history-next-dashboard/snapshot.json), [CP16 report](../runs/ex13-history-to16-dashboard/reported-results.txt), [CP20 report](../runs/ex13-history-to20-dashboard/reported-results.txt). Table values are sums of the snapshots' combined-diagnostic outcome rows.

## Search changed the checkpoint ranking

The completed original search campaign used **known exact decklists**, while sampling hidden card placements. Both models received 320 simulations, four worlds, batch size four and zero search noise at every nonforced decision. Each panel contained 520 games: 130 four-game blocks, with 40 candidate appearances per archetype and balanced seats/openers.

Each row below is from the perspective of the first checkpoint. Intervals are the recorded four-game-block 95% intervals, conditional on this fixed pool.

| Matchup | Panel | Wins / losses | Score | 95% interval |
|---|---|---:|---:|---:|
| CP12 vs CP8 | Primary | 218 / 302 | 41.92% | 37.76–46.21% |
| CP12 vs CP8 | Supplemental confirmation | 230 / 290 | 44.23% | 40.02–48.53% |
| CP16 vs CP8 | Primary | 203 / 317 | 39.04% | 34.94–43.30% |
| CP16 vs CP12 | Primary | 224 / 296 | 43.08% | 38.89–47.37% |
| CP20 vs CP8 | Primary | 221 / 299 | 42.50% | 38.32–46.79% |
| CP20 vs CP8 | Confirmation | 204 / 316 | 39.23% | 35.00–43.49% |
| CP20 vs CP12 | Primary | 261 / 259 | 50.19% | 45.91–54.47% |
| CP20 vs CP12 | Confirmation | 261 / 259 | 50.19% | 45.91–54.47% |
| CP20 vs CP16 | Primary | 292 / 228 | 56.15% | 51.86–60.36% |

The original eight panels total **4,160 games**; the supplemental CP12–CP8 confirmation adds **520**, for 4,680 completed games. The two CP20–CP12 panels happen to have identical aggregate totals; they are separate seed schedules, not duplicate rows. Shared primary seeds and multiple comparisons prevent treating the entire table as independent evidence.

A separate **2,600-game policy-only CP12–CP8 test** gave CP12 **1,374 wins and 1,226 losses: 52.85%, with a recorded 95% interval of 50.92–54.76%**. Thus the tested policy-only and search settings favor different checkpoints. The cohorts and protocols differ, so subtracting these scores would not estimate a controlled causal effect of enabling search.

**Lesson:** retain CP8 as a serious search baseline; do not replace it solely because a later checkpoint has better anchor scores. The evidence does not identify the cause of the reversal. Policy priors, value calibration and search allocation are hypotheses to investigate, not established explanations. An earlier CP16 search-lift study failed and remains failed; it cannot supply the missing controlled comparison.

A separate 26-position audit increased search from 320 to 1,280 simulations. The first action changed on 4 positions for CP8, 10 for CP12, 7 for CP16 and 6 for CP20. This demonstrates budget sensitivity on those saved positions, not that the larger budget improved play. The positions and their trajectory outcomes are correlated and do not calibrate the models' predicted win probabilities. See the [position audit](../runs/ex13-model-evaluation-20261029/position-audit/reported-results.txt).

Evidence: [original search reports](../runs/ex13-model-evaluation-20261029/index.html), [supplemental confirmation](../runs/ex13-cp12-vs-cp8-confirmation-20261029/cloud/cp12-vs-cp8-confirmation/reported-results.txt), [policy-only comparison](../runs/ex13-cp12-vs-cp8-policy-20261029/summary.json).

## Mulligan behavior is measurable but its benefit remains uncertain

The matched-root diagnostic completed **7,488 search decisions** over **208 saved roots**: four models, three world counts and three paired search seeds. Those are repeated decisions on a finite set of positions, not 7,488 independent games.

Increasing the world count from four to sixteen reduced the fraction of roots whose decisions changed across the three seeds:

| Checkpoint | Unstable with 4 worlds | Unstable with 16 worlds |
|---|---:|---:|
| CP8 | 37.98% | 11.06% |
| CP12 | 39.90% | 11.54% |
| CP16 | 37.98% | 17.79% |
| CP20 | 38.46% | 18.27% |

More sampled worlds made decisions more stable in this diagnostic. Stability alone does not establish correctness or higher win rate. Likewise, a level-3 card or an apparent access card in hand does not prove a playable sequence after costs, egg compatibility and required material are considered.

The follow-up forced-branch study completed **832 games over 52 preregistered roots**, belonging to 49 original source blocks. Each checkpoint had 104 paired keep/redraw outcomes, with its own follow-on pilot against a fixed CP8 opponent. The overall keep-minus-redraw effects were:

| Checkpoint | Difference in percentage points | Source-block 95% interval |
|---|---:|---:|
| CP8 | −4.81 | −17.92 to +9.00 |
| CP12 | −4.81 | −17.92 to +8.65 |
| CP16 | 0.00 | −12.00 to +12.26 |
| CP20 | −1.92 | −15.38 to +12.26 |

Every overall interval includes zero. The study does not establish an overall keep/redraw advantage, equivalence, or a universally optimal mulligan policy. With only four roots per archetype, apparent archetype effects are particularly sparse.

Evidence: [matched-root diagnostic](../runs/ex13-mulligan-full-report-20261030/index.html), [controlled continuations](../runs/ex13-mulligan-continuations-report-20261030/index.html).

## Tamer and evolution telemetry needs opportunity denominators

In the continuation study, direct legal Tamer evolution was executed on **28/65 opportunities for CP8, 23/51 for CP12, 29/66 for CP16 and 31/54 for CP20**. CP20 also left fewer available Tamer plans unvisited in this cohort: 9/157, compared with 123/274 for CP8. These are descriptive counts across correlated branches and decisions, not independent samples of move quality.

We can now distinguish a route being unavailable, available but unvisited, searched but not selected, and actually executed. That is more useful than counting how often a card appeared in a winning game. Raising evolution, battle evolution, DNA evolution and promotion remain separate routes.

The reported priors are normalized **complete-plan priors**, not raw network action probabilities. Direct Tamer opportunities exclude forced roots and pending effect-sequence opportunities. The latter remain unmeasured. Deterministic evaluation with zero noise also does not measure exploration during training.

**Lesson:** use opportunity, search-visit and execution counts together before diagnosing a model as “ignoring” a mechanic. A Tamer-first baseline is implemented, but its planned rollout is still pending; implementation is not outcome evidence.

Evidence: [candidate opportunity summary](../runs/ex13-mulligan-continuations-report-20261030/candidate-opportunity-summary.json), [telemetry contract](../openspec/changes/evaluate-frozen-checkpoints-closed-deck-search/search-opportunity-telemetry.md), [Tamer baseline plan](../openspec/changes/evaluate-frozen-checkpoints-closed-deck-search/tamer-first-baselines.md).

## Historical closed-decklist qualification and the first scored failures

This section records the October 30 runtime and failures. The November 6 qualification and scored completion are recorded in the following section.

Closed-decklist search may use public facts, the acting seat's entitled knowledge and a frozen training-only deck prior. It must not consume the true hidden opponent composition. Known-list controls answer a different question and remain explicitly labeled.

The original closed scored queues stopped after two different failures:

| Panel | Failure | Consequence |
|---|---|---|
| CP12 vs CP8 on Hetzner | Container OOM under a 6,307,729,408-byte limit | 23 committed two-game receipts and one rejected request; no complete panel score |
| CP16 vs CP12 locally | RunTail continuation context rejected by the hidden-information audit | 64 committed two-game receipts and one rejected request; no complete panel score |

Both failures retain pre-search action snapshots: 208 recorded actions for the OOM request and 44 for the continuation rejection. A retained snapshot is not an independently validated replay, and the failed requests lack final native outputs.

A conservative production candidate permits projection of unavailable captured references only for narrowly certified literal-memory/empty continuations. Opaque operations, formulas and unsafe surrounding state remain rejected. Its **96 software tests per OS** and fresh qualification do not prove that it repairs the exact failed CP16–CP12 game; that game's residual sequence and entitlement-loss cause remain unresolved.

Both fresh candidate runtimes completed **134 requests / 268 excluded reference games per OS**, with zero rejects. Exact Windows/Linux comparison passed all 268 games with no non-timing differences. The repeated OS references are correlated software qualification, not 536 independent strength games.

An earlier parity failure traced to platform `f32.exp` after matching raw ONNX outputs. Pinning portable policy math allowed fresh exact qualification; the earlier failure was preserved. This showed why matching models and nominal seeds is insufficient without checking the actual inference/runtime behavior.

**Lessons:** qualification reduces known implementation risk but does not establish scored reliability on every position; memory samples are not upper bounds; information-safety rejections must remain visible rather than being weakened to finish a run.

Evidence: [failure diagnosis](../runs/ex13-closed-continuation-failure-audit-20261030/diagnosis.md), [candidate review](../runs/ex13-closed-residual-certificate-20261030/candidate-review.md), [candidate qualification proof](../runs/ex13-closed-candidate-qualification-report-20261030/completion-verified.json), [numeric diagnosis](../runs/ex13-mulligan-numeric-probe-20261030/completion-verified.json).

## Latest held-out closed-list evaluation

The November 6 Windows attempt used runtime source 82c2877f88ac071e97506c6eb7419c25624d3566, with the compiled breeding-selection repair. Both checkpoints used 320 total simulations, four worlds, batch size four and zero root noise. Each panel had 120 four-game blocks with balanced seats and opening order.

| CP20 opponent | Wins | Losses | Games | Win rate | Four-game-block 95% interval |
|---|---:|---:|---:|---:|---:|
| CP12 | 231 | 249 | 480 | 48.125% | 43.958% to 52.292% |
| CP16 | 253 | 227 | 480 | 52.708% | 48.958% to 56.458% |

CP20 was competitive against both checkpoints in this setting. Both intervals include 50%, so the ordering remains uncertain. Both sides used search, leaving search lift relative to policy-only play unresolved.

Deck-specific slices varied. CP20 scored 65.0% with Chronomon against CP12 and 67.5% against CP16, compared with 25.0% and 30.0% with Medusamon. Each slice has 40 games per opponent. These exploratory comparisons require uncertainty and multiple-comparison context.

Source-specific Windows/Linux qualification passed all 134 excluded reference requests / 268 games per platform with exact non-timing parity. The Linux evidence retains separate 71-request and 63-request cohorts. The scored Windows attempt completed 480 paired requests / 960 games with zero rejections. Both panels were independently reviewed and archived, all 480 native exits were recorded as zero, and terminal evidence confirmed host-lock release. Monitoring was removed after terminal verification.

Evidence for the two scored panels and their exact hashes is indexed in [validation.json](../openspec/changes/evaluate-frozen-checkpoints-closed-deck-search/validation.json). Its current evaluation section binds qualification proof e2739b63dafdd13d17fd5e62bb0037b06e65ceb42dfa1ffbbe2796dfda383759 and terminal proof 576983d709b10bb38a30074f643b3e1c1a7229f642855c0560ca79d45afe1c6b. The underlying archives remain in the operational workspace.

## Remaining evaluation work

The planned study covers 22 panels / 11,200 games. OpenSpec remains ACTIVE at 21/35. The latest authorized two-panel dispatch is complete and has no further scored scope.

CP20 vs CP8 retains its independently verified 480-game panel from a separate prior runtime. Fourteen historical complete panels / 7,280 games await corrected-rules eligibility, and five previously claimed incomplete panels remain unresolved. The earlier known-list search results above retain their historical runtime and population labels.

The next reporting task is to reconcile those cached proofs against the planned matrix. A complete held-out round robin also needs the three checkpoint pairs among CP8, CP12 and CP16, subject to runtime compatibility and separately authorized scope. Matched information-regime comparisons, final resource analysis and the next-experiment decision remain open.

The [review summary](../openspec/changes/evaluate-frozen-checkpoints-closed-deck-search/execution-status.md) records current completion and limitations. Earlier launch authorizations, partial outcomes and failed attempts remain in their original evidence roots.

## Operational practices worth retaining

1. **Freeze inputs before execution.** Bind source, binary, models, priors, decks, seeds and request schedules. A source overlay is not a complete build-input manifest; changed code needs a new runtime and qualification.
2. **Count committed outputs.** Receipt timestamps show progress. Process existence and heartbeat timestamps alone do not. Verify complete cohorts before reporting scores; rejected or truncated evaluations do not become draws.
3. **Separate failure classes.** Packaging/compile failures before inference are setup failures. OOM and native safety rejections during evaluation remain failed attempts. Preserve both kinds without silently replacing them.
4. **Reserve resources from live capacity.** The October 30 local host had roughly 64 GiB installed and about 34 GiB available at launch. That queue reserved 8 GiB plus sampled-worker headroom and assigned one worker to the first OOM retry. The November 6 scored owner also used one worker. Qualification peaks are samples, not guarantees.
5. **Archive before reclaiming storage.** Thirty-three completed Hetzner artifact directories were copied into verified local archives. Every regular file was hash-checked and the remote inventory rechecked before removal. This reclaimed 11.39 GiB and left about 14.72 GiB free at cleanup. Training-generation roots and shared runtime/claims remain on Hetzner.

The [archive catalog](../runs/hetzner-artifact-archive-20261030/index.html) includes restore instructions, file manifests, per-archive hashes and retained terminal-container records. Archived remote paths describe historical execution; their absence is not a new failure and must not trigger a restart. Existing local reports remain readable.

For ongoing operations, use the [training runbook](TRAINING_RUNBOOK.md), [model evaluation guide](MODEL_EVALUATION.md), and [finite-program tasks](../openspec/changes/evaluate-frozen-checkpoints-closed-deck-search/tasks.md). The current evaluation additions above summarize the independently reviewed November 6 evidence.

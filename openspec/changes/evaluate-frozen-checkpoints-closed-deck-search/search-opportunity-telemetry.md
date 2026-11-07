# Opportunity, exploration and temperature telemetry

Authorized by the user's 2026-10-30 request to measure legal Tamer evolution alternatives, their probabilities/search effort, and what was actually played. This adds observability; it does not authorize changing training hyperparameters or frozen runs.

## Recorded unit

One non-forced searched root, with all complete commitment plans. Page navigation is transport, not a separate strategic alternative. `search-opportunities-v1` records each plan's pre-noise and post-noise prior, visits, Q (null when unvisited), category, selection probability, selected plan, and actual executed action prefix. An incomplete prefix is not an executed commitment. It records game/decision/search seeds, actor, model/history role, deck labels, phase, requested/actual simulations, world count, full/cheap search, policy weight and temperature clock in training context.

Priors are search's normalized **complete-plan priors**, not raw ONNX logits or necessarily a single action's softmax probability. Under PIMC they are means across sampled worlds; visits are summed. Retaining them does not evaluate the model again or consume extra search RNG. Q is from the searching actor's perspective.

Classify direct Tamer digivolve, raising digivolve, battle digivolve, DNA, promotion and hard play separately. Follow page prefixes only on clones, validate live masks, and inspect the final commitment before execution. A field effect that might later offer an evolution is **not** counted as an already-legal direct Tamer route. Effect-sequence opportunities remain unmeasured. Existing execution events describe effect routes separately.

## Outputs

Future self-play builds opt in with `DIGIMON_SEARCH_OPPORTUNITY_TELEMETRY=1` and write `search_opportunities.jsonl` beside the shards (create-new; no overwrite). The manifest declares the schema/path. This is diagnostic evidence, not an input to policy/value fitting. Policy-only and forced roots are outside its denominator. Root rows may belong to incomplete games; they are never strength scores.

The new, not-yet-qualified `frozen_checkpoint_modes` driver records the same object in each `search_diagnostics[].opportunity`, with the enclosing request/model/runtime/receipt identifying provenance. Existing frozen binaries and records are untouched. New scored runtimes must be rebuilt, frozen and qualified with this instrumentation; time profiling must include its overhead.

Run `python -m tools.cloud_search.search_opportunity_report INPUT.jsonl --out report.json` with `code` on PYTHONPATH. New native match JSON files are also supported. Reports include opportunity counts (one per root regardless of target/copy count), prior/visit mass, zero-visit alternatives, selected-below-max-prior/visits, sampling entropy and explicit execution, grouped by actor/model/deck/phase/full-cheap/selection mode. Means carry measured-root denominators. Missing priors and zero-total visit shares remain unmeasured, never zero-filled. Decisions/repeats are correlated and these descriptive reports imply neither move correctness nor strength.

## Existing temperature semantics

Self-play uses visit-proportional sampling (tau=1) while the non-forced decision clock is below `temperature_decisions` (default/continuation launcher 16), then most-visited complete plan selection (tau=0). The clock covers both actors and includes non-forced route navigation. Zero total visits uses a separately labeled uniform-over-plans fallback. Full searches may mix Dirichlet root noise; cheap searches disable it and produce value-only rows. Evaluation selects the most-visited plan with zero root noise. No new temperature setting or schedule change is introduced.

## Validation

Require source-state preservation, no extra inference/RNG consumption, complete-plan prior propagation through serial/batched/PIMC/IS-MCTS, page-prefix execution integrity, direct Tamer classification, same-card hard-play comparison, zero/missing-data denominators and end-to-end training-on/off shard equality. Preserve historical selected-replay counts as a separate report; no backfilled probabilities or reconstructed searches represented as original decisions.

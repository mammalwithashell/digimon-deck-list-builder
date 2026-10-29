# English EX13 search pilot — 2026-10-23

User-authorized scope: repair native search admission, model-contract/profile
alignment, and terminal-outcome value fitting; then run a 200-game check and
one 2,000-game generation. This does not complete the broader search roadmap.

## Experiment

- Current meta reference: <https://digimon.digilab.cards/meta> (checked October 23).
- Original English EX13-labelled tournament lists from the committed library,
  dated October 1–23. The local library is an older frozen ingest; this job does
  not claim that it contains every list published through October 23.
- Eight strategies: Examon, TS Jupitermon, Glowing Dawn, TS Mervamon, Chronicle,
  Toho Braves, Three Musketeers, Chronomon. Exact selected lists and source URLs
  are recorded in `pool_report.json`. Two distinct lists train each strategy;
  one additional distinct list is held out. Sampling is uniform over strategies.
- Every selected list must pass mechanics admission, per-card admission, the
  fresh compiled-registry/production-data intersection, and English deck legality.
  Ulforce is excluded by admission; this is a supported meta subset.
- Official English restriction updates are sourced from
  <https://en.digimoncard.com/rule/restriction_card/>: April 4 and September 1,
  2026 changes, including the two April unrestrictions.
- `standard_lite_v2`, current seven-field model contract and action semantics;
  feed-forward policy/value model. No action/tensor contract changes.
- Known exact decklists, sampled hidden arrangements: an open-decklist search
  experiment, not closed-decklist inference or an equilibrium guarantee.

## Seed and value semantics

No contract-bearing compatible checkpoint was found among the local starter,
specialist, old AlphaZero or downloaded checkpoints. Do not stamp modern metadata
onto those weights. Initialize a fresh card-embedding model, collect 400 legal
greedy games, and fit greedy policy demonstrations for four epochs. This is a
heuristic bootstrap, not a trained tournament champion or search demonstration.

Fit only critic-specific layers to completed-game outcomes for eight epochs,
resetting the final value head and keeping actor/shared feature weights fixed.
Hold out 20% of completed games for critic fitting metrics. The initial imitation
uses no value loss, so these held-out outcomes were not fitted by that stage.
The marker `terminal_outcome` records target semantics; it does not certify
probability calibration. Save fit/held-out MSE in the calibration report.

Truncated rows remain visible in the data but never become draw targets for the
critic. `terminated.npy` distinguishes real terminal draws from decision/turn
caps. Search rejects models without terminal-value semantics and bounds terminal
value outputs to [-1, 1].

## Run

Local hardware: Ryzen 7 3700X, eight physical cores, approximately 64 GB RAM,
RTX 4070 12 GB. Four native workers with one ONNX intra-op thread each; CUDA
for network updates. No cloud resources are provisioned.

Active repaired-engine retry: `models/2026-10-23-english-search-v3/`. The earlier
`models/2026-10-23-english-search/` and `...-v2/` are retained as preflight and
invalidated-engine evidence respectively.
Build and verification logs:
`runs/2026-10-23-english-search/`. Use the task-local `.venv-selfplay` and the
fresh wheel built from this checkout, not the system-installed engine wheel.

1. Prepare and validate frozen training and held-out snapshots.
2. Bootstrap policy, fit terminal critic, and export the compatible seed.
3. Check 100 games at 160 total simulations and 100 at 320, four worlds per
   decision, matched initial seed streams. First 16 meaningful decisions sample
   visit counts; subsequent decisions choose the maximum. Cap at 1,200 decisions.
   This is health/throughput evidence, not a mirror-win-rate strength test.
4. Only if both check runs complete with at most 5% truncations, generate 2,000
   games at 320 total simulations / four worlds. Train two epochs with batch 512,
   learning rate 0.0001. Save the checkpoint and re-export ONNX.
5. Evaluate the plain trained policy on the held-out list snapshot against greedy
   and the frozen bootstrap seed, 300 opening-seat-balanced games per anchor.
   Evaluation failure prevents marking the generation complete. No promotion is
   automatic. Search-assisted playing strength and exploitability are not measured
   by this initial job.

Commands are implemented by `code/tools/english_meta_search_job.py` phases
`prepare`, `bootstrap`, `check`, `generation`. Preserve failed-run artifacts;
never overwrite them with a successful retry. A missing checkpoint contract,
missing admission file or missing evaluation snapshot fails closed.

## Evidence

Initial implementation verification: tier 0 all 13 checks; Python 111 passed;
native CLI 12 passed; deck-tools/determinization/ONNX/search/self-play suites
88 passed. The critic-fit synthetic test confirms unchanged actor probabilities
and excluded truncation labels. Native command tests reject blocked/illegal
decks and missing artifacts even with `--pool all`.

Initial bootstrap: 400 greedy games, 390 completed and 10 capped. Of 47,159
decision rows, 11,952 capped-game rows were excluded from value fitting.
Critic fit used 28,348 rows and held out 6,859 rows by complete game. Final
held-out outcome MSE 0.95338; this is a weak starting critic, not a strength
claim. ONNX/Python maximum logit difference 2.38e-6 and value difference 7.15e-7.

The first held-out evaluation failed at game index 67, seed 42522181. Its
native recording and context are preserved in the initial model directory.
This was an EX13-060 Assembly decline/retry cycle in the greedy opponent,
not an action-mask rejection: actions alternated PLAY 0 / decline 62. A failing
Rust regression was added before fixing the greedy material-selection choice.
The generic opponent action cap misleadingly described this as a single-action
no-op. That failure is not scored as a draw or an evaluated result. The initial
bootstrap is retained as preflight evidence; the active v2 run collects its seed
from the corrected heuristic before the requested check/generation.

The initial heuristic regression passed with the full 15-test policy suite.
However, the follow-up investigation below found that its premise of preserving
decline at an unaffordable unreduced cost was wrong; that policy change masked
an engine legality defect.
Anchored evaluation can now save failure context/native recordings through
`DIGIMON_EVAL_FAILURE_DIR`; safety-cap exits do not count as terminal draws.

The captured failing game completes after the fix (see
`runs/2026-10-23-english-search/repro-fixed.json`). Follow-up Python checks:
17 passed, one skipped; these overlap the earlier Python suite.

Corrected v2 bootstrap: all 400 games completed, zero capped, 36,677 decision
rows in 142.6 seconds. Critic fitting used 29,480 rows, with 7,197 rows held out
by whole game. Final held-out outcome MSE is 1.05385, so this critic remains
weak despite its corrected target semantics. ONNX/Python maximum logit
difference is 1.43e-6 and value difference 2.38e-7. The 200-game native search
health check was started. A separate seed-policy evaluation used held-out
lists using the same evaluation seeds reserved for comparing generation 0;
concurrent CPU load means check throughput is not an isolated benchmark.

Additional evaluator regression: an engine `game_over` flag with terminal reason
`step_limit` or `crash` must fail evaluation rather than count as a draw; a
genuine terminal draw remains allowed. The test failed for both safety reasons
before the fix. Final focused Python suite: 119 passed, one skipped.

The strict v2 seed evaluation stopped at seed 42522129 (game index 41) on
`step_limit`. The saved 1,000-action recording shows the learned candidate
alternating PLAY 5 and decline 62 on turn 12. This was initially attributed to
the weak policy, but exact replay identifies an engine legality defect below.
It is not a scored draw or a completed win-rate measurement. Its context and
recording are in the active run's `eval_failures/`. The earlier pre-strict
baseline process was stopped and its log preserved separately. Generation 0
still has to pass its own strict held-out evaluation; bootstrap success does
not establish policy strength.

An independent hidden supervisor was configured to wait for the health check,
then invoke the complete generation/training/evaluation phase only if the
passing `check_summary.json` existed. `pipeline_state.json`,
`supervisor_launch.json`, `check.log` and `generation.log` in the active run
record its state. The supervisor and unfinished check workers were stopped on
discovery of the engine defect. Partial v2 results are preserved and cannot
certify the repaired engine; generation 0 was not started. Task 3.9 remains
unchecked until all requested execution finishes on the repaired build.

### Assembly loop investigation, 2026-10-23

Fresh checkout audit: HEAD `6ab7f3627`, 22 commits ahead of fetched `origin/main`,
zero behind. Replaying the saved actions reproduces the cycle starting at
record index 45: at zero memory Player 1 declares EX13-060 Alphamon from hand
slot 5, then takes PASS 62 on the first Assembly material prompt. Alphamon
prints cost 13 and Assembly -5, so it can be paid only with the reduction
(8 memory). Declining returns the unchanged card to hand without paying.

Comprehensive Rules 4.3 section 1-3-11-4 requires completing as much Assembly
processing as needed to pay a declared play; section 7-3-2-9 does not make
unpayable declaration branches legal. The local DCGO `SelectAssemblyClass.cs`
sets `canNoSelect: () => true` unconditionally and is not authoritative against
the official rule here. Source: <https://world.digimoncard.com/rule/pdf/general_rule.pdf>.

The defect is in `install_assembly_element`: it uses `is_first` as the optional
zero-pick gate without checking unreduced payment affordability, and copies
that unconditional flag into the cloneable resume frame. Evidence:
`runs/2026-10-23-english-search/assembly-loop-trace.json`. The fix must gate
both live and resumed selections by the actual pending cost, including effect
reductions and assigned costs; it must preserve every material choice and
preserve declining when payment without Assembly is legal.

Repair verification: the new required-payment Rust test fails on the old
engine. After repair, all 36 Assembly tests and 28 neighboring
policy/cast-time/assignment tests pass, including exact memory boundary,
effect-reduced cost, assigned base and cloned selections. Tier 0 passes all
13 checks after refreshing stale lowering coverage; admission classifications
are unchanged. The broad greedy workaround was removed. The CLI and wheel
were rebuilt and the wheel installed only in the task venv.

The exact failing model-versus-greedy case now completes in **112 actions**,
winner Player 2 by `security_attack`, using the same v2 seed checkpoint,
decks and seed 42522129 without retraining. Its result and full recording
are `runs/2026-10-23-assembly-payment-fix/exact-loop-fixed{.json,.recording.json}`.
This directly falsifies the earlier policy-only diagnosis.
The action prefix is identical through record index 45; the first divergence
is record index 46, where the old run chose illegal decline 62 and the repaired
run chooses material 1153. The earlier game trajectory did not change.

The invalidated v2 health run had 72 finished games and two truncated games
when stopped. Two workers had unusually long Mervamon mirror games; those
timings do not independently prove another engine bug. A separate traced
native diagnostic repeated the worker's five-game seed stream at 160 total
simulations / four worlds with the frozen v2 seed model: all five games
completed, zero truncated, 476 decisions / 442 rows in 345.2 seconds. The
Mervamon mirror completed in 200 decisions; its earlier stall was not
reproduced in this retry. This does not establish why the old run slowed.
Optional
`DIGIMON_SELFPLAY_TRACE_DECISIONS=1` saves the current root scenario, seeds and
decision number to `current-decision.json` and logs action progress, so a
slow decision can be distinguished from a declaration cycle.

The v3 retry uses the `pipeline` phase: fresh prepare/bootstrap, the full
200-game check, then 2,000-game generation/training/strict evaluation only
on a passing check. It refuses an existing run directory. Two focused Python
tests prove failed checks cannot start generation and existing checkpoints
are preserved; the related Python gate/evaluator suite is 13 passed, one
skipped. No model is promoted automatically.

V3 execution has started: all 400 bootstrap games completed with zero
truncations, 36,415 rows in 140.5 seconds. Final critic held-out outcome MSE
1.08098 remains weak; corrected targets and a repaired mask are not a playing
strength certificate. ONNX export parity passed (maximum logit difference
1.67e-6, value difference 8.34e-7). The 200-game health check is running on four
native workers. `models/2026-10-23-english-search-v3/pipeline_state.json` records
the live stage, and generation/training/evaluation follows only on a passing
check. The first generation is not yet complete.

Generated phase manifests/logs remain the primary evidence; source compilation,
synthetic tests and admission alone are not proof of full-game fidelity or
playing strength. Final execution results will be appended after the job.

## Current status at PR preparation

The repaired v3 health check completed all 200 games (100 at 160 simulations,
100 at 320), with zero truncations. The user then stopped the 2,000-game
generation to add batching. Its 111 reported completions are interruption
telemetry, not certification that all corresponding shards were committed.
`pipeline_state.json` remains `stopped_by_user`, with `auto_resume: false`.
The first generation's fitting and evaluation are incomplete; nothing was
promoted, and task 3.9 remains open.

Subsequent work ran isolated diagnostics and evaluation only. See
[batching and throughput](2026-10-23-search-throughput.md),
[profiling and scheduling](2026-10-23-search-scheduling.md),
[budget study](2026-10-23-search-budget-study.md), and
[expanded confirmation with card metrics](2026-10-23-search-budget-confirmation.md).
Those studies retain 320 simulations/four worlds/batch four as the practical
default; they do not establish the frozen seed model's tournament strength.

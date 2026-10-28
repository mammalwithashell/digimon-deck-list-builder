# CPU search experiments

Publish `training-search-v*` tags through `.github/workflows/training-image.yml`.
The CPU image contains the native search CLI, matching PyO3 wheel, model
exporter, AlphaZero trainer, scheduler and anchored evaluator. The existing
`training-v*` GPU image workflow is unchanged. The CPU workflow smoke-checks
registry/contract parity and native shard generation before pushing the exact
tested local image. The smoke's deliberately capped games are not training or
playing-strength evidence.

Deploy by registry digest, recording the CI run, image source commit, digest,
model/data hashes and mounted deployment-controller hash in the experiment's
provenance. `run_experiment.py` is a separately versioned, read-only mounted
deployment controller; it uses the engine and training tools inside the image.
It validates inputs, checks serial/batched targets, compares worker counts,
checks 100 complete-game attempts each at 160/4 and 320/4, and only then runs
one approved generation at 320/4/batch 4. Either health panel exceeding 5%
truncation fails the launch. Generation uses dynamic four-game chunks, bounded
retries and CPU fitting. No model is promoted automatically.

The native pool samples slots uniformly. Use two slots per archetype when the
meta archetypes have two distinct lists and pet archetypes have one. Repeating
a pet list in two sampling slots gives it equal archetype weight; it is still
one distinct list. Canonical pool snapshots retain each distinct list once.
Preserve exact card counts and normalize only alternate-art suffixes. Do not
repair inadmissible personal lists by silently substituting cards.

Evaluate the meta validation panel and the exact pet lists separately. The
existing eight-list panel was used for budget tuning; neither that panel nor
same-list pet evaluation is untouched promotion evidence. Add distinct variants
later to test deck-list generalization.

Install `experiment.service` as **`digimon-search.service`**: the journal
mirror (`monitor-journal.service`) follows that exact unit name, and per-epoch
training losses never reach the monitor under any other name.
`experiment.service` supervises the container without automatic restart and
stops it after 48 hours. This is a workload limit, **not a billing limit**:
Hetzner charges until the server is deleted. Keep it until artifacts are
harvested and verified. The bind-mounted experiment directory survives a
container exit and a server reboot, but not server deletion.

Run `backup_experiment.py --host root@HOST --key PATH --out PATH --watch` on the
development host to copy immutable committed chunks and checkpoint archives
every ten minutes. Transfers are hash-verified. Backups pause if that machine
is offline; the remote experiment continues. The helper never deletes a server
or remote data. The controller and backup helper refuse competing ownership or
implicit experiment restart. Investigate failed runs before explicit resume.
The archive step skips any round in which a checkpoint `.zip` is under a
minute old or fails its CRC check, and keeps the previous good archive as
`checkpoint-backup.previous.tar.gz`.

To resume a run whose `experiment_state.json` stage is `failed`, run the
controller once by hand with `--resume` (same image and mounts as the
service, appending `--resume` to `python /deployment/run_experiment.py`).
Inputs are re-validated; stages listed in `completed_stages.json` are skipped;
a partial diagnostic or health-check directory is renamed `*.failed-<ns>`
rather than deleted; and an unfinished generation continues through
`run_selfplay_generation.py --resume-selfplay`, which re-checks frozen input
and committed chunk hashes. A generation that already wrote training
artifacts is refused rather than overwritten.

## Repaired-image checkpoint transitions

`evaluate_checkpoint_transition.py` evaluates saved checkpoints 3 and 4 from a
stopped campaign under one new immutable image. It requires explicit parent
plan/checkpoint hashes and a new output directory. Mount a parent directory and
create the assessment below it; the output itself must not already exist.
Keep that same absolute assessment path when mounting it for later preparation,
because the frozen anchor registry contains absolute container paths.

Both checkpoints use the original frozen decks and learned anchors, the same
seeds and the current `archetype-paired-v2` schedule. Complete canonical card
reports, exact cohorts, runtime/card/input hashes and paired intervals are
required. Historical reports from a different engine or schedule are not
substituted. Selection is a conservative continuation choice, not promotion.

`prepare_checkpoint_continuation.py` consumes the completed assessment and an
explicit selected checkpoint hash. It accepts a saved checkpoint from a failed
parent without changing that failure or its evidence. It validates the full
assessment bundle and the refreshed EX13 pool, copies weights only, and requires
full-budget native health games before writing a usable campaign plan. The
default remains four 1,100-game generations, eight workers, 320 simulations,
four worlds, leaf batch four and a three-generation fresh-replay window.

The EX13 combined diagnostic and original-pet regression panels are evaluated
separately, totaling 10,800 games per checkpoint. Preserve their explicit
same-list/held-out denominators in the plan and W&B config. The supplemental
pool is retained but is not evaluated by default.

`publish_checkpoint_transition.py` publishes the completed assessment's two
card reports and paired comparison to a separate W&B run. It receives a
read-only assessment mount and a separate writable publisher-state directory;
only this logging process receives W&B credentials. New campaign telemetry
continues through the existing read-only `wandb_monitor.py` sidecar. None of
these helpers starts training or changes the archived campaign automatically.

## W&B monitoring

### Smaller-generation campaign

`run_small_generations.py` runs four 1,000-game updates from a validated parent,
with unchanged 320 simulations / four worlds / batch four. Its separate root
preserves the original run. A hash-bound `campaign-plan.json` pins inputs,
parent validation, engine image and Python overlays; existing outputs are
never silently adopted. New truncations stop fitting and require investigation.

Replay uses three source generations, capped at 1,000 games each by a fixed,
outcome-independent ordering of complete four-game chunks. It records selected
manifest and quarantine hashes in `replay_selection.json`; existing quarantine
files still remove whole games before both losses. This balances game budgets,
not decision-row counts: longer games still supply more decision examples.
The original 3,300-game source remains intact. Early updates have fewer than
three source generations. Never split one old collection into several claimed
policy-improvement generations.

Equal native sampling slots give equal expected archetype weight. Generation
coverage is saved in `archetype_coverage.json` and sent to W&B. For A archetypes,
use at least `ceil(75*A)` games for 150 expected player-side appearances; the
current 12-archetype pool uses 1,000 (about 167 expected), not an exact quota.
For 18 archetypes, preserve that coverage with 1,500 games.

The controller owns checkpoint evaluation: baseline once, then each new model,
200 games per candidate archetype per anchor (4,800 total for 12 archetypes and
two anchors). Fixed deck pairs alternate starting player and rotate opponent
archetypes and list variants. Meta validation and same-list pets remain separate
panels. No interval or duplicate final evaluation runs. Stop the old independent
card-evaluation service before switching to this controller; it publishes the
same report/status format for the read-only W&B monitor. The standalone worker
also supports `--checkpoint-only --games-per-archetype 200` for other jobs.

Evaluation failure blocks the next generation; no model is automatically
promoted. A separate W&B run identifies the changed protocol and preserves the
old run's history.

`wandb_monitor.py` is a separate read-only sidecar. Install `monitor.service`
and `monitor-journal.service` as `digimon-search-monitor.service` and
`digimon-search-journal.service`; create `/opt/digimon-search/monitor` first.
The training container retains its network isolation and never receives W&B
credentials. The monitor uses the same pinned image, a read-only experiment
mount, a separate writable monitor directory, and at most 0.25 CPU / 1 GiB.
Starting or restarting it does not restart training.

Create a root-only `monitor/credentials.env` (mode 0600). With `WANDB_MODE=offline`
the monitor only spools JSONL locally. For online publishing set `WANDB_MODE=online`,
`WANDB_API_KEY`, `WANDB_ENTITY`, and `WANDB_PROJECT` in that file. Never commit or
print its contents. Restart only the monitor after changing these values.
Use a unique `--run-id` for each new experiment; preserve it and the monitor
state directory on logger restarts. This experiment defaults to
`english-meta-pets-20261024`. Online startup writes the run URL to
`monitor/state/wandb_run.json` and backfills spooled records.

Metrics include pipeline stage, committed games and positions, positions/sec,
games/hour, truncation rate, worker count, heartbeat age, and an estimated
remaining self-play time while progress is fresh. Diagnostic throughput stays
separate from health-check and training throughput. Fitting losses appear per
epoch; meta-validation and same-list pet evaluation have separate metric keys.
Stable model checkpoints and metadata upload as versioned W&B artifacts.
The logger publishes per-card reports produced by the independent evaluator
described below. It does not send email/Slack alerts. SDK system statistics are disabled because the sidecar's
CPU/memory usage would not describe the training container. A stale scheduler
heartbeat flags stalled progress; the
monitor does not restart a failed training job. Its SDK files and JSONL spool
remain on disk if publication fails; the local upload cursor means SDK enqueue,
not server acknowledgment.

## Scheduled card evaluation

Install `card-eval.service` as `digimon-search-card-eval.service`. Create
`/opt/digimon-search/card-evaluation` for results and
`/opt/digimon-search/deployment/card-eval` for the versioned Python overlay.
The overlay contains `card_eval_worker.py`, `anchored_eval_cli.py`,
`anchored_eval.py`, and `card_eval.py` from this revision. The service binds
these Python modules over the pinned CI image; verify that image exports
`CARD_EVAL_EVENT_VERSION >= 1`. Record overlay hashes alongside deployment
provenance. A future image built from this revision contains the same code.

The monitor service also mounts evaluation results read-only and uses the
updated publisher. Start the evaluator and restart only the monitor; training
does not need to restart. Baseline and periodic windows use the frozen seed
until a new model ZIP is stable. Defaults are 24 games per anchor per panel,
every two hours, plus each new checkpoint and final completion. See
`docs/CARD_EVALUATION.md` for interpretation, configuration and dashboard filters.

Keep the evaluator's output and schedule across restarts. `status.json` exposes
failures and `window-*/reports/checkpoint_*.json` contains atomic local reports.
The monitor waits for the final evaluator window before finishing its W&B run.
To disable evaluation, stop its service and remove the evaluation result mount
from the monitor service (preserve the directory for harvest). Evaluation
failure never stops or resumes training. Copy this directory when harvesting
the experiment; the original checkpoint backup helper covers training files.

## EX13 continuation and diagnosis

The [October 28 training lessons](../../../docs/TRAINING_RUNBOOK.md#ex13-training-lessons-and-raising-diagnostic-2026-10-28)
record the 13-archetype pool denominators, 10,800-game checkpoint evaluation,
checkpoint lineage, token-departure interruption and matched raising study.
Use the dated protocol rather than applying the older 12-archetype examples
above to a new campaign.

`extend_completed_campaign prepare` verifies a completed source and copies its
parent checkpoint plus two replay generations into a fresh root. Its separate
`start` subcommand changes live services and requires explicit rollout scope.
The small-generation controller accepts up to four updates at continuing
logical checkpoint indices; inherited replay is not new progress.

`raising_diagnostic` is a local diagnostic only: frozen-position policy/search
comparisons and paired continuations, with no training or model promotion.
Build its Rust example and follow the runbook's commands and interpretation
limits. The October 28 repairs have not been deployed by these diagnostics.

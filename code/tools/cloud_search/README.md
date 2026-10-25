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

## W&B monitoring

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

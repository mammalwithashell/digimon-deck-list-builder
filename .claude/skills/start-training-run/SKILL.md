---
name: start-training-run
description: Start a Digimon RL training run — on a cloud host (RunPod GPU or Hetzner/DO CPU) or locally. Use WHENEVER the user wants to start / launch / kick off / spin up a training run, train an agent or model, begin a new RL run, run pilot_training, train an LSTM/MLP pilot, stand up a training pod or droplet, or continue training from a checkpoint. Covers picking the host, launching via scripts/train_remote.sh or code/tools/run_training_job.py (cloud) or pilot_training (local), bringing up the TensorBoard watcher, and monitoring/mirroring runs. Cloud-first; the authoritative runbooks are docs/CLOUD_TRAINING.md and docs/TRAINING_RUNBOOK.md. NOT for evaluating an already-trained model (that is anchored eval — docs/MODEL_EVALUATION.md) and NOT for deploying the API (deploy-hosted-api) or the desktop app (cut-desktop-release).
---

# Start a training run

Launch an RL training run for the Digimon agent. **Cloud is the primary path**
(GPU for LSTM / frozen-pool play, CPU for MLP-vs-greedy); local is for quick
iteration. The authoritative runbooks are
[`docs/CLOUD_TRAINING.md`](../../../docs/CLOUD_TRAINING.md) (provisioning,
end-to-end) and [`docs/TRAINING_RUNBOOK.md`](../../../docs/TRAINING_RUNBOOK.md)
(wrapper chain, gauntlet, reward profiles). This skill is the happy-path recipe;
consult the runbooks for anything off the path.

A training run can burn many GPU-hours on a paid host. Confirm host, deck pool,
timesteps, and opponent with the user before launching, and confirm where
artifacts will be persisted (see the **ephemeral storage** gotcha).

## Decide: cloud or local

- **Cloud (default).** Real runs. Path A = RunPod GPU (LSTM, frozen-pool).
  Path B = Hetzner/DO CPU (MLP-vs-greedy, cheaper). Pick per
  `docs/CLOUD_TRAINING.md`.
- **Local.** Smoke / quick iteration only — for a real signal you still need the
  anchored-eval frame below.

## Cloud path

1. **Provision / pick the host** per `docs/CLOUD_TRAINING.md` (RunPod via
   `runpodctl`, or a Hetzner/DO droplet). Join it to the tailnet.
2. **Stage data + start the watcher** (from your laptop / the host):
   ```bash
   rsync -az data/ <host>:~/digimon-training/data/
   rsync -az training_jobs/ <host>:~/digimon-training/training_jobs/
   rsync -az docker-compose.watch.yml <host>:~/digimon-training/
   # on the host, once:
   docker compose -f docker-compose.watch.yml up -d   # TensorBoard on :6006 (tailnet only)
   ```
3. **Launch the trainer.** Either:
   - DO GPU droplet helper: `./scripts/train_remote.sh <droplet_ip> <training_jobs/your_job.yaml> [--publish]`, or
   - the job runner directly: `python code/tools/run_training_job.py <job_config>` (see its `--help`).
4. **Monitor.** TensorBoard at `http://<host>:6006` (tailnet); mirror runs back
   with `scripts/sync_cloud_runs.sh`; inspect with the training MCP
   (`python -m digimon_training_mcp --runs-dir ./runs --models-dir ./models`).

## Local path

```bash
python -m digimon_gym.agents.pilot_training --timesteps 500000
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
python -m digimon_gym.agents.pilot_training --opponent pool --opponent-pool-manifest pool.json --timesteps 1000000
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000
python -m digimon_gym.agents.pilot_training --archetypes rocks,ts-olympos --timesteps 500000
```

`--match-format bo3` (one episode = one best-of-three match) is the default; pass
`--match-format single` for the legacy one-game episode.

## Judging the result (do NOT trust the in-run win rate)

The in-run / mirror eval win rate is **not a cross-mode learning signal** and is
degenerate under self-play. Rank a model only with **anchored evaluation** —
against fixed references (greedy floor + frozen champions), seat-balanced:
`python code/tools/anchored_eval_cli.py --deck-pool-snapshot <run>/deck_pool_snapshot.json --n <adequate-n>`
(see `docs/MODEL_EVALUATION.md`). Training runs also log an in-training anchored
panel (`pilot/anchored/*`), but promotion decisions come from the post-hoc frame.

## Gotchas (each has cost a real run)

- **Ephemeral storage.** Launching with `cd /app` writes artifacts to ephemeral
  `/app/models` (NOT persistent `/workspace`). **Mirror or download the run
  before terminating the pod**, or it is gone.
- **`opponent="self-play"` is retired** and fails at startup. Use
  `opponent="pool"` with a `champion_admin.py emit-pool` manifest.
- **Resume-from-checkpoint:** `run_training_job.py` cannot continue from a
  checkpoint without a small `init_from` patch; the bare CLI also traps on a
  missing `default.yaml`. See the runbook before relying on resume.
- **Reachability:** TensorBoard `:6006` is tailnet-only by design — the cloud
  firewall blocks it publicly. Reach it over Tailscale, not a public IP.
- **Building the engine for training?** Rust target dirs are isolated per
  worktree; a shared `CARGO_TARGET_DIR` causes phantom compile errors. See
  CLAUDE.md rule 31.

## Reference

`docs/CLOUD_TRAINING.md` (provisioning + the two paths), `docs/TRAINING_RUNBOOK.md`
(wrapper chain, gauntlet, standing eval cadence), `docs/MODEL_EVALUATION.md`
(why the in-run number lies; the anchored frame), `AGENTS.md` (architecture).

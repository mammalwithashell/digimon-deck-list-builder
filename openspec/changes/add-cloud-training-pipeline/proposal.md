## Why

13-hour generalist training runs currently tie up the user's local machine, and the generalist mode loads the entire DSL-IMPLEMENTED archetype set with no way to scope it down. LSTM and self-play runs are also pinned to ~95% of a 12 GB local GPU's VRAM (~300 MiB free), so larger curricula don't fit at all. The work has to move off the dev box (onto a cheap GPU pod for LSTM/self-play, a CPU droplet for MLP-vs-greedy), the deck pool needs an explicit knob, and the resulting cloud runs still need to be observable (TensorBoard glances from anywhere, MCP-driven inspection from Claude sessions) — without standing up a domain or a new SaaS vendor.

## What Changes

- Add an `allowed_archetypes` filter that scopes the eligible deck pool used by `MetaGauntlet` and the generalist deck-pool loader; surface it as both a `training_jobs/*.json` field and a `--archetypes` CLI flag on `pilot_training`.
- Make the auto-snapshot at `models/<run_id>/deck_pool_snapshot.json` **default-on** for generalist runs so the resolved pool is always reproducible (currently optional via `--curriculum-pool-out`).
- Add a GitHub Actions workflow that builds `Dockerfile.training` on tag push and publishes to `ghcr.io/<repo-owner>/digimon-trainer:<tag>`.
- Add a watcher stack (`ops/training/docker-compose.watch.yml`) that runs a TensorBoard sidecar over the same `./runs/` volume the trainer container writes to, listening on `0.0.0.0:6006` on the training host's tailnet interface.
- Add two regime-aware access patterns surfaced through one rsync wrapper (`scripts/sync_cloud_runs.sh`, with `DIGIMON_REMOTE_PORT` + `DIGIMON_REMOTE_RUNS` env overrides): RunPod's built-in HTTPS / SSH proxy for GPU pods (Path A), Tailscale-named hosts for Hetzner / DO droplets (Path B). Both terminate in the same mirrored `runs/` tree so the existing `digimon-training-mcp` queries (`list_runs` / `run_metric` / `run_summary`) work uniformly.
- Add a cloud runbook (`docs/CLOUD_TRAINING.md`) with a Decision section routing readers to Path A (RunPod RTX 3090 community at ~$0.30/hr for LSTM / self-play) or Path B (Hetzner CCX23 at ~$0.04/hr for MLP-vs-greedy), plus a Local Mitigations section listing `PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True` and PPO hyperparameter levers that buy headroom before cloud is necessary.

## Capabilities

### New Capabilities
- `cloud-training-pipeline`: published training image, watcher stack, tailnet access, run-mirror sync, and the cloud-runbook contract that ties them together.

### Modified Capabilities
- `generalist-pilot-pretraining`: eligible deck pool gains an optional declared `allowed_archetypes` filter intersected with the DSL-implemented set; auto-snapshot becomes default-on for generalist runs.

## Impact

- **Code**: `code/digimon_gym/agents/gauntlet.py` (constructor + load filter), `code/digimon_gym/agents/pilot_training.py` (CLI flag, snapshot default), `code/tools/run_training_job.py` (job-config field plumbing).
- **CI**: new `.github/workflows/training-image.yml` builds and pushes `Dockerfile.training`. No change to existing API-image workflow.
- **Infra**: new `ops/training/docker-compose.watch.yml` + `ops/training/README.md` for the watcher sidecar. No change to `docker-compose.prod.yml` (API stack).
- **Docs**: new `docs/CLOUD_TRAINING.md` runbook; cross-link from `docs/TRAINING_RUNBOOK.md`.
- **Scripts**: new `scripts/sync_cloud_runs.sh` rsync wrapper.
- **Dependencies**: no new Python or Rust deps; Tailscale is installed at the OS level per host. `requirements-training.txt` unchanged.
- **Non-goals (deferred)**: automatic model upload to the hosted API's `/admin/models/...`, spot-instance resume, multi-droplet orchestration, public domain + Let's Encrypt cert, wandb integration.

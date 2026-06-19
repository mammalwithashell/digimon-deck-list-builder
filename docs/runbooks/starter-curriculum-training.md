# Starter-deck curriculum training (greedy floor → champion pool)

> **Procedural runbook** for the two-phase starter-deck (ST-1..6) curriculum
> run. For the general cloud-training topology (image build, TensorBoard,
> run mirroring) see [`docs/CLOUD_TRAINING.md`](../CLOUD_TRAINING.md); for the
> wrapper chain / reward profiles see
> [`docs/TRAINING_RUNBOOK.md`](../TRAINING_RUNBOOK.md); for why the in-run win
> rate is not a learning signal see [`docs/MODEL_EVALUATION.md`](../MODEL_EVALUATION.md).

## What it does

[`code/tools/train_starter_curriculum.py`](../../code/tools/train_starter_curriculum.py)
runs **one MLP** through two phases in a single command, by invoking the tested
`pilot_training` CLI twice as subprocesses (so all of `main()`'s setup — deck
validation, generalist deck-pool build + snapshot write, reward resolution,
callbacks, the in-training anchored panel — is reused verbatim):

| | Phase 1 — floor | Phase 2 — pool |
|---|---|---|
| Opponent | `greedy` (parallelizable) | champion pool `v020` + `v022` (single-env) |
| Steps | 1M (`--floor-steps`) | 5M (`--pool-steps`) |
| Init | fresh MLP | warm-start `--init-from` phase 1 `final.zip` |
| LR | 3e-4 | 1e-4 |
| Decks | 6 starters, resolved fresh | reuse phase-1 snapshot (`--curriculum-pool`) |
| Shared | `standard_lite_deck_v2`, reward `starter1_6_flat`, BO3, curriculum/eval seeds 123/999 |

The "swap after 1M" is the boundary between the two subprocesses; the warm-start
is plain `--init-from` (same MLP arch + tensor profile across phases, so the
checkpoint contract validates). Run names are pinned via `--set run_name=…`, so
output paths are deterministic:

```
models/starter_floor_v1/{final.zip, deck_pool_snapshot.json}
models/starter_pool_v1/final.zip          ← the candidate
```

## Why these choices (load-bearing)

- **MLP + CPU host.** MLP-PPO is env-bound; GPU buys ~nothing. The champions run
  only forward-inference as the opponent.
- **Greedy floor first.** A fresh MLP vs the strong LSTM champions is a steep
  curriculum; the 1M greedy floor establishes a skill floor (and a free
  anchored-eval reference checkpoint) before the swap.
- **Phase 2 is single-env.** The engine rejects `n_envs > 1` for `pool`
  opponents (`make_vec_env`: "n_envs>1 currently supports greedy/random"), so the
  pool phase cannot parallelize and runs at ~8 steps/sec → **5M pool steps is a
  multi-day run regardless of host/core count.** Phase 1 (greedy) parallelizes
  via `--floor-envs N`. The champion-loop cadence (TRAINING_RUNBOOK §14) trains
  1M pool steps/cycle; `--pool-steps 1500000` is the tractable single-night
  alternative, extended via the next `init_from` cycle.

## Local (quick iteration only)

```bash
# from the BASE repo root (has configs/, data/, cloud_downloads/), release wheel installed
python code/tools/champion_admin.py emit-pool --out pool_starters.json
python code/tools/train_starter_curriculum.py --pool-manifest pool_starters.json
```

`--dry-run` prints both phase commands; phase 1 auto-skips if its `final.zip`
already exists (`--force-floor` to redo, `--skip-floor` to jump to phase 2).

## Cloud (the real run) — CPU host

### 0. Pick a provider

**Dedicated vCPU, not shared/burstable, not spot.** Phase 2 runs single-core for
days; shared/burstable instances throttle and spot instances get reclaimed (no
checkpoint-resume is wired). Cost is a rounding error either way (~$15–45 total).

| Provider | Instance | Dedicated vCPU | RAM | ~$/hr (Jun 2026) |
|---|---|---|---|---|
| Hetzner | CCX33 | 8 (EPYC Milan) | 32 GB | ~$0.26 |
| Hetzner | CCX23 | 4 | 16 GB | ~$0.16 |
| DigitalOcean | c-8 (CPU-Optimized) | 8 | 16 GB | ~$0.25 |

Hetzner CCX33 is cheapest-per-core and best for phase-1 parallelism; DO c-8
reuses the existing `doctl` / API-droplet setup. Provision + bootstrap (Docker,
firewall, optional Tailscale) per CLOUD_TRAINING.md §B.

### 1. Stage data + champions (the one gotcha)

`emit-pool` writes **local absolute paths**, and both champion files share a
filename — so rename them and hand-write a manifest with **container** paths:

```bash
# from your laptop, base repo root
D=~/digimon-training
rsync -az data/    droplet:$D/data/        # deck_library, cards, overrides, tested, aliases
rsync -az configs/ droplet:$D/configs/     # default.yaml the CLI loads
scp code/tools/train_starter_curriculum.py droplet:$D/
scp cloud_downloads/v022-hf4zm2hl82qk48/models/pilot_ppo_starter_decks_generalist_v1.zip droplet:$D/champions/v022.zip
scp cloud_downloads/v020-3fm5tm9kci2isy/models/pilot_ppo_starter_decks_generalist_v1.zip droplet:$D/champions/v020.zip
```

`~/digimon-training/pool_starters.json` on the droplet:

```json
{ "version": 1, "entries": [
  {"name": "v022-generalist-v1", "weights_path": "/app/champions/v022.zip", "algorithm": "lstm", "win_rate_vs_pool": 0.5, "games_played": 0},
  {"name": "v020-generalist-v1", "weights_path": "/app/champions/v020.zip", "algorithm": "lstm", "win_rate_vs_pool": 0.5, "games_played": 0}
]}
```

### 2. One detached `docker run` of the driver

The published `digimon-trainer` image bakes only `run_training_job.py`, so mount
the driver in (it shells the in-image `pilot_training`). Use the latest published
tag — ≥ the one that produced `v022`, already on `standard_lite_deck_v2`.

```bash
docker run -d --name digimon-train \
  -v ~/digimon-training/data:/app/data \
  -v ~/digimon-training/configs:/app/configs \
  -v ~/digimon-training/champions:/app/champions:ro \
  -v ~/digimon-training/pool_starters.json:/app/pool_starters.json:ro \
  -v ~/digimon-training/train_starter_curriculum.py:/app/tools/train_starter_curriculum.py:ro \
  -v ~/digimon-training/models:/app/models \
  -v ~/digimon-training/runs:/app/runs \
  ghcr.io/mammalwithashell/digimon-trainer:<tag> \
  python tools/train_starter_curriculum.py \
    --pool-manifest /app/pool_starters.json --cwd /app \
    --save-dir /app/models --log-dir /app/runs \
    --floor-envs 8 --floor-steps 1000000 --pool-steps 5000000
# follow: docker logs -f digimon-train
```

`--floor-envs 8` parallelizes the greedy floor across the vCPUs (hours); phase 2
ignores it. Optional TensorBoard watcher: `docker compose -f
docker-compose.watch.yml up -d` (tailnet only — see CLOUD_TRAINING.md §B).

### 3. Mirror, retrieve, judge, tear down

```bash
# BEFORE teardown — the lost-weights failure mode (TRAINING_RUNBOOK §14)
scp -r droplet:~/digimon-training/models/starter_pool_v1  ./models/
scp -r droplet:~/digimon-training/models/starter_floor_v1 ./models/   # floor reference

# Judge with anchored eval — NOT the in-run win rate (CLAUDE.md rule 30)
python code/tools/anchored_eval_cli.py \
  --candidate models/starter_pool_v1/final.zip \
  --deck-pool-snapshot models/starter_floor_v1/deck_pool_snapshot.json --n 100
python code/tools/elo_ladder_cli.py --run models/starter_pool_v1   # forgetting check

# Tear down (Hetzner / DO)
hcloud server delete digimon-train
# doctl compute droplet delete digimon-train --force
```

If the candidate clears the gate (≥55% vs the compatible champion panel,
seat-balanced), promote it with `champion_admin.py promote` — the registry grows
and the next cycle's pool is larger.

## Tests

`code/tests/rl/test_starter_curriculum_driver.py` (7 cases) covers both phase
command-lines, the phase-1→phase-2 handoff (warm-start checkpoint + reused
snapshot), shared profile/reward/seeds, and the `--floor-envs` parallelism lever.
The full pipeline (including the champion-pool load + warm-start) was validated
with a tiny both-phases smoke before first use.

---
name: launch-starter-curriculum
description: >-
  Provision a cloud CPU host and launch the two-phase starter-deck (ST-1..6)
  curriculum training run end-to-end — fresh MLP vs greedy (1M, parallel) then
  warm-start + swap to the frozen champion pool (5M). Cuts a fresh training
  image first if the published one is stale. Use WHENEVER the user wants to
  start / launch / kick off the starter-curriculum run on the cloud, run
  train_starter_curriculum.py on a droplet, or "spin up the starter training on
  DigitalOcean". For the LOCAL recipe or the driver internals see
  docs/runbooks/starter-curriculum-training.md; for generic training see the
  start-training-run skill. NOT for evaluating a trained model (anchored eval),
  the hosted API, or the desktop app.
---

# Launch the starter-curriculum run on a cloud CPU host

Drives [`code/tools/train_starter_curriculum.py`](../../../code/tools/train_starter_curriculum.py)
(the two-phase driver: 1M vs greedy → warm-start → 5M vs champion pool) inside the
published `digimon-trainer` image on a DigitalOcean CPU droplet. The driver +
phases are documented in
[`docs/runbooks/starter-curriculum-training.md`](../../../docs/runbooks/starter-curriculum-training.md);
this skill is the **cloud provisioning + image + staging** recipe around it.

A training run spends real money (a dedicated CPU droplet ~$0.19/hr, multi-day).
**Confirm host + step budget with the user before provisioning.** Verify every
prerequisite BEFORE creating the droplet — a misconfig wastes spend.

## 0. Prerequisites (read-only — do these before spending)

```bash
BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"; cd "$BASE"
doctl account get                                   # DO authed
gh auth status                                      # gh authed
doctl compute ssh-key list --format ID,Name,FingerPrint   # note the key ID
for f in ~/.ssh/*; do [ -f "$f" ] && head -1 "$f" 2>/dev/null | grep -q PRIVATE \
  && echo "$f $(ssh-keygen -E md5 -lf "$f" 2>/dev/null | awk '{print $2}')"; done
# Match a DO key's MD5 fingerprint to a LOCAL PRIVATE key -> that's your -i key + --ssh-keys ID.
ls cloud_downloads/v022-*/models/*.zip cloud_downloads/v020-*/models/*.zip   # champions present
```

## 1. Cut a fresh image if the published one is stale

The image (`ghcr.io/mammalwithashell/digimon-trainer:training-v<N>`) lags `main`.
If engine fixes have landed since the last `training-v*` tag, cut a new one
(builds from the tagged commit on `main` via `.github/workflows/training-image.yml`):

```bash
git fetch origin --tags
git rev-list --count "$(git describe --tags --abbrev=0 --match 'training-v*' origin/main 2>/dev/null || echo training-v0.22)"..origin/main   # how stale
NEXT=training-v0.26      # bump from the latest training-v* tag
git tag "$NEXT" origin/main && git push origin "$NEXT"
gh run watch "$(gh run list --workflow=training-image.yml -L1 --json databaseId --jq '.[0].databaseId')" --interval 30
# CONFIRM it published (gh run watch's exit code can mislead — probe the manifest):
gh run view <id> --json conclusion --jq .conclusion        # must be "success"
TOKEN=$(curl -s "https://ghcr.io/token?service=ghcr.io&scope=repository:mammalwithashell/digimon-trainer:pull" | python -c "import json,sys;print(json.load(sys.stdin)['token'])")
curl -so /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $TOKEN" \
  -H "Accept: application/vnd.oci.image.index.v1+json" \
  "https://ghcr.io/v2/mammalwithashell/digimon-trainer/manifests/$NEXT"   # want 200
```

**Known build breakages (fixed on branch `claude/mystifying-darwin-d11683`; verify they're on `main` before cutting):**
- `.dockerignore` is an allowlist (`*` then `!`); it MUST re-include every path
  `Dockerfile.training` COPYs (`requirements-training.txt`,
  `code/tools/run_training_job.py`, `training_jobs/`, `docker/runpod-start.sh`).
- `Dockerfile.training` must COPY all 3 `include_str!` data embeds:
  `data/cards.json data/tested_cards.json data/deck_formats.json`.

## 2. Provision the droplet

CPU host (MLP is env-bound; GPU buys nothing). **Dedicated vCPU, not shared/spot**
— phase 2 runs single-core for days.

```bash
doctl compute droplet create digimon-train --region nyc3 --image ubuntu-24-04-x64 \
  --size g-4vcpu-16gb --ssh-keys <KEY_ID> --tag-name digimon-train --wait \
  --format ID,Name,PublicIPv4,Status
```

> **Tier gotcha:** CPU-Optimized `c-*` (and possibly larger `g-*`) sizes 422 with
> "open a ticket to increase your account tier". `g-4vcpu-16gb` works on the
> default tier. If the chosen size 422s, fall back to `g-4vcpu-16gb` or basic
> `s-8vcpu-16gb` (shared — throttles on the multi-day single-core phase 2), or
> have the user request a tier increase in the DO console.

## 3. Bootstrap + stage

```bash
IP=<droplet-ip>; KEY=~/.ssh/<your-private-key>
SSH="ssh -i $KEY -o StrictHostKeyChecking=accept-new root@$IP"
SCP="scp -i $KEY -o StrictHostKeyChecking=accept-new -q"
$SSH "curl -fsSL https://get.docker.com | sh >/dev/null 2>&1; mkdir -p ~/digimon-training/{data,configs,qa/qa-reports,champions,models,runs}"

# data (incl. deck_library for the generalist pool), the WHOLE configs/ tree,
# the qa DSL ledger, the driver, and the champions RENAMED to distinct files
# (both are pilot_ppo_starter_decks_generalist_v1.zip).
$SCP data/cards.json data/tested_cards.json data/deck_formats.json data/deck_library.json \
     data/archetype_aliases.json data/card_overrides.json root@$IP:~/digimon-training/data/
$SCP -r configs/* root@$IP:~/digimon-training/configs/
$SCP qa/qa-reports/validated_cards_dsl.json root@$IP:~/digimon-training/qa/qa-reports/
$SCP code/tools/train_starter_curriculum.py root@$IP:~/digimon-training/
$SCP cloud_downloads/v022-*/models/pilot_ppo_starter_decks_generalist_v1.zip root@$IP:~/digimon-training/champions/v022.zip
$SCP cloud_downloads/v020-*/models/pilot_ppo_starter_decks_generalist_v1.zip root@$IP:~/digimon-training/champions/v020.zip

# champion pool manifest with CONTAINER paths (emit-pool writes LOCAL paths)
$SSH "cat > ~/digimon-training/pool_starters.json" <<'JSON'
{ "version": 1, "entries": [
  {"name":"v022-generalist-v1","weights_path":"/app/champions/v022.zip","algorithm":"lstm","win_rate_vs_pool":0.5,"games_played":0},
  {"name":"v020-generalist-v1","weights_path":"/app/champions/v020.zip","algorithm":"lstm","win_rate_vs_pool":0.5,"games_played":0}
]}
JSON
$SSH "docker pull ghcr.io/mammalwithashell/digimon-trainer:$NEXT"
```

## 4. Launch (detached)

The image flattens `code/digimon_gym` → `/app/digimon_gym`, but a few config
defaults are repo-relative (`code/digimon_gym/agents/reward/*.yaml`). Prepend a
`code/` symlink so they resolve. `--floor-envs` = the droplet's vCPU count
(phase 1 parallelizes; phase 2 ignores it).

```bash
IMG=ghcr.io/mammalwithashell/digimon-trainer:$NEXT
$SSH "docker rm -f digimon-train 2>/dev/null; docker run -d --name digimon-train --restart no \
  -v ~/digimon-training/data:/app/data -v ~/digimon-training/configs:/app/configs \
  -v ~/digimon-training/qa:/app/qa -v ~/digimon-training/champions:/app/champions:ro \
  -v ~/digimon-training/pool_starters.json:/app/pool_starters.json:ro \
  -v ~/digimon-training/train_starter_curriculum.py:/app/tools/train_starter_curriculum.py:ro \
  -v ~/digimon-training/models:/app/models -v ~/digimon-training/runs:/app/runs \
  \$IMG sh -c 'mkdir -p /app/code && ln -sf /app/digimon_gym /app/code/digimon_gym && \
    exec python tools/train_starter_curriculum.py --pool-manifest /app/pool_starters.json \
    --cwd /app --save-dir /app/models --log-dir /app/runs \
    --floor-envs 4 --floor-steps 1000000 --pool-steps 5000000'"
```

## 5. Verify it's actually training (don't walk away on a crash)

```bash
$SSH "docker logs digimon-train 2>&1 | grep -viE 'UserWarning|raw_rust fn' | tail -30"   # banner, no traceback
$SSH "docker stats --no-stream --format 'cpu={{.CPUPerc}}' digimon-train"                 # ~Nx100% across envs
$SSH "ls ~/digimon-training/models/starter_floor_v1/"   # deck_pool_snapshot.json + reward_profiles.meta.json + per-env mulligan logs
```
Healthy = container Up, no Python traceback, CPU pegged across cores, snapshot
written, TB events flowing under `runs/starter_floor_v1/`.

## 6. Monitor, retrieve, judge, tear down

```bash
$SSH "docker logs -f digimon-train"                                  # live; phase 2 starts after phase 1
$SSH "ssh -L 6006:localhost:6006 ...; docker run ... tensorboard"    # optional TB over SSH tunnel
# BEFORE teardown (the lost-weights failure):
scp -i $KEY -o StrictHostKeyChecking=accept-new -r root@$IP:~/digimon-training/models/starter_pool_v1  ./models/
scp -i $KEY -o StrictHostKeyChecking=accept-new -r root@$IP:~/digimon-training/models/starter_floor_v1 ./models/
# Judge with ANCHORED eval, never the in-run win rate (CLAUDE.md rule 30):
python code/tools/anchored_eval_cli.py --candidate models/starter_pool_v1/final.zip \
  --deck-pool-snapshot models/starter_floor_v1/deck_pool_snapshot.json --n 100
doctl compute droplet delete digimon-train --force
```

## Investigating a low / odd win rate

The driver records **eval games by default** (`--record-games eval`,
`--record-games-max 300`) → `models/<run>/recordings/*.json` (each holds the
initial state + the full action sequence). Plus `eval_game_log.jsonl` (one row
per inner game: result, length, digivolves) and the in-training `pilot/anchored/*`
TB panel. To diagnose:

- **Read the real step / curve from TB, not `docker logs`** (stdout is
  block-buffered in the container and shows nothing useful). Use the
  `docker exec … EventAccumulator` one-liner.
- **Concede check (action 93):** grep the recordings' action lists for `93`.
  Do NOT infer concedes from short games alone — a smooth/unimodal loss-length
  distribution (peak ~13–16 steps, no short-end spike) means concedes are
  negligible even if a few 2-step losses exist.
- **Suspiciously fast wins/losses (≤8–12 steps):** open those recordings and
  replay via `digimon-engine-mcp` / `dcgo-replay` — confirm the terminal
  (deck-out? security? concede? mis-scored?).
- **Sanity baseline:** a fresh MLP-vs-greedy run on these starters with
  `starter1_6_flat` has historically reached **~70–85% vs greedy**
  (TRAINING_RUNBOOK §14, `starter1_6_flat_control_v1`). If a run plateaus far
  below that, treat it as a **regression**, not an MLP ceiling. Prime suspects:
  the `--floor-envs N` SubprocVecEnv path (the known-good baseline used
  `n_envs=1`), an engine behavioral change since the baseline, or reward-profile
  drift. Re-run with `--floor-envs 1` to isolate the parallelism variable.

## Future cleanup (would simplify steps 3–4)

Folding these into `Dockerfile.training` makes the image self-contained, dropping
the `code/` symlink and the `configs/` + qa-ledger staging:
- `COPY configs/ configs/` (for `configs/training/eval_suite.yaml` + `default.yaml`)
- `COPY qa/qa-reports/validated_cards_dsl.json qa/qa-reports/`
- Resolve reward-profile paths package-relatively (or add a `/app/code/digimon_gym`
  symlink in the image) so the repo-relative defaults work under `/app`.

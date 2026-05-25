# Cloud Training Runbook

End-to-end procedure for running a generalist, gauntlet, or self-play pilot
training job on a cheap cloud GPU pod, watching it via TensorBoard, and
mirroring `runs/` back to your laptop so `digimon-training-mcp` can query
the run from Claude sessions.

Companion to [TRAINING_RUNBOOK.md](TRAINING_RUNBOOK.md), which covers local
training. Read this one when:

- A training job will exceed a few hours and you'd rather not tie up your
  laptop.
- You're hitting local VRAM ceilings on LSTM or self-play runs and want
  headroom (your 12 GB card is full at 11.7+ GB; a 24 GB cloud card doubles
  the headroom).
- You want phone/away-from-desk visibility into training progress.

## 0. Decision: GPU or CPU?

The LSTM and self-play training paths are **VRAM-bound** on a 12 GB local
card (~11.7 GB used, ~300 MiB free). They need a GPU with ≥ 12 GB VRAM —
ideally 24 GB for 2× headroom. The MLP-vs-greedy path is **env-bound** and
runs fine on CPU.

```
                       lstm / self-play       mlp / vs-greedy
                       ─────────────────      ───────────────
  needs GPU?           yes (VRAM-bound)       no
  recommended host     RunPod 3090            Hetzner CCX23
  $/hr (24 GB / 8 vCPU) ~$0.30                ~$0.04
  24h run cost          ~$7                   ~$1
```

Most readers want path A. Path B is documented below for the cases where
it applies.

## Why these choices?

The decisions below are recorded in detail in
`openspec/changes/add-cloud-training-pipeline/design.md`. The short version:

- **RunPod over Hetzner/DO for GPU runs** — RunPod has 24 GB consumer cards
  (3090/A5000) at $0.25–0.40/hr community pricing. DigitalOcean and Hetzner
  only sell H100-class GPUs ($3+/hr) at this layer, which is overkill for a
  workload that uses ~50% of a 3090's compute.
- **No domain, no Let's Encrypt** — RunPod's built-in HTTPS proxy gives each
  pod a unique URL (e.g. `https://abc-6006.proxy.runpod.net`) good for as
  long as the pod lives. Tailscale is the alternative if you want a stable
  bookmark across pod recreations; v1 ships without it because RunPod's
  proxy is simpler.
- **Trainer image runs directly as the pod** — no Docker-in-Docker. RunPod
  lets you specify a custom image when creating a pod; we point it at
  `ghcr.io/<owner>/digimon-trainer:<tag>`.
- **Snapshot the deck pool on every run** — `models/<run_id>/deck_pool_snapshot.json`
  records the resolved archetype set, independent of later changes to
  `data/deck_library.json`.

---

# Path A: GPU runs on RunPod (LSTM, self-play, anything VRAM-heavy)

## A.1 Prerequisites (one-time)

```bash
# Local CLI tools:
#   pip install runpod          # optional but handy; web UI works too
#   brew install rsync ssh      # already on most Unix-likes

# Account setup:
# 1. Sign up at runpod.io
# 2. Add a payment method (per-minute billing; $10 minimum top-up)
# 3. Settings → SSH Public Keys → paste ~/.ssh/id_ed25519.pub
# 4. Settings → API Keys → generate one if you'll use runpodctl
```

## A.2 Push the training image to GHCR

Tag and push triggers `.github/workflows/training-image.yml`, which builds
`Dockerfile.training` and publishes to GHCR:

```bash
# From your laptop
git tag training-v0.1
git push origin training-v0.1

# Wait for the workflow to finish (~5 min). Confirm publication:
docker pull ghcr.io/<your-handle-lowercase>/digimon-trainer:training-v0.1
```

If your GHCR package is private, you'll need to make it public OR add a
RunPod container-registry credential under Settings → Container Registry
Auth so the pod can pull it.

## A.3 Create a pod

Easiest path is the web UI (Pods → Deploy). Pick:

| Field | Value | Notes |
|-------|-------|-------|
| GPU | RTX 3090 (24 GB) — community cloud | 4090 / A5000 also fine; A6000 if you'll scale architecture |
| Instance count | 1 | |
| Container image | `ghcr.io/<owner>/digimon-trainer:training-v0.1` | Click "Edit Template" if not shown |
| Container disk | 20 GB | Fits ~600 MB image + work dirs |
| Volume disk | 40 GB | Mount at `/workspace` — persistent across pod restarts |
| Expose HTTP ports | `6006` | TensorBoard |
| Expose TCP ports | `22` | SSH proxy |
| Docker command | `bash -c "sleep infinity"` | Override the trainer entrypoint; we'll kick off training manually after staging data |
| Environment vars | `PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True` | Buys ~200–500 MiB of VRAM on long runs |

Click **Deploy**. The pod boots in ~30 seconds (image pull dominates on first
provision).

The pod page now shows:

- An SSH connection string under "Connect" → "SSH" — copy it:
  `ssh root@<pod>.proxy.runpod.net -p <port> -i ~/.ssh/id_ed25519`
- An HTTPS URL for port 6006 under "Connect" → "HTTP Service":
  `https://<pod>-6006.proxy.runpod.net`

Bookmark the HTTPS URL — that's your TensorBoard for the life of this pod.

## A.4 Stage data and configs

From your laptop, push card data + your job config into the pod's persistent
volume:

```bash
# Convenience: define an alias once
export DIGIMON_POD="root@<pod>.proxy.runpod.net"
export DIGIMON_POD_PORT="<port>"
SCP="scp -P ${DIGIMON_POD_PORT}"
SSH="ssh -p ${DIGIMON_POD_PORT}"

# Card data
$SSH "${DIGIMON_POD}" "mkdir -p /workspace/{runs,models,data,jobs}"
rsync -az -e "$SSH" data/ "${DIGIMON_POD}:/workspace/data/"

# Job configs — cloud-side job_name and output.name should be prefixed
# with `cloud_` so mirrored runs are distinguishable on your laptop.
rsync -az -e "$SSH" training_jobs/ "${DIGIMON_POD}:/workspace/jobs/"
```

## A.5 Run the training job

```bash
$SSH "${DIGIMON_POD}"

# Inside the pod (your custom image is now the shell)
cd /app

# Symlink the persistent volume's dirs into the image's expected locations
ln -sf /workspace/runs   /app/runs
ln -sf /workspace/models /app/models
ln -sf /workspace/data   /app/data

# Start TensorBoard in the background; serves on :6006 (RunPod-proxied)
tensorboard --logdir /app/runs --bind_all --port 6006 &

# Verify GPU passthrough works inside the container
nvidia-smi   # should show your 3090, not "No devices found"

# Verify torch sees CUDA before committing to a long run
python -c "import torch; print('cuda:', torch.cuda.is_available(), torch.cuda.get_device_name(0) if torch.cuda.is_available() else None)"

# Kick off the trainer
python tools/run_training_job.py /workspace/jobs/cloud_my_generalist.json
```

The trainer runs in foreground; TB scalars appear in your browser at the
HTTPS URL within ~30 seconds.

### Example job config: scoped self-play LSTM

```json
{
  "job_name": "cloud_rocks_self_play",
  "agent_deck": { "source": "default" },
  "training": {
    "generalist": true,
    "allowed_archetypes": ["Rocks", "Yellow Hybrid"],
    "timesteps": 5000000,
    "opponent": "self-play",
    "use_lstm": true,
    "lstm_hidden_size": 256,
    "learning_rate": 3e-4,
    "n_steps": 2048,
    "batch_size": 64,
    "eval_freq": 25000,
    "n_eval_episodes": 50
  },
  "output": {
    "name": "cloud_rocks_sp_v1",
    "save_dir": "models",
    "log_dir": "runs/pilot_ppo"
  }
}
```

The `allowed_archetypes` field scopes the eligible deck pool to just those
archetypes. The resolved set is intersected with the DSL-implemented safety
floor and written to `models/<run_id>/deck_pool_snapshot.json` for
reproducibility. Aliases (e.g. `"RockClose"` for `"Rocks"`) canonicalize
through `data/archetype_aliases.json` automatically.

## A.6 Detach without killing the run

If you'll close your laptop and reattach later, run inside the pod under
`screen` or `tmux`:

```bash
# Inside the pod
tmux new -s train
python tools/run_training_job.py /workspace/jobs/cloud_my.json
# Ctrl-B then D to detach. Reattach later with `tmux attach -t train`.
```

## A.7 Mirror runs/ to your laptop

```bash
# On your LAPTOP, in the repo root
DIGIMON_REMOTE_RUNS=/workspace/runs/ \
DIGIMON_REMOTE_PORT=${DIGIMON_POD_PORT} \
scripts/sync_cloud_runs.sh ${DIGIMON_POD}
```

Cron snippet on your laptop, active while a pod run is alive:

```cron
*/5 * * * * cd ~/digimon && DIGIMON_REMOTE_RUNS=/workspace/runs/ \
            DIGIMON_REMOTE_PORT=<port> \
            scripts/sync_cloud_runs.sh <pod-ssh-host> \
            >> /tmp/digimon-sync.log 2>&1
```

After a sync, `digimon-training-mcp` queries hit the mirrored run as if it
were local:

- `list_runs` shows the cloud run with `latest_step` / `latest_win_rate`
  populated and a `last_modified` that reflects rsync time (so you can tell
  if the mirror is stale).
- `run_metric` / `run_summary` / `run_per_game_evals` work end-to-end against
  the mirrored TB event files and eval sidecar.

The `cloud_`-prefixed names from cloud job configs keep them visually
distinguishable in `list_runs`.

## A.8 Retrieve the trained model

When the trainer exits cleanly:

```bash
# On your LAPTOP
RUN_ID=cloud_rocks_sp_v1
$SCP -r "${DIGIMON_POD}:/workspace/models/${RUN_ID}" ./models/
```

You now have:

```
models/${RUN_ID}/
├── final.zip                 ← the SB3 model
├── deck_pool_snapshot.json   ← resolved deck pool (for reproducibility)
├── eval_game_log.jsonl       ← per-game eval log
└── training_run.json         ← run metadata sidecar
```

When you're ready to ship it to the client app, upload via the hosted API's
`/admin/models/upload` endpoint or your existing `tools/upload_model.py`
flow — this runbook deliberately does NOT automate that step. Curate before
publishing.

## A.9 Tear down

RunPod web UI → Pods → your pod → **Terminate**. Billing stops at the minute
the pod is terminated. The volume disk also goes away unless you "Stop" the
pod instead of "Terminate", in which case storage charges continue at
~$0.10/GB-month.

For your one-run-at-a-time workflow, **Terminate**: rsync the model to your
laptop first, then tear the whole thing down.

---

# Path B: CPU runs on Hetzner CCX (MLP vs greedy, env-bound jobs only)

For MLP-vs-greedy runs that don't need a GPU. Roughly 3× cheaper than DO at
the same vCPU shape. Skip this whole section if you're running LSTM or
self-play.

## B.1 Prerequisites (one-time)

```bash
# Install Tailscale on your laptop and phone.
# macOS:   brew install --cask tailscale
# Linux:   curl -fsSL https://tailscale.com/install.sh | sh
# Windows: download from tailscale.com/download/windows
# iOS/Android: App Store / Play Store

tailscale up   # signs you in via browser

# Generate an ephemeral auth key.
# Tailscale admin console → Settings → Keys → Generate auth key
#   - Reusable: no
#   - Ephemeral: yes
#   - Pre-approved: yes
#   - Tags: tag:training
# Save as TS_AUTH_KEY in your shell env.
```

## B.2 Provision the droplet

```bash
# Install hcloud CLI: brew install hcloud
hcloud context create digimon
hcloud ssh-key create --name laptop --public-key-from-file ~/.ssh/id_ed25519.pub

# 8 vCPU, 32 GB, dedicated-vCPU — ~$0.04/hr.
hcloud server create \
  --name digimon-train \
  --type ccx23 \
  --image ubuntu-24.04 \
  --location nbg1 \
  --ssh-key laptop

# Block public inbound except SSH.
hcloud firewall create --name digimon-train
hcloud firewall add-rule digimon-train \
  --direction in --protocol tcp --port 22 --source-ips 0.0.0.0/0 --source-ips ::/0
hcloud firewall apply-to-resource digimon-train \
  --type server --server digimon-train
```

### DigitalOcean (fallback)

```bash
doctl compute droplet create digimon-train \
  --region nyc3 --image ubuntu-24-04-x64 --size c-8 \
  --ssh-keys "$(doctl compute ssh-key list --format ID --no-header | head -1)" \
  --wait
```

## B.3 Bootstrap (first SSH)

```bash
# Tailscale
curl -fsSL https://tailscale.com/install.sh | sh
sudo tailscale up --authkey="${TS_AUTH_KEY}" --hostname=digimon-train --ssh

# Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER && newgrp docker

# Workspace
mkdir -p ~/digimon-training/{runs,models,data,training_jobs,ops/training}
cd ~/digimon-training
```

## B.4 Stage data, pull image, start watcher

```bash
# From your LAPTOP
rsync -az data/ digimon-train:~/digimon-training/data/
rsync -az training_jobs/ digimon-train:~/digimon-training/training_jobs/
rsync -az ops/training/ digimon-train:~/digimon-training/ops/training/

# On the DROPLET
docker pull ghcr.io/<your-handle-lowercase>/digimon-trainer:training-v0.1

cd ~/digimon-training
docker compose -f ops/training/docker-compose.watch.yml up -d
# TB now at http://digimon-train:6006 from any tailnet member
```

## B.5 Run the trainer

```bash
# On the DROPLET (CPU-only: no --gpus flag)
docker run --rm \
  -v ~/digimon-training/runs:/app/runs \
  -v ~/digimon-training/models:/app/models \
  -v ~/digimon-training/data:/app/data \
  -v ~/digimon-training/training_jobs:/app/jobs:ro \
  ghcr.io/<owner>/digimon-trainer:training-v0.1 \
  /app/jobs/cloud_mlp_run.json
```

If you ever do put a GPU on a Hetzner/DO host (Hetzner does sell GPU instances
under a different SKU), add `--gpus all` and ensure the NVIDIA Container
Toolkit is installed:
```bash
distribution=$(. /etc/os-release; echo $ID$VERSION_ID) \
  && curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | \
     sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg \
  && curl -s -L https://nvidia.github.io/libnvidia-container/$distribution/libnvidia-container.list | \
     sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list \
  && sudo apt-get update && sudo apt-get install -y nvidia-container-toolkit \
  && sudo nvidia-ctk runtime configure --runtime=docker \
  && sudo systemctl restart docker
```

## B.6 Mirror, retrieve, tear down

```bash
# Mirror, on LAPTOP
scripts/sync_cloud_runs.sh digimon-train

# Retrieve
scp -r digimon-train:~/digimon-training/models/${RUN_ID} ./models/

# Tear down
hcloud server delete digimon-train          # Hetzner
doctl compute droplet delete digimon-train --force   # DO
```

---

# Local mitigations for VRAM pressure

When your local 12 GB card is at 11.7+ GB and you want to keep training
without going to the cloud:

1. **`export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True`** (torch ≥ 2.1).
   Cuts fragmentation; can save 200–500 MiB on long runs. Effectively free.
2. **Halve `n_steps`** (2048 → 1024). Linearly halves BPTT activation memory
   per env. Doesn't break PPO; smaller batches per update.
3. **Reduce `lstm_hidden_size`** (256 → 192). Linear savings in recurrent
   state buffers and parameters.
4. **Use `--match-format single`** for diagnostic runs to halve episode
   memory if you don't need BO3-quality eval signal.

If you've tried 1–4 and still OOM, you're at the point where the 24 GB cloud
card is the right move. The cost (~$7 for a 24h run) is a small fraction of
an hour of your time.

---

# Non-goals (deliberately not built — don't ship by accident)

- **Automatic model upload** to the hosted API on training completion.
  Curate before publishing.
- **Spot-instance resume** / SB3 checkpoint restart. Runs here are babysat;
  don't add restart complexity until the cadence justifies it.
- **Multi-pod orchestration**, parallel-run comparisons, sweep agents.
- **Domain + Let's Encrypt cert** for public TB URLs. RunPod's proxy
  (path A) and Tailscale (path B) are the documented access paths.
- **Weights & Biases** or any external experiment-tracking SaaS integration.
- **Tailscale inside the RunPod image** for stable cross-pod URLs. Possible
  (bake into `Dockerfile.training`), deferred until cadence justifies it.

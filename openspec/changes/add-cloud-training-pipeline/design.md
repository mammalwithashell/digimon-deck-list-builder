## Context

The Digimon TCG RL training pipeline runs entirely on the user's local machine today. `Dockerfile.training` exists (multi-stage Rust → PyO3 wheel → lean Python runtime) and is the wired-up entrypoint for `tools/run_training_job.py`, but it is never built in CI, never pushed to a registry, and never executed on a cloud host. `pilot_training --generalist` loads every DSL-IMPLEMENTED archetype with no way to scope down to a subset. The user's typical generalist run is ~13 hours and is GPU-light (under 30% GPU utilization), making it a candidate for cheap CPU-optimized cloud compute (Hetzner CCX23 ≈ $0.04/hr; DO CPU-Optimized ≈ $0.12/hr).

Two observation paths exist in the codebase: TensorBoard event files written under `runs/<job_id>/` by SB3, and the `digimon-training-mcp` stdio server that reads that same directory and exposes `list_runs` / `run_metric` / `run_summary` / `run_per_game_evals` to Claude sessions. Both paths are filesystem-pure — no external sinks, no DB.

The hosted API (`docker-compose.prod.yml`) is unrelated to training; it stays untouched.

The user does not own a domain.

## Goals / Non-Goals

**Goals:**
- A reproducible cloud-training cycle: push tagged image → provision droplet → `docker run` → observe → retrieve model → tear droplet down.
- A declared `allowed_archetypes` filter on the generalist deck pool that flows from `training_jobs/*.json` through `MetaGauntlet`/`load_generalist_deck_pool` to the eligible pool, intersected with the existing DSL-implemented set.
- A resolved deck-pool snapshot written to `models/<run_id>/deck_pool_snapshot.json` by default for every generalist run, so the executed pool is reviewable and re-loadable independently of `data/deck_library.json` mutations.
- Phone-checkable TensorBoard during cloud runs, with no public-internet exposure and no domain or wandb account.
- Cloud `runs/` queryable via the existing `digimon-training-mcp` from local Claude sessions, with no MCP code change.

**Non-Goals:**
- Automatic upload of trained models to the hosted API's `/admin/models/...` endpoint (manual `scp` + `upload_model.py` for now).
- Spot-instance resume / SB3 checkpoint orchestration (runs are babysat).
- Multi-droplet orchestration, parallel-run comparisons, or sweep agents.
- Domain registration, Let's Encrypt certificates, public TensorBoard URLs.
- Weights & Biases or any external experiment-tracking SaaS integration.
- Changing the hosted API stack (`docker-compose.prod.yml`).
- Touching the trainer's own runtime (the trainer container stays a one-shot `docker run`, not a long-lived compose service).

## Decisions

### 1. Filter shape: `allowed_archetypes` as a declared set, intersected with the DSL-implemented set

The new filter lives on `MetaGauntlet.__init__` as `allowed_archetypes: Optional[Set[str]] = None`. Inside `MetaGauntlet.load()`, the existing `fully_implemented_archetypes` set is the **safety floor** (no unimplemented cards ever ship to training), and `allowed_archetypes` is an additional **declared scope** intersected on top. The order is:

```
eligible_pool = (
    archetypes_in_deck_library
    ∩ fully_implemented_archetypes_from_DSL_ledger
    ∩ (allowed_archetypes if provided else "all")
)
```

`allowed_archetypes` names are canonicalized through `canonicalize_archetype()` before intersection, so a job config can use the printed/alias name ("Red Hybrid") and still match the canonical library entry ("Red Hybrid (AncientGreymon)"). Names that don't match any canonical archetype after canonicalization are logged as warnings but do not fail the run — the safety floor still applies, so the worst outcome is a smaller-than-expected pool, not an unsafe one.

**Alternative considered**: a curated commit-time snapshot file (`pools/dev.json`) and pointing `--curriculum-pool` at it. Rejected as the *primary* mechanism because it requires building and committing a snapshot artifact for every scope change, and it decouples the declared scope from the job config (the reviewable input). The snapshot mechanism still exists and is still valuable — see Decision 2 — just not as the user-facing filter.

**Alternative considered**: making the filter apply only in generalist mode. Rejected; putting it on `MetaGauntlet` lets gauntlet-mode runs scope opponent pools the same way (e.g. "train Medusamon against just the top-5 meta archetypes"). The shared surface keeps a single mental model.

### 2. Default-on snapshot for reproducibility, `runs/` for events, `models/<run_id>/` for the snapshot

Today `--curriculum-pool-out` is optional. Making it default-on for generalist runs ensures every run leaves behind the **resolved** pool, not just the declared scope, so reproduction works even if `data/deck_library.json` later changes. The snapshot lands at `models/<run_id>/deck_pool_snapshot.json` (sibling to the model `.zip`), matching the existing convention that per-run artifacts colocate with the model.

The snapshot format and hash semantics are already defined by `GeneralistDeckPool.to_snapshot()` / `from_snapshot()` (schema_version 1, content-addressed deck IDs, SHA-256 over canonical JSON). No format change is needed — only making the write unconditional.

Reproducibility contract:
- **Same `allowed_archetypes` + same `data/deck_library.json` snapshot** → same resolved snapshot hash, deterministically.
- **Same resolved snapshot hash + same `curriculum_seed`** → same deck-pair sample schedule (already guaranteed by `generalist-pilot-pretraining` spec).
- **Different `data/deck_library.json` between runs with the same declared scope** → different snapshot hashes; reload via `--curriculum-pool` against the older snapshot to reproduce exactly.

### 3. Cloud target: regime-dependent — RunPod GPU pod (primary) or Hetzner CPU droplet (CPU-only fallback)

The original decision picked Hetzner CCX23 CPU based on a `<30%` GPU utilization observation that was MLP-era. A live `nvidia-smi` sweep on an LSTM self-play run revealed the actual binding constraint is **VRAM**, not GPU compute: ~11.7 GB / 12.3 GB used on a local RTX 4070 (~95% saturation, ~300 MiB free), with GPU-util averaging ~33% and power draw flat at 24W (memory-bandwidth-bound, not compute-bound). The MLP-vs-greedy regime is genuinely env-bound and CPU is fine there, but LSTM and self-play need a GPU with ≥ 12 GB VRAM, ideally 24 GB for 2× headroom.

```
                       lstm / self-play       mlp / vs-greedy
                       ─────────────────      ───────────────
  bottleneck           VRAM (95% pinned)      env-step (Rust)
  GPU util             ~33% (spiky)           <30%
  cloud target         RunPod 3090            Hetzner CCX23
  $/hr                 ~$0.30                 ~$0.04
  24h run cost         ~$7                    ~$1
```

**Primary target — RunPod RTX 3090 Secure Cloud at $0.46/hr.** Chosen because:
- 24 GB VRAM provides 2× headroom over current local footprint, with room to scale `n_steps`, `lstm_hidden_size`, or `n_envs` without rebuying compute.
- Compute is half-used at ~33% on the local 4070, so paying for a 4090 or A100 buys speed we can't consume. 3090 ties 4090 in wall-clock at this workload and is roughly half the price.
- Per-minute billing; the image cache survives between pod recreations on the same template.

**Original decision was Community Cloud at $0.22-0.35/hr** with the rationale "preemption risk is acceptable because runs are babysat". The first end-to-end deploy invalidated that: on the Community worker we landed on, the 2.78 GB image never finished pulling (`uptime` stuck at `0s` for 8+ minutes), while the same image deployed cleanly to Secure Cloud in 2.5 minutes. The 2× price premium is the cost of "the worker can actually pull the image reliably". Path 11.3 in `tasks.md` is parked to investigate whether the CC failure was transient or systemic; if transient, we may revert to CC default later.

**Operationally, the RunPod path is different from the original Hetzner plan**: RunPod runs our published image **directly as the pod** rather than as a `docker run` invocation inside a Linux VM. The user specifies `ghcr.io/<owner>/digimon-trainer:<tag>` when creating the pod, RunPod handles GPU passthrough, and TB is reached via RunPod's built-in HTTPS proxy (`https://<pod-id>-6006.proxy.runpod.net`) rather than via Tailscale. The trainer is still one-shot per the original Decision 3 rationale, but the entrypoint is overridden to `sleep infinity` so the user can SSH in, start the trainer + TB in `tmux`, detach, and reattach later — RunPod pods don't fit the bare `docker run` lifecycle because the pod itself is the container.

**Secondary target — Hetzner CCX23 at ~$0.04/hr** is retained as the cheap path for MLP-vs-greedy runs that don't need a GPU. The original Tailscale + `docker run` flow applies unchanged for that case. DigitalOcean CPU-Optimized at ~$0.12/hr is the further fallback if the user prefers single-vendor management with the API host.

**Alternatives considered and rejected**:
- **DigitalOcean GPU droplets**: only sells H100s at this tier, $3+/hr — 7× VRAM headroom we won't use, at 10× the cost.
- **Hetzner GPU instances**: H100-class only, same problem as DO.
- **Vast.ai 3090 community**: cheaper than RunPod at $0.20/hr, but spot-style preemption is more frequent and the operator UX is rougher. Acceptable secondary; documented as a fallback in the runbook.
- **Lambda A10 (24 GB)**: dedicated, $0.75/hr — 2.5× more expensive than RunPod community 3090 for reliability we don't need (single user, babysat).
- **`Dockerfile.training` modification to bake in TB sidecar**: rejected to keep the image lean; on RunPod the user runs `tensorboard … &` inside the pod after SSH-in. On Hetzner the existing `docker-compose.watch.yml` sidecar handles it.

### 4. Watcher stack: TensorBoard sidecar via compose, sharing the trainer's `./runs/` volume

The watcher is the *opposite* of the trainer in shape: long-lived, restart-on-failure, declarative. Compose fits. `ops/training/docker-compose.watch.yml` defines one service (`tensorboard`) using the upstream `tensorflow/tensorflow:latest` image with `tensorboard --logdir /runs --bind_all --port 6006`. It mounts `./runs:/runs:ro` (read-only — the watcher cannot corrupt training output) and listens on `0.0.0.0:6006`. Started once per droplet at provisioning time, survives trainer restarts.

**Why not `--bind_all` to 0.0.0.0 without a firewall?** Because the droplet's firewall (Hetzner Cloud Firewall / DO Cloud Firewall) blocks all inbound except SSH at the cloud-network layer; the Tailscale daemon provides the only path to `:6006`. Defense in depth.

### 5. Network: regime-dependent — RunPod proxy (Path A) or Tailscale (Path B)

The original decision picked Tailscale uniformly. With the GPU pivot (Decision 3) the access path also bifurcates.

**Path A — RunPod**: each pod gets a built-in HTTPS proxy at `https://<pod-id>-<port>.proxy.runpod.net` plus an SSH proxy at `<pod-id>.proxy.runpod.net:<port>`. Both are TLS-grade, account-gated, and require no setup beyond pod creation. Tradeoff: the URL is per-pod, not per-tailnet, so the bookmark breaks every time the user terminates and re-creates a pod. Accepted for v1 because (a) cadence is low (one pod per long run), (b) RunPod's pod page UI surfaces the URL prominently on each provision, and (c) the alternative — baking Tailscale into `Dockerfile.training` — bloats the image and complicates pod startup with a `tailscale up --authkey=...` step inside the container.

**Path B — Hetzner/DO**: Tailscale on the droplet, unchanged from the original decision. Without a domain, realistic alternatives for phone-checkable TLS were: (a) Cloudflare Quick Tunnel (ephemeral URLs, broken bookmarks), (b) Caddy with a self-signed cert (phones refuse), (c) plain HTTP + basic auth over the public internet (passwords in clear), (d) Tailscale. Tailscale wins on all three constraints: zero cost, no domain, encrypted by default. The training host gets a stable MagicDNS name like `train-box.<tailnet>.ts.net`; the URL survives droplet teardowns when the next droplet joins the tailnet with the same node name.

**Alternatives considered and rejected for Path A**:
- **Tailscale on RunPod via baked-in image install**: viable but defers to Decision 5b when cadence justifies it. Bloats the image and complicates pod startup with a `tailscale up --authkey=...` step. Logged as a non-goal.
- **Cloudflare Tunnel with free `*.trycloudflare.com` URL**: rejected for the same per-session-URL-rotation reason as on Path B.
- **Domain + Caddy + Let's Encrypt fronted by the API host**: rejected because the user doesn't own a domain and the dual-host Caddy hop adds operational complexity for no end-user-visible benefit.

### 6. MCP integration via rsync mirror into a single `runs/` tree

The existing `digimon-training-mcp` reads `runs/` from the local filesystem. To make cloud runs queryable in Claude sessions, the local `runs/` directory becomes a *mirror* of both local and cloud runs. The cloud-side mirror happens via `scripts/sync_cloud_runs.sh`, a thin rsync wrapper:

```
rsync -az --delete-after \
  <user>@<train-box>:~/digimon-training/runs/ \
  ./runs/
```

Cron snippet in the runbook for the laptop: every 5 minutes while a cloud run is active. The lag is acceptable because MCP queries are advisory ("how's the run trending?"), not realtime.

**Collision avoidance**: cloud runs prefix their `output.name` with `cloud_` in the job config (enforced as a convention in the runbook, not in code). Cloud and local `runs/` directory entries coexist; `list_runs` returns them sorted by `last_modified`, so the active one floats to the top regardless of origin.

**Alternative considered**: separate `runs-cloud/` directory + restarting the MCP server with `--runs-dir ./runs-cloud/` per query. Rejected: the explicit value of the merge is cross-run comparison ("how does this cloud run compare to my local baseline?"), which is exactly what `digimon-training-mcp` is good at. Two trees defeats the purpose.

**Alternative considered**: running `digimon-training-mcp` directly on the cloud host. Rejected: MCP transport in this codebase is local stdio, and the rsync mirror gets you the same answers without an extra network protocol.

### 7. Image publish: GHCR via GitHub Actions on tag push

`Dockerfile.training` is already production-shaped; the missing piece is the build automation. The workflow triggers on git tags matching `training-v*` to keep training-image releases decoupled from API-image releases, and publishes to `ghcr.io/<repo-owner>/digimon-trainer:<tag>`. The workflow runs a `--dry-run` against `training_jobs/example_boardwalk_medusamon.json` to catch obvious breakage (missing card data, broken DSL ledger, bad gauntlet config) before the image ships.

**Alternative considered**: push on every `main` commit. Rejected: training images are heavy (~2 GB) and the user wants explicit promotion. Tagged releases match the "manual model handoff" preference.

## Risks / Trade-offs

- **Risk**: Tailscale dependency becomes a single point of failure for both TB and MCP-mirror. → **Mitigation**: SSH access remains separate (cloud firewall allows port 22 from anywhere or from the user's home IP). Worst case, `rsync` over plain SSH still works and the user can `ssh -L 6006:localhost:6006` for emergency TB access.

- **Risk**: rsync mirror diverges from the live cloud `runs/` and the user makes a decision off stale numbers. → **Mitigation**: the runbook recommends a 5-minute cron and the MCP's `list_runs` returns `last_modified` so Claude (and the user) can see the freshness explicitly. For high-stakes glances, the runbook says "open TB instead of asking via MCP."

- **Risk**: `cloud_` prefix collision discipline lapses, and a local `cloud_foo` shadows a cloud `cloud_foo`. → **Mitigation**: low impact (the merge is filesystem-level, so the more-recently-rsync'd one wins for shared keys, and the older one stays visible under its own job_id if the names actually differ). The runbook explicitly calls this out.

- **Risk**: GHCR image size + Hetzner bandwidth costs on first pull. → **Mitigation**: the Dockerfile is already two-stage; the runtime image is the lean stage (no Rust toolchain). Estimated ~600 MB. Hetzner includes 20 TB/month outbound on CCX-class, well above the pull cost.

- **Risk**: An `allowed_archetypes` typo silently produces a tiny pool the user doesn't notice until eval. → **Mitigation**: `pilot_training` logs the resolved archetype count and the dropped-because-not-recognized names at startup; the existing snapshot write makes the actual pool inspectable in `models/<run_id>/deck_pool_snapshot.json` before training is far along.

- **Trade-off**: requiring Tailscale on the user's phone is one more app to manage. Accepted because it's free, low-friction (one-time setup), and the alternative is buying a domain.

- **Trade-off**: TB sidecar reads `./runs/` read-only — if SB3 ever needs to write back to that directory (it doesn't today; it only writes), this would break. Accepted; SB3's TB logging is append-only.

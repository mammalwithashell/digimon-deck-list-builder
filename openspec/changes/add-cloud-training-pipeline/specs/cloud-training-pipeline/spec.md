## ADDED Requirements

### Requirement: Training image is published from CI on tagged releases

The repository SHALL include a GitHub Actions workflow that builds `Dockerfile.training` and pushes the resulting image to `ghcr.io/<repo-owner>/digimon-trainer` on git tags matching `training-v*`. The published image SHALL be tagged with both the git tag (e.g. `training-v0.1`) and `latest`. The workflow SHALL fail the build if a smoke `--dry-run` against `training_jobs/example_boardwalk_medusamon.json` exits non-zero inside the freshly built image.

#### Scenario: Tag push publishes a registry image

- **WHEN** a git tag matching `training-v*` is pushed to the repository
- **THEN** the workflow builds `Dockerfile.training` and publishes `ghcr.io/<repo-owner>/digimon-trainer:<tag>` and `ghcr.io/<repo-owner>/digimon-trainer:latest`
- **AND** the registry image accepts `docker pull` for the pushed tag

#### Scenario: Smoke dry-run blocks a broken image from publishing

- **WHEN** the freshly built image fails to execute `tools/run_training_job.py training_jobs/example_boardwalk_medusamon.json --dry-run`
- **THEN** the workflow exits non-zero before any image is pushed to `ghcr.io`
- **AND** the workflow logs identify the failing command and its exit code

#### Scenario: Untagged commits do not publish

- **WHEN** a commit lands on `main` without a matching `training-v*` tag
- **THEN** the workflow does not push any image to `ghcr.io`

### Requirement: Trainer container runs as a one-shot docker invocation

The published image SHALL execute as `docker run --rm -v <host_runs>:/app/runs -v <host_models>:/app/models -v <host_data>:/app/data -v <host_jobs>:/app/jobs ghcr.io/<owner>/digimon-trainer:<tag> /app/jobs/<config>.json`. The image SHALL NOT be packaged as a long-lived compose service in any infrastructure file shipped with the repository. The container SHALL exit non-zero on training failure so the host operator observes loud failure rather than a silent restart loop.

#### Scenario: One-shot run completes and exits

- **WHEN** the user runs the documented `docker run` command against a valid job config
- **THEN** the container runs the job to completion and exits with code 0
- **AND** the host `runs/` and `models/` directories contain the run's TensorBoard events, eval sidecar, model `.zip`, and curriculum-pool snapshot

#### Scenario: No compose service wraps the trainer

- **WHEN** a contributor reads the infrastructure files under `ops/training/`
- **THEN** no `docker-compose*.yml` defines the trainer as a service with `restart:` other than `no`

### Requirement: Watcher stack serves TensorBoard read-only over the trainer's runs volume

The repository SHALL ship `ops/training/docker-compose.watch.yml`, defining a single `tensorboard` service that mounts the shared `./runs/` directory **read-only** at `/runs` and listens on `0.0.0.0:6006`. The service SHALL be declared with `restart: unless-stopped` so it survives droplet reboots. The watcher SHALL NOT depend on the trainer container or block the trainer's lifecycle in either direction.

#### Scenario: Watcher exposes TB on tailnet interface

- **WHEN** the watcher stack is brought up with `docker compose -f ops/training/docker-compose.watch.yml up -d` on a host joined to the user's tailnet
- **THEN** the TensorBoard UI is reachable at `http://<host-magicdns-name>:6006` from any other tailnet member
- **AND** the response shows scalars from the most recent or active run in `./runs/`

#### Scenario: Watcher cannot mutate runs

- **WHEN** the watcher container attempts to write into `/runs`
- **THEN** the operation fails because the mount is read-only
- **AND** the trainer's writes to the same host directory are unaffected

#### Scenario: Trainer restart does not affect watcher

- **WHEN** the trainer container is stopped and restarted while the watcher is running
- **THEN** the watcher continues serving TensorBoard without restart

### Requirement: Network access path is regime-aware and never requires a public TLS exposure

The cloud runbook SHALL document at least one supported access mechanism per provisioning regime for both human (TensorBoard) and operator (rsync mirror) traffic to the training host. For RunPod pods the supported mechanism SHALL be RunPod's built-in HTTPS / SSH proxy. For Hetzner / DigitalOcean droplets the supported mechanism SHALL be Tailscale. The runbook SHALL NOT instruct the user to expose port 6006 to the public internet, to obtain a Let's Encrypt certificate, or to provision a domain.

#### Scenario: RunPod path uses the platform's own HTTPS proxy

- **WHEN** a contributor reads the RunPod section of `docs/CLOUD_TRAINING.md`
- **THEN** TensorBoard access goes through the `https://<pod-id>-<port>.proxy.runpod.net` URL surfaced on the pod's Connect tab
- **AND** the rsync mirror invocation uses the pod's SSH proxy (`<pod-id>.proxy.runpod.net:<port>`)
- **AND** no step instructs the user to install Tailscale on the pod

#### Scenario: Hetzner / DigitalOcean path uses Tailscale

- **WHEN** a contributor reads the Hetzner / DigitalOcean section of `docs/CLOUD_TRAINING.md`
- **THEN** the runbook contains a copy-pastable shell snippet that installs Tailscale, runs `tailscale up` with an ephemeral auth key, and verifies the host's MagicDNS name
- **AND** the snippet does not open inbound firewall rules for ports other than SSH

#### Scenario: Runbook does not require a domain on any path

- **WHEN** a contributor follows the runbook end-to-end on either path
- **THEN** no step requires owning or pointing a DNS record
- **AND** no step requires obtaining a Let's Encrypt certificate

### Requirement: Cloud runs mirror into the local runs directory for MCP queries

The repository SHALL include `scripts/sync_cloud_runs.sh`, an rsync wrapper that pulls the training host's `runs/` directory into the user's local `runs/` directory. The wrapper SHALL support both the Tailscale-named-host transport (Path B) and the RunPod SSH-proxy transport (Path A) via a `DIGIMON_REMOTE_PORT` environment variable that, when set, threads a custom SSH port through to rsync. The remote runs directory SHALL be configurable via `DIGIMON_REMOTE_RUNS` to accommodate both `~/digimon-training/runs/` (Path B) and `/workspace/runs/` (Path A). The runbook SHALL document a cron snippet for invoking the wrapper periodically while a cloud run is active, and SHALL specify that cloud-run job configs use a `cloud_` prefix on the `output.name` so cloud and local run directories coexist without filename collisions. After mirroring, the existing `digimon-training-mcp` (queried with no extra flags) SHALL surface both cloud and local runs through `list_runs`, `run_summary`, `run_metric`, and `run_per_game_evals`.

#### Scenario: Sync wrapper pulls runs over the tailnet (Path B)

- **WHEN** the user runs `scripts/sync_cloud_runs.sh <tailnet-host>` after a cloud run has produced files under `~/digimon-training/runs/<job_id>/` on the training host
- **THEN** the local `runs/<job_id>/` directory contains the same TensorBoard events, eval sidecar, and curriculum-pool snapshot as the remote
- **AND** subsequent invocations transfer only changed files

#### Scenario: Sync wrapper pulls runs through the RunPod SSH proxy (Path A)

- **WHEN** the user runs `DIGIMON_REMOTE_PORT=<port> DIGIMON_REMOTE_RUNS=/workspace/runs/ scripts/sync_cloud_runs.sh root@<pod>.proxy.runpod.net`
- **THEN** rsync uses the `ssh -p <port>` transport and pulls from the pod's persistent volume mount
- **AND** the local `runs/<job_id>/` directory contains the same artifacts as the remote

#### Scenario: MCP surfaces a mirrored cloud run

- **WHEN** the user runs `scripts/sync_cloud_runs.sh` and then asks the `digimon-training-mcp` to `list_runs`
- **THEN** the returned list contains the mirrored cloud run with a populated `latest_step` and `latest_win_rate`
- **AND** the `last_modified` field reflects the rsync time, allowing the caller to detect mirror lag

#### Scenario: Cloud and local runs coexist

- **WHEN** the local `runs/` directory holds both a local run and a `cloud_` -prefixed mirrored cloud run
- **THEN** `list_runs` returns both entries
- **AND** their names do not collide

### Requirement: Cloud training runbook documents the end-to-end cycle for both GPU and CPU regimes

The repository SHALL include `docs/CLOUD_TRAINING.md` covering two parallel provisioning paths, with a decision section at the top routing the reader to the right one based on workload:

- **Path A (GPU runs — LSTM, self-play, VRAM-bound):** RunPod pod creation with a custom `ghcr.io/<owner>/digimon-trainer` image, the `Expose HTTP Port: 6006` configuration, GPU passthrough verification (`nvidia-smi` + `torch.cuda.is_available()`), data staging via rsync over RunPod's SSH proxy, manual trainer + TensorBoard startup inside the pod, mirror via the SSH proxy, model retrieval via `scp`, and pod termination.
- **Path B (CPU runs — MLP-vs-greedy, env-bound):** Hetzner CCX-class or DigitalOcean CPU-Optimized droplet provisioning, Tailscale install and tailnet join, Docker install, image pull from `ghcr.io`, the `docker run` invocation for the trainer, bringing up the watcher stack, the rsync mirror cron snippet, retrieving the model artifact, and tearing the droplet down.

The runbook SHALL include a Local Mitigations section listing torch environment variables and PPO hyperparameter levers (`PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True`, halving `n_steps`, reducing `lstm_hidden_size`) that buy local VRAM headroom before cloud is necessary. The runbook SHALL cross-link from `docs/TRAINING_RUNBOOK.md`.

#### Scenario: Runbook covers both paths with copy-pastable commands

- **WHEN** a contributor reads `docs/CLOUD_TRAINING.md` end-to-end
- **THEN** every step from provisioning through teardown is documented for both Path A (RunPod) and Path B (Hetzner / DigitalOcean) with copy-pastable commands
- **AND** Path A names RTX 3090 community cloud as the default GPU choice with RTX A5000 / RTX A6000 documented as upgrade options
- **AND** Path B names both Hetzner CCX23 and DigitalOcean CPU-Optimized as supported targets

#### Scenario: Decision section routes readers to the right path

- **WHEN** a contributor opens `docs/CLOUD_TRAINING.md`
- **THEN** an early section explains that LSTM / self-play / VRAM-bound runs go to Path A and MLP-vs-greedy / env-bound runs go to Path B
- **AND** the section gives the recommended host and approximate 24-hour run cost for each path

#### Scenario: Local mitigations precede the cloud recommendation

- **WHEN** a contributor reads `docs/CLOUD_TRAINING.md` while hitting local VRAM pressure
- **THEN** the runbook surfaces `PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True`, halving `n_steps`, and reducing `lstm_hidden_size` as no-cost mitigations to try before provisioning a cloud pod

#### Scenario: Runbook is reachable from the main training runbook

- **WHEN** a contributor reads `docs/TRAINING_RUNBOOK.md`
- **THEN** they find a link to `docs/CLOUD_TRAINING.md`
- **AND** the cross-link explains when to read the cloud runbook (long runs, off-machine training, local VRAM ceilings)

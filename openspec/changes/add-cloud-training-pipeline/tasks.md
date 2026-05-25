## 1. Archetype filter (code)

- [x] 1.1 Add `allowed_archetypes: Optional[Set[str]] = None` parameter to `MetaGauntlet.__init__` in `code/digimon_gym/agents/gauntlet.py`; persist as `self._allowed_archetypes`.
- [x] 1.2 In `MetaGauntlet.load()`, after computing `fully_implemented_archetypes`, intersect with the canonicalized `_allowed_archetypes` set when provided; log a warning naming any entries that did not canonicalize to a known archetype; log an info line for entries dropped because not DSL-implemented.
- [x] 1.3 Thread `allowed_archetypes` through `load_generalist_deck_pool()` so it forwards into the `MetaGauntlet` it constructs.
- [x] 1.4 Add an `--archetypes` CLI flag to `pilot_training` (comma-separated list); parse into a set; pass to `load_generalist_deck_pool()` and, for gauntlet-mode runs, to the `MetaGauntlet` constructor in `tools/run_training_job.py`.
- [x] 1.5 Add an `allowed_archetypes` field to the training-job JSON schema (top-level or under `training`); plumb it through `tools/run_training_job.py` into both `build_gauntlet` and the generalist pool loader.
- [x] 1.6 In `pilot_training` startup logging, print the resolved eligible archetype count and the dropped-because-not-recognized names so a typo in `allowed_archetypes` is visible immediately.

## 2. Snapshot default-on for generalist runs

- [x] 2.1 Audit the current `pilot_training` generalist path — confirmed default-on at `pilot_training.py:1718-1724`: snapshot writes to `<run_dir>/deck_pool_snapshot.json` whenever neither `--curriculum-pool` nor `--curriculum-pool-out` is supplied. No code change needed.
- [x] 2.2 Verify the snapshot record's `eligible_archetypes` array reflects the resolved (post-`allowed_archetypes`-intersection) set in canonical form — covered by `test_snapshot_roundtrip_preserves_filtered_pool` and `GeneralistDeckPool.to_snapshot()` building from the already-filtered `self.archetypes`.
- [x] 2.3 Verify `GeneralistDeckPool.from_snapshot()` continues to reproduce the snapshot's resolved pool regardless of subsequent changes to `data/deck_library.json` or the DSL ledger — covered by `test_snapshot_roundtrip_preserves_filtered_pool`, which mutates the library between write and read.

## 3. Tests

- [x] 3.1 Unit test in `code/tests/rl/` covering `MetaGauntlet` with `allowed_archetypes={"Rocks", "Yellow Hybrid"}` against a fixture deck library; assert the eligible pool intersects correctly. → `test_filters_pool_to_declared_subset`
- [x] 3.2 Unit test covering alias canonicalization: `allowed_archetypes={"Red Hybrid"}` resolves to the canonical `"Red Hybrid (AncientGreymon)"` entry. → `test_alias_canonicalizes_to_library_entry` (uses real alias "RockClose" → "Rocks").
- [x] 3.3 Unit test covering the safety floor: `allowed_archetypes={"NotImplementedArchetype"}` produces an empty pool with a logged warning rather than the full pool. → `test_unrecognized_archetype_logs_warning_and_continues` + `test_safety_floor_overrides_allowed` + `test_allowed_archetypes_empty_set_produces_empty_pool`.
- [x] 3.4 Unit test covering CLI ↔ job-config parity: same `allowed_archetypes` via flag and via JSON produces identical resolved pools. → `test_yaml_allowed_archetypes_loads_as_list` + `test_cli_archetypes_flag_matches_yaml` + `test_cli_archetypes_overrides_yaml`.
- [x] 3.5 Snapshot round-trip test: write a snapshot with a filtered pool, reload via `from_snapshot()`, assert eligible archetypes and decks match exactly even when the underlying `data/deck_library.json` fixture is mutated between write and read. → `test_snapshot_roundtrip_preserves_filtered_pool`.
- [x] 3.6 Gauntlet-mode test confirming `allowed_archetypes` restricts opponent sampling. → `test_gauntlet_opponent_sampling_honors_filter`.

## 4. Training image CI

- [x] 4.1 Add `.github/workflows/training-image.yml` triggered on tags matching `training-v*`; build `Dockerfile.training`; tag with `<git-tag>` and `latest`.
- [x] 4.2 In the same workflow, run a smoke `--dry-run` step inside the built image against `training_jobs/_smoke.json` (newly added — uses the ST1 default deck so it has no external file dependency); fail the workflow if it exits non-zero.
- [x] 4.3 On success, push both tags to `ghcr.io/<repo-owner>/digimon-trainer` via `docker/build-push-action@v6`.
- [x] 4.4 Workflow is `on: push: tags: [training-v*]` only — un-tagged `main` pushes do not trigger it.
- [x] 4.5 Tag convention `training-v0.1` / `training-v0.2` documented in `docs/CLOUD_TRAINING.md` §4.

## 5. Watcher stack

- [x] 5.1 Added `ops/training/docker-compose.watch.yml` with single `tensorboard` service using `tensorflow/tensorflow:latest`, `command: [tensorboard, --logdir, /runs, --bind_all, --port, "6006"]`, `./runs:/runs:ro`, `restart: unless-stopped`, ports `6006:6006`.
- [x] 5.2 Added `ops/training/README.md` explaining: watcher reads `./runs/` read-only, started independently of the trainer container, reachable over Tailscale only, cloud firewall blocks public `:6006`.
- [x] 5.3 Confirmed by inspection: `ops/training/docker-compose.watch.yml` contains only the `tensorboard` service; no trainer service is declared in any compose file under `ops/training/`.

## 6. Tailscale + mirror

- [x] 6.1 Added `scripts/sync_cloud_runs.sh`: rsync wrapper taking `<train-host>` as first arg, pulling `~/digimon-training/runs/` into `./runs/` with `-az --delete-after --partial`. Supports `DIGIMON_REMOTE_RUNS` / `DIGIMON_LOCAL_RUNS` env overrides.
- [x] 6.2 Recommended cron snippet (`*/5 * * * *` while a cloud run is active) documented in `docs/CLOUD_TRAINING.md` §7 and in the script's own header comment.
- [x] 6.3 `cloud_` prefix convention for `output.name` documented in `docs/CLOUD_TRAINING.md` §3 and §6. (No code-side enforcement — kept lightweight per task scope.)
- [ ] 6.4 **User action required**: After a real cloud run + mirror sync, confirm `digimon-training-mcp list_runs` surfaces the mirrored cloud run with populated `latest_step` and `latest_win_rate`. Defer until Task 8.4 — same dependency on a live droplet.

## 7. Cloud runbook

- [x] 7.1 Created `docs/CLOUD_TRAINING.md` with sections: Why these choices?, Prerequisites, Provisioning (Hetzner CCX23 default + DigitalOcean CPU-Optimized fallback), Bootstrap (Tailscale install + tailnet join, Docker install), Stage data, Pull image, Watcher up, Trainer `docker run`, Mirror runs/, Model retrieval, Teardown.
- [x] 7.2 Each step has copy-pastable shell snippets. **End-to-end verification against a fresh droplet (Task 8.4) is the user action that signs off on these snippets working in the wild.**
- [x] 7.3 "Why these choices?" sidebar at top of runbook references the design.md decisions (no domain, no compose for trainer, Tailscale over Caddy, CPU over GPU, Hetzner default).
- [x] 7.4 Cross-linked from `docs/TRAINING_RUNBOOK.md` via a top-of-file pointer block explaining when to use `CLOUD_TRAINING.md`.
- [x] 7.5 Non-goals section at the bottom of `docs/CLOUD_TRAINING.md` calls out: no auto model upload, no spot resume, no multi-droplet, no domain/LE, no wandb.

## 8. Verification

- [x] 8.1 `python -m pytest code/tests/rl -q` → 316 passed in 440.60s. All 11 new tests (8 filter + 3 config-parity) green; no regressions in the existing 305.
- [x] 8.2 `PYTHONPATH=code python code/tools/run_training_job.py training_jobs/_smoke.json --dry-run` → exits 0, prints expected meta-scope/output lines, then "[dry-run] Skipping training." (The pre-existing `example_boardwalk_medusamon.json` config still depends on a missing `decks/medusamon_list.json` file — orthogonal to this change; that's why `_smoke.json` exists.)
- [ ] 8.3 **User action required**: Local Docker build. Requires Docker daemon, not available in this worktree env. Command: `docker build -f Dockerfile.training -t digimon-trainer:dev .` then `docker run --rm -v $(pwd)/data:/app/data -v $(pwd)/training_jobs:/app/jobs digimon-trainer:dev /app/jobs/_smoke.json --dry-run` should exit 0.
- [ ] 8.4 **User action required**: Provision one Hetzner droplet, follow `docs/CLOUD_TRAINING.md` end-to-end with a small generalist job (10k timesteps + `allowed_archetypes=["Rocks"]`). Confirm: trainer exits 0, TB reachable over tailnet, `scripts/sync_cloud_runs.sh` populates local `runs/`, `digimon-training-mcp list_runs` surfaces the mirrored run, `scp` retrieves the model.
- [ ] 8.5 **Conditional on 8.4**: Update this `tasks.md` with any follow-ups discovered during the live smoke. (E.g., snippet typos, missed permissions, GHCR auth surprises.)

## 9. GPU-pivot follow-ups (post-`nvidia-smi` reality check)

A live `nvidia-smi` sweep on the user's LSTM self-play run revealed VRAM (not GPU compute) is the binding constraint — ~11.7 GB / 12.3 GB pinned. The original CPU-droplet recommendation was MLP-era and wrong for the user's current workload. The decisions in §3 and §5 of `design.md` were updated to reflect a regime-aware split: RunPod RTX 3090 for GPU runs (Path A), Hetzner CCX23 retained for CPU-only runs (Path B).

- [x] 9.1 Update `docs/CLOUD_TRAINING.md` with a top-level Decision section, a Path A (RunPod) section, a Path B (Hetzner/DO, unchanged from original) section, a Local Mitigations section (`PYTORCH_CUDA_ALLOC_CONF`, halve `n_steps`, reduce `lstm_hidden_size`), and updated Non-goals.
- [x] 9.2 Update `scripts/sync_cloud_runs.sh` to accept `DIGIMON_REMOTE_PORT` (threads to `rsync -e "ssh -p ..."`) so the same wrapper drives both the Tailscale and RunPod transports. Default remote runs dir documented as `~/digimon-training/runs/` (Path B) or `/workspace/runs/` (Path A).
- [x] 9.3 Update `design.md` Decision 3 (cloud target) to record the regime split and the rejection rationale for Lambda / Vast.ai / DO-GPU / Hetzner-GPU under this workload.
- [x] 9.4 Update `design.md` Decision 5 (network) to record the RunPod-HTTPS-proxy access path for Path A alongside the existing Tailscale access path for Path B.
- [x] 9.5 Update `specs/cloud-training-pipeline/spec.md` access-path requirement and runbook requirement to be regime-aware. (Original "Tailscale is the documented access path" widened to "Network access path is regime-aware and never requires a public TLS exposure"; original "runbook covers the full cycle" widened to cover both paths + Local Mitigations + the routing Decision section.)
- [x] 9.6 Update `proposal.md` Why and What-Changes blurbs to reflect the GPU pivot.
- [ ] 9.7 **User action required**: On the first real RunPod smoke (combined with Task 8.4), verify the trainer image's PyTorch wheel sees CUDA inside a RunPod pod. Inside the pod after SSH: `python -c "import torch; print(torch.cuda.is_available(), torch.cuda.get_device_name(0) if torch.cuda.is_available() else None)"` — expected output is `True 'NVIDIA GeForce RTX 3090'`. If `False`, the `python:3.11-slim` base + default `torch>=2.0` wheel is not CUDA-enabled in this configuration and `Dockerfile.training` needs a base-image change (e.g. `nvidia/cuda:12.x-runtime-ubuntu22.04` + manual Python install) or a torch pin to a CUDA-tagged build. Defer image change until smoke confirms it's needed.
- [ ] 9.8 **Conditional on 9.7**: If torch ends up CPU-only inside the RunPod pod, file a follow-up tasks.md entry to swap `Dockerfile.training`'s base image and re-publish (`training-v0.2`). Don't pre-emptively change the base; verify the actual behavior first.

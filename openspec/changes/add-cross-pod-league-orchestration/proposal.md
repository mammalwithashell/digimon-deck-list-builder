## Why

The deck-specialist league (`code/tools/train_specialist_league.py`) runs on a **single box**: it trains all decks' specialists per round, sequentially or in-pod-parallel, on one host. A 2026-06-30 investigation established that **in-pod parallel (`--topology parallel`) is reliable but does not reliably improve throughput** — and that **RunPod containers are CPU-cgroup-throttled** (a pod reporting `nproc=96` was capped at ~7.65 effective cores via `cpu.max`), which confounds in-box scaling entirely. The scalable, dependable path is **one specialist per pod, fanned out across many pods**. That requires orchestration the codebase does not have today.

## What Changes

- **Tier 1 — embarrassingly-parallel fan-out (`league-pod-fanout`)**: a launcher that provisions N pods, runs **one deck per pod** against a **fixed** opponent set (the MLP generalist + a frozen champion pool — no cross-deck evolving pool, so **no per-round barrier**), and auto-harvests each pod's `final.zip` before teardown. Trains all decks concurrently with ~N× speedup and minimal coordination.
- **Tier 2 — true distributed league (`distributed-league-orchestration`)**: keeps the evolving cross-deck PFSP pool. Adds a **shared artifact store** (registry + checkpoints + round-pool manifests), an **outer orchestrator** that drives rounds (reusing the existing `build_specialist_argv` / `write_round_pool` / `_barrier` building blocks), **per-pod dispatch + completion detection**, and a **per-round barrier** (round *k+1*'s pool = all decks' round-*k* specialists, so every deck must finish round *k* before any starts *k+1*).
- **Pod provisioning contract (`training-pod-provisioning`)**: a reusable provision/verify/harvest layer encoding this session's hard-won constraints (effective-core selection via `cpu.max`, memory headroom, A40-volume avoidance, harvest-before-terminate).
- **Runbook updates** (not spec, done alongside): fold the lessons (CPU-throttle confound, in-pod-parallel verdict, league-specialist gotchas) into the training runbooks.

This is **additive** — the single-box driver and `--topology parallel` path are unchanged; fan-out is a new layer on top.

## Capabilities

### New Capabilities
- `training-pod-provisioning`: Provision and **verify** cloud training pods by *effective* resources (`cpu.max` quota, not host `nproc`; RAM cgroup headroom), with image/data staging, `--terminate-after` guards, and a harvest-before-terminate contract (no reliable persistent volume).
- `league-pod-fanout`: Tier-1 one-deck-per-pod launcher + harvester against a fixed champion pool — barrier-free, ~N× parallel, the recommended default for parallelizing a league.
- `distributed-league-orchestration`: Tier-2 outer orchestrator for the full evolving cross-deck league — shared artifact store, per-pod round dispatch, and a per-round barrier, reusing the single-box driver's pool/barrier primitives.

### Modified Capabilities
<!-- None: the single-box league driver and in-pod parallel path are unchanged; this is a new orchestration layer. -->

## Impact

- **New code** (no existing files modified for the core): a fan-out launcher + orchestrator under `code/tools/` (e.g., `league_fanout.py`, `league_orchestrator.py`) reusing `train_specialist_league.py` primitives (`build_specialist_argv`, `write_round_pool`, `_barrier`, `SpecialistRegistry`) and the `champion_admin.py` champion pool.
- **Shared storage**: Tier 2 depends on a shared artifact store — reuse `code/server/storage/` adapters (object storage / Spaces) for registry + checkpoints + round-pool manifests.
- **Pod control plane**: dispatch via `runpodctl` (provision/SSH) or the RunPod API; an orchestrator runs on a durable control node (laptop / small VM), since training pods are ephemeral.
- **Docs**: `docs/CLOUD_TRAINING.md` and `docs/TRAINING_RUNBOOK.md` (and `docs/runbooks/`) updated with the provisioning contract and this session's lessons.
- **No runtime/engine changes**; no new training-image requirement beyond the existing `training-v0.46` (cu128 + LSTM league fixes).

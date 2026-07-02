## Context

The deck-specialist league driver (`code/tools/train_specialist_league.py`) is single-host. It seeds a `SpecialistRegistry`, then for each round trains every deck's specialist (sequential via `_run`, or in-pod-parallel via `_run_parallel`) against a frozen round pool emitted by `SpecialistRegistry.write_round_pool`, and snapshots results back at a per-round `_barrier`. Round *k+1*'s pool for **every** deck is **all decks'** round-*k* specialists (PFSP) — a cross-deck dependency that forces a per-round barrier.

A 2026-06-30 investigation tried to scale *inside* one pod (`--topology parallel`). Findings that shape this design:
- **In-pod parallel is reliable but its throughput benefit is unproven.** Four real bugs were fixed (opponents on GPU per worker → CUDA-load collision; OMP/MKL/rayon thread storm; simultaneous-spawn collision; unbounded per-worker opponent cache → cgroup OOM). But all throughput numbers were **confounded by CPU-cgroup throttling**.
- **RunPod containers are CPU-throttled and `nproc` lies.** A pod reporting `nproc=96` had `cpu.max = 765000/100000` = **7.65 effective cores**; `cpu.stat` showed massive `throttled_usec`. RunPod's `vcpuCount` reflects the *real* quota. Cross-pod throughput comparisons were therefore apples-to-oranges.
- **Conclusion:** the dependable, scalable path is **one specialist per pod** (each gets a full, uncapped box), fanned out across pods.

## Goals / Non-Goals

**Goals:**
- Parallelize a league across pods with ~N× wall-clock, reliably.
- Encode the hard-won pod-provisioning constraints so throughput isn't silently throttled.
- Reuse the single-box driver's primitives (`build_specialist_argv`, `write_round_pool`, `_barrier`, `SpecialistRegistry`) rather than reimplementing league logic.
- Two tiers: a simple barrier-free fan-out (Tier 1) and the full evolving-pool league (Tier 2).

**Non-Goals:**
- Changing the single-box driver's behavior or the `--topology parallel` path (left as-is; documented as not-a-speedup).
- A general-purpose distributed scheduler — this is league-specific orchestration.
- Multi-GPU-per-pod or NVIDIA MPS exploration (separate effort).
- Solving in-pod parallel throughput (explicitly abandoned in favor of per-pod).

## Decisions

**D1 — Two tiers, not one.** Tier 1 (barrier-free, one deck per pod vs a *fixed* champion pool) and Tier 2 (full evolving cross-deck pool with a per-round barrier).
- *Why:* the per-round barrier + shared registry is the only hard part; ~all of its complexity comes from the *evolving* pool. Relaxing the pool to a fixed champion set makes decks independent → embarrassingly parallel → ships fast and delivers most of the value. Tier 2 layers on when the evolving-pool dynamic is actually wanted.
- *Alternative considered:* only build Tier 2. Rejected — far more infra (shared store + barrier) before any payoff.

**D2 — Provision/verify pods by *effective cores* (`cpu.max`), never `nproc`.** The provisioning layer reads `cpu.max` (effective = quota ÷ period) inside the container and rejects/re-rolls pods below a configurable floor; it also surfaces `cpu.stat` throttling and RAM cgroup headroom.
- *Why:* `nproc` shows host cores; the container quota can be ~8 cores behind a 96-core `nproc`, silently throttling training. This confounded the whole in-pod investigation.
- *Alternative:* trust `runpodctl vcpuCount` only. Acceptable as a pre-filter (it ≈ the quota), but verify in-container `cpu.max` post-boot to be certain.

**D3 — Harvest-before-terminate; treat pods as ephemeral with no reliable volume.** Each pod's artifacts (`final.zip` + meta + registry deltas) are pulled to durable storage as soon as produced; an auto-`--terminate-after` guard caps idle billing.
- *Why:* A40-secure pods **hang provisioning when a `--volume-in-gb` is attached** (observed repeatedly), so container-disk-only is the reliable config — which means artifacts are lost on terminate unless harvested. No-volume also dodges that provisioning hang.

**D4 — Tier 2 state lives in a shared artifact store; the orchestrator is the single writer of the registry.** Registry + checkpoints + round-pool manifests go to object storage (reuse `code/server/storage/` adapters). Per-pod jobs read the round pool from the store and write their checkpoint back; the orchestrator runs the `_barrier` (registry update) centrally.
- *Why:* the single-box driver assumes local disk; pods don't share disk. Centralizing the barrier write avoids registry races.
- *Alternative:* shared network volume (NFS) mounted on all pods. Simpler conceptually but fragile/slow on RunPod and ties pods together; object storage decouples them.

**D5 — Orchestrator on a durable control node; pods dispatched, not self-coordinating.** A long-lived process (laptop / small VM) drives rounds: emit pools → dispatch one deck per pod → wait-for-all → harvest + barrier → next round. Dispatch via `runpodctl` (provision + SSH) or the RunPod API; completion detected via a per-pod done-marker / `final.zip` poll (the pattern already used for harvesting).
- *Why:* training pods are ephemeral; the round state machine must survive any pod dying. Reuse the marker/poll pattern proven this session.

**D6 — Thread/memory caps stay concurrency-gated and off for one-per-pod.** One specialist per pod = a full uncapped box → no thread cap, opponent cache effectively unbounded. The per-worker caps only ever apply when `DIGIMON_LEAGUE_CONCURRENCY>1` (in-pod parallel), which fan-out does not use.
- *Why:* capping a single specialist's workers throttled collection ~5×; pinning OMP in the learner throttled its update ~5×. One-per-pod sidesteps both — keep it that way.

## Risks / Trade-offs

- **[Tier 1 loses the evolving-pool dynamic]** → specialists never face each other's *current* versions, only a fixed champion pool. Mitigation: acceptable for most runs; promote round outputs into the champion pool between full passes, or use Tier 2 when the dynamic matters.
- **[RunPod capacity/quotas are volatile]** (community pods vary; A40-volume hangs; consumer GPUs intermittently "no resources") → Mitigation: provisioning layer retries across GPU types, verifies `cpu.max`/RAM, prefers secure/dedicated for guaranteed cores.
- **[Per-pod harvest can lose a pod's work if it dies mid-round]** → Mitigation: periodic checkpoint sync to the store (not only at the end) + Tier-2 round is resumable from the last barriered registry; Tier-1 just re-runs the lost deck.
- **[Orchestrator is a new control-plane dependency]** → Mitigation: keep state in the shared store (idempotent, resumable); the orchestrator is restartable and reads round state from storage.
- **[Cost: many pods + harvest overhead]** → Mitigation: `--terminate-after` guards; tear pods down immediately after harvest; Tier 1 pods are independent so a slow/cheap mix is fine.

## Migration Plan

Additive — nothing to migrate. Ship Tier 1 first (launcher + provisioning + harvest), validate on a small deck set, then build Tier 2 (shared store + orchestrator + barrier) on top. Rollback is trivial: the single-box driver remains the default and is untouched.

## Open Questions

- Dispatch mechanism: `runpodctl` + SSH (proven this session) vs the RunPod REST API (cleaner, but needs auth plumbing) — pick during Tier 1.
- Shared store backend for Tier 2: which `server/storage/` adapter (Spaces/S3) and bucket layout for registry vs checkpoints vs manifests.
- Completion signal: poll `final.zip` vs an explicit per-pod status file in the store (lean toward the status file for Tier 2).
- Effective-core floor + GPU-class policy: what minimum `cpu.max` and which GPU tiers to accept/prefer.

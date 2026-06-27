## Context

We've run single-learner training on Hetzner (ccx33 = 8 dedicated cores; cx53 = 16 *shared* cores) and DO, published the trainer image via CI-on-tag, and served models through the hosted-API `/models/manifest.json` + desktop cache. Running the deck-specialist league (sibling change) is operationally heavier: it is multi-learner and round-structured, so it needs a topology decision, durable round/registry state, six published models, and discipline around the failure modes we already hit — idle boxes billing, losing a peak checkpoint to `keep_last=3`, and shared-vCPU throttling under sustained load. New-account Hetzner projects also cap dedicated cores at 8 until a support bump.

## Goals / Non-Goals

**Goals:**
- A repeatable recipe to provision and run a full league (rounds → snapshots → next round) on cloud compute, resilient to box restarts.
- Publish the six deck-keyed specialists so the in-app/desktop AI loads the right one per deck (generalist fallback).
- Make budget + teardown first-class so a six-learner league doesn't quietly bill for idle boxes.

**Non-Goals:**
- The league *algorithm/system* itself (sibling `add-deck-specialist-league`).
- GPU/LSTM provisioning specifics beyond noting the cost multiplier.
- The DB-backed hosted gauntlet pipeline (we run standalone).

## Decisions

**1. Topology is a dial; default to "one box, specialists sequential per round," scale out only if rounds are too slow.** Because a round trains against frozen pools, specialists need not be simultaneous. Start on one 8–16-core box running the round's specialists sequentially (cheap, simple, one thing to babysit); fan out to multiple boxes (one specialist each) only when round wall-time is the bottleneck. *Why:* matches the compute dial in the system design and our existing single-box ops; avoids standing up a cluster prematurely.

**2. Durable round + registry state, never ephemeral `/app`.** The specialist registry, snapshots, and per-round artifacts live on persistent storage (host volume mirrored locally, or object storage), so a box restart or teardown mid-league doesn't lose the round barrier. *Why:* the `cd /app` ephemeral-storage trap already cost us; a six-learner league across rounds is far more exposed to it.

**3. Dedicated (CCX) over shared (CX) for league rounds; document the quota bump.** Sustained, pinned multi-hour rounds belong on dedicated vCPU; the cx53-class shared box risks fair-use throttling. Record the new-account 8-core cap and the `support@hetzner.com` bump path so a parallel topology (e.g. 2× ccx43) is unblocked when needed. *Why:* we verified the quota wall and the shared-vCPU caveat first-hand.

**4. Image carries (or mounts) the league driver, on the v0.35+ concede-disabled engine.** Bake the standalone league orchestrator + specialist-registry tooling into `digimon-trainer` (or mount as we've done for fast iteration), published via the existing tag-triggered CI. *Why:* one artifact to deploy; concede-disabled is a hard requirement from the system change.

**5. Publish via registry→manifest, keyed by deck; desktop resolves by the deck it pilots.** The specialist registry is the source of truth; a publish step emits per-deck entries into `/models/manifest.json` with layout-hash + version tags, and the desktop model cache + agent loop pick the specialist matching the AI's current deck, falling back to the generalist. *Why:* reuses the existing manifest/cache pipeline; deck-keying is the natural deployment shape for per-deck specialists.

**6. Teardown + budget are first-class orchestration steps.** Each round's runbook ends with "download artifacts + snapshot to durable store + destroy/idle the box," and the league carries a per-round + total budget estimate. *Why:* we repeatedly left idle boxes billing; ×6 learners makes this expensive to forget.

**7. Monitoring = per-specialist TensorBoard + training MCP + the matchup matrix as the dashboard.** Reuse the watcher sidecar + `sync_cloud_runs.sh` + the inspection MCP; the per-round 6×6 matrix is the human-facing progress signal. *Why:* the in-run win rate is degenerate; the matrix is what actually tells you the league is improving.

## Risks / Trade-offs

- **Idle billing × 6 learners** → teardown is a mandatory orchestration step, not a manual afterthought; prefer one-box-sequential to minimize concurrent idle surface.
- **Shared-vCPU throttling (cx53)** under sustained rounds → prefer CCX-dedicated for real rounds; treat the cx53 as burst/dev.
- **Hetzner 8-core quota wall** blocks wide fan-out → document + pre-request the bump before a parallel league.
- **Lost peak checkpoints** (the `keep_last=3` lesson) → generous retention + durable storage so the per-round best is always re-selectable.
- **Manifest/layout-version skew** — six specialists must share the app's observation layout/version, or the desktop loads an incompatible model → layout-hash gate at publish time; version the manifest entries.
- **Box churn mid-round** losing state → durable registry/snapshots make a round resumable from the last barrier.

## Migration Plan

1. Run a **2-deck, 1-round** league on one existing box (the ccx33 or cx53) sequentially; confirm durable registry + snapshot survive a deliberate box restart.
2. Add the **publish step**: emit those 2 specialists into a staging manifest and load them in the desktop app behind a flag; verify deck-keyed resolution + generalist fallback.
3. Scale to **6 decks, multi-round**, choosing sequential-on-one-box vs fan-out by measured round wall-time and budget.
4. **Rollback**: the generalist remains the shipped default; specialists publish behind a version/flag and can be withdrawn from the manifest without touching the generalist.

## Open Questions

- Durable store: host block volume mirrored via `sync_cloud_runs.sh` vs an object store (S3/Spaces) for the registry+snapshots.
- Parallel topology economics: is 6× boxes for a round ever worth it over sequential, given round budgets?
- How the desktop determines "which deck the AI is piloting" to pick the specialist (engine already knows the deck; surface it to the agent loop).
- Specialist versioning/rollback semantics in the manifest (per-deck version, pinning, A/B).
- Whether to bump the Hetzner dedicated-core quota now (pre-clear the parallel path) or stay one-box-sequential until proven necessary.

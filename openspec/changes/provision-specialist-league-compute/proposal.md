## Why

The `add-deck-specialist-league` change defines the league *system*, but running six co-evolving specialists is a different operational shape than the single-learner runs we've done so far: it needs either one big box hosting several learners or a fan-out across boxes, a training image that carries the league driver, round-barrier orchestration that survives box churn, and a way to publish six deck-keyed models so the desktop/in-app AI can load the right specialist per deck. We've also repeatedly paid for idle boxes and lost peak checkpoints; the league multiplies those failure modes by six. This change captures the provisioning, run-ops, and deployment needed to actually execute the league and ship its output.

## What Changes

- **Provisioning recipe for the league** on Hetzner Cloud: pick the topology (one 16-core box running specialists sequentially/few-at-a-time vs. fan-out across boxes for parallel rounds), with the new-account **8-dedicated-core quota** and the support-bump path documented, plus the CX-shared-vs-CCX-dedicated trade-off and US/EU placement.
- **Training image** (`digimon-trainer`) carries the league driver + specialist-registry tooling (or mounts them), on the **v0.35+ concede-disabled** engine; published via the existing CI-on-tag flow.
- **Round-barrier orchestration ops**: launch a round, detect completion, snapshot all specialists into the registry, emit the next round's pools, and continue — resilient to box restarts (artifacts + registry on durable storage, not ephemeral `/app`).
- **Monitoring**: per-specialist TensorBoard + the training-inspection MCP + `sync_cloud_runs.sh` mirroring, and the per-round matchup-matrix as the dashboard signal.
- **Cost + teardown discipline**: per-round / per-league budget estimates, generous checkpoint retention (we lost a peak to `keep_last=3`), and an explicit "download artifacts + destroy box" step so idle boxes don't bill (a repeated lesson).
- **Deployment of the six specialists**: publish them to the model manifest the hosted API serves and the desktop caches, **keyed by deck**, so the in-app AI loads the matching specialist (generalist as fallback) — with layout-hash/version tagging.

## Capabilities

### New Capabilities
- `specialist-league-compute`: the provisioning, run-orchestration ops, monitoring, cost/teardown discipline, and deck-keyed model publishing required to execute the deck-specialist league on cloud compute and ship its specialists to the app.

### Modified Capabilities
<!-- Additive: composes the existing cloud-training pipeline, training image/CI, model manifest/cache, and training-inspection MCP without changing their requirements. -->

## Impact

- **Infra/ops**: extends `docs/CLOUD_TRAINING.md` (Path B / Hetzner CCX-CX), the `digimon-trainer` image + `training-image.yml` CI, and `scripts/` provisioning helpers (e.g. `provision_hetzner_train.sh`) to the multi-learner/round shape.
- **Deployment**: the hosted-API `/models/manifest.json` + the desktop model cache (`code/src-tauri/src/models.rs`) gain per-deck specialist entries; the desktop agent loop resolves a specialist by the deck it is piloting.
- **Dependencies**: `add-deck-specialist-league` (the system being run), the v0.35 concede-disabled image, a Hetzner project with adequate (possibly bumped) core quota, and the champion/specialist registry as the source of truth for what gets published.
- **Cost**: small per-round (tens of dollars on 8–16 cores), but multiplied by six specialists × rounds — so budget + teardown discipline are first-class here, not an afterthought.

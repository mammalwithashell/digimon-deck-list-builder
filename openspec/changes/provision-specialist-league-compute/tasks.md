## 1. Durable state

- [ ] 1.1 Choose the durable store for registry + snapshots + per-round results (host volume mirrored via `sync_cloud_runs.sh` vs object storage) and document it
- [ ] 1.2 Wire the league orchestrator to read/write round-barrier state from the durable store (never only ephemeral `/app`)
- [ ] 1.3 Verify a round resumes from the last barrier after a deliberate box restart/teardown

## 2. Provisioning recipe

- [ ] 2.1 Extend `docs/CLOUD_TRAINING.md` (Path B) + `scripts/provision_hetzner_train.sh` for the league: single-box-sequential (default) and fan-out topologies
- [ ] 2.2 Document the Hetzner 8-dedicated-core quota + `support@hetzner.com` bump path; prefer CCX-dedicated over CX-shared for sustained rounds
- [ ] 2.3 Surface the quota wall (clear error + bump pointer) when a parallel topology exceeds available cores

## 3. League image + driver

- [ ] 3.1 Bake (or mount) the standalone league orchestrator + specialist-registry tooling into `digimon-trainer`, on the v0.35+ concede-disabled engine
- [ ] 3.2 Publish via the existing tag-triggered `training-image.yml` CI; verify GHCR manifest + a real import-chain smoke

## 4. Round-ops orchestration

- [ ] 4.1 Round runbook: provision → run specialists (sequential or fan-out) → detect completion → snapshot all into the registry → emit next round's pools
- [ ] 4.2 Make teardown an explicit step: persist artifacts to durable store, then destroy/idle the box (no idle billing left behind)
- [ ] 4.3 Attach per-round + total budget estimates and generous checkpoint retention (per-round best always recoverable)

## 5. Publish specialists to the manifest

- [ ] 5.1 Publish step: registry → `/models/manifest.json` entries keyed by deck, tagged with `tensor_layout_hash` + version
- [ ] 5.2 Layout-hash gate at publish time: reject specialists whose layout mismatches the app's active observation layout
- [ ] 5.3 Versioning/rollback semantics for per-deck specialist entries (pin, withdraw, A/B), generalist remaining the default

## 6. Deck-keyed inference resolution

- [ ] 6.1 Surface "which deck the AI is piloting" to the desktop agent loop (`code/src-tauri/`) so it can pick a specialist
- [ ] 6.2 Resolve specialist-by-deck from the manifest in the desktop model cache (`models.rs`), with generalist fallback
- [ ] 6.3 Verify deck-keyed load + fallback in a desktop bot game behind a flag

## 7. Monitoring

- [ ] 7.1 Per-specialist TensorBoard + `sync_cloud_runs.sh` mirroring + training-inspection MCP coverage for league runs
- [ ] 7.2 Surface the per-round 6×6 matchup matrix as the human-facing progress dashboard

## 8. Bring-up: stage → scale

- [ ] 8.1 Provision + run a 2-deck, 1-round league on one box sequentially; confirm durable state survives a forced restart
- [ ] 8.2 Publish those 2 specialists to a staging manifest; load them in the desktop app behind a flag (deck-keyed + fallback)
- [ ] 8.3 Scale to 6 decks, multi-round; pick sequential-vs-fan-out by measured round wall-time + budget; confirm teardown leaves nothing billing

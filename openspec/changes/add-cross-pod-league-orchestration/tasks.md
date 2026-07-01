## 1. Pod provisioning contract (`training-pod-provisioning`)

- [ ] 1.1 Add a `provision_pod` helper (under `code/tools/`) that creates a RunPod pod via `runpodctl`, container-disk-only (no `--volume-in-gb`), with a configurable `--terminate-after` guard, retrying across GPU classes on "no resources".
- [ ] 1.2 After boot, SSH in and read effective cores from `cpu.max` (quota ÷ period); reject/re-roll the pod if below a configurable `--min-cores` floor; pre-filter on `runpodctl vcpuCount` before provisioning.
- [ ] 1.3 Capture and log `cpu.stat` (`nr_throttled`/`throttled_usec`) and `memory.max`/headroom alongside any reported throughput, so a throttled pod is attributable.
- [ ] 1.4 Add a `harvest_pod` helper: download `final.zip` + `.meta.json` (and any registry deltas) to a durable local path, verify the download, and only then `runpodctl remove pod`.
- [ ] 1.5 Unit-test provisioning/verify/harvest with a fake `runpodctl` (cpu.max parsing, floor rejection, harvest-before-terminate ordering).

## 2. Tier 1 — barrier-free fan-out (`league-pod-fanout`)

- [ ] 2.1 Add `code/tools/league_fanout.py`: take a deck list + pod budget + a fixed champion-pool manifest (from `champion_admin.py emit-pool`) + the MLP generalist.
- [ ] 2.2 For each deck, build the training command via `train_specialist_league.build_specialist_argv` against the FIXED pool (no per-round barrier), with `DIGIMON_LEAGUE_CONCURRENCY=1` (no thread caps, no learner OMP pin, large opponent cache).
- [ ] 2.3 Dispatch one deck per pod up to the budget; queue extra decks and dispatch each to the next freed pod.
- [ ] 2.4 Poll per-pod completion (done-marker / `final.zip`), harvest+verify, then tear the pod down; mark a deck failed and optionally re-dispatch to a fresh pod if it dies before a verified final.
- [ ] 2.5 Support both MLP and `--lstm` specialists (pass through to `build_specialist_argv`).
- [ ] 2.6 Tests: fan-out launch with a fake provisioner — N decks → N pods, queue overflow, independent teardown, re-dispatch on pod death.

## 3. Tier 2 — distributed league (`distributed-league-orchestration`)

- [ ] 3.1 Add a shared-store adapter wrapper (reuse `code/server/storage/`) with a bucket layout: registry, per-deck/round checkpoints, per-round pool manifests, done-markers.
- [ ] 3.2 Add `code/tools/league_orchestrator.py` that runs on a durable control node and drives rounds: emit pools → dispatch one deck per pod → wait-for-all → harvest → barrier → next round.
- [ ] 3.3 Pods read their round pool from the store and upload their checkpoint back; the orchestrator is the SOLE registry writer (barrier folds all decks' round-k checkpoints centrally) — reuse `write_round_pool` / `_barrier` / `SpecialistRegistry`.
- [ ] 3.4 Enforce the per-round barrier: do not emit round k+1 pools or dispatch any round-k+1 pod until every deck's round-k checkpoint is harvested + verified.
- [ ] 3.5 Make the orchestrator resumable: reload the registry from the store on restart and continue from the last completed round; re-dispatch a deck whose round-k checkpoint is missing.
- [ ] 3.6 Tests: barrier holds round k+1 until all of round k; single-writer registry under concurrent finishes; resume-from-store after orchestrator restart; re-dispatch on pod death.

## 4. Docs / runbooks

- [ ] 4.1 Update `docs/CLOUD_TRAINING.md` with the provisioning contract (verify `cpu.max` not `nproc`; A40-volume hang → container-disk-only; harvest-before-terminate; `--terminate-after`).
- [ ] 4.2 Update `docs/TRAINING_RUNBOOK.md` (+ `docs/runbooks/`) with the in-pod-parallel verdict (reliable, not a proven speedup; confounded by CPU throttling), the one-specialist-per-pod recommendation, and the Tier 1 / Tier 2 fan-out usage.
- [ ] 4.3 Cross-link the OpenSpec change from the runbooks and record the lessons in the RunPod ops memory.

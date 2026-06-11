# Design — harden-training-pipeline

## Context

The training stack ([pilot_training.py](../../../code/digimon_gym/agents/pilot_training.py)) and the model-evaluation harness (`add-model-evaluation-harness`, landed 2026-05-31) are individually mature, but the loop that turns them into competitive models has never closed:

- **Self-play is structurally unsound.** `DigimonEnv` builds observations from **Player 1's perspective only** (`digimon_gym.py:130`). `opponent="self-play"` simply skips `OpponentWrapper` (`pilot_training.py:1865-1866`), so the SB3 agent picks Player 2's actions while seeing Player 1's-perspective tensors. P2 therefore plays on garbage input, and the policy gradient trains on those wrong-perspective transitions. This explains the 2026-05-31 collapse: a self-play run reported a flat 100% in-run win rate (the agent-as-P1 beating its own flailing P2 seat) while anchored checks showed 22.5% vs greedy at 500k steps — a ~55-point regression from its init checkpoint.
- **In-run signal is not trustworthy in exactly the regimes that matter.** The `WinRateCallback` eval reuses the training opponent, so it is meaningless under self-play and only weakly comparable across modes (CLAUDE.md rule 30). The anchored-eval harness exists but runs post-hoc via CLIs only; the in-training anchored suite (task 2.3 of `add-model-evaluation-harness`, and the "Default anchored suite for runs" requirement in its `anchored-model-evaluation` spec) was deferred. The self-play run burned a full A40 budget because nothing in-run could see the collapse.
- **The champion loop has never cycled.** `models/champions/registry.json` holds v020 + v022; the validated `starter1_6_flat_control_v1` model (the "keeper" from the 2026-05-31 A/B) was never registered; no run has ever trained against a registry-derived opponent pool. The one frozen-pool run drifted (50%→30% vs its two fixed opponents) precisely because the pool never grew.
- **Provenance gaps**: no git SHA in `TrainingRunMetadata`; bounty params absent from the sidecar; checkpoint contract validates `action_space_size` as a single int (sub-range reorders would pass silently); `tools/run_training_job.py` cannot forward `init_from` (patched in-container on a now-dead pod, never upstreamed); a typo'd reward-YAML path silently falls back to legacy rewards.

## Goals / Non-Goals

**Goals:**
- No training configuration can silently train against a passive or wrong-perspective opponent seat.
- A collapsing run is visible from inside the run (anchored scalars) within 1–2 eval windows.
- The opponent pool for a new run is derivable from the champion registry in one step, and the promotion cadence is documented and exercised at least once (registering the starter-flat control).
- Every run artifact is auditable after the fact: git SHA, reward/bounty/curriculum knobs, action-space structure.

**Non-Goals:**
- True dual-perspective self-play (P2-perspective observation tensors + sided reward bookkeeping). That is an engine/tensor feature, out of scope; pool-based fictitious self-play covers the curriculum need.
- Equilibrium methods (Deep CFR / ReBeL) — still blocked on `make-engine-cloneable`.
- Hyperparameter sweep tooling, throughput/vectorization work, W&B integration.
- Changing reward magnitudes or the BO3 calibration.

## Decisions

### D1 — Retire `opponent="self-play"` with a hard error, do not fix it

`make_env` raises `ValueError` for `opponent="self-play"` with a message explaining the P1-perspective limitation and pointing at the replacement recipe (`opponent="pool"` + champion-derived manifest). The CLI flag `--self-play` is kept as a parse-able flag that fails with the same message (so old scripts fail loudly with guidance, not with argparse noise).

*Why not fix it:* a correct two-seat self-play needs perspective-flipped observations, which `DigimonEnv`/the tensor profiles do not support; bolting it on is a large engine feature for a curriculum benefit that pool-based fictitious self-play (train vs frozen champions → promote → grow pool) already delivers with better stability (AlphaStar-league family). The collapse evidence shows the current mode is worse than useless — it silently destroys good checkpoints.

*Alternative considered:* keep the mode behind `--i-know-what-im-doing`. Rejected: there is no valid use of a wrong-perspective opponent seat.

### D2 — In-training anchored eval as a separate `AnchoredEvalCallback`

A new SB3 callback, independent of `WinRateCallback`, fires every `anchored_eval_freq` steps (default 100,000; `0` disables). It reuses `anchored_eval.evaluate_against_anchors` (seat-balanced via `_seat_balanced_seed`) against greedy plus layout-compatible champions from the registry, `anchored_eval_games` per anchor (default 24).

- Scalars: `pilot/anchored/greedy/win_rate`, `pilot/anchored/<champion>/win_rate`, plus `pilot/anchored/panel_mean`.
- Sidecar: rows appended to a **separate** `anchored_evals.jsonl` (not `evals.jsonl`) — existing readers of `evals.jsonl` (training MCP `run_summary`, ad-hoc trend scripts) assume one row per training-eval window; mixing row kinds risks corrupting trends. The training MCP gains an `run_anchored_evals(name)` reader and `run_summary` surfaces the latest anchored row.
- Deck source: the run's own `deck_pool_snapshot.json` when present, else the candidate/opponent decks of the run config, else the 6 starters (same fallback ladder as `anchored_eval_cli`).

*Why a separate callback:* `WinRateCallback` is ~900 lines of training-opponent eval + telemetry; the anchored panel has different opponents, different cost profile, and different cadence. Separation keeps both testable and lets operators disable one without the other.

*Cost control:* default panel (greedy + 2 champions) × 24 games at release-build speed is minutes per window at 100k cadence — single-digit % overhead on a 1M-step run. `anchored_eval_freq`/`anchored_eval_games` are config knobs; the callback logs wall-clock per window so the overhead is observable.

### D3 — Champion-derived opponent pools

`OpponentPool` gains a constructor `from_champion_registry(registry_path, layout_hash)` that builds a pool manifest from all layout-compatible champions (uniform weights by default; PFSP sampling already exists at sample time). `champion_admin.py` gains an `emit-pool --out <manifest.json>` subcommand for the file-based flow used by cloud job configs. The runbook documents the standing cadence:

```
train (opponent=pool from registry) → anchored_eval_cli + elo_ladder_cli
  → champion_admin promote (≥55% gate) → registry grows → next run's pool grows
```

As part of this change the loop is exercised once: `starter1_6_flat_control_v1` (downloaded at `cloud_downloads/starter1_6_flat_control_v1/`) is run through the gate and registered (with `--force` + a `source` note only if the gate is unmeetable for cohort reasons — the registry records provenance either way).

*Why uniform default weights:* PFSP weighting already exists as a `sample(mode=...)` choice; baking weights into the manifest would duplicate it.

### D4 — Fail-fast reward-profile loading (BREAKING)

If `reward_profiles_path` / `reward_gameplay_path` are set (including their defaults) and the file does not exist, `train()` raises `FileNotFoundError` naming the path, instead of silently returning the identity factory (legacy rewards). Explicitly opting into legacy rewards becomes `reward_profiles_path: null`.

*Why include the default paths:* the dangerous case observed in practice is a cloud image missing a config file — the run proceeds with a different reward function than the operator believes. An explicit `null` preserves the legitimate opt-out.

### D5 — Provenance hardening

- `TrainingRunMetadata` gains `git_sha` (best-effort `git rev-parse HEAD`, `"unknown"` outside a repo — cloud images may not carry `.git`), `bounty_threshold`, `bounty_bonus`.
- Checkpoint `.meta.json` gains an `action_space_structure` tuple `(SOURCE_SELECT_END, BREEDING_SOURCE_SELECT_START, BREEDING_SOURCE_SELECT_END)`; `resume_from`/`init_from` validation compares it when present and warns-only when absent (older checkpoints).
- `tools/run_training_job.py` adds `"init_from"` to `_TRAIN_KWARGS` and `train()` accepts `init_from=` directly (upstreaming the in-container patch verbatim).

## Risks / Trade-offs

- [Anchored panel slows cloud runs] → defaults sized to single-digit % overhead; both knobs configurable; wall-clock per panel logged; `anchored_eval_freq=0` opt-out.
- [Anchored eval constructs fresh envs inside a training process — engine panic during a panel could kill the run] → wrap each panel game in the same try/except contract as `TrainingRecordingWrapper`; a failed panel logs and skips, never aborts training.
- [Retiring self-play breaks existing job configs/scripts] → hard error carries the migration recipe; `docs/TRAINING_RUNBOOK.md` and `MODEL_EVALUATION.md` updated in the same change; no silent behavior change.
- [Reward-YAML fail-fast breaks cloud images missing config files] → that is the point; the error message names the missing file and the `null` opt-out. Release notes in the runbook.
- [Starter-flat control may not pass the 55% gate vs v022] → gate result is recorded either way; if it fails, registration uses `--force` with the anchored-vs-greedy evidence noted in `source`, or is skipped with the verdict documented — the deliverable is an *exercised, recorded* promotion decision, not an unconditional registration.
- [Champion pool growth makes opponents nonstationary across runs (but stationary within a run)] → this is the intended fictitious-self-play design; within-run drift of the kind seen in the frozen-pool run is addressed by the pool *growing between* runs, and in-run anchored scalars now make any drift visible.

## Migration Plan

1. Land D1 (hard error) + D4 (fail-fast) + D5 (provenance) — all local, no retraining implications.
2. Land D2 callback + MCP reader; verify on a short local run (`--timesteps 50000`, `anchored_eval_freq 25000`).
3. Land D3 pool derivation; run the promotion decision for `starter1_6_flat_control_v1`; update runbook cadence section.
4. Rollback: each decision is independent; D1/D4 revert by restoring the old branches; D2/D3 are additive.

## Open Questions

- Does the Rust engine already expose (or cheaply could expose) a P2-perspective observation, which would reopen true self-play later? Tracked as a question for `docs/RUST_ENGINE_GAPS.md`, not blocking.
- Should `run_summary` in the training MCP gate its "active/healthy" heuristic on anchored trend (e.g., flag a run whose anchored-vs-greedy drops >10 points window-over-window)? Deferred — start with surfacing the data.

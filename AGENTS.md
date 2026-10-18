# AGENTS.md - Agent Engineering Guide

Refer to `CLAUDE.md` for repository-wide architecture, service boundaries, and working rules.
Refer to `docs/digimon-rules/` (verified derivations of `general_rule.pdf`; `docs/RULES_CONTEXT.md` is a retired pointer) for rule implementation details.
Refer to `docs/TRAINING_RUNBOOK.md` for training operations and DB-backed pipeline details.
Refer to `docs/TENSOR_SPEC.md` and `docs/ACTION_SPEC.md` before changing observation or action contracts.
Refer to `docs/RUST_ENGINE_API.md`, `docs/RUST_DSL_TEST_API.md`, and `docs/RUST_ENGINE_GAPS.md` before planning Rust card-effect or DSL work.

## How to Use This Guide

`CLAUDE.md` is the repository-wide engineering guide. This file narrows that guidance to agent-facing contracts: RL training, deck optimization, action masking, observation profiles, model export, gauntlets, and the hosted training pipeline.

When this file conflicts with a more specific live spec, use the live spec and update this file in the same change. The most common live specs are:

- Observation/action contracts: `docs/TENSOR_SPEC.md`, `docs/ACTION_SPEC.md`, and `docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md`.
- Rust engine and DSL gaps: `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`, and `docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md`.
- Training and model operations: `docs/TRAINING_RUNBOOK.md`, `docs/TOOLS.md`, and `docs/ARCHITECTURE.md`.

Avoid adding new hardcoded snapshot claims here unless they are part of a stable contract. Prefer named profile IDs, exported constants, and layout metadata over copied numbers.

## Scope

This document describes the RL/deckbuilding agent stack in the current Rust-pivot repository. It focuses on the stable contracts that agent code depends on:

- Architect: deck optimization by card swaps.
- Pilot: battle play inside the headless engine.
- Observation/action profiles, training wrappers, gauntlets, model export, and DB-backed orchestration.
- Rust card-effect and DSL readiness work when it changes what agents can legally observe or choose.

The no-approximations policy from `CLAUDE.md` applies to agent work: every legal choice must be surfaced through the engine action/pending-selection contracts so RL agents can learn it. Do not add card-effect stubs, auto-selections, or hidden UI-only decisions.

## Source Priority for Rules and Card Behavior

`CLAUDE.md` ("Source priority for card / keyword / rules questions") is authoritative; this is a summary. When deciding what a card, keyword, timing window, or selection should do, use sources in this order:

1. The official rules manual `Digimon TCG resources/general_rule.pdf` (+ `glossary.pdf`) — canonical for rules, keyword semantics, and timing. Base-repo only (CLAUDE.md rule 32); the committed verified derivations in `docs/digimon-rules/` are the cheap first stop.
2. DCGO C# source (base-repo `DCGO/`, CLAUDE.md rule 29) — authority for how a specific card actually resolves; outranks the card-text JSON.
3. `data/card_overrides.json`, then `data/cards.json` — printed text (`effect_text`, `inherited_text`, `security_text`). API-ingested and lossy.
4. Fandom wiki pages for card-specific ruling notes and errata context.

For what a card *prints* (traits including Rule-granted ones, costs, printed text, official Q&A), the official Bandai DB mirror (`data/card_official.json`, `data/card_bundles/<ID>.md`, `data/card_qa.json`) and the card image outrank both DCGO and `cards.json`. The retired `docs/RULES_CONTEXT.md` is no longer a source.

## Engine Status

The project is migrating to the Rust engine as the source of truth:

- Target engine: `code/digimon-engine/`.
- Card scripting DSL: `code/digimon-dsl/` plus YAML card specs under `code/digimon-engine/cards/`.
- Python bindings: `code/digimon-engine-py/`, exposed as `digimon_engine`.
- RL environment: `code/digimon_gym/digimon_gym.py`.
The legacy Python engine was removed. There is no Python engine fallback.

`DigimonEnv` always uses `RustHeadlessGame` from the PyO3 wheel:

- `DIGIMON_BACKEND` unset or `rust`: use Rust.
- Any other explicit backend raises `RuntimeError`.
- A missing Rust wheel raises `RuntimeError` with build instructions.

`docs/RUST_PYTHON_PARITY.md` is historical. Current engine/DSL gaps live in `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/dsl-vocab-gaps.md`.

`DigimonEnv` also selects an observation profile:

- Default pilot profile: `standard_lite_v2`.
- Compact profile `standard_compact_v1` is engine-only compatibility; the training adapter rejects it.
- Env override: `DIGIMON_TENSOR_PROFILE`.

`DigimonEnv` defaults to `standard_lite_v2` (8410 floats). The Rust engine default and `digimon_engine.TENSOR_SIZE` use `standard_lite_deck_v2` (8850 floats). The 1375-float `standard_compact_v1` remains available only through explicit engine compatibility APIs. Training, inference, export, model metadata, and feature extractors must resolve the selected profile from exported layout metadata rather than treating `TENSOR_SIZE` as the training observation size.

---

# 0. Workflows Agents Must Respect

## 0.1 Engine and DSL Gap Work

Use capability-centric gap language. If a card cannot be implemented faithfully, file or reference the missing reusable primitive in `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, or `qa/dsl-vocab-gaps.md`; do not describe it as a one-off card TODO when the same primitive can unblock multiple cards.

For archetype readiness checks, use `.codex/skills/assess-rust-engine-archetype/`. That workflow is read-only: it inspects printed text, current YAML/DSL lowering, action/pending-selection support, and tests, then reports `ready`, `dsl-gap`, `engine-gap`, `rules-gap`, `test-gap`, or `data-gap`.

For planned gap-roadmap groups, use `.codex/skills/plan-rust-engine-gap-group/` and the roadmap spec in `docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md`.

Group-style roadmap work has one important invariant: do not expand `ACTION_SPACE_SIZE` or change active tensor contracts as a side effect of card unlock work. If a missing player-visible choice requires a contract change, stop and plan it as an action/tensor contract change with `docs/ACTION_SPEC.md`, `docs/TENSOR_SPEC.md`, Rust constants, PyO3 exports, RL wrappers, frontend constants, and model metadata updated together.

## 0.2 Rust Card Effects

New Rust card effects are TDD:

1. Read the printed text (card image / `data/card_bundles/<ID>.md` / `data/cards.json`) and relevant rule sections.
2. Write a failing Rust behavioral test under `code/digimon-engine/tests/cards_behavioral/<set>/`.
3. Implement DSL-first: a YAML spec in `code/digimon-engine/cards/<set>/` lowered by `code/digimon-dsl/` (widen the DSL/engine when it can't express the card); a hand-written `CardEffect` in `src/cards/raw_rust/` is the last resort (CLAUDE.md rule 28). No hidden auto-choices.
4. Verify the action mask exposes every legal decision.
5. Update gap trackers and parity notes when the work closes or discovers a reusable primitive.

Do not author new Python card scripts for cards already implemented in Rust. Cards migrate one direction only: legacy Python reference to Rust ownership.

---

# 1. Agent Tracks

The project has two active agent tracks:

1. Architect: deck optimizer trained to recommend card swaps against a target meta.
2. Pilot: battle agent trained/evaluated inside the headless engine.

Both are standalone training surfaces under `code/digimon_gym/agents/` and must not depend on FastAPI, auth, or database code.

---

# 2. Architect (Deck Builder Agent)

Goal: optimize a starting deck by proposing legal card swaps that improve weighted win rate against meta opponents.

## 2.1 Implemented Modules

- `architect_agent.py`: pure PyTorch `ArchitectDQN` with Double DQN, epsilon-greedy masked action selection, and prioritized experience replay.
- `architect_env.py`: Gymnasium `DeckBuildingEnv` for finite-horizon deck swap optimization.
- `architect_pool.py`: `CandidatePool`, swap action encoding, action masks, constraints, restricted-list handling, and candidate filtering.
- `architect_simulator.py`: batch win-rate evaluator for candidate decks using greedy/random/ONNX/SB3 pilot policies.
- `architect_optimizer.py`: end-to-end `MetaOptimizer` that builds opponents, trains the DQN, checkpoints, and saves optimized deck artifacts.
- `architect_training.py`: CLI entrypoint.
- `architect_cotraining.py`, `architect_explain.py`: supporting co-training/explanation utilities.

## 2.2 Architect MDP

State:

- Normalized deck vector over the candidate pool.
- Normalized meta-opponent weights.
- Step counter fraction.

Action:

- `0`: no-op, ending the episode early.
- `1..N`: encoded `(remove_card, add_card)` swap over the candidate pool.
- Total action size is `n_candidates * n_candidates + 1`.

Reward:

- Exponential win-rate delta: `exp(b * new_wr) - exp(b * old_wr)`.
- The default reward amplification constant is `10.0`.

Episode:

- Starts from a base deck.
- Separates Digi-Eggs from main deck; eggs are fixed and excluded from swap candidates.
- Applies up to `max_swaps` card swaps.
- Evaluates each candidate deck through `DeckSimulator`.

## 2.3 Candidate Pool Rules

`CandidatePool` keeps the action space tractable:

- Builds candidates from archetype decklists in `data/deck_library.json`, unless a custom pool is supplied.
- Always includes cards already in the base deck.
- Filters out Digi-Eggs from swap candidates.
- Filters to cards reported by `digimon_engine.load_implemented_card_ids()`.
- Supports extra cards, locked counts, min/max counts, and optional restricted-list enforcement.
- Uses stable sorted candidate ordering for reproducibility.

Action masks enforce:

- No-op is always valid.
- A card can be removed only if the current count is above its minimum.
- A card can be added only if the current count is below its maximum.
- A card cannot be swapped with itself.

## 2.4 Deck Simulator

`DeckSimulator` computes weighted win rate for an evaluated deck:

- Supports pilot policy names `"greedy"` and `"random"`.
- Supports `.onnx` policies through `digimon_gym.inference.onnx_policy`.
- Supports `.zip` SB3 models (`MaskablePPO` or `MaskableRecurrentPPO`).
- Alternates play/draw sides across games to reduce first-player bias.
- Counts simulation crashes as draws so card-script gaps do not poison training runs.
- Can cache matchup results by order-independent deck hash.
- Can evaluate in-process or through `ProcessPoolExecutor`.

LSTM policies must call/reset their recurrent state at episode boundaries.

## 2.5 Architect CLI

Run from repo root:

```bash
python -m digimon_gym.agents.architect_training --archetype "Medusamon"
python -m digimon_gym.agents.architect_training --archetype "Medusamon" --meta local_meta.json --pilot models/mlp_agent.onnx --episodes 500
python -m digimon_gym.agents.architect_training --archetype "Medusamon" --extra-cards "BT24-001,EX10-036" --workers 4 --seed 42
```

Artifacts are saved under `architect_runs/` by default.

---

# 3. Pilot (Battle Agent)

Goal: play Digimon matches inside the headless engine to generate win/loss and policy quality signals.

## 3.1 Implemented Pilot Types

### MaskablePPO (MLP baseline)

- Feed-forward policy from `sb3_contrib.MaskablePPO` with `"MlpPolicy"` over the `CardEmbeddingExtractor` features extractor (`features_extractor.py`); `--net-arch` sets the pi/vf head widths (default SB3 `[64, 64]`).
- Uses `ActionMasker` so illegal actions are never sampled.
- Training entrypoint: `code/digimon_gym/agents/pilot_training.py`.
- Observation shape comes from the selected observation layout, not from `digimon_engine.TENSOR_SIZE`.
- The default pilot observation layout is `standard_lite_v2`, currently `8410` floats.
- `digimon_engine.TENSOR_SIZE` reports the engine default `standard_lite_deck_v2` (8850 floats), not the training default.
- Action space comes from `digimon_engine.ACTION_SPACE_SIZE` and is currently `2192`.

### MaskableRecurrentPPO (Custom LSTM)

Custom implementation in `code/digimon_gym/agents/maskable_recurrent/`.

Motivation: SB3's `RecurrentPPO` and `MaskablePPO` are separate algorithms with no official combined implementation. This module merges recurrent state handling with invalid-action masking.

Architecture:

- `MaskableMlpLstmPolicy` (`policies.py`)
  - Extends `RecurrentActorCriticPolicy`.
  - Replaces the categorical action distribution with a maskable distribution.
  - Supports separate actor/critic LSTMs.
  - Accepts `action_masks` in `forward()`, `evaluate_actions()`, `get_distribution()`, `_predict()`, and `predict()`.
- `MaskableRecurrentRolloutBuffer` (`buffers.py`)
  - Extends `RecurrentRolloutBuffer`.
  - Stores action masks alongside recurrent state and episode-start boundaries.
  - Pads masks for recurrent minibatches; padded timesteps are excluded from loss.
- `MaskableRecurrentPPO` (`maskable_recurrent_ppo.py`)
  - Extends `RecurrentPPO`.
  - Collects action masks at rollout time.
  - Passes masks through policy evaluation during training.

Default LSTM settings from `pilot_training.py`:

- `lstm_hidden_size`: `256`.
- `n_lstm_layers`: `1`.
- `enable_critic_lstm`: `True`.
- `net_arch`: `dict(pi=[64], vf=[64])` unless `--net-arch` / `net_arch` overrides it.
- `batch_size`: forced to `n_steps` for recurrent training.

Inference rule:

- Thread `(state, episode_start)` or `(h, c)` state across steps in an episode.
- Reset state to `None` and `episode_start=True` at episode boundaries.
- The same reset rule applies to ONNX LSTM policies.

### Greedy Policy (Heuristic)

Location: `digimon_gym.digimon_gym.greedy_policy()`, which delegates to the Rust heuristic `runner.greedy_action()` (`code/digimon-engine/src/policies/greedy.rs`) and only falls back to a first-legal-non-pass pick when that binding is unavailable. It never returns concede (action 93) while any other action is legal.

Used as the default fast baseline and common training opponent. Priority logic (see `greedy.rs` for the authoritative version):

- Mulligan: keep if hand contains a level-3 Digimon, else mulligan.
- `SelectPlayOrder` (BO3): play first when offered, else second.
- Breeding: hatch; move only if the raising Digimon has value or a hand evolution line; else pass.
- Main: setup play (search/resource cards or Tamers), then best keep-turn digivolve, then best attack (security pressure before board trades), then best play, then pass.
- Trash-from-hand selection: discard the lowest-value card (non-Digimon first, then lower cost).
- Other phases: pass or first valid non-pass action.

### Random Policy

`pilot_training.random_policy()` samples uniformly from valid actions. If no valid action exists, it falls back to `ACTION_PASS_TURN`.

## 3.2 Gym Environment Contract

Primary env: `DigimonEnv` in `code/digimon_gym/digimon_gym.py`.

API:

- `reset(seed, options)` returns `(obs, info)`.
- `options` supports per-episode `deck1` and `deck2` overrides.
- `step(action)` returns `(obs, reward, terminated, truncated, info)`.
- `action_mask()` returns an `int8` mask of shape `(ACTION_SPACE_SIZE,)`.
- `get_action_mask()` returns a boolean mask alias for compatibility.

Observation/action spaces:

- Observation: `Box(shape=(selected_layout.tensor_size,), low=-10.0, high=20001.0, dtype=float32)`.
- Default selected layout: `standard_lite_v2`, currently `8410`.
- `standard_compact_v1` is rejected by `DigimonEnv`; explicit engine compatibility APIs retain its 1375-float layout.
- Action: `Discrete(ACTION_SPACE_SIZE)`.
- Current `ACTION_SPACE_SIZE`: `2192`.

Profile metadata:

- `DIGIMON_TENSOR_PROFILE` or `DigimonEnv(tensor_profile=...)` selects the profile.
- `info["tensor_profile"]`, `info["tensor_feature_schema_version"]`, and `info["tensor_layout_hash"]` are included on reset/step.
- `digimon_engine.get_observation_layout(profile_id)` is the canonical source for tensor size, section table, card-ID positions, scalar positions, layout hash, and feature schema version.
- Feature extractors and ONNX export must consume layout metadata; they must not infer card-ID slots from legacy Python tensor-layout imports when Rust layout metadata is available.

Mask delivery:

- `info["action_mask"]` is included on `reset()` and `step()`.
- `ActionMasker` calls through to `env.action_mask()`/`env.get_action_mask()` through the wrapper chain.
- Engine masks use `1.0` for legal, `0.0` for illegal; env masks threshold at `> 0.5`.

Reward shaping (`docs/REWARD_PROFILES.md` is the live reference):

- By default `TrainingConfig.reward_profiles_path` / `reward_gameplay_path` point at `code/digimon_gym/agents/reward/{profiles,gameplay}.yaml`, so `RewardProfileWrapper` wraps `DigimonEnv` (inside `OpponentWrapper`), sets `legacy_reward=False`, and supplies the reward. A configured-but-missing YAML is a hard error; setting both paths to null/empty opts back into the legacy recipe.
- Legacy recipe (`DigimonEnv._compute_reward`, `legacy_reward=True`): win `+10.0` plus up to `+5.0` fast-win bonus (`(200 - steps)/200 * 5`), loss `-10.0`, draw `-1.0`; dense `+1.5` per opponent security card removed and `-0.5` per own security card lost; optional digivolve shaping; `-0.001` per step. Calibrated for BO3 (`add-bo3-match-training/design.md` §D9).
- `GauntletWrapper` can add a bounty bonus on terminal wins against high-threat opponents.

Truncation:

- Safety cap is derived from `max_turns`.

## 3.3 Action Masking Deep Dive

Masking must be preserved end to end:

1. Engine exposes `get_action_mask(...)` with `ACTION_SPACE_SIZE` entries.
2. `DigimonEnv` thresholds the engine mask and exposes `info["action_mask"]`.
3. `ActionMasker` is the outermost training wrapper.
4. `MaskablePPO` reads masks during rollout collection and training.
5. `MaskableRecurrentPPO` stores masks in `MaskableRecurrentRolloutBuffer`.
6. Inference must pass `action_masks` into `.predict()`.

The maskable distribution sets illegal-action logits to negative infinity. Illegal actions should have zero probability during sampling and log-prob evaluation, and entropy should exclude masked actions.

The action mask is the legality oracle. Observation metadata may explain phases, pending choices, source provenance, effect categories, or target profiles, but it must never drive game legality or rules resolution. If observation metadata is incomplete, encode unknown/zero metadata and keep the mask correct.

`standard_lite_v2` adds rich pending-choice features for currently installed selections. These rows are aligned to prompt presentation order (`pending_selection.valid_action_ids`), not raw action-ID ranges, because action IDs are phase-aware and reused. Keep that row-order contract stable when adding selection kinds.

## 3.4 Wrapper Chain

Full pilot training wrapper chain, innermost to outermost (`pilot_training.make_env` / `make_vec_env`):

```text
DigimonEnv
  -> RewardProfileWrapper        (when reward profiles are configured — the default)
  -> OpponentWrapper
  -> MatchEnv                    (match_format="bo3" — the TrainingConfig default)
  -> DeckPoolWrapper             (deck_pool_variants)
  -> GeneralistDeckPoolWrapper   (--generalist)
  -> LeaguePoolWrapper           (--opponent league)
  -> GauntletWrapper             (--gauntlet)
  -> TrainingRecordingWrapper    (always — engine-panic safety rail + optional recording)
  -> MulliganLogWrapper          (--mulligan-log on, the default)
  -> ActionMasker
```

Only `OpponentWrapper`, `TrainingRecordingWrapper`, and `ActionMasker` are unconditional. `MatchEnv` sits between `OpponentWrapper` and the deck-pool wrappers so deck/opponent sampling fires once per match, not per game (CLAUDE.md rule 26).

### OpponentWrapper

Location: `pilot_training.OpponentWrapper`.

Converts the two-player game into a single-agent MDP:

- Agent controls Player 1.
- Player 2 is auto-played through a configurable `opponent_fn`.
- Handles Player 2 going first after `reset()`.
- Discards dense rewards from opponent auto-steps.
- Only terminal rewards from opponent sequences pass through.

Every accepted opponent mode (`greedy`, `random`, `pool`, `league`, plus
the config-only `agent`) routes Player 2 through this wrapper with a concrete
`opponent_fn`. The old
`self-play` mode (which skipped the wrapper) is retired and fails at startup:
observations are built from Player 1's perspective only, so the learner was
selecting Player 2's actions against wrong-perspective input
(`harden-training-pipeline`). Use `opponent="pool"` over frozen champions
instead.

### DeckPoolWrapper

Location: `code/digimon_gym/agents/deck_pool.py`.

Varies the agent deck per episode for robustness:

- `reset()` samples a variant and injects it as `options["deck1"]`.
- `"eager"` mode samples uniformly from pre-generated variants.
- `"hybrid"` mode samples mostly from pre-generated variants and sometimes generates new variants dynamically.
- `analyze_core()` treats max-copy cards as core and remaining main-deck cards as flex; Digi-Eggs are excluded from both.
- Variant generation modifies flex slots only and preserves the 50-card main deck constraint.

### GauntletWrapper

Location: `code/digimon_gym/agents/gauntlet.py`.

Samples opponent decks from `MetaGauntlet`:

- `reset()` injects the sampled opponent deck as `options["deck2"]`.
- Adds `opponent_archetype` and `opponent_threat_index` to `info`.
- Adds `bounty_bonus` on terminal wins against opponents above `bounty_threshold`.

### LeaguePoolWrapper

Location: `code/digimon_gym/agents/league_opponent.py`.

Used by `--opponent league` (deck-specialist league, `--league-pool-manifest`):

- Samples a coupled (policy, deck) opponent per match from the round-pool manifest and repoints the `LeagueOpponentController` that `OpponentWrapper` calls.
- Sits outside `MatchEnv`, so one sample spans a whole BO3 match.
- `sampling_mode` `pfsp` (default; uniform until enough games are observed) or `uniform`; `cache_size` bounds the per-worker loaded-opponent cache.

### LeagueOpponentWrapper

Location: `code/digimon_gym/agents/league_wrapper.py`.

Written for Stage 2 of `GauntletOrchestrator` (which records `sampling_mode` in each job's `config_json`). As of 2026-10-18 no non-test code constructs this wrapper:

- `meta_weighted`: samples by meta share with a small minimum weight.
- `pfsp`: inverse win-rate sampling, uniform until enough games have been observed.
- Tracks per-opponent games and wins on terminal episodes.

---

# 4. MetaGauntlet

Location: `code/digimon_gym/agents/gauntlet.py`.

Purpose: sample opponent decks with meta-aware weighting for training and evaluation.

## 4.1 Threat Index

If `digilab_times_played >= confidence_min_appearances`:

```text
TI = (digilab_meta_share * alpha) + (digilab_conversion_rate * beta)
```

Otherwise:

```text
TI = digilab_meta_share * alpha
```

Default parameters:

- `alpha = 1.0`.
- `beta = 2.0`.
- `confidence_min_appearances = 5`.

## 4.2 Sleeper Rule

If all conditions are true:

1. `digilab_times_played >= confidence_min_appearances`.
2. `digilab_conversion_rate > sleeper_threshold`.
3. Current sampling probability is below `sleeper_floor`.

Then the archetype probability is raised to `sleeper_floor`, and non-sleeper weights are reduced proportionally.

## 4.3 Survivorship Bias Guard

- Statistical weights come from DigiLab tournament log data.
- Scraper sources contribute decklists, not meta-share or conversion-rate weights.
- `digilab_meta_share = digilab_times_played / total_across_all_archetypes`.

## 4.4 Deck Pool Routing

Within an archetype, prefer higher-quality deck sources:

```text
digimonmeta > egman > digimoncard_io > file/manual/test
```

(`gauntlet._SOURCE_PREFERENCE`; DigiLab supplies archetype statistics, not decklists.)

Keep this priority aligned with the actual loader/optimizer code when source names change.

Pipeline:

```text
code/tools/meta_loader.py -> data/deck_library.json -> MetaGauntlet.load()
```

## 4.5 Deck Admission

`MetaGauntlet.load()` (and `load_generalist_deck_pool`, which wraps it) admits each decklist on its own:

1. Every card ID is playable (`gauntlet.load_playable_card_ids()`): registered in the Rust engine (`digimon_engine.load_implemented_card_ids()`) and present in the card database `RustHeadlessGame` loads (`data/cards.json`). A YAML spec alone registers a card without giving the runner its card data.
2. No card has a not-ready verdict in `qa/qa-reports/validated_cards_dsl.json`: `PARTIAL`, `BLOCKED`, `AUDITED-DRIFT` or `AUDITED-MISSING-TESTS`. A playable card with no ledger entry counts as ready. This gate runs only for the default library path, and is skipped (with a warning) when the ledger file is missing.
3. Every card is `ready` in `data/oracle_readiness.json` — the DCGO oracle readiness gate (CLAUDE.md rule 34). It applies to every library path and to pool snapshots (`validate_oracle_ready_deck`), has no override, raises `OracleReadinessMissingError` if the artifact is absent, and raises `EmptyTrainingPoolError` naming the top blocking cards when no decklist survives. Regenerate with `PYTHONPATH=code python -m tools.clause_coverage.readiness` (`--plan` picks the next exams).

The ledger's `archetype` field is a batch label and is never matched against library archetype names. An archetype is eligible when at least one of its lists is admitted; `allowed_archetypes` narrows that set and logs, at INFO, each scoped name left with no admitted list.

---

# 5. GauntletOrchestrator (DB-Backed Pipeline)

Location: `code/server/workers/gauntlet_orchestrator.py`.

This is hosted-API infrastructure, not standalone training CLI code. It requires the FastAPI backend, database, and `TrainingJobWorker`.

Stage flow:

```text
configuring -> stage_1 -> stage_2 -> stage_3 -> completed
                                      \-> failed if more than 50% of stage jobs fail
```

Stage 1: Bootstrap

- All participants train against the greedy opponent.
- Creates an `Agent` plus `TrainingJob(job_type="train_vs_greedy")` per participant.
- Timesteps are derived from `stage1_games * STEPS_PER_GAME_ESTIMATE`.

Stage 2: Meta training

- Core agents use meta-weighted sampling.
- Supporting agents use PFSP-style sampling over core agents.
- Creates `TrainingJob(job_type="train_vs_agent")`.
- Stores `opponent_pool` in `config_json`.

Stage 3: Evaluation

- Runs round-robin matchups between core agents.
- Creates `TrainingJob(job_type="evaluate")` per matchup pair.
- Finalization computes matchup matrix, ETWR, and rankings.

Transitions:

- `on_job_finished()` advances stages automatically.
- Optimistic locking prevents duplicate transitions from parallel job completions.
- If more than half of a stage's jobs fail, the gauntlet is marked failed.

---

# 6. Training Job Worker

Location: `code/server/workers/training_worker.py`.

Hosted-API queue worker responsibilities:

- Poll queued `TrainingJob` rows.
- Claim jobs with DB coordination.
- Bound concurrent execution through `TRAINING_WORKER_MAX_CONCURRENT`.
- Round-robin devices across CUDA GPUs or CPU fallback.
- Recover stale jobs from other workers after the configured timeout.
- Notify `GauntletOrchestrator` when gauntlet-linked jobs finish.
- Atomically increment agent wins/losses/draws/timesteps.

Implementation note: the queue, claiming, recovery, stats, and gauntlet hooks are the durable pieces. Verify current implementation before assuming train/evaluate job bodies are complete; `docs/TRAINING_RUNBOOK.md` has historically marked some execution bodies as placeholders.

---

# 7. Service Boundaries

Standalone agent modules must not import FastAPI, DB, auth, or hosted API code:

- `pilot_training.py`
- `gauntlet.py`
- `deck_pool.py`
- `features_extractor.py`
- `maskable_recurrent/`
- `architect_*.py`

DB-dependent agent modules belong to hosted API infrastructure:

- `training_worker.py`
- `gauntlet_orchestrator.py`

Engine-only routers and desktop/Tauri paths must not depend on `digimon_gym` training internals. Desktop gameplay, inference, and deck tools run through Tauri `invoke()` into `digimon-engine`, and the Tauri build must not link a Python runtime.

The legacy Python engine was removed. Do not import `engine_py_legacy.*` or introduce a Python engine fallback.

---

# 8. Model Artifacts and Inference

Training outputs:

- Pilot training defaults to `<repo_root>/models/<run_id>/`.
- The hosted API's `/models/manifest.json` scans `models/`.
- Architect training defaults to `architect_runs/`.

Policy formats:

- SB3 `.zip`: `MaskablePPO` or custom `MaskableRecurrentPPO`.
- ONNX `.onnx`: loaded through `digimon_gym.inference.onnx_policy`.
- Heuristic names: `"greedy"` and `"random"`.

Model metadata must include the observation/action contract it was trained or exported against:

- `observation_profile`
- `tensor_version`
- `feature_schema_version`
- `tensor_size`
- `tensor_layout_hash`
- `action_space_size`

The desktop model compatibility gate checks manifest/model metadata against the embedded engine before download/use. Any active observation-profile or action-space change requires retraining or re-exporting live models; do not let shape drift fail silently.

ONNX export:

```bash
python code/tools/export_onnx.py --type mlp --input models/mlp_agent.zip --output models/mlp_agent.onnx --tensor-profile standard_lite_v2
python code/tools/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx --tensor-profile standard_lite_v2
```

Recurrent inference rule:

- SB3 LSTM and ONNX LSTM policies must reset recurrent state at episode boundaries.
- Evaluation loops must thread both state and episode-start flags correctly.

---

# 9. Commands

Run from repo root unless noted.
Examples use POSIX-style env prefixes. In PowerShell, use `$env:DIGIMON_BACKEND='rust'; <command>` or set the variable once for the session.

```bash
# Install
pip install -r requirements.txt
pip install -e .

# Pilot training
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training --tensor-profile standard_lite_v2 --timesteps 500000
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training --lstm --tensor-profile standard_lite_v2 --timesteps 500000
# Pool-based fictitious self-play (the `--self-play` flag is RETIRED — fails at
# startup; observations are P1-perspective only, so the old mode corrupted the
# policy. Emit the pool from the champion registry first.)
python code/tools/champion_admin.py emit-pool --out pool.json
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training --opponent pool --opponent-pool-manifest pool.json --tensor-profile standard_lite_v2 --timesteps 1000000
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training --gauntlet --tensor-profile standard_lite_v2 --timesteps 500000

# Architect training
python -m digimon_gym.agents.architect_training --archetype "Medusamon" --episodes 200

# Env smoke check
DIGIMON_BACKEND=rust python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(seed=1); print(obs.shape, info['tensor_profile'], info['action_mask'].shape)"
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(tensor_profile='standard_lite_deck_v2'); obs,info=env.reset(seed=1); print(obs.shape, info['tensor_profile'])"

# Rust backend PyO3 bindings
cd code/digimon-engine-py && maturin develop

# Rust binding surface tests
python -m pytest code/tests/test_rust_bindings_surface.py -v

# RL tests
python -m pytest code/tests/rl -v
python -m pytest code/tests/rl/test_tensor_profiles.py code/tests/rl/test_rust_runner_adapter.py -v
```

Do not run Uvicorn with `--reload` for long-running training workers; reload creates child processes that are not appropriate for worker jobs.

---

# 10. Implementation Rules for Contributors

1. Keep tensor/action contracts synchronized with `docs/TENSOR_SPEC.md`, `docs/ACTION_SPEC.md`, Rust constants, PyO3 exports, env wrappers, frontend constants, model metadata, and ONNX export.
2. Preserve headless-first game logic; UI reflects state and never owns rules.
3. Maintain legal action masking for every decision step.
4. Every engine choice that affects gameplay must flow through actions or pending selection.
5. Do not bypass masks in training, simulation, inference, greedy policy code, or test helpers.
6. Recurrent loops must thread `(state, episode_start)` correctly.
7. Reset recurrent state to `None` at episode boundaries.
8. `OpponentWrapper` discards dense rewards from opponent steps; only terminal rewards pass through.
9. WebSocket state broadcasts must use `state_filter.py`; never send raw `to_ui_json()` to network clients.
10. `state_filter.py` must redact both `handIds` and `handCards` for opponents.
11. Engine-only routers must not import DB/auth/AI pipeline modules.
12. Training CLI modules must not import DB/auth/FastAPI modules.
13. Desktop builds use `VITE_BUILD_TARGET=desktop` to tree-shake admin/training UI.
14. Observation code must use profile/layout metadata. Do not assume `digimon_engine.TENSOR_SIZE` is the active pilot observation size.
15. Default serious pilot training uses `standard_lite_v2`; compact compatibility checks must use explicit engine APIs, not `DigimonEnv`.
16. New Rust card effects are TDD: write a failing behavioral test under `code/digimon-engine/tests/` before implementing the `CardEffect` or DSL lowering.
17. Do not author new Python card scripts for cards already implemented in Rust.
18. Do not add no-op/raw-Rust placeholders to claim archetype readiness. Mark blocked gaps explicitly until behavior and tests are real.
19. All source code lives under `code/`; do not add new top-level source directories.

---

# 11. Generated Script Promotion Workflow (retired)

This workflow promoted generated Python card scripts into the frozen lane of the legacy Python engine. It no longer exists: `code/engine_py_legacy/` (including `script_promotion.py` and the scripts manifest) was deleted on 2026-06-15, and `code/tools/promote_script.py` is gone. New cards are authored DSL-first in Rust (§0.2, CLAUDE.md rule 28). Read the old workflow from git history if needed.

---

# 12. Documentation Index

Key docs:

- `CLAUDE.md`: current repository architecture and working rules.
- `docs/ARCHITECTURE.md`: detailed architecture reference.
- `docs/digimon-rules/`: verified rules derivations (`keyword-semantics.md`, `digest.md`, `rules-index.json`); `docs/RULES_CONTEXT.md` is a retired pointer.
- `docs/TENSOR_SPEC.md`: observation tensor layout.
- `docs/ACTION_SPEC.md`: action IDs and mask contract.
- `docs/TRAINING_RUNBOOK.md`: training pipeline operations.
- `docs/CLOUD_TRAINING.md`: cloud training (RunPod GPU / Hetzner-DO CPU), training image, run mirroring.
- `docs/MODEL_EVALUATION.md`: anchored evaluation (CLAUDE.md rule 30).
- `docs/REWARD_PROFILES.md`: reward profiles and the gameplay reward shape.
- `docs/TOOLS.md`: CLI tools and operational workflows.
- `docs/RUST_ENGINE_API.md`: Rust card scripting API.
- `docs/RUST_DSL_TEST_API.md`: Rust DSL test helpers and authoring checks.
- `docs/RUST_PYTHON_PARITY.md`: transitional Rust/Python divergence tracker.
- `docs/RUST_ENGINE_GAPS.md`: reusable Rust scripting capability gaps surfaced by archetype audits.
- `qa/dsl-vocab-gaps.md`: DSL vocabulary/lowering gaps surfaced by audits and batch card implementation.
- `qa/archetype-qa/engine-gaps.md`: known rule/card gaps that block no-approximations compliance.
- `.codex/skills/assess-rust-engine-archetype/`: Codex read-only DSL readiness assessment workflow for archetypes, decks, card groups, or card lists.
- `.codex/skills/plan-rust-engine-gap-group/`: Codex workflow for planning numbered Rust engine/DSL gap roadmap groups.


## Dedicated count-choice contract

Count prompts use existing actions 30–57 for commits and 58/59 for previous/next
page, with optional decline 62. They retain every count and bind the selected
declared value. Page navigation is not a gameplay decision that pays costs or
resolves effects. Read the live ACTION_SPEC and TENSOR_SPEC for the exact
reserved-field assignment and current schema/hash tuple; model compatibility
requires all six metadata fields even when dimensions did not change. JSON
count and physical-identity values are decimal strings. Do not infer them from
labels, encode them as imprecise JavaScript numbers, or accept old model
contracts solely because tensor/action shapes match.


### Model action semantic compatibility

Model training contracts require `action_schema_version` in addition to the six observation/action shape fields. Its sole authority is Rust `action::space::SCHEMA_VERSION`, exported by PyO3 as `ACTION_SCHEMA_VERSION`. Equal shapes do not imply compatible actions. Missing, malformed, or mismatched action semantics reject inference, checkpoint resume/export, publication, download and loading. Upload parsers preserve declared artifact values; they never stamp current semantics onto old weights. Legacy database rows remain null and incompatible. New training and newly initialized random exports may capture the current contract; re-export requires the authentic compatible training tuple. This preparatory gate does not bump SCHEMA_VERSION (still 1), resize actions/tensors, or complete the capacity migration.

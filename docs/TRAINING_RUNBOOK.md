# Training Runbook

Operational guide for the Digimon TCG RL training pipeline.
For architecture details, see `../AGENTS.md`.

> ## ⚠️ Action-space break — 2026-05-20 (Task S1.3)
>
> The engine action space grew from **2168** to **2192** actions (Task S1.3
> appended a breeding-carrier source-selection sub-range). This widens the
> policy/value action head, so **every model trained against the pre-S1.3
> 2168-action engine is incompatible and must be retrained from scratch**
> against a post-S1.3 engine — checkpoints cannot be resumed across the
> bump, and old ONNX exports cannot be served (see
> [MODEL_CATALOG.md](MODEL_CATALOG.md)).
>
> The project owner has explicitly accepted this break. After rebuilding
> the PyO3 bindings (`cd code/digimon-engine-py && maturin develop`), all
> `pilot_training` / `architect_training` runs start fresh. The default
> `standard_lite_v2` observation tensor is size-unchanged; only the action
> dimension (and its mask array) grew 2168 → 2192. The `standard_full_v2`
> profile additionally grew its `action_id_features` block — see
> [TENSOR_SPEC.md](TENSOR_SPEC.md).

> ## ⚠️ Observation break — 2026-05-20 (Task S1.4)
>
> Task S1.4 raised the v2 profiles' `PERM_MAX_SOURCES` from **11** to **12**
> so every selectable digivolution-source slot is observable. The default
> **`standard_lite_v2` observation tensor grew 8320 → 8410** floats
> (`feature_schema_version` `standard_lite_v2.2`); `standard_full_v2` grew
> **43392 → 43482** (`standard_full_v2.3`). This widens the policy/value
> **input** layer, so **every model predating `standard_lite_v2.2` is
> observation-incompatible and must be retrained from scratch** — bundle
> this with the S1.3 action-space retrain above as one breaking
> checkpoint. Rebuild the PyO3 bindings (`cd code/digimon-engine-py &&
> maturin develop`) before retraining so `digimon_engine` reports the new
> layout. `standard_compact_v1` (`1375`) is unchanged.

---

## 1. Quick Reference Commands

### CLI Training (pilot_training.py)

```bash
# MLP baseline vs greedy
python -m digimon_gym.agents.pilot_training --timesteps 500000

# LSTM vs greedy
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000

# Self-play
python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000

# With MetaGauntlet opponent sampling
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000

# With uniform random sampling across QA-clean gauntlet decks
python -m digimon_gym.agents.pilot_training --gauntlet --gauntlet-sampling random --timesteps 500000

# Generalist base pilot: sample both player decks from eligible Rust DSL archetypes
python -m digimon_gym.agents.pilot_training --generalist --curriculum-seed 123 --eval-seed 999 --timesteps 5000000

# Tensor-profile A/B run: reuse the same frozen generalist deck pool
python -m digimon_gym.agents.pilot_training --generalist --curriculum-pool models/generalist_a/deck_pool_snapshot.json --curriculum-seed 123 --eval-seed 999 --tensor-profile standard_lite_v2 --timesteps 5000000

# Fine-tune an archetype pilot from a compatible generalist base checkpoint
python -m digimon_gym.agents.pilot_training --init-from models/generalist_a/final.zip --deck1 path/to/deck.txt --gauntlet --gauntlet-sampling meta --lr 0.00005 --timesteps 1000000

# With custom deck
python -m digimon_gym.agents.pilot_training --deck1 path/to/deck.txt --timesteps 500000

# With a specific opponent deck
python -m digimon_gym.agents.pilot_training --deck1 path/to/deck.txt --deck2 path/to/opponent.txt --timesteps 500000

# Full LSTM + gauntlet with bounty tuning
python -m digimon_gym.agents.pilot_training --lstm --lstm-hidden-size 256 \
  --gauntlet --bounty-threshold 0.15 --bounty-bonus 0.5 \
  --timesteps 1000000 --eval-freq 20000 --eval-episodes 50
```

### All CLI Arguments

| Argument | Default | Description |
|---|---|---|
| `--timesteps` | 100000 | Total training timesteps |
| `--opponent` | greedy | Opponent policy (`greedy`, `random`) |
| `--self-play` | off | Agent plays both sides (mutually exclusive with `--opponent`) |
| `--lr` | 3e-4 | Learning rate |
| `--batch-size` | 64 | Minibatch size |
| `--n-steps` | 2048 | Rollout buffer size |
| `--eval-freq` | 10000 | Steps between evaluations |
| `--eval-episodes` | 20 | Games per evaluation |
| `--log-dir` | `runs/pilot_ppo` | TensorBoard log directory |
| `--save-dir` | `models` | Model save directory |
| `--gauntlet` | off | Enable MetaGauntlet opponent sampling from QA-clean fully implemented DSL archetypes |
| `--gauntlet-sampling` | meta | Sampling mode for `--gauntlet`: `meta` threat-index weights or `random` uniform deck sampling |
| `--generalist` | off | Sample both player decks from eligible fully implemented Rust DSL archetypes |
| `--curriculum-seed` | none | Seed for generalist deck-pair sampling, independent from the training seed |
| `--eval-seed` | none | Seed for generalist evaluation deck-pair sampling |
| `--curriculum-pool` | none | Reuse a frozen generalist deck-pool snapshot |
| `--curriculum-pool-out` | run directory | Write the frozen generalist deck-pool snapshot to this path |
| `--init-from` | none | Initialize a fine-tune run from a compatible base checkpoint |
| `--deck1` | none | Path to player 1 deck file |
| `--deck-json` / `--deck1-json` | none | Path to JSON file containing a flat list of player 1 card IDs |
| `--deck2` | none | Path to player 2 deck file; mutually exclusive with `--gauntlet` |
| `--deck2-json` | none | Path to JSON file containing a flat list of player 2 card IDs |
| `--bounty-threshold` | 0.15 | TI threshold for bounty bonus |
| `--bounty-bonus` | 0.5 | Bonus reward for beating high-TI opponents |
| `--lstm` | off | Use LSTM policy (MaskableRecurrentPPO) |
| `--lstm-hidden-size` | 256 | LSTM hidden units per layer |

---

## 2. MetaGauntlet Setup

### Building the Deck Library

```bash
python code/tools/meta_loader.py --build
```

- Scrapes tournament data from DigiLab, DigimonMeta, Egman Events.
- Outputs: `data/deck_library.json`.
- Format: archetypes → decklists + `digilab_stats`.
- Runtime gauntlet loading keeps only archetypes whose `qa/qa-reports/validated_cards_dsl.json` entries are all `IMPLEMENTED`, then keeps only decklists where every card ID is present in the Rust engine's implemented-card registry.

### Configuration Parameters

| Parameter | Default | Description |
|---|---|---|
| `alpha` | 1.0 | Weight on `meta_share` in TI formula |
| `beta` | 2.0 | Weight on `conversion_rate` in TI formula |
| `sleeper_threshold` | 0.50 | Conversion rate to trigger sleeper rule |
| `sleeper_floor` | 0.05 | Minimum 5% sampling for sleeper archetypes |
| `confidence_min_appearances` | 5 | Minimum DigiLab appearances before conversion factors into TI |

### Verifying MetaGauntlet State

```python
from digimon_gym.agents.gauntlet import MetaGauntlet

g = MetaGauntlet()
g.load()
print(f"Archetypes: {g.archetype_count}, Decks: {g.deck_count}")
for row in g.get_archetype_summary()[:10]:
    print(row)
```

---

## 3. Generalist Pilot Pretraining

Generalist pilot pretraining creates a reusable base weights file by exposing
the pilot to multiple fully implemented Rust DSL archetypes. Unlike gauntlet
training, which varies only the opponent deck, generalist mode samples both
`deck1` and `deck2` on each episode reset.

Sampling is intentionally broad:

1. Choose a fully eligible archetype uniformly.
2. Choose a deck uniformly from that archetype.
3. Repeat independently for `deck1` and `deck2`.

This avoids over-weighting archetypes simply because they have more decklists
in `data/deck_library.json`.

### Pretraining a Base Model

```bash
python -m digimon_gym.agents.pilot_training \
  --generalist \
  --curriculum-seed 123 \
  --eval-seed 999 \
  --tensor-profile standard_lite_v2 \
  --timesteps 5000000
```

At run start, the trainer writes a frozen `deck_pool_snapshot.json` unless
`--curriculum-pool` points at an existing snapshot. The snapshot records the
eligible archetypes, stable content-addressed deck IDs, deck contents, and a
snapshot hash. Reusing the same snapshot and `--curriculum-seed` keeps the
deck-pair curriculum stable even after `data/deck_library.json` is rebuilt.

### Tensor-Profile A/B Comparison

Use the same training seed, curriculum seed, eval seed, and frozen pool for
both runs. The model weights will not be bit-identical across tensor profiles,
but the sampled deck curriculum is held constant.

```bash
python -m digimon_gym.agents.pilot_training \
  --generalist \
  --curriculum-pool models/generalist_a/deck_pool_snapshot.json \
  --curriculum-seed 123 \
  --eval-seed 999 \
  --tensor-profile standard_lite_v2 \
  --timesteps 5000000
```

### Fine-Tuning an Archetype Pilot

Fine-tuning loads a compatible generalist checkpoint, validates the checkpoint's
tensor profile, tensor layout hash, and action-space size, then trains with the
requested fixed archetype deck and opponent curriculum.

```bash
python -m digimon_gym.agents.pilot_training \
  --init-from models/generalist_a/final.zip \
  --deck1 path/to/medusamon.txt \
  --gauntlet \
  --gauntlet-sampling meta \
  --lr 0.00005 \
  --timesteps 1000000
```

All explicit `--deck1` / `--deck2` inputs are validated against the Rust
implemented-card registry before training starts. Invalid decks fail fast and
list the missing card IDs.

---

## 4. GauntletOrchestrator Pipeline

### Overview

3-stage DB-backed training pipeline managed by `GauntletOrchestrator` (`code/server/workers/gauntlet_orchestrator.py`). Requires running backend (FastAPI + TrainingJobWorker).

### Stage Flow

```
configuring → stage_1 (bootstrap) → stage_2 (meta training) → stage_3 (evaluation) → completed
                                                                                    → failed (if >50% jobs fail)
```

### Stage 1: Bootstrap Training

- **What**: Each participant agent trains vs greedy opponent.
- **Job type**: `train_vs_greedy`
- **Duration**: `stage1_games * 50` (`STEPS_PER_GAME_ESTIMATE`) timesteps.
- **Output**: Initial agent weights.

### Stage 2: Meta-Weighted / PFSP Training

- **What**: Agents train against each other.
- **Core agents**: meta_weighted sampling (opponent weight = `meta_share`).
- **Supporting agents**: PFSP sampling (inverse win-rate, targets weak matchups).
- **Job type**: `train_vs_agent`
- **Duration**: `stage2_games * 50` timesteps.

### Stage 3: Round-Robin Evaluation

- **What**: All C(n,2) pairwise matchups between core agents.
- **Job type**: `evaluate`
- **Duration**: `stage3_games_per_matchup` games per pair.
- **Output**: Matchup matrix, ETWR rankings stored in gauntlet row.

### ETWR Formula

```
ETWR(A) = sum( win_rate(A, X) * meta_share(X) ) / sum( meta_share(X) )
           for all X != A
```

Interpretation: probability of beating a random meta-field opponent.

### Monitoring

- `TrainingJob` rows in DB (`status`: queued/running/completed/failed).
- `Agent` rows updated atomically with win/loss/draw counts.
- `Gauntlet` row holds `matchup_matrix_json` and `tournament_rankings_json`.

---

## 5. DeckPoolWrapper Usage

### Core/Flex Analysis

```python
from digimon_gym.agents.deck_pool import analyze_core

core, flex = analyze_core(card_ids)
# core: {card_id: count} for cards at max copies
# flex: {card_id: count} for cards below max copies
# Digi-Egg cards excluded from both
```

### Generating Variants

```python
from digimon_gym.agents.deck_pool import generate_variants

variants = generate_variants(
    base_deck=card_ids,
    core_cards=core,
    flex_cards=flex,
    side_cards=side_board_ids,
    count=8,
    seed=42,
)
# Returns list of valid 50-card deck variants
```

### Variant Generation Algorithm

1. Start from base deck counts.
2. Apply `n_mods` modifications (scales with flex pool size: 1-3, 2-5, or 3-8).
3. Each modification: 50% chance side-swap, 50% chance count-adjust.
4. Trim/grow to maintain exactly 50 main-deck cards.
5. Validate deck, deduplicate, return up to `count` variants.

### Modes

- `"eager"`: pre-generate all variants before training; uniform sampling.
- `"hybrid"`: 80% from pre-generated pool, 20% on-the-fly (capped at `hybrid_max_dynamic`, default 10).

---

## 6. LeagueOpponentWrapper

### Meta-Weighted Mode

- Opponent pool: list of `{agent_id, weights_path, algorithm, deck, weight}`.
- `weight` = `meta_share` from DigiLab stats.
- Sampling: proportional to weight (min 0.01).

### PFSP Mode (Prioritized Fictitious Self-Play)

- Uniform sampling until 5+ games per opponent.
- After 5 games: `weight = max(0.01, 1.0 - win_rate)`.
- Effect: focuses training on matchups the agent loses.

---

## 7. Wrapper Chain Reference

### Standard Training Chain

```
DigimonEnv                        (1375-obs, 2192-action, reward shaping)
  → OpponentWrapper              (single-agent MDP, auto-plays P2)
  → DeckPoolWrapper              (agent deck variation, optional)
  → GauntletWrapper              (opponent deck sampling from MetaGauntlet, optional)
  → ActionMasker                 (SB3 mask interface)
```

### make_env() Parameters

See `pilot_training.make_env()` for the full parameter list covering: opponent selection, deck overrides, gauntlet config, deck pool config, and bounty settings.

---

## 8. TensorBoard Monitoring

### Logged Metrics (WinRateCallback)

| Metric | Description |
|---|---|
| `pilot/win_rate` | Fraction of eval games won by Player 1 |
| `pilot/draw_rate` | Fraction of eval games that draw |
| `pilot/mean_eval_reward` | Average episode reward in eval |
| `pilot/mean_eval_episode_length` | Average steps per eval episode |
| `pilot/games_played` | Cumulative training episodes |
| `pilot/mean_eval_digivolves_per_game` | Agent (p1) regular digivolves per eval game |
| `pilot/mean_eval_dna_digivolves_per_game` | Agent (p1) DNA digivolves per eval game |
| `pilot/mean_eval_opponent_digivolves_per_game` | Opponent (p2) regular digivolves per eval game |
| `pilot/mean_eval_opponent_dna_digivolves_per_game` | Opponent (p2) DNA digivolves per eval game |
| `pilot/agent_archetype/<X>/digivolves_per_game` | Cumulative agent digivolves piloting `<X>` ÷ games as `<X>` |
| `pilot/agent_archetype/<X>/dna_digivolves_per_game` | Cumulative agent DNA digivolves piloting `<X>` ÷ games as `<X>` |
| `pilot/archetype/<X>/opponent_digivolves_per_game` | Cumulative opponent digivolves when opp is `<X>` ÷ games vs `<X>` |
| `pilot/archetype/<X>/opponent_dna_digivolves_per_game` | Cumulative opponent DNA digivolves when opp is `<X>` ÷ games vs `<X>` |

Digivolve telemetry fires unconditionally — it is observational, not gated on `digivolve_shaping`. Runs with shaping off emit the same scalar set with their actual (often zero) values, so the shaping-on vs. shaping-off A/B compare uses an identical schema.

### Viewing Logs

```bash
tensorboard --logdir runs/pilot_ppo
```

Default log directory: `runs/pilot_ppo` (override with `--log-dir`).

### Eval Sidecar (`runs/<name>/evals.jsonl`)

One JSON line per eval window. Top-level fields include the headline scalars plus four per-eval digivolve means:

| Field | Description |
|---|---|
| `step` / `wall_time` / `games_played` | Training-step, time, cumulative episodes |
| `win_rate` / `draw_rate` / `mean_reward` | Headline outcomes |
| `mean_terminal_score` / `mean_dense_reward` / `mean_eval_episode_length` | Reward decomposition |
| `mean_eval_digivolves_per_game` | Agent (p1) regular digivolves per game, this eval window |
| `mean_eval_dna_digivolves_per_game` | Agent (p1) DNA digivolves per game, this eval window |
| `mean_eval_opponent_digivolves_per_game` | Opponent (p2) regular digivolves per game |
| `mean_eval_opponent_dna_digivolves_per_game` | Opponent (p2) DNA digivolves per game |
| `by_archetype` | Object keyed by opponent archetype; see below |

`by_archetype` carries cumulative-since-callback-construction counts per opponent archetype:

```json
"by_archetype": {
  "DNA Omnimon": {
    "wins": 12, "draws": 1, "games": 30, "win_rate": 0.4,
    "digivolves": 28, "dna_digivolves": 0,
    "opponent_digivolves": 22, "opponent_dna_digivolves": 1
  }
}
```

**Naming asymmetry — important.** Within a `by_archetype` value, `digivolves` and `dna_digivolves` are the **agent's** counts in games where this entry's key was the opponent (sourced from `p1_*`). `opponent_digivolves` / `opponent_dna_digivolves` are the **opponent's** counts in those same games (sourced from `p2_*`). This mirrors the existing `wins` semantic (the agent's wins vs this opponent) — the `by_archetype` block is opponent-indexed, but its agent-side counters and opponent-side counters live side by side.

**Forward compatibility.** Sidecar rows written before this change lack the four top-level mean fields and the four per-archetype count fields. Lenient readers (the training MCP, ad-hoc `json.loads`-and-`.get`) work unchanged; strict whitelist readers need to widen.

---

## 9. Game Recording Artifacts

Pilot training can optionally write deterministic per-game recording artifacts
for bug triage. Recording is disabled by default so normal training runs do not
pay the storage or serialization cost.

Useful modes:

```bash
# Record only evaluation games
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training \
  --record-games eval --timesteps 100000

# Record draws/crashes/anomalies from train and eval episodes
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training \
  --record-games anomalies --record-games-max 25

# Sample ordinary games as well
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training \
  --record-games sampled --record-games-sample-rate 0.01
```

CLI/config controls:

| Option | Default | Description |
|---|---:|---|
| `--record-games` | `off` | One of `off`, `all`, `sampled`, `draws`, `anomalies`, or `eval` |
| `--record-games-dir` | `<run>/recordings` | Output directory for JSON artifacts |
| `--record-game-tensors` | false | Include per-step tensor and action-mask snapshots |
| `--record-games-max` | 25 | Maximum artifacts to save |
| `--record-games-sample-rate` | 0.01 | Sample rate for `sampled` mode |

Each artifact wraps the engine recording with run metadata and outcome metadata:

- `recording.initial_state`: post-shuffle deck, digitama, security, and opening-hand order.
- `recording.actions`: action IDs with player, phase, turn, and memory metadata.
- `outcome`: `winner_id`, `win_reason`, `draw_reason`, `terminated`, `truncated`, and step count.
- `run`: backend, tensor profile, action-space size, source split, environment index, and game index.

Tensor snapshots are useful for model debugging but can be large; keep them off
unless you need to inspect exact observations and masks. The current server
replay endpoints still use the legacy Python replay runner, so Rust training
recordings should be treated as deterministic bug artifacts first. A Rust-native
replay/seek tool can consume the same JSON contract in a follow-up.

---

## 10. Model Artifacts

### Save Location

- Default: `models/` directory.
- Filename: `pilot_ppo_{timestamp}.zip` (CLI) or `pilot_ppo_{job_id}.zip` (worker).

### Loading a Saved Model

```python
from sb3_contrib import MaskablePPO
from digimon_gym.agents.maskable_recurrent import MaskableRecurrentPPO

# MLP
model = MaskablePPO.load("models/pilot_ppo_20260228_120000")

# LSTM
model = MaskableRecurrentPPO.load("models/pilot_ppo_abc12345")
```

### Using as Opponent

```python
from digimon_gym.agents.pilot_training import make_agent_opponent_fn

opponent_fn = make_agent_opponent_fn(
    weights_path="models/pilot_ppo_20260228_120000",
    algorithm="mlp",  # or "lstm"
)
# opponent_fn(env) -> action_id
# For LSTM: opponent_fn.reset_state() between episodes
```

---

## 11. Training Job Worker Operations

### Starting the Worker

Worker auto-starts with the FastAPI server (unless `TRAINING_WORKER_DISABLED=1`).

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `TRAINING_WORKER_POLL_SECONDS` | 5.0 | Polling interval |
| `TRAINING_WORKER_STALE_SECONDS` | 7200 (2h) | Stale job timeout |
| `TRAINING_WORKER_MAX_CONCURRENT` | 1 | Max parallel jobs |
| `TRAINING_WORKER_DEVICES` | auto | Comma-separated devices, e.g. `cuda:0,cuda:1` |

### Device Assignment

- Auto-discovers CUDA GPUs via `torch.cuda`.
- Falls back to CPU if no GPUs.
- Round-robin assignment across available devices.

### Implementation Status

The DB queue mechanics, job claiming, stale recovery, and gauntlet hooks are fully implemented. The actual training execution methods (`_run_heuristic_training`, `_run_agent_training`, `_run_evaluation`) are currently placeholder stubs.

---

## 12. Dependencies

Key RL/ML packages:

- `gymnasium` >= 0.29
- `torch` >= 2.0
- `stable-baselines3` >= 2.0
- `sb3-contrib` >= 2.0
- `numpy` >= 1.24
- `tensorboard` (for monitoring)

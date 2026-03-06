# Training Runbook

Operational guide for the Digimon TCG RL training pipeline.
For architecture details, see `../AGENTS.md`.

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

# With custom deck
python -m digimon_gym.agents.pilot_training --deck1 path/to/deck.txt --timesteps 500000

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
| `--gauntlet` | off | Enable MetaGauntlet opponent sampling |
| `--deck1` | none | Path to player 1 deck file |
| `--bounty-threshold` | 0.15 | TI threshold for bounty bonus |
| `--bounty-bonus` | 0.5 | Bonus reward for beating high-TI opponents |
| `--lstm` | off | Use LSTM policy (MaskableRecurrentPPO) |
| `--lstm-hidden-size` | 256 | LSTM hidden units per layer |

---

## 2. MetaGauntlet Setup

### Building the Deck Library

```bash
python tools/meta_loader.py --build
```

- Scrapes tournament data from DigiLab, DigimonMeta, Egman Events.
- Outputs: `digimon_gym/engine/data/deck_library.json`.
- Format: archetypes → decklists + `digilab_stats`.

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

## 3. GauntletOrchestrator Pipeline

### Overview

3-stage DB-backed training pipeline managed by `GauntletOrchestrator` (`digimon_gym/agents/gauntlet_orchestrator.py`). Requires running backend (FastAPI + TrainingJobWorker).

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

## 4. DeckPoolWrapper Usage

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

## 5. LeagueOpponentWrapper

### Meta-Weighted Mode

- Opponent pool: list of `{agent_id, weights_path, algorithm, deck, weight}`.
- `weight` = `meta_share` from DigiLab stats.
- Sampling: proportional to weight (min 0.01).

### PFSP Mode (Prioritized Fictitious Self-Play)

- Uniform sampling until 5+ games per opponent.
- After 5 games: `weight = max(0.01, 1.0 - win_rate)`.
- Effect: focuses training on matchups the agent loses.

---

## 6. Wrapper Chain Reference

### Standard Training Chain

```
DigimonEnv                        (1375-obs, 2168-action, reward shaping)
  → OpponentWrapper              (single-agent MDP, auto-plays P2)
  → DeckPoolWrapper              (agent deck variation, optional)
  → GauntletWrapper              (opponent deck sampling from MetaGauntlet, optional)
  → ActionMasker                 (SB3 mask interface)
```

### make_env() Parameters

See `pilot_training.make_env()` for the full parameter list covering: opponent selection, deck overrides, gauntlet config, deck pool config, and bounty settings.

---

## 7. TensorBoard Monitoring

### Logged Metrics (WinRateCallback)

| Metric | Description |
|---|---|
| `pilot/win_rate` | Fraction of eval games won by Player 1 |
| `pilot/draw_rate` | Fraction of eval games that draw |
| `pilot/mean_eval_reward` | Average episode reward in eval |
| `pilot/mean_eval_episode_length` | Average steps per eval episode |
| `pilot/games_played` | Cumulative training episodes |

### Viewing Logs

```bash
tensorboard --logdir runs/pilot_ppo
```

Default log directory: `runs/pilot_ppo` (override with `--log-dir`).

---

## 8. Model Artifacts

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

## 9. Training Job Worker Operations

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

## 10. Dependencies

Key RL/ML packages:

- `gymnasium` >= 0.29
- `torch` >= 2.0
- `stable-baselines3` >= 2.0
- `sb3-contrib` >= 2.0
- `numpy` >= 1.24
- `tensorboard` (for monitoring)

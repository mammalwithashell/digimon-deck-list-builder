# Training Runbook

Operational guide for the Digimon TCG RL training pipeline.
For architecture details, see `../AGENTS.md`.

> **Running training in the cloud?** See [CLOUD_TRAINING.md](CLOUD_TRAINING.md) —
> end-to-end runbook for Hetzner/DO CPU droplets with Tailscale-based
> TensorBoard access and rsync-mirrored `runs/` so `digimon-training-mcp`
> can query cloud runs from your local Claude sessions. Use it for long
> (>~8h) jobs, off-machine training, or phone-checkable runs.

> ## ⚠️ Engine default profile flipped — 2026-05-25 (`flip-engine-default-to-lite-deck-v2`)
>
> The engine's canonical default observation profile is now
> **`standard_lite_deck_v2`** (`8850` floats; v2_lite prefix + 55-row own
> original decklist + 256 reserved). `tensor::TENSOR_SIZE`,
> `tensor_profiles::default_profile()`, and the PyO3
> `digimon_engine.TENSOR_PROFILE_ID` all report this value. Previously the
> default was `standard_compact_v1` (1375).
>
> **Existing v1-trained ONNX checkpoints stop loading on desktop and via
> the engine's default PyO3 surface.** They can still be loaded by pinning
> `observation_profile="standard_compact_v1"` explicitly on
> `RustHeadlessGame` and routing through a v1-shaped inference path, but
> the desktop bundled-manifest gate now rejects them. Retrain or re-export
> against `standard_lite_deck_v2` (or `standard_lite_v2` for the no-deck
> variant). This is bundled with the S1.3 / S1.4 retrain precedent.

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

> ## ⚠️ Self-play retirement + reward-YAML fail-fast — 2026-06-11 (`harden-training-pipeline`)
>
> **`opponent="self-play"` / `--self-play` is RETIRED and fails at startup.**
> `DigimonEnv` builds observations from Player 1's perspective only; the old
> mode skipped `OpponentWrapper`, so the learner picked Player 2's actions
> against wrong-perspective input — silently corrupting the policy. (The
> 2026-05-31 self-play run collapsed to 22.5% vs greedy while its in-run eval
> read a flat 100%.) Use **pool-based fictitious self-play** instead: train
> against frozen champions via `--opponent pool --opponent-pool-manifest
> pool.json`, where the manifest is emitted from the champion registry with
> `python code/tools/champion_admin.py emit-pool --out pool.json`. Promotion
> grows the pool between runs (see the standing-cadence section).
>
> **Reward-YAML loading is now fail-fast (BREAKING).** A configured
> `reward_profiles_path` / `reward_gameplay_path` (including the defaults)
> whose file does not exist raises `FileNotFoundError` at run start, instead
> of silently training with legacy rewards. To intentionally train with the
> legacy reward path, set `reward_profiles_path: null` explicitly.

---

## 0. Pre-flight: release-mode bindings

Always train against a **release-mode** build of the Rust engine + PyO3 bindings. The `dev` profile is ~10× slower per engine step and turns a 1M-step training run into days. Recompile after every engine change (and after every checkout of new commits):

```bash
# Build the release wheel
cd code/digimon-engine-py
python -m maturin build --release

# Install it into the current Python env (overwrites prior install)
pip install --force-reinstall --no-deps \
  ../../target/wheels/digimon_engine-0.1.0-cp311-abi3-win_amd64.whl
# (substitute the actual wheel filename — abi3 wheel is platform-tagged
#  and the version may have moved; pick the freshest from target/wheels/)
```

If you have a `.venv` configured, `python -m maturin develop --release` does both steps in one command (compile + install). Without a venv, the build+pip flow above is the equivalent. Either way, **never train against a `--debug` or default-profile wheel**.

### Verifying release mode

```bash
python -c "
from digimon_engine import RustHeadlessGame
# A fresh game in release mode should construct in well under 100ms.
import time
t = time.perf_counter()
g = RustHeadlessGame(['ST1-01']*5 + ['ST1-03']*45, ['ST1-01']*5 + ['ST1-03']*45, seed=1)
print(f'Constructed in {(time.perf_counter() - t)*1000:.1f}ms')
"
# Dev-mode build: ~500–1000ms. Release build: <100ms.
```

### Panic safety rail (always on)

`pilot_training` inserts `TrainingRecordingWrapper` into the env chain unconditionally — it catches PyO3 `PanicException` (and any other `BaseException` from the inner engine step), logs the crash to stderr, increments a class-level `crash_count`, and synthesises a terminal step so SB3's VecEnv auto-resets and the run continues. **You don't need `--record-games anomalies` to get this protection**; that flag controls whether the crash is also persisted as a JSON artifact for triage. Set `--record-games anomalies` (or `--record-games all`) on long unattended runs if you want post-mortem artifacts.

---

## 1. Quick Reference Commands

### CLI Training (pilot_training.py)

```bash
# MLP baseline vs greedy
python -m digimon_gym.agents.pilot_training --timesteps 500000

# LSTM vs greedy
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000

# Pool-based fictitious self-play (train vs frozen champions; the old
# `--self-play` flag is RETIRED — see "Self-play retirement" below)
python code/tools/champion_admin.py emit-pool --out pool.json
python -m digimon_gym.agents.pilot_training --opponent pool --opponent-pool-manifest pool.json --timesteps 1000000

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

# Deck-specialist league: turn one generalist into six per-deck specialists
# (add-deck-specialist-league). Round-based PFSP against frozen snapshots.
python code/tools/train_specialist_league.py --generalist models/starter_pool_v1/final.zip --rounds 3 --dry-run
python code/tools/train_specialist_league.py --generalist models/starter_pool_v1/final.zip --rounds 3 --steps-per-round 1000000 --eval-n 24
# Throughput-tuned launch (the league is OPPONENT-INFERENCE-bound — see
# "Throughput levers for neural-opponent runs"). FIRST register the generalist
# as a champion, or the in-training anchored panel degrades to greedy-only:
#   python code/tools/champion_admin.py promote --candidate models/starter_pool_v1/final.zip --name generalist-v1 --force
DIGIMON_ONNX_OPPONENT=1 python code/tools/train_specialist_league.py \
  --generalist models/starter_pool_v1/final.zip --rounds 3 --steps-per-round 500000 \
  --n-envs 8 --eval-n 24 --promote-min-wr 0.55 \
  --batch-size 256 --anchored-eval-freq 50000 --anchored-eval-games 48 \
  --eval-freq 100000 --eval-episodes 8        # ONNX opponent + tuned cadence ≈ 2x steps/sec
# Warm-start is DECOUPLED from the gate (decouple-league-warmstart-from-gate):
#   --warmstart accumulate  (DEFAULT) each round continues the deck's OWN latest
#                           round checkpoint (round 1 from the generalist), so a
#                           deck that fails the gate ("kept") compounds experience
#                           instead of re-rolling from the generalist. The gate still
#                           governs only the opponent pool + registry (keep-best),
#                           so the pool stays the gated champions.
#   --warmstart champion    legacy/gate-coupled — warm-start from the registry
#                           champion; pass this to reproduce the pre-decoupling run.
# Experiment tracking: add --wandb (+ --wandb-project / --wandb-group / --wandb-mode)
# to log every specialist to Weights & Biases. It uses wandb.init(sync_tensorboard=True),
# so ALL existing TensorBoard scalars (eval win rate, anchored panels, reward curves)
# mirror to W&B with no extra instrumentation; runs are grouped (default group
# "specialist-league") with per-deck/round names "<deck>_r<rnd>". The API key is read
# from WANDB_API_KEY in the env (pass -e WANDB_API_KEY=... to the container; never bake
# it into the image). Online mode auto-downgrades to offline if the key is missing
# (buffer locally, `wandb sync` later). Flag is OFF by default — runs are byte-identical
# without it. Same --wandb flag works on a bare `pilot_training` run.

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
| `--self-play` | — | **RETIRED** — fails at startup with migration guidance (see "Self-play retirement") |
| `--opponent-pool-manifest` | none | OpponentPool manifest JSON for `--opponent pool` (emit from the champion registry via `champion_admin.py emit-pool`) |
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

## 6.5 Throughput levers for neural-opponent runs

**Diagnosis first.** A `--opponent league`/`pool` run runs the frozen opponent's
policy forward pass on EVERY env step, which dominates per-step cost: such runs
are **~3× slower than the no-neural-opponent greedy floor** (e.g. cpx62: ~22
steps/sec league vs ~59 floor) and are **inference-bound, not core-bound** — more
cores past `n_envs` won't help (contrast the update-bound ceiling of vs-greedy
runs). Confirm which regime you're in: if the
rollout games/sec doesn't move when you change `batch_size`/eval cadence, you're
inference-bound and the ONNX opponent is the real lever.

Levers, in order of impact for an inference-bound run — **none require more
compute**:

| Lever | How | Effect |
|---|---|---|
| **ONNX opponent** | `DIGIMON_ONNX_OPPONENT=1` | frozen MLP opponent runs via ORT (1 intra-op thread/subproc) instead of torch eager → **~2× games/sec**. Parity-safe: opponent plays deterministic argmax, so identical logits → identical action. MLP-only; torch fallback on failure. |
| Update phase | `--batch-size 256` (default 64), optionally `--n-epochs` ↓ | fewer, bigger PPO minibatches. Helps when update-bound; harmless otherwise. `n_epochs`↓ trades sample reuse — be conservative. |
| Eval overhead | `--eval-freq 100000 --eval-episodes 8` (from 50k/20) | the in-run win rate is **degenerate** (rule 30 — see §14); minimise time spent computing a number you don't trust. |
| Logging / hot-reload | mulligan log off; `reward_profiles_hot_reload=False` | trims per-game JSON I/O + per-step file stat. Minor. |

**ONNX-opponent gotchas** (each cost an image rebuild before it worked): the
export needs `dynamo=False` (the
legacy TorchScript exporter; torch≥2.x defaults to the dynamo path which needs
`onnxscript`, absent from the training image) **and** `onnx` in
`requirements-training.txt` (the image shipped only `onnxruntime`). The lazy
export uses a **pid-unique temp** because N subprocs race to export the same
opponent at startup. Tell-tale that it silently fell back: `ONNX opponent for ...
failed (...); falling back to torch.` in `docker logs`, and no `*.opponent.onnx`
appearing next to the `.zip`.

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

### Per-Game Eval Log (`models/<name>/eval_game_log.jsonl`)

One JSON line per **completed eval game** (not per eval window). Written by `WinRateCallback._run_evaluation` immediately after each per-game iteration of the eval loop computes its outcome. Default on; toggle with `--eval-game-log {on,off}`.

This is the *raw* layer under the eval-window means above. The mean of 0.4 digivolves/game in the sidecar can't distinguish "one whale game with 4 digivolves" from "consistent 0.4 across 10 games" — the per-game rows here answer those questions.

**One row = one inner game, including in BO3 mode** — a 3-game match emits 3 rows.

| Field | Description |
|---|---|
| `step` | Training step at the moment the eval window started; same for all rows in one window |
| `eval_window_idx` | 0-based monotonic index of the eval window within the run |
| `game_idx` | 0-based game index within the eval window (monotonic across the whole window; in BO3, increments per inner game, not per match) |
| `source` | Always `"eval"` for v1; schema-stable for future training-side rows |
| `match_format` | `"single"` or `"bo3"` — the run's `TrainingConfig.match_format` |
| `match_idx` | BO3 only: 0-based index of the BO3 match this row belongs to within the window. Null in single mode. |
| `game_in_match_idx` | BO3 only: 0/1/2 — which inner game of the BO3 match. Null in single mode. |
| `agent_archetype` / `opponent_archetype` | From the env's `info` dict (null outside generalist/gauntlet modes) |
| `digivolves_agent` / `dna_digivolves_agent` | Agent's (p1) per-game counts captured at the moment that inner game ended (BO3: via `MatchEnv.match_game_history`; single: via `_rl_state()` at episode end) |
| `digivolves_opponent` / `dna_digivolves_opponent` | Same for opponent (p2) |
| `result` | `"win"` / `"loss"` / `"draw"` — per inner game |
| `episode_length` | Env steps inside the inner game (BO3) or the whole episode (single) |
| `terminal_score` | ±1.0 / 0.0 per inner game (no dense shaping) |
| `recording_path` | Absolute path to the recording file. In BO3 mode, recordings are written at match end so all rows from one match share the same path. Null when recording is off. |

```jsonl
// single mode
{"step": 100000, "eval_window_idx": 4, "game_idx": 0, "source": "eval", "match_format": "single", "match_idx": null, "game_in_match_idx": null, "agent_archetype": "BlueFlare", "opponent_archetype": "Omnimon", "digivolves_agent": 2, "dna_digivolves_agent": 1, "digivolves_opponent": 3, "dna_digivolves_opponent": 0, "result": "win", "episode_length": 18, "terminal_score": 1.0, "recording_path": ".../win_decked_out.json"}

// BO3 mode — one match producing three rows
{"step": 100000, "eval_window_idx": 4, "game_idx": 0, "source": "eval", "match_format": "bo3", "match_idx": 0, "game_in_match_idx": 0, "agent_archetype": "BlueFlare", "opponent_archetype": "Omnimon", "digivolves_agent": 2, "dna_digivolves_agent": 1, "digivolves_opponent": 1, "dna_digivolves_opponent": 0, "result": "win", "episode_length": 21, "terminal_score": 1.0, "recording_path": ".../match_recording.json"}
{"step": 100000, "eval_window_idx": 4, "game_idx": 1, "source": "eval", "match_format": "bo3", "match_idx": 0, "game_in_match_idx": 1, ..., "result": "loss", "terminal_score": -1.0, "recording_path": ".../match_recording.json"}
{"step": 100000, "eval_window_idx": 4, "game_idx": 2, "source": "eval", "match_format": "bo3", "match_idx": 0, "game_in_match_idx": 2, ..., "result": "win", "terminal_score": 1.0, "recording_path": ".../match_recording.json"}
```

Query via the training MCP:

```
run_per_game_evals(name="generalist_v2", filter={dna_digivolves_agent_min: 3})
  → row.recording_path → digimon-engine-mcp:load_recording → step through
```

See [TRAINING_MCP.md](TRAINING_MCP.md#run_per_game_evalsname-filter-limit) for the full filter set.

---

## 9. Game Recording Artifacts

Pilot training can optionally write deterministic per-game recording artifacts
for bug triage. Recording is disabled by default so normal training runs do not
pay the storage or serialization cost.

**Note**: the underlying `TrainingRecordingWrapper` is now inserted into the env
chain unconditionally — it is the **panic safety rail** that survives engine
crashes regardless of the `--record-games` setting (see §0). The
`--record-games` flag controls only whether the wrapper PERSISTS crash + game
artifacts to disk. Set it to `anomalies` (or `all`) on long unattended runs if
you want JSON artifacts to triage from after a crash.

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

## 12. Best-of-three Match Training

Default Gym episode shape (since `add-bo3-match-training`, 2026-05). One episode = one best-of-three match; deck pair sampled once per match and held across all games; LSTM hidden state carries across games within a match. Legacy single-game episodes still available via `--match-format single`.

### CLI

```bash
# Default — BO3 with concede + play-order selection enabled
python -m digimon_gym.agents.pilot_training --generalist --timesteps 500000

# Opt out — legacy single-game episodes
python -m digimon_gym.agents.pilot_training --generalist --match-format single

# Override digivolve shaping default (BO3 turns it on)
python -m digimon_gym.agents.pilot_training --set digivolve_shaping=false
```

### Action surface

| Action | Meaning | Mask rule |
|---|---|---|
| `93` | Concede game (`Game::concede(player)`) | Legal whenever player has any other legal action |
| `94` | Play first in next game | Legal only during `SelectPlayOrder` |
| `95` | Play second in next game | Legal only during `SelectPlayOrder` |

`ACTION_SPACE_SIZE` is unchanged at `2192`; actions `93`/`94`/`95` occupy the previously-unused `93-99` range. Existing checkpoints can be loaded but will produce near-random behavior on the new actions until additional fine-tune timesteps.

### Reward calibration

```
Per-step dense:
  +1.5 per opponent security removed (asymmetric — was ±2.0 symmetric)
  -0.5 per own security lost
  +0.1 per agent digivolve, +0.4 per DNA digivolve (default ON in BO3)
  -0.001 step penalty

Per-game terminal (BO3 only — replaces DigimonEnv's ±10 + up to +5 fast):
  +12 win, -12 loss, + up to +3 fast-game bonus (par 50, zero at 150)

Per-match terminal (fires at match end):
  +30 match win, -30 match loss, -1 draw (rare, hard step-limit only)
  +10 sweep bonus (2-0 wins)
  +5 smart-concede bonus (won match AND any conceded game)
  -10 scared-concede penalty (0-2 loss AND any conceded game)
  + up to +15 fast-match bonus (par 150, zero at 450, win only)
```

See `openspec/changes/add-bo3-match-training/design.md` §D9 for the full calibration rationale and the scenario-by-scenario payoff table.

### Eval cost

In BO3 mode, `--eval-episodes N` evaluates `N` matches ≈ `2.5 × N` games. Default eval frequency is unchanged; consider reducing `--eval-episodes` for match-format runs if eval cost becomes prohibitive.

### Wrapper chain

```
DigimonEnv → OpponentWrapper → MatchEnv → GeneralistDeckPoolWrapper (or DeckPoolWrapper)
                                        → GauntletWrapper → TrainingRecordingWrapper
                                        → MulliganLogWrapper → ActionMasker
```

`MatchEnv` sits immediately above `OpponentWrapper` so OpponentWrapper sees one continuous episode (= one match) and LSTM hidden state threads normally across games-within-match. The deck-pool wrappers sit ABOVE `MatchEnv` so deck sampling fires once per match (not per game). `OpponentWrapper.reset_inner_only(...)` is the per-game inner-reset path used by `MatchEnv` between games — it resets `DigimonEnv` without resetting `opponent_fn.reset_state`, preserving the opponent's recurrent state.

### Checkpoint compatibility

Loading a pre-BO3 checkpoint into a BO3 run will:
- ✅ Work — observation tensor and action space size unchanged.
- ⚠️ Produce near-random behavior on actions 93/94/95 because those were never seen during the checkpoint's training.

Recommended fine-tune procedure: load checkpoint, run ~100k–500k timesteps in `--match-format bo3` to teach the policy when concede + play-order picks are valuable, evaluate, then continue full training.

## 13. Reward profiles

Composable, YAML-defined reward shaping per archetype. Full reference: [REWARD_PROFILES.md](REWARD_PROFILES.md).

**Two-file layout** (since `add-gameplay-reward-config`, 2026-05): reward shaping is split across two sibling YAMLs loaded together — `code/digimon_gym/agents/reward/gameplay.yaml` (universal game-mechanic shape, one profile `gameplay`) and `code/digimon_gym/agents/reward/profiles.yaml` (archetype overlays, every profile MUST `inherits: gameplay`). `ProfileLoader` takes both paths via `gameplay_path=` and `profiles_path=`; the `TrainingConfig.reward_gameplay_path` field defaults to the gameplay path.

**Default behavior** (no config change): the shipped `gameplay` profile applies the universal "win fast or it hurts" shape — `quick_win_bonus` peaks at +5 on turn 3 and decays to 0 by turn 7, `stall_penalty` starts at turn 8 and grows quadratically without bound (terminal landscape: turn 3 win = +15, turn 7 win = +10, turn 20 loss = −26.9, turn 30 draw = −53.9). The `_default` profile is now a thin pass-through to `gameplay` — the byte-identical-to-legacy guarantee is gone.

**Selecting a profile**:

```yaml
# training_config.yaml — three modes
# 1) Default (no change): _default profile = gameplay shape.
# 2) Force a specific profile regardless of archetype:
reward_profile_override: dna_omnimon_combo_v1

# 3) Per-archetype assignment (generalist mode):
generalist: true
# `assignments:` in profiles.yaml drives per-episode profile pick from
# info["deck1_archetype"]. Unknown archetypes fall back to `_default`.
```

**Authoring a new profile**: edit `code/digimon_gym/agents/reward/profiles.yaml`. Add a profile that `inherits: gameplay` and declare a `key_cards:` block with the archetype's win-condition cards. Hot-reload (`reward_profiles_hot_reload: true` by default) means edits take effect at the next env reset without restarting training.

**Resume safety**: at run-start, a `<run_dir>/reward_profiles.meta.json` sidecar records BOTH file hashes (`reward_gameplay_hash`, `reward_profiles_hash`) plus paths and the override/assignments snapshot (6 fields total). On resume both hashes are re-checked; mismatch fails with the message naming which file (`gameplay.yaml` or `profiles.yaml`) drifted. Override with `--reward-profiles-override-mismatch` only when intentionally switching reward shape mid-run — the flag covers both files.

**Telemetry**: per-component, per-profile, and boss-arrival scalars surface as `pilot/reward/*`, `pilot/profile/*`, and `pilot/mean_eval_digivolves_into_boss_per_game` in TensorBoard. Two new gameplay-shape TB scalars are emitted per eval window: `pilot/mean_eval_winning_turn` (mean `turn_count` at terminal across agent-win games; not emitted when no wins) and `pilot/mean_eval_digivolve_driven_attacks` (mean agent-side `DigivolveDrivenAttack` count per eval game). Per-archetype × component drilldowns land in `evals.jsonl` under `by_archetype[X].component_means` and `by_agent_archetype[X]`.

**Deprecation**: `digivolve_reward` / `dna_digivolve_bonus` flat fields warn when set to non-default values. `digivolve_shaping: true` is now INERT — it is accepted with no warning and has no effect on profile selection (the universal gameplay shape always carries the new digivolve weights). The `legacy_terminal_exclusivity` flag, the `_digivolve_shaped` profile, and the `_base_terminal` profile have all been removed; the loader errors with a migration message if YAML still sets the flag. Flat-field removal still targeted for v2.

## 14. Standing cadence: the champion loop

The loop that turns individual runs into monotonic progress
(`harden-training-pipeline`; see `docs/MODEL_EVALUATION.md` for the
rationale). One cycle:

```bash
# 1. Derive the opponent pool from the champion registry (uniform weights;
#    PFSP reweighting happens at sample time).
python code/tools/champion_admin.py emit-pool --out pool.json

# 2. Train against the frozen pool (pool-based fictitious self-play).
#    The in-training anchored panel (anchored_eval_freq, default 100k)
#    gives a trustworthy in-run curve vs greedy + champions.
python -m digimon_gym.agents.pilot_training --generalist \
  --opponent pool --opponent-pool-manifest pool.json --timesteps 1000000

# 3. Evaluate the result against the FIXED reference frame.
python code/tools/anchored_eval_cli.py --candidate models/<run>/final.zip \
  --deck-pool-snapshot models/<run>/deck_pool_snapshot.json --n 100
python code/tools/elo_ladder_cli.py --run models/<run>     # forgetting check

# 4. Gated promotion (≥55% vs the compatible champion panel, seat-balanced).
python code/tools/champion_admin.py promote --candidate models/<run>/final.zip \
  --name v<NN> --deck-pool-snapshot models/<run>/deck_pool_snapshot.json

# 5. The registry grew — the NEXT run's step 1 derives a larger pool.
```

**Promotion decisions come ONLY from the anchored frame** (anchored eval /
the gate panel / the Elo ladder). The in-run training-opponent win rate and
any mirror metric are never promotion evidence — CLAUDE.md rule 30. The
`pilot/anchored/*` scalars exist precisely so a collapsing run is visible
mid-run; they are still in-run conveniences, and the gate panel is the
decision of record.

> **The anchored panel needs anchors.** It plays vs greedy + *every
> layout-compatible champion in the registry* (`models/champions/registry.json`).
> If the registry is empty — common on a fresh cloud box — the panel silently
> degrades to `vs ['greedy']` only, a single noisy anchor (the startup log line
> reads `Anchored eval: ... vs ['greedy']`). Before a warm-started run (e.g. the
> deck-specialist league), **register the seed as a champion** so the panel
> answers the question that matters — "is this beating what it was seeded from?":
> `champion_admin.py promote --candidate <seed>.zip --name <name> --force`
> (`--force` skips the gate; the seed must be layout-compatible — same
> `tensor_layout_hash`). The callback builds its anchor list lazily at the first
> panel, so register **before** launch, not mid-run. Raise `--anchored-eval-games`
> (default 24) to cut the per-panel noise.

**Recorded promotion decisions:**

| Date | Candidate | Decision | Evidence |
|---|---|---|---|
| 2026-05-31 | `v020-generalist-v1` | registered (seed, `--force`) | first champion; 65% in-run vs greedy |
| 2026-05-31 | `v022-generalist-v1` | registered | best model; ~77.5% anchored vs greedy |
| 2026-06-11 | `starter1_6_flat_control_v1` | **not registered — candidate weights lost** | The 2026-05-31 control run (fresh + vs-greedy + `starter1_6_flat` reward) reached ~70–85% vs greedy and validated the flat reward, but its `final.zip` no longer exists locally (`cloud_downloads/starter1_6_flat_control_v1/` absent) and the RunPod pod was terminated without a volume. Gate panel could not be played. Follow-up: re-train the recipe fresh (it is cheap — vs-greedy, ~1M steps) and gate that result instead. |

## 15. Dependencies

Key RL/ML packages:

- `gymnasium` >= 0.29
- `torch` >= 2.0
- `stable-baselines3` >= 2.0
- `sb3-contrib` >= 2.0
- `numpy` >= 1.24
- `tensorboard` (for monitoring)

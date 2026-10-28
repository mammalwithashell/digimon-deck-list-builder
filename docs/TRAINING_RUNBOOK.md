# Training Runbook

Operational guide for the Digimon TCG RL training pipeline.
For architecture details, see `../AGENTS.md`.

For periodic per-card usage, entry routes, legal opportunities, W&B checkpoint
Tables, and the local/W&B dashboard, see [Card evaluation](CARD_EVALUATION.md).

> **Running training in the cloud?** See [CLOUD_TRAINING.md](CLOUD_TRAINING.md) —
> end-to-end runbook for Hetzner/DO CPU droplets with Tailscale-based
> TensorBoard access and rsync-mirrored `runs/` so `digimon-training-mcp`
> can query cloud runs from your local Claude sessions. Use it for long
> (>~8h) jobs, off-machine training, or phone-checkable runs.

## Owned source/security choices: current contract

`support-complete-digivolution-source-choices` uses action semantic schema **2**
with **2192** actions. Pending card pages carry 128-float rows; active profile
sizes are `standard_lite_v2` **9434**, `standard_lite_deck_v2` **9874**, and
`standard_full_v2` **44506**. Resolve sizes, positions, schemas and hashes from
`digimon_engine.get_observation_layout()`; the older migration numbers below
record earlier allocations.

Rebuild the optimized PyO3 wheel before training. Old checkpoints and exports
must fail the seven-field model contract, including `action_schema_version`,
even when an action-head size happens to match. Do not relabel old metadata to
accept old weights. Newly initialized current-contract models and their exports
can be validated independently of policy quality; a compatibility smoke test is
not model promotion or oracle certification.

Paged choices include legal commit rows, navigation and permission-filtered
inspection rows. Navigation never pays a cost or advances effect processing.
Refresh rechecks live eligibility; impossible mandatory completion contributes
no payment or reduction credit. The action mask remains the legality oracle.
See [ACTION_SPEC.md](ACTION_SPEC.md) and [TENSOR_SPEC.md](TENSOR_SPEC.md).

> ## ⚠️ Engine default profile flipped — 2026-05-25 (`flip-engine-default-to-lite-deck-v2`)
>
> The engine's canonical default observation profile is now
> **`standard_lite_deck_v2`** (currently `9874` floats, `8850` at the original flip; v2_lite prefix + 55-row own
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

# With uniform random sampling across training-ready gauntlet decks
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
# LSTM specialists: add --lstm to train MaskableRecurrentPPO specialists. Cross-arch
# warm-start is supported — an MLP generalist transfers ONLY its shared
# CardEmbeddingExtractor into the LSTM specialists via --init-extractor-from (the
# source is loaded by ITS OWN arch, read from <model>.meta.json, not the target's);
# --init-from is full-checkpoint and requires matching arch. Promotion gating + the
# 6x6 cross-deck matrix come from --promote-min-wr (keep-best threshold) and --eval-n
# (games per matrix cell — small n is deck-luck noisy; use >=24).
#   python code/tools/train_specialist_league.py --generalist models/starter_pool_v1/final.zip \
#     --lstm --init-extractor-from models/starter_pool_v1/final.zip \
#     --rounds 3 --steps-per-round 500000 --eval-n 24 --promote-min-wr 0.55
# Scaling a league ACROSS pods: --topology parallel (multiple specialists in ONE pod)
# is reliable but NOT a proven speedup and is confounded by RunPod CPU-cgroup
# throttling — see docs/CLOUD_TRAINING.md §A.11. The scalable path is one specialist
# per pod fanned out across pods (OpenSpec: add-cross-pod-league-orchestration);
# provision/verify pods by cpu.max (not nproc) and harvest finals before terminate.

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
| `--gauntlet` | off | Enable MetaGauntlet opponent sampling from training-ready decklists (see §2) |
| `--gauntlet-sampling` | meta | Sampling mode for `--gauntlet`: `meta` threat-index weights or `random` uniform deck sampling |
| `--generalist` | off | Sample both player decks from training-ready decklists (see §2) |
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
- Runtime gauntlet loading applies a two-layer training gate (spec `docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md` §5b). It reads only two committed `data/` artifacts, because the training image ships no `qa/`; a missing artifact stops the load (`MechanicsReadinessMissingError` / `CardAdmissibilityMissingError`, each naming its regenerate command):
  1. **Mechanics (global).** `data/mechanics_readiness.json` must be `ready`: no mechanics-set rules obligation is flagged outcome-changing (`blocks_training`). This is checked once per load, before any deck is examined. Otherwise `MechanicsNotReadyError` names the blocking rule and gap ids. Regenerate with `PYTHONPATH=code python -m tools.rules.mechanics_readiness`. See [What `ready` means](#what-ready-means-and-what-it-does-not) for what the flag does and does not cover.
  2. **Light per-card (per decklist).** Every card ID must be playable, meaning registered in the Rust engine and present in `data/cards.json` (rejection `unregistered`). It must also be admissible in `data/card_admissibility.json` (rejection `not_admissible`, logged with the card's reason):
     - lowering-covered: every construct its YAML references (verbs, predicates, timings, keywords, as `tools.lowering_coverage` attributes them from the impact index) is tested behaviourally by the mechanics tests; a `parsed_only` construct does not count (`untested_lowering`, with the constructs listed in the row's `untested_constructs`). A non-DSL card (raw_rust / hand-written, or unimplemented) has no constructs and is `untested_lowering` unless it has its own test module. The attribution is a proxy -- see [Attribution caveat](#attribution-caveat);
     - hybrid rule: a card with >= 4 clauses or any protection construct (the `protection_constructs` family in `qa/impact/lowering_coverage.json`) also needs its own behavioural test module, `code/digimon-engine/tests/cards_behavioral/<set>/<card_id>.rs` declared from `main.rs` / `<set>/mod.rs` with a plain `mod x;`, or an existing ledger `test_path` (`untested_card`; rows carry `needs_card_test` / `has_card_test`). A `#[cfg(..)]`/`#[path]` module declaration or a dedicated top-level `tests/<id>_*.rs` file is not seen (fail-closed: the card reads `untested_card`, it is never wrongly admitted);
     - no failed ledger verdict, i.e. `PARTIAL`, `BLOCKED`, `AUDITED-DRIFT`, `AUDITED-MISSING-TESTS`, `FAILED`, `FAIL` or `QA-FAIL` in either ledger (`failed_verdict`);
     - no `diverged` exam row without a cited `dcgo_quirk`/`stale` triage (`diverged_no_quirk`).

     An unmeasured clause blocks nothing. Regenerate with `PYTHONPATH=code python -m digimon_gym.agents.card_admissibility`.
- Both layers apply to every training library path (including the Architect *training* entrypoints -- `architect_training` / `architect_cotraining` base decks, and, through `MetaOptimizer(training_gate=True)`, their opponents and mutation pool; `code/digimon_gym/agents/architect_admission.py` loads the artifacts once per process. The gate is a TRAINING gate: the shared `MetaOptimizer` / `CandidatePool` / `_pick_best_decklist` default to ungated, so the hosted deck tools (`/deck-optimizer/evaluate`, `/deck-optimizer/optimize`), `store_night` and everything under `code/server/` never read it), to pool snapshots (`GeneralistDeckPool.from_snapshot`, `--curriculum-pool`, the eval CLIs) and to a job's `agent_deck: {source: deck_id}` library deck. There is no off switch. `--legacy-clause-gate` (pilot_training) only *adds* the old per-card requirement, `ready` in `data/oracle_readiness.json` (see [Legacy: oracle readiness is a report](#legacy-oracle-readiness-is-a-report-formerly-the-gate-2026-10-04)), for transition comparison runs. It never admits a deck the two layers reject.
- When no decklist survives, `EmptyTrainingPoolError` lists the cards blocking the most decklists, each with its reason, and gives both regenerate commands. An archetype is eligible when at least one of its lists is admitted.
- `--archetypes` / `allowed_archetypes` narrows the pool further. Each scoped name that ends up with no admitted list is logged at INFO with how many lists each check rejected (`unregistered`, `not_admissible`, and `not_oracle_ready` under the legacy flag).
- `code/tools/meta_coverage.py`'s **Trainable today** figure is only an approximation: it still uses the pre-2026-10-21 ledger rule (registry plus the old four-status ledger list), not the two layers above. The admission truth is `data/card_admissibility.json` (`PYTHONPATH=code python -m digimon_gym.agents.card_admissibility`). Aligning `meta_coverage.py` and `card_loop/candidates.py` with it is a follow-up.

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
the pilot to the training-ready decklists of many archetypes. Unlike gauntlet
training, which varies only the opponent deck, generalist mode samples both
`deck1` and `deck2` on each episode reset.

Sampling is intentionally broad:

1. Choose an archetype uniformly among those with at least one training-ready deck.
2. Choose a training-ready deck uniformly from that archetype.
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

### 6.5.1 In-pod parallel vs one-specialist-per-pod (league scaling)

`train_specialist_league.py --topology parallel` trains several specialists
concurrently in **one** box. A 2026-06-30 investigation made it *reliable* (fixed
GPU-load collisions, a thread storm, a spawn-collision race via a ready-gate, and
an OOM from an unbounded opponent cache) **but never proved it faster**, because:

- **RunPod containers are CPU-cgroup-throttled and `nproc` lies.** A pod showing
  `nproc=96` had `cpu.max = 765000 100000` ≈ **7.65 effective cores**. Every
  "fps with 2 specialists vs 1" number was dividing ~8 real cores, not 96 — the
  comparison was confounded. **Always check `cat /sys/fs/cgroup/cpu.max`
  (quota ÷ period) — and `runpodctl vcpuCount`, which reflects the real quota —
  before trusting any throughput claim.**
- The per-worker thread caps and bounded opponent cache that keep the parallel
  case from thread-storming / OOMing **throttle a lone specialist** (~5× on
  collection; pinning OMP in the learner throttles its update ~5×). They are
  gated on `DIGIMON_LEAGUE_CONCURRENCY>1`, so a normal one-per-box run is
  untouched — but it means parallel pays a tax solo runs don't.

**Recommendation: one specialist per pod, fanned out across pods** — each gets a
full uncapped box. See `docs/CLOUD_TRAINING.md` §A.11 and the OpenSpec change
`add-cross-pod-league-orchestration` (Tier 1 barrier-free fan-out against a fixed
champion pool; Tier 2 distributed league with a shared store + per-round barrier).

---

## 7. Wrapper Chain Reference

### Standard Training Chain

```
DigimonEnv                        (selected observation layout, exported action space, reward shaping)
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

### Models trained before 2026-10-03 never saw EX12 card identities

From the EX12 ingest (2026-07-05) until 2026-10-03, the 77 EX12 cards had no `index` in
`data/cards.json`, so `CardRegistry` encoded every one of them as `0`, the empty-slot id
(see `docs/TENSOR_SPEC.md` "Card Identity Encoding"). A model trained in that window
could not tell EX12 cards apart, or from an empty slot, and its embedding rows for the
indices EX12 now has (4402–4478) were never updated. Run against today's data, such a
model reads those cards through untrained rows instead of padding. Treat it as unproven
on any deck with EX12 cards (Toho Braves, for one): fine-tune or retrain, and judge it on
anchored evaluation (rule 30), not on its old numbers. `code/tools/train_card_autoencoder.py`
could not build `card_embeddings.npy` at all in that window (`KeyError: 'index'`); a table
built earlier has no rows for those indices.

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

## Training gate: mechanics + card admissibility (2026-10-21)

Training admits a decklist only through two layers (spec
`docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md`
§5b; plan `docs/superpowers/plans/2026-10-15-rules-conformance-2-mechanics-gate.md`).
The gate is always on; there is no flag to disable it. The admission rules are
in §2 ("Runtime gauntlet loading ..."); this section is the operator view.

| Layer | Artifact | Regenerate | Fails with |
|---|---|---|---|
| Mechanics (global) | `data/mechanics_readiness.json` | `PYTHONPATH=code python -m tools.rules.mechanics_readiness` | `MechanicsNotReadyError` / `MechanicsReadinessMissingError` |
| Per-card (per decklist) | `data/card_admissibility.json` | `PYTHONPATH=code python -m digimon_gym.agents.card_admissibility` (no engine wheel; reads `data/card_registry.json`, runs `tools.lowering_coverage.build()` in-process, scans the `cards_behavioral/` test modules, both ledgers and the exam verdicts) | `NotAdmissibleDeckError` / `CardAdmissibilityMissingError` |

Both take `--check` (run by `scripts/verify` tier 0, which CI's tier-1 job
includes, beside `lowering_coverage_check`). Regenerate the mechanics artifact
after any change to the rules ledger (`qa/rules/`), and the admissibility
artifact (plus `PYTHONPATH=code python -m tools.lowering_coverage`) after any
change to the validated-cards ledgers, exam verdicts, `data/card_registry.json`,
card YAML, the mechanics tests, or the behavioural test modules (a new or
removed `cards_behavioral/<set>/<id>.rs`).

**The work queue.** `qa/impact/lowering_coverage.json` is the human-readable
report behind `untested_lowering`: per construct, whether a mechanics test
exercises it; per card, its `untested_constructs`. A construct test in the
mechanics suites (`tests/dsl`, `tests/rules_conformance`, ... -- never
`cards_behavioral`) unblocks every card that uses it. `PYTHONPATH=code python -m
tools.lowering_coverage --report OUT.md --decks deckpool.json` ranks the untested
constructs by decks blocked.

### DigiLab-11087 top-10 construct campaign (2026-10-22)

For the frozen `qa/impact/decks/digilab-11087-top10.json` pool (10 lists,
540 card entries, 167 unique IDs), the per-card artifact now admits **10/10
lists and 167/167 unique IDs**. The original campaign reached 9/10; the
[App Fuse OPT followup](superpowers/research/2026-10-22-construct-tests/app-fuse-opt-followup.md)
closed BT25-089's verdict hold and admitted Rebootmon. All 51 keys in the
original frozen untested union, plus four previously parsed-only keys
(`memory_lte`, `name_not_shared_by_field_tamer`, `event_target_is_source`,
`on_ally_played`), have behavioral construct tests. The scanner's deck report
also says **10/10 lowering-covered**; it includes parsed-only credits and
is not the admission gate. Regenerate both artifacts with
`PYTHONPATH=code python -m tools.lowering_coverage` and
`PYTHONPATH=code python -m digimon_gym.agents.card_admissibility`, then compare
each deck's IDs against `cards[id].admissible`. Gate semantics and global
mechanics triage are unchanged. This result does not certify DCGO parity or
training fidelity: separate App Fuse procedure details, effect-link timing,
and other campaign findings remain in `docs/RUST_ENGINE_GAPS.md` and
`qa/dsl-vocab-gaps.md`.

### What `ready` means, and what it does not

`ready` = **no mechanics obligation is flagged outcome-changing**.
`blocks_training` is a hand-set field in `qa/rules/cr_obligations_status.yaml`
(default false); its basis is the Plan 4 triage. Nothing sets it automatically,
and the gap trackers' "Blocks training" lines are not read. Every other open
mechanics gap is counted in the artifact as `unflagged_open_gaps` (316 on
2026-10-21; printed by the regenerate command) and listed in
`qa/rules/cr_obligations.json`. Those gaps were not each judged
outcome-neutral. Flagging policy (`qa/rules/mechanics-gaps-2026-10.md`): a new
mechanics gap that can change a game outcome (ordering, legality, deletion,
memory, security) is filed with `blocks_training: true` until fixed.

The spec's second layer-1 condition -- the `rules_conformance` suite green at the
committed manual version -- is **not** in the artifact. It is enforced by the PR
CI gate `.github/workflows/engine-rules-conformance.yml`.

### Attribution caveat

Lowering coverage is token presence, not proof of assertion. The exact rule is
the `code/tools/lowering_coverage.py` module docstring. A construct is credited
to a `#[test]` when it appears in the test body or in a same-file helper the
body calls. The credit is behavioural only when that body or helper runs the
engine (`ENGINE_RUN_MARKERS`, decided per test); a test that only inspects
compiled IR, a catalog or a lowering table credits `parsed_only`. Cross-file
helpers and `include_str!` fixtures are not seen, so a construct can read
untested when it is tested (fail-closed). A test can also name a construct and
assert nothing about it, so a construct can read tested when nothing checks it
(over-credit). On 2026-10-21, 68 of the 354 behaviourally-credited constructs
were credited only through a YAML string literal, and 150 of the 537 admissible
cards depend on one.

**Pre-flight for a training or cloud run.** Both artifacts must be present on the
host: they are the only gate inputs, because the training image and cloud pods
ship no `qa/`. The image does not bake them in: `data/` is a runtime mount
(`-v data:/app/data`, `DIGIMON_DATA_DIR`), so that mount must carry them; for a
bare pod, `scp` them with the rest of `data/` (docs/CLOUD_TRAINING.md). Override locations with
`DIGIMON_MECHANICS_READINESS` / `DIGIMON_CARD_ADMISSIBILITY` (or `DIGIMON_DATA_DIR`).

If training stops with `EmptyTrainingPoolError`, the message lists the cards
blocking the most decklists, each with its reason (`unregistered`,
`untested_lowering`, `untested_card`, `failed_verdict`, `diverged_no_quirk`), and
both regenerate commands. Fix the cause (test the construct, add the card's own
test, resolve the ledger verdict, triage the divergence with a cited
`dcgo_quirk`) and regenerate; do not edit the artifacts by hand.

Pool snapshots (`--curriculum-pool`, eval CLIs) are refused if they contain an
inadmissible card (`NotAdmissibleDeckError`; `NotOracleReadyDeckError` and
`validate_oracle_ready_deck` are deprecated aliases).

### Legacy: oracle readiness is a report (formerly the gate, 2026-10-04)

From 2026-10-04 to 2026-10-21 admission required every card to be oracle-ready
(every printed clause and gating interaction adjudicated against DCGO). That is
no longer a gate: `data/oracle_readiness.json` is a progress **report** of the
opt-in DCGO exam deepening, regenerated with

    PYTHONPATH=code python -m tools.clause_coverage.readiness

and still checked by `scripts/verify` tier 0 (`oracle_readiness_check`). Its
`--plan --limit 25` still ranks which exams to run next. `pilot_training
--legacy-clause-gate` re-adds the old requirement on top of the two layers (it
never admits a deck they reject) for transition comparison runs; only that path
reads the report and can raise `OracleReadinessMissingError`. Spec:
`docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md` (§4.7 is
superseded as a gate).

## Independent-world search evaluation batching

Physical card menus are searched as complete commitments across all pages.
Serial MCTS, batched PIMC and IS-MCTS evaluate each permitted page and expand
routes ending in a card commit or a legal decline. Navigation does not create
game-tree edges or consume additional simulations. The native self-play actor
selects a complete route once, validates each transport action against the live
mask, and records conditional visit targets on every non-forced page along it.
Search consumers must execute `SearchResult::best_plan()` (or sample
`root_plans` by visits) rather than re-searching after page navigation.
`root_visits` and `root_q` are projections for the current page, not complete
card-choice rankings. The public action IDs, tensor layout and model contracts
are unchanged; all physical candidates remain available.

The generation-3 Proximamon mirror failure (game 398, seed
5215200225685576850, 2026-10-27) demonstrated why this matters: fresh searches
alternated pages for 991 decisions. Raising the decision cap cannot repair that
cycle. Preserve the source shards and checkpoint when replacing an affected run.

Native search self-play and `run_selfplay_generation.py` accept
`--eval-batch-size` (default 4). This batches the next required evaluation of
independent PIMC world trees; each tree finishes its own evaluation and backup
before selecting another simulation. It uses no virtual loss or speculative
same-tree leaves. `--eval-batch-size 1` is the serial reference. IS-MCTS remains
serial. The search simulation budget and world count are separate knobs:
`--sims 320 --worlds 4` remains 320 total simulations, not 320 per world.

Native manifests record requested and effective evaluation batch sizes. Game
workers remain separate processes (`--procs`); within each worker the independent
trees cooperate on one thread and share one batched ONNX evaluator. Batch size
is bounded by available worlds, so increasing it above the world count creates
no additional simulations or worlds. The evaluator must be deterministic and
batch-independent. Compare complete search results and recorded targets against
batch size one when changing model/runtime/provider, not merely model argmaxes.

The optional exact-input output cache remains controlled independently by
`DIGIMON_SEARCH_EVAL_CACHE_ENTRIES` (default zero); use matching cache settings
when measuring batching. The Windows native CLI uses mimalloc. Compare generated
decision positions/sec as well as games/hour on the same hardware, model, pool
and search budget. Inference-only speedups are not full-game speedups.

Use fresh output directories for validation and retries. Do not relabel partial
shards or a user-stopped generation as complete, and do not reuse truncated
diagnostic prefixes as terminal-outcome training data.

### Dynamic native search scheduling and recovery

`run_selfplay_generation.py` defaults to `--scheduling dynamic --chunk-games 16`.
The controller assigns the next chunk to whichever process becomes available.
All workers use one master seed and global game indices (`selfplay --game-start`),
preserving the serial deck RNG stream and per-game/search/action seeds. Search
simulations, sampled worlds, depth caps, noise and temperature are unchanged.
Changing concurrency cannot change targets. Legacy `--scheduling static` retains
the historical per-worker seed scheme and does not support partial resume.

Completed chunks appear under `genN/shards/proc<global-first-game>/`, sorted by
game ID, with hashed `chunk_commit.json` records. Workers write private
`genN/attempts/` directories; incomplete attempts never enter the training window.
`schedule.json` freezes inputs/settings. `scheduler_progress.json` reports
committed games/rows, active PIDs and heartbeat time; `scheduler_events.jsonl`
records dispatches, completions and retries. Keep failed attempts/logs for diagnosis.
Reported native game completions inside an unfinished chunk are not committed
training rows. The maximum recomputation is the active chunks, not the generation.

Use explicit `--resume-selfplay` with the same run directory and frozen inputs.
Resume validates native executable, model/metadata, deck/card/admission data,
search/environment settings and every committed array hash. It permits a changed
`--procs`, but rejects changed chunk partition, incompatible inputs, corrupt data,
legacy untracked outputs, or generations already containing training artifacts.
There is no automatic resume of a user-stopped run. This resumes the self-play
stage of an explicitly invoked generation command; once complete, that command
continues to its configured training/evaluation stages. To perform diagnostics
without any training, use the benchmark tool below instead.

Defaults are one retry and a 3600-second **whole-chunk** deadline, including
startup (`--worker-retries`, `--worker-timeout-seconds`). Set the deadline above
the measured slow-chunk tail on the target machine. A deterministic crash still
fails after bounded retries; it is never dropped from the requested game set.
Ctrl-C/SIGTERM stops dispatch and terminates/drains owned child processes. Kernel
file locks prevent simultaneous controllers and release on process death.

### Development measurements versus unattended Linux runs

Use bounded diagnostics in new directories, with `PYTHONPATH=code`:

```bash
python -m tools.benchmark_search_workers profile --cli /path/to/digimon-engine-cli \
  --model /path/to/seed.onnx --decks /path/to/decks.json --cards-json data/cards.json \
  --out runs/profile-new --games 2 --decision-cap 40
python -m tools.benchmark_search_workers scale --cli /path/to/digimon-engine-cli \
  --model /path/to/seed.onnx --decks /path/to/decks.json --cards-json data/cards.json \
  --out runs/scaling-new --workers 1 2 4 --games 8 --chunk-games 1 --decision-cap 40
```

Profiling toggles `DIGIMON_SEARCH_PROFILE=1` and checks all target arrays against
an unprofiled reference. Native manifest spans report inclusive and exclusive
wall time; sum **self_seconds**, not overlapping inclusive totals. Timings cover
registered search regions, not all allocations or effects individually. Use
`perf`/flamegraphs on Linux for deeper hotspots once coarse spans identify a region.
The scaling benchmark uses the identical global game set and verifies all saved
arrays across worker counts. It reports wall-clock generated positions/sec,
budgeted simulations/sec, truncations, native chunk durations and native-work
utilization (not CPU utilization). Deliberately capped rows are not outcome data.
Both tools leave training stopped. Profile overhead and process startup are
visible; short prefixes are not full-game production ETAs.

For cloud Linux, build the native release executable for Linux in the frozen
training image; do not copy the Windows executable or infer Linux allocator
performance from Windows. Re-run profiling and scaling on the actual instance,
respecting its CPU quota/affinity and memory limit. Increase the game set and
repeat count, then measure representative full games and chunk sizes (for
example 4/8/16) before selecting concurrency/deadlines. More workers can increase
contention. One native inference thread per worker remains the default.

Store schedules, attempts, shards and checkpoints on the same persistent local
POSIX volume. Publication assumes same-filesystem atomic rename and working OS
locks; object-store mounts and unverified network filesystems are unsuitable.
Hashes detect corruption, but this is process-interruption recovery, not a
power-loss durability guarantee for native array writes. Keep off-host backups.
Use a process-group-aware supervisor (for systemd, `KillMode=control-group` and
a bounded stop timeout) so a hard-killed controller cannot leave orphan workers.
Make any supervisor restart policy explicit; never enable one for a run stopped
by the user. Alert on stale controller heartbeat, exhausted retries, disk space,
memory pressure, or absent committed progress over the calibrated slow-chunk
interval. A fresh heartbeat alone does not prove a worker is making progress.

### Frozen-model search-budget evaluation

`search_quality` is a diagnostic native example, separate from the self-play
training driver. Build with `cargo build --release -p digimon-engine --example
search_quality`. With `PYTHONPATH=code`, invoke:

```bash
python -m tools.search_quality_study \
  --engine target/release/examples/search_quality \
  --model models/<frozen-run>/seed.onnx \
  --snapshot models/<frozen-run>/heldout_snapshot.json \
  --cards-json data/cards.json --out runs/<new-budget-study> --workers 4
```

On Windows use the actual isolated Cargo target and `.exe` suffix. The tool
freezes model, executable, card/admission and held-out-pool hashes. It creates
replayable decision roots, compares a simulation/world grid, runs six abstract
tactical controls, selects two alternatives using declared proxy criteria, and
runs seat/deck-balanced full games against the baseline plus fixed greedy and
policy-only anchors. Root noise is zero for competitive evaluation. Training
exploration settings and model weights are untouched; no trainer is imported.

Outputs include `plan.json`, `corpus.json`, per-job requests/results and hashes,
`study_progress.json`, `sweep_summary.json`, and `evaluation_summary.json`.
`--stage sweep`/`evaluate` supports staged execution; `--resume` is explicit and
validates completed output hashes. A failed native job stops the study and
preserves logs/results. Unknown uncommitted result files are not overwritten.
The existing model run's user-stopped pipeline state, when present, is hashed
and checked before dispatch. Do not point output at the training run directory.

Read the [preregistered protocol](superpowers/research/2026-10-23-search-budget-study.md)
before interpreting results. Higher-budget agreement is not tactical ground
truth, the small paired panel is exploratory, and synthetic controls do not
certify real card interactions. A fixed-model budget recommendation does not
establish the best budget for learning at equal total training compute, nor does
it enable closed-decklist beliefs or model promotion.

For the expanded 640/4-versus-320/4 panel with main's per-card statistics, use
`python -m tools.search_budget_confirmation --engine <new-native-example>
--parent-study runs/<completed-budget-study> --out runs/<fresh-confirmation>
--workers 4`. Build into a separate Cargo target to preserve the earlier frozen
executable. The confirmation uses fresh seeds and all ordered pairings of the
same eight lists, with fixed sample size and reversed-budget seats. It reuses
`CardEvalCollector` offline on actual-game events and masks, never search-rollout
events. This path uses the native executable directly and requires no training
wheel rebuild. No training job is resumed or model fit.

The output includes `summary.json`, `report.md`, `cards.csv`, `routes.csv`, and
dashboard-compatible reports under `eval_cards/s640-w4` and `eval_cards/s320-w4`.
In each report player1 is the named budget, regardless of native seat; the two
reports describe the same games from opposite roles and must not be added as
independent samples. Use the existing card dashboard's Local reports source.
`--resume` validates frozen identities and committed jobs; `--analyze-only
--resume` regenerates reports from verified completed results. The earlier build's
score is contextual, never pooled into the new panel. See the
[confirmation protocol](superpowers/research/2026-10-23-search-budget-confirmation.md).

### Quarantining verified bad self-play games

Keep committed shard arrays and `chunk_commit.json` intact. To exclude a verified
bad game from both policy and value fitting, place `training_exclusions.json`
beside that chunk's `manifest.json`:

```json
{
  "schema": "selfplay-training-exclusions-v1",
  "manifest_sha256": "<SHA-256 of manifest.json bytes>",
  "games": [{"game_id": 875, "reason": "Verified repeated-state loop"}]
}
```

IDs refer to that chunk's original `game_id.npy`, before loader rebasing. The
loader verifies the manifest hash, nonempty reasons, unique IDs, and that every
excluded ID exists; invalid exclusions stop fitting. Other games in the same
chunk remain eligible. The trainer reports `excluded_rows`, `excluded_games`,
and `training_exclusions` in its summary. These are explicit exclusions, separate
from the default handling of truncations: unquarantined truncated rows still
contribute policy loss, while their value loss is masked.

The generation driver imports the trainer after collection. Any operational
Python overlay applied to an already-running container must record its original
and replacement hashes and be reinstalled on an explicit resume into a fresh
container. A local source edit alone does not change a running cloud job.


## Matched development-teacher seed comparison

`tools.cloud_search.run_seed_comparison` is an opt-in, isolated fresh-seed path.
It collects greedy-only, `development_v1`-only and 50/50 player-game mixtures
using the same frozen 13-archetype snapshots, seeds and list schedule. Each arm
has 416 full games, 64 appearances per archetype, with independent opening-seat
and teacher balance. Two-list archetypes expose both teachers to both lists.
`development_v1` scores legal native candidates; unknown conditional benefits
and optional cost reducers remain explicitly unknown, not guaranteed gains.
Its versioned weights and limitations are recorded in
`openspec/changes/add-mixed-development-bootstrap/`.

Each player-game contributes 64 sampled rows with replacement; game IDs remain
intact for the terminal-value held-out split. All three corpora must finish
without caps or crashes before any fitting begins. Four imitation epochs and
eight critic epochs use matching initialization and update budgets. These
one-hot demonstrations never enter the later search replay pool.

Each seed is evaluated for 5,200 policy-only games against the same greedy and
frozen greedy-bootstrap anchors, then gets one 1,100-game search update at
320 simulations / four worlds / batch four. A second 5,200-game evaluation
uses the same decks and seeds. Meta held-out and pet same-list panels remain
separate. Paired opening-seat bootstrap intervals use a predeclared five-point
aggregate noninferiority gate; there is no automatic promotion.

The controller requires `DIGIMON_DEPLOYED_IMAGE` to be an immutable digest.
Run `python -m tools.cloud_search.run_seed_comparison --root /experiment` only
in a fresh directory with `inputs/{decks,train_snapshot,heldout_snapshot,
pet_evaluation_snapshot}.json`. `deploy_seed_comparison.py` provisions separate
48-hour, no-restart systemd services on the existing CPU host after the image
passes the recorded game-268 search regression. It does not modify or resume
the failed baseline. W&B has one run per arm, including bootstrap counts,
checkpoint artifacts and per-card evaluation reports.

An explicitly approved retry before search starts can use
`--validate-resume`, followed by `--resume-before-search`, on the same root.
The gate requires a failed comparison, unchanged frozen inputs/settings and
runtime contract, qualified bootstrap corpora, byte-identical saved seed copies,
the shared anchor, and no generation beyond `gen0`. Completed evaluation windows
are reused only after model/cohort provenance and complete paired-game checks.
Failed windows move into a timestamped `resume-history` directory before being
rerun in full; old reports are never mixed into the replacement window. No
bootstrap recollection or fitting occurs. The resume record preserves old/new
image digests, controller hash, seed hashes and archived deployment metadata.
This is not automatic recovery or support for resuming interrupted search.

For a controller-only operational overlay, mount the committed controller file
read-only into the tested immutable image and retain its source commit and
SHA-256 in the host resume record. Keep `Restart=no`, preserve the original
service definition, and verify the resumed stage and existing W&B run IDs.

Preserve any `failure.json`, partial shards and recording. Investigate and use
a new directory/image for a changed teacher; do not skip a failed game, resume
into an altered corpus, or fit from partial outputs. DNA denominators are exact
lists containing printed DNA targets, not archetype names. Per-action net hand
changes are available; gross draw/search/retrieval counts remain unmeasured
where native events do not distinguish them.

For a live, read-only local dashboard of progress, checkpoint comparisons and
card behavior, see [Live experiment dashboard](CARD_EVALUATION.md#live-experiment-dashboard).
Its SSH collector caches compact summaries and never starts or stops training.
The dashboard distinguishes stale observations, saved models, completed
evaluation panels and per-game progress (when supported by the deployed image).

### Frozen mixed/development head-to-head

`python -m tools.cloud_search.head_to_head --prepare --source /source --root /experiment`
validates the completed seed comparison, checkpoint-1 model contracts, card-data
hash and native-image identity. It combines the frozen held-out meta and pet
snapshots through the existing admission gate, copies both models, and saves
the exact game schedule. Run the same command without `--prepare` to evaluate;
the root cannot be silently restarted or overwritten.

Default coverage is 6,760 policy-only games: 13 x 13 ordered deck cells x 40
games. Each model is the candidate in one 3,380-game evaluator process, with
opposite first players for each deck pair. Both processes use Torch inference,
unchanged action masks and fresh fixed seeds, with per-card reports and progress
heartbeats. No optimizer or search runs. Meta/pet source identities remain in
the plan; the joint pool adds the previously missing cross-pool matchups.

The result normalizes both engine roles to mixed's perspective and emits a full
matrix plus a 95% bootstrap interval over four-game blocks (both roles/openers).
Missing, duplicate, mismatched or incomplete games fail the comparison rather
than counting as draws. A small per-cell sample is descriptive; the aggregate
interval does not certify every matchup. Use the same immutable native image
plus a recorded, committed read-only Python overlay; retain failure recordings
and keep `Restart=no`. No automatic model promotion follows evaluation.

### Versioned global EX13 pool refresh

The prepared October 26 pool lives in
[`data/training_pools/ex13-20261026`](../data/training_pools/ex13-20261026/README.md).
It retains the same 13 archetypes and uses exact global lists; English-only
legality is not a source filter. The report separates competitive qualification,
lower-evidence published variants, engine admission and original pet fallbacks.
Never substitute cards to make a sourced list pass admission.

`decks.json` is the native flat-sampler input. Repeated `:slotN` entries give
archetypes equal sampling weight when they have fewer distinct training lists;
they are not additional deck variants. `train_snapshot.json` preserves distinct
training lists, `heldout_snapshot.json` contains unseen list fingerprints, and
`pet_regression_snapshot.json` preserves the five original pets. Use
`combined_diagnostic_snapshot.json` for an all-archetype panel, retaining its
explicit same-list fallback labels rather than calling the entire panel held out.

Rebuild into a fresh directory using the prepared source evidence and the actual
baseline snapshot being excluded from held-out selection:

```bash
python -m tools.cloud_search.prepare_ex13_pool prepare \
  --sources data/training_pools/ex13-20261026/source_candidates.json \
  --baseline data/training_pools/ex13-20261026/source_baseline_train_snapshot.json \
  --cli-bin /path/to/digimon-engine-cli --out /new-pool
```

The native registry must agree with the Python binding registry. The preparer
checks current admission and standard deck legality, records source/gate/binary
hashes, rejects an existing output, and does not deploy a run. Record a fresh
engine image and pool identity before using the new assets; keep affected
diagnostic replay and evaluation results separate from repaired-engine results.

Balanced evaluation now records `cohort_scheme=archetype-paired-v2`. Within each
ordered archetype matchup it rotates through the Cartesian product of list
variants while preserving paired first-player balance. With two variants per
side, four pairs per cell are needed to visit all four list combinations once.
Smaller quotas do not certify full variant coverage. Preserve the saved schedule
and its hash when comparing checkpoints; do not compare a new multi-list cohort
to an old lockstep cohort as if the deck schedule were unchanged.

## EX13 training lessons and raising diagnostic (2026-10-28)

These are dated experiment findings, not proof of model proficiency or agreement
with tournament meta rankings. The portable [raising experiment evidence](training-evidence/2026-10-28-raising-diagnostic.json)
records model/input hashes, counts, estimates and the selected counterexample.

### Collection, fitting, evaluation and promotion are separate

The EX13 campaign uses **13 archetypes, 21 distinct training lists and 26
equally weighted native sampling slots**. Repeated slots balance archetype
weight; they do not create new lists. Each 1,100-game generation supplies about
169 expected player-side appearances per archetype. Three generations provide
up to 3,300 replay games, or about 508 expected appearances per archetype per
update. These are expectations, not quotas; longer games supply more decision
rows. Scale generation size with pool size to preserve the rough floor of 150
appearances per archetype. More training alone does not establish strength.

The recorded operating point is **320 total simulations, four worlds, leaf
batch four and eight workers**. Four worlds divide the simulation budget;
batching and workers control throughput. This is not a demonstrated optimal
search budget. The fit uses two epochs, batch 512, learning rate 0.0001 and a
three-generation replay window. Do not change rewards, search quality or
sampling to conceal a rules failure or improve a throughput headline.

This search/self-play path fits policy visit targets and a terminal-outcome
value head (+1/-1 from the mover's perspective; 0 for a completed draw). It is
not a PPO update and does not use the dense PPO gameplay reward profile above.
Truncation is marked separately and excluded from terminal-value fitting;
verified bad games are quarantined before either training loss. Changing a PPO
quick-win or digivolution reward will not change this native search objective.
The recorded models use `standard_lite_v2`, feature schema `standard_lite_v2.4`,
9,434 floats and action schema 2 with 2,192 actions. Validate exported layout
metadata rather than assuming these dated dimensions apply to future images.

Evaluate stable new checkpoints with the frozen policy-only panel. The 10,800
games comprise **7,800 combined diagnostic games** (13 archetypes x three
anchors x 200 games) and **3,000 original-pet regression games** (five
archetypes x three anchors x 200). The combined panel has 24 distinct lists,
including a same-list Light Hybrid fallback; it is not entirely held out. The
five-list pet panel stays separate. Anchors are greedy, `parent-checkpoint4`
and `development-fixed`.

Checkpoint numbers are local to their campaign. The anchor named
`parent-checkpoint4` is an earlier campaign's model, not the page-recovery
campaign's checkpoint 4. Identify models by campaign and artifact hash. Compare
complete, seat-paired cohorts with matching seeds, lists, inference mode,
engine and observation/action contracts. A high greedy win rate can coexist
with weakness against learned anchors. A simulator matchup matrix measures
model-plus-deck performance under that protocol, not tournament meta strength.
A saved checkpoint, completed generation or lower fit loss is not promotion
evidence.

### Continuation lineage and the latest interruption

The page-search recovery warm-started from checkpoint 2 and regenerated
generations 3–4. After completion, the continuation copied checkpoint 4 plus
generations 3–4 and requested bounded updates 5–8. Those **2,200 inherited
replay games are not new collection progress**. `extend_completed_campaign`
validates the completed source, checkpoint, runtime, full evaluation and chunk
hashes before copying replay; preparation and launch are separate commands.
`run_small_generations` allows one to four updates at continuing logical
indices and rejects existing output. No automatic restart or promotion follows.

As of October 28, checkpoint 5 has a complete evaluation. Checkpoint 6 was
fitted, but its combined evaluation failed at seed **44524569**, SW versus Toho
Braves. Its completed pet panel does not make the whole evaluation complete.
Updates 7–8 did not run. The failed source and weights remain preserved;
training is stopped. The token repair and raising diagnostic below were
validated locally and have not been deployed to this cloud run.

The failure exposed a token returned into hand and subsequently offered by the
play mask. **CR 4.3 4-21-5, 15-1-9 and 15-1-10** distinguish a processed return
or placement from an actual addition: a departing token is removed from the
game, still counts for applicable return/placement conditions, and does not
trigger an actual hand/deck addition. The repair covers zone-entry guards,
return receipts, grouped moves and resumable security placement.
`token_departure_rules` verifies the 88-step recording prefix, resumes action
62, and reaches game over after 263 continuation decisions with matching clone
digests. Eleven token regressions and 94 adjacent tests passed. This is
simulator evidence, not new DCGO oracle certification.

Earlier failures established two further invariants: deletion replacements
must verify that their payment succeeded, and paged search must commit a
complete physical-card route. Preserve exact executed decisions from
`DIGIMON_SELFPLAY_TRACE_DECISIONS`; projected visit argmax is not an action log.
Action IDs are phase-dependent, so include pending-selection/phase context
when diagnosing a loop.

Before resuming on a repaired image, qualify the checkpoint on that exact image,
preserve the failed root and explicitly validate reused replay/evaluation.
Do not relabel old-engine data as repaired-engine evidence. A fresh logger
heartbeat is not game progress: compare committed games/rows, controller stage
and evaluation progress, allowing for fitting and worker deadlines. Local
committed artifacts remain the evidence source when W&B is stale or out of
storage. A healthy logger does not establish healthy training.

### Matched raising experiment

This tests **promotion out of raising**, not digivolving in raising or DNA
digivolution. Keep these routes separate and measure opportunities as well as
choices. For Virus Busters, use its egg-mediated DNA sequence telemetry;
a low generic DNA-action count alone is not decisive.

We generated 52 source games (four per archetype, balanced between checkpoints
2/5 and seats, against checkpoint 4), then reservoir-sampled one legal
promote/wait position per completed game. Eight games offered no such position,
leaving **44 unique roots across all 13 archetypes and 23 of the 24 panel
lists**. Rootless games were not replaced. Each root was replayed exactly and
tested with both policies and noise-free search at 320 simulations/four
worlds/batch four on the same local token-corrected engine, with frozen model,
executable and input hashes.

| Model | Policy promotes | Search promotes |
|---|---:|---:|
| Checkpoint 2 | 44/44 (100%) | 35/44 (79.5%) |
| Checkpoint 5 | 31/44 (70.5%) | 34/44 (77.3%) |

For each root, force promotion or waiting, then finish with each actor policy
(checkpoints 2/5) against each opponent policy (checkpoints 2/4/6). All **528
continuations, or 264 action pairs**, completed without failure or truncation.
They are midgame counterfactuals, not 528 independent full-game strength tests.
The same source-game root is the comparison/resampling unit across opponents.

- **Policy/search disagreement:** checkpoint 5 waits where search promotes in
  eight roots. A Chronomon root changes losses into wins against checkpoints
  2 and 4 when promotion is forced. The other seven disagreements do not change
  terminal outcomes under these policy continuations.
- **A search/value counterexample:** in SW root `06-0-2` (seed `28267029`),
  checkpoint 5 and root search wait. Search repeats that choice under five
  seeds. Against checkpoint 2, promoting wins and waiting loses even when the
  actor uses full search for subsequent decisions (38 and 31 searched decisions
  respectively). Inspect the value estimates and sampled worlds; one actual
  hidden realization does not prove an engine bug or an expected-value failure
  across all hidden states.
- **Waiting does not explain the whole decline:** among checkpoint 5's 13 wait
  roots across three opponents, promotion improves five outcomes, worsens two
  and leaves 32 unchanged. Always promoting adds zero net wins against
  checkpoint 2. Averaged over three opponents, the estimated gain is 2.3
  percentage points, with a 95% paired bootstrap interval of -2.3 to +7.6.
  Do not hard-code promotion from this evidence.
- **Opponent diversity remains an open hypothesis:** the change in waiting's
  advantage against checkpoint 6 versus checkpoint 2 is 0 points (95% interval
  -11.4 to +11.4). This screen does not establish that waiting works only against
  recent models, nor rule out broader opponent overfitting.

The 4,000-resample bootstrap keeps action pairs together and samples roots
within archetype. Conditional subsets with a single root in an archetype do
not get a falsely precise interval. The cohort is small, per-archetype findings
are exploratory, and search uses exact-composition open-decklist PIMC.
Policy-only continuations alone cannot diagnose search quality; the selected
SW case therefore also received two complete searched continuations. No model
was fitted, promoted or deployed by this diagnostic.

Next priorities are the saved SW value/search trace and the eight policy/search
disagreements. Check target-to-policy learning and continuation play before
changing reward shaping, forcing promotions or attributing the regression to
opponent diversity.

### Reproducing the raising screen

Build the native example from the engine revision being tested. From the repo
root, with `PYTHONPATH=code`, use a new diagnostic directory containing
`inputs/cp2.onnx`, `cp4.onnx`, `cp5.onnx`, `cp6.onnx`, their `.onnx.meta.json`
sidecars and `inputs/combined_diagnostic_snapshot.json`:

```bash
cargo build -p digimon-engine --example raising_diagnostic
python -m tools.cloud_search.raising_diagnostic prepare \
  --root /new-diagnostic --engine /path/to/raising_diagnostic
python -m tools.cloud_search.raising_diagnostic collect --root /new-diagnostic --workers 4
python -m tools.cloud_search.raising_diagnostic probe --root /new-diagnostic --workers 8
python -m tools.cloud_search.raising_diagnostic summarize --root /new-diagnostic
```

The frozen plan records identities, hashes, search budget, sampling and limits.
Results retain exact prefixes, choices, search values/visits and branch traces.
Incomplete branches never count as draws; both actions must complete before a
pair enters outcome summaries. Preserve partial/failed output and investigate
before explicitly retrying.

For the exploratory SW follow-up, copy the plan to a separate directory, retain
all model entries, set actors to `[2]` (checkpoint 5) and opponents to `[0]`
(checkpoint 2), and copy request `probe-06-0-2.json` with its `plan` path updated
and `search_continuations: true`. Run the same native example as
`raising_diagnostic probe REQUEST.json NEW_RESULT.json`. For root-only seed
sensitivity, use another plan with opponents `[]`, omit the continuation flag,
and vary only `search_seed`. These post-screen checks are selected
counterexamples, not additional independent population samples.

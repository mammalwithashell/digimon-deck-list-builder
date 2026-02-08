# CLAUDE.md — AI Assistant Guide

## Project Overview

Digimon Deck List Builder is a **reinforcement learning game engine** for the Digimon Trading Card Game. It simulates games headlessly, trains RL agents (Q-DeckRec) to optimize deck construction, and exposes a FastAPI endpoint for running simulations. The primary implementation is Python; a C# reference implementation exists in `Digimon.Core/` for comparison.

**Development stage:** Pre-alpha. Active development on the Python game engine and RL gym.

## Repository Structure

```
digimon_gym/                     # PRIMARY CODEBASE
├── __init__.py
├── api.py                       # FastAPI backend (session-based game management)
├── digimon_gym.py               # Gymnasium RL environment (DigimonEnv class)
├── simple_sim.py                # Basic simulation runner
├── engine/
│   ├── game.py                  # Game class — turn management, phases, combat
│   ├── core/
│   │   ├── entity_base.py       # CEntity_Base — card metadata
│   │   ├── card_source.py       # CardSource — card instance wrapper
│   │   ├── player.py            # Player state (hand, deck, board zones)
│   │   ├── permanent.py         # Permanent — digimon/tamer on field
│   │   └── card_script.py       # CardScript base class
│   ├── data/
│   │   ├── cards.json           # Card database (222 cards: ST1, BT14, BT23, BT24)
│   │   ├── enums.py             # CardColor, CardKind, GamePhase, PlayerType, etc.
│   │   ├── card_database.py     # Singleton card loader
│   │   ├── card_registry.py     # Card ID ↔ integer mapping
│   │   └── scripts/             # Per-card effect implementations (208 scripts)
│   │       ├── st1/             # Starter Set 1
│   │       ├── bt14/            # Booster Set 14
│   │       ├── bt23/            # Booster Set 23
│   │       └── bt24/            # Booster Set 24
│   ├── runners/
│   │   ├── base_runner.py       # BaseGameRunner — shared deck setup
│   │   ├── headless_game.py     # HeadlessGame — agent-vs-agent (RL training)
│   │   └── interactive_game.py  # InteractiveGame — human/agent (API-driven)
│   ├── loggers.py               # IGameLogger, SilentLogger, VerboseLogger
│   └── interfaces/
│       └── card_effect.py       # ICardEffect interface
├── scraper/
│   └── scrape_decks.py          # Tournament decklist scraper (Egman Events)

tests/                           # Pytest test suite
├── test_runners.py              # HeadlessGame/InteractiveGame tests (30 tests)
├── test_tensor_and_actions.py   # Tensor encoding/action decoding tests (48 tests)
├── test_bt14_scripts.py         # BT14 card script validation
├── test_bt24_scripts.py         # BT24 card script validation
└── test_rl_gym.py               # Legacy RL gym tests (uses old imports)

scripts/
└── train_smoke_test.py          # SB3 MaskablePPO validation script

Digimon.Core/                    # C# reference implementation (read-only)
AGENTS.md                        # RL agent specifications (Q-DeckRec, Pilot agents)
RULES_CONTEXT.md                 # Digimon TCG official rules reference
Q-Rec Agent Notes                # Q-DeckRec MDP formulation and hyperparameters
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Python 3.11+ |
| Game engine | Custom (Gymnasium-compatible) |
| RL framework | Gymnasium, PyTorch, Stable-Baselines3, sb3-contrib |
| API | FastAPI + Uvicorn |
| Testing | pytest |
| Data | JSON card database (222 cards) |

## Common Commands

```bash
# Run all tests (excludes legacy test_rl_gym.py)
python -m pytest tests/ --ignore=tests/test_rl_gym.py -v

# Run runner tests only
python -m pytest tests/test_runners.py -v

# Run tensor/action tests only
python -m pytest tests/test_tensor_and_actions.py -v

# Run smoke test (validates Gymnasium env + SB3 integration)
python scripts/train_smoke_test.py

# Quick env validation
python -c "from digimon_gym.digimon_gym import DigimonEnv; env = DigimonEnv(); obs, info = env.reset(); print(obs.shape, info['action_mask'].shape)"

# Start the API server
cd digimon_gym && uvicorn api:app --reload
```

**Install dependencies:** `pip install -r requirements.txt`

## Architecture & Key Patterns

### Game Runner Architecture

```
BaseGameRunner (ABC)
├── HeadlessGame     — Agent-vs-Agent, SilentLogger, optimized for RL training
└── InteractiveGame  — Human/Agent, VerboseLogger, pause-on-human semantics
```

- `BaseGameRunner` handles deck setup from card ID lists and calls `game.start_game()`
- `HeadlessGame` provides `step()`, `run_until_conclusion()`, `get_action_mask()`, `get_board_tensor()`
- `InteractiveGame` supports `PlayerType.Human` and `PlayerType.Agent`, pauses on human turns

### Logger System

```
IGameLogger (ABC)
├── SilentLogger   — No-op (headless, max performance)
└── VerboseLogger  — Buffers messages for API retrieval
```

### Gymnasium Environment (DigimonEnv)

```python
from digimon_gym.digimon_gym import DigimonEnv

env = DigimonEnv()
obs, info = env.reset()                     # (695,) float32, info has 'action_mask'
obs, reward, terminated, truncated, info = env.step(action)  # Gymnasium v1.0 API
mask = env.action_mask()                    # (2120,) int8 for SB3 MaskablePPO
```

- Subclasses `gymnasium.Env` for full SB3/RLlib compatibility
- Dense reward: security delta x 0.01, board DP diff x 0.0001, terminal +/-1.0
- Action masking via `info['action_mask']` (SB3 `MaskablePPO` convention)
- `GameState` class retained as deprecated backward-compatible wrapper

### Game State Machine

Phases flow: `Start -> Draw -> Breeding -> Main -> End -> (next turn)`

- Draw phase is skipped on the first player's first turn
- Breeding and Main are "parking" phases — game waits for external action calls
- Main phase loops until the player passes or memory crosses to opponent
- Memory gauge ranges from -10 to +10; crossing 0 ends the turn

### RL Action Space (2120 discrete actions)

| Range | Action |
|-------|--------|
| 0-29 | Play card from hand (index) |
| 30-59 | Trash card from hand (index) |
| 60 | Hatch from egg deck |
| 61 | Move from breeding area |
| 62 | Pass turn / breeding pass / decline optional |
| 63-92 | DNA Digivolve (hand index) |
| 100-399 | Attack with permanent (slot x target) |
| 400-999 | Digivolve (hand x field) |
| 1000-1999 | Effect activation (source x effectIdx) |
| 2000-2119 | Source selection (field x sourceIdx) |

**Selection conventions** (used in `SelectTarget`/`SelectMaterial`/`SelectHand`/`SelectReveal`):

| Range | Selection Meaning |
|-------|-------------------|
| 0-29 | Select hand card by index |
| 30-39 | Select from revealed cards |
| 40-49 | Select from own security stack |
| 50-59 | Select from opponent's security stack |
| 100-111 | Select own battle_area permanent |
| 112-123 | Select opponent's battle_area permanent |
| 1000-1009 | Choose between effect branches |

Action masking via `get_action_mask()` / `action_mask()` enforces legal moves.

### Core Classes

- **`Game`** (`engine/game.py`) — Orchestrates turns, phases, combat resolution. 695-element tensor (680 board + 10 revealed + 5 selection context), 2120 action space.
- **`Player`** (`engine/core/player.py`) — Manages board zones: `hand_cards`, `library_cards`, `security_cards`, `trash_cards`, `breeding_area`, `battle_area`, `digitama_library_cards`
- **`CardSource`** (`engine/core/card_source.py`) — Runtime card instance wrapping `CEntity_Base`
- **`Permanent`** (`engine/core/permanent.py`) — A digimon/tamer on the field with digivolution stack
- **`DigimonEnv`** (`digimon_gym.py`) — Gymnasium wrapper exposing `reset()`, `step()`, `action_mask()`

### Effect System

Card abilities are implemented as per-card scripts:

1. Card metadata lives in `cards.json` with a `card_effect_class_name` field
2. `CardDatabase` dynamically loads `digimon_gym/engine/data/scripts/{set}/{card_id}.py`
3. Each script subclasses `CardScript` and returns `ICardEffect` instances
4. Effects define timing, conditions, and modifiers (DP, security, etc.)

### Singleton Pattern

`CardDatabase` is a lazy-loaded singleton — access via `CardDatabase()`.
`CardRegistry` maps card IDs to integers — call `ensure_initialized()` before use.

## Code Conventions

- **Type hints** used throughout; `TYPE_CHECKING` imports for circular dependency avoidance
- **Enums** for all game constants (`GamePhase`, `CardColor`, `CardKind`, `EffectTiming`, `PendingAction`, `PlayerType`)
- **Property decorators** for computed values on `Permanent` (level, DP, etc.)
- **Import duality**: try/except for `python_impl.*` vs `digimon_gym.*` in core files; new code should use `digimon_gym.*`
- **Headless design** — all game logic runs without UI; state serialized to NumPy arrays

## Testing Guidelines

- Use **pytest** for all tests
- Test files go in `tests/` (root level)
- `test_rl_gym.py` uses legacy `python_impl` imports — excluded via `--ignore`
- Mock card helpers exist in test files — reuse them for new tests
- Run `python -m pytest tests/ --ignore=tests/test_rl_gym.py -v` for the full suite

## Key Documentation

- **AGENTS.md** — RL agent specs, MDP formulation, pilot agent types
- **RULES_CONTEXT.md** — Official Digimon TCG rules, phase structure, combat logic, keywords
- **Q-Rec Agent Notes** — Q-DeckRec network architecture, hyperparameters, training loop

## Known Gaps

- ~110 card scripts have stubbed effect callbacks (target selection, reveal-and-select, digivolve, mind link). Game helper methods are in place — scripts need updating to use them.
- No CI/CD pipeline
- No frontend implementation yet (React planned)
- `test_rl_gym.py` uses old `python_impl` imports
- Q-DeckRec agent not yet implemented (architecture specced in AGENTS.md)

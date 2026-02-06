# CLAUDE.md — AI Assistant Guide

## Project Overview

Digimon Deck List Builder is a **reinforcement learning game engine** for the Digimon Trading Card Game. It simulates games headlessly, trains RL agents (Q-DeckRec) to optimize deck construction, and exposes a FastAPI endpoint for running simulations. The primary implementation is Python; a C# reference implementation exists in `Scripts/` and `dump.cs` but is not actively used.

**Development stage:** Pre-alpha. Active development on the Python game engine and RL gym.

## Repository Structure

```
python_impl/                  # PRIMARY CODEBASE
├── api.py                    # FastAPI backend (POST /simulate, GET /)
├── digimon_gym.py            # Gymnasium RL environment (GameState class)
├── simple_sim.py             # Basic simulation runner
├── engine/
│   ├── game.py               # Game class — turn management, phases, combat
│   ├── core/
│   │   ├── entity_base.py    # CEntity_Base — card metadata
│   │   ├── card_source.py    # CardSource — card instance wrapper
│   │   ├── player.py         # Player state (hand, deck, board zones)
│   │   ├── permanent.py      # Permanent — digimon/tamer on field
│   │   └── card_script.py    # CardScript base class
│   ├── data/
│   │   ├── cards.json        # Card database (~30 cards: ST1, BT23)
│   │   ├── enums.py          # CardColor, CardKind, GamePhase, EffectTiming, etc.
│   │   ├── card_database.py  # Singleton card loader
│   │   └── scripts/          # Per-card effect implementations
│   │       ├── st1/          # Starter Set 1 (ST1-01 through ST1-16)
│   │       └── bt23/         # Booster Set 23
│   └── interfaces/
│       └── card_effect.py    # ICardEffect interface
├── scraper/
│   └── scrape_decks.py       # Tournament decklist scraper (Egman Events)
└── tests/
    ├── test_gym.py           # Unittest + FastAPI TestClient tests
    └── test_simulation.py    # Game loop simulation tests

tests/
└── test_rl_gym.py            # Pytest-based RL gym validation

Scripts/                      # C# reference implementation (read-only)
AGENTS.md                     # RL agent specifications (Q-DeckRec, Pilot agents)
RULES_CONTEXT.md              # Digimon TCG official rules reference
Q-Rec Agent Notes             # Q-DeckRec MDP formulation and hyperparameters
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Python 3.11+ |
| Game engine | Custom (Gymnasium-compatible) |
| RL framework | Gymnasium, PyTorch, Stable-Baselines3 |
| API | FastAPI + Uvicorn |
| Testing | pytest (preferred), unittest |
| Data | JSON card database |

## Common Commands

```bash
# Run all tests (requires numpy, pytest, fastapi, httpx)
python -m pytest tests/ python_impl/tests/ -v

# Run only the RL gym tests
python -m pytest tests/test_rl_gym.py -v

# Run only the simulation test
python -m pytest python_impl/tests/test_simulation.py -v

# Start the API server
cd python_impl && uvicorn api:app --reload

# Run a basic simulation
python python_impl/simple_sim.py
```

**Required packages:** `numpy`, `pytest`, `fastapi`, `httpx`, `uvicorn`, `gymnasium`, `torch`

Install with: `pip install numpy pytest fastapi httpx uvicorn gymnasium torch`

Note: There is no `requirements.txt` at the project root yet.

## Architecture & Key Patterns

### Game State Machine

Phases flow: `Start → Draw → Breeding → Main → End → (next turn)`

- Draw phase is skipped on the first player's first turn
- Main phase loops until the player passes or memory crosses to opponent
- Memory gauge ranges from -10 to +10; crossing 0 ends the turn

### Core Classes

- **`Game`** (`engine/game.py`) — Orchestrates turns, phases, combat resolution
- **`Player`** (`engine/core/player.py`) — Manages board zones: `hand_cards`, `library_cards`, `security_cards`, `trash_cards`, `breeding_area`, `battle_area`, `digitama_library_cards`
- **`CardSource`** (`engine/core/card_source.py`) — Runtime card instance wrapping `CEntity_Base`
- **`Permanent`** (`engine/core/permanent.py`) — A digimon/tamer on the field with digivolution stack
- **`GameState`** (`digimon_gym.py`) — Gymnasium wrapper exposing `reset()`, `step(action)`, `get_action_mask()`

### RL Action Space (50 discrete actions)

| Range | Action |
|-------|--------|
| 0–9 | Play card from hand (index) |
| 10–19 | Trash card from hand (index) |
| 20 | Hatch from egg deck |
| 21 | Unsuspend |
| 22 | Pass turn |
| 23–32 | Attack with permanent (index) |

Action masking via `get_action_mask()` enforces legal moves.

### Effect System

Card abilities are implemented as per-card scripts:

1. Card metadata lives in `cards.json` with a `card_effect_class_name` field
2. `CardDatabase` dynamically loads `python_impl/engine/data/scripts/{set}/{card_id}.py`
3. Each script subclasses `CardScript` and returns `ICardEffect` instances
4. Effects define timing, conditions, and modifiers (DP, security, etc.)

### Singleton Pattern

`CardDatabase` is a lazy-loaded singleton — access via `CardDatabase.get_instance()`.

## Code Conventions

- **Type hints** used throughout; `TYPE_CHECKING` imports for circular dependency avoidance
- **Enums** for all game constants (`GamePhase`, `CardColor`, `CardKind`, `EffectTiming`, `PendingAction`)
- **Property decorators** for computed values on `Permanent` (level, DP, etc.)
- **No frontend yet** — React is planned but not implemented
- **Headless design** — all game logic runs without UI; state serialized to NumPy arrays

## Testing Guidelines

- Use **pytest** for new tests (not unittest)
- Tests that need the gym require `numpy` and `gymnasium`
- Test files go in `tests/` (root) for integration tests or `python_impl/tests/` for unit tests
- Mock card helpers exist in `tests/test_rl_gym.py` — reuse them for new gym tests

## Key Documentation

- **AGENTS.md** — RL agent specs, MDP formulation, pilot agent types
- **RULES_CONTEXT.md** — Official Digimon TCG rules, phase structure, combat logic, keywords
- **Q-Rec Agent Notes** — Q-DeckRec network architecture, hyperparameters, training loop

## Known Gaps

- No `requirements.txt` or `pyproject.toml` at root
- No CI/CD pipeline
- Limited card database (~30 cards from ST1 and BT23)
- No frontend implementation yet
- Test coverage is basic — effect system and complex combat logic lack tests

# Digimon TCG Deck List Builder & Optimizer

> **Status:** Pre-Alpha / Active Development
> **Stack:** Python 3.11+ (Gymnasium, PyTorch, Stable-Baselines3, FastAPI)

A high-performance, headless game engine and deck optimization platform for the **Digimon Trading Card Game**. The project treats deck building as a **Markov Decision Process (MDP)**, using Reinforcement Learning to discover optimal lists against a weighted meta gauntlet.

## Architecture

### Game Engine (`digimon_gym/engine/`)

A custom Python implementation of the Digimon TCG rules, built to the **Gymnasium** standard.

- **Headless by design** — runs purely on CPU with no graphical overhead, enabling thousands of simulations per minute
- **981-float board tensor** — game state (security, hand, battle area, breeding, trash) serialized into NumPy arrays for direct neural network consumption
- **2,120 discrete actions** — play, digivolve, attack, hatch, activate effects, select targets, and more (see [ACTION_SPEC.md](ACTION_SPEC.md))
- **Action masking** — strict validity masking prevents illegal moves (summoning sickness, wrong evolution level, insufficient memory, etc.)
- **Game state machine** — Start > Draw > Breeding > Main > End, with memory gauge (-10 to +10) controlling turn flow

### Card Effect System

Card abilities are implemented via a **transpiler pipeline** that converts C# scripts from the [DCGO](https://github.com/DCGO2/DCGO) Unity project into Python:

- **3,651 cards** across 45 sets in the card database (`cards.json`)
- **412 transpiled scripts** across 5 sets (ST1, BT14, BT20, BT23, BT24) with keyword flags, effect timing, and action callbacks
- **27+ keyword mechanics** — Blocker, Rush, Piercing, Jamming, Retaliation, Blitz, Collision, Raid, Reboot, Armor Purge, Evade, Barrier, and more
- **Transpiler** (`tools/transpiler/`) — regex-based C# to Python converter with 52 factory patterns, 32 action types, and 59 timing mappings

### RL Environment (`digimon_gym/digimon_gym.py`)

```python
from digimon_gym.digimon_gym import DigimonEnv

env = DigimonEnv()
obs, info = env.reset()                     # (981,) float32
obs, reward, terminated, truncated, info = env.step(action)
mask = env.action_mask()                    # (2,120) int8 for MaskablePPO
```

- Full **Gymnasium v1.0** API compatibility (SB3, RLlib, CleanRL)
- Dense reward: security delta, board DP differential, terminal +/-1.0
- Action masking via `info['action_mask']` (SB3 `MaskablePPO` convention)

### Agents

| Agent | Role | Status |
|-------|------|--------|
| **Pilot (PPO)** | Plays games during simulation | Training pipeline implemented (`agents/pilot_training.py`) |
| **Q-DeckRec** | Optimizes deck construction | Architecture specced ([AGENTS.md](AGENTS.md)), not yet implemented |

### DCGO Reference (`DCGO/`)

The full [DCGO Unity project](https://github.com/DCGO2/DCGO) is included as a **sparse-checkout git submodule**, providing:

- `Assets/Scripts/CardEffect/` — per-card C# effect scripts (BT1-BT24, EX1-EX11, ST1-ST24+)
- `Assets/Scripts/Script/` — core game logic (`CardController.cs`, `AttackProcess.cs`, `AutoProcessing.cs`, `TurnStateMachine.cs`, etc.)
- `Assets/CardBaseEntity/` — card metadata ScriptableObjects

This serves as the authoritative reference for game rules — the Python engine replicates game mechanics, not Unity logic.

---

## Getting Started

### Prerequisites

- Python 3.11+
- Git (with submodule support)

### Installation

```bash
# Clone with submodules
git clone --recurse-submodules https://github.com/mammalwithashell/digimon-deck-list-builder.git
cd digimon-deck-list-builder

# Install dependencies
pip install -r requirements.txt
```

If you already cloned without `--recurse-submodules`:

```bash
git submodule update --init
cd DCGO && git sparse-checkout set Assets/Scripts Assets/CardBaseEntity && cd ..
```

### Quick Validation

```bash
# Verify the Gymnasium environment
python -c "from digimon_gym.digimon_gym import DigimonEnv; env = DigimonEnv(); obs, info = env.reset(); print(obs.shape, info['action_mask'].shape)"

# Run the full test suite (1,122 tests)
python -m pytest tests/ --ignore=tests/test_rl_gym.py -v

# Run the SB3 smoke test
python scripts/train_smoke_test.py
```

---

## Usage

### Transpiler Pipeline

Convert DCGO C# card scripts to Python engine scripts:

```bash
# 1. Ingest card metadata from digimoncard.io
python tools/ingest_cards.py --set BT22           # Single set
python tools/ingest_cards.py --bulk                # All priority sets

# 2. Transpile C# scripts to Python
python tools/transpile_dcgo.py DCGO/Assets/Scripts/CardEffect/BT20 digimon_gym/engine/data/scripts/bt20

# 3. Analyze pattern coverage
python tools/transpile_dcgo.py --scan-api BT22
```

### API Server

```bash
cd digimon_gym && uvicorn api:app --reload
```

Session-based game management supporting human-vs-agent and agent-vs-agent modes.

---

## Testing

```bash
# Full suite
python -m pytest tests/ --ignore=tests/test_rl_gym.py -v

# By category
python -m pytest tests/test_runners.py -v              # Game runner tests (30)
python -m pytest tests/test_tensor_and_actions.py -v    # Tensor/action tests (48)
python -m pytest tests/test_bt24_scripts.py -v          # BT24 script validation (216)
python -m pytest tests/test_digivolve_validation.py -v  # Digivolution rules (25)
python -m pytest tests/test_dna_digivolve.py -v         # DNA/Jogress mechanics (50)
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [ACTION_SPEC.md](ACTION_SPEC.md) | Full action space specification (2,120 discrete actions) |
| [TENSOR_SPEC.md](TENSOR_SPEC.md) | Board state tensor layout (981-float tensor) |
| [AGENTS.md](AGENTS.md) | RL agent specs, MDP formulation, pilot agent types |
| [RULES_CONTEXT.md](RULES_CONTEXT.md) | Digimon TCG official rules reference |
| [CLAUDE.md](CLAUDE.md) | AI assistant development guide |

---

## References & Credits

- **Q-DeckRec:** Chen et al. (2018) — *A Fast Deck Recommendation System for CCGs*
- **DCGO:** Open-source Unity Digimon TCG engine ([github.com/DCGO2/DCGO](https://github.com/DCGO2/DCGO))
- **digimoncard.io:** Card database API for metadata and effect text
- **Egman Events / Digimon Meta:** Tournament deck list sources

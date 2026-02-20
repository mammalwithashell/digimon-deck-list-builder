# GEMINI.md — AI Assistant Guide

## Project Overview

Digimon Deck List Builder is a **reinforcement learning game engine** for the Digimon Trading Card Game. It simulates games headlessly, trains RL agents (Q-DeckRec) to optimize deck construction, and exposes a FastAPI endpoint for running simulations. The full DCGO Unity project is included as a sparse-checkout submodule in `DCGO/` for reference game logic and card effect scripts.

**Development stage:** Pre-alpha. Active development on the Python game engine and RL gym.

## Repository Structure

```
digimon_gym/                     # PRIMARY CODEBASE
├── __init__.py
├── api.py                       # FastAPI backend (session-based game management)
├── digimon_gym.py               # Gymnasium RL environment (DigimonEnv class)
├── agents/
│   ├── pilot_training.py        # Pilot agent training (MaskablePPO / MaskableRecurrentPPO)
│   ├── gauntlet.py              # MetaGauntlet — meta-weighted opponent deck sampling
│   └── maskable_recurrent/      # LSTM + action masking (custom SB3 extension)
│       ├── __init__.py           # Package exports
│       ├── buffers.py            # MaskableRecurrentRolloutBuffer
│       ├── policies.py           # MaskableRecurrentActorCriticPolicy / MaskableMlpLstmPolicy
│       └── maskable_recurrent_ppo.py  # MaskableRecurrentPPO algorithm
├── engine/
│   ├── game.py                  # Game class — turn management, phases, combat
│   ├── core/
│   │   ├── entity_base.py       # CEntity_Base — card metadata
│   │   ├── card_source.py       # CardSource — card instance wrapper
│   │   ├── player.py            # Player state (hand, deck, board zones)
│   │   ├── permanent.py         # Permanent — digimon/tamer on field
│   │   └── card_script.py       # CardScript base class
│   ├── data/
│   │   ├── cards.json           # Card database (3,921 cards across 61 sets, dict format)
│   │   ├── enums.py             # CardColor, CardKind, GamePhase, PlayerType, etc.
│   │   ├── card_database.py     # Singleton card loader
│   │   ├── card_registry.py     # Card ID ↔ integer/norm_id mapping (REGISTRY_CAPACITY=20,000)
│   │   ├── evo_cost.py          # EvoCost, DnaCost, DnaRequirement dataclasses
│   │   ├── deck_library.json    # Tournament-scraped decklists with meta weights
│   │   └── scripts/             # Per-card effect implementations (749 scripts)
│   │       ├── st1/             # Starter Set 1 (11 scripts)
│   │       ├── p/               # Promo Set (228 scripts)
│   │       ├── bt14/            # Booster Set 14 (95 scripts)
│   │       ├── bt20/            # Booster Set 20 (104 scripts)
│   │       ├── bt21/            # Booster Set 21 (104 scripts)
│   │       ├── bt23/            # Booster Set 23 (104 scripts)
│   │       ├── bt24/            # Booster Set 24 (103 scripts)
│   │       ├── ex8/             # EX Set 8 (placeholder, 0 scripts)
│   │       └── ex10/            # EX Set 10 (placeholder, 0 scripts)
│   ├── runners/
│   │   ├── base_runner.py       # BaseGameRunner — shared deck setup
│   │   ├── headless_game.py     # HeadlessGame — agent-vs-agent (RL training)
│   │   └── interactive_game.py  # InteractiveGame — human/agent (API-driven)
│   ├── validation/
│   │   └── digivolve_validator.py  # Digivolution & DNA Digivolve legality checks
│   ├── loggers.py               # IGameLogger, SilentLogger, VerboseLogger
│   └── interfaces/
│       └── card_effect.py       # ICardEffect interface
├── db/                          # Database & API layer
│   ├── __init__.py
│   ├── auth.py                  # Authentication utilities
│   ├── database.py              # Database connection setup
│   ├── models.py                # SQLAlchemy ORM models (User, Deck, GameRecord, etc.)
│   ├── schemas.py               # Pydantic request/response schemas
│   └── routers/                 # FastAPI route modules
│       ├── auth.py              # Authentication endpoints
│       ├── users.py             # User management
│       ├── decks.py             # Deck CRUD operations
│       ├── friends.py           # Friend/social features
│       └── assets.py            # Card asset serving

tools/                           # Build & pipeline tools
├── build_registry.py            # Future-proof card registry builder (API fetch, append-only indices)
├── transpile_dcgo.py            # CLI entry point (thin wrapper)
├── transpiler/                  # C# → Python card script transpiler package
│   ├── __init__.py              # Public API: parse_cs_file, generate_python_script, main
│   ├── patterns.py              # Regex patterns, TIMING_MAP, GAIN_KEYWORD_MAP (~290 lines)
│   ├── models.py                # EffectBlock dataclass (~70 lines)
│   ├── extractors.py            # C# parsing & effect extraction (~810 lines)
│   ├── generators.py            # Python code generation from EffectBlocks (~840 lines)
│   ├── validation.py            # Cross-validation against digimoncard.io (~50 lines)
│   └── cli.py                   # main() function: arg parsing, file I/O, reporting (~200 lines)
├── meta_loader.py               # Deck library builder (DigimonMeta, Egman, DigiLab scraping)
├── scraper/
│   └── scrape_decks.py          # Tournament decklist scraper (Egman Events)
├── ingest_cards.py              # Card metadata ingestion from digimoncard.io API
└── ingest_bt14_cards.py         # BT14-specific card metadata ingestion

tests/                           # Pytest test suite
├── test_runners.py              # HeadlessGame/InteractiveGame tests (30 tests)
├── test_tensor_and_actions.py   # Tensor encoding/action decoding tests (48 tests)
├── test_keyword_mechanics.py    # Engine keyword mechanics tests (75 tests)
├── test_p_scripts.py            # P (Promo) card script validation (463 parametrized tests)
├── test_bt14_scripts.py         # BT14 card script validation (200 parametrized tests)
├── test_bt20_scripts.py         # BT20 card script validation (215 parametrized tests)
├── test_bt21_scripts.py         # BT21 card script validation (216 parametrized tests)
├── test_bt23_scripts.py         # BT23 card script validation (211 parametrized tests)
├── test_bt24_scripts.py         # BT24 card script validation (216 parametrized tests)
├── test_digivolve_validation.py # Digivolution rules tests (25 tests)
├── test_dna_digivolve.py        # DNA/Jogress mechanics tests (50 tests)
├── test_build_registry.py       # Card registry builder tests (21 tests)
├── test_deck_loader.py          # Deck loading tests
├── test_phase_decoders.py       # Game phase state tests (33 tests)
├── test_maskable_recurrent.py   # MaskableRecurrentPPO LSTM+mask tests (16 tests)
├── test_gauntlet.py             # MetaGauntlet opponent sampling tests
├── test_meta_loader.py          # Deck library pipeline tests
├── test_db.py                   # Database integration tests
├── test_delay_mechanics.py      # Delay keyword mechanics tests
├── test_recording.py            # Game recording tests
└── test_replay.py               # Game replay tests

scripts/
├── train_smoke_test.py          # SB3 MaskablePPO + MaskableRecurrentPPO validation
└── fetch_card_effects.py        # Fetch card effect text from digimoncard.io API

DCGO/                            # Git submodule (sparse checkout): full DCGO Unity project
├── Assets/Scripts/CardEffect/   #   Per-card C# effect scripts (BT1-BT24, EX1-EX11, ST1-ST22+)
├── Assets/Scripts/Script/       #   Core game logic (CardController, AttackProcess, AutoProcessing, etc.)
└── Assets/CardBaseEntity/       #   Card metadata ScriptableObjects (.asset files)
ACTION_SPEC.md                   # Action space specification (2120 discrete actions)
TENSOR_SPEC.md                   # Board state tensor specification (981-float tensor)
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
| Data | JSON card database (3,921 cards across 61 sets, dict format with stable indices) |

## Common Commands

```bash
# Run all tests
python -m pytest tests/ -v

# Run runner tests only
python -m pytest tests/test_runners.py -v

# Run tensor/action tests only
python -m pytest tests/test_tensor_and_actions.py -v

# Run smoke test (validates Gymnasium env + SB3 MaskablePPO + MaskableRecurrentPPO)
python scripts/train_smoke_test.py

# Train pilot agent (MLP default, or LSTM with --lstm)
python -m digimon_gym.agents.pilot_training --timesteps 500000
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000

# Quick env validation
python -c "from digimon_gym.digimon_gym import DigimonEnv; env = DigimonEnv(); obs, info = env.reset(); print(obs.shape, info['action_mask'].shape)"

# Start the API server
cd digimon_gym && uvicorn api:app --reload

# Fetch card effect text from API (saves to card_effects_api.json)
python scripts/fetch_card_effects.py

# --- Card Registry ---

# Build/rebuild card registry from DigimonCard.io API (append-only, stable indices)
python tools/build_registry.py                     # Full build from API
python tools/build_registry.py --dry-run           # Fetch + stats, no write
python tools/build_registry.py --offline           # Rebuild norm_ids from existing data
python tools/build_registry.py --sets BT25 EX12    # Only fetch specific sets

# --- Transpiler Pipeline ---

# Ingest card metadata from digimoncard.io into cards.json
python tools/ingest_cards.py --set BT22           # Single set by ID
python tools/ingest_cards.py --bulk                # All priority sets

# Transpile a set of C# DCGO card scripts to Python
python tools/transpile_dcgo.py <DCGO_CARDEFFECT_DIR> <OUTPUT_DIR>
# Example: python tools/transpile_dcgo.py DCGO/Assets/Scripts/CardEffect/BT20 digimon_gym/engine/data/scripts/bt20
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
obs, info = env.reset()                     # (981,) float32, info has 'action_mask'
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
| 62 | Decline optional selection |
| 99 | Select own breeding area permanent |
| 100-111 | Select own battle_area permanent |
| 112-123 | Select opponent's battle_area permanent |
| 130-179 | Select trash card by index (up to 50) |
| 1000-1009 | Choose between effect branches |

Action masking via `get_action_mask()` / `action_mask()` enforces legal moves.

### Core Classes

- **`Game`** (`engine/game.py`) — Orchestrates turns, phases, combat resolution. 981-element tensor (966 board + 10 revealed + 5 selection context), 2120 action space. Card identities encoded as `norm_id` floats (index / 20000) for stable RL training.
- **`Player`** (`engine/core/player.py`) — Manages board zones: `hand_cards`, `library_cards`, `security_cards`, `trash_cards`, `breeding_area`, `battle_area`, `digitama_library_cards`. Handles security battles (Jamming), deletion prevention (Armor Purge/Evade/Barrier), deck-out loss.
- **`CardSource`** (`engine/core/card_source.py`) — Runtime card instance wrapping `CEntity_Base`
- **`Permanent`** (`engine/core/permanent.py`) — A digimon/tamer on the field with digivolution stack. Key methods: `has_keyword(attr)` (generic keyword scanner + granted keywords), `security_attack_modifier()`, `can_attack()` (summoning sickness, Rush, restrictions), `can_attack_player()` (cannot_attack_player check), `can_block()` (Blocker, Collision, restrictions). Supports `linked_cards` (option sideways attach), `opt_total`/`opt_used` (once-per-turn effect tracking), `grant_keyword(attr, duration)` for runtime keyword grants.
- **`DigimonEnv`** (`digimon_gym.py`) — Gymnasium wrapper exposing `reset()`, `step()`, `action_mask()`
- **`DigivolveValidator`** (`engine/validation/digivolve_validator.py`) — Validates digivolution and DNA digivolution legality. Provides `can_digivolve()`, `has_valid_dna_targets()`, `get_valid_dna_first_targets()`, `get_valid_dna_second_targets()`, `get_dna_stacking_order()`.
- **`EvoCost` / `DnaCost`** (`engine/data/evo_cost.py`) — Dataclasses for evolution costs and DNA Digivolution requirements (color, level, name/trait constraints).

### Effect System

Card abilities are implemented as per-card scripts:

1. Card metadata lives in `cards.json` with a `card_effect_class_name` field
2. `CardDatabase` dynamically loads `digimon_gym/engine/data/scripts/{set}/{card_id}.py`
3. Each script subclasses `CardScript` and returns `ICardEffect` instances
4. Effects define timing, conditions, and modifiers (DP, security, etc.)

### Card Registry & cards.json Format

`cards.json` is a **JSON dict** keyed by card_id (e.g. `"BT14-001"`). Each entry contains card metadata plus stable registry fields:

```json
{
  "BT14-001": {
    "index": 1479,
    "norm_id": 0.07395,
    "card_id": "BT14-001",
    "card_name_eng": "Koromon",
    ...
  }
}
```

- **`index`** — Stable integer (1-based, 0 reserved for padding). Once assigned, never changes.
- **`norm_id`** — `index / REGISTRY_CAPACITY` (= `index / 20000`). Float in `(0, 1]` used in tensor encoding.
- **Append-only** — When new sets are added, existing cards keep their indices. New cards get indices after the current max. This prevents catastrophic forgetting in trained RL agents.
- **Natural sort** — Cards are sorted by `(prefix_letters, set_number, card_number)` for the initial index assignment. E.g., BT1 < BT2 < BT10, not alphabetical BT1 < BT10 < BT2.

**Card ID patterns** — Set prefixes use non-zero-padded numbers: `BT1` (not `BT01`), `EX8` (not `EX08`). Known set types: BT1-BT26, EX1-EX13, ST1-ST24, AD1, RB1, LM, P.

**`CardRegistry`** (`card_registry.py`) provides:
- `get_id(card_id) -> int` — raw integer index
- `get_norm_id(card_id) -> float` — normalized float for tensor encoding (0.0 for unknown)
- `get_string_id(int_id) -> str` — reverse lookup
- `CAPACITY = 20_000` — registry ceiling
- Supports both dict format (reads `index`/`norm_id` fields) and legacy array format (sorts alphabetically)

**`build_registry.py`** (`tools/`) fetches all cards from DigimonCard.io API, builds the append-only registry, and writes the dict-format `cards.json`. Run it when new card sets are released.

### Transpiler Pipeline (`tools/transpiler/`)

Converts DCGO C# `CardEffect` scripts into Python `CardScript` files using regex-based extraction (not a full C# parser). The transpiler is organized as a Python package under `tools/transpiler/`:

| Module | Responsibility |
|--------|---------------|
| `patterns.py` | All compiled regex patterns, `TIMING_MAP`, `GAIN_KEYWORD_MAP` |
| `models.py` | `EffectBlock` dataclass (extracted effect metadata) |
| `extractors.py` | C# parsing: timing blocks, factory effects, activate effects, shared coroutine resolution |
| `generators.py` | Python code generation: conditions, callbacks, action emission, full script assembly |
| `validation.py` | Cross-validation against digimoncard.io data, look-ahead scanning (26 keywords, 18 actions, 15 timings) |
| `cli.py` | CLI orchestration: `--validate`, `--scan-api SET_ID\|ALL`, transpilation loop, reporting |

Entry point: `python tools/transpile_dcgo.py` (thin wrapper importing `transpiler.cli.main`).

**Capabilities:**
- **52 factory regexes** (`RE_FACTORY_*`) — 39 keyword recognition patterns + 13 value/condition extraction patterns
- **32 action types** in `_emit_action()` — delete, bounce, suspend, draw, play, digivolve, de-digivolve, mill, keyword grant with selection, token play, forced attack, SA modifier, effect disable, temp effect grant, security placement, descriptive tags, etc.
- **SharedActivateCoroutine resolution** — when a timing block delegates to a shared coroutine, the transpiler extracts the shared method body and scans it for actions
- **59 timing mappings** (`TIMING_MAP`) — C# timing enum → Python `EffectTiming` enum
- **Condition/target extraction** — parses C# lambda closures into Python conditionals

**Supported keywords** (emitted as `_is_{keyword} = True` on effects):

| Category | Keywords |
|----------|----------|
| Combat | blocker, piercing, jamming, retaliation, collision |
| Offensive | rush, raid, alliance, blitz, overclock, vortex, progress |
| Defensive | armor_purge, evade, barrier, decoy, save, material_save, fortitude |
| Phase | reboot, training |
| Digivolution | blast_digivolve, digisorption, digiburst, partition, digixros, decode |
| Modifiers | security_attack_plus, dp_modifier, change_digi_cost, alt_digivolve_req |
| Resource | set_memory_3, gain_memory_tamer, security_play |
| Special | delay, scapegoat, iceclad, fragment, execute |

**Adding a new card set:**
1. `python tools/ingest_cards.py --set BT22` — fetches metadata into `cards.json` (or `--bulk` for all priority sets)
2. `python tools/transpile_dcgo.py DCGO/Assets/Scripts/CardEffect/<SET> <OUTPUT_DIR>` — generates Python scripts
3. Write `tests/test_{set}_scripts.py` — validate transpiled scripts load and produce effects
4. Check `<OUTPUT_DIR>/TRANSPILE_REPORT.md` — tracks stubs vs fully-implemented scripts
5. `python tools/transpile_dcgo.py --scan-api <SET_ID|ALL>` — analyze keyword/action/timing pattern coverage

### Engine Keyword Mechanics

The engine checks keyword abilities at runtime via the `Permanent.has_keyword()` pattern — a generic scanner that checks inherited effects (from digivolution sources), non-inherited effects (from top card), and runtime-granted keywords (from `_granted_keywords` dict).

**Combat resolution** (`game._resolve_battle()`):
- **Summoning sickness** — `Permanent.can_attack()` blocks attacks on the turn played; `_is_rush` bypasses
- **Blitz** — when digivolved this turn, can attack even when opponent has memory (memory < 0)
- **Collision** — all opponent Digimon gain Blocker; opponent must block if able (cannot decline)
- **Piercing** — after winning a battle vs a Digimon, attacker checks opponent's security
- **Retaliation** — when a Digimon with Retaliation loses a battle, the winner is also deleted
- **Security Attack +/-** — `Permanent.security_attack_modifier()` sums all `_security_attack_modifier` values; security checks loop accordingly
- **Jamming** — in `Player.security_attack()`, attacker survives security battle losses

**Restriction keywords** (enforced at runtime):
- **Cannot Attack** (`_is_cannot_attack`) — `can_attack()` returns False
- **Cannot Attack Player** (`_is_cannot_attack_player`) — security attack masked out in action mask
- **Cannot Block** (`_is_cannot_block`) — `can_block()` returns False
- **Cannot Be Blocked** (`_is_cannot_be_blocked`) — `can_block()` returns False for potential blockers
- **Cannot Unsuspend** (`_is_cannot_unsuspend`) — skipped in `unsuspend_all()`

**Granted keyword mechanism** (`Permanent._granted_keywords`):
- Effects can grant keywords to permanents at runtime via `grant_keyword(attr, duration)`
- Duration: -1 = permanent, positive = absolute turn number of expiry
- Expired grants cleaned up at turn start via `clear_expired_grants()`
- `has_keyword()` checks granted keywords before scanning card effects

**Deletion prevention** (`player.delete_permanent()`), checked in order:
1. **Decoy** — another Digimon with Decoy is deleted instead (opponent effects only, not battle)
2. **Progress** — immune to opponent effects while attacking (blocks deletion from opponent effects)
3. **Material Save** — place 1 digivolution source under a Tamer before deletion (does not prevent deletion)
4. **Armor Purge** — trash top digivolution source to survive
5. **Evade** — suspend self (if unsuspended) to survive
6. **Barrier** — trash top security card (battle only) to survive
7. **Fortitude** — after deletion, replay top card as new permanent if had digivolution cards
8. **Save** — after deletion, place top card under a Tamer (if no Fortitude)

**Progress immunity** (`permanent.is_immune_to_opponent_effects`):
- Blocks opponent effect deletion (including Retaliation) while attacking
- Blocks security effect callbacks during security checks
- Ignores negative DP modifiers (both pre-existing and newly applied) while attacking
- DP debuffs re-apply after attack ends (`clear_attack_state()`)
- Does NOT prevent normal DP-comparison battle/security deletion (game rules, not effects)

**Phase mechanics:**
- **Reboot** — during opponent's unsuspend phase, Reboot permanents unsuspend; during own unsuspend phase, they are skipped
- **Raid** — action mask allows attacking the unsuspended highest-DP opponent Digimon
- **Training** — in Main phase, unsuspended Digimon with Training can activate via effect action: suspends self, places top deck card at bottom of digivolution stack

**End-of-turn keywords** (checked in `phase_end()` → `EndOfTurnAction` phase):
- **Vortex** — can attack opponent Digimon (only) after turn would normally end; bypasses summoning sickness
- **Overclock** — can sacrifice another Digimon to unsuspend and attack after turn end; requires a sacrifice target

**Alliance** (`resolve_attack()` → `AllianceTiming` phase):
- When an attacker with Alliance declares attack, game parks at `AllianceTiming`
- Player can select unsuspended allies to suspend for DP + SA+1 bonus per ally
- Declining (action 62) or no more allies proceeds to blocker check

**Play restrictions:**
- **Option color requirement** — action mask enforces that Options can only be played when the player has a matching-color Digimon or Tamer on the field

**Game-ending conditions:**
- **Deck-out** — `Player.draw_cards()` declares opponent as winner when deck is empty

### Singleton Pattern

`CardDatabase` is a lazy-loaded singleton — access via `CardDatabase()`.
`CardRegistry` maps card IDs to integers and normalized floats — call `ensure_initialized()` before use. Use `get_norm_id()` for tensor encoding.

## Code Conventions

- **Type hints** used throughout; `TYPE_CHECKING` imports for circular dependency avoidance
- **Enums** for all game constants (`GamePhase`, `CardColor`, `CardKind`, `EffectTiming`, `PendingAction`, `PlayerType`)
- **Property decorators** for computed values on `Permanent` (level, DP, etc.)
- **Imports** use `digimon_gym.*` package prefix throughout
- **Headless design** — all game logic runs without UI; state serialized to NumPy arrays
- **Transpiler-first policy** — when card script stubs or missing effects are found, fix the transpiler (`tools/transpile_dcgo.py`) rather than editing individual scripts. This ensures fixes apply to all cards with the same C# pattern and are preserved on re-transpile.
- **Rules-aware implementation** — when implementing or reviewing card effects (in the transpiler or individual scripts), consult `RULES_CONTEXT.md` for official keyword behavior, effect timing semantics, and processing conditions. Key distinctions: mandatory vs optional processing, persistent vs trigger vs immediate effect types, and turn-player-first simultaneous resolution.

## Testing Guidelines

- Use **pytest** for all tests
- Test files go in `tests/` (root level)
- Mock card helpers exist in test files — reuse them for new tests
- Script validation tests (`test_p_scripts.py`, `test_bt{14,20,21,23,24}_scripts.py`) are parametrized — 463, 200, 215, 216, 211, and 216 tests respectively verifying transpiled scripts load, produce correct effect counts, and set expected keyword flags
- `test_build_registry.py` — 21 tests for natural sort key parsing, append-only index preservation, capacity validation, determinism, and deduplication
- `test_maskable_recurrent.py` — 16 tests for LSTM+mask buffer, policy, and algorithm integration
- `test_gauntlet.py` — MetaGauntlet opponent sampling and GauntletWrapper tests
- `test_meta_loader.py` — Deck library pipeline tests
- Run `python -m pytest tests/ -v` for the full suite (2,077 tests)

## Card Data API

Card details for any Digimon card can be fetched from the digimoncard.io public API:
```
https://digimoncard.io/index.php/api-public/search?card=BT20-001
https://digimoncard.io/index.php/api-public/search?pack=BT-20:%20Booster%20Over%20the%20X
```
- Single card lookup: `?card={CARD_ID}` (e.g. `?card=BT20-001`)
- Full set lookup: `?pack=BT-{N}:%20{SetName}` (e.g. `?pack=BT-20:%20Booster%20Over%20the%20X`)
- Returns JSON with card metadata including effect text, DP, level, colors, evolution costs, traits, etc.
- Use this API to verify card effect text when implementing or reviewing card scripts.

## Key Documentation

- **ACTION_SPEC.md** — Full action space specification (2120 discrete actions, selection phases, attack flow)
- **TENSOR_SPEC.md** — Board state tensor layout (981-float tensor, slot encoding, norm_id card encoding, global data)
- **AGENTS.md** — RL agent specs, MDP formulation, pilot agent types
- **RULES_CONTEXT.md** — Comprehensive Digimon TCG rules reference derived from the official Comprehensive Rules Manual (Ver.3.6) and Official Rule Manual (Ver.5.0). Consult this when implementing card effects, especially for keyword mechanics, effect timing/triggering rules, and processing conditions.
- **Q-Rec Agent Notes** — Q-DeckRec network architecture, hyperparameters, training loop

## Known Gaps

### Engine Keywords Not Yet Runtime-Checked
The transpiler emits flags for these keywords, but the engine does not yet act on them:

| Keyword | Flag | Scripts using it | Notes |
|---------|------|:---:|-------|
| Delay | `_is_delay` | 19 | Option cards with delayed effects |
| Execute | `_is_execute` | 5 | Trigger effect when placed in trash from hand |
| Cannot Be Deleted By Battle | `_is_cannot_be_deleted_by_battle` | 5 | Restriction keyword |
| Cannot Return To Hand | `_is_cannot_return_to_hand` | 4 | Restriction keyword |
| Cannot Return To Deck | `_is_cannot_return_to_deck` | 4 | Restriction keyword |
| Decode | `_is_decode` | 3 | Alternative digivolution from deck |
| Partition | `_is_partition` | 3 | Split into component Digimon on deletion |
| Scapegoat | `_is_scapegoat` | 3 | Redirect deletion to another target |
| Cannot Suspend | `_is_cannot_suspend` | 3 | Restriction keyword |
| Cannot Unsuspend (opponent) | `_is_cannot_unsuspend_player` | 3 | Restrict opponent unsuspend |
| Cannot Suspend (opponent) | `_is_cannot_suspend_player` | 2 | Restrict opponent suspend |
| Immune to DP Minus | `_is_immune_dp_minus` | 1 | Ignore DP reduction effects |

**All runtime-implemented keywords:** Rush, Blocker, Piercing, Jamming, Retaliation, Collision, Blitz, Raid, Reboot, Blast Digivolve, Alliance, Training, Progress, Fortitude, Save, Decoy, Material Save, Vortex, Overclock, Security Attack +/-, Armor Purge, Evade, Barrier, and restriction keywords (cannot_attack, cannot_attack_player, cannot_block, cannot_be_blocked, cannot_unsuspend), plus the granted keyword mechanism and option color requirement.

### Descriptive-Tagged Effects (Pending Engine Features)
~201 effect callbacks across 5 sets are recognized by the transpiler but emit `pass # descriptive-tagged` because the engine lacks support. These represent the largest category of incomplete card functionality:

| Tag | Count | Engine feature needed |
|-----|:-----:|----------------------|
| cost_reduction | 44 | Dynamic play/digivolve cost modification |
| force_attack | 40 | Force an opponent's Digimon to attack |
| disable_effect | 17 | Invalidate/nullify effects on a target |
| play_token | 11 | Token Digimon creation and placement |
| change_security_attack | 11 | Dynamic SA+/- modification via effects |
| also_treated_as_name | 11 | Treat card as having additional names |
| redirect_attack | 5 | Change attack target mid-combat (SwitchDefender) |
| effect_immunity | 5 | Grant immunity to opponent effects (CanNotAffectedClass) |
| grant_skill | 5 | Grant keywords to other permanents (AddSkillClass) |
| add_temp_effect | 4 | Temporarily add effects to permanents |
| also_treated_as_level | 2 | Treat card as having additional levels |
| attack_unsuspended | 2 | Allow attacking unsuspended Digimon |

| Set | No-action stubs | Descriptive-tagged files |
|-----|:--------------:|:-----------------------:|
| P | 34 | 26 |
| BT14 | 1 | 13 |
| BT20 | 8 | 34 |
| BT21 | 31 | 15 |
| BT23 | 7 | 36 |
| BT24 | 3 | 24 |

### Transpiler Stub Summary
84 effect callbacks across 6 sets still produce no-action stubs (50 across BT14/20/21/23/24, plus 34 in the P promo set). The BT set stubs were reduced from 116 after P7 stub reduction: widened `_extract_method_body()` regex, ChangeCostClass value extraction, Mode.Custom helper class scanning (IDegeneration, SwitchDefender, PlayPermanentCards, DigivolveIntoHandOrTrashCard, CanNotAffectedClass), AddSkillClass detection, metadata class detection (AddJogressLevelsClass, ChangeCardNamesClass, CanAttackTargetDefendingPermanentClass), and orphan pass elimination. BT21 transpilation introduced additional pattern fixes: `RushSelfStaticEffect` and `CollisionSelfStaticEffect` recognition, hashtable lambda delegate resolution for shared `ActivateCoroutine`, and narrowed `RE_TRASH_HAND` to only match explicit `Mode.Discard` (preventing false positives from `SelectHandEffect` in custom mode).

Remaining stubs are complex multi-step sequences with nested coroutines, `OnAddDigivolutionCards` timing blocks, and effects requiring engine features not yet supported.

### Other Gaps
- EX8 and EX10 script directories are empty placeholders (no scripts transpiled yet)
- ST1 scripts (11 files) have no dedicated test coverage
- No CI/CD pipeline
- No frontend implementation yet (React planned)
- Q-DeckRec agent not yet implemented (architecture specced in AGENTS.md); pilot agent training exists in `digimon_gym/agents/pilot_training.py`

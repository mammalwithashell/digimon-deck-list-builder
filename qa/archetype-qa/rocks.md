# Archetype QA: Rocks
Date: 2026-04-08
Total cards: 47
Pipeline: batch-fix-cards (12 batches)

## Summary
- **FAITHFUL (PASS)**: 9
- **FIXED**: 38
- **PARTIAL**: 0
- **BLOCKED**: 0

## Per-Card Verdicts

### Batch 1 — EX10 Core Combo
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| EX10-032 | Proganomon | FIXED | 14/14 | 3-step selection flow for OP/WD/WA, SEL_TRASH_START fix, is_tamer/is_digimon guards |
| EX10-063 | Close | FIXED | 15/15 | Removed overly restrictive condition pre-check |
| EX10-069 | Unique Emblem: Gravel Hearts | FIXED | 15/15 | Digi filter allows Rock+LIBERATOR, is_tamer check |
| EX10-025 | Sunarizamon | FIXED | 12/12 | Selection order, SEL_TRASH_START bug, target validation |

### Batch 2 — EX10 Higher-End
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| EX10-033 | Pyramidimon | FIXED | 26/26 | SEL_TRASH_START fix, player selection for cost-reduce |
| EX10-034 | Blastmon | FIXED | 10/10 | Condition (≥3 cards), player selects which 2 to trash |
| EX10-036 | Magneticdramon | FIXED | 17/17 | Alt-digi (Proganomon base + Close cond), delete uses player selection |
| EX10-028 | Landramon | FIXED | 10/10 | Inherited missing card-in-trashed check, proper API |

### Batch 3 — EX8 Core
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| EX8-067 | Close | FIXED | 11/11 | Auto-selection replaced with player-driven SelectTrash |
| EX8-048 | Landramon | FIXED | 11/11 | Inherited missing card-in-trashed check |
| EX8-047 | Sunarizamon | FIXED | 19/19 | Added is_opponent_effect=True to delete_permanent |
| EX8-005 | Tumblemon | PASS | 11/11 | Faithful |

### Batch 4 — EX8 Mid/High
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| EX8-050 | Gogmamon | FIXED | 15/15 | Redirect attack implemented (was stub), On Deletion reveal refactored |
| EX8-051 | Proganomon | PASS | 26/26 | Faithful, minor _fragment_count consistency |
| EX8-055 | Pyramidimon | FIXED | 12/12 | Auto-trash to SelectSource, SEL_TRASH_START fix in EOT |
| EX8-070 | Zofr Kabus | FIXED | 18/18 | Trash auto-select replaced with player SelectSource |

### Batch 5 — Inherited Evo-Trash Triggers
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| BT21-055 | Sunarizamon | FIXED | 11/11 | **CRITICAL: timing was BeforePayCost (dead) → WhenWouldDigivolve**, wrong context key |
| EX8-046 | Gotsumon | PASS | 13/13 | Faithful |
| EX10-003 | Tumblemon | FIXED | 14/14 | OnUseAttack→OnAllyAttack, player source selection, action_decoder combat resume |
| EX11-038 | Sunarizamon | FIXED | 14/14 | Zone choice, OnMove gating, action_move_from_breeding fix |

### Batch 6 — Promo Core
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| P-167 | Landramon | FIXED | 15/15 | trash_specific_digivolution_cards API, inherited missing card check |
| P-169 | Close | FIXED | 10/10 | Trash auto-selection replaced with player SelectTrash |
| P-107 | Defense Training | FIXED | 15/15 | Missing security effect added |
| P-215 | Icemon | FIXED | 8/8 | Trash player choice, hand/trash zone choice, protection targets 1 not all |

### Batch 7 — EX7/EX11
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| EX7-049 | Metallicdramon | FIXED | 11/11 | WhenRemoveField own-effect check, permanent_of_this_card fix |
| EX11-044 | Pyramidimon | FIXED | 12/12 | Auto-trash to player SelectSource, place-from-trash to SelectTrash |
| EX11-065 | Close | FIXED | 10/10 | Branch choice for hand/trash, removed duplicate WhenDigivolving |
| EX7-074 | Vortex Resonance | PASS | 21/21 | Faithful |

### Batch 8 — Generic Utility Options
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| LM-031 | Black Scramble | FIXED | 17/17 | SEL_TRASH_START, broken selection chaining, OnStartTurn condition |
| P-039 | Black Memory Boost! | FIXED | 9/9 | Added missing security effect |
| P-206 | Digital Gate Open | PASS | 9/9 | Faithful |
| BT9-103 | Kongou | FIXED | 7/7 | **CRITICAL: CANNOT_ATTACK→CANNOT_ATTACK_PLAYER**, modifier identity condition |

### Batch 9 — BT Tech Cards
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| BT14-009 | Gotsumon | FIXED | 7/7 | CANNOT_PLAY_CARD→CANNOT_PLAY_BY_EFFECT |
| BT16-082 | Ukkomon | FIXED | 9/9 | Hatch chaining bug — was overwriting reveal selection |
| BT18-064 | Mercurymon | PASS | 10/10 | Faithful |
| BT20-055 | Invisimon | FIXED | 17/17 | Security play uses effect_play_from_security, face-up security tracking |

### Batch 10 — BT Splashable Tech
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| BT21-021 | OmniShoutmon | FIXED | 8/8 | Alt-digi level + Hero trait, missing self-deletion (added on_played callback) |
| BT23-059 | Justimon: Blitz Arm | FIXED | 11/11 | Alt-digi CS trait, OPT shared counter, unsuspend trigger broken |
| BT23-096 | Comet Hammer | PASS | 8/8 | Faithful |
| BT4-072 | Gogmamon | FIXED | 5/5 | **Wouldn't compile** (MainAction enum), DP duration wrong |

### Batch 11-12 — Remaining Tech
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| BT8-094 | Digimon Emperor | FIXED | 11/11 | Deletion observer pattern fix |
| P-123 | Ukkomon | PASS | 5/5 | Faithful |
| P-130 | Lui Ohwada | FIXED | 8/8 | Missing on-play process callback |
| P-186 | Gallantmon | PASS | 16/16 | Faithful |
| ST13-08 | Chikurimon | FIXED | 5/5 | Declarative passive + engine support for `_blocks_cost_reduction` |
| ST22-11 | Defense Plug-In F | FIXED | 7/7 | Missing color bypass clause |
| LM-032 | Purple Scramble | PASS | 11/11 | Faithful |

## Critical Bug Categories

### 1. Auto-Selection Anti-Pattern
Many cards auto-selected which trash card to use, violating the no-approximations policy. All converted to player choice via `SelectTrash`/`SelectSource` phases. Affected: EX10-025, EX10-032, EX10-033, EX10-034, EX10-036, EX8-055, EX8-067, EX8-070, EX10-003, EX11-038, EX11-044, EX11-065, P-169, P-215.

### 2. SEL_TRASH_START Subtraction Bug
Trash selection callbacks received raw `action_id` (e.g., 130+) but treated it as a list index without subtracting `SEL_TRASH_START`. The check `if idx < len(player.trash_cards)` always failed silently. Affected: EX10-025, EX10-032, EX10-033, EX10-036, EX8-055, LM-031, others.

### 3. Inherited "card in trashed_cards" Check Missing
Inherited effects with "When effects trash this card" pattern triggered for ANY card trashed from the same permanent, not just the specific card. Affected: EX10-028, EX8-048, P-167.

### 4. Wrong Timing for Cost Reduction (CRITICAL)
**BT21-055 Sunarizamon**: Used `BeforePayCost` for digivolution cost reduction, but this timing only fires for play-from-hand. The digivolve cost flow uses `WhenWouldDigivolve`. Effect was completely dead.

### 5. Wrong Modifier Type (CRITICAL)
**BT9-103 Kongou**: Used `CANNOT_ATTACK` (blocks all attacks) instead of `CANNOT_ATTACK_PLAYER` (blocks only player attacks).

### 6. Non-Compiling Script (CRITICAL)
**BT4-072 Gogmamon**: Used non-existent `EffectTiming.MainAction` — script wouldn't load.

## Engine Improvements

1. **`permanent.py`** — `OnDigivolutionCardDiscarded` now fires before card removal, enabling inherited "trash this card" effects to be found by normal field scan
2. **`game/effects.py`** — Added `on_played` callback to `effect_play_from_zone`
3. **`game/__init__.py`**:
   - Declarative `_blocks_cost_reduction` scan for ST13-08 Chikurimon
   - `action_move_from_breeding` now allows OnMove selection phases
   - `security_cards` added to zone scan for face-up security effects
4. **`action_decoder.py`** — `_decode_trash_selection` and `_decode_source_selection` now resume combat after WA-style selections
5. **`combat.py`** + **`player.py`** — `OnSecurityCheck` context now includes `security_card` and `security_was_face_up`

## Test Statistics
- 47 test files created/updated
- 470+ behavioral tests written
- All tests pass when run per-batch
- 8 pre-existing test isolation failures unaffected (BT20-016, BT21-029, BT21-081, EX11-020)

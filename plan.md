# EDH Mode — Engine-Only Implementation Plan

## Goal
Add EDH (Commander) mode as a parallel engine lane alongside the standard 2-player game.
No changes to existing `Game`, `Player`, `BaseGameRunner`, or standard runners.

---

## Phase 1: EDH Constants & Data Layer

### File: `digimon_gym/engine/edh/__init__.py`
- Empty package init.

### File: `digimon_gym/engine/edh/edh_constants.py`
Define EDH-specific tensor/action space constants:
- `EDH_PLAYER_COUNT = 4`
- `EDH_STARTING_SECURITY = 7`
- `EDH_DECK_SIZE = 70` (singleton)
- `EDH_MAX_TURNS = 600`
- Selection constants expanded for 3 opponents:
  - `SEL_OPP1_FIELD_START/END` (112-123), `SEL_OPP2_FIELD_START/END` (200-211), `SEL_OPP3_FIELD_START/END` (212-223)
  - Same for security: 3 opponent security ranges
- Action space:
  - Attack range expanded: 12 attackers × (12 targets per opponent × 3 opponents + 3 security attacks) = 12 × 39 = 468 attack actions
  - Keep play/digivolve/effect ranges same as standard
  - Commander play/digivolve: new range (e.g., 2120-2139)
  - `EDH_ACTION_SPACE_SIZE = 2360`
- Tensor layout (~1876 floats):
  - [0-9] Global (turn, phase, memory, active_player_seat, eliminated_mask)
  - [10-381] My field (12 × 31)
  - [382-753] Opp1 field
  - [754-1125] Opp2 field
  - [1126-1497] Opp3 field
  - [1498-1517] My hand (20)
  - [1518-1567] Opp hands (3 × ~16)
  - [1568-1637] Trash zones
  - [1638-1677] Security zones (me + 3 opps)
  - [1678-1739] Breeding zones (4 × 31, but compact)
  - [1740-1779] Revealed + selection context
  - [1780-1811] Commander zones (4 × 8: card_id, tax, in_zone, on_field...)
  - `EDH_TENSOR_SIZE = ~1876`

---

## Phase 2: EDHPlayer

### File: `digimon_gym/engine/edh/edh_player.py`

```python
class EDHPlayer(Player):
```

**New attributes:**
- `self.opponents: List[Player]` — all living opponents (clockwise order)
- `self.commander_zone: List[CardSource]` — 0 or 1 card (the commander)
- `self.commander_tax: int = 0` — increments by 2 each return-to-zone
- `self.is_eliminated: bool = False`
- `self.seat: int` — 0-3, clockwise position
- `self.commander_card_id: Optional[str]` — the commander's card ID for zone-return detection

**Backward compat:**
- `@property enemy` → returns `self.opponents[0]` (clockwise-next) for existing card scripts

**Overrides:**
- `setup_security_stack(count=7)` — 7 instead of 5
- Zone-change hooks: override methods that move cards to trash/hand/deck to intercept commander returns:
  - When a CardSource with `card_id == commander_card_id` would move to trash/hand/deck/security, offer the zone-return replacement
  - In headless mode: always return to command zone (optimal strategy assumption)
  - Increment `commander_tax += 2` on each return

**New methods:**
- `eliminate()` — mark eliminated, clear zones, remove from opponents' lists
- `play_commander(game)` — play commander from zone, paying cost + tax
- `digivolve_commander(game, target_perm)` — digivolve commander onto a field perm

---

## Phase 3: EDHGame

### File: `digimon_gym/engine/edh/edh_game.py`

```python
class EDHGame(Game):
    def __init__(self, logger=None):
        # Skip Game.__init__() — it hardcodes 2 players
        # Manually init all required state
```

**Players:**
- `self.players: List[EDHPlayer]` — 4 players, seats 0-3
- `self.turn_player` / `self.opponent_player` — backward-compat aliases
  - `turn_player`: current active player
  - `opponent_player`: clockwise-next (the memory seesaw partner)
- `self.player1` / `self.player2` — aliases to `players[0]` / `players[1]` for card script compat
- `self.active_players: List[EDHPlayer]` — non-eliminated, in seat order

**Memory (clockwise seesaw):**
- `self.memory: int` — seesaw between active player and clockwise-next
- Override `_get_memory_for(player)`:
  - If player is turn_player: return `self.memory`
  - If player is opponent_player (next clockwise): return `-self.memory`
  - For other opponents: return 0 (not on the seesaw)

**Turn management:**
- `switch_turn()`: advance to next non-eliminated player clockwise
  - Negate memory, update `turn_player` / `opponent_player`
- `pass_turn()`: set memory to -3, advance

**Win condition:**
- `eliminate_player(player)`: call `player.eliminate()`, check if 1 remains → `declare_winner()`
- `check_elimination(player)`: called on deck-out, direct attack hit with 0 security

**Mulligan:**
- 4-player mulligan order (clockwise from first player)

**Phases — overrides:**
- `phase_start()`: Reboot unsuspends for ALL opponents (not just one)
- Draw phase: all players skip draw on round 1

**Combat — overrides:**
- `action_attack_player(attacker_idx, target_player_idx)` — attack ANY opponent's security
- `action_attack_digimon(attacker_idx, target_player_idx, target_digimon_idx)` — expanded targeting
- BlockTiming/CounterTiming: only the defending player (the attacked opponent) responds

**Card effect compat — "your opponent" disambiguation:**

The key discovery is that card scripts don't directly iterate `player.enemy.battle_area` for targeting — they call engine methods like `game.effect_select_opponent_permanent()`. This is the choke point.

**Single-target effects** ("delete 1 of your opponent's Digimon"):
- Scripts call `game.effect_select_opponent_permanent(player, callback, filter_fn)`
- Standard: builds valid indices from `SEL_OPP_FIELD_START` (112-123), one opponent
- **EDH override**: builds valid indices across ALL opponents' fields using expanded ranges (`SEL_OPP1_FIELD 112-123`, `SEL_OPP2_FIELD 200-211`, `SEL_OPP3_FIELD 212-223`)
- Player chooses which opponent's Digimon to target — **no script changes needed**
- The `on_select` callback maps the selected index back to the correct opponent + permanent

**"All opponents" effects** ("all your opponent's Digimon get -3000 DP"):
- Scripts that iterate `player.enemy.battle_area` only hit one opponent via the `enemy` shim
- **EDH override**: `EDHPlayer` adds an `all_opponent_permanents()` helper that yields `(opponent, perm)` for all opponents
- Engine methods that apply blanket effects (e.g., `effect_apply_to_all_opponent_digimon`) are overridden in EDHGame to iterate all opponents
- Scripts using the `game.effect_*` API work automatically; scripts directly accessing `player.enemy.battle_area` get only clockwise-next (acceptable v1 compromise, audit later)

**"Your opponent" direct effects** ("your opponent trashes 1 security"):
- ~920 occurrences across ~313 card scripts directly access `player.enemy.trash_cards`, `.security_cards`, `.hand_cards`
- EDH override: inject a **player-selection step** before the effect resolves — player chooses which opponent is affected
- Two-layer approach:
  1. **Engine-mediated path**: override `effect_select_opponent_permanent` and `effect_select_opponent_security` in EDHGame to present targets across all opponents (already covered above)
  2. **Direct `player.enemy` path**: EDHPlayer's `enemy` property is **context-aware** — when the game is in a selection/effect resolution phase, `enemy` returns the **currently targeted opponent** (set by a prior opponent-selection step). Outside of effect context, returns clockwise-next as default.
  3. **New `SelectOpponent` phase**: added to EDHGame. When an effect needs "your opponent", EDHGame first pushes a `SelectOpponent` selection (choose opponent seat 0-2), then sets `player._targeted_enemy` to the chosen opponent before the effect callback runs. The `enemy` property reads `_targeted_enemy` if set, otherwise falls back to `opponents[0]`.
- This means existing scripts like `enemy.security_cards.pop(0)` work correctly — the `enemy` reference resolves to whichever opponent was chosen
- Adds `SelectOpponent` as a new `GamePhase` value in EDH (or reuse `SelectTarget` with seat indices)

**Tensor writer — override:**
- `get_board_state_tensor(player_id)` → builds ~1876-float tensor with 4P perspective

**Action mask — override:**
- `get_action_mask(player_id)` → 2360-float mask
  - Attack range: expanded for 3 opponents
  - Commander play/digivolve: new range
  - Selection ranges: expanded opponent field indices

**Action decoder — override:**
- `decode_action(action_id, player_id)` → expanded attack decoding for multi-opponent targeting
  - New commander play/digivolve actions

---

## Phase 4: EDH Base Runner

### File: `digimon_gym/engine/edh/edh_base_runner.py`

```python
class EDHBaseRunner(ABC):
```

- Takes 4 deck lists + optional commander IDs
- Creates `EDHGame` instead of `Game`
- Sets up commander zones from commander card IDs
- Validates singleton 70-card decks
- Reuses `BaseGameRunner._setup_deck()` static method for card loading

---

## Phase 5: EDH Headless Runner

### File: `digimon_gym/engine/runners/edh_headless_game.py`

```python
class EDHHeadlessGame(EDHBaseRunner):
```

- 4-player agent-only simulation
- `run_until_conclusion(max_turns=600, policy_fn=None)` → winner seat or 0
- `step(action_id)` — same interface as `HeadlessGame`
- `get_action_mask()` / `get_board_tensor()` using EDH sizes
- Commander zone-return decisions: auto-return in headless (default policy)

---

## Phase 6: Tests

### File: `tests/test_edh_mode.py`

**Test cases:**
1. **Game creation**: 4 players initialized, 70 cards each, 7 security
2. **Commander zone**: commander starts in zone, playable with tax
3. **Commander tax**: tax increments on zone return, cost increases
4. **Memory seesaw**: clockwise memory passing works correctly
5. **Turn order**: clockwise P1→P2→P3→P4→P1, skipping eliminated
6. **Player elimination**: eliminated player removed, last standing wins
7. **Attack targeting**: can attack any opponent's security or Digimon
8. **Block/counter**: only defending player can block
9. **Reboot**: unsuspends on each opponent's turn
10. **Round 1 draw skip**: all players skip draw on turn 1
11. **Backward compat**: `player.enemy` returns clockwise-next
12. **Effect target disambiguation**: `effect_select_opponent_permanent` offers targets across all 3 opponents' fields; selecting opp2 slot 3 maps to the correct permanent
13. **All-opponents effects**: blanket effects (e.g., DP reduction) hit all opponents' permanents
14. **Direct enemy targeting**: scripts accessing `player.enemy` in effect callbacks get the opponent chosen via `SelectOpponent` phase, not just clockwise-next
15. **Full game simulation**: 4 random-policy agents play to conclusion
15. **Tensor shape**: tensor is correct size (~1876)
16. **Action mask shape**: mask is correct size (2360)

---

## Implementation Order

```
Phase 1: edh_constants.py          (no deps)
Phase 2: edh_player.py             (depends on Player, Phase 1)
Phase 3: edh_game.py               (depends on Game, Phase 1-2)
Phase 4: edh_base_runner.py        (depends on Phase 3)
Phase 5: edh_headless_game.py      (depends on Phase 4)
Phase 6: test_edh_mode.py          (depends on all above)
```

Each phase is independently testable. Phase 3 (EDHGame) is the largest — it can be broken into sub-PRs:
- 3a: init, players, memory, turn management
- 3b: combat (attack targeting, blocking)
- 3c: tensor writer + action mask + decoder

---

## Key Risk Areas

1. **Card script compatibility**: ~2500 card scripts reference `player.enemy`. The `enemy` property shim should handle most cases, but effects that do `player.enemy.battle_area` iterations may behave differently when there are 3 opponents. Need audit of "all opponent's" patterns.

2. **Game.py method coupling**: EDHGame needs to override many methods. If Game.py methods call each other (e.g., `action_play_card` calls `check_turn_end` calls `switch_turn`), EDHGame overrides must be consistent. May need to override more methods than initially expected.

3. **Memory model simplicity**: The "clockwise seesaw" is simple but means only 2 players are on the gauge at any time. Card effects that manipulate memory for non-seesaw players need a clear rule (spec says: applies to active seesaw).

4. **Tensor/action space stability**: The EDH tensor and action spaces are completely separate from standard. No risk of breaking standard training.

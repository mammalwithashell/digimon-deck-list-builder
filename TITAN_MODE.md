# Digimon TCG — Titan Mode

An asymmetric multiplayer format for the Digimon TCG where one powerful Titan player faces off against a team of 2-3 standard players.

## Format Overview

| Parameter | Standard | Titan (Titan Player) | Titan (Team Player) |
|-----------|----------|---------------------|---------------------|
| Players | 2 | 1 (Titan) | 2-3 (Team) |
| Main deck | 50 cards | 80 cards | 50 cards |
| Egg deck | 0-5 cards | 0-5 cards | 0-5 cards |
| Copies per card | Up to 4 | Up to 4 | Up to 4 |
| Starting security | 5 | 15 | 5 |
| Starting hand | 5 | 7 | 5 |
| Battle area slots | 12 | 12 | 12 |
| Memory system | Shared seesaw (2P) | Alternating seesaw (Titan ↔ next team player) | Same |
| Win condition | Opponent's security depleted + direct attack | Eliminate all team players | Eliminate the Titan |

---

## Deck Construction

### Titan Player
- **80-card main deck** — standard copy limits (up to 4 copies of any card).
- **Egg deck** — 0 to 5 Digi-Egg cards, same as standard.
- No singleton, commander, or special deck-building restrictions.

### Team Players
- **50-card main deck** — standard deck construction rules.
- **Egg deck** — 0 to 5 Digi-Egg cards, same as standard.
- Each team player builds their deck independently. There are no restrictions on cards shared between team players' decks.

---

## Titan Player Advantages

The Titan compensates for facing multiple opponents through stat advantages only — no special abilities or boss mechanics:

- **80-card deck** (vs 50) — deeper card pool, significantly harder to deck out. The Titan must sustain a longer game while the team chips away at their security.
- **15 starting security** (vs 5) — the team must collectively work through three times the security of a normal player. This is the primary balancing lever.
- **7-card starting hand** (vs 5) — more opening options to compensate for facing multiple boards from turn 1.

The Titan uses standard draw (1 card per draw phase), standard battle area slots (12), and standard memory rules. The Titan takes more total turns than any individual team player (one turn per rotation slot), which naturally provides a tempo advantage.

---

## Memory System — Alternating Seesaw

The standard Digimon TCG memory gauge is a shared seesaw between two players. In Titan Mode, this is adapted for the alternating turn structure:

### Turn Rotation

Turns alternate between the Titan and each team player in sequence:

**With 3 team players:**
Titan → Team P1 → Titan → Team P2 → Titan → Team P3 → Titan → Team P1 → ...

**With 2 team players:**
Titan → Team P1 → Titan → Team P2 → Titan → Team P1 → ...

The Titan always takes the first turn of the game.

### How the Seesaw Works

The memory seesaw always exists between exactly two players: the **active player** and the **next player in rotation**.

- When the Titan is active, the seesaw is between the Titan and the next team player.
- When Team P1 is active, the seesaw is between Team P1 and the Titan.
- When the Titan is active again, the seesaw is between the Titan and Team P2.
- And so on.

### Memory Rules (same as standard, applied to the active pair)

- At the start of your turn, if memory is at 0 or less (on your side), it resets to 3.
- Playing cards and using effects costs memory, moving the gauge toward the next player's side.
- If memory crosses to the next player's side (goes below 0 for the active player), the active player's turn ends.
- **Pass action:** When the active player passes, memory is set to 3 on the next player's side (memory = -3), and the turn ends.
- The memory gauge ranges from -10 to +10.

### Turn Transition

When a turn ends and the next player's turn begins:
- The memory value is **negated** (flipped perspective) — standard seesaw flip.
- The new seesaw is between the new active player and the next player after them in rotation.

### Example: 3 Team Players

```
Turn 1 (Titan vs Team P1 seesaw):
  Titan starts with memory = 3
  Titan plays cards, memory drops to -2
  → Turn ends. Memory negated: -(-2) = 2

Turn 2 (Team P1 vs Titan seesaw):
  Team P1 starts with memory = 2
  Team P1 plays cards, memory drops to -1
  → Turn ends. Memory negated: -(-1) = 1

Turn 3 (Titan vs Team P2 seesaw):
  Titan starts with memory = 1 (≤ 0 check: 1 > 0, no reset)
  Titan plays a 4-cost card, memory drops to -3
  → Turn ends. Memory negated: -(-3) = 3

Turn 4 (Team P2 vs Titan seesaw):
  Team P2 starts with memory = 3
  ...and so on
```

### Strategic Implications

- The Titan can "feed" a specific team player by ending their turn with a large positive memory balance (which negates to a large value for that team player's turn).
- Conversely, the Titan can "starve" a team player by spending aggressively, leaving the team player with minimal memory.
- Team players face an interesting tension: spending aggressively helps attack the Titan but leaves the Titan with more memory for their next turn.
- Since the Titan takes a turn between every team player's turn, the Titan naturally gets N turns for every 1 turn each team player gets (where N = number of team players in the seesaw cycle). However, the Titan's memory on each individual turn may be low.

---

## Turn Structure

Same phase order as standard:

**Unsuspend Phase → Draw Phase → Breeding Phase → Main Phase → End Phase**

### Modifications for Titan Mode

**Unsuspend Phase:**
- The turn player unsuspends all their suspended Digimon and Tamers (Reboot permanents are skipped, as in standard).
- **Reboot** permanents unsuspend during each opponent's unsuspend phase:
  - The Titan's Reboot permanents unsuspend on **every team player's turn** (2-3 times per full rotation).
  - A team player's Reboot permanents unsuspend only on the **Titan's turns** (but the Titan takes multiple turns per rotation, so Reboot triggers frequently).

**Draw Phase:**
- The Titan (who always goes first) skips their draw on the first turn of the game, same as standard's "first player skips draw" rule.
- Team players draw normally on their first turns (they are not the "first player").
- The Titan draws 1 card per draw phase (standard). Despite having an 80-card deck, the Titan does not get bonus draws.
- Deck-out causes **elimination**, not an immediate game win for an opponent.

**Breeding Phase:**
- Same as standard. Hatch, move, or do nothing.

**Main Phase:**
- Same actions available as standard: play cards, digivolve, use options, attack, activate effects, or pass.
- **Attack targeting for team players:** Team players can only attack the Titan. There is only one opponent.
- **Attack targeting for the Titan:** The Titan may attack **any** team player — choosing which opponent's Digimon or security stack to target.

**End Phase:**
- Same as standard.

---

## Combat

### Attacking

**Team player attacks:**
- Team players may only attack the Titan. From a team player's perspective, combat functions identically to standard 2-player rules.

**Titan attacks:**
- The Titan may declare attacks against **any** team player's Digimon or security stack.
- The Titan declares which opponent they are targeting before selecting a target permanent or security.

### Blocking and Counter Timing

- Only the **defending player** (the player being attacked) may declare a blocker or use counter timing effects.
- Other team players cannot intervene in combat between the Titan and a specific team player.
- In practice, this means blocking always works as in standard — the defender decides whether to block with their own Digimon.

### Security Attacks

- Security attacks check the defending player's security stack, same as standard.
- Jamming, Piercing, Security Attack +/- all function as in standard, applied to the defending player.
- When attacking the Titan's security, the team player checks the Titan's security stack (which starts at 15).

### Direct Attack (Elimination)

- When a player with 0 security cards is successfully attacked by a Digimon that can perform 1 or more security checks, that player is **eliminated**.
- Eliminating the Titan ends the game immediately (team wins).
- Eliminating a team player removes them from the rotation (see Player Elimination).

---

## Player Elimination

### Team Player Elimination

When a team player is eliminated:
- All their permanents are removed from the field.
- Their breeding area is cleared.
- They are **removed from the turn rotation**.
- The Titan's opponent list is updated to exclude the eliminated player.

After elimination, the Titan alternates with the remaining team players:

**Before (3 team players):** Titan → P1 → Titan → P2 → Titan → P3 → ...
**After P2 eliminated:** Titan → P1 → Titan → P3 → Titan → P1 → ...
**After P3 also eliminated:** Titan → P1 → Titan → P1 → ... (effectively standard 2-player)

### Titan Elimination

When the Titan is eliminated (security depleted + direct attack, or deck-out):
- The game **ends immediately**. The team wins.
- It does not matter how many team players remain — any surviving team player's team is victorious.

### Elimination During Own Turn

If a player is eliminated during their own turn (e.g., by a security Digimon's effect or deck-out from a card effect):
- The turn immediately ends.
- Play passes to the next player in rotation.

### Simultaneous Elimination

If the Titan and a team player would be eliminated simultaneously (e.g., mutual Retaliation), the Titan is considered eliminated and the team wins.

### Deck-Out

- If the Titan's deck runs out during a draw, the Titan is eliminated (team wins).
- If a team player's deck runs out during a draw, that team player is eliminated and removed from rotation.
- If the last team player is eliminated by deck-out, the Titan wins.

---

## Card Effects in Multiplayer

### Team Player Perspective

From a team player's perspective, the game is functionally a 1-vs-1 against the Titan:

- **"Your opponent"** = the Titan (always unambiguous).
- **"Your opponent's Digimon"** = the Titan's Digimon.
- **"Your opponent's security"** = the Titan's security stack.
- **[Opponent's Turn]** timing = triggers on the Titan's turns. Since the Titan takes a turn between every team player's turn, this triggers frequently.
- **[Your Turn]** timing = triggers only on the owning team player's turn.

### Titan Perspective

The Titan has multiple opponents, so effect targeting must be resolved:

- **"Your opponent"** effects that **target** (e.g., "delete 1 of your opponent's Digimon") — the Titan chooses which team player's permanent to target from among all team players.
- **"Your opponent"** effects that **affect directly** (e.g., "your opponent trashes 1 security card") — the Titan chooses which team player is affected.
- **"All your opponent's Digimon"** or similar — applies to **all** team players' permanents/zones.
- **"Your opponent reveals"** / **"your opponent trashes from hand"** — the Titan chooses which team player is affected.
- **[Opponent's Turn]** timing = triggers on **each** team player's turn.
- **[Your Turn]** timing = triggers only on the Titan's turns.

### Option Card Color Requirement

Same as standard: Option cards can only be played when the player has a Digimon or Tamer of a matching color on the field. This rule is unchanged in Titan Mode.

---

## Open Design Questions

These are areas that may need refinement after playtesting:

1. **Titan starting hand size** — Currently set at 7. May need to be 6 or 8 depending on balance testing.

2. **Titan draw count** — Currently the Titan draws 1 card per draw phase (standard). If the Titan decks out too quickly or struggles with card advantage, drawing 2 per turn may be needed.

3. **Titan memory reset** — Currently memory resets to 3 when ≤ 0 at turn start (same as standard). Since the Titan takes more total turns with potentially lower memory each turn, a higher reset (e.g., 4) could be considered.

4. **Game length limits** — With 15 Titan security and alternating turns, games will be significantly longer than standard. The max turn limit for headless simulation should be higher (suggested: 400-600 steps).

5. **Titan battle area slots** — Currently 12 (same as standard). If the Titan consistently runs out of field space against 2-3 opponents, expanding to 15 slots may be needed.

6. **First-turn draw skip** — Currently only the Titan (first player) skips draw on their first turn. An alternative is all players skip draw in round 1 (matching EDH rules). Needs playtesting to determine which feels better.

7. **Security recovery** — Should the Titan have any way to recover security (e.g., a "recovery" mechanic unique to Titan, or just relying on card effects)? Currently: no special recovery, cards only.

8. **Team player count balance** — The 15 starting security is balanced around 3 team players. With 2 team players, the Titan may be too strong. Consider variable starting security: 15 for 3 opponents, 10 for 2 opponents.

---

## Implementation Notes

This mode will be implemented as a **parallel pipeline** alongside the standard 2-player engine — no modifications to existing standard engine code.

### New Files (planned)

```
digimon_gym/engine/titan/             # Titan Mode package
├── __init__.py
├── titan_player.py                   # TitanPlayer(Player) subclass
├── titan_game.py                     # TitanGame(Game) subclass
├── titan_constants.py                # Titan-specific constants (security, hand, etc.)
└── titan_turn_manager.py             # Alternating rotation logic

digimon_gym/engine/runners/
├── titan_headless_game.py            # Agent-vs-agents headless simulation
└── titan_interactive_game.py         # Mixed human/agent Titan play

tests/
└── test_titan_mode.py                # Titan Mode-specific tests
```

### Key Architecture Decisions

- **`TitanGame`** subclasses `Game` (may skip `super().__init__()` due to hardcoded 2P setup in base class, similar to EDH approach).
- **`TitanPlayer`** subclasses `Player` with:
  - `role: TitanRole` enum (`Titan` or `Team`)
  - `opponents: List[Player]` (Titan has 2-3, team players have 1)
  - `is_eliminated: bool` for team player removal
- **`TitanTurnManager`** handles the alternating rotation:
  - Maintains ordered list: `[Titan, P1, Titan, P2, Titan, P3]`
  - Tracks current index, advances on turn end
  - Handles removal of eliminated players from rotation
  - Provides `next_player()`, `seesaw_partner()` methods
- **`player.enemy`** backward-compat property: team players → Titan; Titan → current seesaw partner. This ensures existing card scripts work unmodified.

### Action Space

**Team players:** Use the **standard 2120 action space** unchanged, since they have exactly 1 opponent (the Titan). No modifications needed.

**Titan player:** Expanded action space to handle multiple opponents:

| Range | Action | Change from Standard |
|-------|--------|---------------------|
| 0-29 | Play from hand | Unchanged |
| 30-59 | Trash from hand | Unchanged |
| 60 | Hatch | Unchanged |
| 61 | Move from breeding | Unchanged |
| 62 | Pass | Unchanged |
| 63-92 | DNA Digivolve | Unchanged |
| 100-579 | Attack | **Expanded**: `100 + attacker * 40 + opponent_id * 13 + target` (12 attacker slots × 3 opponents × 13 targets per opponent) |
| 600-999 | Digivolve | Renumbered, same logic |
| 1000-1999 | Effect activation | Unchanged |
| 2000-2119 | Source selection (own) | Unchanged |
| 2120-2479 | Source selection (multi-opponent) | **New**: target opponent's permanents/security |

Estimated Titan action space: **~2480 actions**.

Action masking ensures only legal actions for existing opponents are unmasked (eliminated players' slots are permanently masked).

### Tensor Layout

**Team player tensor:** ~1000 floats (close to standard 981)
- Global data (10): turn, phase, memory (relative), titan_security_count, team_player_count, etc.
- My field (372): 12 slots × 31 floats (standard)
- Opponent field (372): 12 Titan slots × 31 floats (standard)
- Hand, trash, security, breeding sections: same as standard
- Minimal expansion for Titan-specific global data

**Titan player tensor:** ~1500 floats (expanded for multiple opponents)
- Global data (15): turn, phase, memory (relative), opponent_count, per-opponent security counts
- My field (372): 12 slots × 31 floats
- Opponent 1 field (372): 12 slots × 31 floats
- Opponent 2 field (372): 12 slots × 31 floats
- Opponent 3 field (372): 12 slots × 31 floats (zero-padded if only 2 opponents)
- Per-opponent hand/trash/security/breeding sections
- Zero-padding for missing opponents (2-player games pad opponent 3's section)

### Backward Compatibility

- Existing card scripts work unmodified via `player.enemy` property returning the appropriate opponent.
- Standard 2-player engine is untouched — Titan Mode is an entirely separate code path.
- Card effects that reference `player.enemy` from the Titan's perspective resolve to the current seesaw partner, which is contextually correct for most effects. Effects that target "all opponents" use the `opponents` list.

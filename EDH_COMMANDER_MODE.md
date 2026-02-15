# Digimon TCG — EDH / Commander Mode

A custom multiplayer format for the Digimon TCG, inspired by Magic: The Gathering's Commander (EDH) format.

## Format Overview

| Parameter | Standard | EDH / Commander |
|-----------|----------|-----------------|
| Players | 2 | 4 (free-for-all) |
| Main deck | 50 cards | 70 cards (singleton) |
| Egg deck | 0-5 cards | 0-5 cards |
| Commander | None | 1 Digimon (separate from deck) |
| Copies per card | Up to 4 | Exactly 1 |
| Starting security | 5 | 7 |
| Starting hand | 5 | 5 |
| Memory system | Shared seesaw (2P) | Clockwise seesaw (active ↔ next) |
| Win condition | Opponent's security depleted + direct attack | Last player standing |

---

## Deck Construction

- **70-card singleton main deck** — each card ID may appear at most once.
- **Commander** — one Digimon card designated as commander. The commander is **separate** from the 70-card deck (does not count toward the deck limit), similar to MTG Commander where the commander is the 100th card outside the 99-card deck.
- **Egg deck** — 0 to 5 Digi-Egg cards, same as standard. The singleton rule applies to the egg deck as well.
- The commander card ID must not appear in the main deck or egg deck.

---

## Commander Rules

### Command Zone
- The commander starts the game face-up in a public **command zone**, visible to all players.
- The command zone is a new zone specific to EDH mode.

### Playing the Commander
- The commander may be **played** from the command zone as if it were in the player's hand, paying its normal play cost **plus a commander tax**.
- The commander may be **digivolved onto** from the command zone — the player pays the digivolution cost plus the commander tax, placing the commander on top of a valid Digimon on the field.
- **Commander tax** starts at 0 and increases by 2 each time the commander returns to the command zone. This tax applies to both playing and digivolving.

### Commander Zone Return
Whenever the commander would move from the battlefield to **any** of the following zones, its owner may choose to return it to the command zone instead:
- **Trash** (deletion by battle or effect)
- **Hand** (bounce effects)
- **Deck** (shuffle in, return to top/bottom)
- **Security** (placed into security by effect)

Each time the commander returns to the command zone by this replacement effect, the commander tax increases by 2.

If the owner chooses not to return the commander to the command zone, it moves to the destination zone normally (and can be played from the command zone again only if it later returns there by some other means).

### Commander in Digivolution Stack
If the commander is part of a digivolution stack (i.e., it was digivolved onto or digivolved into), the zone-change replacement applies when the commander card itself would leave the field — for example, if the Digimon is deleted and sources go to trash, the commander card is returned to the command zone while other sources go to trash normally.

---

## Memory System — Clockwise Seesaw

The standard Digimon TCG memory gauge is a shared seesaw between two players. In EDH, this is adapted for 4 players:

### How It Works
- Players sit in a circle. Turn order proceeds clockwise: P1 → P2 → P3 → P4 → P1 → ...
- The memory seesaw exists between the **active player** and the **next player clockwise**.
- When P1 is the active player, the seesaw is between P1 and P2.
- When P2 is the active player, the seesaw is between P2 and P3.
- And so on.

### Memory Rules (same as standard, applied to the active pair)
- At the start of your turn, if memory is at 0 or less (on your side), it resets to 3.
- Playing cards and using effects costs memory, moving the gauge toward the next player's side.
- If memory crosses to the next player's side (goes below 0 for the active player), the active player's turn ends.
- **Pass action:** When the active player passes, memory is set to 3 on the next player's side, and the turn ends.
- The memory gauge ranges from -10 to +10.

### Turn Transition
When a turn ends and the next player's turn begins:
- The memory value is **negated** (flipped perspective) — what was "3 on your side" from the previous pair becomes the starting point for the new pair.
- The new seesaw is between the new active player and the player clockwise after them.

---

## Turn Structure

Same phase order as standard:

**Unsuspend Phase → Draw Phase → Breeding Phase → Main Phase → End Phase**

### Modifications for Multiplayer

**Unsuspend Phase:**
- The turn player unsuspends all their suspended Digimon and Tamers (Reboot permanents are skipped, as in standard).
- **Reboot** permanents unsuspend during **each opponent's** unsuspend phase — meaning a Digimon with Reboot effectively unsuspends on every other player's turn start, not just one opponent's.

**Draw Phase:**
- First round: each player skips their draw on their first turn (same as standard's "first player skips draw" rule, extended to all players in round 1).
- Deck-out causes **elimination**, not an immediate game win for an opponent.

**Breeding Phase:**
- Same as standard. Hatch, move, or do nothing.

**Main Phase:**
- Same actions available as standard: play cards, digivolve, use options, attack, activate effects, or pass.
- **Additional actions:** Play commander from zone, digivolve commander from zone.
- **Attack targeting:** The active player may attack **any** opponent — not just the "next clockwise" player. Attacks can target any opponent's Digimon or any opponent's security stack directly.

**End Phase:**
- Same as standard.

---

## Combat

### Attacking
- The active player may declare attacks against **any** opponent's Digimon or security stack.
- The defending player (the one being attacked) is the one who may declare blockers and use counter timing effects.
- Only the defending player's Digimon may block, not other opponents' Digimon.

### Blocking and Counter Timing
- Only the **defending player** (the player being attacked) may declare a blocker or use counter timing effects.
- Other opponents cannot intervene in combat between two players.

### Security Attacks
- Security attacks check the defending player's security stack, same as standard.
- Jamming, Piercing, Security Attack +/- all function as in standard, applied to the defending player.

### Direct Attack (Game-Ending)
- When a player with 0 security cards is successfully attacked by a Digimon that can perform 1 or more security checks, that player is **eliminated** (not an immediate game win).

---

## Player Elimination

- When a player is eliminated, they are removed from the game:
  - All their permanents are removed from the field.
  - Their breeding area is cleared.
  - They are skipped in turn order.
  - Other players' opponent lists are updated to exclude the eliminated player.

- **Last player standing wins** — when only one player remains, they win the game.

- If the active player is eliminated during their own turn (e.g., by a card effect or deck-out), the turn immediately ends and passes to the next active player clockwise.

- If multiple players are eliminated simultaneously (e.g., by a card effect), and only one remains, that player wins.

---

## Card Effects in Multiplayer

### "Your Opponent" Effects
Many card effects reference "your opponent" (singular). In EDH mode:
- Effects that **target** an opponent's Digimon/Tamer (e.g., "delete 1 of your opponent's Digimon") allow the player to choose which opponent's permanent to target from among all opponents.
- Effects that **affect your opponent** directly (e.g., "your opponent trashes 1 security card") target the clockwise-next opponent by default. Future refinement may allow targeting any opponent.

### "All Your Opponent's" Effects
Effects that reference "all your opponent's Digimon" or similar apply to **all opponents'** permanents/zones.

### [Opponent's Turn] Timing
Effects with [Opponent's Turn] timing trigger on **each** opponent's turn, not just one.

### [Your Turn] / [Start of Your Turn] Timing
These function the same as standard — they trigger only on the card owner's turn.

---

## Option Card Color Requirement

Same as standard: Option cards can only be played when the player has a Digimon or Tamer of a matching color on the field. This rule is unchanged in EDH.

---

## Open Design Questions

These are areas that may need further refinement after playtesting:

1. **Commander color identity** — Should the commander restrict which cards can go in the deck (like MTG's color identity rule)? Currently: no restriction.

2. **Commander damage** — Should there be a "commander damage" win condition (like MTG's 21 damage from a single commander)? Currently: no.

3. **Politics / alliances** — No formal alliance mechanics. Players negotiate freely but all game actions are individual.

4. **Egg deck singleton** — Should the egg deck also be singleton? Currently: yes, same rule as main deck.

5. **First turn draw** — Currently all players skip draw on round 1. Alternative: only P1 skips draw (closer to standard). Needs playtesting.

6. **Multiplayer-specific keywords** — No new keywords planned for EDH. All standard keywords function as described in the main rules.

7. **Game length** — With 7 security, 70 cards, and 4 players, games will be significantly longer than standard. The max turn limit for headless simulation should be higher (e.g., 600 steps vs 200).

---

## Implementation Notes

This mode will be implemented as a **parallel pipeline** alongside the standard 2-player engine — no modifications to existing standard engine code.

### New Files (planned)
```
digimon_gym/engine/edh/              # EDH package
├── __init__.py
├── edh_player.py                    # EDHPlayer(Player) subclass
├── edh_game.py                      # EDHGame(Game) subclass
├── edh_constants.py                 # Tensor/action space/selection constants
└── edh_base_runner.py               # Shared deck setup + validation

digimon_gym/engine/runners/
├── edh_headless_game.py             # Agent-only 4P simulation
└── edh_interactive_game.py          # Mixed human/agent play

digimon_gym/engine/data/
└── game_format.py                   # GameFormat dataclass

tests/
└── test_edh_mode.py                 # EDH-specific tests
```

### Key Architecture Decisions
- `EDHGame` subclasses `Game` (skipping `super().__init__()` due to hardcoded 2P setup)
- `EDHPlayer` subclasses `Player` with added `opponents` list, `commander_zone`, `is_eliminated`
- `player.enemy` backward-compat property returns `opponents[0]` for card script compatibility
- EDH action space: 2360 actions (expanded attack targeting for 3 opponents)
- EDH tensor: ~1876 floats (4 player perspectives)
- Existing card scripts work unmodified via backward-compat properties

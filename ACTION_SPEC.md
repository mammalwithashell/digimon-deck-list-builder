# Action Decoder Specification

The action space consists of **2120 discrete actions**. An action mask (`get_action_mask`) flags which actions are legal in the current game state. The decoder (`decode_action`) maps an action ID to the corresponding game operation.

## Action Ranges

| Range | Count | Action | Formula |
|-------|-------|--------|---------|
| 0-29 | 30 | Play card from hand | `hand_index` |
| 30-59 | 30 | Trash card from hand (effect-driven) | `30 + hand_index` |
| 60 | 1 | Hatch from egg deck | — |
| 61 | 1 | Move from breeding area to battle area | — |
| 62 | 1 | Pass / end turn / decline optional | — |
| 63-92 | 30 | Initiate DNA Digivolve | `63 + hand_index` |
| 93-99 | 7 | Unused | — |
| 100-399 | 300 | Attack | `100 + attacker * 15 + target` |
| 400-999 | 600 | Digivolve | `400 + hand * 15 + field` |
| 1000-1999 | 1000 | Activate effect | `1000 + source * 10 + effectIdx` |
| 2000-2119 | 120 | Source selection | `2000 + field * 10 + sourceIdx` |

## Detailed Action Descriptions

### Play Card (0-29)

Play hand card at `hand_index` onto the battle area. The card's play cost is deducted from memory.

**Valid when:** Main phase, card's play cost <= current memory.

### Trash From Hand (30-59)

Trash a hand card at `hand_index - 30`. Used by effect-driven discard/trash-as-cost mechanics.

**Valid when:** Effect requests hand card selection via `SelectHand` phase.

### Hatch (60)

Move the top card from the Digitama (egg) deck to the breeding area.

**Valid when:** Breeding phase, breeding area is empty, egg deck is non-empty.

### Move From Breeding (61)

Move the permanent in the breeding area to the battle area.

**Valid when:** Breeding phase, breeding area has a permanent with level >= 3.

### Pass / Decline (62)

Context-dependent:
- **Breeding phase:** Skip breeding, advance to Main phase
- **Main phase:** End turn, advance to opponent's turn
- **BlockTiming:** Decline to block an attack
- **CounterTiming:** Decline to blast digivolve
- **Selection phases (when `is_optional`):** Decline an optional effect

### DNA Digivolve (63-92)

Initiate DNA Digivolution using hand card at `hand_index = action - 63`. After this action, the game enters `SelectMaterial` phase where the player selects two field permanents as materials.

**Valid when:** Main phase, hand card has valid DNA digivolve targets on the field.

### Attack (100-399)

Attack with a battle area permanent. Formula: `100 + attacker_idx * 15 + target_idx`.

| Target Index | Target |
|-------------|--------|
| 0-11 | Opponent's battle area permanent at that index |
| 12 | Opponent's security stack (direct attack) |
| 13-14 | Unused |

The attacker is suspended upon attacking. If attacking security, the top security card is revealed and checked. If attacking a Digimon, DP comparison determines the outcome.

**Valid when:** Main phase, attacker is unsuspended Digimon. Digimon targets must be suspended.

**During BlockTiming:** Actions 100-111 select a blocker (`100 + blocker_idx`). Only unsuspended Digimon with `<Blocker>` are valid.

### Digivolve (400-999)

Digivolve a hand card onto a field permanent. Formula: `400 + hand_idx * 15 + field_idx`.

The digivolution cost (from the card's `evo_costs`) is deducted instead of the play cost. The hand card is placed on top of the field permanent's digivolution stack.

**Valid when:**
- **Main phase:** Hand card's evo requirements match the field permanent (correct level, color). Evo cost <= current memory.
- **CounterTiming:** Only for cards with `<Blast Digivolve>` (free cost). Uses the defender's perspective.

### Activate Effect (1000-1999)

Activate an effect from a field permanent. Formula: `1000 + permanent_idx * 10 + effect_idx`.

**Valid when:** An effect is activatable on the permanent (not yet used this turn if OPT, conditions met).

### Source Selection (2000-2119)

Select a specific card from a permanent's digivolution stack. Formula: `2000 + field_idx * 10 + source_idx`.

**Valid when:** `SelectSource` phase, for effects that target specific cards within a digivolution stack (e.g. de-digivolve, source stripping).

## Selection Phases

When an effect requires the player to choose a target, the game enters a selection phase. During selection, the action mask is built from `PendingSelection.valid_indices`, which uses conventions to identify what's being selected.

### Selection Conventions

| Index Range | Meaning |
|-------------|---------|
| 0-29 | Select hand card by index |
| 30-39 | Select from revealed cards |
| 40-49 | Select from own security stack |
| 50-59 | Select from opponent's security stack |
| 62 | Decline optional selection |
| 99 | Select own breeding area permanent |
| 100-111 | Select own battle area permanent |
| 112-123 | Select opponent's battle area permanent |
| 130-179 | Select trash card by index (up to 50) |
| 1000-1009 | Choose between effect branches |

### Selection Phases

| Phase | Enum Value | Description |
|-------|------------|-------------|
| SelectTarget | 5 | Select a permanent or player as effect target |
| SelectMaterial | 6 | Select DNA digivolve materials |
| SelectTrash | 9 | Select card from trash |
| SelectSource | 10 | Select card from digivolution stack |
| SelectHand | 11 | Select card from hand (effect-driven) |
| SelectReveal | 12 | Select from revealed cards |
| SelectEffectChoice | 13 | Choose between effect branches |
| SelectSecurity | 14 | Select from security stack |

### Optional Selections

When `PendingSelection.is_optional` is `True`, the player can decline with action 62. The effect is skipped and the game returns to the previous phase.

## Attack Flow

A full attack sequence involves multiple phases:

```
1. Main Phase: Player selects attack action (100-399)
   → Attacker is suspended
   → Game enters BlockTiming

2. BlockTiming: Opponent decides whether to block
   → Action 62: Decline → proceed to CounterTiming
   → Action 100-111: Select blocker → blocker suspends, attack redirected → CounterTiming

3. CounterTiming: Opponent decides whether to blast digivolve
   → Action 62: Decline → resolve battle
   → Action 400-999: Blast digivolve → resolve battle with new DP

4. Battle Resolution:
   → Attacking Digimon: DP comparison (lower DP is deleted, tie = both deleted)
   → Attacking Security: Top security card is checked
```

## Memory System

- Memory ranges from -10 to +10
- Positive memory = current turn player's favour
- Playing a card deducts its cost from memory
- If memory goes to 0 or below after an action, the turn ends and passes to the opponent
- The opponent starts their turn with |memory| in their favour

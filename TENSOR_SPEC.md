# Game State Tensor Specification

The game state is encoded as a **981-float tensor** representing the board from the perspective of a single player. All values are relative to the observing player ("me" vs "opponent").

## Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `TENSOR_SIZE` | 981 | Total floats in tensor |
| `SLOT_SIZE` | 31 | Floats per permanent slot |
| `FIELD_SLOTS` | 12 | Max permanents per battle area |
| `MAX_SOURCES` | 8 | Max digivolution sources per permanent |
| `MAX_HAND` | 20 | Max hand cards encoded |
| `MAX_TRASH` | 45 | Max trash cards encoded |
| `MAX_SECURITY` | 10 | Max security cards encoded |
| `MAX_REVEALED` | 10 | Max revealed cards encoded |

## Top-Level Layout

| Index Range | Size | Section |
|-------------|------|---------|
| `0-9` | 10 | Global data |
| `10-381` | 372 | My battle area (12 slots x 31) |
| `382-753` | 372 | Opponent battle area (12 slots x 31) |
| `754-773` | 20 | My hand (card IDs) |
| `774-793` | 20 | Opponent hand (card IDs) |
| `794-838` | 45 | My trash (card IDs) |
| `839-883` | 45 | Opponent trash (card IDs) |
| `884-893` | 10 | My security (card IDs) |
| `894-903` | 10 | Opponent security (card IDs) |
| `904-934` | 31 | My breeding area (1 slot x 31) |
| `935-965` | 31 | Opponent breeding area (1 slot x 31) |
| `966-975` | 10 | Revealed cards (card IDs) |
| `976-980` | 5 | Selection context |

## Global Data (indices 0-9)

| Index | Field | Description |
|-------|-------|-------------|
| 0 | Turn count | Current turn number |
| 1 | Phase | `GamePhase` enum value (see below) |
| 2 | Memory | Memory gauge relative to observer (positive = their favour) |
| 3-9 | Reserved | Always 0.0 |

### GamePhase Values

| Value | Phase | Description |
|-------|-------|-------------|
| 0 | Start | Turn start (automatic) |
| 1 | Draw | Draw phase (automatic) |
| 2 | Breeding | Breeding phase (player decision) |
| 3 | Main | Main phase (player decision) |
| 4 | End | Turn end (automatic) |
| 5 | SelectTarget | Effect target selection |
| 6 | SelectMaterial | DNA digivolve material selection |
| 7 | BlockTiming | Defender's block decision |
| 8 | CounterTiming | Defender's blast digivolve decision |
| 9 | SelectTrash | Trash card selection |
| 10 | SelectSource | Digivolution source selection |
| 11 | SelectHand | Hand card selection (effect-driven) |
| 12 | SelectReveal | Revealed card selection |
| 13 | SelectEffectChoice | Multi-branch effect choice |
| 14 | SelectSecurity | Security stack selection |

## Permanent Slot Layout (31 floats)

Each permanent on the field, in the breeding area, or in an opponent's zone is encoded with 31 floats. Slots are used for **My battle area** (12 slots), **Opponent battle area** (12 slots), **My breeding** (1 slot), and **Opponent breeding** (1 slot).

### Header (7 floats)

| Offset | Field | Description |
|--------|-------|-------------|
| +0 | Card ID | Internal registry ID of top card (0 = empty) |
| +1 | DP | Current DP including all active modifiers (base + inherited + temporary) |
| +2 | Suspended | 1.0 if suspended (tapped), 0.0 if active |
| +3 | OPT total | Count of all once-per-turn effects on this permanent (inherited + top card + linked) |
| +4 | OPT used | Count of OPT effects that have been activated this turn |
| +5 | Linked count | Number of option cards linked sideways (e.g. [TS] cards) |
| +6 | Source count | Number of cards in the digivolution stack |

### Source Entries (8 entries x 3 floats = 24 floats)

Starting at offset +7, each of the 8 source slots encodes 3 floats. Sources are ordered bottom-to-top (index 0 = base card, last = top card).

| Offset | Field | Description |
|--------|-------|-------------|
| +0 | Card ID | Internal registry ID of the source card (0 = empty) |
| +1 | OPT state | Once-per-turn availability for this source (see below) |
| +2 | DP contribution | DP modifier this source currently provides (see below) |

#### OPT State Values

| Value | Meaning |
|-------|---------|
| -1.0 | Source has **no** once-per-turn effects |
| 0.0 | All OPT effects **exhausted** this turn |
| 1.0 | All OPT effects **available** |
| 0.0-1.0 | Fraction available (e.g. 0.5 = 1 of 2 still usable) |

Only effects relevant to the source's position are considered:
- **Inherited sources** (under top card): only inherited effects count
- **Top card**: only non-inherited effects count

#### DP Contribution

The `dp_contribution` float shows the DP modifier this specific source card currently provides. This value reflects **turn-specific conditions** evaluated at read time:

- A `[Your Turn] +2000` inherited effect shows `2000.0` on your turn, `0.0` on the opponent's turn
- A flat `+2000` inherited effect (all turns) always shows `2000.0`
- A conditional `+2000 if X` shows `2000.0` when the condition is met, `0.0` otherwise
- A source with no DP-modifying effects shows `0.0`

This allows the agent to see exactly which cards in a digivolution stack are contributing DP and whether those contributions are turn-dependent.

## Card ID Zones

Hand, trash, security, and revealed card zones are encoded as flat lists of internal card registry IDs, padded with 0.0.

| Zone | Max Size | Encoding |
|------|----------|----------|
| Hand | 20 | Card registry IDs, 0-padded |
| Trash | 45 | Card registry IDs, 0-padded |
| Security | 10 | Card registry IDs, 0-padded |
| Revealed | 10 | Card registry IDs, 0-padded |

Card IDs are integers assigned by `CardRegistry`. ID 0 is the padding value (no card). IDs are assigned alphabetically for determinism.

## Selection Context (indices 976-980)

Active during selection phases (SelectTarget, SelectMaterial, SelectHand, SelectReveal, SelectEffectChoice, SelectSecurity).

| Index | Field | Description |
|-------|-------|-------------|
| 976 | Selection phase | `GamePhase` enum value if in a selection phase, else 0.0 |
| 977 | Valid count | Number of valid selection options |
| 978 | Selecting player | Player ID of the selecting player (1 or 2), 0 if none |
| 979-980 | Reserved | Always 0.0 |

## Perspective

The tensor is always built from one player's perspective:
- "My" zones appear first (field, hand, trash, security, breeding)
- "Opponent" zones appear second
- Memory is positive when in the observer's favour, negative otherwise
- `get_board_state_tensor(1)` builds P1's view, `get_board_state_tensor(2)` builds P2's view

## Example

A Digimon with BT14-003 (Rookie, 3000 DP) digivolved into BT14-010 (Champion, 6000 DP), where BT14-003 has an inherited `[Your Turn] +2000 DP` effect. On **your turn**, the slot encodes:

```
+0:  card_id(BT14-010)    # top card
+1:  8000.0               # 6000 base + 2000 inherited
+2:  0.0                  # not suspended
+3:  1.0                  # 1 OPT effect (the inherited one)
+4:  0.0                  # not yet used
+5:  0.0                  # no linked cards
+6:  2.0                  # 2 sources in stack
+7:  card_id(BT14-003)    # source[0] = base card
+8:  1.0                  # source[0] OPT available
+9:  2000.0               # source[0] contributing +2000 DP
+10: card_id(BT14-010)    # source[1] = top card
+11: -1.0                 # source[1] no OPT effects
+12: 0.0                  # source[1] no DP modifier
+13..+30: 0.0             # remaining source slots empty
```

On **opponent's turn**, the same slot would show:

```
+1:  6000.0               # inherited [Your Turn] effect inactive
+9:  0.0                  # source[0] not contributing DP right now
```

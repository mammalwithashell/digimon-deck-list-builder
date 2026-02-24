# Game State Tensor Specification

The game state is encoded as a `981`-float tensor from one player's perspective.

## Constants

| Constant | Value |
|---|---:|
| `TENSOR_SIZE` | 981 |
| `SLOT_SIZE` | 31 |
| `FIELD_SLOTS` | 12 |
| `MAX_SOURCES` | 8 |
| `MAX_HAND` | 20 |
| `MAX_TRASH` | 45 |
| `MAX_SECURITY` | 10 |
| `MAX_REVEALED` | 10 |

## Top-Level Layout

| Index Range | Size | Section |
|---|---:|---|
| `0-9` | 10 | Global data |
| `10-381` | 372 | My battle area (`12 x 31`) |
| `382-753` | 372 | Opponent battle area (`12 x 31`) |
| `754-773` | 20 | My hand IDs |
| `774-793` | 20 | Opponent hand IDs |
| `794-838` | 45 | My trash IDs |
| `839-883` | 45 | Opponent trash IDs |
| `884-893` | 10 | My security IDs |
| `894-903` | 10 | Opponent security IDs |
| `904-934` | 31 | My breeding area (`1 x 31`) |
| `935-965` | 31 | Opponent breeding area (`1 x 31`) |
| `966-975` | 10 | Revealed card IDs |
| `976-980` | 5 | Selection context |

## Global Data (`0-9`)

| Index | Field | Notes |
|---:|---|---|
| `0` | Turn count | Current turn number |
| `1` | Phase | `GamePhase` enum value |
| `2` | Memory | Relative to observer (`+` means observer-favored) |
| `3-9` | Reserved | `0.0` |

### GamePhase Values

| Value | Phase |
|---:|---|
| `0` | Start |
| `1` | Draw |
| `2` | Breeding |
| `3` | Main |
| `4` | End |
| `5` | SelectTarget |
| `6` | SelectMaterial |
| `7` | BlockTiming |
| `8` | CounterTiming |
| `9` | SelectTrash |
| `10` | SelectSource |
| `11` | SelectHand |
| `12` | SelectReveal |
| `13` | SelectEffectChoice |
| `14` | SelectSecurity |
| `15` | EndOfTurnAction |
| `16` | AllianceTiming |
| `17` | Mulligan |

## Permanent Slot Layout (`31` floats)

Each slot in battle area and breeding area uses this format.

### Header (`+0` to `+6`)

| Offset | Field | Notes |
|---:|---|---|
| `+0` | Top card ID | Normalized ID (`norm_id`) |
| `+1` | DP | Current DP with modifiers |
| `+2` | Suspended | `1.0` suspended, `0.0` active |
| `+3` | OPT total | Count of OPT effects on permanent |
| `+4` | OPT used | OPT effects used this turn |
| `+5` | Linked count | Linked side cards count |
| `+6` | Source count | Digivolution stack size |

### Source Entries (`8 x 3` = `24` floats)

Start at `+7`, bottom-to-top ordering.

| Per-source Offset | Field | Notes |
|---:|---|---|
| `+0` | Source card ID | `norm_id` |
| `+1` | OPT state | `-1` none, `0..1` availability |
| `+2` | DP contribution | Active DP modifier from this source |

## Card ID Encoding

Card IDs are encoded as normalized registry IDs:

- `norm_id = index / 20000`
- `0.0` means empty/padding
- Registry is append-only to keep IDs stable for trained agents

## Selection Context (`976-980`)

These fields are populated only during selection phases:
`SelectTarget`, `SelectMaterial`, `SelectHand`, `SelectReveal`, `SelectEffectChoice`, `SelectSecurity`, `SelectTrash`, `SelectSource`.

Interrupt phases like `BlockTiming`, `CounterTiming`, `EndOfTurnAction`, and `AllianceTiming` do not use pending-selection context.

| Index | Field | Notes |
|---:|---|---|
| `976` | Selection phase | Active selection phase value, else `0.0` |
| `977` | Valid count | Number of legal selection options |
| `978` | Selecting player | `1` or `2`, else `0` |
| `979-980` | Reserved | `0.0` |

## Perspective Rules

- "My" zones always appear before opponent zones
- Memory sign is observer-relative
- `get_board_state_tensor(1)` and `get_board_state_tensor(2)` produce mirrored perspectives

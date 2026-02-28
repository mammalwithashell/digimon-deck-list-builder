# Action Decoder Specification

The engine exposes `2120` discrete action IDs. Legal actions are provided by `get_action_mask(player_id)` and executed by `decode_action(action_id, player_id)`.

## Global Action Ranges

| Range | Count | Meaning | Formula |
|---|---:|---|---|
| `0-29` | 30 | Play hand card | `hand_idx` |
| `30-59` | 30 | Trash hand card | `30 + hand_idx` |
| `60` | 1 | Hatch |
| `61` | 1 | Move from breeding |
| `62` | 1 | Pass / decline |
| `63-92` | 30 | Initiate DNA Digivolve | `63 + hand_idx` |
| `93-99` | 7 | Unused |
| `100-399` | 300 | Attack-like selections | `100 + slot * 15 + target` |
| `400-999` | 600 | Digivolve | `400 + hand_idx * 15 + field_idx` |
| `1000-1999` | 1000 | Effect activation | `1000 + source_idx * 10 + effect_idx` |
| `2000-2119` | 120 | Source selection | `2000 + field_idx * 10 + source_idx` |

## Phase-Aware Meaning

Action IDs are intentionally reused across phases.

### Mulligan

- `0`: keep opening hand
- `1`: mulligan opening hand (redraw 5)

### Main

- `0-29`: play card from hand
- `62`: pass turn
- `63-92`: initiate DNA digivolve
- `100-399`: attack (`target=12` means security)
- `400-999`: digivolve (`field_idx=12` means breeding-area digivolve)
- `1000-1999`: effect activations currently used for training/delay style actions

### Breeding

- `60`: hatch
- `61`: move from breeding
- `62`: pass breeding
- `1000-1999`: breeding-side training activation (virtual slot mapping)

### BlockTiming

- `100-111`: choose blocker (`100 + blocker_slot`)
- `62`: decline block (unless rules force block, for example Collision state)

### CounterTiming

- `400-999`: blast digivolve candidates
- `62`: decline counter

### EndOfTurnAction (`GamePhase=15`)

- `62`: decline / finish end-of-turn action window
- `100-...`: Vortex end-of-turn attack selections
- `1000-...`: Overclock-style effect activations

### AllianceTiming (`GamePhase=16`)

- `100-111`: choose ally to suspend for Alliance bonus
- `62`: decline Alliance

### Selection Phases

- Generic selection phases (`SelectTarget`, `SelectMaterial`, `SelectHand`, `SelectReveal`, `SelectEffectChoice`, `SelectSecurity`) use `pending_selection.valid_indices`.
- `SelectTrash`: uses `130-179` (`130 + trash_idx`)
- `SelectSource`: uses `2000-2119` (`2000 + field_idx * 10 + source_idx`)
- Optional selections allow decline with `62`.

## Selection Conventions

| Index Range | Selection Meaning |
|---|---|
| `0-29` | Hand card index |
| `30-39` | Revealed-card index |
| `40-49` | Own security index |
| `50-59` | Opponent security index |
| `62` | Decline optional selection |
| `99` | Own breeding permanent |
| `100-111` | Own battle-area permanent |
| `112-123` | Opponent battle-area permanent |
| `130-179` | Trash-card index |
| `1000-1009` | Effect branch choice |

## Attack Formula

`action_id = 100 + attacker_slot * 15 + target`

Target mapping:

- `0-11`: opponent battle-area slots
- `12`: opponent security
- `13-14`: reserved/unused

## Digivolve Formula

`action_id = 400 + hand_idx * 15 + field_idx`

- `field_idx=0..11`: battle area
- `field_idx=12`: breeding area (when valid)

## Effect Formula

`action_id = 1000 + source_idx * 10 + effect_idx`

Current engine usage includes training, delay, and end-of-turn effect hooks (for example overclock flow), depending on phase and card state.

## Source Selection Formula

`action_id = 2000 + field_idx * 10 + source_idx`

Used in `SelectSource` when effects need a specific card from a digivolution stack.

## Masking Contract

- Mask size is always `2120`
- `1.0` means legal, `0.0` illegal
- Frontend and RL agents must select only masked-legal actions
- Backend decoder is phase-aware and resolves semantics from current `GamePhase`

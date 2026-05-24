# Action Decoder Specification

The engine exposes `2192` discrete action IDs. Legal actions are provided by `get_action_mask(player_id)` and executed by `decode_action(action_id, player_id)`.

## Global Action Ranges

| Range | Count | Meaning | Formula |
|---|---:|---|---|
| `0-29` | 30 | Play hand card | `hand_idx` |
| `30-59` | 30 | [Hand][Main] effect (Main) / Trash hand card (selection) | `30 + hand_idx` |
| `60` | 1 | Hatch |
| `61` | 1 | Move from breeding |
| `62` | 1 | Pass / decline |
| `63-92` | 30 | Initiate DNA Digivolve | `63 + hand_idx` |
| `93` | 1 | Concede game (always legal at agent decision points) |
| `94` | 1 | Play first (BO3 `SelectPlayOrder` only) |
| `95` | 1 | Play second (BO3 `SelectPlayOrder` only) |
| `96-99` | 4 | Unused |
| `100-399` | 300 | Attack / block / alliance (phase-dependent) | `100 + slot * 15 + target` |
| `400-999` | 600 | Digivolve | `400 + hand_idx * 15 + field_idx` |
| `1000-1999` | 1000 | Effect activation | `1000 + perm_idx * 10 + effect_idx` |
| `2000-2167` | 168 | Source selection (battle-area carrier) | `2000 + field_idx * 12 + source_idx` |
| `2168-2191` | 24 | Source selection (breeding-area carrier) | `2168 + carrier_owner * 12 + source_idx` |

## Key Constants

| Name | Value | Notes |
|---|---:|---|
| `FIELD_SLOTS` | 14 | Battle area slots per player |
| `MAX_SOURCES` | 11 | Max digivolution stack depth |
| `SECURITY_TARGET` | 14 | Attack target index for security (`= FIELD_SLOTS`) |
| `BREEDING_SLOT` | 14 | Virtual field index for breeding (`= FIELD_SLOTS`) |
| `TARGETS_PER_ATTACKER` | 15 | Stride for attack formula |
| `FIELDS_PER_HAND` | 15 | Stride for digivolve formula |
| `EFFECTS_PER_PERM` | 10 | Stride for effect formula |
| `SOURCES_PER_FIELD` | 12 | Stride for source selection formula |
| `BREEDING_SOURCE_CARRIERS` | 2 | Breeding carriers addressable for source selection (one stack per player) |
| `BREEDING_SOURCE_SELECT_START` | 2168 | First breeding-carrier source-selection action ID |

## Phase-Aware Meaning

Action IDs are intentionally reused across phases.

### Mulligan

- `0`: keep opening hand
- `1`: mulligan opening hand (redraw 5)

### Main

- `0-29`: play card from hand
- `30-59`: activate [Hand][Main] effect on hand card (`30 + hand_idx`)
- `62`: pass turn
- `63-92`: initiate DNA digivolve
- `100-399`: attack (`target=14` means security)
- `400-999`: digivolve (`field_idx=14` means breeding-area digivolve)
- `1000-1999`: effect activations currently used for training/delay style actions

### Breeding

- `60`: hatch
- `61`: move from breeding
- `62`: pass breeding
- `1000-1999`: breeding-side training activation (virtual slot 14)

### BlockTiming

- `100-113`: choose blocker (`100 + blocker_slot`)
- `62`: decline block (unless rules force block, for example Collision state)

### CounterTiming

- `400-999`: blast digivolve candidates
- `62`: decline counter

### EndOfTurnAction (`GamePhase=15`)

- `62`: decline / finish end-of-turn action window
- `100-...`: Vortex end-of-turn attack selections
- `1000-...`: Overclock-style effect activations

### AllianceTiming (`GamePhase=16`)

- `100-113`: choose ally to suspend for Alliance bonus
- `62`: decline Alliance

### SelectPlayOrder (BO3 match training only)

- `94`: play first in the next game
- `95`: play second in the next game

Entered between games of a best-of-three match by `Game::request_play_order_selection(loser_id)`. Only the loser of the previous game is the chooser; the mask reports `94`/`95` legal only for that player. The chosen `PlayOrder` is written to `Game::last_play_order_choice` for the Python `MatchEnv` wrapper to consume.

### Concede (always legal)

- `93`: concede the game (`Game::concede(player)`). Mask reports `93 = 1` whenever the player has any other legal action — i.e., at every agent decision point (mulligan, main, breeding, block / counter / alliance timing, end-of-turn-action, every selection variant including `SelectPlayOrder`). The engine decoder accepts `93` regardless of `pending_selection` state — concede during a pending selection clears the selection, drains the effect queue, emits a `GameEvent::Concede` event before the terminal `GameEvent::GameOver`, and reports `win_reason = "concede"`.

### Selection Phases

- Generic selection phases (`SelectTarget`, `SelectMaterial`, `SelectHand`, `SelectReveal`, `SelectEffectChoice`, `SelectSecurity`) use `pending_selection.valid_indices`.
- `SelectTrash`: uses `pending_selection.valid_indices` when available, otherwise falls back to `130-179` (`130 + trash_idx`). Optional selections allow decline with `62`.
- `SelectSource` / `SelectMaterial`: uses `pending_selection.valid_indices` when available, otherwise falls back to `2000-2167` (`2000 + field_idx * 12 + source_idx`) for a battle-area carrier, or `2168-2191` (`2168 + carrier_owner * 12 + source_idx`) for a breeding-area carrier (King Drasil). Optional selections allow decline with `62`.
- `SelectBudgeted`: uses field-target action IDs for opponent battle-area permanents and allows decline with `62` once the minimum pick count is satisfied.
- `SelectBreedingPermanent`: uses phase-scoped breeding selection IDs (`14` for player 0, `15` for player 1) for breeding-area permanent choices.
- Optional selections allow decline with `62`.

### Selection Primitive Reuse

Group 2 selection primitives reuse existing action ranges.

- Cross-permanent source selections (battle-area carriers) use `SOURCE_SELECT_START..SOURCE_SELECT_END`.
- Breeding-area carrier source selections use `BREEDING_SOURCE_SELECT_START..BREEDING_SOURCE_SELECT_END` (`2168..2192`), keyed by the carrier's owning player. This sub-range was appended in Task S1.3, raising `ACTION_SPACE_SIZE` from `2168` to `2192` — a deliberate action-space version bump (existing trained RL models must be retrained).
- Up-to-N source selections expose `PASS` only after the minimum pick count is satisfied.
- DP-budget permanent selections reuse field-target action IDs during `SelectBudgeted`.
- Breeding permanent selections use phase-scoped breeding selection IDs only while `SelectBreedingPermanent` is pending.

Any future expansion that requires more source slots, more breeding targets, or additional simultaneous selection surfaces must update this document, Rust constants, PyO3 constants, and RL environment constants in the same change.

## Selection Conventions

| Index Range | Selection Meaning |
|---|---|
| `0-29` | Hand card index |
| `30-39` | Revealed-card index |
| `40-49` | Own security index |
| `50-59` | Opponent security index |
| `62` | Decline optional selection |
| `14/15` | Own breeding permanent during `SelectBreedingPermanent` |
| `100-113` | Own battle-area permanent |
| `114-127` | Opponent battle-area permanent |
| `130-179` | Trash-card index |
| `1000-1009` | Effect branch choice |

Arts Digivolve uses `99` for the controller's breeding-area target during its optional selection prompt.

## Attack Formula

`action_id = 100 + attacker_slot * 15 + target`

Target mapping:

- `0-13`: opponent battle-area slots
- `14`: opponent security (`SECURITY_TARGET`)

## Digivolve Formula

`action_id = 400 + hand_idx * 15 + field_idx`

- `field_idx=0..13`: battle area
- `field_idx=14`: breeding area (`BREEDING_SLOT`)

## Effect Formula

`action_id = 1000 + perm_idx * 10 + effect_idx`

Current engine usage includes training, delay, and end-of-turn effect hooks (for example overclock flow), depending on phase and card state.

## Source Selection Formula

Battle-area carrier:

`action_id = 2000 + field_idx * 12 + source_idx`

Breeding-area carrier (King Drasil — Task S1.3):

`action_id = 2168 + carrier_owner * 12 + source_idx`

Used in `SelectSource` / `SelectMaterial` when effects need a specific card from a digivolution stack. A breeding-area carrier holds exactly one digivolving stack per player, so it is addressed by the carrier's owning player rather than a field index.

## Masking Contract

- Mask size is always `2192`
- `1.0` means legal, `0.0` illegal
- Frontend and RL agents must select only masked-legal actions
- Backend decoder is phase-aware and resolves semantics from current `GamePhase`

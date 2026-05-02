# Game State Tensor Specification

The game state is encoded as a flat `float32` tensor from one player's perspective.
Card identities are integer registry indices (float-cast). The `nn.Embedding` lookup
happens inside the `CardEmbeddingExtractor` on the GPU, not in the tensor writer.

## Constants

| Constant | Value | Notes |
|---|---:|---|
| `TENSOR_SIZE` | 1375 | Compact layout |
| `SLOT_SIZE` | 40 | `1 + 6 + MAX_SOURCES * 3` |
| `SOURCE_ENTRY_SIZE` | 3 | `card_id + opt_state + dp_contribution` |
| `FIELD_SLOTS` | 14 | Battle area slots per player |
| `MAX_SOURCES` | 11 | Max digivolution stack depth |
| `MAX_HAND` | 20 | |
| `MAX_TRASH` | 45 | |
| `MAX_SECURITY` | 10 | |
| `MAX_REVEALED` | 10 | |

## Tensor Profiles

The canonical board tensor profile is `standard_compact_v1`:

| Field | Value |
|---|---:|
| `id` | `standard_compact_v1` |
| `version` | 1 |
| `tensor_size` | 1375 |
| `field_slots` | 14 |
| `slot_size` | 40 |
| `max_sources` | 11 |
| `card_id_slot_count` | 520 |
| `scalar_slot_count` | 855 |

`standard_v1` and `compact_v1` are compatibility aliases for older code and design notes. New code and model metadata should write `standard_compact_v1`.

Canonical tensor profile definitions live under `code/digimon-engine/src/tensor_profiles/<game_mode>/<version>.rs`. The current profile is defined in `code/digimon-engine/src/tensor_profiles/standard/v1.rs`, which owns the Standard v1 tensor size, section ranges, slot shape, and derived card/scalar positions. `code/digimon-engine/src/tensor.rs` is the Standard v1 tensor writer and compatibility surface; it re-exports the current layout constants but does not own them.

`standard_compact_v1` owns its structured layout tables in the registry: top-level sections, slot header fields, source fields, and the source stride live together with the profile so the card-ID and scalar positions are easy to audit. These tables use named offsets defined with the profile-owned layout constants instead of magic numeric indices.

### `standard_compact_v1` Sections

| Section id | Start offset | Length | Kind |
|---|---:|---:|---|
| `global` | `OFF_GLOBAL` = 0 | `GLOBAL_SIZE` = 10 | `Scalars` |
| `my_battle` | `OFF_MY_BATTLE` = 10 | `BATTLE_SIZE` = 560 | `PermanentSlots` |
| `opponent_battle` | `OFF_OPP_BATTLE` = 570 | `BATTLE_SIZE` = 560 | `PermanentSlots` |
| `my_hand` | `OFF_MY_HAND` = 1130 | `HAND_SIZE` = 20 | `CardIds` |
| `opponent_hand` | `OFF_OPP_HAND` = 1150 | `HAND_SIZE` = 20 | `CardIds` |
| `my_trash` | `OFF_MY_TRASH` = 1170 | `TRASH_SIZE` = 45 | `CardIds` |
| `opponent_trash` | `OFF_OPP_TRASH` = 1215 | `TRASH_SIZE` = 45 | `CardIds` |
| `my_security` | `OFF_MY_SECURITY` = 1260 | `SECURITY_SIZE` = 10 | `CardIds` |
| `opponent_security` | `OFF_OPP_SECURITY` = 1270 | `SECURITY_SIZE` = 10 | `CardIds` |
| `my_breeding` | `OFF_MY_BREEDING` = 1280 | `BREEDING_SIZE` = 40 | `PermanentSlots` |
| `opponent_breeding` | `OFF_OPP_BREEDING` = 1320 | `BREEDING_SIZE` = 40 | `PermanentSlots` |
| `revealed` | `OFF_REVEALED` = 1360 | `REVEALED_SIZE` = 10 | `CardIds` |
| `selection` | `OFF_SELECTION` = 1370 | `SELECTION_SIZE` = 5 | `Scalars` |

### Permanent Slot Header Fields

| Field id | Offset | Kind |
|---|---:|---|
| `top_card_id` | `SLOT_TOP_CARD_OFFSET` = 0 | `CardId` |
| `dp` | `SLOT_DP_OFFSET` = 1 | `Scalar` |
| `suspended` | `SLOT_SUSPENDED_OFFSET` = 2 | `Scalar` |
| `opt_total` | `SLOT_OPT_TOTAL_OFFSET` = 3 | `Scalar` |
| `opt_used` | `SLOT_OPT_USED_OFFSET` = 4 | `Scalar` |
| `linked_count` | `SLOT_LINKED_COUNT_OFFSET` = 5 | `Scalar` |
| `source_count` | `SLOT_SOURCE_COUNT_OFFSET` = 6 | `Scalar` |

### Source Entry Fields

| Field id | Per-source offset | Kind |
|---|---:|---|
| `card_id` | `SOURCE_CARD_ID_OFFSET` = 0 | `CardId` |
| `opt_state` | `SOURCE_OPT_STATE_OFFSET` = 1 | `Scalar` |
| `dp_contribution` | `SOURCE_DP_CONTRIBUTION_OFFSET` = 2 | `Scalar` |

### Source Stride

| Field | Value |
|---|---:|
| `source_start` | `SLOT_SOURCE_START_OFFSET` = 7 |
| `source_entry_size` | `SOURCE_ENTRY_SIZE` = 3 |
| `max_sources` | `MAX_SOURCES` = 11 |
| `slot_header_size` | `SLOT_HEADER_SIZE` = 7 |
| `slot_size` | `SLOT_SIZE` = `SLOT_HEADER_SIZE + MAX_SOURCES * SOURCE_ENTRY_SIZE` = 40 |

Future tensor profiles must define their own profile id and version, and must include matching tests and documentation updates.

## Top-Level Layout

| Index Range | Size | Section |
|---|---:|---|
| `0-9` | 10 | Global data |
| `10-569` | 560 | My battle area (`14 × 40`) |
| `570-1129` | 560 | Opponent battle area (`14 × 40`) |
| `1130-1149` | 20 | My hand card IDs (`20 × 1`) |
| `1150-1169` | 20 | Opponent hand card IDs (`20 × 1`) |
| `1170-1214` | 45 | My trash card IDs (`45 × 1`) |
| `1215-1259` | 45 | Opponent trash card IDs (`45 × 1`) |
| `1260-1269` | 10 | My security card IDs (`10 × 1`, face-up only; face-down = `0.0`) |
| `1270-1279` | 10 | Opponent security card IDs (`10 × 1`, face-up only; face-down = `0.0`) |
| `1280-1319` | 40 | My breeding area (`1 × 40`) |
| `1320-1359` | 40 | Opponent breeding area (`1 × 40`) |
| `1360-1369` | 10 | Revealed card IDs (`10 × 1`) |
| `1370-1374` | 5 | Selection context |

## Global Data (`0-9`)

| Index | Field | Notes |
|---:|---|---|
| `0` | Turn count | `turn_count / 30.0` clamped to `1.0` |
| `1` | Phase | `GamePhase` enum value |
| `2` | Memory | `memory / 10.0`, relative to observer (`+` means observer-favored) |
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

## Permanent Slot Layout (`40` floats)

Each slot in battle area and breeding area uses this format.

### Header (`+0` to `+6`)

| Offset | Field | Notes |
|---:|---|---|
| `+0` | Top card ID | Integer registry index (0 = empty) |
| `+1` | DP | `dp / 30000.0` (normalized) |
| `+2` | Suspended | `1.0` suspended, `0.0` active |
| `+3` | OPT total | Count of OPT effects on permanent |
| `+4` | OPT used | OPT effects used this turn |
| `+5` | Linked count | Linked side cards count |
| `+6` | Source count | Digivolution stack size |

### Source Entries (`11 × 3` = `33` floats)

Start at `+7`, bottom-to-top ordering.

| Per-source Offset | Field | Notes |
|---:|---|---|
| `+0` | Source card ID | Integer registry index |
| `+1` | OPT state | `0` none/exhausted, `0..1` availability |
| `+2` | DP contribution | `dp_contribution / 30000.0` (normalized) |

## Card Identity Encoding

Card identities are encoded as integer registry indices (float-cast):

- `0` means empty/padding
- Production card indices come from stable explicit `index` values in `data/cards.json`
- Legacy/test card data without explicit indices uses a deterministic sorted fallback
- `CardRegistry` rejects duplicate explicit indices
- The `CardEmbeddingExtractor` contains a trainable `nn.Embedding(20000, 16)` that maps
  these integers to learned 16-float vectors on the GPU
- The embedding table is part of the model checkpoint — no external files at inference
- Warm-start: embedding weights can be initialized from autoencoder embeddings
  (`card_embeddings.npy`) for faster convergence

### Tensor Layout Metadata

Canonical tensor layout metadata lives in `code/digimon-engine/src/tensor_profiles/standard/v1.rs`,
is exposed to Python by `digimon_engine.get_tensor_profile()`, and is consumed through
`digimon_gym.tensor_profiles.get_tensor_profile()`.

The profile provides:

- `card_id_positions`: list of 520 tensor indices that hold card IDs
- `scalar_positions`: list of 855 tensor indices that hold scalar values
- Metadata used by `CardEmbeddingExtractor` to split observations for embedding lookup

`code/engine_py_legacy/engine/data/tensor_layout.py` remains a legacy fallback only.

## Selection Context (`1370-1374`)

These fields are populated only during selection phases:
`SelectTarget`, `SelectMaterial`, `SelectHand`, `SelectReveal`, `SelectEffectChoice`, `SelectSecurity`, `SelectTrash`, `SelectSource`.

Interrupt phases like `BlockTiming`, `CounterTiming`, `EndOfTurnAction`, and `AllianceTiming` do not use pending-selection context.

| Index | Field | Notes |
|---:|---|---|
| `1370` | Selection phase | Active selection phase value, else `0.0` |
| `1371` | Valid count | Number of legal selection options |
| `1372` | Selecting player | `1` or `2`, else `0` |
| `1373-1374` | Reserved | `0.0` |

## Perspective Rules

- "My" zones always appear before opponent zones
- Memory sign is observer-relative
- `get_board_state_tensor(1)` and `get_board_state_tensor(2)` produce mirrored perspectives

## Features Extractor

The `CardEmbeddingExtractor` processes the 1375-float tensor:

1. Splits into 520 card-ID positions and 855 scalar positions
2. Looks up card IDs in `nn.Embedding(20000, 16)` → 520 × 16 = 8320 floats
3. Concatenates with 855 scalars → 9175 floats
4. Projects through `Linear(9175, 512) + ReLU` → 512-dim feature vector
5. Features feed into the policy/value heads (MLP or LSTM)

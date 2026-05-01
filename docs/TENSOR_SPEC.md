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

The canonical board tensor profile is `standard_v1`:

| Field | Value |
|---|---:|
| `id` | `standard_v1` |
| `version` | 1 |
| `tensor_size` | 1375 |
| `field_slots` | 14 |
| `slot_size` | 40 |
| `max_sources` | 11 |
| `card_id_slot_count` | 520 |
| `scalar_slot_count` | 855 |

The profile registry lives in `code/digimon-engine/src/tensor_profile.rs`. A profile is metadata for describing and auditing the tensor layout; it does not change tensor writer values, legal action masks, or action IDs.

`standard_v1` owns its structured layout tables in the registry: top-level sections, slot header fields, source fields, and the source stride live together with the profile so the card-ID and scalar positions are easy to audit. These tables use named offsets imported from `tensor.rs` instead of magic numeric indices.

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
- Each card gets a stable integer index from `CardRegistry` (1-based, sorted alphabetically)
- The `CardEmbeddingExtractor` contains a trainable `nn.Embedding(20000, 16)` that maps
  these integers to learned 16-float vectors on the GPU
- The embedding table is part of the model checkpoint — no external files at inference
- Warm-start: embedding weights can be initialized from autoencoder embeddings
  (`card_embeddings.npy`) for faster convergence

### Tensor Layout Metadata

`code/engine_py_legacy/engine/data/tensor_layout.py` exports:

- `CARD_ID_POSITIONS`: list of 520 tensor indices that hold card IDs
- `SCALAR_POSITIONS`: list of 855 tensor indices that hold scalar values
- Used by `CardEmbeddingExtractor` to split observations for embedding lookup

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

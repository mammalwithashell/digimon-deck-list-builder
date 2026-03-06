# Game State Tensor Specification

The game state is encoded as a flat `float32` tensor from one player's perspective.
Card identities are represented as frozen autoencoder embeddings (`EMBEDDING_DIM` floats
per card) instead of single normalized IDs.

## Constants

| Constant | Value | Notes |
|---|---:|---|
| `TENSOR_SIZE` | 6891 | With `EMBEDDING_DIM=16` |
| `EMBEDDING_DIM` | 16 | Autoencoder bottleneck dimension |
| `SLOT_SIZE` | 166 | `EMBEDDING_DIM + 6 + MAX_SOURCES * SOURCE_ENTRY_SIZE` |
| `SOURCE_ENTRY_SIZE` | 18 | `EMBEDDING_DIM + 2` |
| `FIELD_SLOTS` | 12 | |
| `MAX_SOURCES` | 8 | |
| `MAX_HAND` | 20 | |
| `MAX_TRASH` | 45 | |
| `MAX_SECURITY` | 10 | |
| `MAX_REVEALED` | 10 | |

## Top-Level Layout

| Index Range | Size | Section |
|---|---:|---|
| `0-9` | 10 | Global data |
| `10-2001` | 1992 | My battle area (`12 × 166`) |
| `2002-3993` | 1992 | Opponent battle area (`12 × 166`) |
| `3994-4313` | 320 | My hand embeddings (`20 × 16`) |
| `4314-4633` | 320 | Opponent hand embeddings (`20 × 16`) |
| `4634-5353` | 720 | My trash embeddings (`45 × 16`) |
| `5354-6073` | 720 | Opponent trash embeddings (`45 × 16`) |
| `6074-6233` | 160 | My security embeddings (`10 × 16`) |
| `6234-6393` | 160 | Opponent security embeddings (`10 × 16`) |
| `6394-6559` | 166 | My breeding area (`1 × 166`) |
| `6560-6725` | 166 | Opponent breeding area (`1 × 166`) |
| `6726-6885` | 160 | Revealed card embeddings (`10 × 16`) |
| `6886-6890` | 5 | Selection context |

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

## Permanent Slot Layout (`166` floats)

Each slot in battle area and breeding area uses this format.

### Header (`+0` to `+21`)

| Offset | Field | Notes |
|---:|---|---|
| `+0..+15` | Top card embedding | `EMBEDDING_DIM` floats from autoencoder |
| `+16` | DP | Current DP with modifiers |
| `+17` | Suspended | `1.0` suspended, `0.0` active |
| `+18` | OPT total | Count of OPT effects on permanent |
| `+19` | OPT used | OPT effects used this turn |
| `+20` | Linked count | Linked side cards count |
| `+21` | Source count | Digivolution stack size |

### Source Entries (`8 × 18` = `144` floats)

Start at `+22`, bottom-to-top ordering.

| Per-source Offset | Field | Notes |
|---:|---|---|
| `+0..+15` | Source card embedding | `EMBEDDING_DIM` floats |
| `+16` | OPT state | `-1` none, `0..1` availability |
| `+17` | DP contribution | Active DP modifier from this source |

## Card Embedding Encoding

Card identities are encoded as frozen autoencoder embeddings:

- Each card ID is replaced by a `EMBEDDING_DIM`-float vector from a pretrained autoencoder
- The autoencoder is trained on structured card attributes (~111 floats: kind, colors, level, cost, DP, form, attribute, keywords, evo costs)
- Zero-vector means empty/padding
- Embeddings are stored in `digimon_gym/engine/data/card_embeddings.npy`
- Registry is append-only; re-run `tools/train_card_autoencoder.py` after adding cards

## Selection Context (`6886-6890`)

These fields are populated only during selection phases:
`SelectTarget`, `SelectMaterial`, `SelectHand`, `SelectReveal`, `SelectEffectChoice`, `SelectSecurity`, `SelectTrash`, `SelectSource`.

Interrupt phases like `BlockTiming`, `CounterTiming`, `EndOfTurnAction`, and `AllianceTiming` do not use pending-selection context.

| Index | Field | Notes |
|---:|---|---|
| `6886` | Selection phase | Active selection phase value, else `0.0` |
| `6887` | Valid count | Number of legal selection options |
| `6888` | Selecting player | `1` or `2`, else `0` |
| `6889-6890` | Reserved | `0.0` |

## Perspective Rules

- "My" zones always appear before opponent zones
- Memory sign is observer-relative
- `get_board_state_tensor(1)` and `get_board_state_tensor(2)` produce mirrored perspectives

## Features Extractor

The observation tensor is projected through a `CardEmbeddingExtractor` (single `Linear(6891, 512) + ReLU`) before reaching the policy/value heads. This prevents the LSTM from receiving a 6891-dim input directly.

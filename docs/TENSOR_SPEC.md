# Game State Tensor Specification

The game state is encoded as a flat `float32` tensor from one player's perspective.
Card identities are integer registry indices (float-cast). The `nn.Embedding` lookup
happens inside the `CardEmbeddingExtractor` on the GPU, not in the tensor writer.

## Constants

| Constant | Value | Notes |
|---|---:|---|
| `TENSOR_SIZE` | 8850 | Engine default — `standard_lite_deck_v2`. Top-level Rust `tensor::TENSOR_SIZE` and PyO3 `digimon_engine.TENSOR_SIZE` both report this value. |
| `ACTION_SPACE_SIZE` | 2192 | Unchanged by the default-profile flip. |

`standard_compact_v1`-specific layout constants (`SLOT_SIZE = 40`, `FIELD_SLOTS = 14`, `MAX_HAND = 20`, etc.) are now reachable only via their module path: `digimon_engine::tensor_profiles::standard::v1::*` in Rust and `get_tensor_profile("standard_compact_v1")` in Python. They are no longer re-exported from the top-level `tensor` module — code that imported them from there must migrate either to the explicit module path (if it intentionally targets v1) or to `TensorSection` / `TensorSlotLayout` profile metadata (if it wants the active default).

## Tensor Profiles

The engine default pilot observation profile is **`standard_lite_deck_v2`** (`8850` floats; flipped 2026-05-25 per the `flip-engine-default-to-lite-deck-v2` change). It is selected by:

- Rust: `tensor_profiles::standard::DEFAULT_PROFILE`, `tensor_profiles::default_profile()`, `observation::default_observation_profile()` — all return the same value.
- Python: `digimon_engine.DEFAULT_OBSERVATION_PROFILE` and `digimon_engine.TENSOR_PROFILE_ID` both report `"standard_lite_deck_v2"`.

`standard_lite_v2` (`8410` floats) is the lite_deck_v2 prefix without the own-original-decklist section. Callers that don't need decklist-aware observations can request it explicitly via `observation_profile="standard_lite_v2"` on `RustHeadlessGame`, `--tensor-profile standard_lite_v2` on `pilot_training`, or `profile_by_id("standard_lite_v2")` from Rust.

`standard_compact_v1` (`1375` floats) is the historical baseline profile. It remains reachable for legacy callers (parity tests, archived recordings, baseline models) via `observation_profile="standard_compact_v1"` and `profile_by_id("standard_compact_v1")`. Existing ONNX checkpoints trained against this profile are NOT loadable through the engine's default surface — they must be loaded with an explicit profile pin and an action-space-compatible inference path.

`standard_full_v2` (`43482` floats) is the maximally informative profile used for full-state agents; unchanged by the default flip.

### `standard_lite_v2`

> **Task S1.4:** the v2 `PERM_MAX_SOURCES` grew `11 → 12` so the tensor
> surfaces every digivolution-source slot the action space can select. The
> `SOURCE_SELECT` and `BREEDING_SOURCE_SELECT` action ranges use
> `SOURCES_PER_FIELD = 12`; previously only 11 source slots were encoded, so
> source index 11 — including a breeding-carrier's 12th source reachable via
> `BREEDING_SOURCE_SELECT` — was selectable but invisible to the agent. Each
> permanent slot grew 3 floats (`slot_size` `96 → 99`), `permanent_slots`
> `2880 → 2970`, `tensor_size` `8320 → 8410`, every section after
> `permanent_slots` shifted `+90`, and `feature_schema_version` bumped to
> `standard_lite_v2.2`. `standard_compact_v1` (`MAX_SOURCES` stays 11) is
> unchanged.

| Field | Value |
|---|---:|
| `id` | `standard_lite_v2` |
| `version` | 2 |
| `tensor_version` | 2 |
| `feature_schema_version` | `standard_lite_v2.2` |
| `tensor_size` | 8410 |
| `field_slots` | 15 |
| `slot_size` | 99 |
| `max_sources` | 12 |
| `card_id_slot_count` | 572 |
| `scalar_slot_count` | 7838 |

Top-level sections:

| Section id | Start offset | Shape | Size |
|---|---:|---:|---:|
| `global_features` | 0 | `[64]` | 64 |
| `player_summary` | 64 | `[2][32]` | 64 |
| `permanent_slots` | 128 | `[2][15][99]` | 2970 |
| `own_hand` | 3098 | `[30][32]` | 960 |
| `known_zone_cards` | 4058 | `[120][8]` | 960 |
| `decision_context` | 5018 | `[64]` | 64 |
| `pending_choice_features` | 5082 | `[32][96]` | 3072 |
| `reserved` | 8154 | `[256]` | 256 |

`standard_lite_v2` card ID positions total `572`; scalar positions total `7838`. These lists, the section table, layout hash, tensor version, and feature schema version are exported by `digimon_engine.get_observation_layout("standard_lite_v2")`.

### `standard_lite_deck_v2`

`standard_lite_deck_v2` is an opt-in fair-information profile for experiments where the pilot should know the composition of the deck it is playing. It reuses all `standard_lite_v2` sections and appends `own_original_decklist[55][8]` before `reserved`. It does not change `ACTION_SPACE_SIZE`, action masks, action decoding, or the default pilot observation profile.

The decklist section encodes the observer's original submitted deck composition only:

- Rows represent unique card IDs, not physical copy rows.
- Rows are sorted by stable card-registry index, not submitted order or shuffled order.
- Opponent decklist composition is not encoded.
- Current shuffled deck order, topdeck identity, and face-down security identity are not encoded.
- Counts are original submitted counts; this profile does not encode remaining-count inference.

| Field | Value |
|---|---:|
| `id` | `standard_lite_deck_v2` |
| `version` | 2 |
| `tensor_version` | 2 |
| `feature_schema_version` | `standard_lite_deck_v2.1` |
| `tensor_size` | 8850 |
| `field_slots` | 15 |
| `slot_size` | 99 |
| `max_sources` | 12 |
| `card_id_slot_count` | 627 |
| `scalar_slot_count` | 8223 |

Top-level sections:

| Section id | Start offset | Shape | Size |
|---|---:|---:|---:|
| `global_features` | 0 | `[64]` | 64 |
| `player_summary` | 64 | `[2][32]` | 64 |
| `permanent_slots` | 128 | `[2][15][99]` | 2970 |
| `own_hand` | 3098 | `[30][32]` | 960 |
| `known_zone_cards` | 4058 | `[120][8]` | 960 |
| `decision_context` | 5018 | `[64]` | 64 |
| `pending_choice_features` | 5082 | `[32][96]` | 3072 |
| `own_original_decklist` | 8154 | `[55][8]` | 440 |
| `reserved` | 8594 | `[256]` | 256 |

`own_original_decklist[row]` fields:

| Offset | Field |
|---:|---|
| 0 | Present flag |
| 1 | Card ID registry index |
| 2 | Original copy count divided by `4.0` |
| 3 | Main-deck flag |
| 4 | Digi-Egg flag |
| 5-7 | Reserved, currently `0.0` |

`standard_lite_deck_v2` card ID positions total `627`; scalar positions total `8223`. The 55 decklist card ID offsets are included in `card_id_positions` so `CardEmbeddingExtractor` embeds them like other card identities.

### `standard_full_v2`

`standard_full_v2` is an opt-in experimental profile. `standard_lite_v2`
remains the default pilot observation profile. Full v2 extends
`standard_lite_v2` with `action_id_features[2192][16]`.

> **Task S1.3:** `ACTION_SPACE_SIZE` grew `2168 → 2192` (appended
> breeding-carrier source-selection sub-range). `action_id_features` has
> one row per action ID, so it gained 24 rows (`24 * 16 = 384` floats):
> `tensor_size` `43008 → 43392`, `reserved` shifted `42752 → 43136`, and
> `feature_schema_version` bumped to `standard_full_v2.2`. The
> `ACTION_SPACE_SIZE`-independent `standard_compact_v1` and `standard_lite_v2`
> profiles are unchanged in size (their action mask, delivered separately,
> grows `2168 → 2192` automatically).

> **Task S1.4:** `standard_full_v2` re-exports the `standard_lite_v2`
> layout constants, so the v2 `PERM_MAX_SOURCES` `11 → 12` bump cascades:
> `permanent_slots` grew `2880 → 2970`, every section after it shifted
> `+90` (`action_id_features` `8064 → 8154`, `reserved` `43136 → 43226`),
> `tensor_size` grew `43392 → 43482`, and `feature_schema_version` bumped
> to `standard_full_v2.3`.

| Field | Value |
|---|---:|
| `id` | `standard_full_v2` |
| `version` | 2 |
| `tensor_version` | 2 |
| `feature_schema_version` | `standard_full_v2.3` |
| `tensor_size` | 43482 |
| `card_id_slot_count` | 572 |
| `scalar_slot_count` | 42910 |

Top-level sections:

| Section id | Start offset | Shape | Size |
|---|---:|---:|---:|
| `global_features` | 0 | `[64]` | 64 |
| `player_summary` | 64 | `[2][32]` | 64 |
| `permanent_slots` | 128 | `[2][15][99]` | 2970 |
| `own_hand` | 3098 | `[30][32]` | 960 |
| `known_zone_cards` | 4058 | `[120][8]` | 960 |
| `decision_context` | 5018 | `[64]` | 64 |
| `pending_choice_features` | 5082 | `[32][96]` | 3072 |
| `action_id_features` | 8154 | `[2192][16]` | 35072 |
| `reserved` | 43226 | `[256]` | 256 |

`action_id_features[action_id]` fields:

| Offset | Field |
|---:|---|
| 0 | legal flag, equal to `get_action_mask(player)[action_id]` |
| 1 | raw action ID normalized by `ACTION_SPACE_SIZE` |
| 2 | action family bucket |
| 3 | phase bucket |
| 4 | source zone bucket |
| 5 | source index bucket |
| 6 | target zone bucket |
| 7 | target index bucket |
| 8 | source permanent slot bucket |
| 9 | target permanent slot bucket |
| 10 | reserved cost/memory bucket, currently `0.0` |
| 11 | reserved amount/count bucket, currently `0.0` |
| 12 | uses hand card flag |
| 13 | uses permanent flag |
| 14 | prompt/selection action flag |
| 15 | reserved, currently `0.0` |

### `standard_compact_v1`

`standard_compact_v1` is the compact compatibility and baseline profile:

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

`standard_v1` and `compact_v1` are compatibility aliases for older code and design notes. New compact-profile code and model metadata should write `standard_compact_v1`.

Canonical tensor profile definitions live under `code/digimon-engine/src/tensor_profiles/<game_mode>/<version>.rs`. `standard_lite_v2` is defined in `code/digimon-engine/src/tensor_profiles/standard/v2_lite.rs`; `standard_lite_deck_v2` is defined in `code/digimon-engine/src/tensor_profiles/standard/v2_lite_deck.rs`; `standard_full_v2` is defined in `code/digimon-engine/src/tensor_profiles/standard/v2_full.rs`; `standard_compact_v1` is defined in `code/digimon-engine/src/tensor_profiles/standard/v1.rs`. `code/digimon-engine/src/tensor.rs` is the Standard compact v1 tensor writer and compatibility surface; it re-exports compact layout constants but does not define the default pilot observation profile.

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

## `standard_compact_v1` Top-Level Layout

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

Canonical tensor layout metadata lives in `code/digimon-engine/src/tensor_profiles/standard/`.
Observation layouts are exposed to Python by `digimon_engine.get_observation_layout(profile_id)`
and consumed through `digimon_gym.tensor_profiles.get_tensor_profile(profile_id)`.
The compact registry metadata remains available through `digimon_engine.get_tensor_profile()`
for `standard_compact_v1` compatibility.

The profile provides:

- `card_id_positions`: tensor indices that hold card IDs
- `scalar_positions`: tensor indices that hold scalar values
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

The `CardEmbeddingExtractor` is layout-driven. Training code passes the active observation layout, or the extractor resolves the profile from the observation shape:

1. Reads `card_id_positions`, `scalar_positions`, `tensor_size`, and layout metadata from the active profile.
2. Verifies the observation shape matches the profile tensor size and that card/scalar positions cover the tensor exactly once.
3. Looks up card IDs in `nn.Embedding(20000, 16)`.
4. Concatenates embedded card IDs with scalar positions.
5. Projects through `Linear(..., 512) + ReLU` to produce the 512-dim feature vector used by MLP or LSTM policy/value heads.

For `standard_lite_v2`, this means `572` card-ID positions embedded to `572 × 16 = 9152` floats, concatenated with `7838` scalar positions before projection. For `standard_lite_deck_v2`, this means `627` card-ID positions embedded to `627 × 16 = 10032` floats, concatenated with `8223` scalar positions before projection. For `standard_compact_v1`, the compatibility path remains `520` card-ID positions and `855` scalar positions.

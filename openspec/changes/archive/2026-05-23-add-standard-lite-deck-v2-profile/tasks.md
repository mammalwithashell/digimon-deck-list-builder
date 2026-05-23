## 1. Game State Source Data

- [x] 1.1 Add immutable original-deck composition storage to Rust game/player setup, captured before shuffling, drawing, security setup, or mulligan changes.
- [x] 1.2 Store enough metadata to distinguish main-deck and Digi-Egg rows for each original card ID.
- [x] 1.3 Add unit coverage proving original-deck composition is stable across shuffle seeds and mulligan redraws.

## 2. Rust Profile Layout

- [x] 2.1 Add `standard_lite_deck_v2` profile constants derived from `standard_lite_v2`.
- [x] 2.2 Add an own-original-decklist section sized for 55 unique rows with 8 floats per row.
- [x] 2.3 Include decklist row card ID offsets in `card_id_positions` and all other row offsets in `scalar_positions`.
- [x] 2.4 Register the profile in Rust profile lookup/listing APIs and observation profile parsing.
- [x] 2.5 Add layout tests for tensor size, section offsets, shape, card/scalar counts, and layout hash recomputation.

## 3. Tensor Writer

- [x] 3.1 Implement `standard_lite_deck_v2` tensor building by reusing `standard_lite_v2` sections and writing the appended decklist section.
- [x] 3.2 Sort populated decklist rows by stable card registry index.
- [x] 3.3 Encode present flag, card ID, normalized original count, main-deck flag, and Digi-Egg flag for each row.
- [x] 3.4 Add behavioral tensor tests proving repeated copies collapse into one counted row.
- [x] 3.5 Add behavioral tensor tests proving opponent decklist, shuffled order, topdeck identity, and face-down security identity are not encoded.

## 4. Bindings And Python Consumers

- [x] 4.1 Expose `standard_lite_deck_v2` through PyO3 observation layout exports and profile listing.
- [x] 4.2 Update Python tensor profile tests for the new profile's metadata and position coverage.
- [x] 4.3 Verify `CardEmbeddingExtractor` accepts the new profile and embeds decklist card ID positions.
- [x] 4.4 Update RL environment smoke tests to allow selecting `tensor_profile="standard_lite_deck_v2"` without changing action-mask length.

## 5. Documentation And Compatibility

- [x] 5.1 Update `docs/TENSOR_SPEC.md` with the new profile, section shape, row fields, tensor size, card ID count, and scalar count.
- [x] 5.2 Update model metadata/export tests so `standard_lite_deck_v2` artifacts carry the distinct observation profile, tensor size, schema version, and layout hash.
- [x] 5.3 Confirm existing `standard_lite_v2` metadata and tests remain unchanged.
- [x] 5.4 Document that `standard_lite_deck_v2` is opt-in and does not change `ACTION_SPACE_SIZE`.

## 6. Verification

- [x] 6.1 Run Rust tensor/profile tests for the new profile.
- [x] 6.2 Run Rust headless runner observation-profile tests.
- [x] 6.3 Run Python RL tensor profile tests.
- [x] 6.4 Run a Rust-backend `DigimonEnv` smoke check selecting `standard_lite_deck_v2`.

## ADDED Requirements

### Requirement: standard_lite_deck_v2 is the engine default profile

The `standard_lite_deck_v2` profile SHALL be the value returned by `tensor_profiles::default_profile()` and the profile reflected by the top-level constants `tensor::TENSOR_SIZE`, `tensor::build_tensor`, and `EngineContract::current()`. The profile's own contract — sections, layout, action-space preservation, hidden-information rules — remains unchanged.

#### Scenario: Profile reports itself as the engine default

- **WHEN** a caller compares `tensor_profiles::profile_by_id("standard_lite_deck_v2")` with `tensor_profiles::default_profile()`
- **THEN** the two `TensorProfile` values are byte-equal

#### Scenario: Default builder produces a standard_lite_deck_v2 tensor

- **WHEN** `tensor::build_tensor(game, player_id, registry)` is called
- **THEN** the returned tensor's length equals `profile_by_id("standard_lite_deck_v2").unwrap().tensor_size`
- **AND** the populated tensor contains a non-empty own-original-decklist section for the observing player when that player's submitted decklist is non-empty

#### Scenario: Desktop EngineContract advertises the deck-aware profile

- **WHEN** the desktop build evaluates `EngineContract::current()`
- **THEN** the reported `tensor_size` equals `profile_by_id("standard_lite_deck_v2").unwrap().tensor_size`

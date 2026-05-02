# Standard Full V2 Tensor Profile Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the experimental `standard_full_v2` observation profile by extending `standard_lite_v2` with `action_id_features[2168][16]`.

**Architecture:** Keep `standard_lite_v2` as the default pilot profile and build `standard_full_v2` as an opt-in profile. The full writer reuses the lite tensor sections exactly through offset `8064`, appends a scalar-only action table, shifts the reserved tail to `42752`, and exposes profile-owned layout metadata through the same Rust, PyO3, Gym, extractor, and artifact metadata path.

**Tech Stack:** Rust `digimon-engine`, PyO3 `digimon-engine-py`, Python `digimon_gym`, Gymnasium, SB3 feature extraction, pytest, Rust integration tests.

---

## Current State

`standard_lite_v2` is already implemented and default:

- Profile module: `code/digimon-engine/src/tensor_profiles/standard/v2_lite.rs`
- Tensor writer: `code/digimon-engine/src/tensor_v2_lite.rs`
- Profile dispatch: `code/digimon-engine/src/observation.rs`
- Python layout consumption: `code/digimon_gym/tensor_profiles.py`
- Default shape: `8320`

The spec leaves `standard_full_v2` as a future experiment:

```text
standard_lite_v2 sections through offset 8064
action_id_features[2168][16] at offset 8064
reserved[256] at offset 42752
TENSOR_SIZE = 43008
```

Card ID positions remain identical to lite: `542`. The action table contains no card IDs, so scalar positions increase to `42466`.

## File Structure

- Create `code/digimon-engine/src/tensor_profiles/standard/v2_full.rs`: full profile constants, section table, and layout metadata.
- Create `code/digimon-engine/src/tensor_v2_full.rs`: tensor writer that copies lite sections and writes `action_id_features`.
- Modify `code/digimon-engine/src/tensor_profiles/standard/mod.rs`: expose `v2_full`.
- Modify `code/digimon-engine/src/tensor_profiles/mod.rs`: register `standard_full_v2` and include it in profile listing.
- Modify `code/digimon-engine/src/observation.rs`: parse, list, layout, and dispatch `standard_full_v2`.
- Modify `code/digimon-engine/src/lib.rs`: export `tensor_v2_full`.
- Test `code/digimon-engine/tests/mask_and_tensor/tensor_profile_full_v2.rs`: profile shape and position coverage.
- Test `code/digimon-engine/tests/mask_and_tensor/tensor_v2_full.rs`: tensor writer and action-table semantics.
- Modify `code/digimon-engine/tests/mask_and_tensor/main.rs`: include new test modules.
- Modify docs: `docs/TENSOR_SPEC.md`, `docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md`, and `docs/superpowers/specs/2026-05-01-observation-profile-registry-design.md`.

---

### Task 1: Add Full V2 Profile Layout

**Files:**
- Create: `code/digimon-engine/src/tensor_profiles/standard/v2_full.rs`
- Modify: `code/digimon-engine/src/tensor_profiles/standard/mod.rs`
- Modify: `code/digimon-engine/src/tensor_profiles/mod.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/tensor_profile_full_v2.rs`
- Modify: `code/digimon-engine/tests/mask_and_tensor/main.rs`

- [ ] **Step 1: Write failing full-profile layout tests**

Create `code/digimon-engine/tests/mask_and_tensor/tensor_profile_full_v2.rs`:

```rust
use digimon_engine::tensor_profiles::{all_profile_ids, profile_by_id, TensorSectionKind};

#[test]
fn standard_full_v2_layout_matches_spec() {
    let profile = profile_by_id("standard_full_v2").unwrap();

    assert_eq!(profile.id, "standard_full_v2");
    assert_eq!(profile.game_mode, "standard");
    assert_eq!(profile.version, 2);
    assert_eq!(profile.tensor_version, 2);
    assert_eq!(profile.feature_schema_version, "standard_full_v2.1");
    assert_eq!(profile.tensor_size, 43008);
    assert_eq!(profile.card_id_slot_count, 542);
    assert_eq!(profile.scalar_slot_count, 42466);

    assert_eq!(profile.section("global_features").unwrap().start, 0);
    assert_eq!(profile.section("player_summary").unwrap().start, 64);
    assert_eq!(profile.section("permanent_slots").unwrap().start, 128);
    assert_eq!(profile.section("own_hand").unwrap().start, 3008);
    assert_eq!(profile.section("known_zone_cards").unwrap().start, 3968);
    assert_eq!(profile.section("decision_context").unwrap().start, 4928);
    assert_eq!(profile.section("pending_choice_features").unwrap().start, 4992);
    assert_eq!(profile.section("action_id_features").unwrap().start, 8064);
    assert_eq!(profile.section("reserved").unwrap().start, 42752);

    assert_eq!(profile.section("action_id_features").unwrap().shape, &[2168, 16]);
    assert_eq!(profile.section("action_id_features").unwrap().kind, TensorSectionKind::Scalars);
}

#[test]
fn standard_full_v2_positions_cover_tensor_once() {
    let profile = profile_by_id("standard_full_v2").unwrap();
    let (cards, scalars) = profile.positions();

    assert_eq!(cards.len(), 542);
    assert_eq!(scalars.len(), 42466);
    assert_eq!(cards.len() + scalars.len(), profile.tensor_size);

    let card_set: std::collections::BTreeSet<_> = cards.iter().copied().collect();
    let scalar_set: std::collections::BTreeSet<_> = scalars.iter().copied().collect();
    assert!(card_set.is_disjoint(&scalar_set));

    let all: std::collections::BTreeSet<_> = card_set.union(&scalar_set).copied().collect();
    let expected: std::collections::BTreeSet<_> = (0..profile.tensor_size).collect();
    assert_eq!(all, expected);
}

#[test]
fn profile_list_includes_full_v2() {
    let profiles = all_profile_ids();
    assert!(profiles.contains(&"standard_compact_v1"));
    assert!(profiles.contains(&"standard_lite_v2"));
    assert!(profiles.contains(&"standard_full_v2"));
}
```

In `code/digimon-engine/tests/mask_and_tensor/main.rs`, add:

```rust
mod tensor_profile_full_v2;
```

- [ ] **Step 2: Run the failing layout tests**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile_full_v2 -- --nocapture
```

Expected: FAIL because `standard_full_v2` is not registered.

- [ ] **Step 3: Add the full profile module**

Create `code/digimon-engine/src/tensor_profiles/standard/v2_full.rs`:

```rust
use crate::tensor_profiles::standard::v2_lite;
use crate::tensor_profiles::{
    TensorProfile, TensorSection, TensorSectionKind, TensorSlotLayout,
};

pub const PROFILE_ID: &str = "standard_full_v2";
pub const GAME_MODE: &str = "standard";
pub const VERSION: u32 = 2;
pub const TENSOR_VERSION: u16 = 2;
pub const FEATURE_SCHEMA_VERSION: &str = "standard_full_v2.1";
pub const LAYOUT_HASH: &str = "sha256:REPLACE_WITH_COMPUTED_HASH";

pub const ACTION_ID_ROWS: usize = crate::action::space::ACTION_SPACE_SIZE;
pub const ACTION_ID_ROW_SIZE: usize = 16;
pub const ACTION_ID_FEATURES_SIZE: usize = ACTION_ID_ROWS * ACTION_ID_ROW_SIZE;
pub const RESERVED_SIZE: usize = v2_lite::RESERVED_SIZE;

pub const OFF_GLOBAL_FEATURES: usize = v2_lite::OFF_GLOBAL_FEATURES;
pub const OFF_PLAYER_SUMMARY: usize = v2_lite::OFF_PLAYER_SUMMARY;
pub const OFF_PERMANENT_SLOTS: usize = v2_lite::OFF_PERMANENT_SLOTS;
pub const OFF_OWN_HAND: usize = v2_lite::OFF_OWN_HAND;
pub const OFF_KNOWN_ZONE_CARDS: usize = v2_lite::OFF_KNOWN_ZONE_CARDS;
pub const OFF_DECISION_CONTEXT: usize = v2_lite::OFF_DECISION_CONTEXT;
pub const OFF_PENDING_CHOICE_FEATURES: usize = v2_lite::OFF_PENDING_CHOICE_FEATURES;
pub const OFF_ACTION_ID_FEATURES: usize = v2_lite::OFF_RESERVED;
pub const OFF_RESERVED: usize = OFF_ACTION_ID_FEATURES + ACTION_ID_FEATURES_SIZE;
pub const TENSOR_SIZE: usize = OFF_RESERVED + RESERVED_SIZE;

pub const SHAPE_ACTION_ID_FEATURES: &[usize] = &[ACTION_ID_ROWS, ACTION_ID_ROW_SIZE];
pub const SHAPE_RESERVED: &[usize] = &[RESERVED_SIZE];

pub const SECTIONS: &[TensorSection] = &[
    TensorSection {
        id: "global_features",
        start: OFF_GLOBAL_FEATURES,
        len: v2_lite::GLOBAL_FEATURES_SIZE,
        shape: v2_lite::SHAPE_GLOBAL_FEATURES,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "player_summary",
        start: OFF_PLAYER_SUMMARY,
        len: v2_lite::PLAYER_SUMMARY_SIZE,
        shape: v2_lite::SHAPE_PLAYER_SUMMARY,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "permanent_slots",
        start: OFF_PERMANENT_SLOTS,
        len: v2_lite::PERMANENT_SLOTS_SIZE,
        shape: v2_lite::SHAPE_PERMANENT_SLOTS,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "own_hand",
        start: OFF_OWN_HAND,
        len: v2_lite::OWN_HAND_SIZE,
        shape: v2_lite::SHAPE_OWN_HAND,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "known_zone_cards",
        start: OFF_KNOWN_ZONE_CARDS,
        len: v2_lite::KNOWN_ZONE_SIZE,
        shape: v2_lite::SHAPE_KNOWN_ZONE_CARDS,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "decision_context",
        start: OFF_DECISION_CONTEXT,
        len: v2_lite::DECISION_CONTEXT_SIZE,
        shape: v2_lite::SHAPE_DECISION_CONTEXT,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "pending_choice_features",
        start: OFF_PENDING_CHOICE_FEATURES,
        len: v2_lite::PENDING_CHOICE_SIZE,
        shape: v2_lite::SHAPE_PENDING_CHOICE_FEATURES,
        kind: TensorSectionKind::StandardLiteV2Rows,
    },
    TensorSection {
        id: "action_id_features",
        start: OFF_ACTION_ID_FEATURES,
        len: ACTION_ID_FEATURES_SIZE,
        shape: SHAPE_ACTION_ID_FEATURES,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "reserved",
        start: OFF_RESERVED,
        len: RESERVED_SIZE,
        shape: SHAPE_RESERVED,
        kind: TensorSectionKind::Scalars,
    },
];

pub const SLOT_LAYOUT: TensorSlotLayout = v2_lite::SLOT_LAYOUT;
pub const CARD_ID_SLOT_COUNT: usize = v2_lite::CARD_ID_SLOT_COUNT;
pub const SCALAR_SLOT_COUNT: usize = TENSOR_SIZE - CARD_ID_SLOT_COUNT;

pub const PROFILE: TensorProfile = TensorProfile {
    id: PROFILE_ID,
    game_mode: GAME_MODE,
    version: VERSION,
    tensor_version: TENSOR_VERSION,
    feature_schema_version: FEATURE_SCHEMA_VERSION,
    layout_hash: LAYOUT_HASH,
    tensor_size: TENSOR_SIZE,
    field_slots: v2_lite::PERMANENT_SLOTS_PER_PLAYER,
    slot_size: v2_lite::PERMANENT_SLOT_SIZE,
    max_sources: v2_lite::PERM_MAX_SOURCES,
    slot_layout: SLOT_LAYOUT,
    card_id_slot_count: CARD_ID_SLOT_COUNT,
    scalar_slot_count: SCALAR_SLOT_COUNT,
    sections: SECTIONS,
};
```

After it compiles with a temporary valid hash, print and replace the real hash using:

```rust
#[test]
fn print_full_v2_hash() {
    let profile = digimon_engine::tensor_profiles::profile_by_id("standard_full_v2").unwrap();
    println!(
        "{}",
        profile.layout_hash_with_schema_version_for_test(profile.feature_schema_version)
    );
}
```

Remove the temporary test before committing.

- [ ] **Step 4: Register the profile**

In `code/digimon-engine/src/tensor_profiles/standard/mod.rs`:

```rust
pub mod v2_full;
```

In `code/digimon-engine/src/tensor_profiles/mod.rs`, add:

```rust
pub const STANDARD_FULL_V2_PROFILE_ID: &str = standard::v2_full::PROFILE_ID;
```

Update `all_profile_ids()`:

```rust
pub fn all_profile_ids() -> Vec<&'static str> {
    vec![
        standard::v1::PROFILE_ID,
        standard::v2_lite::PROFILE_ID,
        standard::v2_full::PROFILE_ID,
    ]
}
```

Update `profile_by_id()`:

```rust
standard::v2_full::PROFILE_ID => Some(standard::v2_full::PROFILE),
```

- [ ] **Step 5: Run layout tests**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile_full_v2 -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-engine/src/tensor_profiles/standard/v2_full.rs code/digimon-engine/src/tensor_profiles/standard/mod.rs code/digimon-engine/src/tensor_profiles/mod.rs code/digimon-engine/tests/mask_and_tensor/main.rs code/digimon-engine/tests/mask_and_tensor/tensor_profile_full_v2.rs
git commit -m "feat: add standard full v2 tensor profile"
```

---

### Task 2: Implement Full V2 Tensor Writer

**Files:**
- Create: `code/digimon-engine/src/tensor_v2_full.rs`
- Modify: `code/digimon-engine/src/lib.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/tensor_v2_full.rs`
- Modify: `code/digimon-engine/tests/mask_and_tensor/main.rs`

- [ ] **Step 1: Write failing full-writer tests**

Create `code/digimon-engine/tests/mask_and_tensor/tensor_v2_full.rs`:

```rust
use digimon_engine::action::space::{ACTION_SPACE_SIZE, PASS};
use digimon_engine::observation::{build_observation_tensor, parse_observation_profile};
use digimon_engine::tensor_profiles::standard::{v2_full, v2_lite};

use crate::tensor_helpers::sample_game_with_known_cards;

#[test]
fn v2_full_tensor_has_expected_size_and_keeps_lite_prefix() {
    let (game, registry) = sample_game_with_known_cards();
    let lite = build_observation_tensor(
        &game,
        0,
        &registry,
        parse_observation_profile("standard_lite_v2").unwrap(),
    );
    let full = build_observation_tensor(
        &game,
        0,
        &registry,
        parse_observation_profile("standard_full_v2").unwrap(),
    );

    assert_eq!(full.len(), v2_full::TENSOR_SIZE);
    assert_eq!(&full[..v2_lite::OFF_RESERVED], &lite[..v2_lite::OFF_RESERVED]);
}

#[test]
fn v2_full_action_legal_bits_match_mask() {
    let (game, registry) = sample_game_with_known_cards();
    let mask = digimon_engine::action::build_action_mask(&game, 0);
    let tensor = build_observation_tensor(
        &game,
        0,
        &registry,
        parse_observation_profile("standard_full_v2").unwrap(),
    );

    for action_id in 0..ACTION_SPACE_SIZE {
        let base = v2_full::OFF_ACTION_ID_FEATURES + action_id * v2_full::ACTION_ID_ROW_SIZE;
        assert_eq!(tensor[base], mask[action_id], "action {action_id}");
        assert_eq!(
            tensor[base + 1],
            action_id as f32 / ACTION_SPACE_SIZE as f32,
            "action {action_id} normalized id"
        );
    }
}

#[test]
fn v2_full_marks_pass_as_prompt_action_during_optional_selection() {
    let (mut game, registry) = sample_game_with_known_cards();
    let source_card = game.players[0].battle_area[0].top_card().handle();
    game.current_phase = digimon_engine::enums::GamePhase::EffectChoice;
    game.pending_selection = Some(digimon_engine::selection::PendingSelection {
        kind: digimon_engine::selection::SelectionKind::EffectChoice,
        selecting_player: 0,
        previous_phase: digimon_engine::enums::GamePhase::Main,
        valid_action_ids: vec![PASS],
        is_optional: true,
        prompt: "decline optional effect".to_string(),
        effect_choices: None,
        source_card,
        source_permanent: None,
        source_kind: digimon_engine::enums::EffectSourceKind::Digimon,
        callback: Box::new(|_, _| {}),
        on_decline: Some(Box::new(|_| {})),
    });

    let tensor = build_observation_tensor(
        &game,
        0,
        &registry,
        parse_observation_profile("standard_full_v2").unwrap(),
    );
    let base = v2_full::OFF_ACTION_ID_FEATURES + PASS as usize * v2_full::ACTION_ID_ROW_SIZE;

    assert_eq!(tensor[base], 1.0);
    assert_eq!(tensor[base + 14], 1.0);
}
```

In `code/digimon-engine/tests/mask_and_tensor/main.rs`, add:

```rust
mod tensor_v2_full;
```

- [ ] **Step 2: Run failing writer tests**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_v2_full -- --nocapture
```

Expected: FAIL because the writer and observation dispatch are missing.

- [ ] **Step 3: Implement writer by extending lite**

Create `code/digimon-engine/src/tensor_v2_full.rs`:

```rust
use crate::action::explain::{explain_action, ActionKind, ActionZone};
use crate::action::space::ACTION_SPACE_SIZE;
use crate::card_registry::CardRegistry;
use crate::enums::{GamePhase, PlayerId};
use crate::game::Game;
use crate::tensor_profiles::standard::{v2_full as full, v2_lite as lite};

pub fn build_tensor_standard_full_v2(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
) -> Vec<f32> {
    let lite_tensor = crate::tensor_v2_lite::build_tensor_standard_lite_v2(
        game,
        player_id,
        registry,
    );
    let mut tensor = vec![0.0f32; full::TENSOR_SIZE];
    tensor[..lite::OFF_RESERVED].copy_from_slice(&lite_tensor[..lite::OFF_RESERVED]);
    write_action_id_features(&mut tensor, game, player_id);
    tensor
}

fn write_action_id_features(t: &mut [f32], game: &Game, player_id: PlayerId) {
    let mask = crate::action::build_action_mask(game, player_id);
    let prompt_actions = game
        .pending_selection
        .as_ref()
        .filter(|sel| sel.selecting_player == player_id)
        .map(|sel| sel.valid_action_ids.as_slice());

    for action_id in 0..ACTION_SPACE_SIZE {
        let action = action_id as u16;
        let base = full::OFF_ACTION_ID_FEATURES + action_id * full::ACTION_ID_ROW_SIZE;
        let explanation = explain_action(game, player_id, action);

        t[base] = mask[action_id];
        t[base + 1] = action_id as f32 / ACTION_SPACE_SIZE as f32;
        t[base + 2] = action_kind_bucket(explanation.kind);
        t[base + 3] = phase_bucket(game.current_phase);
        t[base + 4] = zone_bucket(explanation.source_zone);
        t[base + 5] = index_bucket(explanation.source_index);
        t[base + 6] = zone_bucket(explanation.target_zone);
        t[base + 7] = index_bucket(explanation.target_index);
        t[base + 8] = permanent_slot_bucket(explanation.source_zone, explanation.source_index);
        t[base + 9] = permanent_slot_bucket(explanation.target_zone, explanation.target_index);
        t[base + 12] = if explanation.source_zone == Some(ActionZone::Hand) { 1.0 } else { 0.0 };
        t[base + 13] = if matches!(
            explanation.source_zone,
            Some(ActionZone::Battle | ActionZone::Breeding)
        ) { 1.0 } else { 0.0 };
        t[base + 14] = if prompt_actions
            .map(|actions| actions.contains(&action))
            .unwrap_or(false)
        { 1.0 } else { 0.0 };
    }
}

fn action_kind_bucket(kind: ActionKind) -> f32 {
    match kind {
        ActionKind::Play => 1.0,
        ActionKind::HandEffect => 2.0,
        ActionKind::Hatch => 3.0,
        ActionKind::Move => 4.0,
        ActionKind::Pass => 5.0,
        ActionKind::DnaDigivolve => 6.0,
        ActionKind::Attack => 7.0,
        ActionKind::Digivolve => 8.0,
        ActionKind::FieldEffect => 9.0,
        ActionKind::TrashEffect => 10.0,
        ActionKind::SourceSelect => 11.0,
        ActionKind::Selection => 12.0,
        ActionKind::Unknown => 0.0,
    }
}

fn phase_bucket(phase: GamePhase) -> f32 {
    phase.tensor_value() as f32 / 24.0
}

fn zone_bucket(zone: Option<ActionZone>) -> f32 {
    match zone {
        Some(ActionZone::Hand) => 1.0,
        Some(ActionZone::Battle) => 2.0,
        Some(ActionZone::Breeding) => 3.0,
        Some(ActionZone::Security) => 4.0,
        Some(ActionZone::Trash) => 5.0,
        Some(ActionZone::Source) => 6.0,
        Some(ActionZone::Revealed) => 7.0,
        Some(ActionZone::EffectChoice) => 8.0,
        None => 0.0,
    }
}

fn index_bucket(index: Option<u16>) -> f32 {
    index.map(|idx| idx as f32 / 30.0).unwrap_or(0.0)
}

fn permanent_slot_bucket(zone: Option<ActionZone>, index: Option<u16>) -> f32 {
    match zone {
        Some(ActionZone::Battle) => index.map(|idx| idx as f32 / 14.0).unwrap_or(0.0),
        Some(ActionZone::Breeding) => 1.0,
        _ => 0.0,
    }
}
```

This first pass deliberately leaves offsets `10`, `11`, and `15` zero. They are reserved by the spec for cost/magnitude/reserved values and should only be assigned when the engine can produce structured values cheaply.

- [ ] **Step 4: Export the module**

In `code/digimon-engine/src/lib.rs`, add:

```rust
pub mod tensor_v2_full;
```

- [ ] **Step 5: Run writer tests**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_v2_full -- --nocapture
```

Expected: still FAIL until observation dispatch is added in Task 3.

---

### Task 3: Wire Observation Dispatch And PyO3 Layout

**Files:**
- Modify: `code/digimon-engine/src/observation.rs`
- Modify: `code/digimon-engine-py/src/lib.rs`
- Test: `code/tests/rl/test_tensor_profiles.py`

- [ ] **Step 1: Add failing Python profile tests**

Append to `code/tests/rl/test_tensor_profiles.py`:

```python
def test_get_standard_full_v2_tensor_profile_from_rust():
    from digimon_gym.tensor_profiles import get_tensor_profile

    profile = get_tensor_profile("standard_full_v2")

    assert profile.id == "standard_full_v2"
    assert profile.tensor_size == 43008
    assert profile.tensor_version == 2
    assert profile.feature_schema_version == "standard_full_v2.1"
    assert profile.card_id_slot_count == 542
    assert profile.scalar_slot_count == 42466
    sections = {section.name: section for section in profile.sections}
    assert sections["action_id_features"].offset == 8064


def test_digimon_env_accepts_standard_full_v2(monkeypatch):
    pytest.importorskip("digimon_engine")
    monkeypatch.setenv("DIGIMON_BACKEND", "rust")

    from digimon_gym.digimon_gym import DigimonEnv

    env = DigimonEnv(deck1=DECK, deck2=DECK, tensor_profile="standard_full_v2")
    obs, info = env.reset(seed=7)

    assert env.tensor_profile == "standard_full_v2"
    assert env.observation_space.shape == (43008,)
    assert obs.shape == (43008,)
    assert info["tensor_profile"] == "standard_full_v2"
```

- [ ] **Step 2: Extend Rust observation enum and parser**

In `code/digimon-engine/src/observation.rs`, add:

```rust
StandardFullV2,
```

Update `as_str()`:

```rust
Self::StandardFullV2 => STANDARD_FULL_V2_PROFILE_ID,
```

Update `parse_observation_profile()`:

```rust
STANDARD_FULL_V2_PROFILE_ID => Ok(ObservationProfileId::StandardFullV2),
```

Update `build_observation_tensor()`:

```rust
ObservationProfileId::StandardFullV2 => {
    crate::tensor_v2_full::build_tensor_standard_full_v2(game, player_id, registry)
}
```

Keep `default_observation_profile()` returning `StandardLiteV2`.

- [ ] **Step 3: Import the full profile ID**

At the top of `code/digimon-engine/src/observation.rs`, include:

```rust
STANDARD_FULL_V2_PROFILE_ID,
```

from `crate::tensor_profiles`.

- [ ] **Step 4: Run Rust dispatch tests**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile_full_v2 tensor_v2_full -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Rebuild PyO3 and run Python tests**

Run:

```powershell
Push-Location code/digimon-engine-py
maturin develop
Pop-Location
$env:DIGIMON_BACKEND='rust'
python -m pytest code/tests/rl/test_tensor_profiles.py -v
```

Expected: PASS. `get_observation_layout("standard_full_v2")` is handled by the existing PyO3 layout export once the Rust registry and parser know the ID.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-engine/src/observation.rs code/digimon-engine/src/lib.rs code/digimon-engine/src/tensor_v2_full.rs code/digimon-engine-py/src/lib.rs code/tests/rl/test_tensor_profiles.py code/digimon-engine/tests/mask_and_tensor/main.rs code/digimon-engine/tests/mask_and_tensor/tensor_v2_full.rs
git commit -m "feat: build standard full v2 observations"
```

---

### Task 4: Validate Feature Extraction And Training Metadata

**Files:**
- Test: `code/tests/rl/test_tensor_profiles.py`
- Test: `code/tests/rl/test_pilot_training_config.py`
- Test: `code/tests/rl/test_onnx_export_profiles.py`

- [ ] **Step 1: Add a feature extractor smoke test**

Append to `code/tests/rl/test_tensor_profiles.py`:

```python
def test_feature_extractor_accepts_standard_full_v2():
    import torch
    from gymnasium import spaces
    from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
    from digimon_gym.tensor_profiles import get_tensor_profile

    profile = get_tensor_profile("standard_full_v2")
    space = spaces.Box(
        low=-10.0,
        high=20001.0,
        shape=(profile.tensor_size,),
        dtype=np.float32,
    )

    extractor = CardEmbeddingExtractor(space, observation_layout=profile)
    out = extractor(torch.zeros((2, profile.tensor_size), dtype=torch.float32))

    assert extractor.card_id_indices.numel() == 542
    assert extractor.scalar_indices.numel() == 42466
    assert tuple(out.shape) == (2, 512)
```

- [ ] **Step 2: Verify training and export profile plumbing already accepts full**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profiles.py::test_feature_extractor_accepts_standard_full_v2 code/tests/rl/test_pilot_training_config.py code/tests/rl/test_onnx_export_profiles.py -v
```

Expected: PASS. No production training-code edits should be needed because profile selection is already layout-driven.

- [ ] **Step 3: Run an explicit env smoke check**

Run:

```powershell
$env:DIGIMON_BACKEND='rust'
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(tensor_profile='standard_full_v2'); obs,info=env.reset(seed=1); print(obs.shape, info['tensor_profile'], info['action_mask'].shape)"
```

Expected:

```text
(43008,) standard_full_v2 (2168,)
```

- [ ] **Step 4: Commit tests**

```powershell
git add code/tests/rl/test_tensor_profiles.py
git commit -m "test: cover standard full v2 python profile plumbing"
```

---

### Task 5: Docs And Performance Guardrails

**Files:**
- Modify: `docs/TENSOR_SPEC.md`
- Modify: `docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md`
- Modify: `docs/superpowers/specs/2026-05-01-observation-profile-registry-design.md`
- Modify: `docs/TOOLS.md`

- [ ] **Step 1: Document `standard_full_v2` in `docs/TENSOR_SPEC.md`**

Add a section after `standard_lite_v2`:

```markdown
### `standard_full_v2`

`standard_full_v2` is an opt-in experimental profile. It extends
`standard_lite_v2` with a shallow action-aligned table:

| Field | Value |
|---|---:|
| `id` | `standard_full_v2` |
| `version` | 2 |
| `tensor_version` | 2 |
| `feature_schema_version` | `standard_full_v2.1` |
| `tensor_size` | 43008 |
| `card_id_slot_count` | 542 |
| `scalar_slot_count` | 42466 |

| Section id | Start offset | Shape | Size |
|---|---:|---:|---:|
| `global_features` | 0 | `[64]` | 64 |
| `player_summary` | 64 | `[2][32]` | 64 |
| `permanent_slots` | 128 | `[2][15][96]` | 2880 |
| `own_hand` | 3008 | `[30][32]` | 960 |
| `known_zone_cards` | 3968 | `[120][8]` | 960 |
| `decision_context` | 4928 | `[64]` | 64 |
| `pending_choice_features` | 4992 | `[32][96]` | 3072 |
| `action_id_features` | 8064 | `[2168][16]` | 34688 |
| `reserved` | 42752 | `[256]` | 256 |
```

Also document the action row layout:

```markdown
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
| 15 | reserved |
```

- [ ] **Step 2: Mark full v2 as opt-in in design docs**

In both design specs, update the `standard_full_v2` text from "future experiment" to "implemented opt-in experiment". Keep `standard_lite_v2` as the default profile.

- [ ] **Step 3: Add a profiling command to `docs/TOOLS.md`**

Add:

```powershell
$env:DIGIMON_BACKEND='rust'
python -m digimon_gym.agents.pilot_training --tensor-profile standard_full_v2 --timesteps 10000
```

Note that full v2 should be compared by wall-clock throughput and sample efficiency against `standard_lite_v2`.

- [ ] **Step 4: Run final verification**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile_full_v2 tensor_v2_full -- --nocapture
$env:DIGIMON_BACKEND='rust'
python -m pytest code/tests/rl/test_tensor_profiles.py code/tests/rl/test_onnx_export_profiles.py -v
```

Expected: PASS.

- [ ] **Step 5: Commit docs**

```powershell
git add docs/TENSOR_SPEC.md docs/TOOLS.md docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md docs/superpowers/specs/2026-05-01-observation-profile-registry-design.md
git commit -m "docs: document standard full v2 observation profile"
```

---

## Open Decisions

1. Keep `standard_lite_v2` as default. `standard_full_v2` should stay opt-in until profiling proves the action table pays for its much larger input.
2. Leave action row offsets `10` and `11` zero in the first pass. Populate them only after there is structured cost/amount metadata that does not duplicate rule logic or parse labels.
3. Do not put card IDs in `action_id_features`. Card identity remains in hand, permanent, known-zone, and pending-choice sections so the extractor's card/scalar split stays simple.
4. Do not add aliases like `full_v2` unless the CLI ergonomics really need them. Stable public ID should be `standard_full_v2`.

## Self-Review

- Spec coverage: The plan implements `standard_full_v2` as lite plus `action_id_features[2168][16]`, keeps the action mask as legality oracle, preserves fair-information redaction by reusing lite sections, leaves default training on lite, and exports layout metadata through the existing profile system.
- Placeholder scan: The only generated value is `LAYOUT_HASH`, with an explicit computation procedure. All file paths, profile IDs, tensor sizes, offsets, commands, and expected outputs are concrete.
- Type consistency: Profile ID is `standard_full_v2`; tensor size is `43008`; action row count is `ACTION_SPACE_SIZE = 2168`; row width is `16`; card/scalar counts are `542` and `42466`.

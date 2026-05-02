# Standard Lite V2 Tensor Profile Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `standard_lite_v2` fair-information RL observation profile and make it selectable end-to-end through Rust, PyO3, `DigimonEnv`, feature extraction, training metadata, and ONNX export.

**Architecture:** Build v2 as an explicit profile alongside the compact v1 profile before changing defaults. Rust owns profile layout, tensor writing, card/scalar positions, feature schema version, and layout hash; Python consumes exported layout metadata and never guesses tensor shape. The first complete v2 pass writes mechanically useful board/hand/known-zone/decision data and basic pending-choice provenance, with effect-category tags added through explicit observation metadata instead of prompt parsing.

**Tech Stack:** Rust `digimon-engine`, PyO3 `digimon-engine-py`, Python `digimon_gym`, Gymnasium, SB3 MaskablePPO, pytest, Rust integration tests, `maturin develop`.

---

## Prerequisite

Complete [2026-05-02-standard-compact-profile-naming.md](2026-05-02-standard-compact-profile-naming.md) first. This plan assumes:

- The current 1375-float profile ID is `standard_compact_v1`.
- `standard_v1` and `compact_v1` are accepted aliases.
- New v2 public IDs follow `standard_<shape>_v<version>`.

Do not start this plan by renaming v1 again. If the prerequisite is not landed, land it or adapt only the exact profile strings while preserving this plan's architecture.

## File Structure

- Create `code/digimon-engine/src/tensor_profiles/standard/v2_lite.rs`: profile-owned layout constants for `standard_lite_v2`, section table, row sizes, card/scalar position generation.
- Modify `code/digimon-engine/src/tensor_profiles/standard/mod.rs`: expose `v2_lite` and keep v1 as the default until the final switch task.
- Modify `code/digimon-engine/src/tensor_profiles/mod.rs`: add layout schema metadata, feature schema version, layout hash, canonical profile list, alias parsing, and position coverage validation.
- Create `code/digimon-engine/src/observation.rs`: profile enum, parser, selected-profile layout API, and dispatch to profile-specific tensor builders.
- Create `code/digimon-engine/src/tensor_v2_lite.rs`: `standard_lite_v2` tensor writer.
- Modify `code/digimon-engine/src/tensor.rs`: keep compact v1 compatibility writer unchanged.
- Modify `code/digimon-engine/src/runners/headless.rs`: store observation profile and dispatch tensor calls through `observation`.
- Modify `code/digimon-engine-py/src/lib.rs`: expose observation profile/list/layout APIs and allow `RustHeadlessGame(..., observation_profile=None)`.
- Modify `code/digimon_gym/tensor_profiles.py`: consume richer Rust layout metadata and keep compact fallback only.
- Modify `code/digimon_gym/digimon_gym.py`: accept `tensor_profile`, resolve env var, set observation space from layout, and include layout metadata in `info`.
- Modify `code/digimon_gym/agents/features_extractor.py`: accept an `observation_layout` dict or dataclass and assert layout compatibility.
- Modify `code/digimon_gym/agents/training_config.py`: add `tensor_profile`.
- Modify `code/digimon_gym/agents/pilot_training.py`: pass profile to env/extractor, log layout metadata, and write metadata sidecars.
- Modify `code/digimon_gym/agents/training_metrics.py`: record observation profile metadata.
- Modify `code/tools/export_onnx.py` and `code/tools/export_random_onnx.py`: export with selected profile size and sidecar metadata.
- Modify docs: `docs/TENSOR_SPEC.md`, `docs/ACTION_SPEC.md` if needed, `docs/TOOLS.md`, and v2 design specs.

---

### Task 1: Profile Layout Metadata And Hash Foundation

**Files:**
- Modify: `code/digimon-engine/Cargo.toml`
- Modify: `code/digimon-engine/src/tensor_profiles/mod.rs`
- Modify: `code/digimon-engine/src/tensor_profiles/standard/v1.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs`

- [ ] **Step 1: Write failing layout metadata tests**

Append these tests to `code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs`:

```rust
#[test]
fn every_profile_has_schema_version_and_layout_hash() {
    for id in all_profile_ids() {
        let profile = profile_by_id(id).unwrap();
        assert!(!profile.feature_schema_version.is_empty());
        assert!(profile.layout_hash.starts_with("sha256:"));
        assert_eq!(profile.layout_hash.len(), "sha256:".len() + 64);
    }
}

#[test]
fn layout_hash_changes_when_feature_schema_version_changes() {
    let profile = default_profile();
    let baseline = profile.layout_hash;
    let changed = profile.layout_hash_with_schema_version_for_test("schema-version-test-only");

    assert_ne!(changed, baseline);
    assert!(changed.starts_with("sha256:"));
}

#[test]
fn sections_expose_debug_shapes() {
    let profile = default_profile();
    let global = profile.section("global").unwrap();
    assert_eq!(global.shape, &[10]);

    let my_battle = profile.section("my_battle").unwrap();
    assert_eq!(my_battle.shape, &[14, 40]);
}
```

- [ ] **Step 2: Run the Rust profile tests and verify they fail**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile -- --nocapture
```

Expected: FAIL because `feature_schema_version`, `layout_hash`, `shape`, and `layout_hash_with_schema_version_for_test` do not exist yet.

- [ ] **Step 3: Add hashing dependency**

In `code/digimon-engine/Cargo.toml`, add:

```toml
sha2 = "0.10"
```

under `[dependencies]`.

- [ ] **Step 4: Extend shared tensor profile metadata**

In `code/digimon-engine/src/tensor_profiles/mod.rs`, change `TensorSection` and `TensorProfile` to:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorSection {
    pub id: &'static str,
    pub start: usize,
    pub len: usize,
    pub shape: &'static [usize],
    pub kind: TensorSectionKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorProfile {
    pub id: &'static str,
    pub game_mode: &'static str,
    pub version: u32,
    pub tensor_version: u16,
    pub feature_schema_version: &'static str,
    pub layout_hash: &'static str,
    pub tensor_size: usize,
    pub field_slots: usize,
    pub slot_size: usize,
    pub max_sources: usize,
    pub slot_layout: TensorSlotLayout,
    pub card_id_slot_count: usize,
    pub scalar_slot_count: usize,
    pub sections: &'static [TensorSection],
}
```

Add canonical hash input helpers below `impl TensorProfile`:

```rust
impl TensorProfile {
    pub fn layout_hash_with_schema_version_for_test(&self, schema_version: &str) -> String {
        compute_layout_hash(
            self.id,
            self.tensor_version,
            schema_version,
            self.tensor_size,
            self.sections,
            &self.positions().0,
            &self.positions().1,
        )
    }
}

pub fn compute_layout_hash(
    profile_id: &str,
    tensor_version: u16,
    feature_schema_version: &str,
    tensor_size: usize,
    sections: &[TensorSection],
    card_id_positions: &[usize],
    scalar_positions: &[usize],
) -> String {
    use sha2::{Digest, Sha256};

    let mut canonical = String::new();
    canonical.push_str(profile_id);
    canonical.push('|');
    canonical.push_str(&tensor_version.to_string());
    canonical.push('|');
    canonical.push_str(feature_schema_version);
    canonical.push('|');
    canonical.push_str(&tensor_size.to_string());
    for section in sections {
        canonical.push('|');
        canonical.push_str(section.id);
        canonical.push(':');
        canonical.push_str(&section.start.to_string());
        canonical.push(':');
        canonical.push_str(&section.len.to_string());
        canonical.push(':');
        canonical.push_str(&format!("{:?}", section.shape));
        canonical.push(':');
        canonical.push_str(&format!("{:?}", section.kind));
    }
    canonical.push('|');
    canonical.push_str(&format!("{:?}", card_id_positions));
    canonical.push('|');
    canonical.push_str(&format!("{:?}", scalar_positions));

    let digest = Sha256::digest(canonical.as_bytes());
    format!("sha256:{digest:x}")
}
```

Use this simple canonical string now so the plan does not introduce a serialization subsystem. The hash is deterministic and changes when schema, sections, size, or positions change.

- [ ] **Step 5: Populate compact v1 metadata**

In `code/digimon-engine/src/tensor_profiles/standard/v1.rs`, add:

```rust
pub const TENSOR_VERSION: u16 = 1;
pub const FEATURE_SCHEMA_VERSION: &str = "standard_compact_v1.1";
pub const LAYOUT_HASH: &str = "sha256:REPLACE_WITH_COMPUTED_HASH";
```

Add shape arrays:

```rust
pub const SHAPE_GLOBAL: &[usize] = &[GLOBAL_SIZE];
pub const SHAPE_BATTLE: &[usize] = &[FIELD_SLOTS, SLOT_SIZE];
pub const SHAPE_HAND: &[usize] = &[HAND_SIZE];
pub const SHAPE_TRASH: &[usize] = &[TRASH_SIZE];
pub const SHAPE_SECURITY: &[usize] = &[SECURITY_SIZE];
pub const SHAPE_BREEDING: &[usize] = &[1, SLOT_SIZE];
pub const SHAPE_REVEALED: &[usize] = &[REVEALED_SIZE];
pub const SHAPE_SELECTION: &[usize] = &[SELECTION_SIZE];
```

Add `shape` to every `TensorSection`, for example:

```rust
TensorSection {
    id: "global",
    start: OFF_GLOBAL,
    len: GLOBAL_SIZE,
    shape: SHAPE_GLOBAL,
    kind: TensorSectionKind::Scalars,
},
TensorSection {
    id: "my_battle",
    start: OFF_MY_BATTLE,
    len: BATTLE_SIZE,
    shape: SHAPE_BATTLE,
    kind: TensorSectionKind::PermanentSlots,
},
```

Update the `PROFILE` initializer:

```rust
pub const PROFILE: TensorProfile = TensorProfile {
    id: PROFILE_ID,
    game_mode: GAME_MODE,
    version: VERSION,
    tensor_version: TENSOR_VERSION,
    feature_schema_version: FEATURE_SCHEMA_VERSION,
    layout_hash: LAYOUT_HASH,
    tensor_size: TENSOR_SIZE,
    field_slots: FIELD_SLOTS,
    slot_size: SLOT_SIZE,
    max_sources: MAX_SOURCES,
    slot_layout: SLOT_LAYOUT,
    card_id_slot_count: CARD_ID_SLOT_COUNT,
    scalar_slot_count: SCALAR_SLOT_COUNT,
    sections: SECTIONS,
};
```

To compute the real `LAYOUT_HASH`, temporarily set it to any valid-looking hash, run this one-off Rust test output command after the code compiles:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor print_compact_hash -- --nocapture
```

Add this temporary test only while computing, then remove it:

```rust
#[test]
fn print_compact_hash() {
    let profile = default_profile();
    println!("{}", profile.layout_hash_with_schema_version_for_test(profile.feature_schema_version));
}
```

Replace `REPLACE_WITH_COMPUTED_HASH` with the printed hash and remove the temporary test before committing.

- [ ] **Step 6: Run the Rust profile tests and verify they pass**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile -- --nocapture
```

Expected: PASS. Existing compact v1 tensor size and position counts remain unchanged.

- [ ] **Step 7: Commit**

```powershell
git add code/digimon-engine/Cargo.toml code/digimon-engine/src/tensor_profiles/mod.rs code/digimon-engine/src/tensor_profiles/standard/v1.rs code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs
git commit -m "feat: add tensor profile layout hashes"
```

---

### Task 2: Add `standard_lite_v2` Profile Layout

**Files:**
- Create: `code/digimon-engine/src/tensor_profiles/standard/v2_lite.rs`
- Modify: `code/digimon-engine/src/tensor_profiles/standard/mod.rs`
- Modify: `code/digimon-engine/src/tensor_profiles/mod.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/tensor_profile_v2.rs`
- Modify: `code/digimon-engine/tests/mask_and_tensor/main.rs`

- [ ] **Step 1: Write failing v2 layout tests**

Create `code/digimon-engine/tests/mask_and_tensor/tensor_profile_v2.rs`:

```rust
use digimon_engine::tensor_profiles::{all_profile_ids, profile_by_id, TensorSectionKind};

#[test]
fn standard_lite_v2_layout_matches_spec() {
    let profile = profile_by_id("standard_lite_v2").unwrap();

    assert_eq!(profile.id, "standard_lite_v2");
    assert_eq!(profile.game_mode, "standard");
    assert_eq!(profile.version, 2);
    assert_eq!(profile.tensor_version, 2);
    assert_eq!(profile.feature_schema_version, "standard_lite_v2.1");
    assert_eq!(profile.tensor_size, 8320);
    assert_eq!(profile.card_id_slot_count, 542);
    assert_eq!(profile.scalar_slot_count, 7778);

    assert_eq!(profile.section("global_features").unwrap().start, 0);
    assert_eq!(profile.section("player_summary").unwrap().start, 64);
    assert_eq!(profile.section("permanent_slots").unwrap().start, 128);
    assert_eq!(profile.section("own_hand").unwrap().start, 3008);
    assert_eq!(profile.section("known_zone_cards").unwrap().start, 3968);
    assert_eq!(profile.section("decision_context").unwrap().start, 4928);
    assert_eq!(profile.section("pending_choice_features").unwrap().start, 4992);
    assert_eq!(profile.section("reserved").unwrap().start, 8064);

    assert_eq!(profile.section("permanent_slots").unwrap().shape, &[2, 15, 96]);
    assert_eq!(profile.section("own_hand").unwrap().shape, &[30, 32]);
    assert_eq!(profile.section("known_zone_cards").unwrap().shape, &[120, 8]);
    assert_eq!(profile.section("pending_choice_features").unwrap().shape, &[32, 96]);
    assert_eq!(profile.section("reserved").unwrap().kind, TensorSectionKind::Scalars);
}

#[test]
fn standard_lite_v2_positions_cover_tensor_once() {
    let profile = profile_by_id("standard_lite_v2").unwrap();
    let (cards, scalars) = profile.positions();

    assert_eq!(cards.len(), 542);
    assert_eq!(scalars.len(), 7778);
    assert_eq!(cards.len() + scalars.len(), profile.tensor_size);

    let card_set: std::collections::BTreeSet<_> = cards.iter().copied().collect();
    let scalar_set: std::collections::BTreeSet<_> = scalars.iter().copied().collect();
    assert!(card_set.is_disjoint(&scalar_set));

    let all: std::collections::BTreeSet<_> = card_set.union(&scalar_set).copied().collect();
    let expected: std::collections::BTreeSet<_> = (0..profile.tensor_size).collect();
    assert_eq!(all, expected);
}

#[test]
fn profile_list_includes_compact_and_lite_v2() {
    let profiles = all_profile_ids();
    assert!(profiles.contains(&"standard_compact_v1"));
    assert!(profiles.contains(&"standard_lite_v2"));
}
```

In `code/digimon-engine/tests/mask_and_tensor/main.rs`, add:

```rust
mod tensor_profile_v2;
```

- [ ] **Step 2: Run v2 layout tests and verify they fail**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile_v2 -- --nocapture
```

Expected: FAIL because `standard_lite_v2` does not exist.

- [ ] **Step 3: Add the v2 profile layout module**

Create `code/digimon-engine/src/tensor_profiles/standard/v2_lite.rs`:

```rust
use crate::tensor_profiles::{
    TensorFieldKind, TensorProfile, TensorSection, TensorSectionKind, TensorSlotField,
    TensorSlotLayout,
};

pub const PROFILE_ID: &str = "standard_lite_v2";
pub const GAME_MODE: &str = "standard";
pub const VERSION: u32 = 2;
pub const TENSOR_VERSION: u16 = 2;
pub const FEATURE_SCHEMA_VERSION: &str = "standard_lite_v2.1";
pub const LAYOUT_HASH: &str = "sha256:REPLACE_WITH_COMPUTED_HASH";

pub const GLOBAL_FEATURES_SIZE: usize = 64;
pub const PLAYER_SUMMARY_PLAYERS: usize = 2;
pub const PLAYER_SUMMARY_ROW_SIZE: usize = 32;
pub const PLAYER_SUMMARY_SIZE: usize = PLAYER_SUMMARY_PLAYERS * PLAYER_SUMMARY_ROW_SIZE;
pub const PERMANENT_PLAYERS: usize = 2;
pub const PERMANENT_SLOTS_PER_PLAYER: usize = 15;
pub const PERMANENT_SLOT_SIZE: usize = 96;
pub const PERMANENT_SLOTS_SIZE: usize =
    PERMANENT_PLAYERS * PERMANENT_SLOTS_PER_PLAYER * PERMANENT_SLOT_SIZE;
pub const OWN_HAND_ROWS: usize = 30;
pub const OWN_HAND_ROW_SIZE: usize = 32;
pub const OWN_HAND_SIZE: usize = OWN_HAND_ROWS * OWN_HAND_ROW_SIZE;
pub const KNOWN_ZONE_ROWS: usize = 120;
pub const KNOWN_ZONE_ROW_SIZE: usize = 8;
pub const KNOWN_ZONE_SIZE: usize = KNOWN_ZONE_ROWS * KNOWN_ZONE_ROW_SIZE;
pub const DECISION_CONTEXT_SIZE: usize = 64;
pub const PENDING_CHOICE_ROWS: usize = 32;
pub const PENDING_CHOICE_ROW_SIZE: usize = 96;
pub const PENDING_CHOICE_SIZE: usize = PENDING_CHOICE_ROWS * PENDING_CHOICE_ROW_SIZE;
pub const RESERVED_SIZE: usize = 256;

pub const OFF_GLOBAL_FEATURES: usize = 0;
pub const OFF_PLAYER_SUMMARY: usize = OFF_GLOBAL_FEATURES + GLOBAL_FEATURES_SIZE;
pub const OFF_PERMANENT_SLOTS: usize = OFF_PLAYER_SUMMARY + PLAYER_SUMMARY_SIZE;
pub const OFF_OWN_HAND: usize = OFF_PERMANENT_SLOTS + PERMANENT_SLOTS_SIZE;
pub const OFF_KNOWN_ZONE_CARDS: usize = OFF_OWN_HAND + OWN_HAND_SIZE;
pub const OFF_DECISION_CONTEXT: usize = OFF_KNOWN_ZONE_CARDS + KNOWN_ZONE_SIZE;
pub const OFF_PENDING_CHOICE_FEATURES: usize = OFF_DECISION_CONTEXT + DECISION_CONTEXT_SIZE;
pub const OFF_RESERVED: usize = OFF_PENDING_CHOICE_FEATURES + PENDING_CHOICE_SIZE;
pub const TENSOR_SIZE: usize = OFF_RESERVED + RESERVED_SIZE;

pub const PERM_TOP_CARD_ID_OFFSET: usize = 8;
pub const PERM_SOURCE_START_OFFSET: usize = 63;
pub const PERM_SOURCE_ENTRY_SIZE: usize = 3;
pub const PERM_MAX_SOURCES: usize = 11;
pub const OWN_HAND_CARD_ID_OFFSET: usize = 1;
pub const KNOWN_ZONE_CARD_ID_OFFSET: usize = 1;
pub const PENDING_SOURCE_CARD_ID_OFFSET: usize = 44;

pub const SHAPE_GLOBAL_FEATURES: &[usize] = &[GLOBAL_FEATURES_SIZE];
pub const SHAPE_PLAYER_SUMMARY: &[usize] = &[PLAYER_SUMMARY_PLAYERS, PLAYER_SUMMARY_ROW_SIZE];
pub const SHAPE_PERMANENT_SLOTS: &[usize] =
    &[PERMANENT_PLAYERS, PERMANENT_SLOTS_PER_PLAYER, PERMANENT_SLOT_SIZE];
pub const SHAPE_OWN_HAND: &[usize] = &[OWN_HAND_ROWS, OWN_HAND_ROW_SIZE];
pub const SHAPE_KNOWN_ZONE_CARDS: &[usize] = &[KNOWN_ZONE_ROWS, KNOWN_ZONE_ROW_SIZE];
pub const SHAPE_DECISION_CONTEXT: &[usize] = &[DECISION_CONTEXT_SIZE];
pub const SHAPE_PENDING_CHOICE_FEATURES: &[usize] =
    &[PENDING_CHOICE_ROWS, PENDING_CHOICE_ROW_SIZE];
pub const SHAPE_RESERVED: &[usize] = &[RESERVED_SIZE];

pub const SECTIONS: &[TensorSection] = &[
    TensorSection { id: "global_features", start: OFF_GLOBAL_FEATURES, len: GLOBAL_FEATURES_SIZE, shape: SHAPE_GLOBAL_FEATURES, kind: TensorSectionKind::Scalars },
    TensorSection { id: "player_summary", start: OFF_PLAYER_SUMMARY, len: PLAYER_SUMMARY_SIZE, shape: SHAPE_PLAYER_SUMMARY, kind: TensorSectionKind::Scalars },
    TensorSection { id: "permanent_slots", start: OFF_PERMANENT_SLOTS, len: PERMANENT_SLOTS_SIZE, shape: SHAPE_PERMANENT_SLOTS, kind: TensorSectionKind::Custom },
    TensorSection { id: "own_hand", start: OFF_OWN_HAND, len: OWN_HAND_SIZE, shape: SHAPE_OWN_HAND, kind: TensorSectionKind::Custom },
    TensorSection { id: "known_zone_cards", start: OFF_KNOWN_ZONE_CARDS, len: KNOWN_ZONE_SIZE, shape: SHAPE_KNOWN_ZONE_CARDS, kind: TensorSectionKind::Custom },
    TensorSection { id: "decision_context", start: OFF_DECISION_CONTEXT, len: DECISION_CONTEXT_SIZE, shape: SHAPE_DECISION_CONTEXT, kind: TensorSectionKind::Scalars },
    TensorSection { id: "pending_choice_features", start: OFF_PENDING_CHOICE_FEATURES, len: PENDING_CHOICE_SIZE, shape: SHAPE_PENDING_CHOICE_FEATURES, kind: TensorSectionKind::Custom },
    TensorSection { id: "reserved", start: OFF_RESERVED, len: RESERVED_SIZE, shape: SHAPE_RESERVED, kind: TensorSectionKind::Scalars },
];

pub const SLOT_HEADER_FIELDS: &[TensorSlotField] = &[
    TensorSlotField { id: "top_card_id", offset: PERM_TOP_CARD_ID_OFFSET, kind: TensorFieldKind::CardId },
];

pub const SOURCE_FIELDS: &[TensorSlotField] = &[
    TensorSlotField { id: "card_id", offset: 0, kind: TensorFieldKind::CardId },
    TensorSlotField { id: "opt_state", offset: 1, kind: TensorFieldKind::Scalar },
    TensorSlotField { id: "dp_contribution", offset: 2, kind: TensorFieldKind::Scalar },
];

pub const SLOT_LAYOUT: TensorSlotLayout = TensorSlotLayout {
    size: PERMANENT_SLOT_SIZE,
    source_start: PERM_SOURCE_START_OFFSET,
    source_entry_size: PERM_SOURCE_ENTRY_SIZE,
    max_sources: PERM_MAX_SOURCES,
    header_fields: SLOT_HEADER_FIELDS,
    source_fields: SOURCE_FIELDS,
};

pub const CARD_ID_SLOT_COUNT: usize = 542;
pub const SCALAR_SLOT_COUNT: usize = TENSOR_SIZE - CARD_ID_SLOT_COUNT;

pub const PROFILE: TensorProfile = TensorProfile {
    id: PROFILE_ID,
    game_mode: GAME_MODE,
    version: VERSION,
    tensor_version: TENSOR_VERSION,
    feature_schema_version: FEATURE_SCHEMA_VERSION,
    layout_hash: LAYOUT_HASH,
    tensor_size: TENSOR_SIZE,
    field_slots: 14,
    slot_size: PERMANENT_SLOT_SIZE,
    max_sources: PERM_MAX_SOURCES,
    slot_layout: SLOT_LAYOUT,
    card_id_slot_count: CARD_ID_SLOT_COUNT,
    scalar_slot_count: SCALAR_SLOT_COUNT,
    sections: SECTIONS,
};
```

- [ ] **Step 4: Teach `TensorProfile::positions()` about custom v2 sections**

In `code/digimon-engine/src/tensor_profiles/mod.rs`, add `Custom`:

```rust
pub enum TensorSectionKind {
    Scalars,
    CardIds,
    PermanentSlots,
    Custom,
}
```

In `TensorProfile::positions()`, add:

```rust
TensorSectionKind::Custom => {
    custom_section_positions(
        self.id,
        section,
        &mut card_positions,
        &mut scalar_positions,
    );
}
```

Add this helper:

```rust
fn custom_section_positions(
    profile_id: &str,
    section: &TensorSection,
    card_positions: &mut Vec<usize>,
    scalar_positions: &mut Vec<usize>,
) {
    if profile_id != crate::tensor_profiles::standard::v2_lite::PROFILE_ID {
        scalar_positions.extend(section.start..section.start + section.len);
        return;
    }

    use crate::tensor_profiles::standard::v2_lite as v2;
    match section.id {
        "permanent_slots" => {
            for row in 0..(v2::PERMANENT_PLAYERS * v2::PERMANENT_SLOTS_PER_PLAYER) {
                let base = section.start + row * v2::PERMANENT_SLOT_SIZE;
                card_positions.push(base + v2::PERM_TOP_CARD_ID_OFFSET);
                for source in 0..v2::PERM_MAX_SOURCES {
                    card_positions.push(
                        base + v2::PERM_SOURCE_START_OFFSET + source * v2::PERM_SOURCE_ENTRY_SIZE
                    );
                }
            }
            push_remaining_scalars(section, card_positions, scalar_positions);
        }
        "own_hand" => {
            for row in 0..v2::OWN_HAND_ROWS {
                card_positions.push(
                    section.start + row * v2::OWN_HAND_ROW_SIZE + v2::OWN_HAND_CARD_ID_OFFSET
                );
            }
            push_remaining_scalars(section, card_positions, scalar_positions);
        }
        "known_zone_cards" => {
            for row in 0..v2::KNOWN_ZONE_ROWS {
                card_positions.push(
                    section.start + row * v2::KNOWN_ZONE_ROW_SIZE + v2::KNOWN_ZONE_CARD_ID_OFFSET
                );
            }
            push_remaining_scalars(section, card_positions, scalar_positions);
        }
        "pending_choice_features" => {
            for row in 0..v2::PENDING_CHOICE_ROWS {
                card_positions.push(
                    section.start
                        + row * v2::PENDING_CHOICE_ROW_SIZE
                        + v2::PENDING_SOURCE_CARD_ID_OFFSET
                );
            }
            push_remaining_scalars(section, card_positions, scalar_positions);
        }
        _ => scalar_positions.extend(section.start..section.start + section.len),
    }
}

fn push_remaining_scalars(
    section: &TensorSection,
    card_positions: &[usize],
    scalar_positions: &mut Vec<usize>,
) {
    for position in section.start..section.start + section.len {
        if !card_positions.contains(&position) {
            scalar_positions.push(position);
        }
    }
}
```

- [ ] **Step 5: Register v2 profile**

In `code/digimon-engine/src/tensor_profiles/standard/mod.rs`, add:

```rust
pub mod v2_lite;
```

and update:

```rust
pub fn profile_by_version(version: u32) -> Option<TensorProfile> {
    match version {
        1 => Some(v1::PROFILE),
        2 => Some(v2_lite::PROFILE),
        _ => None,
    }
}
```

In `code/digimon-engine/src/tensor_profiles/mod.rs`, update:

```rust
pub const STANDARD_LITE_V2_PROFILE_ID: &str = standard::v2_lite::PROFILE_ID;

pub fn all_profile_ids() -> Vec<&'static str> {
    vec![standard::v1::PROFILE_ID, standard::v2_lite::PROFILE_ID]
}

pub fn profile_by_id(id: &str) -> Option<TensorProfile> {
    match id {
        standard::v1::PROFILE_ID
        | STANDARD_V1_LEGACY_PROFILE_ID
        | COMPACT_V1_LEGACY_PROFILE_ID => Some(standard::v1::PROFILE),
        standard::v2_lite::PROFILE_ID | "v2_lite" => Some(standard::v2_lite::PROFILE),
        _ => None,
    }
}
```

- [ ] **Step 6: Compute and pin the v2 layout hash**

Use the same temporary-test pattern from Task 1:

```rust
#[test]
fn print_v2_hash() {
    let profile = profile_by_id("standard_lite_v2").unwrap();
    println!("{}", profile.layout_hash_with_schema_version_for_test(profile.feature_schema_version));
}
```

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor print_v2_hash -- --nocapture
```

Replace `REPLACE_WITH_COMPUTED_HASH` in `v2_lite.rs` with the printed hash and remove the temporary test.

- [ ] **Step 7: Run v2 layout tests and verify they pass**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile_v2 -- --nocapture
```

Expected: PASS.

- [ ] **Step 8: Commit**

```powershell
git add code/digimon-engine/src/tensor_profiles code/digimon-engine/tests/mask_and_tensor
git commit -m "feat: add standard lite v2 tensor profile"
```

---

### Task 3: Observation Profile Dispatch In Rust

**Files:**
- Create: `code/digimon-engine/src/observation.rs`
- Modify: `code/digimon-engine/src/lib.rs`
- Modify: `code/digimon-engine/src/runners/headless.rs`
- Test: `code/digimon-engine/tests/infra/headless_runner.rs`

- [ ] **Step 1: Write failing runner profile-selection tests**

In `code/digimon-engine/tests/infra/headless_runner.rs`, add:

```rust
#[test]
fn runner_default_observation_profile_is_compact_v1() {
    let runner = sample_runner();

    assert_eq!(runner.observation_profile_id(), "standard_compact_v1");
    assert_eq!(runner.observation_layout().tensor_size, digimon_engine::tensor::TENSOR_SIZE);
    assert_eq!(runner.get_board_tensor(None).len(), digimon_engine::tensor::TENSOR_SIZE);
}

#[test]
fn runner_can_use_standard_lite_v2_observation_profile() {
    let runner = sample_runner_with_observation_profile("standard_lite_v2");

    assert_eq!(runner.observation_profile_id(), "standard_lite_v2");
    assert_eq!(runner.observation_layout().tensor_size, 8320);
    assert_eq!(runner.get_board_tensor(None).len(), 8320);
}
```

If `sample_runner()` currently constructs directly through `HeadlessRunner::new`, add this helper in the same test file:

```rust
fn sample_runner_with_observation_profile(profile_id: &str) -> digimon_engine::HeadlessRunner {
    let mut runner = sample_runner();
    runner.set_observation_profile_for_test(profile_id).unwrap();
    runner
}
```

- [ ] **Step 2: Run infra tests and verify they fail**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test infra observation_profile -- --nocapture
```

Expected: FAIL because runner profile methods do not exist.

- [ ] **Step 3: Create the observation dispatch module**

Create `code/digimon-engine/src/observation.rs`:

```rust
use crate::card_registry::CardRegistry;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::tensor_profiles::{default_profile, profile_by_id, TensorProfile};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationProfileId {
    StandardCompactV1,
    StandardLiteV2,
}

impl ObservationProfileId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StandardCompactV1 => default_profile().id,
            Self::StandardLiteV2 => crate::tensor_profiles::standard::v2_lite::PROFILE_ID,
        }
    }
}

pub fn default_observation_profile() -> ObservationProfileId {
    ObservationProfileId::StandardCompactV1
}

pub fn parse_observation_profile(raw: &str) -> Result<ObservationProfileId, String> {
    match raw {
        "standard_compact_v1" | "standard_v1" | "compact_v1" => {
            Ok(ObservationProfileId::StandardCompactV1)
        }
        "standard_lite_v2" | "v2_lite" => Ok(ObservationProfileId::StandardLiteV2),
        other => Err(format!("unknown observation profile: {other}")),
    }
}

pub fn list_observation_profiles() -> Vec<&'static str> {
    crate::tensor_profiles::all_profile_ids()
}

pub fn observation_layout(profile: ObservationProfileId) -> TensorProfile {
    profile_by_id(profile.as_str()).expect("registered observation profile")
}

pub fn build_observation_tensor(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
    profile: ObservationProfileId,
) -> Vec<f32> {
    match profile {
        ObservationProfileId::StandardCompactV1 => crate::tensor::build_tensor(game, player_id, registry),
        ObservationProfileId::StandardLiteV2 => {
            crate::tensor_v2_lite::build_tensor_v2_lite(game, player_id, registry)
        }
    }
}
```

In `code/digimon-engine/src/lib.rs`, add:

```rust
pub mod observation;
pub mod tensor_v2_lite;
```

- [ ] **Step 4: Add runner profile storage**

In `code/digimon-engine/src/runners/headless.rs`, change imports:

```rust
use crate::observation::{
    build_observation_tensor, default_observation_profile, observation_layout,
    parse_observation_profile, ObservationProfileId,
};
```

Add a field to `HeadlessRunner`:

```rust
observation_profile: ObservationProfileId,
```

Initialize it in `new`:

```rust
observation_profile: default_observation_profile(),
```

Add methods:

```rust
pub fn new_with_observation_profile(
    deck1_ids: Vec<String>,
    deck2_ids: Vec<String>,
    all_card_data: &HashMap<String, CardData>,
    verbose: bool,
    record_actions: bool,
    record_tensors: bool,
    seed: Option<u64>,
    observation_profile: ObservationProfileId,
) -> Result<Self, String> {
    let mut runner = Self::new(
        deck1_ids,
        deck2_ids,
        all_card_data,
        verbose,
        record_actions,
        record_tensors,
        seed,
    )?;
    runner.observation_profile = observation_profile;
    Ok(runner)
}

pub fn observation_profile_id(&self) -> &'static str {
    self.observation_profile.as_str()
}

pub fn observation_layout(&self) -> crate::tensor_profiles::TensorProfile {
    observation_layout(self.observation_profile)
}

pub fn set_observation_profile_for_test(&mut self, raw: &str) -> Result<(), String> {
    self.observation_profile = parse_observation_profile(raw)?;
    Ok(())
}
```

Change `get_board_tensor` to:

```rust
pub fn get_board_tensor(&self, player_id: Option<PlayerId>) -> Vec<f32> {
    let pid = player_id.unwrap_or_else(|| self.current_decision_player());
    build_observation_tensor(&self.game, pid, &self.registry, self.observation_profile)
}
```

- [ ] **Step 5: Add a temporary v2 tensor builder stub**

Create `code/digimon-engine/src/tensor_v2_lite.rs`:

```rust
use crate::card_registry::CardRegistry;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::tensor_profiles::standard::v2_lite;

pub fn build_tensor_v2_lite(
    _game: &Game,
    _player_id: PlayerId,
    _registry: &CardRegistry,
) -> Vec<f32> {
    let mut tensor = vec![0.0; v2_lite::TENSOR_SIZE];
    tensor[v2_lite::OFF_GLOBAL_FEATURES] = 2.0;
    tensor
}
```

This stub is intentionally tiny so dispatch can land before the writer. Task 4 replaces it with the real writer.

- [ ] **Step 6: Run infra tests and verify they pass**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test infra observation_profile -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add code/digimon-engine/src/observation.rs code/digimon-engine/src/tensor_v2_lite.rs code/digimon-engine/src/lib.rs code/digimon-engine/src/runners/headless.rs code/digimon-engine/tests/infra/headless_runner.rs
git commit -m "feat: dispatch observation tensors by profile"
```

---

### Task 4: Implement The `standard_lite_v2` Tensor Writer

**Files:**
- Modify: `code/digimon-engine/src/tensor_v2_lite.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/tensor_v2_lite.rs`
- Modify: `code/digimon-engine/tests/mask_and_tensor/main.rs`

- [ ] **Step 1: Write failing v2 tensor behavior tests**

Create `code/digimon-engine/tests/mask_and_tensor/tensor_v2_lite.rs`:

```rust
use digimon_engine::observation::{build_observation_tensor, parse_observation_profile};
use digimon_engine::tensor_profiles::standard::v2_lite;

use crate::tensor_helpers::{make_registry, sample_game_with_known_cards};

#[test]
fn v2_lite_tensor_has_expected_size_and_version_marker() {
    let (game, registry) = sample_game_with_known_cards();
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);

    assert_eq!(tensor.len(), 8320);
    assert_eq!(tensor[v2_lite::OFF_GLOBAL_FEATURES], 2.0);
}

#[test]
fn v2_lite_does_not_encode_opponent_hand_identities() {
    let (game, registry) = sample_game_with_known_cards();
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);
    let opponent_hand_card_index = registry.get_index("ST1-03") as f32;

    let known_positions = &digimon_engine::tensor_profiles::profile_by_id("standard_lite_v2")
        .unwrap()
        .positions()
        .0;
    for position in known_positions {
        if *position < v2_lite::OFF_OWN_HAND
            || *position >= v2_lite::OFF_OWN_HAND + v2_lite::OWN_HAND_SIZE
        {
            assert_ne!(tensor[*position], opponent_hand_card_index);
        }
    }
}

#[test]
fn v2_lite_uses_breeding_slot_14_with_battle_affordances_off() {
    let (game, registry) = sample_game_with_known_cards();
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);

    let own_breeding_base =
        v2_lite::OFF_PERMANENT_SLOTS + 14 * v2_lite::PERMANENT_SLOT_SIZE;
    assert_eq!(tensor[own_breeding_base + 0], 1.0); // active
    assert_eq!(tensor[own_breeding_base + 3], 0.0); // zone_battle
    assert_eq!(tensor[own_breeding_base + 4], 1.0); // zone_breeding
    assert_eq!(tensor[own_breeding_base + 33], 0.0); // can_attack_now
    assert_eq!(tensor[own_breeding_base + 34], 0.0); // can_block_now
}
```

If `sample_game_with_known_cards()` does not exist, add it to `code/digimon-engine/tests/mask_and_tensor/tensor_helpers.rs` as a small helper that constructs a game with ST1 cards, one own breeding permanent, and opponent hand cards. The helper must not reveal opponent hand identities through public zones.

Add to `code/digimon-engine/tests/mask_and_tensor/main.rs`:

```rust
mod tensor_v2_lite;
```

- [ ] **Step 2: Run v2 tensor tests and verify they fail**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_v2_lite -- --nocapture
```

Expected: FAIL because the writer only emits a version marker.

- [ ] **Step 3: Implement v2 writer section scaffolding**

Replace `code/digimon-engine/src/tensor_v2_lite.rs` with:

```rust
use crate::card_data::CardData;
use crate::card_registry::CardRegistry;
use crate::card_source::CardSource;
use crate::enums::{CardKind, GamePhase, PlayerId};
use crate::game::Game;
use crate::permanent::{Permanent, PermanentHandle};
use crate::player::Player;
use crate::tensor::DP_NORM;
use crate::tensor_profiles::standard::v2_lite as layout;

pub fn build_tensor_v2_lite(game: &Game, player_id: PlayerId, registry: &CardRegistry) -> Vec<f32> {
    let mut tensor = vec![0.0f32; layout::TENSOR_SIZE];
    let opponent_id = game.next_clockwise(player_id);

    write_global_features(&mut tensor, game, player_id);
    write_player_summary(&mut tensor, game, player_id, 0, player_id);
    write_player_summary(&mut tensor, game, player_id, 1, opponent_id);
    write_permanent_table(&mut tensor, game, registry, player_id, 0, player_id);
    write_permanent_table(&mut tensor, game, registry, player_id, 1, opponent_id);
    write_own_hand(&mut tensor, game, registry, player_id);
    write_known_zone_cards(&mut tensor, game, registry, player_id, opponent_id);
    write_decision_context(&mut tensor, game, player_id);
    write_pending_choice_features(&mut tensor, game, registry);

    tensor
}
```

Add helper functions in the same file:

```rust
fn write_global_features(t: &mut [f32], game: &Game, observer: PlayerId) {
    let base = layout::OFF_GLOBAL_FEATURES;
    t[base] = 2.0;
    t[base + 1] = (game.turn_count as f32 / 30.0).min(1.0);
    t[base + 2] = if game.turn_player() == observer { game.memory } else { -game.memory } as f32 / 10.0;
    write_phase_one_hot(t, base + 8, game.current_phase);
    t[base + 40] = relative_player(game.turn_player(), observer);
    t[base + 41] = if game.game_over { 1.0 } else { 0.0 };
    if let Some(winner) = game.winner {
        t[base + 42] = relative_player(winner, observer);
    }
}

fn write_player_summary(
    t: &mut [f32],
    game: &Game,
    observer: PlayerId,
    row: usize,
    player_id: PlayerId,
) {
    let player = game.player(player_id);
    let base = layout::OFF_PLAYER_SUMMARY + row * layout::PLAYER_SUMMARY_ROW_SIZE;
    t[base] = player.deck.len() as f32 / 60.0;
    t[base + 1] = player.digitama_deck.len() as f32 / 10.0;
    t[base + 2] = player.hand.len() as f32 / 30.0;
    t[base + 3] = player.security.len() as f32 / 10.0;
    t[base + 4] = player.trash.len() as f32 / 60.0;
    t[base + 5] = player.battle_area.len() as f32 / 14.0;
    t[base + 6] = if player.breeding_area.is_some() { 1.0 } else { 0.0 };
    t[base + 7] = relative_player(player_id, observer);
}

fn write_permanent_table(
    t: &mut [f32],
    game: &Game,
    registry: &CardRegistry,
    observer: PlayerId,
    player_row: usize,
    player_id: PlayerId,
) {
    let player = game.player(player_id);
    for (slot, permanent) in player.battle_area.iter().take(14).enumerate() {
        let handle = PermanentHandle { player: player_id, index: slot as u8 };
        write_permanent_row(
            t,
            layout::OFF_PERMANENT_SLOTS
                + (player_row * layout::PERMANENT_SLOTS_PER_PLAYER + slot)
                    * layout::PERMANENT_SLOT_SIZE,
            game,
            registry,
            observer,
            player_id,
            slot,
            permanent,
            Some(handle),
            false,
        );
    }
    if let Some(permanent) = player.breeding_area.as_ref() {
        write_permanent_row(
            t,
            layout::OFF_PERMANENT_SLOTS
                + (player_row * layout::PERMANENT_SLOTS_PER_PLAYER + 14)
                    * layout::PERMANENT_SLOT_SIZE,
            game,
            registry,
            observer,
            player_id,
            14,
            permanent,
            None,
            true,
        );
    }
}
```

Continue with these exact row writers:

```rust
fn write_permanent_row(
    t: &mut [f32],
    base: usize,
    game: &Game,
    registry: &CardRegistry,
    observer: PlayerId,
    controller: PlayerId,
    slot: usize,
    permanent: &Permanent,
    handle: Option<PermanentHandle>,
    is_breeding: bool,
) {
    let top = permanent.top_card();
    let card = &game.card_data[top.card_index as usize];
    t[base] = 1.0;
    t[base + 1] = relative_player(controller, observer);
    t[base + 2] = slot as f32 / 14.0;
    t[base + 3] = if is_breeding { 0.0 } else { 1.0 };
    t[base + 4] = if is_breeding { 1.0 } else { 0.0 };
    t[base + layout::PERM_TOP_CARD_ID_OFFSET] = registry.get_index(&top.card_id(&game.card_data)) as f32;
    write_static_card_features(t, base + 9, card);
    t[base + 21] = permanent.base_dp(&game.card_data).unwrap_or(0) as f32 / DP_NORM;
    t[base + 22] = if permanent.is_suspended { 1.0 } else { 0.0 };
    t[base + 23] = permanent.card_sources.len() as f32 / 11.0;
    t[base + 24] = permanent.linked_cards.len() as f32 / 5.0;
    if !is_breeding {
        t[base + 33] = if permanent.is_suspended { 0.0 } else { 1.0 };
        t[base + 34] = 0.0;
    }
    for (source_idx, source) in permanent.card_sources.iter().take(layout::PERM_MAX_SOURCES).enumerate() {
        let source_base =
            base + layout::PERM_SOURCE_START_OFFSET + source_idx * layout::PERM_SOURCE_ENTRY_SIZE;
        t[source_base] = registry.get_index(&source.card_id(&game.card_data)) as f32;
        if let Some(handle) = handle {
            t[source_base + 1] = game.source_opt_state(handle, source_idx);
            t[source_base + 2] = game.source_dp_contribution(handle, source_idx) as f32 / DP_NORM;
        }
    }
}

fn write_own_hand(t: &mut [f32], game: &Game, registry: &CardRegistry, player_id: PlayerId) {
    let player = game.player(player_id);
    let mask = crate::action::build_action_mask(game, player_id);
    for (idx, card_source) in player.hand.iter().take(layout::OWN_HAND_ROWS).enumerate() {
        let base = layout::OFF_OWN_HAND + idx * layout::OWN_HAND_ROW_SIZE;
        let card = &game.card_data[card_source.card_index as usize];
        t[base] = 1.0;
        t[base + layout::OWN_HAND_CARD_ID_OFFSET] = registry.get_index(&card_source.card_id(&game.card_data)) as f32;
        write_static_card_features(t, base + 2, card);
        t[base + 19] = mask.get(idx).copied().unwrap_or(0.0);
        t[base + 20] = mask.get(30 + idx).copied().unwrap_or(0.0);
        t[base + 21] = if (400 + idx * 15..400 + (idx + 1) * 15)
            .any(|action| mask.get(action).copied().unwrap_or(0.0) > 0.5) { 1.0 } else { 0.0 };
        t[base + 22] = mask.get(63 + idx).copied().unwrap_or(0.0);
    }
}
```

Implement known-zone and decision helpers:

```rust
fn write_known_zone_cards(
    t: &mut [f32],
    game: &Game,
    registry: &CardRegistry,
    observer: PlayerId,
    opponent: PlayerId,
) {
    let mut row = 0usize;
    row = write_card_rows(t, row, game, registry, observer, game.player(observer), &game.player(observer).trash, 1.0, 0.0, 45);
    row = write_card_rows(t, row, game, registry, observer, game.player(opponent), &game.player(opponent).trash, -1.0, 1.0, 45);
    write_security_rows(t, row, game, registry, observer, game.player(observer), 1.0, 2.0);
    write_security_rows(t, 100, game, registry, observer, game.player(opponent), -1.0, 3.0);
    write_card_rows(t, 110, game, registry, observer, game.player(observer), &game.revealed_cards, 0.0, 4.0, 10);
}

fn write_decision_context(t: &mut [f32], game: &Game, observer: PlayerId) {
    let base = layout::OFF_DECISION_CONTEXT;
    write_phase_one_hot(t, base, game.current_phase);
    t[base + 24] = relative_player(game.turn_player(), observer);
    if let Some(sel) = game.pending_selection.as_ref() {
        t[base + 25] = 1.0;
        t[base + 26] = relative_player(sel.selecting_player, observer);
        t[base + 27] = if sel.is_optional { 1.0 } else { 0.0 };
        t[base + 28] = sel.valid_action_ids.len() as f32 / 32.0;
    }
}

fn write_pending_choice_features(t: &mut [f32], game: &Game, registry: &CardRegistry) {
    if let Some(sel) = game.pending_selection.as_ref() {
        for (row, action_id) in sel.valid_action_ids.iter().take(layout::PENDING_CHOICE_ROWS).enumerate() {
            let base = layout::OFF_PENDING_CHOICE_FEATURES + row * layout::PENDING_CHOICE_ROW_SIZE;
            t[base] = 1.0;
            t[base + 1] = 1.0;
            t[base + 2] = *action_id as f32 / crate::action::space::ACTION_SPACE_SIZE as f32;
            t[base + 3] = row as f32 / layout::PENDING_CHOICE_ROWS as f32;
            t[base + 4] = sel.valid_action_ids.len() as f32 / layout::PENDING_CHOICE_ROWS as f32;
            t[base + 18] = if sel.is_optional { 1.0 } else { 0.0 };
            t[base + layout::PENDING_SOURCE_CARD_ID_OFFSET] =
                registry.get_index(&sel.source_card.card_id(&game.card_data)) as f32;
        }
    }
}
```

Add utility functions:

```rust
fn write_static_card_features(t: &mut [f32], base: usize, card: &CardData) {
    t[base] = card_kind_bucket(card.card_kind);
    t[base + 1] = card.level.unwrap_or(0) as f32 / 7.0;
    t[base + 2] = card.dp.unwrap_or(0) as f32 / DP_NORM;
    t[base + 3] = card.play_cost as f32 / 15.0;
    for color in &card.colors {
        let idx = (*color as usize).min(6);
        t[base + 4 + idx] = 1.0;
    }
}

fn card_kind_bucket(kind: CardKind) -> f32 {
    match kind {
        CardKind::Digimon => 1.0,
        CardKind::Tamer => 2.0,
        CardKind::Option => 3.0,
        CardKind::DigiEgg => 4.0,
        CardKind::Token => 5.0,
        CardKind::Dual => 6.0,
    }
}

fn relative_player(player: PlayerId, observer: PlayerId) -> f32 {
    if player == observer { 1.0 } else { -1.0 }
}

fn write_phase_one_hot(t: &mut [f32], start: usize, phase: GamePhase) {
    let idx = phase.tensor_value() as usize;
    if idx < 20 {
        t[start + idx] = 1.0;
    }
}
```

Add row helpers using public `CardSource::card_id` and `Player::face_up_security`:

```rust
fn write_card_rows(
    t: &mut [f32],
    start_row: usize,
    game: &Game,
    registry: &CardRegistry,
    _observer: PlayerId,
    _owner: &Player,
    cards: &[CardSource],
    owner_relative: f32,
    zone_bucket: f32,
    limit: usize,
) -> usize {
    for (idx, card_source) in cards.iter().take(limit).enumerate() {
        let base = layout::OFF_KNOWN_ZONE_CARDS + (start_row + idx) * layout::KNOWN_ZONE_ROW_SIZE;
        let card = &game.card_data[card_source.card_index as usize];
        t[base] = 1.0;
        t[base + layout::KNOWN_ZONE_CARD_ID_OFFSET] =
            registry.get_index(&card_source.card_id(&game.card_data)) as f32;
        t[base + 2] = owner_relative;
        t[base + 3] = zone_bucket;
        t[base + 4] = idx as f32 / limit.max(1) as f32;
        t[base + 5] = card_kind_bucket(card.card_kind);
        t[base + 6] = card.level.unwrap_or(0) as f32 / 7.0;
        t[base + 7] = card.dp.unwrap_or(card.play_cost as i32) as f32 / DP_NORM;
    }
    start_row + limit
}

fn write_security_rows(
    t: &mut [f32],
    start_row: usize,
    game: &Game,
    registry: &CardRegistry,
    _observer: PlayerId,
    owner: &Player,
    owner_relative: f32,
    zone_bucket: f32,
) {
    for (idx, card_source) in owner.security.iter().take(10).enumerate() {
        let base = layout::OFF_KNOWN_ZONE_CARDS + (start_row + idx) * layout::KNOWN_ZONE_ROW_SIZE;
        t[base] = 1.0;
        if owner.face_up_security.contains(&card_source.card_index) {
            let card = &game.card_data[card_source.card_index as usize];
            t[base + layout::KNOWN_ZONE_CARD_ID_OFFSET] =
                registry.get_index(&card_source.card_id(&game.card_data)) as f32;
            t[base + 5] = card_kind_bucket(card.card_kind);
            t[base + 6] = card.level.unwrap_or(0) as f32 / 7.0;
            t[base + 7] = card.dp.unwrap_or(card.play_cost as i32) as f32 / DP_NORM;
        }
        t[base + 2] = owner_relative;
        t[base + 3] = zone_bucket;
        t[base + 4] = idx as f32 / 10.0;
    }
}
```

- [ ] **Step 4: Run v2 tensor tests and verify they pass**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_v2_lite -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run compact tensor tests to catch regressions**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_and_mask tensor_hidden_info tensor_source_contributions -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-engine/src/tensor_v2_lite.rs code/digimon-engine/tests/mask_and_tensor
git commit -m "feat: write standard lite v2 observations"
```

---

### Task 5: PyO3 Profile Selection And Layout Export

**Files:**
- Modify: `code/digimon-engine-py/src/lib.rs`
- Test: `code/tests/test_rust_bindings_surface.py`

- [ ] **Step 1: Write failing PyO3 tests**

In `code/tests/test_rust_bindings_surface.py`, add to `TestTensorProfiles`:

```python
def test_observation_layout_for_standard_lite_v2(self):
    from digimon_engine import get_observation_layout, list_observation_profiles

    assert "standard_lite_v2" in list_observation_profiles()
    layout = get_observation_layout("standard_lite_v2")

    assert layout["profile_id"] == "standard_lite_v2"
    assert layout["tensor_version"] == 2
    assert layout["feature_schema_version"] == "standard_lite_v2.1"
    assert layout["tensor_size"] == 8320
    assert len(layout["card_id_positions"]) == 542
    assert len(layout["scalar_positions"]) == 7778
    assert layout["layout_hash"].startswith("sha256:")
    assert layout["sections"][0] == {
        "name": "global_features",
        "offset": 0,
        "size": 64,
        "shape": [64],
    }


def test_rust_headless_game_accepts_observation_profile(self):
    from digimon_engine import RustHeadlessGame

    deck = ["ST1-01"] * 5 + ["ST1-03"] * 45
    game = RustHeadlessGame(deck, deck, seed=1, observation_profile="standard_lite_v2")

    assert game.observation_profile_id == "standard_lite_v2"
    assert game.get_observation_layout()["tensor_size"] == 8320
    assert game.get_board_tensor(1).shape == (8320,)
```

- [ ] **Step 2: Run PyO3 tests and verify they fail**

Run:

```powershell
python -m pytest code/tests/test_rust_bindings_surface.py::TestTensorProfiles -v
```

Expected: FAIL because the new functions and constructor argument are not exposed.

- [ ] **Step 3: Expose layout dictionaries**

In `code/digimon-engine-py/src/lib.rs`, import:

```rust
use ::digimon_engine::observation::{
    default_observation_profile, list_observation_profiles as rust_list_observation_profiles,
    observation_layout, parse_observation_profile, ObservationProfileId,
};
```

Add helper:

```rust
fn profile_to_pydict(py: Python<'_>, profile: &RustTensorProfile) -> PyResult<PyObject> {
    let d = PyDict::new_bound(py);
    let (card_id_positions, scalar_positions) = profile.positions();
    d.set_item("profile_id", profile.id)?;
    d.set_item("tensor_version", profile.tensor_version)?;
    d.set_item("feature_schema_version", profile.feature_schema_version)?;
    d.set_item("tensor_size", profile.tensor_size)?;
    d.set_item("card_id_positions", card_id_positions)?;
    d.set_item("scalar_positions", scalar_positions)?;
    d.set_item("layout_hash", profile.layout_hash)?;
    let sections = PyList::empty_bound(py);
    for section in profile.sections {
        let sd = PyDict::new_bound(py);
        sd.set_item("name", section.id)?;
        sd.set_item("offset", section.start)?;
        sd.set_item("size", section.len)?;
        sd.set_item("shape", section.shape.to_vec())?;
        sections.append(sd)?;
    }
    d.set_item("sections", sections)?;
    Ok(d.into_py(py))
}
```

Add functions:

```rust
#[pyfunction]
fn list_observation_profiles() -> Vec<String> {
    rust_list_observation_profiles().into_iter().map(str::to_string).collect()
}

#[pyfunction]
#[pyo3(signature = (profile_id = None))]
fn get_observation_layout(py: Python<'_>, profile_id: Option<String>) -> PyResult<PyObject> {
    let parsed = match profile_id {
        None => default_observation_profile(),
        Some(raw) => parse_observation_profile(&raw).map_err(PyValueError::new_err)?,
    };
    let profile = observation_layout(parsed);
    profile_to_pydict(py, &profile)
}
```

Register both in `#[pymodule]`:

```rust
m.add_function(wrap_pyfunction!(list_observation_profiles, m)?)?;
m.add_function(wrap_pyfunction!(get_observation_layout, m)?)?;
m.add("DEFAULT_OBSERVATION_PROFILE", default_observation_profile().as_str())?;
```

- [ ] **Step 4: Add `RustHeadlessGame` constructor profile argument**

Change the constructor signature to:

```rust
#[pyo3(signature = (
    deck1_ids,
    deck2_ids,
    verbose = false,
    record_actions = false,
    record_tensors = false,
    seed = None,
    observation_profile = None
))]
fn new(
    deck1_ids: Vec<String>,
    deck2_ids: Vec<String>,
    verbose: bool,
    record_actions: bool,
    record_tensors: bool,
    seed: Option<u64>,
    observation_profile: Option<String>,
) -> PyResult<Self> {
```

Parse and construct:

```rust
let profile = match observation_profile {
    None => default_observation_profile(),
    Some(raw) => parse_observation_profile(&raw).map_err(PyValueError::new_err)?,
};
let runner = HeadlessRunner::new_with_observation_profile(
    deck1_ids,
    deck2_ids,
    db,
    verbose,
    record_actions,
    record_tensors,
    seed,
    profile,
)
.map_err(PyValueError::new_err)?;
```

Add PyO3 methods:

```rust
#[getter]
fn observation_profile_id(&self) -> &'static str {
    self.inner.observation_profile_id()
}

fn get_observation_layout(&self, py: Python<'_>) -> PyResult<PyObject> {
    let profile = self.inner.observation_layout();
    profile_to_pydict(py, &profile)
}
```

- [ ] **Step 5: Rebuild PyO3 and run tests**

Run:

```powershell
Push-Location code/digimon-engine-py
maturin develop
Pop-Location
python -m pytest code/tests/test_rust_bindings_surface.py::TestTensorProfiles -v
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-engine-py/src/lib.rs code/tests/test_rust_bindings_surface.py
git commit -m "feat: expose observation profiles through pyo3"
```

---

### Task 6: `DigimonEnv` Profile Selection And Feature Extraction

**Files:**
- Modify: `code/digimon_gym/tensor_profiles.py`
- Modify: `code/digimon_gym/digimon_gym.py`
- Modify: `code/digimon_gym/agents/features_extractor.py`
- Test: `code/tests/rl/test_tensor_profiles.py`
- Test: `code/tests/rl/test_rust_runner_adapter.py`

- [ ] **Step 1: Write failing Python env/extractor tests**

Add to `code/tests/rl/test_tensor_profiles.py`:

```python
def test_get_standard_lite_v2_tensor_profile_from_rust():
    from digimon_gym.tensor_profiles import get_tensor_profile

    profile = get_tensor_profile("standard_lite_v2")

    assert profile.id == "standard_lite_v2"
    assert profile.tensor_size == 8320
    assert profile.card_id_slot_count == 542
    assert profile.scalar_slot_count == 7778
    assert profile.layout_hash.startswith("sha256:")


def test_feature_extractor_accepts_observation_layout():
    import numpy as np
    import torch
    from gymnasium import spaces
    from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
    from digimon_gym.tensor_profiles import get_tensor_profile

    profile = get_tensor_profile("standard_lite_v2")
    space = spaces.Box(low=-10.0, high=20001.0, shape=(profile.tensor_size,), dtype=np.float32)
    extractor = CardEmbeddingExtractor(space, observation_layout=profile)
    out = extractor(torch.zeros((2, profile.tensor_size), dtype=torch.float32))

    assert extractor.card_id_indices.numel() == 542
    assert extractor.scalar_indices.numel() == 7778
    assert tuple(out.shape) == (2, 512)
```

Add to `code/tests/rl/test_rust_runner_adapter.py`:

```python
def test_digimon_env_uses_requested_standard_lite_v2_profile(monkeypatch):
    monkeypatch.setenv("DIGIMON_BACKEND", "rust")
    from digimon_gym.digimon_gym import DigimonEnv

    env = DigimonEnv(tensor_profile="standard_lite_v2")
    obs, info = env.reset(seed=7)

    assert env.tensor_profile == "standard_lite_v2"
    assert env.observation_space.shape == (8320,)
    assert obs.shape == (8320,)
    assert info["tensor_profile"] == "standard_lite_v2"
    assert info["tensor_feature_schema_version"] == "standard_lite_v2.1"
    assert info["tensor_layout_hash"].startswith("sha256:")
```

- [ ] **Step 2: Run Python tests and verify they fail**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profiles.py code/tests/rl/test_rust_runner_adapter.py -v
```

Expected: FAIL because Python wrappers and env do not accept v2 profile selection.

- [ ] **Step 3: Extend Python tensor profile wrapper**

In `code/digimon_gym/tensor_profiles.py`, extend the dataclass:

```python
@dataclass(frozen=True)
class TensorSection:
    name: str
    offset: int
    size: int
    shape: tuple[int, ...]

@dataclass(frozen=True)
class TensorProfile:
    id: str
    game_mode: str
    version: int
    tensor_version: int
    feature_schema_version: str
    tensor_size: int
    field_slots: int
    slot_size: int
    max_sources: int
    card_id_slot_count: int
    scalar_slot_count: int
    card_id_positions: tuple[int, ...]
    scalar_positions: tuple[int, ...]
    layout_hash: str = ""
    sections: tuple[TensorSection, ...] = ()
```

When Rust has `get_observation_layout`, prefer it:

```python
get_layout = getattr(digimon_engine, "get_observation_layout", None)
if get_layout is not None:
    raw = get_layout(profile_id)
    return TensorProfile(
        id=raw["profile_id"],
        game_mode=raw["profile_id"].split("_", 1)[0],
        version=int(raw["tensor_version"]),
        tensor_version=int(raw["tensor_version"]),
        feature_schema_version=raw["feature_schema_version"],
        tensor_size=int(raw["tensor_size"]),
        field_slots=15 if raw["profile_id"] == "standard_lite_v2" else 14,
        slot_size=96 if raw["profile_id"] == "standard_lite_v2" else 40,
        max_sources=11,
        card_id_slot_count=len(raw["card_id_positions"]),
        scalar_slot_count=len(raw["scalar_positions"]),
        card_id_positions=tuple(raw["card_id_positions"]),
        scalar_positions=tuple(raw["scalar_positions"]),
        layout_hash=raw["layout_hash"],
        sections=tuple(
            TensorSection(
                name=s["name"],
                offset=int(s["offset"]),
                size=int(s["size"]),
                shape=tuple(s["shape"]),
            )
            for s in raw["sections"]
        ),
    )
```

Keep compact fallback only for no Rust module. If fallback receives `standard_lite_v2`, raise:

```python
raise ValueError("standard_lite_v2 requires digimon_engine observation layout support")
```

- [ ] **Step 4: Update `DigimonEnv` profile selection**

In `code/digimon_gym/digimon_gym.py`, import:

```python
from digimon_gym.tensor_profiles import get_tensor_profile
```

Change `_make_runner` signature:

```python
def _make_runner(deck1: List[str], deck2: List[str], seed: Optional[int] = None,
                 tensor_profile: str = "standard_compact_v1"):
```

Pass the profile to Rust:

```python
return RustHeadlessGame(deck1, deck2, seed=seed, observation_profile=tensor_profile)
```

Reject non-compact Python legacy:

```python
if tensor_profile not in ("standard_compact_v1", "standard_v1", "compact_v1"):
    raise RuntimeError(
        f"tensor_profile={tensor_profile!r} requires DIGIMON_BACKEND=rust"
    )
```

Change `DigimonEnv.__init__` signature:

```python
def __init__(self, deck1: Optional[List[str]] = None,
             deck2: Optional[List[str]] = None,
             render_mode: Optional[str] = None,
             max_turns: int = 100,
             tensor_profile: Optional[str] = None):
```

Resolve layout:

```python
self.tensor_profile = tensor_profile or os.environ.get("DIGIMON_TENSOR_PROFILE") or "standard_compact_v1"
self.observation_layout = get_tensor_profile(self.tensor_profile)
self.tensor_profile = self.observation_layout.id
self.observation_space = spaces.Box(
    low=-10.0,
    high=20001.0,
    shape=(self.observation_layout.tensor_size,),
    dtype=np.float32,
)
```

Pass profile on reset:

```python
self.runner = _make_runner(deck1, deck2, seed=seed, tensor_profile=self.tensor_profile)
```

Add helper:

```python
def _tensor_info(self) -> Dict[str, Any]:
    return {
        "tensor_profile": self.tensor_profile,
        "tensor_feature_schema_version": self.observation_layout.feature_schema_version,
        "tensor_layout_hash": self.observation_layout.layout_hash,
    }
```

Change reset and step info creation:

```python
info = {"action_mask": self.action_mask(), **self._tensor_info()}
```

Change `GameState.get_observation()` zeros shape:

```python
return {"tensor": np.zeros(self._env.observation_layout.tensor_size, dtype=np.float32)}
```

- [ ] **Step 5: Update feature extractor layout injection**

In `code/digimon_gym/agents/features_extractor.py`, change constructor signature:

```python
observation_layout=None,
tensor_profile_id: Optional[str] = None,
```

Resolve:

```python
profile = observation_layout or get_tensor_profile(tensor_profile_id)
```

Use existing fields. Add a coverage assertion:

```python
positions = set(profile.card_id_positions) | set(profile.scalar_positions)
if len(positions) != profile.tensor_size:
    raise ValueError(f"tensor profile {profile.id} positions do not cover tensor")
```

- [ ] **Step 6: Run Python tests and verify they pass**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profiles.py code/tests/rl/test_rust_runner_adapter.py -v
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add code/digimon_gym/tensor_profiles.py code/digimon_gym/digimon_gym.py code/digimon_gym/agents/features_extractor.py code/tests/rl/test_tensor_profiles.py code/tests/rl/test_rust_runner_adapter.py
git commit -m "feat: select observation profiles in digimon env"
```

---

### Task 7: Training CLI And Model Metadata

**Files:**
- Modify: `code/digimon_gym/agents/training_config.py`
- Modify: `code/digimon_gym/agents/pilot_training.py`
- Modify: `code/digimon_gym/agents/training_metrics.py`
- Test: `code/tests/rl/test_pilot_training_config.py` or create it if missing

- [ ] **Step 1: Write failing training config metadata tests**

Create `code/tests/rl/test_pilot_training_config.py` if it does not exist:

```python
from pathlib import Path

from digimon_gym.agents.training_config import TrainingConfig


def test_training_config_accepts_tensor_profile(tmp_path: Path):
    config_path = tmp_path / "training.yaml"
    config_path.write_text("tensor_profile: standard_lite_v2\n")

    cfg = TrainingConfig.from_yaml(config_path)

    assert cfg.tensor_profile == "standard_lite_v2"


def test_training_config_rejects_blank_tensor_profile(tmp_path: Path):
    config_path = tmp_path / "training.yaml"
    config_path.write_text("tensor_profile: ''\n")

    try:
        TrainingConfig.from_yaml(config_path)
    except ValueError as exc:
        assert "tensor_profile must not be blank" in str(exc)
    else:
        raise AssertionError("expected ValueError")
```

- [ ] **Step 2: Run config tests and verify they fail**

Run:

```powershell
python -m pytest code/tests/rl/test_pilot_training_config.py -v
```

Expected: FAIL because `TrainingConfig.tensor_profile` does not exist.

- [ ] **Step 3: Add config and metadata fields**

In `code/digimon_gym/agents/training_config.py`, add:

```python
tensor_profile: str = "standard_compact_v1"
```

to the dataclass. Add validation:

```python
if not self.tensor_profile:
    raise ValueError("tensor_profile must not be blank")
```

In `code/digimon_gym/agents/training_metrics.py`, add fields to `TrainingRunMetadata`:

```python
observation_profile: str = ""
tensor_version: int = 0
feature_schema_version: str = ""
tensor_size: int = 0
tensor_layout_hash: str = ""
action_space_size: int = 0
card_registry_capacity: int = 0
embedding_dim: int = 0
```

- [ ] **Step 4: Pass profile through training envs**

In `code/digimon_gym/agents/pilot_training.py`, update `make_env` and `make_vec_env` signatures to accept `tensor_profile: str = "standard_compact_v1"` and construct:

```python
base_env = DigimonEnv(deck1=deck1, tensor_profile=tensor_profile)
```

In `train`, include:

```python
from digimon_gym.tensor_profiles import get_tensor_profile
from digimon_engine import ACTION_SPACE_SIZE, REGISTRY_CAPACITY, EMBEDDING_DIM

observation_layout = get_tensor_profile(cfg.tensor_profile)
```

Pass `tensor_profile=cfg.tensor_profile` into every `make_env`, `make_vec_env`, and eval env call.

Change extractor kwargs:

```python
features_extractor_kwargs=dict(
    features_dim=512,
    pretrained_embeddings=pretrained_embeddings,
    observation_layout=observation_layout,
),
```

Add logging:

```python
print(f"  Tensor profile: {observation_layout.id}")
print(f"  Tensor schema:  {observation_layout.feature_schema_version}")
print(f"  Tensor size:    {observation_layout.tensor_size}")
print(f"  Tensor hash:    {observation_layout.layout_hash}")
```

Add CLI override:

```python
parser.add_argument(
    "--tensor-profile",
    type=str,
    default=None,
    help="Observation tensor profile, e.g. standard_compact_v1 or standard_lite_v2.",
)
```

Add to `legacy_overrides`:

```python
"tensor_profile": args.tensor_profile,
```

- [ ] **Step 5: Save profile metadata sidecar**

In the `TrainingRunMetadata` construction, add:

```python
observation_profile=observation_layout.id,
tensor_version=observation_layout.tensor_version,
feature_schema_version=observation_layout.feature_schema_version,
tensor_size=observation_layout.tensor_size,
tensor_layout_hash=observation_layout.layout_hash,
action_space_size=ACTION_SPACE_SIZE,
card_registry_capacity=REGISTRY_CAPACITY,
embedding_dim=EMBEDDING_DIM,
```

- [ ] **Step 6: Run training config tests**

Run:

```powershell
python -m pytest code/tests/rl/test_pilot_training_config.py -v
```

Expected: PASS.

- [ ] **Step 7: Run a one-episode env smoke check**

Run:

```powershell
$env:DIGIMON_BACKEND='rust'
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(tensor_profile='standard_lite_v2'); obs,info=env.reset(seed=1); print(obs.shape, info['tensor_profile'], info['action_mask'].shape)"
```

Expected output includes:

```text
(8320,) standard_lite_v2 (2168,)
```

- [ ] **Step 8: Commit**

```powershell
git add code/digimon_gym/agents/training_config.py code/digimon_gym/agents/pilot_training.py code/digimon_gym/agents/training_metrics.py code/tests/rl/test_pilot_training_config.py
git commit -m "feat: record observation profiles in pilot training"
```

---

### Task 8: ONNX Export Profile Metadata

**Files:**
- Modify: `code/tools/export_onnx.py`
- Modify: `code/tools/export_random_onnx.py`
- Test: `code/tests/rl/test_onnx_export_profiles.py`

- [ ] **Step 1: Write failing ONNX profile tests**

Create `code/tests/rl/test_onnx_export_profiles.py`:

```python
import argparse


def test_export_onnx_parser_accepts_tensor_profile():
    from tools.export_onnx import build_parser

    parser = build_parser()
    args = parser.parse_args([
        "--type", "mlp",
        "--input", "model.zip",
        "--output", "model.onnx",
        "--tensor-profile", "standard_lite_v2",
    ])

    assert args.tensor_profile == "standard_lite_v2"


def test_export_random_onnx_parser_accepts_tensor_profile():
    from tools.export_random_onnx import build_parser

    parser = build_parser()
    args = parser.parse_args([
        "--type", "mlp",
        "--output", "random.onnx",
        "--tensor-profile", "standard_lite_v2",
    ])

    assert args.tensor_profile == "standard_lite_v2"
```

- [ ] **Step 2: Run ONNX export parser tests and verify they fail**

Run:

```powershell
python -m pytest code/tests/rl/test_onnx_export_profiles.py -v
```

Expected: FAIL because parser builder functions or `--tensor-profile` do not exist.

- [ ] **Step 3: Refactor parsers and selected tensor size**

In both export scripts, add:

```python
from digimon_gym.tensor_profiles import get_tensor_profile
```

Create `build_parser()` that includes:

```python
parser.add_argument(
    "--tensor-profile",
    default=None,
    help="Observation profile for dummy input shape and metadata.",
)
```

Where dummy observations are created, replace `TENSOR_SIZE` with:

```python
layout = get_tensor_profile(args.tensor_profile)
dummy_obs = torch.randn(1, layout.tensor_size, dtype=torch.float32)
```

For modules that need input size in constructors, pass `layout.tensor_size`.

- [ ] **Step 4: Write ONNX sidecar metadata**

After successful export, write `<output>.meta.json`:

```python
meta = {
    "observation_profile": layout.id,
    "tensor_version": layout.tensor_version,
    "feature_schema_version": layout.feature_schema_version,
    "tensor_size": layout.tensor_size,
    "tensor_layout_hash": layout.layout_hash,
    "action_space_size": ACTION_SPACE_SIZE,
    "card_registry_capacity": REGISTRY_CAPACITY,
    "embedding_dim": EMBEDDING_DIM,
}
Path(args.output).with_suffix(Path(args.output).suffix + ".meta.json").write_text(
    json.dumps(meta, indent=2)
)
```

Import `REGISTRY_CAPACITY`, `EMBEDDING_DIM`, `json`, and `Path` if missing.

- [ ] **Step 5: Run ONNX export parser tests**

Run:

```powershell
python -m pytest code/tests/rl/test_onnx_export_profiles.py -v
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add code/tools/export_onnx.py code/tools/export_random_onnx.py code/tests/rl/test_onnx_export_profiles.py
git commit -m "feat: export onnx profile metadata"
```

---

### Task 9: Pending Choice Metadata Enrichment

**Files:**
- Modify: `code/digimon-engine/src/effect.rs`
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/tensor_v2_lite.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/tensor_v2_lite.rs`

- [ ] **Step 1: Write failing pending-choice metadata tests**

Append to `code/digimon-engine/tests/mask_and_tensor/tensor_v2_lite.rs`:

```rust
#[test]
fn v2_lite_pending_choice_rows_follow_valid_action_order() {
    let (mut game, registry) = sample_game_with_known_cards();
    crate::tensor_helpers::install_test_pending_selection(
        &mut game,
        vec![1003, 62, 1001],
        false,
    );
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);

    let base = v2_lite::OFF_PENDING_CHOICE_FEATURES;
    assert_eq!(tensor[base], 1.0);
    assert_eq!(tensor[base + 2], 1003.0 / 2168.0);
    assert_eq!(
        tensor[base + v2_lite::PENDING_CHOICE_ROW_SIZE + 2],
        62.0 / 2168.0
    );
    assert_eq!(
        tensor[base + 2 * v2_lite::PENDING_CHOICE_ROW_SIZE + 2],
        1001.0 / 2168.0
    );
}

#[test]
fn v2_lite_pending_choice_rows_include_source_kind_and_timing() {
    let (mut game, registry) = sample_game_with_known_cards();
    crate::tensor_helpers::install_test_trigger_order_selection(&mut game);
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);

    let base = v2_lite::OFF_PENDING_CHOICE_FEATURES;
    assert!(tensor[base + 34] > 0.0); // source kind bucket
    assert!(tensor[base + 22] > 0.0); // timing bucket
    assert!(tensor[base + 44] > 0.0); // source card ID
}
```

- [ ] **Step 2: Run pending metadata tests and verify they fail**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor pending_choice -- --nocapture
```

Expected: FAIL because timing/source-kind buckets are not populated.

- [ ] **Step 3: Add observation metadata structs**

In `code/digimon-engine/src/effect.rs`, add:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectObservationMetadata {
    pub categories: EffectCategoryFlags,
    pub target_profile: TargetProfileFlags,
    pub duration: EffectDurationKind,
    pub numeric_buckets: EffectNumericBuckets,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectCategoryFlags {
    pub delete: bool,
    pub suspend: bool,
    pub unsuspend: bool,
    pub bounce: bool,
    pub bottom_deck: bool,
    pub dp_change: bool,
    pub draw_search: bool,
    pub memory: bool,
    pub play: bool,
    pub digivolve: bool,
    pub recover: bool,
    pub trash_security: bool,
    pub grant_keyword: bool,
    pub grant_immunity: bool,
    pub protection: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TargetProfileFlags {
    pub own: bool,
    pub opponent: bool,
    pub digimon: bool,
    pub tamer: bool,
    pub option: bool,
    pub battle_area: bool,
    pub breeding_area: bool,
    pub security: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EffectDurationKind {
    #[default]
    Unknown,
    Immediate,
    UntilEndOfTurn,
    UntilOpponentEndOfTurn,
    Persistent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectNumericBuckets {
    pub dp_amount: i16,
    pub memory_amount: i8,
    pub count: u8,
    pub level_threshold: u8,
    pub play_cost_threshold: u8,
}
```

Add a field to `Effect`:

```rust
pub observation_metadata: EffectObservationMetadata,
```

Initialize it in `EffectBuilder::new`:

```rust
observation_metadata: EffectObservationMetadata::default(),
```

Add builder method:

```rust
pub fn observation_metadata(mut self, metadata: EffectObservationMetadata) -> Self {
    self.inner.observation_metadata = metadata;
    self
}
```

- [ ] **Step 4: Carry queued-effect observation metadata into selections**

In `code/digimon-engine/src/selection.rs`, add to `EffectChoiceEntry`:

```rust
pub source_card: Option<CardHandle>,
pub source_kind: Option<EffectSourceKind>,
pub timing: Option<EffectTiming>,
pub is_optional: bool,
pub observation_metadata: crate::effect::EffectObservationMetadata,
```

For existing code that creates `EffectChoiceEntry`, populate defaults:

```rust
EffectChoiceEntry {
    label,
    action_id,
    source_card: None,
    source_kind: None,
    timing: None,
    is_optional: false,
    observation_metadata: Default::default(),
}
```

In the trigger-order queue drainer where `EffectChoiceEntry` is created, populate real values from `QueuedEffect`:

```rust
EffectChoiceEntry {
    label,
    action_id,
    source_card: Some(entry.source_card),
    source_kind: Some(entry.source_kind),
    timing: Some(entry.timing),
    is_optional: entry.is_optional,
    observation_metadata: effect.observation_metadata,
}
```

- [ ] **Step 5: Encode pending metadata in v2 writer**

In `write_pending_choice_features`, find the `EffectChoiceEntry` matching `action_id`:

```rust
let choice = sel
    .effect_choices
    .as_ref()
    .and_then(|choices| choices.iter().find(|choice| choice.action_id == *action_id));
```

Populate timing/source fields:

```rust
if let Some(choice) = choice {
    if let Some(timing) = choice.timing {
        t[base + 22 + timing_bucket(timing)] = 1.0;
    }
    if let Some(source_kind) = choice.source_kind {
        t[base + 34 + source_kind_bucket(source_kind)] = 1.0;
    }
    if let Some(source_card) = choice.source_card {
        t[base + layout::PENDING_SOURCE_CARD_ID_OFFSET] =
            registry.get_index(&source_card.card_id(&game.card_data)) as f32;
    }
    write_effect_category_flags(t, base + 45, choice.observation_metadata.categories);
}
```

Add helpers:

```rust
fn timing_bucket(timing: crate::enums::EffectTiming) -> usize {
    use crate::enums::EffectTiming::*;
    match timing {
        OnPlay => 0,
        WhenDigivolving | OnDigivolve | OnDnaDigivolve => 1,
        OnAttack | WhenAttacking => 2,
        SecuritySkill | OnSecurityCheck | OnLoseSecurity => 3,
        EndOfYourTurn | EndOfOpponentsTurn => 4,
        StartOfYourTurn | StartOfOpponentsTurn | StartOfYourMainPhase => 5,
        OnDeletion | OnAnyDeletion => 6,
        CounterEffect => 7,
        OptionMain | DelayEffect => 8,
        _ => 11,
    }
}

fn source_kind_bucket(source_kind: crate::enums::EffectSourceKind) -> usize {
    match source_kind {
        crate::enums::EffectSourceKind::Digimon => 0,
        crate::enums::EffectSourceKind::Inherited => 1,
        crate::enums::EffectSourceKind::Security => 2,
        crate::enums::EffectSourceKind::Option => 3,
        crate::enums::EffectSourceKind::Tamer => 4,
        _ => 9,
    }
}

fn write_effect_category_flags(
    t: &mut [f32],
    start: usize,
    flags: crate::effect::EffectCategoryFlags,
) {
    t[start] = flags.delete as u8 as f32;
    t[start + 1] = flags.suspend as u8 as f32;
    t[start + 2] = flags.unsuspend as u8 as f32;
    t[start + 3] = flags.bounce as u8 as f32;
    t[start + 4] = flags.bottom_deck as u8 as f32;
    t[start + 5] = flags.dp_change as u8 as f32;
    t[start + 6] = flags.draw_search as u8 as f32;
    t[start + 7] = flags.memory as u8 as f32;
    t[start + 8] = flags.play as u8 as f32;
    t[start + 9] = flags.digivolve as u8 as f32;
    t[start + 10] = flags.recover as u8 as f32;
    t[start + 11] = flags.trash_security as u8 as f32;
    t[start + 12] = flags.grant_keyword as u8 as f32;
    t[start + 13] = flags.grant_immunity as u8 as f32;
    t[start + 14] = flags.protection as u8 as f32;
}
```

- [ ] **Step 6: Run pending metadata tests and verify they pass**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor pending_choice -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add code/digimon-engine/src/effect.rs code/digimon-engine/src/selection.rs code/digimon-engine/src/tensor_v2_lite.rs code/digimon-engine/tests/mask_and_tensor/tensor_v2_lite.rs
git commit -m "feat: encode pending choice metadata in v2 tensor"
```

---

### Task 10: Switch Default Pilot Profile To `standard_lite_v2`

**Files:**
- Modify: `code/digimon-engine/src/observation.rs`
- Modify: `code/digimon-engine-py/src/lib.rs`
- Modify: `code/digimon_gym/digimon_gym.py`
- Modify: `configs/training/default.yaml`
- Test: `code/tests/rl/test_tensor_profiles.py`

- [ ] **Step 1: Write failing default-profile tests**

In `code/tests/rl/test_tensor_profiles.py`, add:

```python
def test_default_observation_profile_is_standard_lite_v2(monkeypatch):
    monkeypatch.delenv("DIGIMON_TENSOR_PROFILE", raising=False)
    monkeypatch.setenv("DIGIMON_BACKEND", "rust")

    from digimon_gym.digimon_gym import DigimonEnv

    env = DigimonEnv()
    obs, info = env.reset(seed=1)

    assert env.tensor_profile == "standard_lite_v2"
    assert obs.shape == (8320,)
    assert info["tensor_profile"] == "standard_lite_v2"
```

- [ ] **Step 2: Run default-profile test and verify it fails**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profiles.py::test_default_observation_profile_is_standard_lite_v2 -v
```

Expected: FAIL because compact v1 remains default.

- [ ] **Step 3: Switch Rust default observation profile**

In `code/digimon-engine/src/observation.rs`, change:

```rust
pub fn default_observation_profile() -> ObservationProfileId {
    ObservationProfileId::StandardLiteV2
}
```

Do not change `tensor::TENSOR_SIZE`; it remains the compact v1 compatibility constant for legacy imports.

- [ ] **Step 4: Switch Python default**

In `code/digimon_gym/digimon_gym.py`, change the fallback default:

```python
self.tensor_profile = tensor_profile or os.environ.get("DIGIMON_TENSOR_PROFILE") or "standard_lite_v2"
```

In `configs/training/default.yaml`, add:

```yaml
tensor_profile: standard_lite_v2
```

- [ ] **Step 5: Run default and explicit compact env tests**

Run:

```powershell
$env:DIGIMON_BACKEND='rust'
python -m pytest code/tests/rl/test_tensor_profiles.py code/tests/rl/test_rust_runner_adapter.py -v
```

Expected: PASS. Tests that explicitly request compact should still get compact size; default should get v2 size.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-engine/src/observation.rs code/digimon-engine-py/src/lib.rs code/digimon_gym/digimon_gym.py configs/training/default.yaml code/tests/rl/test_tensor_profiles.py
git commit -m "feat: make standard lite v2 the default pilot observation"
```

---

### Task 11: Docs And Final Verification

**Files:**
- Modify: `docs/TENSOR_SPEC.md`
- Modify: `docs/ACTION_SPEC.md` only if action metadata cross-reference changed
- Modify: `docs/TOOLS.md`
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md`
- Modify: `docs/superpowers/specs/2026-05-01-observation-profile-registry-design.md`

- [ ] **Step 1: Update docs**

In `docs/TENSOR_SPEC.md`, add a `standard_lite_v2` section that includes:

```markdown
## `standard_lite_v2`

`standard_lite_v2` is the default pilot observation profile. It is a fair-information `8320`-float tensor with these top-level sections:

| Section | Offset | Shape | Size |
|---|---:|---:|---:|
| `global_features` | `0` | `[64]` | `64` |
| `player_summary` | `64` | `[2][32]` | `64` |
| `permanent_slots` | `128` | `[2][15][96]` | `2880` |
| `own_hand` | `3008` | `[30][32]` | `960` |
| `known_zone_cards` | `3968` | `[120][8]` | `960` |
| `decision_context` | `4928` | `[64]` | `64` |
| `pending_choice_features` | `4992` | `[32][96]` | `3072` |
| `reserved` | `8064` | `[256]` | `256` |
```

Add:

```markdown
Card ID positions total `542`; scalar positions total `7778`. The two lists are exported by `digimon_engine.get_observation_layout("standard_lite_v2")`.
```

Update `docs/TOOLS.md` training examples:

```bash
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training --tensor-profile standard_lite_v2
```

- [ ] **Step 2: Run Rust verification**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile tensor_profile_v2 tensor_v2_lite -- --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test infra observation_profile -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Rebuild PyO3 and run Python verification**

Run:

```powershell
Push-Location code/digimon-engine-py
maturin develop
Pop-Location
$env:DIGIMON_BACKEND='rust'
python -m pytest code/tests/test_rust_bindings_surface.py::TestTensorProfiles code/tests/rl/test_tensor_profiles.py code/tests/rl/test_rust_runner_adapter.py -v
```

Expected: PASS.

- [ ] **Step 4: Run smoke checks**

Run:

```powershell
$env:DIGIMON_BACKEND='rust'
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(tensor_profile='standard_lite_v2'); obs,info=env.reset(seed=1); print(obs.shape, info['tensor_profile'], info['action_mask'].shape)"
```

Expected:

```text
(8320,) standard_lite_v2 (2168,)
```

Run compact compatibility:

```powershell
$env:DIGIMON_BACKEND='rust'
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(tensor_profile='standard_compact_v1'); obs,info=env.reset(seed=1); print(obs.shape, info['tensor_profile'])"
```

Expected:

```text
(1375,) standard_compact_v1
```

- [ ] **Step 5: Search for stale assumptions**

Run:

```powershell
Select-String -Path code\\digimon_gym\\**\\*.py,code\\tools\\*.py,docs\\*.md,docs\\superpowers\\specs\\*.md -Pattern 'shape=\\(TENSOR_SIZE','1375-float observation','v2_lite','standard_v1' | Select-Object Path,LineNumber,Line
```

Expected: Any remaining compact references are explicitly compact-profile compatibility notes. New v2 public examples use `standard_lite_v2`.

- [ ] **Step 6: Commit**

```powershell
git add docs/TENSOR_SPEC.md docs/ACTION_SPEC.md docs/TOOLS.md docs/RUST_ENGINE_API.md docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md docs/superpowers/specs/2026-05-01-observation-profile-registry-design.md
git commit -m "docs: document standard lite v2 observation profile"
```

---

## Self-Review

- Spec coverage: The plan covers profile registry/layout metadata, `standard_lite_v2` shape and positions, fair-information tensor writing, unified permanent slots with breeding slot `14`, pending-choice rows, PyO3 layout export, `DigimonEnv` selection, feature extraction, training metadata, ONNX export metadata, default-profile switch, and docs. It leaves `standard_full_v2` as a later profiling experiment, matching the spec's "do not implement full first" guidance.
- Placeholder scan: The plan uses concrete file paths, test names, IDs, offsets, code snippets, commands, and expected outputs. The only generated value is the deterministic layout hash, with an exact computation procedure and required replacement before commit.
- Type consistency: Public profile ID is `standard_lite_v2`; compact baseline is `standard_compact_v1`; Python exposes `TensorProfile.layout_hash`, `feature_schema_version`, and `tensor_version`; Rust dispatch uses `ObservationProfileId`.


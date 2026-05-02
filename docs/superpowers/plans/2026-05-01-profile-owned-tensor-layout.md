# Profile-Owned Tensor Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the board tensor profile registry so `standard/v1.rs` owns the Standard v1 tensor layout constants, ranges, slot metadata, and derived card/scalar positions.

**Architecture:** Replace the single `tensor_profile.rs` metadata catalog with a `tensor_profiles/<game_mode>/<version>.rs` module tree. `tensor_profiles::standard::v1` becomes the source of truth for Standard v1 layout constants; `tensor.rs` remains the Standard v1 tensor writer and re-exports those constants for compatibility. PyO3, RL, Tauri, and docs should prefer `tensor_profiles` while `tensor_profile` remains as a compatibility alias.

**Tech Stack:** Rust `digimon-engine`, PyO3 `digimon-engine-py`, Python `digimon_gym`, Tauri v2 Rust commands, React TypeScript DTO tests, Cargo tests, Pytest.

---

## Scope Check

This is a refactor of ownership and module layout only. It must not change:

- `TENSOR_SIZE = 1375`
- `ACTION_SPACE_SIZE = 2168`
- the values written by `build_tensor()`
- card registry index semantics
- legal action masks
- current default profile id `standard_v1`
- card/scalar position counts `520` and `855`

Do not add EDH or Titan profiles in this plan. The new folder shape makes those future profiles straightforward, but this implementation registers only Standard v1.

## File Structure

### Rust Engine

- Create: `code/digimon-engine/src/tensor_profiles/mod.rs`
  - Owns shared profile structs, enums, registry functions, and compatibility constants.
- Create: `code/digimon-engine/src/tensor_profiles/standard/mod.rs`
  - Owns the Standard profile family.
- Create: `code/digimon-engine/src/tensor_profiles/standard/v1.rs`
  - Owns Standard v1 layout constants, section ranges, slot fields, slot layout, counts, and `PROFILE`.
- Modify: `code/digimon-engine/src/lib.rs`
  - Export `tensor_profiles` and alias it as `tensor_profile`.
- Delete: `code/digimon-engine/src/tensor_profile.rs`
  - Replaced by the `lib.rs` compatibility alias.
- Modify: `code/digimon-engine/src/tensor.rs`
  - Re-export Standard v1 layout constants from `tensor_profiles::standard::v1`.
  - Use `tensor_profiles` for slot metadata and `compute_positions()`.
- Modify: `code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs`
  - Verify the new module tree, `game_mode`, re-export parity, and position coverage.

### PyO3 And Python RL

- Modify: `code/digimon-engine-py/src/lib.rs`
  - Import from `digimon_engine::tensor_profiles`.
  - Add `game_mode` to Python `TensorProfile`.
- Modify: `code/tests/test_rust_bindings_surface.py`
  - Assert `get_tensor_profile().game_mode == "standard"`.
- Modify: `code/digimon_gym/tensor_profiles.py`
  - Add `game_mode` to the Python dataclass and fallback.
- Modify: `code/tests/rl/test_tensor_profiles.py`
  - Assert the Python profile adapter exposes `game_mode`.

### Tauri, Frontend, Docs

- Modify: `code/src-tauri/src/engine_commands.rs`
  - Prefer `digimon_engine::tensor_profiles::default_profile`.
- Modify: `docs/TENSOR_SPEC.md`
  - Document `tensor_profiles/<game_mode>/<version>.rs` as the canonical profile home.
- Modify: `docs/RUST_ENGINE_API.md`
  - Update imports/examples from `tensor_profile` to `tensor_profiles`, while noting compatibility.
- Modify: `docs/TOOLS.md`
  - Update any profile registry path references.
- Modify: `docs/superpowers/plans/2026-05-01-profile-registry-board-tensors.md`
  - Add a short note that the implementation was superseded by profile-owned layout modules.

---

### Task 1: Add Failing Rust Tests For Profile Ownership

**Files:**
- Modify: `code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs`

- [ ] **Step 1: Replace the profile test file with ownership-focused tests**

Replace `code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs` with:

```rust
use digimon_engine::tensor::{
    compute_positions, FIELD_SLOTS, GLOBAL_SIZE, HAND_SIZE, MAX_SOURCES, OFF_GLOBAL, OFF_MY_BATTLE,
    OFF_MY_BREEDING, OFF_MY_HAND, OFF_MY_SECURITY, OFF_MY_TRASH, OFF_OPP_BATTLE, OFF_OPP_BREEDING,
    OFF_OPP_HAND, OFF_OPP_SECURITY, OFF_OPP_TRASH, OFF_REVEALED, OFF_SELECTION, REVEALED_SIZE,
    SECURITY_SIZE, SELECTION_SIZE, SLOT_DP_OFFSET, SLOT_HEADER_SIZE, SLOT_LINKED_COUNT_OFFSET,
    SLOT_OPT_TOTAL_OFFSET, SLOT_OPT_USED_OFFSET, SLOT_SIZE, SLOT_SOURCE_COUNT_OFFSET,
    SLOT_SOURCE_START_OFFSET, SLOT_SUSPENDED_OFFSET, SLOT_TOP_CARD_OFFSET, SOURCE_CARD_ID_OFFSET,
    SOURCE_DP_CONTRIBUTION_OFFSET, SOURCE_ENTRY_SIZE, SOURCE_OPT_STATE_OFFSET, TENSOR_SIZE,
    TRASH_SIZE,
};
use digimon_engine::tensor_profiles::standard;
use digimon_engine::tensor_profiles::{
    all_profile_ids, default_profile, profile_by_id, TensorFieldKind, TensorSectionKind,
    STANDARD_V1_PROFILE_ID,
};

#[test]
fn default_profile_is_standard_v1() {
    let profile = default_profile();

    assert_eq!(profile.id, STANDARD_V1_PROFILE_ID);
    assert_eq!(profile.game_mode, "standard");
    assert_eq!(profile.version, 1);
    assert_eq!(profile.tensor_size, TENSOR_SIZE);
    assert_eq!(profile.field_slots, FIELD_SLOTS);
    assert_eq!(profile.slot_size, SLOT_SIZE);
    assert_eq!(profile.max_sources, MAX_SOURCES);
    assert_eq!(profile.slot_layout.size, SLOT_SIZE);
    assert_eq!(profile.slot_layout.source_entry_size, SOURCE_ENTRY_SIZE);
    assert_eq!(profile.card_id_slot_count, 520);
    assert_eq!(profile.scalar_slot_count, 855);
}

#[test]
fn registry_resolves_standard_profile_by_id() {
    let ids = all_profile_ids();
    assert_eq!(ids, vec![STANDARD_V1_PROFILE_ID]);

    let profile = profile_by_id(STANDARD_V1_PROFILE_ID).unwrap();
    assert_eq!(profile.id, "standard_v1");
    assert_eq!(profile.game_mode, "standard");
    assert!(profile_by_id("missing_profile").is_none());
}

#[test]
fn standard_family_resolves_profile_by_version() {
    let profile = standard::profile_by_version(1).unwrap();

    assert_eq!(standard::DEFAULT_PROFILE, standard::v1::PROFILE);
    assert_eq!(profile, standard::v1::PROFILE);
    assert!(standard::profile_by_version(2).is_none());
}

#[test]
fn standard_v1_owns_tensor_layout_constants() {
    assert_eq!(standard::v1::PROFILE.tensor_size, standard::v1::TENSOR_SIZE);
    assert_eq!(standard::v1::PROFILE.field_slots, standard::v1::FIELD_SLOTS);
    assert_eq!(standard::v1::PROFILE.slot_size, standard::v1::SLOT_SIZE);
    assert_eq!(standard::v1::PROFILE.max_sources, standard::v1::MAX_SOURCES);

    assert_eq!(TENSOR_SIZE, standard::v1::TENSOR_SIZE);
    assert_eq!(FIELD_SLOTS, standard::v1::FIELD_SLOTS);
    assert_eq!(SLOT_SIZE, standard::v1::SLOT_SIZE);
    assert_eq!(MAX_SOURCES, standard::v1::MAX_SOURCES);
}

#[test]
fn standard_profile_sections_cover_tensor_without_overlap() {
    let profile = default_profile();
    let mut covered = Vec::new();

    for section in profile.sections {
        assert!(section.start + section.len <= profile.tensor_size);
        if section.kind == TensorSectionKind::PermanentSlots {
            assert_eq!(section.len % profile.slot_layout.size, 0);
        }
        covered.extend(section.start..section.start + section.len);
    }

    covered.sort();
    assert_eq!(covered.len(), profile.tensor_size);
    covered.dedup();
    assert_eq!(covered.len(), profile.tensor_size);
    assert_eq!(covered[0], 0);
    assert_eq!(*covered.last().unwrap(), profile.tensor_size - 1);
}

#[test]
fn standard_profile_card_and_scalar_positions_match_tensor_module() {
    let profile = default_profile();
    let (profile_cards, profile_scalars) = profile.positions();
    let (v1_cards, v1_scalars) = standard::v1::PROFILE.positions();
    let (tensor_cards, tensor_scalars) = compute_positions();

    assert_eq!(profile_cards, v1_cards);
    assert_eq!(profile_scalars, v1_scalars);
    assert_eq!(profile_cards, tensor_cards);
    assert_eq!(profile_scalars, tensor_scalars);
    assert_eq!(profile_cards.len(), profile.card_id_slot_count);
    assert_eq!(profile_scalars.len(), profile.scalar_slot_count);

    let card_set: std::collections::BTreeSet<_> = profile_cards.iter().copied().collect();
    let scalar_set: std::collections::BTreeSet<_> = profile_scalars.iter().copied().collect();
    assert!(card_set.is_disjoint(&scalar_set));
    assert_eq!(card_set.len() + scalar_set.len(), profile.tensor_size);
}

#[test]
fn standard_profile_marks_card_sections() {
    let profile = default_profile();

    let hand = profile.section("my_hand").unwrap();
    assert_eq!(hand.kind, TensorSectionKind::CardIds);
    assert_eq!(hand.start, 1130);
    assert_eq!(hand.len, 20);

    let battle = profile.section("my_battle").unwrap();
    assert_eq!(battle.kind, TensorSectionKind::PermanentSlots);
    assert_eq!(battle.start, 10);
    assert_eq!(battle.len, 560);

    let global = profile.section("global").unwrap();
    assert_eq!(global.kind, TensorSectionKind::Scalars);
    assert_eq!(global.start, 0);
    assert_eq!(global.len, 10);
}

#[test]
fn standard_profile_sections_match_tensor_layout_constants() {
    let profile = default_profile();

    assert_eq!(
        (
            profile.section("global").unwrap().start,
            profile.section("global").unwrap().len
        ),
        (OFF_GLOBAL, GLOBAL_SIZE)
    );
    assert_eq!(
        (
            profile.section("my_battle").unwrap().start,
            profile.section("my_battle").unwrap().len,
        ),
        (OFF_MY_BATTLE, FIELD_SLOTS * SLOT_SIZE)
    );
    assert_eq!(
        (
            profile.section("opponent_battle").unwrap().start,
            profile.section("opponent_battle").unwrap().len,
        ),
        (OFF_OPP_BATTLE, FIELD_SLOTS * SLOT_SIZE)
    );
    assert_eq!(
        (
            profile.section("my_hand").unwrap().start,
            profile.section("my_hand").unwrap().len
        ),
        (OFF_MY_HAND, HAND_SIZE)
    );
    assert_eq!(
        (
            profile.section("opponent_hand").unwrap().start,
            profile.section("opponent_hand").unwrap().len,
        ),
        (OFF_OPP_HAND, HAND_SIZE)
    );
    assert_eq!(
        (
            profile.section("my_trash").unwrap().start,
            profile.section("my_trash").unwrap().len
        ),
        (OFF_MY_TRASH, TRASH_SIZE)
    );
    assert_eq!(
        (
            profile.section("opponent_trash").unwrap().start,
            profile.section("opponent_trash").unwrap().len,
        ),
        (OFF_OPP_TRASH, TRASH_SIZE)
    );
    assert_eq!(
        (
            profile.section("my_security").unwrap().start,
            profile.section("my_security").unwrap().len,
        ),
        (OFF_MY_SECURITY, SECURITY_SIZE)
    );
    assert_eq!(
        (
            profile.section("opponent_security").unwrap().start,
            profile.section("opponent_security").unwrap().len,
        ),
        (OFF_OPP_SECURITY, SECURITY_SIZE)
    );
    assert_eq!(
        (
            profile.section("my_breeding").unwrap().start,
            profile.section("my_breeding").unwrap().len,
        ),
        (OFF_MY_BREEDING, SLOT_SIZE)
    );
    assert_eq!(
        (
            profile.section("opponent_breeding").unwrap().start,
            profile.section("opponent_breeding").unwrap().len,
        ),
        (OFF_OPP_BREEDING, SLOT_SIZE)
    );
    assert_eq!(
        (
            profile.section("revealed").unwrap().start,
            profile.section("revealed").unwrap().len
        ),
        (OFF_REVEALED, REVEALED_SIZE)
    );
    assert_eq!(
        (
            profile.section("selection").unwrap().start,
            profile.section("selection").unwrap().len,
        ),
        (OFF_SELECTION, SELECTION_SIZE)
    );

    assert_eq!(
        SLOT_HEADER_SIZE + SOURCE_ENTRY_SIZE * MAX_SOURCES,
        SLOT_SIZE
    );
}

#[test]
fn standard_profile_slot_field_offsets_match_tensor_layout_constants() {
    assert_eq!(SLOT_TOP_CARD_OFFSET, 0);
    assert_eq!(SLOT_DP_OFFSET, 1);
    assert_eq!(SLOT_SUSPENDED_OFFSET, 2);
    assert_eq!(SLOT_OPT_TOTAL_OFFSET, 3);
    assert_eq!(SLOT_OPT_USED_OFFSET, 4);
    assert_eq!(SLOT_LINKED_COUNT_OFFSET, 5);
    assert_eq!(SLOT_SOURCE_COUNT_OFFSET, 6);
    assert_eq!(SLOT_SOURCE_START_OFFSET, SLOT_HEADER_SIZE);
    assert_eq!(
        SLOT_SIZE,
        SLOT_SOURCE_START_OFFSET + MAX_SOURCES * SOURCE_ENTRY_SIZE
    );

    assert_eq!(SOURCE_CARD_ID_OFFSET, 0);
    assert_eq!(SOURCE_OPT_STATE_OFFSET, SOURCE_CARD_ID_OFFSET + 1);
    assert_eq!(SOURCE_DP_CONTRIBUTION_OFFSET, SOURCE_OPT_STATE_OFFSET + 1);
    assert!(SOURCE_DP_CONTRIBUTION_OFFSET < SOURCE_ENTRY_SIZE);
}

#[test]
fn standard_profile_slot_layout_is_auditable_metadata() {
    let profile = default_profile();

    assert_eq!(profile.max_sources, MAX_SOURCES);
    assert_eq!(profile.slot_layout.size, SLOT_SIZE);
    assert_eq!(profile.slot_layout.source_start, SLOT_SOURCE_START_OFFSET);
    assert_eq!(profile.slot_layout.source_entry_size, SOURCE_ENTRY_SIZE);
    assert_eq!(profile.slot_layout.max_sources, MAX_SOURCES);

    let header_fields: Vec<_> = profile
        .slot_layout
        .header_fields
        .iter()
        .map(|field| (field.id, field.offset, field.kind))
        .collect();
    assert_eq!(
        header_fields,
        vec![
            ("top_card_id", SLOT_TOP_CARD_OFFSET, TensorFieldKind::CardId),
            ("dp", SLOT_DP_OFFSET, TensorFieldKind::Scalar),
            ("suspended", SLOT_SUSPENDED_OFFSET, TensorFieldKind::Scalar),
            ("opt_total", SLOT_OPT_TOTAL_OFFSET, TensorFieldKind::Scalar),
            ("opt_used", SLOT_OPT_USED_OFFSET, TensorFieldKind::Scalar),
            (
                "linked_count",
                SLOT_LINKED_COUNT_OFFSET,
                TensorFieldKind::Scalar,
            ),
            (
                "source_count",
                SLOT_SOURCE_COUNT_OFFSET,
                TensorFieldKind::Scalar,
            ),
        ]
    );

    let source_fields: Vec<_> = profile
        .slot_layout
        .source_fields
        .iter()
        .map(|field| (field.id, field.offset, field.kind))
        .collect();
    assert_eq!(
        source_fields,
        vec![
            ("card_id", SOURCE_CARD_ID_OFFSET, TensorFieldKind::CardId),
            (
                "opt_state",
                SOURCE_OPT_STATE_OFFSET,
                TensorFieldKind::Scalar,
            ),
            (
                "dp_contribution",
                SOURCE_DP_CONTRIBUTION_OFFSET,
                TensorFieldKind::Scalar,
            ),
        ]
    );
}

#[test]
fn singular_tensor_profile_alias_still_works() {
    let singular = digimon_engine::tensor_profile::default_profile();
    let plural = digimon_engine::tensor_profiles::default_profile();

    assert_eq!(singular, plural);
    assert_eq!(digimon_engine::tensor_profile::STANDARD_V1_PROFILE_ID, "standard_v1");
}
```

- [ ] **Step 2: Run the new tests and verify they fail**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor tensor_profile -- --nocapture
```

Expected: FAIL with unresolved import errors for `digimon_engine::tensor_profiles` and missing `game_mode`.

- [ ] **Step 3: Commit the failing tests**

```powershell
git add code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs
git commit -m "test: require profile-owned tensor layout"
```

---

### Task 2: Create The `tensor_profiles` Module Tree

**Files:**
- Create: `code/digimon-engine/src/tensor_profiles/mod.rs`
- Create: `code/digimon-engine/src/tensor_profiles/standard/mod.rs`
- Create: `code/digimon-engine/src/tensor_profiles/standard/v1.rs`
- Modify: `code/digimon-engine/src/lib.rs`
- Delete: `code/digimon-engine/src/tensor_profile.rs`

- [ ] **Step 1: Add the shared registry module**

Create `code/digimon-engine/src/tensor_profiles/mod.rs`:

```rust
//! Registry metadata for observation tensor layouts.

pub mod standard;

pub const STANDARD_V1_PROFILE_ID: &str = standard::v1::PROFILE_ID;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TensorSectionKind {
    Scalars,
    CardIds,
    PermanentSlots,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorSection {
    pub id: &'static str,
    pub start: usize,
    pub len: usize,
    pub kind: TensorSectionKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TensorFieldKind {
    CardId,
    Scalar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorSlotField {
    pub id: &'static str,
    pub offset: usize,
    pub kind: TensorFieldKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorSlotLayout {
    pub size: usize,
    pub source_start: usize,
    pub source_entry_size: usize,
    pub max_sources: usize,
    pub header_fields: &'static [TensorSlotField],
    pub source_fields: &'static [TensorSlotField],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TensorSlotHeaderField {
    TopCardId,
    Dp,
    Suspended,
    OptTotal,
    OptUsed,
    LinkedCount,
    SourceCount,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TensorSourceField {
    CardId,
    OptState,
    DpContribution,
}

impl TensorSlotLayout {
    pub fn header_offset(&self, field: TensorSlotHeaderField) -> usize {
        self.header_fields[field as usize].offset
    }

    pub fn source_offset(&self, field: TensorSourceField) -> usize {
        self.source_fields[field as usize].offset
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorProfile {
    pub id: &'static str,
    pub game_mode: &'static str,
    pub version: u32,
    pub tensor_size: usize,
    pub field_slots: usize,
    pub slot_size: usize,
    pub max_sources: usize,
    pub slot_layout: TensorSlotLayout,
    pub card_id_slot_count: usize,
    pub scalar_slot_count: usize,
    pub sections: &'static [TensorSection],
}

impl TensorProfile {
    pub fn section(&self, id: &str) -> Option<&'static TensorSection> {
        self.sections.iter().find(|section| section.id == id)
    }

    pub fn positions(&self) -> (Vec<usize>, Vec<usize>) {
        let mut card_positions = Vec::with_capacity(self.card_id_slot_count);
        let mut scalar_positions = Vec::with_capacity(self.scalar_slot_count);

        for section in self.sections {
            match section.kind {
                TensorSectionKind::Scalars => {
                    scalar_positions.extend(section.start..section.start + section.len);
                }
                TensorSectionKind::CardIds => {
                    card_positions.extend(section.start..section.start + section.len);
                }
                TensorSectionKind::PermanentSlots => {
                    for slot_base in
                        (section.start..section.start + section.len).step_by(self.slot_layout.size)
                    {
                        permanent_slot_positions(
                            self.slot_layout,
                            slot_base,
                            &mut card_positions,
                            &mut scalar_positions,
                        );
                    }
                }
            }
        }

        card_positions.sort();
        scalar_positions.sort();
        (card_positions, scalar_positions)
    }
}

pub fn default_profile() -> TensorProfile {
    standard::DEFAULT_PROFILE
}

pub fn all_profile_ids() -> Vec<&'static str> {
    vec![standard::v1::PROFILE_ID]
}

pub fn profile_by_id(id: &str) -> Option<TensorProfile> {
    match id {
        standard::v1::PROFILE_ID => Some(standard::v1::PROFILE),
        _ => None,
    }
}

pub fn standard_v1_positions() -> (Vec<usize>, Vec<usize>) {
    standard::v1::PROFILE.positions()
}

fn permanent_slot_positions(
    layout: TensorSlotLayout,
    slot_base: usize,
    card_positions: &mut Vec<usize>,
    scalar_positions: &mut Vec<usize>,
) {
    for field in layout.header_fields {
        match field.kind {
            TensorFieldKind::CardId => card_positions.push(slot_base + field.offset),
            TensorFieldKind::Scalar => scalar_positions.push(slot_base + field.offset),
        }
    }

    let source_base = slot_base + layout.source_start;
    for source_index in 0..layout.max_sources {
        let source_offset = source_base + source_index * layout.source_entry_size;
        for field in layout.source_fields {
            match field.kind {
                TensorFieldKind::CardId => card_positions.push(source_offset + field.offset),
                TensorFieldKind::Scalar => scalar_positions.push(source_offset + field.offset),
            }
        }
    }
}
```

- [ ] **Step 2: Add the Standard profile family module**

Create `code/digimon-engine/src/tensor_profiles/standard/mod.rs`:

```rust
use crate::tensor_profiles::TensorProfile;

pub mod v1;

pub const DEFAULT_PROFILE: TensorProfile = v1::PROFILE;

pub fn profile_by_version(version: u32) -> Option<TensorProfile> {
    match version {
        1 => Some(v1::PROFILE),
        _ => None,
    }
}
```

- [ ] **Step 3: Add the Standard v1 owned profile**

Create `code/digimon-engine/src/tensor_profiles/standard/v1.rs`:

```rust
use crate::tensor_profiles::{
    TensorFieldKind, TensorProfile, TensorSection, TensorSectionKind, TensorSlotField,
    TensorSlotLayout,
};

pub const PROFILE_ID: &str = "standard_v1";
pub const GAME_MODE: &str = "standard";
pub const VERSION: u32 = 1;

pub const FIELD_SLOTS: usize = 14;
pub const MAX_HAND: usize = 20;
pub const MAX_TRASH: usize = 45;
pub const MAX_SECURITY: usize = 10;
pub const MAX_SOURCES: usize = 11;
pub const MAX_REVEALED: usize = 10;

pub const SOURCE_ENTRY_SIZE: usize = 3;
pub const SLOT_TOP_CARD_OFFSET: usize = 0;
pub const SLOT_DP_OFFSET: usize = 1;
pub const SLOT_SUSPENDED_OFFSET: usize = 2;
pub const SLOT_OPT_TOTAL_OFFSET: usize = 3;
pub const SLOT_OPT_USED_OFFSET: usize = 4;
pub const SLOT_LINKED_COUNT_OFFSET: usize = 5;
pub const SLOT_SOURCE_COUNT_OFFSET: usize = 6;
pub const SLOT_SOURCE_START_OFFSET: usize = 7;
pub const SLOT_HEADER_SIZE: usize = SLOT_SOURCE_START_OFFSET;
pub const SLOT_SIZE: usize = SLOT_HEADER_SIZE + MAX_SOURCES * SOURCE_ENTRY_SIZE;
pub const SOURCE_CARD_ID_OFFSET: usize = 0;
pub const SOURCE_OPT_STATE_OFFSET: usize = 1;
pub const SOURCE_DP_CONTRIBUTION_OFFSET: usize = 2;

pub const GLOBAL_SIZE: usize = 10;
pub const BATTLE_SIZE: usize = FIELD_SLOTS * SLOT_SIZE;
pub const HAND_SIZE: usize = MAX_HAND;
pub const TRASH_SIZE: usize = MAX_TRASH;
pub const SECURITY_SIZE: usize = MAX_SECURITY;
pub const BREEDING_SIZE: usize = SLOT_SIZE;
pub const REVEALED_SIZE: usize = MAX_REVEALED;
pub const SELECTION_SIZE: usize = 5;

pub const OFF_GLOBAL: usize = 0;
pub const OFF_MY_BATTLE: usize = OFF_GLOBAL + GLOBAL_SIZE;
pub const OFF_OPP_BATTLE: usize = OFF_MY_BATTLE + BATTLE_SIZE;
pub const OFF_MY_HAND: usize = OFF_OPP_BATTLE + BATTLE_SIZE;
pub const OFF_OPP_HAND: usize = OFF_MY_HAND + HAND_SIZE;
pub const OFF_MY_TRASH: usize = OFF_OPP_HAND + HAND_SIZE;
pub const OFF_OPP_TRASH: usize = OFF_MY_TRASH + TRASH_SIZE;
pub const OFF_MY_SECURITY: usize = OFF_OPP_TRASH + TRASH_SIZE;
pub const OFF_OPP_SECURITY: usize = OFF_MY_SECURITY + SECURITY_SIZE;
pub const OFF_MY_BREEDING: usize = OFF_OPP_SECURITY + SECURITY_SIZE;
pub const OFF_OPP_BREEDING: usize = OFF_MY_BREEDING + BREEDING_SIZE;
pub const OFF_REVEALED: usize = OFF_OPP_BREEDING + BREEDING_SIZE;
pub const OFF_SELECTION: usize = OFF_REVEALED + REVEALED_SIZE;

pub const TENSOR_SIZE: usize = OFF_SELECTION + SELECTION_SIZE;

pub const SECTIONS: &[TensorSection] = &[
    TensorSection {
        id: "global",
        start: OFF_GLOBAL,
        len: GLOBAL_SIZE,
        kind: TensorSectionKind::Scalars,
    },
    TensorSection {
        id: "my_battle",
        start: OFF_MY_BATTLE,
        len: BATTLE_SIZE,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "opponent_battle",
        start: OFF_OPP_BATTLE,
        len: BATTLE_SIZE,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "my_hand",
        start: OFF_MY_HAND,
        len: HAND_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "opponent_hand",
        start: OFF_OPP_HAND,
        len: HAND_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "my_trash",
        start: OFF_MY_TRASH,
        len: TRASH_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "opponent_trash",
        start: OFF_OPP_TRASH,
        len: TRASH_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "my_security",
        start: OFF_MY_SECURITY,
        len: SECURITY_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "opponent_security",
        start: OFF_OPP_SECURITY,
        len: SECURITY_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "my_breeding",
        start: OFF_MY_BREEDING,
        len: BREEDING_SIZE,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "opponent_breeding",
        start: OFF_OPP_BREEDING,
        len: BREEDING_SIZE,
        kind: TensorSectionKind::PermanentSlots,
    },
    TensorSection {
        id: "revealed",
        start: OFF_REVEALED,
        len: REVEALED_SIZE,
        kind: TensorSectionKind::CardIds,
    },
    TensorSection {
        id: "selection",
        start: OFF_SELECTION,
        len: SELECTION_SIZE,
        kind: TensorSectionKind::Scalars,
    },
];

pub const SLOT_HEADER_FIELDS: &[TensorSlotField] = &[
    TensorSlotField {
        id: "top_card_id",
        offset: SLOT_TOP_CARD_OFFSET,
        kind: TensorFieldKind::CardId,
    },
    TensorSlotField {
        id: "dp",
        offset: SLOT_DP_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "suspended",
        offset: SLOT_SUSPENDED_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "opt_total",
        offset: SLOT_OPT_TOTAL_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "opt_used",
        offset: SLOT_OPT_USED_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "linked_count",
        offset: SLOT_LINKED_COUNT_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "source_count",
        offset: SLOT_SOURCE_COUNT_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
];

pub const SOURCE_FIELDS: &[TensorSlotField] = &[
    TensorSlotField {
        id: "card_id",
        offset: SOURCE_CARD_ID_OFFSET,
        kind: TensorFieldKind::CardId,
    },
    TensorSlotField {
        id: "opt_state",
        offset: SOURCE_OPT_STATE_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
    TensorSlotField {
        id: "dp_contribution",
        offset: SOURCE_DP_CONTRIBUTION_OFFSET,
        kind: TensorFieldKind::Scalar,
    },
];

pub const SLOT_LAYOUT: TensorSlotLayout = TensorSlotLayout {
    size: SLOT_SIZE,
    source_start: SLOT_SOURCE_START_OFFSET,
    source_entry_size: SOURCE_ENTRY_SIZE,
    max_sources: MAX_SOURCES,
    header_fields: SLOT_HEADER_FIELDS,
    source_fields: SOURCE_FIELDS,
};

pub const PERMANENT_SLOT_CARD_ID_COUNT: usize = 1 + MAX_SOURCES;
pub const PERMANENT_SLOT_SCALAR_COUNT: usize =
    SLOT_HEADER_SIZE - 1 + MAX_SOURCES * (SOURCE_ENTRY_SIZE - 1);
pub const PERMANENT_SLOT_COUNT: usize = FIELD_SLOTS * 2 + 2;
pub const CARD_ID_SLOT_COUNT: usize = PERMANENT_SLOT_COUNT * PERMANENT_SLOT_CARD_ID_COUNT
    + HAND_SIZE * 2
    + TRASH_SIZE * 2
    + SECURITY_SIZE * 2
    + REVEALED_SIZE;
pub const SCALAR_SLOT_COUNT: usize =
    PERMANENT_SLOT_COUNT * PERMANENT_SLOT_SCALAR_COUNT + GLOBAL_SIZE + SELECTION_SIZE;

pub const PROFILE: TensorProfile = TensorProfile {
    id: PROFILE_ID,
    game_mode: GAME_MODE,
    version: VERSION,
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

- [ ] **Step 4: Update the Rust crate exports**

In `code/digimon-engine/src/lib.rs`, replace:

```rust
pub mod tensor;
pub mod tensor_profile;
```

with:

```rust
pub mod tensor;
pub mod tensor_profiles;
pub use tensor_profiles as tensor_profile;
```

If the two lines are not adjacent, keep `pub mod tensor;` where it is, add `pub mod tensor_profiles;` next to it, and delete only `pub mod tensor_profile;`.

- [ ] **Step 5: Delete the old singular module file**

Delete:

```text
code/digimon-engine/src/tensor_profile.rs
```

Use `Remove-Item -LiteralPath code\digimon-engine\src\tensor_profile.rs` or an editor delete. This is safe only after `lib.rs` aliases `tensor_profiles` as `tensor_profile`.

- [ ] **Step 6: Run the Rust profile tests and inspect the next failures**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor tensor_profile -- --nocapture
```

Expected: still FAIL because `tensor.rs` imports `crate::tensor_profile` and still owns duplicate constants.

---

### Task 3: Make `tensor.rs` The Standard V1 Writer And Compatibility Surface

**Files:**
- Modify: `code/digimon-engine/src/tensor.rs`

- [ ] **Step 1: Replace the profile import**

In `code/digimon-engine/src/tensor.rs`, replace:

```rust
use crate::tensor_profile::{self, TensorSlotHeaderField, TensorSourceField};
```

with:

```rust
use crate::tensor_profiles::{self, TensorSlotHeaderField, TensorSourceField};
```

- [ ] **Step 2: Replace owned layout constants with re-exports**

In `code/digimon-engine/src/tensor.rs`, replace the entire `// ─── Tensor Layout Constants` block from `pub const FIELD_SLOTS` through `pub const OFF_SELECTION` with:

```rust
// ─── Standard V1 Tensor Layout Constants ──────────────────────────────
//
// `tensor_profiles::standard::v1` owns these values. This module re-exports
// them because it is the Standard v1 tensor writer and many callers still
// import the constants from `tensor`.

pub use crate::tensor_profiles::standard::v1::{
    BATTLE_SIZE, BREEDING_SIZE, FIELD_SLOTS, GLOBAL_SIZE, HAND_SIZE, MAX_HAND, MAX_REVEALED,
    MAX_SECURITY, MAX_SOURCES, MAX_TRASH, OFF_GLOBAL, OFF_MY_BATTLE, OFF_MY_BREEDING,
    OFF_MY_HAND, OFF_MY_SECURITY, OFF_MY_TRASH, OFF_OPP_BATTLE, OFF_OPP_BREEDING, OFF_OPP_HAND,
    OFF_OPP_SECURITY, OFF_OPP_TRASH, OFF_REVEALED, OFF_SELECTION, REVEALED_SIZE, SECURITY_SIZE,
    SELECTION_SIZE, SLOT_DP_OFFSET, SLOT_HEADER_SIZE, SLOT_LINKED_COUNT_OFFSET,
    SLOT_OPT_TOTAL_OFFSET, SLOT_OPT_USED_OFFSET, SLOT_SIZE, SLOT_SOURCE_COUNT_OFFSET,
    SLOT_SOURCE_START_OFFSET, SLOT_SUSPENDED_OFFSET, SLOT_TOP_CARD_OFFSET, SOURCE_CARD_ID_OFFSET,
    SOURCE_DP_CONTRIBUTION_OFFSET, SOURCE_ENTRY_SIZE, SOURCE_OPT_STATE_OFFSET, TENSOR_SIZE,
    TRASH_SIZE,
};

pub const DP_NORM: f32 = 30000.0;
```

Keep `DP_NORM` in `tensor.rs` because it is a value normalization constant used by the writer, not a layout range.

- [ ] **Step 3: Update `compute_positions()`**

Replace:

```rust
pub fn compute_positions() -> (Vec<usize>, Vec<usize>) {
    tensor_profile::standard_v1_positions()
}
```

with:

```rust
pub fn compute_positions() -> (Vec<usize>, Vec<usize>) {
    tensor_profiles::standard::v1::PROFILE.positions()
}
```

- [ ] **Step 4: Update slot metadata access**

In `write_slot()`, replace:

```rust
let slot_layout = tensor_profile::default_profile().slot_layout;
```

with:

```rust
let slot_layout = tensor_profiles::standard::v1::PROFILE.slot_layout;
```

This keeps the writer explicitly tied to Standard v1. Do not call a future global default from inside a writer that is not parameterized by profile.

- [ ] **Step 5: Run focused Rust tests**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor tensor_profile -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run tensor writer tests**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor tensor -- --nocapture
```

Expected: PASS. The tensor writer still emits the same Standard v1 layout.

- [ ] **Step 7: Commit the Rust module refactor**

```powershell
git add code/digimon-engine/src/lib.rs code/digimon-engine/src/tensor.rs code/digimon-engine/src/tensor_profiles code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs
git add -u code/digimon-engine/src/tensor_profile.rs
git commit -m "refactor: move tensor layout ownership into profiles"
```

---

### Task 4: Add `game_mode` To PyO3 And Python Profile Metadata

**Files:**
- Modify: `code/digimon-engine-py/src/lib.rs`
- Modify: `code/tests/test_rust_bindings_surface.py`
- Modify: `code/digimon_gym/tensor_profiles.py`
- Modify: `code/tests/rl/test_tensor_profiles.py`

- [ ] **Step 1: Add failing PyO3 tests for `game_mode`**

In `code/tests/test_rust_bindings_surface.py`, update `TestTensorProfiles.test_get_default_tensor_profile` by adding this assertion after `assert profile.id == "standard_v1"`:

```python
assert profile.game_mode == "standard"
```

- [ ] **Step 2: Add failing RL adapter tests for `game_mode`**

In `code/tests/rl/test_tensor_profiles.py`, update `test_default_tensor_profile_shape()` by adding this assertion after `assert profile.id == TENSOR_PROFILE_ID`:

```python
assert profile.game_mode == "standard"
```

Update `test_tensor_profile_falls_back_when_engine_function_missing()` by adding:

```python
assert profile.game_mode == "standard"
```

- [ ] **Step 3: Run PyO3 and RL tests to verify failure**

Run:

```powershell
python -m pytest code/tests/test_rust_bindings_surface.py::TestTensorProfiles -v
python -m pytest code/tests/rl/test_tensor_profiles.py -v
```

Expected: FAIL with `AttributeError` for missing `game_mode`.

- [ ] **Step 4: Update PyO3 imports**

In `code/digimon-engine-py/src/lib.rs`, replace:

```rust
use ::digimon_engine::tensor_profile::{
    all_profile_ids, default_profile, profile_by_id, TensorProfile as RustTensorProfile,
};
```

with:

```rust
use ::digimon_engine::tensor_profiles::{
    all_profile_ids, default_profile, profile_by_id, TensorProfile as RustTensorProfile,
};
```

- [ ] **Step 5: Add `game_mode` to the PyO3 `TensorProfile` class**

In `code/digimon-engine-py/src/lib.rs`, add this field after `pub id: String,` in the `#[pyclass]`:

```rust
#[pyo3(get)]
pub game_mode: String,
```

Update `py_tensor_profile()` so the returned struct includes:

```rust
game_mode: profile.game_mode.to_string(),
```

The full returned struct should look like:

```rust
TensorProfile {
    id: profile.id.to_string(),
    game_mode: profile.game_mode.to_string(),
    version: profile.version,
    tensor_size: profile.tensor_size,
    field_slots: profile.field_slots,
    slot_size: profile.slot_size,
    max_sources: MAX_SOURCES,
    card_id_slot_count: profile.card_id_slot_count,
    scalar_slot_count: profile.scalar_slot_count,
    card_id_positions,
    scalar_positions,
}
```

Keep the existing `TENSOR_PROFILE_ID` module constant unchanged:

```rust
m.add("TENSOR_PROFILE_ID", default_profile().id)?;
```

- [ ] **Step 6: Add `game_mode` to the Python adapter**

In `code/digimon_gym/tensor_profiles.py`, add `game_mode` to the dataclass after `id`:

```python
@dataclass(frozen=True)
class TensorProfile:
    id: str
    game_mode: str
    version: int
    tensor_size: int
    field_slots: int
    slot_size: int
    max_sources: int
    card_id_slot_count: int
    scalar_slot_count: int
    card_id_positions: tuple[int, ...]
    scalar_positions: tuple[int, ...]
```

In `get_tensor_profile()`, add:

```python
game_mode=raw.game_mode,
```

to the returned `TensorProfile(...)`.

In `_legacy_standard_v1()`, add:

```python
game_mode="standard",
```

to the returned `TensorProfile(...)`.

- [ ] **Step 7: Rebuild PyO3 bindings**

Run:

```powershell
cd code\digimon-engine-py
maturin develop
cd ..\..
```

Expected: build succeeds.

- [ ] **Step 8: Run profile metadata tests**

Run:

```powershell
python -m pytest code/tests/test_rust_bindings_surface.py::TestTensorProfiles -v
python -m pytest code/tests/rl/test_tensor_profiles.py -v
```

Expected: PASS.

- [ ] **Step 9: Commit PyO3 and Python metadata changes**

```powershell
git add code/digimon-engine-py/src/lib.rs code/digimon_gym/tensor_profiles.py code/tests/test_rust_bindings_surface.py code/tests/rl/test_tensor_profiles.py
git commit -m "feat: expose tensor profile game mode"
```

---

### Task 5: Update Tauri And Documentation References

**Files:**
- Modify: `code/src-tauri/src/engine_commands.rs`
- Modify: `docs/TENSOR_SPEC.md`
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `docs/TOOLS.md`
- Modify: `docs/superpowers/plans/2026-05-01-profile-registry-board-tensors.md`

- [ ] **Step 1: Update Tauri imports**

In `code/src-tauri/src/engine_commands.rs`, replace:

```rust
use digimon_engine::tensor_profile::default_profile;
```

with:

```rust
use digimon_engine::tensor_profiles::default_profile;
```

If the file imports `default_profile` inside a grouped `use` block, update only that path.

- [ ] **Step 2: Update `docs/TENSOR_SPEC.md` profile location text**

Replace the sentence:

```markdown
The profile registry lives in `code/digimon-engine/src/tensor_profile.rs`. A profile is metadata for describing and auditing the tensor layout; it does not change tensor writer values, legal action masks, or action IDs.
```

with:

```markdown
Canonical tensor profile definitions live under `code/digimon-engine/src/tensor_profiles/<game_mode>/<version>.rs`. The current profile is defined in `code/digimon-engine/src/tensor_profiles/standard/v1.rs`, which owns the Standard v1 tensor size, section ranges, slot shape, and derived card/scalar positions. `code/digimon-engine/src/tensor.rs` is the Standard v1 tensor writer and compatibility surface; it re-exports the current layout constants but does not own them.
```

In the "Tensor Layout Metadata" section, replace:

```markdown
Canonical tensor layout metadata lives in `code/digimon-engine/src/tensor_profile.rs`,
is exposed to Python by `digimon_engine.get_tensor_profile()`, and is consumed through
`digimon_gym.tensor_profiles.get_tensor_profile()`.
```

with:

```markdown
Canonical tensor layout metadata lives in `code/digimon-engine/src/tensor_profiles/standard/v1.rs`,
is exposed to Python by `digimon_engine.get_tensor_profile()`, and is consumed through
`digimon_gym.tensor_profiles.get_tensor_profile()`.
```

- [ ] **Step 3: Update `docs/RUST_ENGINE_API.md` examples**

Find the board tensor profile section and replace imports of:

```rust
use digimon_engine::tensor_profile::{
```

with:

```rust
use digimon_engine::tensor_profiles::{
```

Add this sentence after the Rust example:

```markdown
`digimon_engine::tensor_profile` remains as a temporary compatibility alias, but new code should use `digimon_engine::tensor_profiles`.
```

- [ ] **Step 4: Update `docs/TOOLS.md` path references**

Replace references to:

```markdown
code/digimon-engine/src/tensor_profile.rs
```

with:

```markdown
code/digimon-engine/src/tensor_profiles/standard/v1.rs
```

If the paragraph describes the registry rather than the current profile, use:

```markdown
code/digimon-engine/src/tensor_profiles/
```

- [ ] **Step 5: Mark the old implementation plan as superseded**

At the top of `docs/superpowers/plans/2026-05-01-profile-registry-board-tensors.md`, after the main heading, add:

```markdown
> Superseded follow-up: `docs/superpowers/plans/2026-05-01-profile-owned-tensor-layout.md` refactors this registry so profile modules own the layout constants under `tensor_profiles/<game_mode>/<version>.rs`.
```

- [ ] **Step 6: Run focused Tauri and doc checks**

Run:

```powershell
cargo test -p digimon-tcg tensor_summary -- --nocapture
git diff --check
```

Expected: Tauri tensor summary tests pass. `git diff --check` exits successfully.

- [ ] **Step 7: Commit Tauri/docs updates**

```powershell
git add code/src-tauri/src/engine_commands.rs docs/TENSOR_SPEC.md docs/RUST_ENGINE_API.md docs/TOOLS.md docs/superpowers/plans/2026-05-01-profile-registry-board-tensors.md
git commit -m "docs: point tensor profiles at owned layout modules"
```

---

### Task 6: Full Verification

**Files:**
- No code edits expected.

- [ ] **Step 1: Run Rust tensor and mask tests**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run phase-flow tests**

Run:

```powershell
cargo test -p digimon-engine --test phase_flow -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Rebuild PyO3 and run binding tests**

Run:

```powershell
cd code\digimon-engine-py
maturin develop
cd ..\..
python -m pytest code/tests/test_rust_bindings_surface.py -q
```

Expected: PASS.

- [ ] **Step 4: Run RL tests**

Run:

```powershell
python -m pytest code/tests/rl -q
```

Expected: PASS.

- [ ] **Step 5: Run Rust backend parity tests**

Run:

```powershell
python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -q
```

Expected: PASS.

- [ ] **Step 6: Run frontend DTO tests**

Run:

```powershell
cd code\frontend
npm run test -- gameApi.test.ts
cd ..\..
```

Expected: PASS.

- [ ] **Step 7: Run final whitespace check**

Run:

```powershell
git diff --check
```

Expected: exits with code 0.

- [ ] **Step 8: Commit verification fixups only if needed**

If the verification steps required code or doc fixes, commit them:

```powershell
git add code docs
git commit -m "fix: stabilize profile-owned tensor layout"
```

If verification passed without additional edits, do not create an empty commit.

---

## Self-Review

### Spec Coverage

- Profile modules own layout numbers: Task 2 creates `tensor_profiles/standard/v1.rs` with all Standard v1 constants.
- Folder structure by game mode and version: Task 2 creates `tensor_profiles/standard/v1.rs`.
- Shared profile types and registry: Task 2 creates `tensor_profiles/mod.rs`.
- Compatibility alias: Task 2 updates `lib.rs` with `pub use tensor_profiles as tensor_profile`.
- `tensor.rs` as writer/compatibility surface: Task 3 re-exports layout constants and uses Standard v1 profile metadata.
- PyO3/Python `game_mode`: Task 4 adds tests and implementation.
- Tauri/docs import preference and path references: Task 5 updates imports and docs.
- Full verification: Task 6 runs Rust, PyO3, RL, parity, frontend, and diff checks.

### Placeholder Scan

No placeholders remain. Every code-changing step lists exact files, exact code snippets, exact commands, and expected outcomes.

### Type Consistency

The plan consistently uses `tensor_profiles` as the new module, `tensor_profile` only as a compatibility alias, `game_mode` as the new metadata field, `standard::v1::PROFILE` as the Standard v1 source of truth, and `standard_v1` as the public profile id.

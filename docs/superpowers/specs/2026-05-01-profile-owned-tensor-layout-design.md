# Profile-Owned Tensor Layout Design

**Goal:** Move board tensor layout ownership into per-game-mode, per-version profile modules so each profile owns its tensor size, section ranges, slot shape, and derived card/scalar positions in one auditable location.

## Problem

The current profile registry describes the standard board tensor, but the layout is still owned by `tensor.rs`. That leaves profile modules as metadata catalogs over another module's constants. It is better than scattered magic numbers, but it does not make future profile differences easy to audit.

Future modes such as Standard, EDH, or Titan may need different values for:

- `TENSOR_SIZE`
- `FIELD_SLOTS`
- `MAX_SOURCES`
- hand, trash, security, revealed, or selection capacities
- slot header shape and source entry shape
- section ordering and section ranges
- card-id and scalar tensor positions

Those values should live with the profile that defines them. A reviewer should be able to open one profile file and understand the full layout contract for that mode/version.

## File Structure

Use a plural profile module tree:

```text
code/digimon-engine/src/tensor_profiles/
  mod.rs
  standard/
    mod.rs
    v1.rs
```

Only `standard/v1.rs` exists for now. Future profiles extend the tree without changing the shape:

```text
code/digimon-engine/src/tensor_profiles/
  standard/
    v1.rs
    v2.rs
  edh/
    v1.rs
  titan/
    v1.rs
```

Keep a temporary compatibility alias:

```rust
pub mod tensor_profiles;
pub use tensor_profiles as tensor_profile;
```

Existing consumers of `digimon_engine::tensor_profile` continue to compile while new code uses `digimon_engine::tensor_profiles`.

## Ownership Model

Each profile module owns its full layout constants. `standard/v1.rs` should define values directly, not import them from `tensor.rs`:

```rust
pub const PROFILE_ID: &str = "standard_v1";
pub const GAME_MODE: &str = "standard";
pub const VERSION: u32 = 1;

pub const FIELD_SLOTS: usize = 14;
pub const MAX_SOURCES: usize = 11;
pub const GLOBAL_SIZE: usize = 10;
pub const HAND_SIZE: usize = 20;
pub const TRASH_SIZE: usize = 45;
pub const SECURITY_SIZE: usize = 10;
pub const REVEALED_SIZE: usize = 10;
pub const SELECTION_SIZE: usize = 5;

pub const SLOT_HEADER_SIZE: usize = 7;
pub const SOURCE_ENTRY_SIZE: usize = 3;
pub const SLOT_SIZE: usize = SLOT_HEADER_SIZE + MAX_SOURCES * SOURCE_ENTRY_SIZE;

pub const BATTLE_SIZE: usize = FIELD_SLOTS * SLOT_SIZE;
pub const BREEDING_SIZE: usize = SLOT_SIZE;

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
```

The same file also owns the section table, slot header fields, source fields, slot layout, position derivation, and final `TensorProfile` value.

## Shared Types

`tensor_profiles/mod.rs` should own the shared metadata types:

```rust
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
```

Add `game_mode` to the profile metadata. This lets callers distinguish `standard/v1` from a future `edh/v1` even if both use version `1`.

Keep existing field and section concepts:

- `TensorSection`
- `TensorSectionKind`
- `TensorSlotLayout`
- `TensorSlotField`
- `TensorFieldKind`
- `TensorSlotHeaderField`
- `TensorSourceField`

The registry API remains deterministic and read-only:

```rust
pub fn default_profile() -> TensorProfile;
pub fn all_profile_ids() -> Vec<&'static str>;
pub fn profile_by_id(id: &str) -> Option<TensorProfile>;
```

## Standard V1 Writer Contract

`tensor.rs` currently writes the standard 1375-float tensor. After this change, it should become the Standard v1 tensor writer and compatibility surface.

For this PR, keep the public constants in `tensor.rs` by re-exporting them from `tensor_profiles::standard::v1`:

```rust
pub use crate::tensor_profiles::standard::v1::{
    BATTLE_SIZE, BREEDING_SIZE, FIELD_SLOTS, GLOBAL_SIZE, HAND_SIZE, MAX_SOURCES,
    OFF_GLOBAL, OFF_MY_BATTLE, OFF_MY_BREEDING, OFF_MY_HAND, OFF_MY_SECURITY,
    OFF_MY_TRASH, OFF_OPP_BATTLE, OFF_OPP_BREEDING, OFF_OPP_HAND,
    OFF_OPP_SECURITY, OFF_OPP_TRASH, OFF_REVEALED, OFF_SELECTION,
    REVEALED_SIZE, SECURITY_SIZE, SELECTION_SIZE, SLOT_SIZE, TENSOR_SIZE,
    TRASH_SIZE,
};
```

This preserves existing imports while making `standard/v1.rs` the source of truth.

`compute_positions()` should delegate to the active Standard v1 profile:

```rust
pub fn compute_positions() -> (Vec<usize>, Vec<usize>) {
    crate::tensor_profiles::standard::v1::PROFILE.positions()
}
```

Do not parameterize `build_tensor()` in this PR. It should continue to write Standard v1. A future profile-specific writer can be introduced when a non-Standard-v1 layout is actually needed.

## Registry Behavior

`tensor_profiles/standard/mod.rs` exposes the Standard profile family:

```rust
pub mod v1;

pub const DEFAULT_PROFILE: TensorProfile = v1::PROFILE;

pub fn profile_by_version(version: u32) -> Option<TensorProfile> {
    match version {
        1 => Some(v1::PROFILE),
        _ => None,
    }
}
```

`tensor_profiles/mod.rs` owns the global registry:

```rust
pub mod standard;

pub const STANDARD_V1_PROFILE_ID: &str = standard::v1::PROFILE_ID;

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
```

Profile IDs stay globally unique. Use `<game_mode>_v<version>` for public ids, e.g. `standard_v1`, `edh_v1`, and `titan_v1`.

## PyO3 And RL Metadata

PyO3 should import from `digimon_engine::tensor_profiles`, not the compatibility alias.

The Python-visible `TensorProfile` should expose `game_mode` in addition to the existing fields:

- `id`
- `game_mode`
- `version`
- `tensor_size`
- `field_slots`
- `slot_size`
- `max_sources`
- `card_id_slot_count`
- `scalar_slot_count`
- `card_id_positions`
- `scalar_positions`

`digimon_gym.tensor_profiles.TensorProfile` should mirror that field. Existing callers that only use `id`, counts, and positions continue to work.

## Documentation

`docs/TENSOR_SPEC.md` should say that canonical profile definitions live under:

```text
code/digimon-engine/src/tensor_profiles/<game_mode>/<version>.rs
```

For the current profile, the doc should point specifically to:

```text
code/digimon-engine/src/tensor_profiles/standard/v1.rs
```

The doc should state that `tensor.rs` is the Standard v1 writer and compatibility surface, not the owner of profile layout constants.

## Testing Requirements

Add or update Rust tests to verify:

- `default_profile().id == "standard_v1"`
- `default_profile().game_mode == "standard"`
- `profile_by_id("standard_v1")` resolves to the Standard v1 profile
- `all_profile_ids()` returns only `["standard_v1"]`
- Standard v1 sections cover `0..TENSOR_SIZE` exactly once
- Permanent slot sections are multiples of `SLOT_SIZE`
- `PROFILE.tensor_size == standard::v1::TENSOR_SIZE`
- `tensor::TENSOR_SIZE == standard::v1::TENSOR_SIZE`
- `tensor::FIELD_SLOTS == standard::v1::FIELD_SLOTS`
- `compute_positions()` matches `standard::v1::PROFILE.positions()`
- card-id and scalar positions are disjoint and cover the full tensor
- card-id and scalar counts remain `520` and `855`

Add or update PyO3/Python tests to verify:

- `get_tensor_profile().game_mode == "standard"`
- `TENSOR_PROFILE_ID == "standard_v1"`
- `list_tensor_profiles() == ["standard_v1"]`
- RL feature extraction still sizes embedding/scalar buffers from the profile

Run the existing tensor, RL, PyO3, and frontend tests after the refactor because this changes module ownership without intending to change runtime behavior.

## Non-Goals

This change does not add EDH or Titan profiles. It creates the folder and ownership model that will make those profiles straightforward later.

This change does not convert profile definitions to TOML, JSON, or another data file format. Rust modules are preferred for now because they keep profile constants compile-time checked, avoid runtime parsing, and allow derived offsets to be expressed without duplicated literals.

This change does not parameterize the engine tensor writer by arbitrary profiles. The current writer remains Standard v1 until another real profile requires a second writer or a generalized writer.

This change does not alter action IDs, masks, selection behavior, card registry indices, or existing tensor values.

## Migration Plan

1. Create `tensor_profiles/` with shared types and a `standard/v1.rs` profile that owns the current constants.
2. Re-export Standard v1 layout constants from `tensor.rs`.
3. Move section tables, slot field tables, slot layout, and position derivation into `standard/v1.rs`.
4. Replace `tensor_profile.rs` with the compatibility alias or remove the file after `lib.rs` aliases `tensor_profiles` as `tensor_profile`.
5. Update PyO3, RL adapter, Tauri, and docs imports to prefer `tensor_profiles`.
6. Add `game_mode` to Rust, PyO3, and Python profile metadata.
7. Run full verification and confirm no tensor values or position counts changed.

## Self-Review

This spec resolves the ownership concern directly: profile modules own layout numbers and derived ranges. It preserves the current public contract through re-exports and a compatibility alias. It does not introduce a second profile, a generalized writer, or runtime data parsing before there is a concrete need for those features.

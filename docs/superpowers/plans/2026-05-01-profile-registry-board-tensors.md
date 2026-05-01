# Profile Registry For Board Tensors Implementation Plan

> Superseded follow-up: `docs/superpowers/plans/2026-05-01-profile-owned-tensor-layout.md` refactors this registry so profile modules own the layout constants under `tensor_profiles/<game_mode>/<version>.rs`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a canonical board-tensor profile registry so Rust, PyO3, RL feature extraction, Tauri summaries, and frontend debug UI all agree on the active tensor layout.

**Architecture:** Introduce a Rust-owned `standard_v1` tensor profile that describes the existing 1375-float board tensor without changing its layout. Existing tensor writing stays in `tensor.rs`; the new registry owns metadata such as profile id, version, section sizes, card-id positions, scalar positions, and slot counts. PyO3 and frontend DTOs consume the same profile metadata so future tensor profiles can be added without hidden drift.

**Tech Stack:** Rust `digimon-engine`, PyO3 `digimon-engine-py`, Python `digimon_gym`, Tauri v2 DTOs, React TypeScript, Cargo tests, Pytest, Vitest.

---

## Scope Check

This plan implements metadata and registry plumbing only. It does not change:

- `TENSOR_SIZE = 1375`
- `ACTION_SPACE_SIZE = 2168`
- Any action id, mask rule, or decoder behavior
- The current `build_tensor(game, player_id, registry)` output
- The default `CardEmbeddingExtractor` feature dimension for the standard layout

The profile registry must be read-only and deterministic. If a future EDH or alternate layout arrives, it should be added as a second profile in the registry with its own tests; this plan only registers `standard_v1`.

## File Structure

### Rust Engine

- Create: `code/digimon-engine/src/tensor_profile.rs`
  - Defines `TensorProfile`, `TensorSection`, `TensorSectionKind`, `STANDARD_V1_PROFILE_ID`, `default_profile()`, `profile_by_id()`, `all_profile_ids()`, and `standard_v1_positions()`.
  - Uses the existing constants from `tensor.rs`; does not own tensor writing.

- Modify: `code/digimon-engine/src/lib.rs`
  - Export `pub mod tensor_profile;`.

- Modify: `code/digimon-engine/src/tensor.rs`
  - Make `compute_positions()` delegate to `tensor_profile::standard_v1_positions()`.
  - Keep constants and `build_tensor()` in place.

- Create: `code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs`
  - Verifies registry ids, section metadata, position split, and parity with `compute_positions()`.

- Modify: `code/digimon-engine/tests/mask_and_tensor/main.rs`
  - Add `mod tensor_profile;`.

### PyO3 And Python RL

- Modify: `code/digimon-engine-py/src/lib.rs`
  - Expose `TensorProfile`, `get_tensor_profile(profile_id=None)`, `list_tensor_profiles()`, and `TENSOR_PROFILE_ID`.

- Create: `code/digimon_gym/tensor_profiles.py`
  - Python adapter for profile metadata.
  - Uses PyO3 when available and falls back to the legacy tensor layout only to keep tests importable before the wheel is rebuilt.

- Modify: `code/digimon_gym/agents/features_extractor.py`
  - Read card/scalar positions from `digimon_gym.tensor_profiles.get_tensor_profile()`.
  - Default remains `standard_v1`.

- Create: `code/tests/rl/test_tensor_profiles.py`
  - Verifies profile metadata, PyO3 export, Python fallback shape, and feature extractor buffer sizes.

- Modify: `code/tests/test_rust_bindings_surface.py`
  - Add binding surface tests for tensor profile exports.

### Tauri And Frontend

- Modify: `code/src-tauri/src/engine_commands.rs`
  - Add profile metadata fields to `TensorSummaryDto`.
  - Populate them from `digimon_engine::tensor_profile::default_profile()`.

- Modify: `code/frontend/src/types/game.ts`
  - Add `profileId`, `profileVersion`, `cardIdSlotCount`, and `scalarSlotCount` to `TensorSummary`.

- Modify: `code/frontend/src/api/gameApi.ts`
  - Translate new snake_case fields from `TensorSummaryDto`.

- Modify: `code/frontend/src/components/board/TensorDebugBadge.tsx`
  - Display the tensor profile id in the existing debug badge.

- Modify: `code/frontend/src/api/gameApi.test.ts`
  - Assert tensor summary translation includes profile metadata.

### Docs

- Modify: `docs/TENSOR_SPEC.md`
  - Add a profile registry section documenting `standard_v1`.

- Modify: `docs/RUST_ENGINE_API.md`
  - Document Rust/PyO3 profile accessors.

---

## Task 1: Add The Rust Tensor Profile Registry

**Files:**
- Create: `code/digimon-engine/src/tensor_profile.rs`
- Modify: `code/digimon-engine/src/lib.rs`
- Modify: `code/digimon-engine/src/tensor.rs`
- Create: `code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs`
- Modify: `code/digimon-engine/tests/mask_and_tensor/main.rs`

- [ ] **Step 1: Write the failing Rust profile tests**

Create `code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs`:

```rust
use digimon_engine::tensor::{compute_positions, FIELD_SLOTS, SLOT_SIZE, TENSOR_SIZE};
use digimon_engine::tensor_profile::{
    all_profile_ids, default_profile, profile_by_id, TensorSectionKind, STANDARD_V1_PROFILE_ID,
};

#[test]
fn default_profile_is_standard_v1() {
    let profile = default_profile();

    assert_eq!(profile.id, STANDARD_V1_PROFILE_ID);
    assert_eq!(profile.version, 1);
    assert_eq!(profile.tensor_size, TENSOR_SIZE);
    assert_eq!(profile.field_slots, FIELD_SLOTS);
    assert_eq!(profile.slot_size, SLOT_SIZE);
    assert_eq!(profile.card_id_slot_count, 520);
    assert_eq!(profile.scalar_slot_count, 855);
}

#[test]
fn registry_resolves_standard_profile_by_id() {
    let ids = all_profile_ids();
    assert_eq!(ids, vec![STANDARD_V1_PROFILE_ID]);

    let profile = profile_by_id(STANDARD_V1_PROFILE_ID).unwrap();
    assert_eq!(profile.id, "standard_v1");
    assert!(profile_by_id("missing_profile").is_none());
}

#[test]
fn standard_profile_sections_cover_tensor_without_overlap() {
    let profile = default_profile();
    let mut covered = Vec::new();

    for section in profile.sections {
        assert!(section.start + section.len <= profile.tensor_size);
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
    let (tensor_cards, tensor_scalars) = compute_positions();

    assert_eq!(profile_cards, tensor_cards);
    assert_eq!(profile_scalars, tensor_scalars);
    assert_eq!(profile_cards.len(), profile.card_id_slot_count);
    assert_eq!(profile_scalars.len(), profile.scalar_slot_count);
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
```

Modify `code/digimon-engine/tests/mask_and_tensor/main.rs` by adding the new module line:

```rust
mod action_explain;
mod action_main_effects_parity;
mod card_registry_parity;
mod mask_end_of_turn_parity;
mod mask_main_effects_parity;
mod mask_main_parity;
mod tensor_and_mask;
mod tensor_helpers;
mod tensor_hidden_info;
mod tensor_profile;
mod tensor_source_contributions;
```

- [ ] **Step 2: Run the failing tests**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor tensor_profile -- --nocapture
```

Expected: FAIL with unresolved import `digimon_engine::tensor_profile`.

- [ ] **Step 3: Add the profile registry module**

Create `code/digimon-engine/src/tensor_profile.rs`:

```rust
//! Board tensor profile registry.
//!
//! Profiles describe the shape and semantic split of observation tensors.
//! They do not write tensor values; `tensor.rs` remains the only writer.

use serde::Serialize;

use crate::tensor::{
    FIELD_SLOTS, MAX_HAND, MAX_REVEALED, MAX_SECURITY, MAX_SOURCES, MAX_TRASH,
    SLOT_SIZE, SOURCE_ENTRY_SIZE, TENSOR_SIZE,
};

pub const STANDARD_V1_PROFILE_ID: &str = "standard_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TensorSectionKind {
    Scalars,
    CardIds,
    PermanentSlots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TensorSection {
    pub name: &'static str,
    pub start: usize,
    pub len: usize,
    pub kind: TensorSectionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TensorProfile {
    pub id: &'static str,
    pub version: u16,
    pub tensor_size: usize,
    pub field_slots: usize,
    pub slot_size: usize,
    pub max_sources: usize,
    pub card_id_slot_count: usize,
    pub scalar_slot_count: usize,
    pub sections: &'static [TensorSection],
}

impl TensorProfile {
    pub fn section(&self, name: &str) -> Option<&TensorSection> {
        self.sections.iter().find(|section| section.name == name)
    }

    pub fn positions(&self) -> (Vec<usize>, Vec<usize>) {
        match self.id {
            STANDARD_V1_PROFILE_ID => standard_v1_positions(),
            _ => (Vec::new(), Vec::new()),
        }
    }
}

const STANDARD_V1_SECTIONS: [TensorSection; 13] = [
    TensorSection { name: "global", start: 0, len: 10, kind: TensorSectionKind::Scalars },
    TensorSection { name: "my_battle", start: 10, len: 560, kind: TensorSectionKind::PermanentSlots },
    TensorSection { name: "opponent_battle", start: 570, len: 560, kind: TensorSectionKind::PermanentSlots },
    TensorSection { name: "my_hand", start: 1130, len: 20, kind: TensorSectionKind::CardIds },
    TensorSection { name: "opponent_hand", start: 1150, len: 20, kind: TensorSectionKind::CardIds },
    TensorSection { name: "my_trash", start: 1170, len: 45, kind: TensorSectionKind::CardIds },
    TensorSection { name: "opponent_trash", start: 1215, len: 45, kind: TensorSectionKind::CardIds },
    TensorSection { name: "my_security", start: 1260, len: 10, kind: TensorSectionKind::CardIds },
    TensorSection { name: "opponent_security", start: 1270, len: 10, kind: TensorSectionKind::CardIds },
    TensorSection { name: "my_breeding", start: 1280, len: 40, kind: TensorSectionKind::PermanentSlots },
    TensorSection { name: "opponent_breeding", start: 1320, len: 40, kind: TensorSectionKind::PermanentSlots },
    TensorSection { name: "revealed", start: 1360, len: 10, kind: TensorSectionKind::CardIds },
    TensorSection { name: "selection", start: 1370, len: 5, kind: TensorSectionKind::Scalars },
];

pub const STANDARD_V1_PROFILE: TensorProfile = TensorProfile {
    id: STANDARD_V1_PROFILE_ID,
    version: 1,
    tensor_size: TENSOR_SIZE,
    field_slots: FIELD_SLOTS,
    slot_size: SLOT_SIZE,
    max_sources: MAX_SOURCES,
    card_id_slot_count: 520,
    scalar_slot_count: TENSOR_SIZE - 520,
    sections: &STANDARD_V1_SECTIONS,
};

pub fn default_profile() -> &'static TensorProfile {
    &STANDARD_V1_PROFILE
}

pub fn all_profile_ids() -> Vec<&'static str> {
    vec![STANDARD_V1_PROFILE_ID]
}

pub fn profile_by_id(id: &str) -> Option<&'static TensorProfile> {
    match id {
        STANDARD_V1_PROFILE_ID => Some(&STANDARD_V1_PROFILE),
        _ => None,
    }
}

pub fn standard_v1_positions() -> (Vec<usize>, Vec<usize>) {
    let mut card_positions = Vec::new();
    let mut scalar_positions = Vec::new();

    for i in 0..10 {
        scalar_positions.push(i);
    }

    for section in STANDARD_V1_SECTIONS {
        match section.kind {
            TensorSectionKind::Scalars => {
                if section.name != "global" {
                    scalar_positions.extend(section.start..section.start + section.len);
                }
            }
            TensorSectionKind::CardIds => {
                card_positions.extend(section.start..section.start + section.len);
            }
            TensorSectionKind::PermanentSlots => {
                let slots = section.len / SLOT_SIZE;
                for i in 0..slots {
                    slot_positions(
                        section.start + i * SLOT_SIZE,
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

fn slot_positions(
    slot_base: usize,
    card_positions: &mut Vec<usize>,
    scalar_positions: &mut Vec<usize>,
) {
    card_positions.push(slot_base);
    for j in 1..7 {
        scalar_positions.push(slot_base + j);
    }

    let src_base = slot_base + 7;
    for source_index in 0..MAX_SOURCES {
        let off = src_base + source_index * SOURCE_ENTRY_SIZE;
        card_positions.push(off);
        scalar_positions.push(off + 1);
        scalar_positions.push(off + 2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_v1_counts_are_stable() {
        let (cards, scalars) = standard_v1_positions();
        assert_eq!(cards.len(), 520);
        assert_eq!(scalars.len(), 855);
        assert_eq!(cards.len() + scalars.len(), TENSOR_SIZE);
        assert_eq!(STANDARD_V1_PROFILE.scalar_slot_count, 855);
    }

    #[test]
    fn constants_are_used_so_imports_stay_live() {
        assert_eq!(MAX_HAND, 20);
        assert_eq!(MAX_TRASH, 45);
        assert_eq!(MAX_SECURITY, 10);
        assert_eq!(MAX_REVEALED, 10);
    }
}
```

- [ ] **Step 4: Export the module from the Rust crate**

In `code/digimon-engine/src/lib.rs`, add the module declaration next to the existing tensor export:

```rust
pub mod tensor;
pub mod tensor_profile;
```

If `pub mod tensor;` already exists in a different block, add only:

```rust
pub mod tensor_profile;
```

- [ ] **Step 5: Delegate `compute_positions()` to the registry**

In `code/digimon-engine/src/tensor.rs`, add this import near the top:

```rust
use crate::tensor_profile::standard_v1_positions;
```

Replace the body of `compute_positions()` with:

```rust
pub fn compute_positions() -> (Vec<usize>, Vec<usize>) {
    standard_v1_positions()
}
```

Leave `slot_positions()` in `tensor.rs` in place for now only if another function still uses it. If the compiler reports it as dead code, delete the private `slot_positions()` helper from `tensor.rs`; the equivalent helper now lives in `tensor_profile.rs`.

- [ ] **Step 6: Run Rust profile tests**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor tensor_profile -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Run existing tensor tests**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor tensor -- --nocapture
```

Expected: PASS. Existing tensor output and position counts remain unchanged.

- [ ] **Step 8: Commit**

```powershell
git add code/digimon-engine/src/tensor_profile.rs code/digimon-engine/src/lib.rs code/digimon-engine/src/tensor.rs code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs code/digimon-engine/tests/mask_and_tensor/main.rs
git commit -m "feat: add board tensor profile registry"
```

---

## Task 2: Expose Tensor Profiles Through PyO3

**Files:**
- Modify: `code/digimon-engine-py/src/lib.rs`
- Modify: `code/tests/test_rust_bindings_surface.py`

- [ ] **Step 1: Add failing PyO3 surface tests**

Append to `code/tests/test_rust_bindings_surface.py`:

```python
class TestTensorProfiles:
    def test_tensor_profile_id_constant(self):
        from digimon_engine import TENSOR_PROFILE_ID

        assert TENSOR_PROFILE_ID == "standard_v1"

    def test_list_tensor_profiles(self):
        from digimon_engine import list_tensor_profiles

        assert list_tensor_profiles() == ["standard_v1"]

    def test_get_default_tensor_profile(self):
        from digimon_engine import TENSOR_SIZE, get_tensor_profile

        profile = get_tensor_profile()
        assert profile.id == "standard_v1"
        assert profile.version == 1
        assert profile.tensor_size == TENSOR_SIZE
        assert profile.card_id_slot_count == 520
        assert profile.scalar_slot_count == 855
        assert len(profile.card_id_positions) == 520
        assert len(profile.scalar_positions) == 855
        assert profile.card_id_positions[0] == 10
        assert profile.scalar_positions[0] == 0

    def test_get_unknown_tensor_profile_raises(self):
        import pytest
        from digimon_engine import get_tensor_profile

        with pytest.raises(ValueError, match="unknown tensor profile"):
            get_tensor_profile("missing")
```

- [ ] **Step 2: Run the failing PyO3 tests**

Run:

```powershell
python -m pytest code/tests/test_rust_bindings_surface.py::TestTensorProfiles -v
```

Expected: FAIL because the profile exports do not exist.

- [ ] **Step 3: Add the PyO3 profile class and functions**

In `code/digimon-engine-py/src/lib.rs`, add imports near the existing tensor imports:

```rust
use ::digimon_engine::tensor_profile::{
    all_profile_ids, default_profile, profile_by_id, TensorProfile as RustTensorProfile,
    STANDARD_V1_PROFILE_ID,
};
```

Add this `#[pyclass]` after the existing enum/class definitions:

```rust
#[pyclass(module = "digimon_engine", name = "TensorProfile")]
#[derive(Clone)]
pub struct TensorProfile {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub version: u16,
    #[pyo3(get)]
    pub tensor_size: usize,
    #[pyo3(get)]
    pub field_slots: usize,
    #[pyo3(get)]
    pub slot_size: usize,
    #[pyo3(get)]
    pub max_sources: usize,
    #[pyo3(get)]
    pub card_id_slot_count: usize,
    #[pyo3(get)]
    pub scalar_slot_count: usize,
    #[pyo3(get)]
    pub card_id_positions: Vec<usize>,
    #[pyo3(get)]
    pub scalar_positions: Vec<usize>,
}

fn py_tensor_profile(profile: &RustTensorProfile) -> TensorProfile {
    let (card_id_positions, scalar_positions) = profile.positions();
    TensorProfile {
        id: profile.id.to_string(),
        version: profile.version,
        tensor_size: profile.tensor_size,
        field_slots: profile.field_slots,
        slot_size: profile.slot_size,
        max_sources: profile.max_sources,
        card_id_slot_count: profile.card_id_slot_count,
        scalar_slot_count: profile.scalar_slot_count,
        card_id_positions,
        scalar_positions,
    }
}

#[pyfunction]
#[pyo3(signature = (profile_id = None))]
fn get_tensor_profile(profile_id: Option<String>) -> PyResult<TensorProfile> {
    let profile = match profile_id {
        None => default_profile(),
        Some(id) => profile_by_id(&id)
            .ok_or_else(|| PyValueError::new_err(format!("unknown tensor profile: {id}")))?,
    };
    Ok(py_tensor_profile(profile))
}

#[pyfunction]
fn list_tensor_profiles() -> Vec<String> {
    all_profile_ids()
        .into_iter()
        .map(str::to_string)
        .collect()
}
```

In the `#[pymodule] fn digimon_engine(...)` body, add:

```rust
m.add_class::<TensorProfile>()?;
m.add_function(wrap_pyfunction!(get_tensor_profile, m)?)?;
m.add_function(wrap_pyfunction!(list_tensor_profiles, m)?)?;
m.add("TENSOR_PROFILE_ID", STANDARD_V1_PROFILE_ID)?;
```

- [ ] **Step 4: Rebuild PyO3 bindings**

Run:

```powershell
cd code\digimon-engine-py
maturin develop
cd ..\..
```

Expected: build succeeds and installs the local `digimon_engine` module.

- [ ] **Step 5: Run PyO3 profile tests**

Run:

```powershell
python -m pytest code/tests/test_rust_bindings_surface.py::TestTensorProfiles -v
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add code/digimon-engine-py/src/lib.rs code/tests/test_rust_bindings_surface.py
git commit -m "feat: expose tensor profiles to python"
```

---

## Task 3: Move RL Feature Extraction To The Profile Adapter

**Files:**
- Create: `code/digimon_gym/tensor_profiles.py`
- Modify: `code/digimon_gym/agents/features_extractor.py`
- Create: `code/tests/rl/test_tensor_profiles.py`

- [ ] **Step 1: Write failing Python profile adapter tests**

Create `code/tests/rl/test_tensor_profiles.py`:

```python
from __future__ import annotations

from gymnasium import spaces
import numpy as np


def test_default_tensor_profile_shape():
    from digimon_gym.tensor_profiles import get_tensor_profile
    from digimon_engine import TENSOR_PROFILE_ID, TENSOR_SIZE

    profile = get_tensor_profile()

    assert profile.id == TENSOR_PROFILE_ID
    assert profile.tensor_size == TENSOR_SIZE
    assert profile.card_id_slot_count == 520
    assert profile.scalar_slot_count == 855
    assert len(profile.card_id_positions) == 520
    assert len(profile.scalar_positions) == 855


def test_tensor_profile_positions_cover_tensor():
    from digimon_gym.tensor_profiles import get_tensor_profile

    profile = get_tensor_profile()
    positions = set(profile.card_id_positions) | set(profile.scalar_positions)

    assert len(positions) == profile.tensor_size
    assert min(positions) == 0
    assert max(positions) == profile.tensor_size - 1
    assert set(profile.card_id_positions).isdisjoint(profile.scalar_positions)


def test_feature_extractor_uses_profile_positions():
    import torch
    from digimon_engine import TENSOR_SIZE
    from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
    from digimon_gym.tensor_profiles import get_tensor_profile

    profile = get_tensor_profile()
    space = spaces.Box(
        shape=(TENSOR_SIZE,),
        low=-10.0,
        high=20001.0,
        dtype=np.float32,
    )

    extractor = CardEmbeddingExtractor(space)

    assert extractor.card_id_indices.numel() == profile.card_id_slot_count
    assert extractor.scalar_indices.numel() == profile.scalar_slot_count

    obs = torch.zeros((2, TENSOR_SIZE), dtype=torch.float32)
    out = extractor(obs)
    assert tuple(out.shape) == (2, 512)
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profiles.py -v
```

Expected: FAIL because `digimon_gym.tensor_profiles` does not exist.

- [ ] **Step 3: Add the Python profile adapter**

Create `code/digimon_gym/tensor_profiles.py`:

```python
"""Board tensor profile metadata used by RL feature extraction.

Rust owns the canonical profile registry. The fallback keeps imports working
before a local PyO3 wheel has been rebuilt, and it must match standard_v1.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable


@dataclass(frozen=True)
class TensorProfile:
    id: str
    version: int
    tensor_size: int
    field_slots: int
    slot_size: int
    max_sources: int
    card_id_slot_count: int
    scalar_slot_count: int
    card_id_positions: tuple[int, ...]
    scalar_positions: tuple[int, ...]


def get_tensor_profile(profile_id: str | None = None) -> TensorProfile:
    try:
        import digimon_engine

        raw = digimon_engine.get_tensor_profile(profile_id)
        return TensorProfile(
            id=raw.id,
            version=raw.version,
            tensor_size=raw.tensor_size,
            field_slots=raw.field_slots,
            slot_size=raw.slot_size,
            max_sources=raw.max_sources,
            card_id_slot_count=raw.card_id_slot_count,
            scalar_slot_count=raw.scalar_slot_count,
            card_id_positions=tuple(raw.card_id_positions),
            scalar_positions=tuple(raw.scalar_positions),
        )
    except (ImportError, AttributeError):
        if profile_id not in (None, "standard_v1"):
            raise ValueError(f"unknown tensor profile: {profile_id}") from None
        return _legacy_standard_v1()


def list_tensor_profiles() -> list[str]:
    try:
        import digimon_engine

        return list(digimon_engine.list_tensor_profiles())
    except (ImportError, AttributeError):
        return ["standard_v1"]


def _legacy_standard_v1() -> TensorProfile:
    from engine_py_legacy.engine.data.tensor_layout import (
        CARD_ID_POSITIONS,
        NUM_CARD_SLOTS,
        NUM_SCALAR_SLOTS,
        SCALAR_POSITIONS,
    )
    from engine_py_legacy.engine.game import FIELD_SLOTS, MAX_SOURCES, SLOT_SIZE, TENSOR_SIZE

    return TensorProfile(
        id="standard_v1",
        version=1,
        tensor_size=TENSOR_SIZE,
        field_slots=FIELD_SLOTS,
        slot_size=SLOT_SIZE,
        max_sources=MAX_SOURCES,
        card_id_slot_count=NUM_CARD_SLOTS,
        scalar_slot_count=NUM_SCALAR_SLOTS,
        card_id_positions=_as_tuple(CARD_ID_POSITIONS),
        scalar_positions=_as_tuple(SCALAR_POSITIONS),
    )


def _as_tuple(values: Iterable[int]) -> tuple[int, ...]:
    return tuple(int(v) for v in values)
```

- [ ] **Step 4: Update the feature extractor to use the adapter**

In `code/digimon_gym/agents/features_extractor.py`, replace the legacy tensor layout import:

```python
from engine_py_legacy.engine.data.tensor_layout import (
    CARD_ID_POSITIONS, SCALAR_POSITIONS, NUM_CARD_SLOTS, NUM_SCALAR_SLOTS,
)  # parity-doc: tensor_layout stays on Python engine
```

with:

```python
from digimon_gym.tensor_profiles import get_tensor_profile
```

Update the constructor signature:

```python
        tensor_profile_id: Optional[str] = None,
```

directly after `pretrained_embeddings: Optional[np.ndarray] = None,`.

At the start of `__init__`, after `super().__init__(observation_space, features_dim)`, add:

```python
        profile = get_tensor_profile(tensor_profile_id)
        if observation_space.shape != (profile.tensor_size,):
            raise ValueError(
                f"observation space shape {observation_space.shape} does not match "
                f"tensor profile {profile.id} size {profile.tensor_size}"
            )
```

Replace the buffer setup with:

```python
        self.register_buffer(
            'card_id_indices',
            torch.tensor(profile.card_id_positions, dtype=torch.long),
        )
        self.register_buffer(
            'scalar_indices',
            torch.tensor(profile.scalar_positions, dtype=torch.long),
        )
```

Replace the projection input calculation:

```python
        combined_dim = NUM_SCALAR_SLOTS + NUM_CARD_SLOTS * embedding_dim
```

with:

```python
        combined_dim = profile.scalar_slot_count + profile.card_id_slot_count * embedding_dim
```

- [ ] **Step 5: Run the profile adapter tests**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profiles.py -v
```

Expected: PASS.

- [ ] **Step 6: Run existing RL tests**

Run:

```powershell
python -m pytest code/tests/rl -v
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add code/digimon_gym/tensor_profiles.py code/digimon_gym/agents/features_extractor.py code/tests/rl/test_tensor_profiles.py
git commit -m "feat: use tensor profiles in rl feature extraction"
```

---

## Task 4: Add Profile Metadata To Tauri Tensor Summaries And Frontend Types

**Files:**
- Modify: `code/src-tauri/src/engine_commands.rs`
- Modify: `code/frontend/src/types/game.ts`
- Modify: `code/frontend/src/api/gameApi.ts`
- Modify: `code/frontend/src/components/board/TensorDebugBadge.tsx`
- Modify: `code/frontend/src/api/gameApi.test.ts`

- [ ] **Step 1: Add failing frontend translation test**

In `code/frontend/src/api/gameApi.test.ts`, add or update a `toTensorSummary` test:

```ts
import { describe, expect, it } from 'vitest';
import { toTensorSummary } from './gameApi';

describe('toTensorSummary', () => {
  it('translates tensor profile metadata', () => {
    const summary = toTensorSummary({
      player_id: 0,
      profile_id: 'standard_v1',
      profile_version: 1,
      tensor_size: 1375,
      mask_size: 2168,
      legal_action_count: 12,
      card_id_slot_count: 520,
      scalar_slot_count: 855,
      turn_count: 4,
      phase: 'Main',
      memory: 2,
      tensor_head: [0, 3, 0.2],
    });

    expect(summary.profileId).toBe('standard_v1');
    expect(summary.profileVersion).toBe(1);
    expect(summary.cardIdSlotCount).toBe(520);
    expect(summary.scalarSlotCount).toBe(855);
    expect(summary.tensorSize).toBe(1375);
  });
});
```

If `gameApi.test.ts` already has imports or a `toTensorSummary` suite, merge this test into the existing suite instead of duplicating imports.

- [ ] **Step 2: Run the failing frontend test**

Run:

```powershell
cd code\frontend
npm run test -- gameApi.test.ts
cd ..\..
```

Expected: FAIL because profile fields are missing from `TensorSummaryDto` or `TensorSummary`.

- [ ] **Step 3: Add profile metadata to the Rust DTO**

In `code/src-tauri/src/engine_commands.rs`, add this import:

```rust
use digimon_engine::tensor_profile::default_profile;
```

Extend `TensorSummaryDto`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorSummaryDto {
    pub player_id: PlayerId,
    pub profile_id: String,
    pub profile_version: u16,
    pub tensor_size: usize,
    pub mask_size: usize,
    pub legal_action_count: usize,
    pub card_id_slot_count: usize,
    pub scalar_slot_count: usize,
    pub turn_count: u16,
    pub phase: String,
    pub memory: i16,
    pub tensor_head: Vec<f32>,
}
```

Update `tensor_summary_for(...)` so it starts by reading the profile and returns the new fields:

```rust
fn tensor_summary_for(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
    mask: &[f32],
) -> TensorSummaryDto {
    let profile = default_profile();
    let tensor = build_tensor(game, player_id, registry);
    TensorSummaryDto {
        player_id,
        profile_id: profile.id.to_string(),
        profile_version: profile.version,
        tensor_size: tensor.len(),
        mask_size: mask.len(),
        legal_action_count: mask.iter().filter(|&&v| v > 0.0).count(),
        card_id_slot_count: profile.card_id_slot_count,
        scalar_slot_count: profile.scalar_slot_count,
        turn_count: game.turn_count,
        phase: format!("{:?}", game.current_phase),
        memory: game.memory,
        tensor_head: tensor.into_iter().take(12).collect(),
    }
}
```

Update the existing Rust test `tensor_summary_reports_engine_contract()` to include:

```rust
assert_eq!(summary.profile_id, "standard_v1");
assert_eq!(summary.profile_version, 1);
assert_eq!(summary.card_id_slot_count, 520);
assert_eq!(summary.scalar_slot_count, 855);
```

- [ ] **Step 4: Add profile fields to frontend types and mapper**

In `code/frontend/src/types/game.ts`, update `TensorSummary`:

```ts
export interface TensorSummary {
  playerId: number;
  profileId: string;
  profileVersion: number;
  tensorSize: number;
  maskSize: number;
  legalActionCount: number;
  cardIdSlotCount: number;
  scalarSlotCount: number;
  turnCount: number;
  phase: string;
  memory: number;
  tensorHead: number[];
}
```

In `code/frontend/src/api/gameApi.ts`, update `TensorSummaryDto`:

```ts
interface TensorSummaryDto {
  player_id: number;
  profile_id: string;
  profile_version: number;
  tensor_size: number;
  mask_size: number;
  legal_action_count: number;
  card_id_slot_count: number;
  scalar_slot_count: number;
  turn_count: number;
  phase: string;
  memory: number;
  tensor_head: number[];
}
```

Update `toTensorSummary`:

```ts
export function toTensorSummary(summary: TensorSummaryDto): TensorSummary {
  return {
    playerId: summary.player_id,
    profileId: summary.profile_id,
    profileVersion: summary.profile_version,
    tensorSize: summary.tensor_size,
    maskSize: summary.mask_size,
    legalActionCount: summary.legal_action_count,
    cardIdSlotCount: summary.card_id_slot_count,
    scalarSlotCount: summary.scalar_slot_count,
    turnCount: summary.turn_count,
    phase: summary.phase,
    memory: summary.memory,
    tensorHead: summary.tensor_head,
  };
}
```

- [ ] **Step 5: Display the profile id on the debug badge**

In `code/frontend/src/components/board/TensorDebugBadge.tsx`, replace the rendered badge body with:

```tsx
return (
  <div className="ib-tensor-badge" aria-label="Board tensor summary">
    <span>{summary.profileId}</span>
    <span>P{summary.playerId}</span>
    <span>T{summary.tensorSize}</span>
    <span>A{summary.maskSize}</span>
    <span>L{summary.legalActionCount}</span>
    <span>{summary.phase}</span>
  </div>
);
```

- [ ] **Step 6: Run Rust Tauri command tests**

Run:

```powershell
cargo test -p digimon-tcg tensor_summary -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Run frontend tests and build**

Run:

```powershell
cd code\frontend
npm run test -- gameApi.test.ts
npm run build
cd ..\..
```

Expected: PASS.

- [ ] **Step 8: Commit**

```powershell
git add code/src-tauri/src/engine_commands.rs code/frontend/src/types/game.ts code/frontend/src/api/gameApi.ts code/frontend/src/components/board/TensorDebugBadge.tsx code/frontend/src/api/gameApi.test.ts
git commit -m "feat: surface tensor profile metadata"
```

---

## Task 5: Document The Profile Contract

**Files:**
- Modify: `docs/TENSOR_SPEC.md`
- Modify: `docs/RUST_ENGINE_API.md`

- [ ] **Step 1: Update `docs/TENSOR_SPEC.md`**

Add this section after the constants table:

```markdown
## Tensor Profiles

The canonical board tensor profile is `standard_v1`.

`standard_v1` describes the existing 1375-float two-player board tensor.
It is registered in `code/digimon-engine/src/tensor_profile.rs` and is the
default profile exposed by Rust, PyO3, RL feature extraction, and desktop
Tauri tensor summaries.

Profile metadata includes:

| Field | `standard_v1` |
|---|---:|
| `id` | `standard_v1` |
| `version` | `1` |
| `tensor_size` | `1375` |
| `field_slots` | `14` |
| `slot_size` | `40` |
| `max_sources` | `11` |
| `card_id_slot_count` | `520` |
| `scalar_slot_count` | `855` |

The profile registry is metadata only. It must not change the tensor writer's
values, hide card decisions, or alter action masks. Any future profile must
have its own id and version, and must update this document, Rust profile tests,
PyO3 exports, RL feature extraction tests, and frontend DTO tests in the same
change.
```

- [ ] **Step 2: Update `docs/RUST_ENGINE_API.md`**

Add this section near the tensor API documentation:

```markdown
## Board Tensor Profiles

Rust exposes canonical board tensor metadata through
`digimon_engine::tensor_profile`.

Important accessors:

```rust
use digimon_engine::tensor_profile::{
    all_profile_ids,
    default_profile,
    profile_by_id,
    STANDARD_V1_PROFILE_ID,
};

let profile = default_profile();
assert_eq!(profile.id, STANDARD_V1_PROFILE_ID);
assert_eq!(profile.tensor_size, digimon_engine::tensor::TENSOR_SIZE);
let (card_id_positions, scalar_positions) = profile.positions();
```

PyO3 mirrors the same metadata:

```python
from digimon_engine import TENSOR_PROFILE_ID, get_tensor_profile

profile = get_tensor_profile()
assert profile.id == TENSOR_PROFILE_ID == "standard_v1"
assert profile.tensor_size == 1375
assert len(profile.card_id_positions) == 520
```

RL feature extractors should consume profile metadata through
`digimon_gym.tensor_profiles.get_tensor_profile()` rather than importing the
legacy Python tensor layout directly.
```

- [ ] **Step 3: Commit**

```powershell
git add docs/TENSOR_SPEC.md docs/RUST_ENGINE_API.md
git commit -m "docs: document board tensor profiles"
```

---

## Task 6: Full Verification

**Files:**
- No code edits expected.

- [ ] **Step 1: Run Rust tensor and mask tests**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run Tauri command tests**

Run:

```powershell
cargo test -p digimon-tcg -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Rebuild PyO3 and run binding tests**

Run:

```powershell
cd code\digimon-engine-py
maturin develop
cd ..\..
python -m pytest code/tests/test_rust_bindings_surface.py::TestTensorProfiles -v
```

Expected: PASS.

- [ ] **Step 4: Run RL tests**

Run:

```powershell
python -m pytest code/tests/rl -v
```

Expected: PASS.

- [ ] **Step 5: Run frontend tests and build**

Run:

```powershell
cd code\frontend
npm run test -- gameApi.test.ts
npm run build
cd ..\..
```

Expected: PASS.

- [ ] **Step 6: Smoke check the desktop tensor summary**

Run:

```powershell
cd code\frontend
npm run dev:desktop -- --host 127.0.0.1 --port 5173
```

Expected manual checks:

- Start a local game.
- The tensor debug badge shows `standard_v1`.
- The badge still shows `T1375` and `A2168`.
- Legal action count is greater than zero when a player can act.
- Human and agent action traces still render.

- [ ] **Step 7: Final fixup commit only if needed**

If verification required small fixes:

```powershell
git add code docs
git commit -m "fix: stabilize board tensor profile registry"
```

If verification passed without changes, do not create an empty commit.

---

## Self-Review

### Spec Coverage

- Canonical profile registry: Task 1 creates Rust `tensor_profile`.
- Board tensor contract preserved: Task 1 delegates metadata only and keeps `build_tensor()` unchanged.
- PyO3/RL access: Tasks 2 and 3 expose profile metadata and stop new RL code from depending directly on legacy tensor layout.
- Desktop tensor summaries: Task 4 surfaces profile id/version/slot counts through Tauri and frontend types.
- Documentation: Task 5 updates tensor and Rust API docs.
- Verification: Task 6 covers Rust, PyO3, RL, frontend, and desktop smoke checks.

### Placeholder Scan

No placeholder steps remain. Every task has concrete files, exact commands, expected outcomes, and code snippets for the intended changes.

### Type Consistency

Rust uses `TensorProfile`, `TensorSection`, `TensorSectionKind`, and `STANDARD_V1_PROFILE_ID`.
PyO3 exposes `TensorProfile`, `get_tensor_profile`, `list_tensor_profiles`, and `TENSOR_PROFILE_ID`.
Python uses `digimon_gym.tensor_profiles.TensorProfile`.
Frontend uses `TensorSummary.profileId`, `profileVersion`, `cardIdSlotCount`, and `scalarSlotCount`.

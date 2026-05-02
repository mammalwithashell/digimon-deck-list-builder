# Standard Compact Tensor Profile Naming Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `standard_compact_v1` the canonical ID for the current 1375-float Standard observation profile, while preserving `standard_v1` and `compact_v1` as compatibility aliases and updating v2 specs to use the same naming pattern.

**Architecture:** Keep the existing `tensor_profiles/standard/v1.rs` layout module and make the profile ID describe both game format and tensor shape. The registry resolves legacy aliases to the canonical profile, but profile metadata and profile listings return the canonical ID only. This is a naming and contract-cleanup change; it must not change tensor values, tensor size, card/scalar positions, or action-space behavior.

**Tech Stack:** Rust `digimon-engine`, PyO3 `digimon-engine-py`, Python `digimon_gym`, pytest, Rust integration tests, Markdown docs.

---

## Naming Decision

Use these stable IDs:

| Meaning | Canonical ID | Compatibility aliases |
|---|---|---|
| Current 1375-float Standard tensor | `standard_compact_v1` | `standard_v1`, `compact_v1` |
| Planned v2 lite Standard tensor | `standard_lite_v2` | `v2_lite` during spec transition only |
| Planned v2 full Standard tensor | `standard_full_v2` | `v2_full` during spec transition only |
| Planned pending metadata ablation | `standard_pending_ablation_v2` | `v2_pending_ablation` during spec transition only |

Rules:

- `default_profile().id` returns `standard_compact_v1`.
- `all_profile_ids()` returns only canonical IDs, starting with `["standard_compact_v1"]`.
- `profile_by_id("standard_v1")` and `profile_by_id("compact_v1")` return the canonical `standard_compact_v1` profile.
- Python fallback behavior accepts the same three IDs and returns profile metadata with `id == "standard_compact_v1"`.
- Documentation may mention legacy aliases, but examples should use `standard_compact_v1`.

## File Structure

- Modify `code/digimon-engine/src/tensor_profiles/standard/v1.rs`: change the profile ID constant to `standard_compact_v1`.
- Modify `code/digimon-engine/src/tensor_profiles/mod.rs`: rename the exported canonical constant and add compatibility alias constants plus resolver support.
- Modify `code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs`: update expectations and add alias-resolution coverage.
- Modify `code/digimon-engine-py/src/lib.rs`: no new API shape, but tests must confirm PyO3 exposes the canonical default ID after Rust changes.
- Modify `code/tests/test_rust_bindings_surface.py`: update profile ID assertions and add legacy alias assertions.
- Modify `code/digimon_gym/tensor_profiles.py`: update fallback ID and accepted aliases.
- Modify `code/tests/rl/test_tensor_profiles.py`: update fallback tests and add alias tests.
- Modify `docs/TENSOR_SPEC.md`, `docs/RUST_ENGINE_API.md`, `docs/TOOLS.md`: update current profile name and compatibility note.
- Modify `docs/superpowers/specs/2026-05-01-observation-profile-registry-design.md` and `docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md`: update future profile names to the format + size + version convention.

---

### Task 1: Rust Registry Canonical ID And Aliases

**Files:**
- Modify: `code/digimon-engine/src/tensor_profiles/standard/v1.rs`
- Modify: `code/digimon-engine/src/tensor_profiles/mod.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs`

- [ ] **Step 1: Write the failing Rust registry tests**

In `code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs`, update the import and add explicit alias checks:

```rust
use digimon_engine::tensor_profiles::{
    all_profile_ids, default_profile, profile_by_id, TensorFieldKind, TensorSectionKind,
    STANDARD_COMPACT_V1_PROFILE_ID, STANDARD_V1_LEGACY_PROFILE_ID,
    COMPACT_V1_LEGACY_PROFILE_ID,
};

#[test]
fn default_profile_is_standard_compact_v1() {
    let profile = default_profile();

    assert_eq!(profile.id, STANDARD_COMPACT_V1_PROFILE_ID);
    assert_eq!(profile.id, "standard_compact_v1");
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
fn registry_lists_only_canonical_profile_ids() {
    assert_eq!(all_profile_ids(), vec![STANDARD_COMPACT_V1_PROFILE_ID]);
}

#[test]
fn registry_resolves_standard_compact_profile_and_legacy_aliases() {
    for id in [
        STANDARD_COMPACT_V1_PROFILE_ID,
        STANDARD_V1_LEGACY_PROFILE_ID,
        COMPACT_V1_LEGACY_PROFILE_ID,
    ] {
        let profile = profile_by_id(id).unwrap();
        assert_eq!(profile.id, STANDARD_COMPACT_V1_PROFILE_ID);
        assert_eq!(profile.game_mode, "standard");
        assert_eq!(profile.tensor_size, TENSOR_SIZE);
    }

    assert!(profile_by_id("missing_profile").is_none());
}
```

Also update `singular_tensor_profile_alias_still_works` so the singular compatibility module exposes the new canonical constant:

```rust
#[test]
fn singular_tensor_profile_alias_still_works() {
    let singular = digimon_engine::tensor_profile::default_profile();
    let plural = digimon_engine::tensor_profiles::default_profile();

    assert_eq!(singular, plural);
    assert_eq!(
        digimon_engine::tensor_profile::STANDARD_COMPACT_V1_PROFILE_ID,
        "standard_compact_v1"
    );
    assert_eq!(
        digimon_engine::tensor_profile::STANDARD_V1_LEGACY_PROFILE_ID,
        "standard_v1"
    );
}
```

- [ ] **Step 2: Run the Rust profile test and verify it fails**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile -- --nocapture
```

Expected: FAIL because `STANDARD_COMPACT_V1_PROFILE_ID`, `STANDARD_V1_LEGACY_PROFILE_ID`, and `COMPACT_V1_LEGACY_PROFILE_ID` are not exported yet, and the profile ID is still `standard_v1`.

- [ ] **Step 3: Update the Rust profile constants and resolver**

In `code/digimon-engine/src/tensor_profiles/standard/v1.rs`, change:

```rust
pub const PROFILE_ID: &str = "standard_v1";
```

to:

```rust
pub const PROFILE_ID: &str = "standard_compact_v1";
```

In `code/digimon-engine/src/tensor_profiles/mod.rs`, replace the old exported constant and resolver with:

```rust
pub const STANDARD_COMPACT_V1_PROFILE_ID: &str = standard::v1::PROFILE_ID;
pub const STANDARD_V1_LEGACY_PROFILE_ID: &str = "standard_v1";
pub const COMPACT_V1_LEGACY_PROFILE_ID: &str = "compact_v1";
```

and:

```rust
pub fn all_profile_ids() -> Vec<&'static str> {
    vec![standard::v1::PROFILE_ID]
}

pub fn profile_by_id(id: &str) -> Option<TensorProfile> {
    match id {
        standard::v1::PROFILE_ID
        | STANDARD_V1_LEGACY_PROFILE_ID
        | COMPACT_V1_LEGACY_PROFILE_ID => Some(standard::v1::PROFILE),
        _ => None,
    }
}
```

Do not change layout constants, sections, slot layout, `TENSOR_SIZE`, or `positions()`.

- [ ] **Step 4: Run the Rust profile test and verify it passes**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile -- --nocapture
```

Expected: PASS. The tensor profile tests should still show `TENSOR_SIZE == 1375`, card ID count `520`, and scalar count `855`.

- [ ] **Step 5: Commit**

```powershell
git add code/digimon-engine/src/tensor_profiles/standard/v1.rs code/digimon-engine/src/tensor_profiles/mod.rs code/digimon-engine/tests/mask_and_tensor/tensor_profile.rs
git commit -m "refactor: rename compact standard tensor profile"
```

---

### Task 2: PyO3 Binding Surface Tests

**Files:**
- Modify: `code/tests/test_rust_bindings_surface.py`
- Inspect only if tests fail: `code/digimon-engine-py/src/lib.rs`

- [ ] **Step 1: Write the failing PyO3 surface tests**

In `code/tests/test_rust_bindings_surface.py`, update the tensor profile tests to assert canonical default metadata and alias behavior:

```python
class TestTensorProfiles:
    def test_default_profile_id_matches_profile(self):
        from digimon_engine import TENSOR_PROFILE_ID, get_tensor_profile

        profile = get_tensor_profile()

        assert TENSOR_PROFILE_ID == "standard_compact_v1"
        assert profile.id == TENSOR_PROFILE_ID
        assert profile.game_mode == "standard"

    def test_tensor_profile_positions(self):
        from digimon_engine import TENSOR_SIZE, get_tensor_profile

        profile = get_tensor_profile()

        assert profile.id == "standard_compact_v1"
        assert profile.tensor_size == TENSOR_SIZE
        assert profile.card_id_slot_count == 520
        assert profile.scalar_slot_count == 855
        assert len(profile.card_id_positions) == 520
        assert len(profile.scalar_positions) == 855
        assert profile.card_id_positions[0] == 10
        assert profile.scalar_positions[0] == 0

    def test_legacy_tensor_profile_aliases_resolve_to_canonical_profile(self):
        from digimon_engine import get_tensor_profile

        canonical = get_tensor_profile("standard_compact_v1")
        for alias in ("standard_v1", "compact_v1"):
            profile = get_tensor_profile(alias)
            assert profile.id == canonical.id == "standard_compact_v1"
            assert profile.tensor_size == canonical.tensor_size
            assert profile.card_id_positions == canonical.card_id_positions
            assert profile.scalar_positions == canonical.scalar_positions

    def test_unknown_tensor_profile_raises(self):
        import pytest
        from digimon_engine import get_tensor_profile

        with pytest.raises(ValueError, match="unknown tensor profile"):
            get_tensor_profile("missing")
```

- [ ] **Step 2: Run the PyO3 surface tests and verify they fail before rebuild or pass after Task 1 rebuild**

Run:

```powershell
python -m pytest code/tests/test_rust_bindings_surface.py::TestTensorProfiles -v
```

Expected before rebuilding the PyO3 wheel: FAIL if the installed binding still exposes `standard_v1`. Expected after rebuilding against Task 1: PASS.

- [ ] **Step 3: Rebuild the local PyO3 wheel if the Python binding is stale**

Run:

```powershell
Push-Location code/digimon-engine-py
maturin develop
Pop-Location
```

Expected: maturin finishes successfully and installs the local `digimon_engine` module.

- [ ] **Step 4: Run the PyO3 surface tests and verify they pass**

Run:

```powershell
python -m pytest code/tests/test_rust_bindings_surface.py::TestTensorProfiles -v
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add code/tests/test_rust_bindings_surface.py
git commit -m "test: cover tensor profile aliases in bindings"
```

---

### Task 3: Python RL Fallback Wrapper

**Files:**
- Modify: `code/digimon_gym/tensor_profiles.py`
- Modify: `code/tests/rl/test_tensor_profiles.py`

- [ ] **Step 1: Write failing Python fallback and alias tests**

In `code/tests/rl/test_tensor_profiles.py`, update the fallback tests and add an explicit alias test:

```python
def test_tensor_profile_falls_back_when_engine_function_missing(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile

    monkeypatch.setitem(sys.modules, "digimon_engine", SimpleNamespace())

    profile = get_tensor_profile()

    assert profile.id == "standard_compact_v1"
    assert profile.game_mode == "standard"
    assert profile.card_id_slot_count == 520
    assert profile.scalar_slot_count == 855


def test_tensor_profile_falls_back_when_engine_module_absent(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile, list_tensor_profiles

    monkeypatch.delitem(sys.modules, "digimon_engine", raising=False)
    monkeypatch.setattr("importlib.util.find_spec", lambda name: None)

    profile = get_tensor_profile()

    assert profile.id == "standard_compact_v1"
    assert profile.game_mode == "standard"
    assert list_tensor_profiles() == ["standard_compact_v1"]


def test_tensor_profile_fallback_accepts_legacy_aliases(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile

    monkeypatch.delitem(sys.modules, "digimon_engine", raising=False)
    monkeypatch.setattr("importlib.util.find_spec", lambda name: None)

    canonical = get_tensor_profile("standard_compact_v1")
    for alias in ("standard_v1", "compact_v1"):
        profile = get_tensor_profile(alias)
        assert profile.id == canonical.id == "standard_compact_v1"
        assert profile.card_id_positions == canonical.card_id_positions
        assert profile.scalar_positions == canonical.scalar_positions

    with pytest.raises(ValueError, match="unknown tensor profile"):
        get_tensor_profile("missing")
```

- [ ] **Step 2: Run the fallback tests and verify they fail**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profiles.py -v
```

Expected: FAIL while `digimon_gym.tensor_profiles` still returns `standard_v1` from the fallback path.

- [ ] **Step 3: Update `digimon_gym.tensor_profiles` fallback aliases**

In `code/digimon_gym/tensor_profiles.py`, add:

```python
_CANONICAL_STANDARD_COMPACT_V1 = "standard_compact_v1"
_STANDARD_COMPACT_V1_ALIASES = {
    _CANONICAL_STANDARD_COMPACT_V1,
    "standard_v1",
    "compact_v1",
}
```

Change both fallback checks in `get_tensor_profile` to:

```python
if profile_id not in (None, *_STANDARD_COMPACT_V1_ALIASES):
    raise ValueError(f"unknown tensor profile: {profile_id}") from None
return _legacy_standard_compact_v1()
```

Change `list_tensor_profiles` fallback returns to:

```python
return [_CANONICAL_STANDARD_COMPACT_V1]
```

Rename `_legacy_standard_v1()` to `_legacy_standard_compact_v1()` and return:

```python
return TensorProfile(
    id=_CANONICAL_STANDARD_COMPACT_V1,
    game_mode="standard",
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
```

- [ ] **Step 4: Run the RL tensor profile tests and verify they pass**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profiles.py -v
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add code/digimon_gym/tensor_profiles.py code/tests/rl/test_tensor_profiles.py
git commit -m "fix: support standard compact tensor profile fallback"
```

---

### Task 4: Documentation And V2 Spec Naming

**Files:**
- Modify: `docs/TENSOR_SPEC.md`
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `docs/TOOLS.md`
- Modify: `docs/superpowers/specs/2026-05-01-observation-profile-registry-design.md`
- Modify: `docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md`

- [ ] **Step 1: Update the current tensor spec language**

In `docs/TENSOR_SPEC.md`, replace current-profile references so the table starts:

```markdown
The canonical board tensor profile is `standard_compact_v1`:

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
```

Add this compatibility note after the table:

```markdown
`standard_v1` and `compact_v1` are compatibility aliases for older code and design notes. New code and model metadata should write `standard_compact_v1`.
```

Update the section heading:

```markdown
### `standard_compact_v1` Sections
```

- [ ] **Step 2: Update Rust API docs examples**

In `docs/RUST_ENGINE_API.md`, update examples to use:

```rust
use digimon_engine::tensor_profiles::{
    default_profile,
    STANDARD_COMPACT_V1_PROFILE_ID,
};

let profile = default_profile();
assert_eq!(profile.id, STANDARD_COMPACT_V1_PROFILE_ID);
```

and:

```python
from digimon_engine import TENSOR_PROFILE_ID, get_tensor_profile

profile = get_tensor_profile()
assert profile.id == TENSOR_PROFILE_ID == "standard_compact_v1"
```

- [ ] **Step 3: Update tooling docs**

In `docs/TOOLS.md`, keep the 1375-float description but add:

```markdown
The current canonical profile ID is `standard_compact_v1`; `standard_v1` and `compact_v1` are accepted only as legacy aliases.
```

- [ ] **Step 4: Update v2 design spec naming**

In `docs/superpowers/specs/2026-05-01-observation-profile-registry-design.md`, replace the profile table with:

```markdown
| Profile | String ID | Purpose |
|---|---|---|
| `StandardCompactV1` | `standard_compact_v1` | Current 1375-float Standard baseline |
| `StandardPendingAblationV2` | `standard_pending_ablation_v2` | Standard compact/fair board plus rich pending-choice metadata, used to isolate pending metadata value |
| `StandardLiteV2` | `standard_lite_v2` | First serious fair-information Standard v2 profile with structured board, hand, known-zone, and pending-choice tables |
| `StandardFullV2` | `standard_full_v2` | Standard v2 including `action_id_features[2168][16]` |
| `StandardOmniscientDebugV1` | `standard_omniscient_debug_v1` | Test-only Standard profile that may expose hidden identities |
```

Replace examples and comparison labels so they read:

```text
standard_compact_v1 vs standard_pending_ablation_v2
standard_compact_v1 vs standard_lite_v2
standard_pending_ablation_v2 vs standard_lite_v2
```

In `docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md`, rename the profile labels:

```markdown
The first implementation target should be `standard_lite_v2`.
```

and:

```markdown
The full experimental profile is `standard_full_v2`.
```

Keep the layout constants named `TENSOR_SIZE_V2_LITE` and `TENSOR_SIZE_V2_FULL`; those are shape constants, not public profile IDs.

- [ ] **Step 5: Search for stale documentation references**

Run:

```powershell
Select-String -Path docs\\*.md,docs\\superpowers\\specs\\*.md -Pattern 'standard_v1','compact_v1','v2_lite','v2_full','v2_pending_ablation' | Select-Object Path,LineNumber,Line
```

Expected: Any remaining `standard_v1` or `compact_v1` references are explicitly described as legacy aliases. Any remaining `v2_lite`, `v2_full`, or `v2_pending_ablation` references are either historical references or layout constant names, not public profile IDs.

- [ ] **Step 6: Commit**

```powershell
git add docs/TENSOR_SPEC.md docs/RUST_ENGINE_API.md docs/TOOLS.md docs/superpowers/specs/2026-05-01-observation-profile-registry-design.md docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md
git commit -m "docs: standardize tensor profile naming"
```

---

### Task 5: Final Verification

**Files:**
- No planned source edits.
- Verify: Rust profile tests, PyO3 profile tests, RL tensor profile tests, docs search.

- [ ] **Step 1: Run Rust tensor profile verification**

Run:

```powershell
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor tensor_profile -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run PyO3 binding profile verification**

Run:

```powershell
python -m pytest code/tests/test_rust_bindings_surface.py::TestTensorProfiles -v
```

Expected: PASS.

- [ ] **Step 3: Run RL tensor profile verification**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profiles.py -v
```

Expected: PASS.

- [ ] **Step 4: Run stale-name search**

Run:

```powershell
Select-String -Path code\\digimon-engine\\src\\*.rs,code\\digimon-engine\\src\\tensor_profiles\\**\\*.rs,code\\digimon-engine-py\\src\\lib.rs,code\\digimon_gym\\**\\*.py,code\\tests\\**\\*.py,docs\\*.md,docs\\superpowers\\specs\\*.md -Pattern 'standard_v1','compact_v1','v2_lite','v2_full','v2_pending_ablation' | Select-Object Path,LineNumber,Line
```

Expected: Stale names only appear as compatibility aliases, test inputs for aliases, or historical/spec-transition explanations. Canonical examples and default metadata use `standard_compact_v1`, `standard_lite_v2`, `standard_full_v2`, or `standard_pending_ablation_v2`.

- [ ] **Step 5: Check git diff for accidental tensor changes**

Run:

```powershell
git diff -- code/digimon-engine/src/tensor.rs code/digimon-engine/src/tensor_profiles/standard/v1.rs
```

Expected: `standard/v1.rs` only changes `PROFILE_ID`; `tensor.rs` has no runtime writer changes.

- [ ] **Step 6: Commit any verification-only test/doc adjustments**

If verification required small test or doc corrections, commit them:

```powershell
git add code docs
git commit -m "test: verify standard compact tensor profile naming"
```

If no files changed after Tasks 1-4, skip this commit.

---

## Self-Review

- Spec coverage: This plan covers the current profile naming ambiguity, Rust registry behavior, PyO3 exposure, Python fallback behavior, docs, and the v2 public-name convention. It intentionally does not implement `standard_lite_v2` or any v2 tensor writer.
- Placeholder scan: The plan uses concrete IDs, file paths, code snippets, commands, and expected outcomes. There are no `TBD` or `TODO` placeholders.
- Type consistency: Rust uses `STANDARD_COMPACT_V1_PROFILE_ID`, `STANDARD_V1_LEGACY_PROFILE_ID`, and `COMPACT_V1_LEGACY_PROFILE_ID`; Python uses `_CANONICAL_STANDARD_COMPACT_V1` and `_STANDARD_COMPACT_V1_ALIASES`; public metadata returns `standard_compact_v1`.


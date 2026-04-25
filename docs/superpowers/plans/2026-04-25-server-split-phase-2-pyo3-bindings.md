# Phase 2: Expand `digimon-engine-py` PyO3 Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand `code/digimon-engine-py` (today: only `RustHeadlessGame`) to expose the Python-visible surface that current Python-engine consumers need. After Phase 2, every binding has working Python tests; no callers cut over yet — that's Phase 3.

**Architecture:** TDD per binding group. Each task adds one logical surface (CardDatabase, deck_loader, enums, tested_cards, CardRegistry, model_utils, load_implemented_card_ids), with a Python-side test that loads the wheel via `maturin develop` and exercises the new export. Where the Rust crate already has the underlying logic (most of these), the work is wrapping. Where it doesn't, the plan adds a thin Rust-side helper rather than half-implementing logic in PyO3.

**Tech Stack:** Rust (digimon-engine, digimon-engine-py), PyO3 0.22, maturin, pytest. The bindings module is `digimon_engine` (Rust crate name `digimon-engine-py` with `lib.name = "digimon_engine"`).

**Spec:** `docs/superpowers/specs/2026-04-25-server-digimon-gym-split-design.md` (Phase 2, §2 binding table).

**Deferrals from the spec:**
- `PendingAction` — not exposed. It's a vestigial Python-engine artifact (only `digimon_gym/digimon_gym.py` checks for `PendingAction.TRASH_CARD`), and the Rust selection state machine surfaces this differently. The Phase 3 cutover handles it without an enum export.
- `PlayerType` (Human/Agent) — not exposed. It's a server orchestration concept, not an engine concept. The Rust engine doesn't differentiate; the server's per-session driver loop is what decides whether to await a WebSocket message or call a policy. Lives in `server/` after Phase 5.

**Coordination with the PvP bindings plan:**
A separate in-flight plan, [docs/superpowers/plans/2026-04-18-pyo3-pvp-bindings.md](2026-04-18-pyo3-pvp-bindings.md), expands `RustHeadlessGame` (or a sibling class) with `to_ui_json`, `get_pending_selection`, `get_events_since_last_step`, and `get_recording` — the engine-side surfaces PvP needs for state introspection mid-game. **That plan is independent of this one** and can land in either order. Phase 2 here adds *game-setup* bindings (card data, deck parsing, enums); the PvP plan adds *game-execution* bindings.

---

## File Map

**Files modified:**
- `digimon-engine/src/deck_tools.rs` — add a `RESTRICTED_LIST` static + `expand_deck_dict` helper if missing.
- `digimon-engine/src/lib.rs` — re-export any new helper publics.
- `digimon-engine-py/Cargo.toml` — add `pyo3-stub-gen` and any feature flags needed for new bindings (likely none).
- `digimon-engine-py/src/lib.rs` — add `CardDatabase`, deck-tool functions, enum classes, `tested_cards`, `CardRegistry` accessors, `get_models_dir`, `load_implemented_card_ids`.

**Files created:**
- `digimon-engine-py/python/digimon_engine/__init__.py` — type stubs / re-export shim if needed (likely just empty).
- `tests/engine/test_rust_bindings_surface.py` — Python-side tests for every new export.

**Files unchanged:**
- All existing Python engine code, all routers, all RL agents — Phase 3 cuts callers over, not Phase 2.

---

## Pre-flight: One-time setup

Each task assumes you can rebuild the wheel and re-run tests. Default workflow:

```bash
cd digimon-engine-py
maturin develop --release  # Rebuilds and installs into the active venv
cd ..
python -m pytest tests/engine/test_rust_bindings_surface.py -v
```

If `maturin develop` fails for an unrelated environment reason, stop and surface it before continuing.

---

## Task 1: Establish the bindings test scaffold

**Files:**
- Create: `tests/engine/test_rust_bindings_surface.py`

- [ ] **Step 1: Write a baseline import test**

Create `tests/engine/test_rust_bindings_surface.py` with:

```python
"""Surface tests for `digimon-engine-py` PyO3 bindings.

Each export added in Phase 2 of the server split gets a smoke test here
before any caller is migrated. The bindings module is `digimon_engine`
(crate `digimon-engine-py`, lib name `digimon_engine`).
"""

from __future__ import annotations

import pytest


def test_module_imports():
    import digimon_engine  # noqa: F401


def test_rust_headless_game_still_exported():
    """Phase 2 must not regress the existing RustHeadlessGame surface."""
    from digimon_engine import RustHeadlessGame  # noqa: F401
```

- [ ] **Step 2: Run the test to confirm it passes today**

Run: `cd digimon-engine-py && maturin develop --release && cd .. && python -m pytest tests/engine/test_rust_bindings_surface.py -v`

Expected: 2 passed.

If `maturin develop` fails with "no module named maturin" or similar: stop and surface — tooling is missing.

- [ ] **Step 3: Commit**

```bash
git add tests/engine/test_rust_bindings_surface.py
git commit -m "$(cat <<'EOF'
test(rust-bindings): scaffold surface test file

Empty smoke harness for Phase 2 PyO3 binding additions. Each
binding group adds its own test as it lands.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Wrap `CardDatabase`

The Rust crate already loads `data/cards.json` into a static `HashMap<String, CardData>` via the existing `card_db()` helper in `digimon-engine-py/src/lib.rs`. This task exposes that lookup as a Python class.

**Files:**
- Modify: `digimon-engine-py/src/lib.rs` — add `#[pyclass] CardDatabase` and `#[pymethods]` block.
- Modify: `tests/engine/test_rust_bindings_surface.py` — add `TestCardDatabase`.

- [ ] **Step 1: Write the failing test**

Append to `tests/engine/test_rust_bindings_surface.py`:

```python
class TestCardDatabase:
    def test_construct(self):
        from digimon_engine import CardDatabase
        db = CardDatabase()
        assert db is not None

    def test_get_known_card(self):
        from digimon_engine import CardDatabase
        db = CardDatabase()
        card = db.get_card("BT1-001")
        assert card is not None
        # Standard agumon should have a name and a level
        assert "Agumon" in card.name or "agumon" in card.name.lower()
        assert card.level is not None  # Rookie level

    def test_get_unknown_card_returns_none(self):
        from digimon_engine import CardDatabase
        db = CardDatabase()
        assert db.get_card("ZZ99-999") is None

    def test_count_cards(self):
        from digimon_engine import CardDatabase
        db = CardDatabase()
        # Whole-database count should be in the thousands
        assert db.count() > 1000
```

- [ ] **Step 2: Run to confirm it fails**

Run: `python -m pytest tests/engine/test_rust_bindings_surface.py::TestCardDatabase -v`

Expected: FAIL with `ImportError: cannot import name 'CardDatabase' from 'digimon_engine'`.

- [ ] **Step 3: Implement the binding**

Open `digimon-engine-py/src/lib.rs`. Above the existing `RustHeadlessGame` class definition, add:

```rust
/// Python-visible wrapper around the static card database.
#[pyclass]
pub struct CardDatabase {}

#[pyclass]
#[derive(Clone)]
pub struct PyCard {
    #[pyo3(get)]
    pub card_id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub level: Option<u32>,
    #[pyo3(get)]
    pub card_kind: String,
    #[pyo3(get)]
    pub colors: Vec<String>,
    #[pyo3(get)]
    pub play_cost: Option<u32>,
    #[pyo3(get)]
    pub dp: Option<u32>,
}

impl PyCard {
    fn from_card_data(card: &::digimon_engine::card_data::CardData) -> Self {
        Self {
            card_id: card.card_id.clone(),
            name: card.name.clone(),
            level: card.level,
            card_kind: format!("{:?}", card.card_kind),
            colors: card.colors.iter().map(|c| format!("{:?}", c)).collect(),
            play_cost: card.play_cost,
            dp: card.dp,
        }
    }
}

#[pymethods]
impl CardDatabase {
    #[new]
    fn new() -> PyResult<Self> {
        // Touch the static loader to force initialization with a clear error.
        let _ = card_db()?;
        Ok(Self {})
    }

    fn get_card(&self, card_id: &str) -> PyResult<Option<PyCard>> {
        let db = card_db()?;
        Ok(db.get(card_id).map(PyCard::from_card_data))
    }

    fn count(&self) -> PyResult<usize> {
        let db = card_db()?;
        Ok(db.len())
    }
}
```

Then in the `#[pymodule]` registration block at the bottom of `lib.rs`, add:

```rust
m.add_class::<CardDatabase>()?;
m.add_class::<PyCard>()?;
```

- [ ] **Step 4: Verify field names against `digimon-engine/src/card_data.rs`**

Open `digimon-engine/src/card_data.rs`. Confirm `CardData` actually has fields `card_id`, `name`, `level`, `card_kind`, `colors`, `play_cost`, `dp`. If any field is named differently, update the `PyCard::from_card_data` mapping. Don't invent field names.

- [ ] **Step 5: Rebuild and run the test**

Run: `cd digimon-engine-py && maturin develop --release && cd .. && python -m pytest tests/engine/test_rust_bindings_surface.py::TestCardDatabase -v`

Expected: 4 passed.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine-py/src/lib.rs tests/engine/test_rust_bindings_surface.py
git commit -m "$(cat <<'EOF'
feat(rust-bindings): expose CardDatabase + PyCard

Wraps the static cards.json loader as a Python-visible CardDatabase
class with get_card / count and a PyCard data-only wrapper for
metadata fields.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Wrap `deck_tools` parse + validate functions

The Rust crate's `deck_tools.rs` already has `parse_deck`, `parse_tts`, `parse_text`, `summarize_deck`, `validate_deck`, `classify_parsed`, `out_of_set_cards`. This task wraps them as module-level Python functions.

**Files:**
- Modify: `digimon-engine-py/src/lib.rs` — add `#[pyfunction]` wrappers + a `PyDeckValidationResult` class.
- Modify: `tests/engine/test_rust_bindings_surface.py`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/engine/test_rust_bindings_surface.py`:

```python
class TestDeckTools:
    def test_parse_tts_simple(self):
        from digimon_engine import parse_tts
        # TTS format: lines like "1 BT1-001"
        ids = parse_tts("1 BT1-001\n2 BT1-002")
        assert ids == ["BT1-001", "BT1-002", "BT1-002"]

    def test_parse_deck_dispatches(self):
        from digimon_engine import parse_deck
        # parse_deck should accept either TTS or text format
        ids = parse_deck("1 BT1-001")
        assert ids == ["BT1-001"]

    def test_summarize_deck(self):
        from digimon_engine import summarize_deck
        summary = summarize_deck(["BT1-001", "BT1-001", "BT1-002"])
        assert summary["BT1-001"] == 2
        assert summary["BT1-002"] == 1

    def test_validate_deck_legal(self):
        from digimon_engine import validate_deck
        # Build a 50-card deck of one card; this is illegal (4-of limit),
        # but validate_deck should at minimum return a result object with
        # an `is_valid` flag.
        result = validate_deck(["BT1-001"] * 50)
        assert hasattr(result, "is_valid")
        assert result.is_valid is False  # 4-of limit violated

    def test_out_of_set_cards(self):
        from digimon_engine import out_of_set_cards
        # Pass a known-bad ID alongside a known-good one
        bad = out_of_set_cards(["BT1-001", "ZZ99-999"])
        assert "ZZ99-999" in bad
        assert "BT1-001" not in bad
```

- [ ] **Step 2: Run to confirm failure**

Run: `python -m pytest tests/engine/test_rust_bindings_surface.py::TestDeckTools -v`

Expected: ImportError on each function.

- [ ] **Step 3: Implement the wrappers**

In `digimon-engine-py/src/lib.rs`, add:

```rust
use ::digimon_engine::deck_tools;

#[pyclass]
#[derive(Clone)]
pub struct PyDeckValidationResult {
    #[pyo3(get)]
    pub is_valid: bool,
    #[pyo3(get)]
    pub errors: Vec<String>,
    #[pyo3(get)]
    pub warnings: Vec<String>,
}

#[pyfunction]
fn parse_tts(raw: &str) -> PyResult<Vec<String>> {
    deck_tools::parse_tts(raw).map_err(PyValueError::new_err)
}

#[pyfunction]
fn parse_text(raw: &str) -> PyResult<Vec<String>> {
    deck_tools::parse_text(raw).map_err(PyValueError::new_err)
}

#[pyfunction]
fn parse_deck(raw: &str) -> PyResult<Vec<String>> {
    deck_tools::parse_deck(raw).map_err(PyValueError::new_err)
}

#[pyfunction]
fn summarize_deck(card_ids: Vec<String>) -> HashMap<String, u32> {
    deck_tools::summarize_deck(&card_ids)
}

#[pyfunction]
fn validate_deck(card_ids: Vec<String>) -> PyDeckValidationResult {
    let result = deck_tools::validate_deck(&card_ids);
    PyDeckValidationResult {
        is_valid: result.is_valid,
        errors: result.errors,
        warnings: result.warnings,
    }
}

#[pyfunction]
fn out_of_set_cards(card_ids: Vec<String>) -> Vec<String> {
    deck_tools::out_of_set_cards(card_ids.iter().cloned()).into_iter().collect()
}
```

In the `#[pymodule]` registration block, add:

```rust
m.add_class::<PyDeckValidationResult>()?;
m.add_function(wrap_pyfunction!(parse_tts, m)?)?;
m.add_function(wrap_pyfunction!(parse_text, m)?)?;
m.add_function(wrap_pyfunction!(parse_deck, m)?)?;
m.add_function(wrap_pyfunction!(summarize_deck, m)?)?;
m.add_function(wrap_pyfunction!(validate_deck, m)?)?;
m.add_function(wrap_pyfunction!(out_of_set_cards, m)?)?;
```

If `wrap_pyfunction` is not yet imported, add `use pyo3::wrap_pyfunction;` near the other `use` statements.

- [ ] **Step 4: Verify Rust signatures**

Open `digimon-engine/src/deck_tools.rs`. Confirm:
- `validate_deck(card_ids: &[String])` returns a `DeckValidationResult` whose fields are `is_valid: bool`, `errors: Vec<String>`, `warnings: Vec<String>`. If field names differ (e.g., `valid` instead of `is_valid`), update both the Rust mapping and the test assertion.
- `out_of_set_cards` takes an iterator of `S: AsRef<str>`. The wrapper passes `card_ids.iter().cloned()` (an iterator of `String`); confirm `String: AsRef<str>` satisfies the bound.

- [ ] **Step 5: Rebuild and run**

Run: `cd digimon-engine-py && maturin develop --release && cd .. && python -m pytest tests/engine/test_rust_bindings_surface.py::TestDeckTools -v`

Expected: 5 passed.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine-py/src/lib.rs tests/engine/test_rust_bindings_surface.py
git commit -m "$(cat <<'EOF'
feat(rust-bindings): expose deck_tools functions

parse_tts, parse_text, parse_deck, summarize_deck, validate_deck,
out_of_set_cards are now Python-callable. validate_deck returns a
PyDeckValidationResult with is_valid / errors / warnings.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Add Rust-side `RESTRICTED_LIST` and `expand_deck_dict`

The Python `deck_loader.RESTRICTED_LIST` is a `CardRestriction` defining the limited/banned list. The Rust crate has `CardRestriction` (per `digimon-engine/src/rules.rs`) but doesn't yet expose a static restriction list. `expand_deck_dict` is a trivial helper.

**Files:**
- Modify: `digimon-engine/src/deck_tools.rs` — add `restricted_list()` returning a `CardRestriction`, and `expand_deck_dict(counts: &HashMap<String, u32>) -> Vec<String>`.
- Modify: `digimon-engine-py/src/lib.rs` — wrap both.
- Modify: `tests/engine/test_rust_bindings_surface.py`.

- [ ] **Step 1: Find the Python `RESTRICTED_LIST`**

Run: `grep -A 30 "^RESTRICTED_LIST = CardRestriction" digimon_gym/engine/data/deck_loader.py | head -50`

Capture the exact restriction values (limited list, banned list) from the Python source. The Rust helper must match.

- [ ] **Step 2: Write the failing tests**

Append to `tests/engine/test_rust_bindings_surface.py`:

```python
class TestRestrictedList:
    def test_restricted_list_exists(self):
        from digimon_engine import restricted_list
        rl = restricted_list()
        # Should expose limited_card_ids and banned_card_ids as Python lists
        assert hasattr(rl, "limited_card_ids")
        assert hasattr(rl, "banned_card_ids")
        assert isinstance(rl.limited_card_ids, list)
        assert isinstance(rl.banned_card_ids, list)


class TestExpandDeckDict:
    def test_expand_basic(self):
        from digimon_engine import expand_deck_dict
        out = expand_deck_dict({"BT1-001": 3, "BT1-002": 1})
        assert sorted(out) == sorted(["BT1-001", "BT1-001", "BT1-001", "BT1-002"])

    def test_expand_empty(self):
        from digimon_engine import expand_deck_dict
        assert expand_deck_dict({}) == []
```

- [ ] **Step 3: Run to confirm failure**

Run: `python -m pytest tests/engine/test_rust_bindings_surface.py -k "Restricted or Expand" -v`

Expected: ImportError.

- [ ] **Step 4: Add Rust helpers**

In `digimon-engine/src/deck_tools.rs`, append:

```rust
use crate::rules::CardRestriction;

/// Return the canonical restricted list (limited + banned card IDs).
///
/// Mirrors the Python `RESTRICTED_LIST` constant in
/// `digimon_gym/engine/data/deck_loader.py`. Update both in lockstep
/// when the official restricted list changes.
pub fn restricted_list() -> CardRestriction {
    CardRestriction {
        limited_card_ids: vec![
            // ... values copied from Python RESTRICTED_LIST ...
        ],
        banned_card_ids: vec![
            // ... values copied from Python RESTRICTED_LIST ...
        ],
    }
}

/// Expand a {card_id -> count} map into a flat list of card IDs.
pub fn expand_deck_dict(counts: &std::collections::HashMap<String, u32>) -> Vec<String> {
    let mut out = Vec::new();
    for (card_id, count) in counts {
        for _ in 0..*count {
            out.push(card_id.clone());
        }
    }
    out
}
```

Fill in the `limited_card_ids` and `banned_card_ids` from the Python source captured in Step 1. If `CardRestriction` has additional fields (other than `limited_card_ids` and `banned_card_ids`), inspect `digimon-engine/src/rules.rs` and pass through identical values.

- [ ] **Step 5: Add the PyO3 wrappers**

In `digimon-engine-py/src/lib.rs`:

```rust
#[pyclass]
#[derive(Clone)]
pub struct PyCardRestriction {
    #[pyo3(get)]
    pub limited_card_ids: Vec<String>,
    #[pyo3(get)]
    pub banned_card_ids: Vec<String>,
}

#[pyfunction]
fn restricted_list() -> PyCardRestriction {
    let rl = deck_tools::restricted_list();
    PyCardRestriction {
        limited_card_ids: rl.limited_card_ids,
        banned_card_ids: rl.banned_card_ids,
    }
}

#[pyfunction]
fn expand_deck_dict(counts: HashMap<String, u32>) -> Vec<String> {
    deck_tools::expand_deck_dict(&counts)
}
```

Register both with `m.add_class::<PyCardRestriction>()?` and `m.add_function(wrap_pyfunction!(restricted_list, m)?)?` / `m.add_function(wrap_pyfunction!(expand_deck_dict, m)?)?`.

- [ ] **Step 6: Add Rust unit test for `restricted_list` parity**

In `digimon-engine/tests/` (or as an inline `#[cfg(test)]` module in `deck_tools.rs`), add a test that the returned list non-empty when the Python source's list is non-empty. Skip this test if the Python `RESTRICTED_LIST` is empty.

- [ ] **Step 7: Build, rebuild bindings, run**

Run:
```bash
cargo build --manifest-path digimon-engine/Cargo.toml
cd digimon-engine-py && maturin develop --release && cd ..
python -m pytest tests/engine/test_rust_bindings_surface.py -k "Restricted or Expand" -v
```

Expected: 3 passed.

- [ ] **Step 8: Commit**

```bash
git add digimon-engine/src/deck_tools.rs digimon-engine-py/src/lib.rs tests/engine/test_rust_bindings_surface.py
git commit -m "$(cat <<'EOF'
feat(engine): add restricted_list + expand_deck_dict helpers

Mirrors the Python deck_loader.RESTRICTED_LIST and expand_deck_dict
on the Rust side. Exposes both via PyO3.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Wrap engine enums (`CardKind`, `GamePhase`)

The Rust crate has `CardKind` and `GamePhase` enums in `digimon-engine/src/enums.rs`. Wrap them as Python-visible classes whose variants match the Python `Enum` names exactly (so callers can do `digimon_engine.CardKind.Digimon == card.card_kind`).

**Files:**
- Modify: `digimon-engine-py/src/lib.rs`.
- Modify: `tests/engine/test_rust_bindings_surface.py`.

- [ ] **Step 1: Confirm variant names**

Run: `grep -E "^\s+(Digimon|Tamer|Option|DigiEgg|Start|Draw|Breeding|Main|End|SelectTarget|SelectMaterial|BlockTiming|CounterTiming|SelectTrash|SelectSource|SelectHand|SelectReveal|SelectEffectChoice|SelectSecurity|EndOfTurnAction|AllianceTiming|Mulligan)" digimon-engine/src/enums.rs digimon_gym/engine/data/enums.py`

Confirm each variant name appears in BOTH the Rust enum and the Python enum with identical spelling. If a name diverges (e.g., Python `DigiEgg` vs Rust `Digiegg`), use the Python spelling — that's what callers will rely on after Phase 3.

- [ ] **Step 2: Write the failing test**

Append to `tests/engine/test_rust_bindings_surface.py`:

```python
class TestEnums:
    def test_card_kind_variants(self):
        from digimon_engine import CardKind
        assert CardKind.Digimon != CardKind.Tamer
        assert CardKind.Option != CardKind.DigiEgg

    def test_game_phase_variants_present(self):
        from digimon_engine import GamePhase
        assert GamePhase.Main is not None
        assert GamePhase.SelectTarget is not None
        assert GamePhase.Mulligan is not None

    def test_card_kind_string_repr(self):
        from digimon_engine import CardKind
        # Each variant should at minimum be hashable and have a useful repr
        assert "Digimon" in repr(CardKind.Digimon) or "Digimon" in str(CardKind.Digimon)
```

- [ ] **Step 3: Run to confirm failure**

Run: `python -m pytest tests/engine/test_rust_bindings_surface.py::TestEnums -v`

Expected: ImportError.

- [ ] **Step 4: Add the enum wrappers**

PyO3 0.22 supports `#[pyclass(eq, eq_int)]` for unit enums. In `digimon-engine-py/src/lib.rs`:

```rust
use ::digimon_engine::enums::{CardKind as RustCardKind, GamePhase as RustGamePhase};

#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
}

impl From<RustCardKind> for CardKind {
    fn from(k: RustCardKind) -> Self {
        match k {
            RustCardKind::Digimon => CardKind::Digimon,
            RustCardKind::Tamer => CardKind::Tamer,
            RustCardKind::Option => CardKind::Option,
            RustCardKind::DigiEgg => CardKind::DigiEgg,
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GamePhase {
    Start,
    Draw,
    Breeding,
    Main,
    End,
    SelectTarget,
    SelectMaterial,
    BlockTiming,
    CounterTiming,
    SelectTrash,
    SelectSource,
    SelectHand,
    SelectReveal,
    SelectEffectChoice,
    SelectSecurity,
    EndOfTurnAction,
    AllianceTiming,
    Mulligan,
}

impl From<RustGamePhase> for GamePhase {
    fn from(p: RustGamePhase) -> Self {
        match p {
            RustGamePhase::Start => GamePhase::Start,
            RustGamePhase::Draw => GamePhase::Draw,
            RustGamePhase::Breeding => GamePhase::Breeding,
            RustGamePhase::Main => GamePhase::Main,
            RustGamePhase::End => GamePhase::End,
            RustGamePhase::SelectTarget => GamePhase::SelectTarget,
            RustGamePhase::SelectMaterial => GamePhase::SelectMaterial,
            RustGamePhase::BlockTiming => GamePhase::BlockTiming,
            RustGamePhase::CounterTiming => GamePhase::CounterTiming,
            RustGamePhase::SelectTrash => GamePhase::SelectTrash,
            RustGamePhase::SelectSource => GamePhase::SelectSource,
            RustGamePhase::SelectHand => GamePhase::SelectHand,
            RustGamePhase::SelectReveal => GamePhase::SelectReveal,
            RustGamePhase::SelectEffectChoice => GamePhase::SelectEffectChoice,
            RustGamePhase::SelectSecurity => GamePhase::SelectSecurity,
            RustGamePhase::EndOfTurnAction => GamePhase::EndOfTurnAction,
            RustGamePhase::AllianceTiming => GamePhase::AllianceTiming,
            RustGamePhase::Mulligan => GamePhase::Mulligan,
        }
    }
}
```

If the Rust `CardKind` or `GamePhase` enums have additional variants not listed in the Python file, **add them too** (Python is the lagging side). If the Python enum has variants the Rust enum lacks, that's a parity gap — log to `docs/RUST_PYTHON_PARITY.md` and skip those variants here.

Register: `m.add_class::<CardKind>()?; m.add_class::<GamePhase>()?;`

- [ ] **Step 5: Update `PyCard.card_kind` to return the typed enum**

In Task 2, `PyCard.card_kind` was returned as a `String` (`format!("{:?}", ...)`). Now that `CardKind` is a real Python class, change `PyCard.card_kind` to return `CardKind`:

```rust
#[pyclass]
#[derive(Clone)]
pub struct PyCard {
    // ...
    #[pyo3(get)]
    pub card_kind: CardKind,
    // ...
}

impl PyCard {
    fn from_card_data(card: &::digimon_engine::card_data::CardData) -> Self {
        Self {
            // ...
            card_kind: CardKind::from(card.card_kind),
            // ...
        }
    }
}
```

Update the corresponding `TestCardDatabase.test_get_known_card` assertion to also check `card.card_kind == CardKind.Digimon` (or whichever variant BT1-001 actually is — confirm via the Python `digimon_gym.engine.data.card_database.CardDatabase().get_card("BT1-001").card_kind`).

- [ ] **Step 6: Rebuild and run**

```bash
cd digimon-engine-py && maturin develop --release && cd ..
python -m pytest tests/engine/test_rust_bindings_surface.py::TestEnums tests/engine/test_rust_bindings_surface.py::TestCardDatabase -v
```

Expected: all passed.

- [ ] **Step 7: Commit**

```bash
git add digimon-engine-py/src/lib.rs tests/engine/test_rust_bindings_surface.py
git commit -m "$(cat <<'EOF'
feat(rust-bindings): expose CardKind and GamePhase enums

Wraps the Rust enums as Python-visible classes with variant names
matching digimon_gym.engine.data.enums. PyCard.card_kind now returns
a typed CardKind instead of a string.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Wrap `tested_cards`

The Rust crate has `deck_tools::tested_cards_set()`, `tested_cards_sorted()`, `is_card_tested(id)`. The Python surface had `load_tested_cards()` (returns set) and `out_of_set_cards()` (Task 3 already wrapped). Add `load_tested_cards`.

**Files:**
- Modify: `digimon-engine-py/src/lib.rs`.
- Modify: `tests/engine/test_rust_bindings_surface.py`.

- [ ] **Step 1: Write the failing test**

Append:

```python
class TestTestedCards:
    def test_load_tested_cards_returns_set_like(self):
        from digimon_engine import load_tested_cards
        tested = load_tested_cards()
        # Should be a Python set or frozenset
        assert isinstance(tested, (set, frozenset))
        # Tested cards list is non-trivial
        assert len(tested) > 100

    def test_is_card_tested(self):
        from digimon_engine import is_card_tested
        # We don't know which IDs are in the tested list without reading
        # the JSON, so just check that the call returns a bool
        assert isinstance(is_card_tested("BT1-001"), bool)
        assert is_card_tested("ZZ99-999") is False
```

- [ ] **Step 2: Run to confirm failure**

Run: `python -m pytest tests/engine/test_rust_bindings_surface.py::TestTestedCards -v`

Expected: ImportError.

- [ ] **Step 3: Add the wrappers**

In `digimon-engine-py/src/lib.rs`:

```rust
use std::collections::HashSet;

#[pyfunction]
fn load_tested_cards() -> HashSet<String> {
    deck_tools::tested_cards_set().clone()
}

#[pyfunction]
fn is_card_tested(card_id: &str) -> bool {
    deck_tools::is_card_tested(card_id)
}
```

Register both. If `tested_cards_set()` returns a `&HashSet<String>` directly (not requiring `.clone()` of the elements), use it directly; the wrapper just needs to convert to Python.

- [ ] **Step 4: Rebuild and run**

```bash
cd digimon-engine-py && maturin develop --release && cd ..
python -m pytest tests/engine/test_rust_bindings_surface.py::TestTestedCards -v
```

Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine-py/src/lib.rs tests/engine/test_rust_bindings_surface.py
git commit -m "$(cat <<'EOF'
feat(rust-bindings): expose tested_cards helpers

load_tested_cards returns a Python set; is_card_tested returns bool.
Backed by the existing deck_tools::tested_cards_set static.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Wrap `CardRegistry`

The Rust crate has `CardRegistry` (`digimon-engine/src/card_registry.rs`). The Python `card_registry.py` exposes `REGISTRY_CAPACITY` and `EMBEDDING_DIM` constants plus per-card embedding lookup. Wrap the same surface.

**Files:**
- Modify: `digimon-engine-py/src/lib.rs`.
- Modify: `tests/engine/test_rust_bindings_surface.py`.

- [ ] **Step 1: Inspect the Rust `CardRegistry` API**

Run: `grep -E "^\s*(pub fn|pub const|pub static)" digimon-engine/src/card_registry.rs | head -20`

Note the public methods. Common shape: `CardRegistry::load(...)`, `capacity() -> usize`, `embedding_dim() -> usize`, `embedding_for(card_id: &str) -> Option<&[f32]>`. If a method is missing, decide in Step 3 whether to add a Rust helper or wrap what's there.

- [ ] **Step 2: Write the failing tests**

```python
class TestCardRegistry:
    def test_capacity_constant(self):
        from digimon_engine import REGISTRY_CAPACITY
        assert REGISTRY_CAPACITY > 0
        assert REGISTRY_CAPACITY <= 100_000  # Sanity

    def test_embedding_dim_constant(self):
        from digimon_engine import EMBEDDING_DIM
        assert EMBEDDING_DIM in (16, 32, 64, 128)  # Standard dims

    def test_card_registry_lookup(self):
        from digimon_engine import CardRegistry
        reg = CardRegistry()
        idx = reg.index_of("BT1-001")
        assert idx is not None
        assert isinstance(idx, int)
        assert idx > 0
```

- [ ] **Step 3: Implement the wrapper**

In `digimon-engine-py/src/lib.rs`, add:

```rust
use ::digimon_engine::card_registry::CardRegistry as RustCardRegistry;

#[pyclass]
pub struct CardRegistry {
    inner: RustCardRegistry,
}

#[pymethods]
impl CardRegistry {
    #[new]
    fn new() -> PyResult<Self> {
        let inner = RustCardRegistry::load_default()
            .map_err(|e| PyRuntimeError::new_err(format!("CardRegistry::load failed: {}", e)))?;
        Ok(Self { inner })
    }

    fn index_of(&self, card_id: &str) -> Option<u32> {
        self.inner.index_of(card_id)
    }

    fn capacity(&self) -> usize {
        self.inner.capacity()
    }
}
```

If the Rust API is `CardRegistry::load_default` doesn't exist, use whatever loader does — `CardRegistry::load(path)` with the canonical path from `data_paths`, or a `card_registry()` static accessor. Match Rust naming.

For the constants, expose at module level:

```rust
fn add_constants(m: &Bound<PyModule>) -> PyResult<()> {
    m.add("REGISTRY_CAPACITY", ::digimon_engine::card_registry::REGISTRY_CAPACITY)?;
    m.add("EMBEDDING_DIM", ::digimon_engine::card_registry::EMBEDDING_DIM)?;
    Ok(())
}
```

Call `add_constants(m)?;` from the `#[pymodule]` block. If the Rust constants are named differently (e.g., `CAPACITY`, `EMBED_DIM`), match them in the `m.add(...)` calls but expose them under the **Python** names `REGISTRY_CAPACITY` and `EMBEDDING_DIM` so callers don't have to learn two vocabularies.

If neither the constants nor an `index_of` method exist on the Rust side, **stop and surface**: this binding requires Rust-side work that's beyond a wrap. Log it to `docs/RUST_PYTHON_PARITY.md` and skip the test until Phase 2's revisit.

- [ ] **Step 4: Rebuild and run**

```bash
cd digimon-engine-py && maturin develop --release && cd ..
python -m pytest tests/engine/test_rust_bindings_surface.py::TestCardRegistry -v
```

Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine-py/src/lib.rs tests/engine/test_rust_bindings_surface.py
git commit -m "$(cat <<'EOF'
feat(rust-bindings): expose CardRegistry + capacity/dim constants

Wraps the static card registry (REGISTRY_CAPACITY, EMBEDDING_DIM) and
the index_of lookup so RL features_extractor and admin_models can
migrate off digimon_gym.engine.data.card_registry.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Wrap `get_models_dir`

The Python helper is 3 lines (`Path(os.environ.get("ONNX_MODELS_DIR", "models"))`). Same on the Rust side.

**Files:**
- Modify: `digimon-engine/src/deck_tools.rs` (or a new `paths.rs` module — see Step 1).
- Modify: `digimon-engine-py/src/lib.rs`.
- Modify: `tests/engine/test_rust_bindings_surface.py`.

- [ ] **Step 1: Decide on the Rust home**

If `digimon-engine/src/` has a generic paths or config module, add `get_models_dir` there. Otherwise, put it in `deck_tools.rs` (it's not deck-specific, but `deck_tools` is the existing "static configuration" home; better than scattering). Record the choice in the commit message.

- [ ] **Step 2: Write the failing test**

```python
class TestGetModelsDir:
    def test_default(self, monkeypatch):
        monkeypatch.delenv("ONNX_MODELS_DIR", raising=False)
        from digimon_engine import get_models_dir
        d = get_models_dir()
        # Default is "models" (relative to CWD)
        assert str(d).rstrip("/").endswith("models")

    def test_env_override(self, monkeypatch, tmp_path):
        monkeypatch.setenv("ONNX_MODELS_DIR", str(tmp_path))
        from digimon_engine import get_models_dir
        # Note: get_models_dir reads env at call-time, not import-time
        d = get_models_dir()
        assert str(d) == str(tmp_path)
```

- [ ] **Step 3: Add the Rust helper**

In `digimon-engine/src/deck_tools.rs` (or wherever Step 1 chose):

```rust
use std::path::PathBuf;

/// Resolve the ONNX models directory. Honors the `ONNX_MODELS_DIR` env
/// var; falls back to `models` relative to the working directory.
pub fn get_models_dir() -> PathBuf {
    std::env::var("ONNX_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("models"))
}
```

- [ ] **Step 4: Wrap it**

In `digimon-engine-py/src/lib.rs`:

```rust
#[pyfunction]
fn get_models_dir() -> PyResult<PathBuf> {
    Ok(deck_tools::get_models_dir())
}
```

`PyResult<PathBuf>` is automatically converted to `pathlib.Path` in PyO3 0.22. Register with `wrap_pyfunction!`.

- [ ] **Step 5: Rebuild and run**

```bash
cd digimon-engine-py && maturin develop --release && cd ..
python -m pytest tests/engine/test_rust_bindings_surface.py::TestGetModelsDir -v
```

Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/deck_tools.rs digimon-engine-py/src/lib.rs tests/engine/test_rust_bindings_surface.py
git commit -m "$(cat <<'EOF'
feat(rust-bindings): expose get_models_dir

Mirrors the Python helper that resolves ONNX_MODELS_DIR or falls back
to ./models. Used by admin_models router and inference loaders.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Wrap `load_implemented_card_ids`

The Python helper reads `digimon_gym/engine/data/scripts/_frozen_manifest.json` — the **Python** card-script frozen lane. Post-Rust-pivot, "implemented" means "registered in the Rust effect registry" (`cards.rs::build_registry()`). The wrapper returns the set of card IDs with Rust `CardEffect` registrations.

**Files:**
- Modify: `digimon-engine-py/src/lib.rs`.
- Modify: `tests/engine/test_rust_bindings_surface.py`.

- [ ] **Step 1: Inspect the Rust effect registry surface**

Run: `grep -E "^\s*(pub fn build_registry|pub fn .*registered|pub fn .*card_ids)" digimon-engine/src/cards.rs digimon-engine/src/cards/*.rs 2>/dev/null | head -10`

Look for an existing accessor that lists registered card IDs. If `CardEffectRegistry` has e.g. `card_ids() -> Vec<&str>` or similar, use it. If not, add one in `cards.rs`:

```rust
impl CardEffectRegistry {
    /// Return all card IDs that have a registered effect implementation.
    pub fn registered_card_ids(&self) -> Vec<String> {
        self.effects.keys().cloned().collect()
    }
}
```

(Inspect the actual struct field name first — `effects` is a guess; confirm against `cards.rs`.)

- [ ] **Step 2: Write the failing test**

```python
class TestLoadImplementedCardIds:
    def test_returns_set(self):
        from digimon_engine import load_implemented_card_ids
        ids = load_implemented_card_ids()
        assert isinstance(ids, set)
        # Should have at least the test cards (TEST-001..022)
        assert any(s.startswith("TEST-") for s in ids)
```

- [ ] **Step 3: Add the wrapper**

```rust
use ::digimon_engine::cards::build_registry;

#[pyfunction]
fn load_implemented_card_ids() -> HashSet<String> {
    build_registry().registered_card_ids().into_iter().collect()
}
```

If `build_registry()` is parameterized (takes a card DB or similar), pass the static one. If no zero-arg form exists, add a thin convenience helper in `cards.rs`.

- [ ] **Step 4: Rebuild and run**

```bash
cd digimon-engine-py && maturin develop --release && cd ..
python -m pytest tests/engine/test_rust_bindings_surface.py::TestLoadImplementedCardIds -v
```

Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/cards.rs digimon-engine-py/src/lib.rs tests/engine/test_rust_bindings_surface.py
git commit -m "$(cat <<'EOF'
feat(rust-bindings): expose load_implemented_card_ids

Returns the set of card IDs registered in the Rust CardEffectRegistry.
Replaces the Python helper that read _frozen_manifest.json from the
sunsetting Python script lane.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Add doc comment to `digimon-engine-py/src/lib.rs`

Update the file-level docstring to reflect the expanded surface so future readers know the bindings are no longer just `RustHeadlessGame`.

**Files:**
- Modify: `digimon-engine-py/src/lib.rs`.

- [ ] **Step 1: Replace the file-level docstring**

Open `digimon-engine-py/src/lib.rs`. Replace the existing top-of-file `//!` comment with:

```rust
//! PyO3 bindings for `digimon-engine`.
//!
//! Exposes:
//! - `RustHeadlessGame` — a 1:1 mirror of Python's
//!   `digimon_gym.engine.runners.headless_game.HeadlessGame`. Used by
//!   `DigimonEnv` (RL gym).
//! - `CardDatabase` / `PyCard` — static cards.json loader and per-card
//!   metadata wrapper.
//! - `parse_deck`, `parse_tts`, `parse_text`, `summarize_deck`,
//!   `validate_deck`, `out_of_set_cards`, `expand_deck_dict`,
//!   `restricted_list` — deck parsing, validation, and configuration.
//! - `CardKind`, `GamePhase` — engine enums.
//! - `load_tested_cards`, `is_card_tested` — tested-cards allowlist.
//! - `CardRegistry`, `REGISTRY_CAPACITY`, `EMBEDDING_DIM` — stable
//!   card-index registry for tensor encoding.
//! - `get_models_dir` — ONNX models directory resolver.
//! - `load_implemented_card_ids` — set of card IDs with registered
//!   Rust `CardEffect` implementations.
//!
//! Conventions:
//! - Deck ids are `list[str]`. DigiEggs are auto-routed into each player's
//!   digitama deck inside `Game::new`.
//! - Player ids are 1/2 on the Python side (matching Python engine), 0/1
//!   inside Rust. This layer converts.
//! - Action and tensor arrays are returned as zero-copy numpy `float32`.
```

- [ ] **Step 2: Commit**

```bash
git add digimon-engine-py/src/lib.rs
git commit -m "$(cat <<'EOF'
docs(rust-bindings): update lib.rs module docstring

Lists the Phase 2 expansion explicitly so future readers don't think
the bindings stop at RustHeadlessGame.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: End-to-end verification

**Files:** none (verification only).

- [ ] **Step 1: Full bindings test run**

```bash
cd digimon-engine-py && maturin develop --release && cd ..
python -m pytest tests/engine/test_rust_bindings_surface.py -v
```

Expected: every test added in Tasks 1–9 passes. Total ≈ 20+ tests.

- [ ] **Step 2: Cargo workspace tests**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml 2>&1 | tail -15
cargo test --manifest-path digimon-engine-py/Cargo.toml 2>&1 | tail -15
```

Expected: both green. Any new failures must be addressed before continuing.

- [ ] **Step 3: Default pytest (regression check on the rest of the repo)**

```bash
python -m pytest tests --ignore=tests/ai_pipeline --ignore=tests/api/test_admin_models.py 2>&1 | tail -5
```

Expected: failure count matches the pre-existing baseline captured in Phase 1's verification (39 pre-existing Windows-encoding failures). No new failures.

- [ ] **Step 4: Confirm no caller has been migrated yet**

Run: `grep -rn "from digimon_engine import\|import digimon_engine" --include="*.py" . | grep -v tests/engine/test_rust_bindings_surface.py | grep -v digimon-engine-py/`

Expected: zero matches. Phase 2 must not migrate any caller — that's Phase 3.

- [ ] **Step 5: Sanity check the sandbox import**

```bash
python -c "
import digimon_engine
print('exports:', sorted(name for name in dir(digimon_engine) if not name.startswith('_')))
"
```

Expected: list contains `CardDatabase`, `CardKind`, `CardRegistry`, `EMBEDDING_DIM`, `GamePhase`, `PyCard`, `PyCardRestriction`, `PyDeckValidationResult`, `REGISTRY_CAPACITY`, `RustHeadlessGame`, `expand_deck_dict`, `get_models_dir`, `is_card_tested`, `load_implemented_card_ids`, `load_tested_cards`, `out_of_set_cards`, `parse_deck`, `parse_text`, `parse_tts`, `restricted_list`, `summarize_deck`, `validate_deck`.

If any expected export is missing, the corresponding task didn't actually register its symbol — go back and fix.

---

## Out-of-Scope

Explicitly **not** done in this phase:

- Migrating any caller to `digimon_engine` imports. That's Phase 3.
- Removing the Python `digimon_gym.engine.data.*` modules. That's Phase 4 (after callers cut over).
- `PendingAction` and `PlayerType` enums. `PendingAction` is a vestigial Python-engine concept; `PlayerType` is server orchestration. Both are handled in Phase 3 / 5 without bindings.
- Adding a `pyo3-stub-gen` step to emit `.pyi` type stubs. Could be a follow-up if IDE support becomes a friction point.
- Touching the existing `RustHeadlessGame` surface. Untouched by Phase 2.
- Wrapping additional engine surfaces (game state inspection, effect introspection). Add later if Phase 3 surfaces a caller that needs them.

## Risks and Mitigations

- **Rust API drift between plan-write time and execution time.** Each task's "verify Rust signatures" step pushes the engineer to look at the actual Rust source before writing the wrapper. Don't skip those steps.
- **`maturin develop` rebuild is slow** (~30–60s per task). Acceptable for 9 binding tasks. If it becomes painful, switch to `maturin develop` (debug build) for iteration and `--release` only for the final run.
- **CardRegistry / build_registry shape uncertainty.** Tasks 7 and 9 explicitly hedge: inspect the actual API first, add a thin Rust helper if missing, escalate to RUST_PYTHON_PARITY.md if a true gap exists.
- **`RESTRICTED_LIST` data drift.** Task 4 asks the engineer to copy values from the Python source. Future restricted-list updates need to update both sides until Phase 4 deletes the Python copy.

## Plan complete

After Task 11 ships green: open the PR for Phase 2 and request review. Phase 3 (cut callers over to `digimon_engine`) gets its own plan when Phase 2 is merged.

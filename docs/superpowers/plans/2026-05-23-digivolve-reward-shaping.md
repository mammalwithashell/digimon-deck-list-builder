# Digivolve Reward Shaping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an opt-in reward shaping signal that nudges the RL pilot agent to digivolve more (especially DNA digivolve), via cumulative counters on the Rust `Game` struct that the Python wrapper turns into per-step rewards.

**Architecture:** Engine bumps `Game::n_digivolutions[player]` / `Game::n_dna_digivolutions[player]` after each successful digivolve action (regular and DNA). PyO3 exposes both counters for both players in the `get_rl_state` dict. `DigimonEnv` keeps prev-state mirrors for the agent (seat 1 only) and credits `+digivolve_reward` per regular digivolve and `+dna_digivolve_bonus` additional per DNA, gated behind a `TrainingConfig.digivolve_shaping` flag that defaults OFF. Shaping config is persisted into the `TrainingRunMetadata` sidecar so shaped/unshaped runs are mechanically distinguishable downstream.

**Tech Stack:** Rust (digimon-engine), PyO3 (digimon-engine-py), Python 3.11 (digimon_gym), Stable-Baselines3 (pilot_training callbacks), Gymnasium.

**Spec:** `docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md`

---

## Task 1: Engine — `Game` struct fields and initialization

**Files:**
- Modify: `code/digimon-engine/src/game.rs` (struct definition starting at line 200; `Game::new` / `Game::default` impls)
- Create: `code/digimon-engine/tests/digivolve_counters.rs`

- [ ] **Step 1: Write failing test for zero-initialized counters**

Create `code/digimon-engine/tests/digivolve_counters.rs`:

```rust
//! Integration tests for `Game::n_digivolutions` / `Game::n_dna_digivolutions`
//! counter instrumentation. These counters back the digivolve reward-shaping
//! signal in `DigimonEnv._compute_reward`; see
//! `docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md`.

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn new_game_starts_with_zero_digivolution_counters() {
    let runner = DebugRunner::builder().start();
    assert_eq!(runner.game.n_digivolutions, [0u32, 0u32]);
    assert_eq!(runner.game.n_dna_digivolutions, [0u32, 0u32]);
}
```

- [ ] **Step 2: Run the test — verify it fails on a missing field**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test digivolve_counters new_game_starts_with_zero_digivolution_counters`

Expected: compile error along the lines of `error[E0609]: no field 'n_digivolutions' on type 'Game'`.

- [ ] **Step 3: Add fields to the `Game` struct**

In `code/digimon-engine/src/game.rs`, find the `pub struct Game {` block (currently around line 200) and add the two fields immediately after `pub turn_count: u16,`:

```rust
    /// Cumulative regular-digivolve count per player. Incremented on every
    /// successful regular digivolve (including via DNA, which also bumps
    /// `n_dna_digivolutions`). Indexed by Rust 0-based PlayerId. Monotonic
    /// per game — never reset, never decremented. Backs the digivolve
    /// reward-shaping signal in DigimonEnv. See
    /// `docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md`.
    pub n_digivolutions: [u32; 2],
    /// Cumulative DNA-digivolve count per player. Incremented on every
    /// successful DNA digivolve, on top of `n_digivolutions`.
    pub n_dna_digivolutions: [u32; 2],
```

- [ ] **Step 4: Initialize the fields in every `Game` constructor**

Search for `Game` constructors and initializers in `code/digimon-engine/src/game.rs`. The struct is built in `Game::new` (and possibly via `..Default::default()` or struct-literal sites). For each constructor, add the two fields to the initializer block, e.g. for a struct literal:

```rust
            n_digivolutions: [0u32, 0u32],
            n_dna_digivolutions: [0u32, 0u32],
```

If a `Default` impl exists, prefer letting it cover these via `[0u32, 0u32]` (the default for `[u32; 2]`). Compile errors during `cargo check` will list any sites still missing the initializer.

- [ ] **Step 5: `cargo check` to find any missing initializer sites**

Run: `cargo check --manifest-path code/digimon-engine/Cargo.toml`

Expected: any "missing fields in initializer" errors point to additional `Game { ... }` literal sites — add the two fields there until `cargo check` is clean.

- [ ] **Step 6: Run the test — verify it passes**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test digivolve_counters new_game_starts_with_zero_digivolution_counters`

Expected: 1 test passed.

- [ ] **Step 7: Run the full engine test suite to verify no regressions**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`

Expected: all previously-passing tests still pass.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-engine/src/game.rs code/digimon-engine/tests/digivolve_counters.rs
git commit -m "engine: add n_digivolutions / n_dna_digivolutions counters to Game"
```

---

## Task 2: Engine — increment counters on regular digivolves

**Files:**
- Modify: `code/digimon-engine/src/game_actions.rs:2870` (`digivolve_onto`)
- Modify: `code/digimon-engine/src/game_actions.rs:3527` (`digivolve_from_hand_inner`)
- Modify: `code/digimon-engine/src/game_actions.rs:4073` (`digivolve_from_hand_onto_breeding`)
- Modify: `code/digimon-engine/tests/digivolve_counters.rs`

- [ ] **Step 1: Add failing test for regular-digivolve increment**

Append to `code/digimon-engine/tests/digivolve_counters.rs`:

```rust
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{GamePhase, PlaySource};

/// Drive a successful regular digivolve via `Game::digivolve_from_hand`
/// (which dispatches through `digivolve_from_hand_inner`) and assert the
/// regular counter incremented exactly once for the acting player, with
/// the DNA counter and the opponent's counters unchanged.
#[test]
fn regular_digivolve_from_hand_increments_only_active_player_regular_counter() {
    let base = make_test_card_with_level("BASE-LV4", "BaseLv4", 4);
    let evo = make_test_card_with_level("EVO-LV5", "EvoLv5", 5);

    let mut runner = DebugRunner::builder()
        .add_card(base)
        .add_card(evo)
        .hand(0, &["EVO-LV5"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE-LV4", Some(0));
    runner.game.current_phase = GamePhase::Main;

    let ok = runner
        .game
        .digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    assert!(ok, "regular digivolve must succeed in this setup");

    assert_eq!(runner.game.n_digivolutions, [1u32, 0u32]);
    assert_eq!(runner.game.n_dna_digivolutions, [0u32, 0u32]);
}
```

- [ ] **Step 2: Run the test — verify it fails**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test digivolve_counters regular_digivolve_from_hand_increments_only_active_player_regular_counter`

Expected: assertion failure — `n_digivolutions == [0, 0]` because the increment hasn't been wired yet.

- [ ] **Step 3: Add increment in `digivolve_from_hand_inner`**

In `code/digimon-engine/src/game_actions.rs`, find `fn digivolve_from_hand_inner` (around line 3527). After the function's legality and cost validation (i.e., after any early `return false` / `return None` paths but before any state mutation that would commit the digivolve), add:

```rust
        // Bump the digivolve counter for reward shaping. Placed after
        // legality+cost validation so failed/rejected attempts do not
        // credit the agent. See
        // docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
        self.n_digivolutions[player as usize] += 1;
```

Use whatever variable name the function uses for the acting player ID (likely `player: PlayerId`). `PlayerId` is `u8` per the engine convention, so `as usize` is the right cast for indexing `[u32; 2]`.

- [ ] **Step 4: Add the same increment in `digivolve_onto`**

In `code/digimon-engine/src/game_actions.rs`, find `pub fn digivolve_onto` (around line 2870). Same placement (after legality, before state mutation) and same one-line increment as Step 3.

- [ ] **Step 5: Add the same increment in `digivolve_from_hand_onto_breeding`**

In `code/digimon-engine/src/game_actions.rs`, find `pub fn digivolve_from_hand_onto_breeding` (around line 4073). Same placement and same one-line increment as Step 3.

- [ ] **Step 6: Run the test — verify it passes**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test digivolve_counters regular_digivolve_from_hand_increments_only_active_player_regular_counter`

Expected: 1 test passed.

- [ ] **Step 7: Run all digivolve-related tests to verify no regression**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action && cargo test --manifest-path code/digimon-engine/Cargo.toml phase_flow::digivolve_action`

Expected: all pre-existing digivolve tests still pass.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-engine/src/game_actions.rs code/digimon-engine/tests/digivolve_counters.rs
git commit -m "engine: increment n_digivolutions on the three regular digivolve sites"
```

---

## Task 3: Engine — increment counters on DNA digivolves

**Files:**
- Modify: `code/digimon-engine/src/game.rs:1947` (`dna_digivolve_inner`)
- Modify: `code/digimon-engine/src/game.rs:2069` (`dna_digivolve_hand_partner_inner`)
- Modify: `code/digimon-engine/tests/digivolve_counters.rs`

- [ ] **Step 1: Add failing test for DNA-digivolve increment with stacking**

Append to `code/digimon-engine/tests/digivolve_counters.rs`:

```rust
use digimon_engine::debug_runner::make_test_dna_card;

/// Drive a successful DNA digivolve via `Game::initiate_dna_digivolve` and
/// the two selection stages. Assert that **both** counters incremented
/// for the active player (DNA stacks on regular per spec decision 5),
/// and the opponent's counters did not move.
#[test]
fn dna_digivolve_increments_both_active_player_counters_once() {
    let lv5 = make_test_card_with_level("TST-LV5", "FiveDigi", 5);
    let lv6 = make_test_card_with_level("TST-LV6", "SixDigi", 6);
    let dna = make_test_dna_card("TST-DNA", "DnaDigi", 5, 6, 0);

    let mut runner = DebugRunner::builder()
        .add_card(lv5)
        .add_card(lv6)
        .add_card(dna)
        .hand(0, &["TST-DNA"])
        .memory(5)
        .start();

    let handle_lv5 = runner.place_on_field(0, "TST-LV5", None);
    let handle_lv6 = runner.place_on_field(0, "TST-LV6", None);
    runner.game.current_phase = GamePhase::Main;

    assert!(runner.game.initiate_dna_digivolve(0, 0));
    runner
        .game
        .resolve_selection(0, handle_lv5.index as u16)
        .expect("stage 1");
    runner
        .game
        .resolve_selection(0, handle_lv6.index as u16)
        .expect("stage 2");

    assert_eq!(runner.game.n_digivolutions, [1u32, 0u32]);
    assert_eq!(runner.game.n_dna_digivolutions, [1u32, 0u32]);
}
```

- [ ] **Step 2: Run the test — verify it fails**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test digivolve_counters dna_digivolve_increments_both_active_player_counters_once`

Expected: assertion failure — counters are still `[0, 0]` because DNA increments haven't been wired.

- [ ] **Step 3: Add increments in `dna_digivolve_inner`**

In `code/digimon-engine/src/game.rs`, find `pub(crate) fn dna_digivolve_inner` (around line 1947). After legality and cost validation, before state mutation, add:

```rust
        // DNA digivolves stack on the regular counter per spec decision 5:
        // a single `digivolve_reward` line in DigimonEnv always fires, plus
        // a separate `dna_digivolve_bonus` line fires only on DNAs. See
        // docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
        self.n_digivolutions[player as usize] += 1;
        self.n_dna_digivolutions[player as usize] += 1;
```

Use whatever the function's local variable name is for the acting player.

- [ ] **Step 4: Add the same two-counter increment in `dna_digivolve_hand_partner_inner`**

In `code/digimon-engine/src/game.rs`, find `pub(crate) fn dna_digivolve_hand_partner_inner` (around line 2069). Same placement and same two-line increment as Step 3.

- [ ] **Step 5: Run the test — verify it passes**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test digivolve_counters dna_digivolve_increments_both_active_player_counters_once`

Expected: 1 test passed.

- [ ] **Step 6: Run all DNA-related tests to verify no regression**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action`

Expected: all 4 pre-existing DNA tests still pass.

- [ ] **Step 7: Run the entire engine test suite**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-engine/src/game.rs code/digimon-engine/tests/digivolve_counters.rs
git commit -m "engine: bump both n_digivolutions and n_dna_digivolutions on DNA digivolve sites"
```

---

## Task 4: Engine — lock the "after legality, before mutation" placement with an invariant test

**Files:**
- Modify: `code/digimon-engine/tests/digivolve_counters.rs`

- [ ] **Step 1: Add invariant test for rejected DNA digivolve**

Append to `code/digimon-engine/tests/digivolve_counters.rs`:

```rust
/// If a DNA digivolve is rejected for a phase-illegality reason (here:
/// invoking it outside the Main phase), the counters must stay at zero.
/// This locks the implementation choice "increment after legality
/// validation, before state mutation" — any refactor that moves the
/// bump earlier will fail this test.
#[test]
fn rejected_dna_digivolve_does_not_increment_counters() {
    let dna = make_test_dna_card("TST-DNA", "DnaDigi", 5, 6, 0);
    let mut runner = DebugRunner::builder()
        .add_card(dna)
        .hand(0, &["TST-DNA"])
        .start();

    // Non-Main phase: initiate_dna_digivolve should reject up-front.
    runner.game.current_phase = GamePhase::EndTurn;

    let ok = runner.game.initiate_dna_digivolve(0, 0);
    assert!(!ok, "non-Main phase must reject the DNA digivolve");

    assert_eq!(runner.game.n_digivolutions, [0u32, 0u32]);
    assert_eq!(runner.game.n_dna_digivolutions, [0u32, 0u32]);
}
```

- [ ] **Step 2: Run the test — it should pass already**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test digivolve_counters rejected_dna_digivolve_does_not_increment_counters`

Expected: PASS. The increments live after legality validation, so a rejected attempt never reaches them. If this test ever fails, somebody has moved the increments to a too-early position in the function.

- [ ] **Step 3: Commit**

```bash
git add code/digimon-engine/tests/digivolve_counters.rs
git commit -m "engine: pin 'after legality, before mutation' counter placement with an invariant test"
```

---

## Task 5: PyO3 — expose counters in `get_rl_state` dict

**Files:**
- Modify: `code/digimon-engine-py/src/lib.rs:703` (inside `RustHeadlessGame::get_rl_state`)
- Create: `code/tests/engine/test_rust_digivolve_counters.py`

- [ ] **Step 1: Write the Python smoke test (will fail because the binding isn't built yet)**

Create `code/tests/engine/test_rust_digivolve_counters.py`:

```python
"""Smoke test that the four digivolve counters round-trip through the PyO3
binding and follow the Python 1/2 player-ID convention.

Catches binding-key typos and mis-indexed `[u32; 2]` -> dict-key mappings
that the Rust-only integration test in `tests/digivolve_counters.rs`
cannot see.
"""

from __future__ import annotations

import pytest

pytest.importorskip("digimon_engine")

from digimon_engine import RustHeadlessGame  # noqa: E402


def test_get_rl_state_exposes_digivolve_counters_for_both_players() -> None:
    game = RustHeadlessGame()
    state = game.get_rl_state()

    assert state["p1_digivolutions"] == 0
    assert state["p2_digivolutions"] == 0
    assert state["p1_dna_digivolutions"] == 0
    assert state["p2_dna_digivolutions"] == 0
```

- [ ] **Step 2: Run the test — it should fail (key missing)**

Run: `python -m pytest code/tests/engine/test_rust_digivolve_counters.py -v`

Expected: `KeyError: 'p1_digivolutions'` (or similar) because the binding doesn't expose the key yet.

- [ ] **Step 3: Append four `set_item` calls in `get_rl_state`**

In `code/digimon-engine-py/src/lib.rs`, find `RustHeadlessGame::get_rl_state` (around line 680). After the existing `d.set_item("p2_total_dp", ...)` line (around line 703), add:

```rust
        d.set_item("p1_digivolutions", game.n_digivolutions[0])?;
        d.set_item("p2_digivolutions", game.n_digivolutions[1])?;
        d.set_item("p1_dna_digivolutions", game.n_dna_digivolutions[0])?;
        d.set_item("p2_dna_digivolutions", game.n_dna_digivolutions[1])?;
```

Note: `p1` maps to Rust index `0`, `p2` to Rust index `1` — same convention as `p1_security` / `p2_security`.

- [ ] **Step 4: Rebuild the PyO3 binding**

Run: `cd code/digimon-engine-py && maturin develop && cd -`

Expected: successful compile, "Installed digimon_engine" message.

- [ ] **Step 5: Run the test — verify it passes**

Run: `python -m pytest code/tests/engine/test_rust_digivolve_counters.py -v`

Expected: 1 test passed.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine-py/src/lib.rs code/tests/engine/test_rust_digivolve_counters.py
git commit -m "pyo3: expose digivolve counters in get_rl_state dict"
```

---

## Task 6: Python — `TrainingConfig` fields and validation

**Files:**
- Modify: `code/digimon_gym/agents/training_config.py:63` (after `mulligan_log` field) and `_validate` method
- Create: `code/tests/rl/test_training_config_digivolve.py`

- [ ] **Step 1: Write the failing test for the new config fields**

Create `code/tests/rl/test_training_config_digivolve.py`:

```python
"""Tests for the digivolve-shaping fields on TrainingConfig."""

from __future__ import annotations

import pytest

from digimon_gym.agents.training_config import TrainingConfig


def test_defaults_are_off_and_unshaped() -> None:
    cfg = TrainingConfig()
    assert cfg.digivolve_shaping is False
    assert cfg.digivolve_reward == 0.1
    assert cfg.dna_digivolve_bonus == 0.3


def test_negative_digivolve_reward_rejected() -> None:
    with pytest.raises(ValueError, match="digivolve_reward must be >= 0"):
        TrainingConfig(digivolve_reward=-0.01)


def test_negative_dna_bonus_rejected() -> None:
    with pytest.raises(ValueError, match="dna_digivolve_bonus must be >= 0"):
        TrainingConfig(dna_digivolve_bonus=-0.5)


def test_zero_reward_and_bonus_accepted() -> None:
    cfg = TrainingConfig(digivolve_reward=0.0, dna_digivolve_bonus=0.0)
    assert cfg.digivolve_reward == 0.0
    assert cfg.dna_digivolve_bonus == 0.0
```

- [ ] **Step 2: Run the test — verify it fails**

Run: `python -m pytest code/tests/rl/test_training_config_digivolve.py -v`

Expected: `AttributeError` / `TypeError` for unknown kwargs / missing attributes.

- [ ] **Step 3: Add the three fields to `TrainingConfig`**

In `code/digimon_gym/agents/training_config.py`, immediately after the `mulligan_log: str = "on"` line (currently line 63), add:

```python
    # Digivolve reward shaping (asymmetric — agent only, never opponent).
    # All three default OFF/zero so existing runs are byte-identical when
    # users don't set them.  See
    # docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
    digivolve_shaping: bool = False
    digivolve_reward: float = 0.1       # per regular digivolve
    dna_digivolve_bonus: float = 0.3    # additional on top of digivolve_reward
```

- [ ] **Step 4: Extend `_validate`**

Still in `code/digimon_gym/agents/training_config.py`, at the end of `_validate` (i.e., add immediately before its return / its final line), append:

```python
        if self.digivolve_reward < 0:
            raise ValueError("digivolve_reward must be >= 0")
        if self.dna_digivolve_bonus < 0:
            raise ValueError("dna_digivolve_bonus must be >= 0")
```

- [ ] **Step 5: Run the test — verify it passes**

Run: `python -m pytest code/tests/rl/test_training_config_digivolve.py -v`

Expected: 4 tests passed.

- [ ] **Step 6: Run pre-existing TrainingConfig tests to confirm no regression**

Run: `python -m pytest code/tests/rl/ -k 'training_config' -v`

Expected: all pre-existing config tests still pass.

- [ ] **Step 7: Commit**

```bash
git add code/digimon_gym/agents/training_config.py code/tests/rl/test_training_config_digivolve.py
git commit -m "config: add digivolve_shaping / digivolve_reward / dna_digivolve_bonus to TrainingConfig"
```

---

## Task 7: Python — `DigimonEnv` constructor kwargs, prev-state mirror, reset

**Files:**
- Modify: `code/digimon_gym/digimon_gym.py:145` (`DigimonEnv.__init__`), `:183-184` (prev-state init block), `:312-313` (reset block)
- Create: `code/tests/rl/test_digivolve_shaping.py`

- [ ] **Step 1: Write the failing test for constructor kwargs and reset behavior**

Create `code/tests/rl/test_digivolve_shaping.py`:

```python
"""Tests for digivolve reward shaping in DigimonEnv.

The reward-math tests come in Task 8 (`test_reward_credits_*`); this file
also gets the snapshot regression test in Task 9. This first test only
exercises the new constructor / state-mirror surface.
"""

from __future__ import annotations

from digimon_gym.digimon_gym import DigimonEnv


def test_constructor_accepts_shaping_kwargs_and_resets_prev_state() -> None:
    env = DigimonEnv(
        digivolve_shaping=True,
        digivolve_reward=0.1,
        dna_digivolve_bonus=0.3,
    )

    assert env.digivolve_shaping is True
    assert env.digivolve_reward == 0.1
    assert env.dna_digivolve_bonus == 0.3
    assert env._prev_p1_digivolutions is None
    assert env._prev_p1_dna_digivolutions is None

    obs, info = env.reset()
    assert env._prev_p1_digivolutions is None
    assert env._prev_p1_dna_digivolutions is None


def test_constructor_defaults_are_off() -> None:
    env = DigimonEnv()
    assert env.digivolve_shaping is False
    assert env.digivolve_reward == 0.1
    assert env.dna_digivolve_bonus == 0.3
```

- [ ] **Step 2: Run the test — verify it fails**

Run: `python -m pytest code/tests/rl/test_digivolve_shaping.py -v`

Expected: `TypeError: __init__() got an unexpected keyword argument 'digivolve_shaping'`.

- [ ] **Step 3: Add the three kwargs to `DigimonEnv.__init__`**

In `code/digimon_gym/digimon_gym.py`, find `def __init__(self, deck1: ...` (around line 145). Add three kwargs to the signature (anywhere after `deck2`, but before `**kwargs` if present), e.g.:

```python
    def __init__(self, deck1: Optional[List[str]] = None,
                 deck2: Optional[List[str]] = None,
                 # ... existing kwargs ...
                 digivolve_shaping: bool = False,
                 digivolve_reward: float = 0.1,
                 dna_digivolve_bonus: float = 0.3,
                 # ...
                 ):
```

(Insert in whatever spot keeps the rest of the signature readable. Defaults match `TrainingConfig`.)

In the `__init__` body, store them on `self`:

```python
        self.digivolve_shaping = digivolve_shaping
        self.digivolve_reward = digivolve_reward
        self.dna_digivolve_bonus = dna_digivolve_bonus
```

- [ ] **Step 4: Add the prev-state mirror initialization**

Still in `__init__`, immediately after the existing `self._prev_p2_security: Optional[int] = None` line (currently line 184), add:

```python
        self._prev_p1_digivolutions: Optional[int] = None
        self._prev_p1_dna_digivolutions: Optional[int] = None
```

We mirror only the agent (`p1`) — the wrapper is asymmetric per spec decision 2.

- [ ] **Step 5: Clear the prev-state mirror in `reset`**

In `DigimonEnv.reset` (around line 309), find the existing two lines:

```python
        self._prev_p1_security = None
        self._prev_p2_security = None
```

Add immediately after them:

```python
        self._prev_p1_digivolutions = None
        self._prev_p1_dna_digivolutions = None
```

- [ ] **Step 6: Run the test — verify it passes**

Run: `python -m pytest code/tests/rl/test_digivolve_shaping.py -v`

Expected: 2 tests passed.

- [ ] **Step 7: Run the wider DigimonEnv test suite — verify no regression**

Run: `python -m pytest code/tests/rl/ -k 'digimon_env or env_' -v`

Expected: pre-existing env tests still pass (or, if no such tests are gathered by that selector, the command completes with no failures).

- [ ] **Step 8: Commit**

```bash
git add code/digimon_gym/digimon_gym.py code/tests/rl/test_digivolve_shaping.py
git commit -m "env: add digivolve-shaping kwargs and prev-state mirror to DigimonEnv"
```

---

## Task 8: Python — `_compute_reward` extension and reward-math tests

**Files:**
- Modify: `code/digimon_gym/digimon_gym.py:376-439` (`DigimonEnv._compute_reward`)
- Modify: `code/tests/rl/test_digivolve_shaping.py`

- [ ] **Step 1: Add the failing reward-math test**

Append to `code/tests/rl/test_digivolve_shaping.py`:

```python
import math

import pytest


def _make_shaped_env() -> DigimonEnv:
    return DigimonEnv(
        digivolve_shaping=True,
        digivolve_reward=0.1,
        dna_digivolve_bonus=0.3,
    )


def test_first_step_credits_no_shaping_reward() -> None:
    """`_prev_*=None` on the very first step must mean zero shaping credit,
    matching the existing security-delta convention."""
    env = _make_shaped_env()
    env.reset()
    # Drive the agent's RL state to "1 digivolve happened" before computing.
    env._prev_p1_digivolutions = None
    env._prev_p1_dna_digivolutions = None

    # Pretend the engine reports one digivolve in the state — but because
    # `_prev_*` is None, no reward should be credited yet.
    state = {
        "game_over": False,
        "p1_security": 5,
        "p2_security": 5,
        "p1_digivolutions": 1,
        "p1_dna_digivolutions": 0,
    }
    # Patch `_rl_state` for this assertion only.
    env._rl_state = lambda: state  # type: ignore[method-assign]
    reward = env._compute_reward(terminated=False)
    # Only the step penalty (-0.001), no shaping credit, no security delta.
    assert math.isclose(reward, -0.001, abs_tol=1e-9)


def test_regular_digivolve_credits_digivolve_reward() -> None:
    env = _make_shaped_env()
    env.reset()
    # Prime prev-state as if the previous step had zero digivolutions.
    env._prev_p1_digivolutions = 0
    env._prev_p1_dna_digivolutions = 0
    env._prev_p1_security = 5
    env._prev_p2_security = 5

    state = {
        "game_over": False,
        "p1_security": 5,
        "p2_security": 5,
        "p1_digivolutions": 1,
        "p1_dna_digivolutions": 0,
    }
    env._rl_state = lambda: state  # type: ignore[method-assign]
    reward = env._compute_reward(terminated=False)
    # +0.1 shaping − 0.001 step penalty.
    assert math.isclose(reward, 0.1 - 0.001, abs_tol=1e-9)


def test_dna_digivolve_credits_full_dna_band() -> None:
    env = _make_shaped_env()
    env.reset()
    env._prev_p1_digivolutions = 0
    env._prev_p1_dna_digivolutions = 0
    env._prev_p1_security = 5
    env._prev_p2_security = 5

    # DNA digivolve stacks on regular: both counters jump by 1.
    state = {
        "game_over": False,
        "p1_security": 5,
        "p2_security": 5,
        "p1_digivolutions": 1,
        "p1_dna_digivolutions": 1,
    }
    env._rl_state = lambda: state  # type: ignore[method-assign]
    reward = env._compute_reward(terminated=False)
    # +0.1 regular + 0.3 DNA bonus − 0.001 step penalty.
    assert math.isclose(reward, 0.4 - 0.001, abs_tol=1e-9)


def test_non_digivolve_step_has_no_shaping_credit() -> None:
    env = _make_shaped_env()
    env.reset()
    env._prev_p1_digivolutions = 2
    env._prev_p1_dna_digivolutions = 1
    env._prev_p1_security = 5
    env._prev_p2_security = 5

    state = {
        "game_over": False,
        "p1_security": 5,
        "p2_security": 5,
        "p1_digivolutions": 2,
        "p1_dna_digivolutions": 1,
    }
    env._rl_state = lambda: state  # type: ignore[method-assign]
    reward = env._compute_reward(terminated=False)
    # Only the step penalty.
    assert math.isclose(reward, -0.001, abs_tol=1e-9)


def test_shaping_off_credits_nothing_even_with_digivolve_delta() -> None:
    env = DigimonEnv(digivolve_shaping=False)
    env.reset()
    env._prev_p1_digivolutions = 0
    env._prev_p1_dna_digivolutions = 0
    env._prev_p1_security = 5
    env._prev_p2_security = 5

    state = {
        "game_over": False,
        "p1_security": 5,
        "p2_security": 5,
        "p1_digivolutions": 1,
        "p1_dna_digivolutions": 1,
    }
    env._rl_state = lambda: state  # type: ignore[method-assign]
    reward = env._compute_reward(terminated=False)
    # Only the step penalty — no shaping when the flag is off.
    assert math.isclose(reward, -0.001, abs_tol=1e-9)
```

- [ ] **Step 2: Run the new tests — verify they fail**

Run: `python -m pytest code/tests/rl/test_digivolve_shaping.py -v`

Expected: assertion failures (rewards come out as `-0.001` everywhere because no shaping has been added yet).

- [ ] **Step 3: Extend `_compute_reward` with the shaping block**

In `code/digimon_gym/digimon_gym.py`, find `_compute_reward` (around line 376). Locate the lines that update the security prevs and compute `dense_reward`:

```python
        self._prev_p1_security = p1_sec
        self._prev_p2_security = p2_sec

        # Per-step stalling penalty (small but non-zero).
        return dense_reward - 0.001
```

Insert the new block **between** the security-prev update and the final return:

```python
        self._prev_p1_security = p1_sec
        self._prev_p2_security = p2_sec

        # Digivolve reward shaping — asymmetric (agent only) and opt-in.
        # See docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
        if self.digivolve_shaping:
            p1_digi = int(state.get("p1_digivolutions", 0))
            p1_dna = int(state.get("p1_dna_digivolutions", 0))

            if (
                self._prev_p1_digivolutions is not None
                and self._prev_p1_dna_digivolutions is not None
            ):
                # DNA stacks on regular in the engine, so the regular reward
                # always fires and the DNA bonus is additive.
                d_digi = p1_digi - self._prev_p1_digivolutions
                d_dna = p1_dna - self._prev_p1_dna_digivolutions
                if d_digi > 0:
                    dense_reward += float(d_digi) * self.digivolve_reward
                if d_dna > 0:
                    dense_reward += float(d_dna) * self.dna_digivolve_bonus

            self._prev_p1_digivolutions = p1_digi
            self._prev_p1_dna_digivolutions = p1_dna

        # Per-step stalling penalty (small but non-zero).
        return dense_reward - 0.001
```

- [ ] **Step 4: Run the tests — verify they pass**

Run: `python -m pytest code/tests/rl/test_digivolve_shaping.py -v`

Expected: 7 tests passed (2 from Task 7 + 5 from this task).

- [ ] **Step 5: Commit**

```bash
git add code/digimon_gym/digimon_gym.py code/tests/rl/test_digivolve_shaping.py
git commit -m "env: credit digivolve_reward (+0.1) and dna_digivolve_bonus (+0.3) in _compute_reward"
```

---

## Task 9: Python — byte-identical-default regression test

**Files:**
- Modify: `code/tests/rl/test_digivolve_shaping.py`

- [ ] **Step 1: Write the regression test**

Append to `code/tests/rl/test_digivolve_shaping.py`:

```python
def test_shaping_off_default_matches_baseline_reward_path() -> None:
    """When shaping is OFF (the default for unset callers), `_compute_reward`
    must produce numerically identical output to the pre-feature shape for
    any sequence of step states. This protects against accidental behavior
    drift in pre-existing runs that don't opt into shaping.
    """
    env = DigimonEnv()  # all shaping kwargs at defaults; shaping is OFF
    env.reset()
    env._prev_p1_security = 5
    env._prev_p2_security = 5
    env._prev_p1_digivolutions = 0
    env._prev_p1_dna_digivolutions = 0

    # Three step states: no change, opponent security removed, own security
    # lost. Each also reports digivolve activity that MUST NOT credit when
    # shaping is OFF.
    cases = [
        # (state, expected_reward)
        ({
            "game_over": False,
            "p1_security": 5, "p2_security": 5,
            "p1_digivolutions": 1, "p1_dna_digivolutions": 1,
        }, -0.001),                  # only step penalty
        ({
            "game_over": False,
            "p1_security": 5, "p2_security": 4,
            "p1_digivolutions": 2, "p1_dna_digivolutions": 2,
        }, 2.0 - 0.001),             # opponent security removed
        ({
            "game_over": False,
            "p1_security": 4, "p2_security": 4,
            "p1_digivolutions": 3, "p1_dna_digivolutions": 3,
        }, -2.0 - 0.001),            # own security lost
    ]

    for state, expected in cases:
        env._rl_state = lambda s=state: s  # type: ignore[method-assign]
        reward = env._compute_reward(terminated=False)
        assert math.isclose(reward, expected, abs_tol=1e-9), (
            f"shaping-off reward {reward} != baseline {expected} for state {state}"
        )
```

- [ ] **Step 2: Run the test — verify it passes**

Run: `python -m pytest code/tests/rl/test_digivolve_shaping.py::test_shaping_off_default_matches_baseline_reward_path -v`

Expected: PASS. Confirms that with `digivolve_shaping=False` (the default), `_compute_reward` is byte-identical to the pre-feature behavior even when the dict reports digivolve counters.

- [ ] **Step 3: Commit**

```bash
git add code/tests/rl/test_digivolve_shaping.py
git commit -m "test: pin byte-identical reward when digivolve_shaping is off"
```

---

## Task 10: Python — thread config into all `DigimonEnv(...)` callsites in `pilot_training.py`

**Files:**
- Modify: `code/digimon_gym/agents/pilot_training.py` (callsites at lines 816, 919, 1196/1210/1220, 1321)

- [ ] **Step 1: Locate every `DigimonEnv(` callsite**

Run: `python -m grep --line-number 'DigimonEnv(' code/digimon_gym/agents/pilot_training.py` (or use any equivalent search).

Expected: the callsites at lines 816, 919, 1196, 1210, 1220, 1321 (one per training mode / opponent variant).

- [ ] **Step 2: Update each `DigimonEnv(...)` call to thread the three config fields**

For each of the six callsites, add three kwargs immediately after the existing `deck2=deck2` line:

```python
            digivolve_shaping=cfg.digivolve_shaping,
            digivolve_reward=cfg.digivolve_reward,
            dna_digivolve_bonus=cfg.dna_digivolve_bonus,
```

(Indent to match the surrounding `DigimonEnv(...)` call. The variable will be named `cfg` in every site — confirm by reading 5 lines above each callsite.)

- [ ] **Step 3: Run the wider pilot-training import-smoke**

Run: `python -c "from digimon_gym.agents.pilot_training import _build_argparser; p = _build_argparser(); print(p.format_help()[:200])"`

Expected: argparser loads and prints help text — confirms the module is importable.

- [ ] **Step 4: Run a 200-step shaped training smoke**

Run:

```bash
python -m digimon_gym.agents.pilot_training \
  --config configs/training/default.yaml \
  --set timesteps=200 \
  --set digivolve_shaping=true \
  --set digivolve_reward=0.1 \
  --set dna_digivolve_bonus=0.3 \
  --set eval_freq=0 \
  --set checkpoint_every=0
```

(Config path is `configs/training/default.yaml` at the repo root, confirmed via glob.)

Expected: training completes its 200 steps without raising. The shaped reward path is exercised.

- [ ] **Step 5: Run a 200-step unshaped training smoke for parity**

Run:

```bash
python -m digimon_gym.agents.pilot_training \
  --config configs/training/default.yaml \
  --set timesteps=200 \
  --set eval_freq=0 \
  --set checkpoint_every=0
```

Expected: training completes with no behavioral difference from before this task (shaping defaults OFF).

- [ ] **Step 6: Commit**

```bash
git add code/digimon_gym/agents/pilot_training.py
git commit -m "training: thread digivolve_shaping config into every DigimonEnv callsite"
```

---

## Task 11: Python — TensorBoard scalars in `WinRateCallback`

**Files:**
- Modify: `code/digimon_gym/agents/pilot_training.py:323` (`WinRateCallback`), lines ~557-559 (eval-aggregate logging block)

- [ ] **Step 1: Read the eval-aggregate block to find the right insertion point**

Open `code/digimon_gym/agents/pilot_training.py` around lines 540-580. The existing pattern is:

```python
        self.logger.record("pilot/win_rate", win_rate)
        ...
        self.logger.record("pilot/mean_eval_terminal_score", mean_terminal_score)
        self.logger.record("pilot/mean_eval_dense_reward", mean_dense_reward)
        self.logger.record("pilot/mean_eval_episode_length", mean_length)
        self.logger.record("pilot/games_played", self.games_played)
```

Find the surrounding loop / aggregation code that produces `mean_length` etc.; each eval game terminates with the final `_rl_state()` available either on the env or via the eval rollout. Identify where each game's terminal `state["p1_digivolutions"]` could be collected — most likely already inside the per-eval-episode loop where `mean_dense_reward` is summed.

- [ ] **Step 2: Add per-eval-episode counters and average them**

Inside the per-eval-episode loop where existing per-episode aggregates accumulate (e.g. `total_dense_reward += ...`), add per-episode reads:

```python
            # Read digivolve counters from the env's last RL state. Both
            # players are exposed but only p1 (the agent) is observational here.
            try:
                final_state = env.unwrapped._rl_state()  # type: ignore[attr-defined]
                ep_digivolves = int(final_state.get("p1_digivolutions", 0))
                ep_dna_digivolves = int(final_state.get("p1_dna_digivolutions", 0))
            except Exception:
                ep_digivolves = 0
                ep_dna_digivolves = 0
            total_digivolves += ep_digivolves
            total_dna_digivolves += ep_dna_digivolves
```

Initialize `total_digivolves = 0` and `total_dna_digivolves = 0` next to the other `total_*` accumulators above the loop.

After the loop (next to the other `mean_*` computations), compute:

```python
        mean_digivolves_per_game = (
            total_digivolves / float(self.games_played) if self.games_played else 0.0
        )
        mean_dna_digivolves_per_game = (
            total_dna_digivolves / float(self.games_played) if self.games_played else 0.0
        )
```

- [ ] **Step 3: Log the new scalars right after `mean_eval_episode_length`**

In the existing log block at ~line 559, add:

```python
        self.logger.record("pilot/mean_eval_digivolves_per_game", mean_digivolves_per_game)
        self.logger.record("pilot/mean_eval_dna_digivolves_per_game", mean_dna_digivolves_per_game)
```

Both scalars fire **regardless of the `digivolve_shaping` flag** — they're observational, not reward-gated. Logging them in unshaped runs gives us the baseline curve.

- [ ] **Step 4: Run a short training smoke with eval enabled**

Run:

```bash
python -m digimon_gym.agents.pilot_training \
  --config configs/training/default.yaml \
  --set timesteps=500 \
  --set eval_freq=200 \
  --set eval_episodes=2 \
  --set checkpoint_every=0
```

Expected: training completes, evaluation fires at least once, no exceptions raised during eval. The new scalars should be readable from the TB event file.

- [ ] **Step 5 (manual): Verify the scalars in TensorBoard logs**

Run: `find runs/ -name '*.tfevents.*' -newer code/digimon_gym/agents/pilot_training.py -mmin -5 | head -1`

Expected: a fresh event file path. (No need to start a TensorBoard server — the smoke from Step 4 confirms `self.logger.record` accepted the keys.) If you want a stronger check, run `python -c "from tensorboard.backend.event_processing import event_accumulator; ea = event_accumulator.EventAccumulator('<event-file>'); ea.Reload(); print([t for t in ea.Tags()['scalars'] if 'digivolves' in t])"` and confirm the two new tags are listed.

- [ ] **Step 6: Commit**

```bash
git add code/digimon_gym/agents/pilot_training.py
git commit -m "training: log mean_eval_digivolves_per_game and mean_eval_dna_digivolves_per_game"
```

---

## Task 12: Python — sidecar fields and round-trip test

**Files:**
- Modify: `code/digimon_gym/agents/training_metrics.py:42` (`TrainingRunMetadata` dataclass)
- Modify: `code/digimon_gym/agents/pilot_training.py` (the site where `TrainingRunMetadata(...)` is constructed and populated from `cfg`)
- Create: `code/tests/rl/test_training_metrics_digivolve_sidecar.py`

- [ ] **Step 1: Write the failing round-trip test**

Create `code/tests/rl/test_training_metrics_digivolve_sidecar.py`:

```python
"""Sidecar round-trip test for digivolve-shaping fields on TrainingRunMetadata."""

from __future__ import annotations

from pathlib import Path

from digimon_gym.agents.training_metrics import TrainingRunMetadata


def test_sidecar_round_trips_digivolve_fields(tmp_path: Path) -> None:
    meta = TrainingRunMetadata(
        run_id="test-run",
        started_at="2026-05-23T00:00:00Z",
        digivolve_shaping=True,
        digivolve_reward=0.1,
        dna_digivolve_bonus=0.3,
    )

    out = tmp_path / "metadata.json"
    meta.save(out)
    loaded = TrainingRunMetadata.load(out)

    assert loaded.digivolve_shaping is True
    assert loaded.digivolve_reward == 0.1
    assert loaded.dna_digivolve_bonus == 0.3


def test_sidecar_legacy_file_loads_with_unshaped_defaults(tmp_path: Path) -> None:
    """A pre-feature sidecar (no digivolve_* keys) must load and produce
    correct unshaped semantics."""
    legacy = tmp_path / "legacy.json"
    legacy.write_text('{"run_id": "legacy", "started_at": "2026-04-01T00:00:00Z"}')

    loaded = TrainingRunMetadata.load(legacy)
    assert loaded.digivolve_shaping is False
    assert loaded.digivolve_reward == 0.0
    assert loaded.dna_digivolve_bonus == 0.0
```

- [ ] **Step 2: Run the test — verify it fails**

Run: `python -m pytest code/tests/rl/test_training_metrics_digivolve_sidecar.py -v`

Expected: `TypeError: __init__() got an unexpected keyword argument 'digivolve_shaping'`.

- [ ] **Step 3: Add the three fields to `TrainingRunMetadata`**

In `code/digimon_gym/agents/training_metrics.py`, inside the `TrainingRunMetadata` dataclass (around line 42), add three fields parallel to `training_seed` / `eval_seed`. Insert near the other run-config fields (e.g. just after `eval_seed: int | None = None`, around line 68):

```python
    # Digivolve reward shaping config (persisted from TrainingConfig so
    # downstream tooling can filter/group shaped vs. unshaped runs without
    # introspecting the hyperparameters dict). Defaults are zero/False so
    # pre-feature sidecars round-trip with correct unshaped semantics.
    digivolve_shaping: bool = False
    digivolve_reward: float = 0.0
    dna_digivolve_bonus: float = 0.0
```

Note the defaults: `0.0` here (not `0.1` / `0.3` as in `TrainingConfig`) — pre-feature sidecars on disk don't have these keys, and a legacy load that defaulted to `digivolve_reward=0.1` would mislabel the run as "shaped at 0.1" instead of "unshaped" (because `digivolve_shaping=False` plus `digivolve_reward=0.1` looks like the shaping happened to be skipped, not absent). Default zero is unambiguous.

- [ ] **Step 4: Populate the fields where `TrainingRunMetadata(...)` is constructed**

In `code/digimon_gym/agents/pilot_training.py`, find every site where `TrainingRunMetadata(...)` is instantiated (grep for `TrainingRunMetadata(`). For each, copy the three values from `cfg`:

```python
        digivolve_shaping=cfg.digivolve_shaping,
        digivolve_reward=cfg.digivolve_reward,
        dna_digivolve_bonus=cfg.dna_digivolve_bonus,
```

(Only `cfg.digivolve_reward` / `cfg.dna_digivolve_bonus` get persisted into the sidecar — when `digivolve_shaping=False` the reward/bonus values are inert, but persisting them makes the configuration record self-describing.)

- [ ] **Step 5: Run the test — verify it passes**

Run: `python -m pytest code/tests/rl/test_training_metrics_digivolve_sidecar.py -v`

Expected: 2 tests passed.

- [ ] **Step 6: Run pre-existing training_metrics tests for no regression**

Run: `python -m pytest code/tests/rl/ -k 'training_metrics' -v`

Expected: any pre-existing metric-related tests still pass.

- [ ] **Step 7: Commit**

```bash
git add code/digimon_gym/agents/training_metrics.py code/digimon_gym/agents/pilot_training.py code/tests/rl/test_training_metrics_digivolve_sidecar.py
git commit -m "metrics: persist digivolve shaping config into TrainingRunMetadata sidecar"
```

---

## Final verification

- [ ] **Step 1: Run the full engine test suite**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`

Expected: all tests pass.

- [ ] **Step 2: Run the full Python test suite**

Run: `python -m pytest -v -k 'not slow'`

Expected: all tests pass (default exclusion of `ai_pipeline` per repo convention).

- [ ] **Step 3: Confirm no `engine_py_legacy` imports were introduced**

Run: `python -m grep -rn 'from engine_py_legacy\\|import engine_py_legacy' code/digimon_gym/ code/digimon-engine-py/`

Expected: no matches. Production code never imports the legacy engine (working rule #22).

- [ ] **Step 4: Confirm shaping is OFF by default**

Run: `python -c "from digimon_gym.agents.training_config import TrainingConfig; c = TrainingConfig(); print(c.digivolve_shaping, c.digivolve_reward, c.dna_digivolve_bonus)"`

Expected: `False 0.1 0.3`. Confirms unset users see no behavioral change.

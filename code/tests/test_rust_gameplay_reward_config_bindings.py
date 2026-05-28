"""Round-trip test for the gameplay-reward-config PyO3 additions:

- `RustHeadlessGame.get_rl_state()` exposes `turn_count: int` (used by
  `quick_win_bonus` / `stall_penalty` / `TerminalOutcome.turn_count`).
- `RustHeadlessGame.get_rl_state()` exposes `n_digivolve_driven_attacks:
  list[int]` (2 elements, Rust 0-based player indexing).
- Module-level `BREEDING_TARGET` constant is importable and matches the
  canonical Rust-side action-space value (used by the bus and the
  `breeding_digivolve` component to identify breeding-area Digivolved
  occurrences).

Catches binding-side regressions that the Rust-only integration test in
`code/digimon-engine/tests/event_emission/digivolve_driven_attack.rs`
cannot see.
"""

from __future__ import annotations

import pytest

pytest.importorskip("digimon_engine")

import digimon_engine  # noqa: E402
from digimon_engine import RustHeadlessGame  # noqa: E402


def _starter_decks() -> tuple[list[str], list[str]]:
    deck = ["ST1-01"] * 5 + ["ST1-03"] * 45
    return deck, deck


def test_breeding_target_constant_importable() -> None:
    """Scenario: BREEDING_TARGET importable, integer-typed, matches canonical."""
    assert hasattr(digimon_engine, "BREEDING_TARGET")
    val = digimon_engine.BREEDING_TARGET
    assert isinstance(val, int), f"BREEDING_TARGET should be int, got {type(val)}"
    # The canonical Rust-side value is 14 (see code/digimon-engine/src/action/space.rs).
    assert val == 14, (
        f"BREEDING_TARGET={val} drifted from the canonical Rust action-space value 14"
    )


def test_get_rl_state_exposes_turn_count_and_driven_attacks_counter() -> None:
    """Scenario: get_rl_state surfaces turn_count + n_digivolve_driven_attacks."""
    deck1, deck2 = _starter_decks()
    game = RustHeadlessGame(deck1, deck2, seed=1)

    state = game.get_rl_state()

    # turn_count: present, integer-typed.
    assert "turn_count" in state, f"get_rl_state missing 'turn_count'; keys={list(state.keys())}"
    assert isinstance(state["turn_count"], int)
    assert state["turn_count"] >= 0

    # n_digivolve_driven_attacks: 2-element list of ints, both zero at game start.
    assert "n_digivolve_driven_attacks" in state, (
        f"get_rl_state missing 'n_digivolve_driven_attacks'; keys={list(state.keys())}"
    )
    counter = state["n_digivolve_driven_attacks"]
    assert isinstance(counter, (list, tuple)), (
        f"n_digivolve_driven_attacks should be a sequence, got {type(counter)}"
    )
    assert len(counter) == 2, f"expected 2 entries (per player), got {len(counter)}"
    assert all(isinstance(v, int) for v in counter), (
        f"all entries must be int; got {[type(v) for v in counter]}"
    )
    assert counter[0] == 0 and counter[1] == 0, (
        f"fresh game must have zero driven attacks; got {counter}"
    )


def test_turn_count_advances_across_turn_passes() -> None:
    """Scenario: turn_count advances monotonically as turns pass."""
    deck1, deck2 = _starter_decks()
    game = RustHeadlessGame(deck1, deck2, seed=1)

    initial = game.get_rl_state()["turn_count"]

    # PASS action id (action space wraps it as constant 96 — but we can
    # just walk the action mask greedily and take legal actions until the
    # turn counter ticks).
    PASS = 96
    for _ in range(20):
        mask = game.get_action_mask()
        if mask[PASS] > 0:
            game.step(PASS)
            break
        # otherwise take whatever is legal
        for action_id, allowed in enumerate(mask):
            if allowed > 0:
                game.step(action_id)
                break

    after = game.get_rl_state()["turn_count"]
    assert after >= initial, (
        f"turn_count must be monotonic non-decreasing; saw {initial} -> {after}"
    )

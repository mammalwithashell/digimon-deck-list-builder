"""Pure-Python tests for the `stall_penalty` component.

Maps to scenarios in
`openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
`stall_penalty component`.

Synthetic: constructs `TerminalOutcome` occurrences by hand.
"""

from __future__ import annotations

import pytest

from digimon_gym.agents.reward.components.stall import StallPenaltyComponent
from digimon_gym.agents.reward.occurrences import TerminalOutcome


def _comp(**kw) -> StallPenaltyComponent:
    return StallPenaltyComponent(name="stall_penalty", **kw)


def _terminal(*, winner_id, turn_count: int = 0) -> TerminalOutcome:
    return TerminalOutcome(
        winner_id=winner_id,
        step_count=100,
        reason=None,
        turn_count=turn_count,
    )


# ── Threshold & curve ─────────────────────────────────────────────


def test_no_penalty_at_or_before_threshold():
    """Scenario: No penalty at or before threshold."""
    comp = _comp()  # threshold=7, scale=0.1
    for turn in (1, 5, 7):
        for winner in (1, 2, None):
            got = comp.compute([_terminal(winner_id=winner, turn_count=turn)], {})
            assert got == 0.0, f"turn={turn} winner={winner} expected 0.0, got {got}"


def test_quadratic_growth_after_threshold():
    """Scenario: Quadratic growth for turns 10/15/20/30."""
    comp = _comp()
    # penalty = -0.1 × (turn - 7)²
    expected = {
        10: -0.1 * 9,    # -0.9
        15: -0.1 * 64,   # -6.4
        20: -0.1 * 169,  # -16.9
        30: -0.1 * 529,  # -52.9
    }
    for turn, want in expected.items():
        got = comp.compute([_terminal(winner_id=2, turn_count=turn)], {})
        assert got == pytest.approx(want), f"turn={turn}: want {want}, got {got}"


# ── Symmetric application ─────────────────────────────────────────


def test_applies_to_all_outcomes_by_default():
    """Scenario: Applies to all outcomes by default."""
    comp = _comp()
    for winner in (1, 2, None):
        got = comp.compute([_terminal(winner_id=winner, turn_count=15)], {})
        assert got == pytest.approx(-6.4), f"winner={winner} should still penalize: got {got}"


def test_apply_to_winner_false_zeros_win_only():
    """Scenario: apply_to_winner=false disables on agent win only."""
    comp = _comp(apply_to_winner=False, apply_to_loser=True)
    assert comp.compute([_terminal(winner_id=1, turn_count=15)], {}) == 0.0
    assert comp.compute([_terminal(winner_id=2, turn_count=15)], {}) == pytest.approx(-6.4)
    assert comp.compute([_terminal(winner_id=None, turn_count=15)], {}) == pytest.approx(-6.4)


def test_apply_to_loser_false_zeros_loss_only():
    """Symmetric: apply_to_loser=false disables on agent loss only."""
    comp = _comp(apply_to_winner=True, apply_to_loser=False)
    assert comp.compute([_terminal(winner_id=1, turn_count=15)], {}) == pytest.approx(-6.4)
    assert comp.compute([_terminal(winner_id=2, turn_count=15)], {}) == 0.0
    assert comp.compute([_terminal(winner_id=None, turn_count=15)], {}) == pytest.approx(-6.4)


def test_draws_always_penalized_regardless_of_apply_flags():
    """Scenario: Draws always penalized regardless of apply flags."""
    comp = _comp(apply_to_winner=False, apply_to_loser=False)
    assert comp.compute([_terminal(winner_id=None, turn_count=15)], {}) == pytest.approx(-6.4)
    # Wins + losses are gated off.
    assert comp.compute([_terminal(winner_id=1, turn_count=15)], {}) == 0.0
    assert comp.compute([_terminal(winner_id=2, turn_count=15)], {}) == 0.0


def test_no_terminal_in_stream_returns_zero():
    comp = _comp()
    assert comp.compute([], {}) == 0.0

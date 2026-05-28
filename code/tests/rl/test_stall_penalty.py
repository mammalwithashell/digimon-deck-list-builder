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


# ── Winner-side cap (winner_max_penalty) ──────────────────────────


def test_winner_max_penalty_caps_winner_only():
    """Default winner_max_penalty=19.0 caps winner penalty at -19.0.
    Loser side stays unbounded — this is the invariant that keeps
    'win never worse than loss' true."""
    comp = _comp()  # default winner_max_penalty=19.0
    # Turn 30: natural penalty = -52.9. Winner clamped to -19.0; loser unchanged.
    assert comp.compute([_terminal(winner_id=1, turn_count=30)], {}) == pytest.approx(-19.0)
    assert comp.compute([_terminal(winner_id=2, turn_count=30)], {}) == pytest.approx(-52.9)
    # Draw also unbounded.
    assert comp.compute([_terminal(winner_id=None, turn_count=30)], {}) == pytest.approx(-52.9)


def test_winner_cap_does_not_apply_below_threshold_magnitude():
    """At moderate turns where the natural penalty is BELOW the cap
    magnitude, the cap doesn't activate (no clamping)."""
    comp = _comp()  # cap=19
    # Turn 15: natural = -6.4 (magnitude well below 19) → no clamp.
    assert comp.compute([_terminal(winner_id=1, turn_count=15)], {}) == pytest.approx(-6.4)
    # Turn 20: natural = -16.9 (still below 19) → no clamp.
    assert comp.compute([_terminal(winner_id=1, turn_count=20)], {}) == pytest.approx(-16.9)
    # Turn 22: natural = -22.5 (over 19) → clamped to -19.0.
    assert comp.compute([_terminal(winner_id=1, turn_count=22)], {}) == pytest.approx(-19.0)


def test_win_always_beats_loss_invariant():
    """Cross-turn invariant: WORST possible win + terminal_outcome.win_base
    must be strictly greater than BEST possible loss + terminal_outcome.loss.
    With the default cap (19.0) + default terminal_outcome (win=10, loss=-10),
    worst-win = 10 + 0 (decayed quick_win) − 19 = -9.0, which is strictly
    greater than worst-loss = -10 + 0 (no stall) = -10.0.
    """
    comp = _comp()
    WIN_BASE = 10.0
    LOSS = -10.0
    # Sweep turns 1..50 and verify (win + win_base) > (loss + LOSS) for ALL
    # turn pairs.
    win_values = []
    loss_values = []
    for turn in range(1, 51):
        win_values.append(WIN_BASE + comp.compute([_terminal(winner_id=1, turn_count=turn)], {}))
        loss_values.append(LOSS + comp.compute([_terminal(winner_id=2, turn_count=turn)], {}))
    min_win = min(win_values)
    max_loss = max(loss_values)
    assert min_win > max_loss, (
        f"INVARIANT VIOLATED: worst win ({min_win:+.2f}) <= best loss ({max_loss:+.2f}). "
        "Increase winner_max_penalty (cap) or adjust terminal_outcome win_base/loss."
    )


def test_winner_max_penalty_can_be_disabled_with_large_value():
    """Operators can effectively disable the cap by setting a very large
    winner_max_penalty (e.g., 1e9). The penalty then matches the
    unbounded loser-side formula."""
    comp = _comp(winner_max_penalty=1e9)
    # Turn 30 winner: natural -52.9, no clamp at 1e9.
    assert comp.compute([_terminal(winner_id=1, turn_count=30)], {}) == pytest.approx(-52.9)


def test_winner_max_penalty_can_be_tightened():
    """Tighter cap (e.g., 5.0) cuts off winner penalty earlier — useful
    if operator wants near-zero win-side stall pressure."""
    comp = _comp(winner_max_penalty=5.0)
    # Turn 15 winner: natural -6.4 → clamped to -5.0.
    assert comp.compute([_terminal(winner_id=1, turn_count=15)], {}) == pytest.approx(-5.0)
    # Loser unaffected.
    assert comp.compute([_terminal(winner_id=2, turn_count=15)], {}) == pytest.approx(-6.4)

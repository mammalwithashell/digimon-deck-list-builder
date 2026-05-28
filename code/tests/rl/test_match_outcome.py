"""Pure-Python tests for the `match_outcome` component (Path C).

Spec: `openspec/specs/gameplay-reward-config/spec.md` `match_outcome component`.

The component fires on `MatchOutcome` occurrences (one per BO3 match end)
and is the profile-side hook by which `MatchEnv` defers its hardcoded
MATCH_TERMINAL_* magnitudes when the active profile includes it.

These tests are fully synthetic — they construct `MatchOutcome` instances
by hand and call `compute(...)` directly. No engine, no PyO3, no env.
The wrapper / MatchEnv integration is covered by
`test_match_env_pure_outcome_profile.py`.
"""

from __future__ import annotations

import pytest

from digimon_gym.agents.reward.components.match_outcome import (
    MatchOutcomeComponent,
)
from digimon_gym.agents.reward.occurrences import MatchOutcome


def _outcome(**overrides):
    """Build a `MatchOutcome` with sensible defaults for the field a test
    isn't varying. Real `MatchEnv` always fills all fields.
    """
    base = dict(
        winner_id=1,
        agent_game_wins=2,
        opp_game_wins=0,
        was_sweep=True,
        was_swept=False,
        agent_conceded_any=False,
        match_step_count=200,
        total_games=2,
    )
    base.update(overrides)
    return MatchOutcome(**base)


# ── Win / loss / draw base ────────────────────────────────────────────


def test_pure_outcome_win_returns_win_value():
    c = MatchOutcomeComponent(name="match_outcome", win=1.0, loss=-1.0, draw=0.0)
    assert c.compute([_outcome(winner_id=1)], {}) == pytest.approx(1.0)


def test_pure_outcome_loss_returns_loss_value():
    c = MatchOutcomeComponent(name="match_outcome", win=1.0, loss=-1.0, draw=0.0)
    out = c.compute(
        [_outcome(winner_id=2, agent_game_wins=0, opp_game_wins=2, was_sweep=False, was_swept=True)],
        {},
    )
    assert out == pytest.approx(-1.0)


def test_draw_returns_draw_value():
    c = MatchOutcomeComponent(name="match_outcome", win=1.0, loss=-1.0, draw=-0.5)
    out = c.compute(
        [_outcome(winner_id=None, agent_game_wins=1, opp_game_wins=1, was_sweep=False, total_games=3)],
        {},
    )
    assert out == pytest.approx(-0.5)


def test_no_match_outcome_returns_zero():
    """Empty occurrence list — the component is a no-op on the (frequent)
    non-terminal calls where MatchEnv passes through dense steps."""
    c = MatchOutcomeComponent(name="match_outcome", win=10.0, loss=-10.0)
    assert c.compute([], {}) == 0.0


def test_unrelated_occurrence_returns_zero():
    """Defensive: if some other occurrence is in the list, the component
    must ignore it (mirrors the other components' guard pattern)."""
    from digimon_gym.agents.reward.occurrences import StepElapsed

    c = MatchOutcomeComponent(name="match_outcome", win=10.0)
    assert c.compute([StepElapsed()], {}) == 0.0


# ── Sweep + swept ──────────────────────────────────────────────────────


def test_sweep_adds_sweep_bonus_to_win():
    c = MatchOutcomeComponent(
        name="match_outcome", win=10.0, loss=-10.0, sweep_bonus=4.0
    )
    out = c.compute([_outcome(was_sweep=True, opp_game_wins=0)], {})
    assert out == pytest.approx(14.0)


def test_non_sweep_win_omits_sweep_bonus():
    c = MatchOutcomeComponent(
        name="match_outcome", win=10.0, loss=-10.0, sweep_bonus=4.0
    )
    out = c.compute(
        [_outcome(was_sweep=False, agent_game_wins=2, opp_game_wins=1, total_games=3)],
        {},
    )
    assert out == pytest.approx(10.0)


def test_swept_loss_adds_swept_penalty():
    c = MatchOutcomeComponent(
        name="match_outcome", win=10.0, loss=-10.0, swept_penalty=-5.0
    )
    out = c.compute(
        [_outcome(
            winner_id=2, agent_game_wins=0, opp_game_wins=2,
            was_sweep=False, was_swept=True,
        )],
        {},
    )
    assert out == pytest.approx(-15.0)


def test_close_loss_omits_swept_penalty():
    c = MatchOutcomeComponent(
        name="match_outcome", win=10.0, loss=-10.0, swept_penalty=-5.0
    )
    out = c.compute(
        [_outcome(
            winner_id=2, agent_game_wins=1, opp_game_wins=2,
            was_sweep=False, was_swept=False, total_games=3,
        )],
        {},
    )
    assert out == pytest.approx(-10.0)


# ── Smart / scared concede ────────────────────────────────────────────


def test_smart_concede_bonus_on_win_with_any_concede():
    """Comeback after a smart concede earns a bonus on top of the win."""
    c = MatchOutcomeComponent(
        name="match_outcome", win=10.0, loss=-10.0,
        smart_concede_bonus=2.0,
    )
    out = c.compute(
        [_outcome(
            winner_id=1, agent_game_wins=2, opp_game_wins=1,
            was_sweep=False, agent_conceded_any=True, total_games=3,
        )],
        {},
    )
    assert out == pytest.approx(12.0)


def test_scared_concede_penalty_only_on_0_2_loss():
    """Scared-concede penalty is asymmetric: only fires on a 0-2 loss WITH
    any agent concede. A 1-2 loss with a concede is NOT extra-penalized
    (the smart-concede gamble that didn't pan out is its own punishment).
    """
    c = MatchOutcomeComponent(
        name="match_outcome", win=10.0, loss=-10.0,
        scared_concede_penalty=-3.0,
    )
    # 0-2 loss with concede → penalty fires.
    out_scared = c.compute(
        [_outcome(
            winner_id=2, agent_game_wins=0, opp_game_wins=2,
            was_swept=True, agent_conceded_any=True,
        )],
        {},
    )
    assert out_scared == pytest.approx(-13.0)
    # 1-2 loss with concede → no scared penalty.
    out_close = c.compute(
        [_outcome(
            winner_id=2, agent_game_wins=1, opp_game_wins=2,
            was_swept=False, agent_conceded_any=True, total_games=3,
        )],
        {},
    )
    assert out_close == pytest.approx(-10.0)


def test_smart_concede_bonus_not_applied_to_loss():
    """`smart_concede_bonus` is win-gated only."""
    c = MatchOutcomeComponent(
        name="match_outcome", win=10.0, loss=-10.0,
        smart_concede_bonus=5.0,
    )
    out = c.compute(
        [_outcome(
            winner_id=2, agent_game_wins=1, opp_game_wins=2,
            was_sweep=False, agent_conceded_any=True, total_games=3,
        )],
        {},
    )
    assert out == pytest.approx(-10.0)


# ── Fast-win bonus ─────────────────────────────────────────────────────


def test_fast_bonus_linear_in_remaining_steps():
    """Bonus = (par − match_step_count) / par × max, clamped at >= 0."""
    c = MatchOutcomeComponent(
        name="match_outcome", win=10.0, loss=-10.0,
        fast_bonus_max=4.0, fast_bonus_par_steps=200,
    )
    out = c.compute([_outcome(winner_id=1, match_step_count=150)], {})
    # 10 + (200-150)/200 × 4 = 10 + 1.0 = 11.0
    assert out == pytest.approx(11.0)


def test_fast_bonus_clamps_to_zero_past_par():
    """A slow win past par_steps yields 0 fast bonus, not a negative."""
    c = MatchOutcomeComponent(
        name="match_outcome", win=10.0, loss=-10.0,
        fast_bonus_max=4.0, fast_bonus_par_steps=200,
    )
    out = c.compute([_outcome(winner_id=1, match_step_count=500)], {})
    assert out == pytest.approx(10.0)


def test_fast_bonus_zero_when_disabled():
    """fast_bonus_max=0 fully disables the bonus path regardless of steps."""
    c = MatchOutcomeComponent(
        name="match_outcome", win=10.0, loss=-10.0, fast_bonus_max=0.0,
    )
    out = c.compute([_outcome(winner_id=1, match_step_count=1)], {})
    assert out == pytest.approx(10.0)


def test_fast_bonus_not_applied_to_loss():
    """Fast bonus is win-gated only."""
    c = MatchOutcomeComponent(
        name="match_outcome", win=10.0, loss=-10.0,
        fast_bonus_max=4.0, fast_bonus_par_steps=200,
    )
    out = c.compute(
        [_outcome(
            winner_id=2, agent_game_wins=0, opp_game_wins=2,
            was_swept=True, match_step_count=50,
        )],
        {},
    )
    assert out == pytest.approx(-10.0)


# ── Legacy-calibration parity ──────────────────────────────────────────


def test_legacy_match_env_calibration_reproduces_match_env_constants():
    """Profile equivalent of MatchEnv's hardcoded MATCH_TERMINAL_* path.
    Win in 2-0 with no concedes, 300 steps:
      base 30 + sweep 10 + fast(450-300)/450 × 15 = 30 + 10 + 5 = 45.
    """
    c = MatchOutcomeComponent(
        name="match_outcome",
        win=30.0, loss=-30.0, draw=-1.0,
        sweep_bonus=10.0, swept_penalty=0.0,
        smart_concede_bonus=5.0, scared_concede_penalty=-10.0,
        fast_bonus_max=15.0, fast_bonus_par_steps=450,
    )
    out = c.compute(
        [_outcome(
            winner_id=1, agent_game_wins=2, opp_game_wins=0,
            was_sweep=True, match_step_count=300,
        )],
        {},
    )
    assert out == pytest.approx(45.0)


def test_pure_outcome_profile_signal_is_symmetric_and_bounded():
    """The shipped _pure_outcome profile spec: ±1 match terminal, no
    bonuses, no penalties. Confirm the contract.
    """
    c = MatchOutcomeComponent(name="match_outcome", win=1.0, loss=-1.0, draw=0.0)
    # Sweep should NOT add anything (no sweep_bonus configured).
    out_sweep = c.compute([_outcome(winner_id=1, was_sweep=True, opp_game_wins=0)], {})
    assert out_sweep == pytest.approx(1.0)
    # Fast win should NOT add anything (no fast_bonus_max configured).
    out_fast = c.compute([_outcome(winner_id=1, match_step_count=10)], {})
    assert out_fast == pytest.approx(1.0)
    # Loss with concede should NOT subtract anything beyond base.
    out_loss = c.compute(
        [_outcome(
            winner_id=2, agent_game_wins=0, opp_game_wins=2,
            was_swept=True, agent_conceded_any=True,
        )],
        {},
    )
    assert out_loss == pytest.approx(-1.0)

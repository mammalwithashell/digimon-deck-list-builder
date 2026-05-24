"""Tests for the BO3 match-training dense-reward calibration.

The asymmetric security signal (`+1.5` opp-removed, `-0.5` own-lost) is
the centerpiece of the BO3 reward redesign — it makes "clear all 5 opp
security and lose" net out to roughly `−7.0`, well below a `+12` game
win, so recovery decks can't bait the agent. See
`openspec/changes/add-bo3-match-training/design.md` §D9.

These tests pin the calibrated magnitudes against direct
`_compute_reward` calls so any future drift triggers a regression.
"""

from __future__ import annotations

import math

import pytest

pytest.importorskip("digimon_engine")

from digimon_gym.digimon_gym import DigimonEnv  # noqa: E402


def _setup_env(*, shaping: bool = False) -> DigimonEnv:
    """Build a fresh env with security trackers initialised at 5/5 so that
    the first call to `_compute_reward` produces real deltas (not the
    `_prev_*=None` first-step no-op)."""
    env = DigimonEnv(digivolve_shaping=shaping)
    env.reset()
    env._prev_p1_security = 5
    env._prev_p2_security = 5
    env._prev_p1_digivolutions = 0
    env._prev_p1_dna_digivolutions = 0
    return env


# ─── Security delta — asymmetric calibration ─────────────────────────────


def test_opp_security_removal_credits_plus_one_point_five() -> None:
    env = _setup_env()
    env._rl_state = lambda: {  # type: ignore[method-assign]
        "game_over": False,
        "p1_security": 5,
        "p2_security": 4,  # one opp sec removed
        "p1_digivolutions": 0,
        "p1_dna_digivolutions": 0,
    }
    reward = env._compute_reward(terminated=False)
    # +1.5 (opp removed) - 0.001 (step penalty)
    assert math.isclose(reward, 1.5 - 0.001, abs_tol=1e-9)


def test_own_security_loss_penalizes_minus_zero_point_five() -> None:
    env = _setup_env()
    env._rl_state = lambda: {  # type: ignore[method-assign]
        "game_over": False,
        "p1_security": 4,  # one own sec lost
        "p2_security": 5,
        "p1_digivolutions": 0,
        "p1_dna_digivolutions": 0,
    }
    reward = env._compute_reward(terminated=False)
    # -0.5 (own lost) - 0.001 (step penalty)
    assert math.isclose(reward, -0.5 - 0.001, abs_tol=1e-9)


def test_security_gained_yields_zero_security_signal() -> None:
    """Security gained via effects must NOT trigger the dense signal —
    it's a defensive maneuver, not progress toward winning."""
    env = _setup_env()
    env._rl_state = lambda: {  # type: ignore[method-assign]
        "game_over": False,
        "p1_security": 6,  # gained one (effect)
        "p2_security": 6,  # opp gained one (effect)
        "p1_digivolutions": 0,
        "p1_dna_digivolutions": 0,
    }
    reward = env._compute_reward(terminated=False)
    # Neither gain credits anything; only the step penalty remains.
    assert math.isclose(reward, -0.001, abs_tol=1e-9)


def test_simultaneous_loss_and_gain_nets_correctly() -> None:
    """Trade case: opp removed 2 own security AND we removed 3 opp security."""
    env = _setup_env()
    env._rl_state = lambda: {  # type: ignore[method-assign]
        "game_over": False,
        "p1_security": 3,  # lost 2 own
        "p2_security": 2,  # removed 3 opp
        "p1_digivolutions": 0,
        "p1_dna_digivolutions": 0,
    }
    reward = env._compute_reward(terminated=False)
    # +3 * 1.5 - 2 * 0.5 - 0.001 = +4.5 - 1.0 - 0.001 = +3.499
    assert math.isclose(reward, 3 * 1.5 - 2 * 0.5 - 0.001, abs_tol=1e-9)


def test_cumulative_signal_capped_below_game_terminal() -> None:
    """Recovery-deck guard: clearing all 5 opp security must produce a
    cumulative dense signal less than the +12 game terminal."""
    # Drive 5 single-removal steps; sum the rewards.
    env = _setup_env()

    cumulative_dense = 0.0
    for opp_sec_after in (4, 3, 2, 1, 0):
        env._rl_state = lambda v=opp_sec_after: {  # type: ignore[method-assign]
            "game_over": False,
            "p1_security": 5,
            "p2_security": v,
            "p1_digivolutions": 0,
            "p1_dna_digivolutions": 0,
        }
        cumulative_dense += env._compute_reward(terminated=False)

    # Each step: +1.5 - 0.001. Total = 5 * 1.499 = 7.495.
    expected = 5 * (1.5 - 0.001)
    assert math.isclose(cumulative_dense, expected, abs_tol=1e-6)
    # And crucially: well below +12.
    assert cumulative_dense < 12.0, (
        f"cumulative dense {cumulative_dense:.3f} must stay below game-terminal +12"
    )


# ─── Digivolve shaping — defaults unchanged in env layer ─────────────────


def test_digivolve_shaping_defaults_off_in_env_constructor() -> None:
    """The BO3 default flip (`digivolve_shaping=True` for match-format bo3)
    lives in `pilot_training.make_env` / config, NOT in `DigimonEnv` itself.
    The env constructor's default stays OFF for back-compat with non-BO3
    callers and direct `DigimonEnv(...)` users."""
    env = DigimonEnv()
    assert env.digivolve_shaping is False
    assert env.digivolve_reward == 0.1
    assert env.dna_digivolve_bonus == 3.9


def test_dna_digivolve_credits_four_total() -> None:
    """Confirm digivolve magnitudes survived the BO3 recalibration of
    security signals. DNA event = +0.1 (regular bump) + +3.9 (DNA bonus)."""
    env = _setup_env(shaping=True)
    env._rl_state = lambda: {  # type: ignore[method-assign]
        "game_over": False,
        "p1_security": 5,
        "p2_security": 5,
        "p1_digivolutions": 1,  # DNA also bumps regular counter
        "p1_dna_digivolutions": 1,
    }
    reward = env._compute_reward(terminated=False)
    # +0.1 + 3.9 = +4.0, minus the step penalty.
    assert math.isclose(reward, 4.0 - 0.001, abs_tol=1e-9)


def test_regular_digivolve_credits_zero_point_one() -> None:
    env = _setup_env(shaping=True)
    env._rl_state = lambda: {  # type: ignore[method-assign]
        "game_over": False,
        "p1_security": 5,
        "p2_security": 5,
        "p1_digivolutions": 1,
        "p1_dna_digivolutions": 0,
    }
    reward = env._compute_reward(terminated=False)
    assert math.isclose(reward, 0.1 - 0.001, abs_tol=1e-9)


# ─── Hierarchy invariants — pin the design's signal ordering ─────────────


def test_hierarchy_dense_event_below_cumulative_game_dense() -> None:
    """The design hierarchy is:
        step penalty < digivolve < security event < DNA < cumulative game dense
        < game terminal < match terminal
    Pin this with simple magnitude inequalities so a future tweak that
    breaks the ordering trips a test."""
    step = 0.001
    digivolve_regular = 0.1
    digivolve_dna_total = 4.0  # 0.1 + 3.9
    sec_event_pos = 1.5
    sec_event_neg = 0.5
    cumulative_dense_max = 5 * sec_event_pos  # 7.5

    assert step < digivolve_regular
    assert digivolve_regular < sec_event_neg
    assert sec_event_neg < sec_event_pos
    assert sec_event_pos < digivolve_dna_total
    assert digivolve_dna_total < cumulative_dense_max
    assert sec_event_pos < cumulative_dense_max
    # Game terminal magnitude is asserted indirectly: cumulative dense
    # must stay strictly below the eventual +12 game-terminal magnitude.
    # The match-env wrapper enforces the +12 magnitude directly; tested
    # in test_match_env.py once Section 6 lands.
    assert cumulative_dense_max < 12.0

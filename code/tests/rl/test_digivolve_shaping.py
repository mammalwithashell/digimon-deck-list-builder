"""Tests for digivolve reward shaping in DigimonEnv.

This file covers the constructor surface (Task 7), the _compute_reward
math (Task 8), and the byte-identical-default regression (Task 9). See
docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
"""

from __future__ import annotations

import math

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

    env.reset()
    assert env._prev_p1_digivolutions is None
    assert env._prev_p1_dna_digivolutions is None


def test_constructor_defaults_are_off() -> None:
    env = DigimonEnv()
    assert env.digivolve_shaping is False
    assert env.digivolve_reward == 0.1
    assert env.dna_digivolve_bonus == 0.3


# ─── _compute_reward math (Task 8) ───────────────────────────────────────


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
    env._prev_p1_digivolutions = None
    env._prev_p1_dna_digivolutions = None
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
    # Only the step penalty (-0.001), no shaping credit, no security delta.
    assert math.isclose(reward, -0.001, abs_tol=1e-9)


def test_regular_digivolve_credits_digivolve_reward() -> None:
    env = _make_shaped_env()
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
        "p1_dna_digivolutions": 0,
    }
    env._rl_state = lambda: state  # type: ignore[method-assign]
    reward = env._compute_reward(terminated=False)
    assert math.isclose(reward, 0.1 - 0.001, abs_tol=1e-9)


def test_dna_digivolve_credits_full_dna_band() -> None:
    env = _make_shaped_env()
    env.reset()
    env._prev_p1_digivolutions = 0
    env._prev_p1_dna_digivolutions = 0
    env._prev_p1_security = 5
    env._prev_p2_security = 5

    # DNA stacks on regular: both counters jump by 1.
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
    assert math.isclose(reward, -0.001, abs_tol=1e-9)

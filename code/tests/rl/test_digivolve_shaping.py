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

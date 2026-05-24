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

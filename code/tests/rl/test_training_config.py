"""TrainingConfig YAML load, CLI-style overrides, and validation."""

from __future__ import annotations

from pathlib import Path

import pytest

from digimon_gym.agents.training_config import TrainingConfig


REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_CFG = REPO_ROOT / "configs" / "training" / "default.yaml"


def test_loads_default():
    cfg = TrainingConfig.from_yaml(DEFAULT_CFG)
    assert cfg.algorithm == "mlp"
    assert cfg.seed == 0
    assert cfg.timesteps > 0
    assert cfg.n_envs == 1


def test_cli_override_merges_over_yaml():
    cfg = TrainingConfig.from_yaml(
        DEFAULT_CFG,
        overrides={"seed": 42, "timesteps": 1000},
    )
    assert cfg.seed == 42
    assert cfg.timesteps == 1000
    assert cfg.algorithm == "mlp"


def test_invalid_algorithm_rejected():
    with pytest.raises(ValueError, match="algorithm"):
        TrainingConfig.from_yaml(DEFAULT_CFG, overrides={"algorithm": "transformer"})


def test_negative_timesteps_rejected():
    with pytest.raises(ValueError):
        TrainingConfig.from_yaml(DEFAULT_CFG, overrides={"timesteps": -1})

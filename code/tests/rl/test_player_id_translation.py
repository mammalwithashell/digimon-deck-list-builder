"""Verify PyO3 player-id translation at the Python/Rust boundary."""

from __future__ import annotations

import numpy as np
import pytest

pytest.importorskip("digimon_engine")

import digimon_gym.digimon_gym as gym_mod  # noqa: E402


@pytest.fixture(autouse=True)
def _rust_backend(monkeypatch):
    # `DIGIMON_BACKEND` is read at `_make_runner` call time (env.reset/step),
    # so setting it via monkeypatch is sufficient — no `importlib.reload` is
    # needed, and the module-top reload that used to live here poisoned
    # already-imported `DigimonEnv` symbols in sibling test modules.
    monkeypatch.setenv("DIGIMON_BACKEND", "rust")


def test_player1_and_player2_observations_differ_after_asymmetric_play():
    env = gym_mod.DigimonEnv(
        deck1=["ST1-01"] * 5 + ["ST1-03"] * 45,
        deck2=["ST1-01"] * 5 + ["ST1-03"] * 45,
    )
    env.reset(seed=99)
    for _ in range(3):
        valid = np.where(env.action_mask() > 0)[0]
        assert len(valid) > 0
        env.step(int(valid[0]))

    obs_p1 = env.runner.get_board_tensor(1)
    obs_p2 = env.runner.get_board_tensor(2)

    assert obs_p1.shape == obs_p2.shape
    assert not np.array_equal(obs_p1, obs_p2)


def test_invalid_player_id_rejected():
    env = gym_mod.DigimonEnv()
    env.reset(seed=1)
    with pytest.raises((ValueError, OverflowError, RuntimeError)):
        env.runner.get_board_tensor(0)
    with pytest.raises((ValueError, OverflowError, RuntimeError)):
        env.runner.get_board_tensor(3)

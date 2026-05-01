"""Rust-backend adapter behavior for DigimonEnv."""

from __future__ import annotations

import importlib
import os

import numpy as np
import pytest

pytest.importorskip("digimon_engine")


DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45


def _rust_env():
    os.environ["DIGIMON_BACKEND"] = "rust"
    import digimon_gym.digimon_gym as gym_mod

    importlib.reload(gym_mod)
    return gym_mod.DigimonEnv(deck1=DECK, deck2=DECK)


def test_rust_env_reports_current_player_without_legacy_game_object():
    env = _rust_env()
    _obs, _info = env.reset(seed=7)

    assert env.game is None
    assert env.current_player_id in (1, 2)
    assert env.is_game_over is False
    assert env.winner_id is None


def test_rust_env_step_computes_reward_without_runner_game_attribute():
    env = _rust_env()
    _obs, info = env.reset(seed=7)
    valid = np.where(info["action_mask"] > 0)[0]

    obs, reward, terminated, truncated, next_info = env.step(int(valid[0]))

    assert obs.shape == env.observation_space.shape
    assert isinstance(reward, float)
    assert isinstance(terminated, bool)
    assert isinstance(truncated, bool)
    assert next_info["action_mask"].shape == (env.action_space.n,)


def test_rust_env_greedy_policy_uses_rust_policy_surface():
    env = _rust_env()
    _obs, _info = env.reset(seed=7)

    import digimon_gym.digimon_gym as gym_mod

    action = gym_mod.greedy_policy(env)
    assert env.action_mask()[action] > 0

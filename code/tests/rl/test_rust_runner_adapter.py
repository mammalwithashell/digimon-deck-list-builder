"""Rust-backend adapter behavior for DigimonEnv."""

from __future__ import annotations

import numpy as np
import pytest

pytest.importorskip("digimon_engine")


DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45


def _rust_env(monkeypatch: pytest.MonkeyPatch, **kwargs):
    monkeypatch.setenv("DIGIMON_BACKEND", "rust")
    import digimon_gym.digimon_gym as gym_mod

    return gym_mod.DigimonEnv(deck1=DECK, deck2=DECK, **kwargs)


def test_rust_env_reports_current_player_without_legacy_game_object(monkeypatch):
    env = _rust_env(monkeypatch)
    _obs, _info = env.reset(seed=7)

    assert env.game is None
    assert env.current_player_id in (1, 2)
    assert env.is_game_over is False
    assert env.winner_id is None


def test_digimon_env_uses_requested_standard_lite_v2_profile(monkeypatch):
    env = _rust_env(monkeypatch, tensor_profile="standard_lite_v2")
    obs, info = env.reset(seed=7)

    assert env.tensor_profile == "standard_lite_v2"
    # Task S1.4: PERM_MAX_SOURCES 11 -> 12; tensor_size 8320 -> 8410.
    assert env.observation_space.shape == (8410,)
    assert obs.shape == (8410,)
    assert info["tensor_profile"] == "standard_lite_v2"
    assert info["tensor_feature_schema_version"] == "standard_lite_v2.2"
    assert info["tensor_layout_hash"].startswith("sha256:")


def test_digimon_env_uses_requested_standard_lite_deck_v2_profile(monkeypatch):
    env = _rust_env(monkeypatch, tensor_profile="standard_lite_deck_v2")
    obs, info = env.reset(seed=7)

    assert env.tensor_profile == "standard_lite_deck_v2"
    assert env.observation_space.shape == (8850,)
    assert obs.shape == (8850,)
    assert info["tensor_profile"] == "standard_lite_deck_v2"
    assert info["tensor_feature_schema_version"] == "standard_lite_deck_v2.1"
    assert info["tensor_layout_hash"].startswith("sha256:")
    assert info["action_mask"].shape == (env.action_space.n,)


def test_legacy_env_rejects_standard_lite_v2_profile(monkeypatch):
    monkeypatch.setenv("DIGIMON_BACKEND", "py")
    import digimon_gym.digimon_gym as gym_mod

    env = gym_mod.DigimonEnv(
        deck1=DECK,
        deck2=DECK,
        tensor_profile="standard_lite_v2",
    )

    with pytest.raises(
        RuntimeError,
        match="tensor_profile='standard_lite_v2' requires DIGIMON_BACKEND=rust",
    ):
        env.reset(seed=7)


def test_rust_env_ansi_render_uses_backend_neutral_state(monkeypatch):
    env = _rust_env(monkeypatch, render_mode="ansi")
    env.reset(seed=7)

    rendered = env.render()

    assert isinstance(rendered, str)
    assert "Turn" in rendered
    assert "Phase" in rendered
    assert "P1" in rendered
    assert "P2" in rendered


def test_rust_env_step_computes_reward_without_runner_game_attribute(monkeypatch):
    env = _rust_env(monkeypatch)
    _obs, info = env.reset(seed=7)
    valid = np.where(info["action_mask"] > 0)[0]

    obs, reward, terminated, truncated, next_info = env.step(int(valid[0]))

    assert obs.shape == env.observation_space.shape
    assert isinstance(reward, float)
    assert isinstance(terminated, bool)
    assert isinstance(truncated, bool)
    assert next_info["action_mask"].shape == (env.action_space.n,)


def test_rust_env_truncation_forces_winner(monkeypatch):
    env = _rust_env(monkeypatch, max_turns=0)
    _obs, info = env.reset(seed=7)
    valid = np.where(info["action_mask"] > 0)[0]

    _obs, _reward, terminated, truncated, _info = env.step(int(valid[0]))

    assert terminated is True
    assert truncated is True
    assert env.winner_id in (1, 2)
    outcome = env.terminal_outcome()
    assert outcome["result"] == "win"
    assert outcome["reason"] == "step_limit"


def test_rust_env_greedy_policy_uses_rust_policy_surface(monkeypatch):
    env = _rust_env(monkeypatch)
    _obs, _info = env.reset(seed=7)

    import digimon_gym.digimon_gym as gym_mod

    action = gym_mod.greedy_policy(env)
    assert env.action_mask()[action] > 0


def test_opponent_wrapper_reset_and_step_work_with_rust_runner(monkeypatch):
    from digimon_gym.agents.pilot_training import OpponentWrapper
    import digimon_gym.digimon_gym as gym_mod

    env = _rust_env(monkeypatch)
    wrapped = OpponentWrapper(env, opponent_fn=gym_mod.greedy_policy)
    obs, info = wrapped.reset(seed=11)

    assert env.current_player_id == 1 or env.is_game_over

    if env.is_game_over:
        return

    valid = np.where(info["action_mask"] > 0)[0]
    obs, reward, terminated, truncated, info = wrapped.step(int(valid[0]))

    assert obs.shape == env.observation_space.shape
    assert isinstance(reward, float)
    assert isinstance(terminated, bool)
    assert isinstance(truncated, bool)
    assert info["action_mask"].shape == (env.action_space.n,)
    assert env.current_player_id == 1 or terminated or truncated

"""Cross-engine parity tests for observations and action masks."""

from __future__ import annotations

import importlib
import os

import numpy as np
import pytest

pytest.importorskip("digimon_engine")


DECK_A = ["ST1-01"] * 5 + ["ST1-03"] * 45
DECK_B = ["ST1-01"] * 5 + ["ST1-03"] * 45
SEED = 12345


def _make_env(backend: str):
    os.environ["DIGIMON_BACKEND"] = backend
    import digimon_gym.digimon_gym as gym_mod

    importlib.reload(gym_mod)
    return gym_mod.DigimonEnv(deck1=DECK_A, deck2=DECK_B)


def _replay_until_fork(env, max_steps: int = 50) -> list[np.ndarray]:
    obs, info = env.reset(seed=SEED)
    history = [obs.copy()]
    for _ in range(max_steps):
        mask = info["action_mask"]
        valid = np.where(mask > 0)[0]
        if len(valid) == 0:
            break
        action = int(valid[0])
        obs, _reward, terminated, truncated, info = env.step(action)
        history.append(obs.copy())
        if terminated or truncated:
            break
    return history


def test_initial_observation_parity():
    py_obs, _ = _make_env("py").reset(seed=SEED)
    rs_obs, _ = _make_env("rust").reset(seed=SEED)

    assert py_obs.shape == rs_obs.shape
    np.testing.assert_allclose(
        py_obs,
        rs_obs,
        rtol=0,
        atol=0,
        err_msg="initial observation tensors diverge",
    )


def test_multi_step_observation_parity():
    py_history = _replay_until_fork(_make_env("py"))
    rs_history = _replay_until_fork(_make_env("rust"))

    assert len(py_history) == len(rs_history)
    for step_i, (py_obs, rs_obs) in enumerate(zip(py_history, rs_history)):
        np.testing.assert_allclose(
            py_obs,
            rs_obs,
            rtol=0,
            atol=0,
            err_msg=f"observation diverges at step {step_i}",
        )


def test_initial_action_mask_parity():
    py_env = _make_env("py")
    py_env.reset(seed=SEED)
    py_mask = py_env.action_mask()

    rs_env = _make_env("rust")
    rs_env.reset(seed=SEED)
    rs_mask = rs_env.action_mask()

    assert py_mask.shape == rs_mask.shape
    diff = np.where(py_mask != rs_mask)[0]
    assert len(diff) == 0, (
        f"action mask diverges at indices {diff[:20].tolist()} "
        f"(py={py_mask[diff[:5]].tolist()}, rs={rs_mask[diff[:5]].tolist()})"
    )


def test_multi_step_action_mask_parity():
    py_env = _make_env("py")
    rs_env = _make_env("rust")
    _py_obs, py_info = py_env.reset(seed=SEED)
    _rs_obs, rs_info = rs_env.reset(seed=SEED)

    for step_i in range(50):
        py_mask = py_info["action_mask"]
        rs_mask = rs_info["action_mask"]
        diff = np.where(py_mask != rs_mask)[0]
        assert len(diff) == 0, (
            f"mask diverges at step {step_i}, indices {diff[:20].tolist()}"
        )

        valid = np.where(py_mask > 0)[0]
        if len(valid) == 0:
            break
        action = int(valid[0])
        _py_obs, _py_reward, py_term, py_trunc, py_info = py_env.step(action)
        _rs_obs, _rs_reward, rs_term, rs_trunc, rs_info = rs_env.step(action)
        if py_term or py_trunc or rs_term or rs_trunc:
            break

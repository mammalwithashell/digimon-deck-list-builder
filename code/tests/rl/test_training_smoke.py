"""Pilot-training smoke tests for reproducibility and callbacks."""

from __future__ import annotations

import numpy as np
import pytest
from sb3_contrib import MaskablePPO
from sb3_contrib.common.wrappers import ActionMasker

from digimon_gym.agents.pilot_training import _seed_everything
from digimon_gym.digimon_gym import DigimonEnv


def _mask_fn(env):
    while not hasattr(env, "action_mask") and hasattr(env, "env"):
        env = env.env
    return env.action_mask()


def _train_n_steps(seed: int, n: int = 128) -> int:
    _seed_everything(seed)
    env = ActionMasker(DigimonEnv(), _mask_fn)
    model = MaskablePPO("MlpPolicy", env, n_steps=64, batch_size=32, seed=seed, verbose=0)
    model.learn(total_timesteps=n)
    obs, info = env.reset(seed=seed)
    action, _ = model.predict(obs, action_masks=info["action_mask"], deterministic=True)
    return int(action)


def test_same_seed_reproduces_first_action():
    assert _train_n_steps(seed=7) == _train_n_steps(seed=7)


def test_periodic_checkpointing(tmp_path):
    from digimon_gym.agents.pilot_training import PeriodicCheckpointCallback

    env = ActionMasker(DigimonEnv(), _mask_fn)
    model = MaskablePPO("MlpPolicy", env, n_steps=64, batch_size=32, verbose=0)
    callback = PeriodicCheckpointCallback(
        save_freq=64,
        save_dir=tmp_path,
        run_name="t",
        keep_last=2,
    )
    model.learn(total_timesteps=256, callback=callback)
    files = sorted((tmp_path / "t" / "checkpoints").glob("step_*.zip"))
    assert 1 <= len(files) <= 2


def test_resume_continues_timesteps(tmp_path):
    env = ActionMasker(DigimonEnv(), _mask_fn)
    model = MaskablePPO("MlpPolicy", env, n_steps=64, batch_size=32, verbose=0)
    model.learn(total_timesteps=128)
    ckpt = tmp_path / "ckpt.zip"
    model.save(str(ckpt))
    initial_steps = model.num_timesteps

    env2 = ActionMasker(DigimonEnv(), _mask_fn)
    model2 = MaskablePPO.load(str(ckpt), env=env2)
    model2.learn(total_timesteps=64, reset_num_timesteps=False)
    assert model2.num_timesteps > initial_steps


def test_action_validity_callback_records_rate():
    from digimon_gym.agents.pilot_training import ActionValidityCallback

    callback = ActionValidityCallback()
    env = ActionMasker(DigimonEnv(), _mask_fn)
    model = MaskablePPO("MlpPolicy", env, n_steps=64, batch_size=32, verbose=0)
    model.learn(total_timesteps=128, callback=callback)
    assert callback.total_count > 0
    assert callback.valid_count / callback.total_count > 0.99


def test_dummy_vec_env_runs():
    from digimon_gym.agents.pilot_training import make_vec_env
    from digimon_gym.agents.training_config import TrainingConfig

    cfg = TrainingConfig(n_envs=2, seed=0, opponent="greedy")
    env = make_vec_env(cfg, opponent_fn=lambda _env: 62)
    obs = env.reset()
    assert obs.shape[0] == 2
    env.close()


@pytest.mark.slow
def test_short_run_beats_random():
    from digimon_gym.agents.pilot_training import OpponentWrapper, random_policy

    _seed_everything(42)
    env = ActionMasker(OpponentWrapper(DigimonEnv(), opponent_fn=random_policy), _mask_fn)
    model = MaskablePPO(
        "MlpPolicy",
        env,
        n_steps=512,
        batch_size=64,
        seed=42,
        verbose=0,
    )
    model.learn(total_timesteps=4096)

    wins = 0
    for i in range(30):
        eval_env = ActionMasker(
            OpponentWrapper(DigimonEnv(), opponent_fn=random_policy),
            _mask_fn,
        )
        obs, info = eval_env.reset(seed=1000 + i)
        terminated = truncated = False
        reward = 0.0
        while not (terminated or truncated):
            action, _ = model.predict(
                obs,
                action_masks=info["action_mask"],
                deterministic=True,
            )
            obs, reward, terminated, truncated, info = eval_env.step(int(action))
        if reward > 0:
            wins += 1

    assert wins / 30 > 0.5

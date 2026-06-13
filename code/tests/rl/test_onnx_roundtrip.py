"""ONNX export/load round-trip tests for pilot policies."""

from __future__ import annotations

import subprocess

import numpy as np
import pytest

pytest.importorskip("onnxruntime")
pytest.importorskip("torch")

from sb3_contrib import MaskablePPO  # noqa: E402
from sb3_contrib.common.wrappers import ActionMasker  # noqa: E402

from digimon_gym.digimon_gym import DigimonEnv  # noqa: E402
from digimon_gym.inference.onnx_policy import OnnxPolicy  # noqa: E402


def _action_mask_fn(env):
    return env.action_mask()


@pytest.mark.slow
def test_mlp_onnx_roundtrip(tmp_path):
    env = ActionMasker(DigimonEnv(), _action_mask_fn)
    model = MaskablePPO("MlpPolicy", env, n_steps=64, batch_size=32, verbose=0)
    model.learn(total_timesteps=128)

    sb3_path = tmp_path / "tiny.zip"
    onnx_path = tmp_path / "tiny.onnx"
    model.save(str(sb3_path))

    result = subprocess.run(
        [
            "python",
            "code/tools/export_onnx.py",
            "--type",
            "mlp",
            "--input",
            str(sb3_path),
            "--output",
            str(onnx_path),
            # Pin to the profile the checkpoint trained with (DigimonEnv's
            # default). Without it, export_onnx falls back to the ENGINE
            # default profile (standard_lite_deck_v2, 8850 floats since the
            # 2026-05-25 flip) and the size validation correctly rejects
            # the 8410-float checkpoint.
            "--tensor-profile",
            "standard_lite_v2",
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    assert onnx_path.exists()

    obs, info = env.reset(seed=42)
    onnx_policy = OnnxPolicy(str(onnx_path))

    for _ in range(20):
        mask = info["action_mask"]
        sb3_action, _ = model.predict(obs, action_masks=mask, deterministic=True)
        onnx_action = onnx_policy.predict(obs, action_mask=mask, deterministic=True)
        assert int(sb3_action) == int(onnx_action)
        obs, _reward, term, trunc, info = env.step(int(sb3_action))
        if term or trunc:
            break


@pytest.mark.slow
def test_lstm_onnx_roundtrip(tmp_path):
    from digimon_gym.agents.maskable_recurrent import MaskableRecurrentPPO

    env = ActionMasker(DigimonEnv(), _action_mask_fn)
    model = MaskableRecurrentPPO(
        "MlpLstmPolicy",
        env,
        n_steps=64,
        batch_size=32,
        policy_kwargs=dict(lstm_hidden_size=64),
        verbose=0,
    )
    model.learn(total_timesteps=128)

    sb3_path = tmp_path / "tiny_lstm.zip"
    onnx_path = tmp_path / "tiny_lstm.onnx"
    model.save(str(sb3_path))

    result = subprocess.run(
        [
            "python",
            "code/tools/export_onnx.py",
            "--type",
            "lstm",
            "--input",
            str(sb3_path),
            "--output",
            str(onnx_path),
            # Same profile pin as the MLP test above.
            "--tensor-profile",
            "standard_lite_v2",
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr

    onnx_policy = OnnxPolicy(str(onnx_path), hidden_size=64)
    onnx_policy.reset()

    obs, info = env.reset(seed=7)
    state = None
    episode_start = np.array([True])
    for _ in range(20):
        mask = info["action_mask"]
        sb3_action, state = model.predict(
            obs,
            state=state,
            episode_start=episode_start,
            action_masks=mask,
            deterministic=True,
        )
        episode_start = np.array([False])
        onnx_action = onnx_policy.predict(obs, action_mask=mask, deterministic=True)
        assert int(sb3_action) == int(onnx_action)
        obs, _reward, term, trunc, info = env.step(int(sb3_action))
        if term or trunc:
            break

"""Value-head ONNX export tests (add-determinized-search task 0.2).

The MLP export gains a ``value`` output on the SAME graph (backward
compatible: all consumers fetch outputs by name and detect model type by
presence, not exact set). The LSTM export keeps its main-graph signature
untouched and emits a companion ``<stem>.value.onnx`` that threads the
CRITIC LSTM's own state (``enable_critic_lstm=True`` is the training
default, so the value path has separate recurrent state from the actor).
"""

from __future__ import annotations

import subprocess
from pathlib import Path

import numpy as np
import pytest

pytest.importorskip("onnxruntime")
pytest.importorskip("torch")

import onnxruntime as ort_rt  # noqa: E402
import torch  # noqa: E402
from sb3_contrib import MaskablePPO  # noqa: E402
from sb3_contrib.common.wrappers import ActionMasker  # noqa: E402

from digimon_gym.digimon_gym import DigimonEnv  # noqa: E402


def _action_mask_fn(env):
    return env.action_mask()


def _export(model_type: str, sb3_path: Path, onnx_path: Path) -> None:
    result = subprocess.run(
        [
            "python",
            "code/tools/export_onnx.py",
            "--type",
            model_type,
            "--input",
            str(sb3_path),
            "--output",
            str(onnx_path),
            # Pin to DigimonEnv's default profile (see test_onnx_roundtrip).
            "--tensor-profile",
            "standard_lite_v2",
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    assert onnx_path.exists()


@pytest.mark.slow
def test_mlp_export_emits_value_head(tmp_path):
    env = ActionMasker(DigimonEnv(), _action_mask_fn)
    model = MaskablePPO("MlpPolicy", env, n_steps=64, batch_size=32, verbose=0)
    model.learn(total_timesteps=128)

    sb3_path = tmp_path / "tiny.zip"
    onnx_path = tmp_path / "tiny.onnx"
    model.save(str(sb3_path))
    _export("mlp", sb3_path, onnx_path)

    sess = ort_rt.InferenceSession(str(onnx_path))
    output_names = {o.name for o in sess.get_outputs()}
    assert "logits" in output_names  # unchanged contract
    assert "value" in output_names  # task 0.2 addition

    # Value output matches SB3's predict_values on a real observation.
    obs, _info = env.reset(seed=11)
    obs_batch = np.asarray(obs, dtype=np.float32)[None, :]
    (onnx_value,) = sess.run(["value"], {"obs": obs_batch})
    with torch.no_grad():
        sb3_value = (
            model.policy.predict_values(
                torch.as_tensor(obs_batch, device=model.device)
            )
            .cpu()
            .numpy()
        )
    assert onnx_value.shape == (1, 1)
    np.testing.assert_allclose(onnx_value, sb3_value, atol=1e-4)


@pytest.mark.slow
def test_lstm_export_emits_companion_value_graph(tmp_path):
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
    _export("lstm", sb3_path, onnx_path)

    # Main graph signature is UNCHANGED (desktop/eval consumers feed exactly
    # obs/h_in/c_in and would break on any new required input).
    main = ort_rt.InferenceSession(str(onnx_path))
    assert {i.name for i in main.get_inputs()} == {"obs", "h_in", "c_in"}
    assert {o.name for o in main.get_outputs()} == {"logits", "h_out", "c_out"}

    # Companion value graph threads the CRITIC's own LSTM state.
    value_path = onnx_path.with_suffix(".value.onnx")
    assert value_path.exists()
    vsess = ort_rt.InferenceSession(str(value_path))
    assert {i.name for i in vsess.get_inputs()} == {"obs", "h_in", "c_in"}
    assert {o.name for o in vsess.get_outputs()} == {"value", "h_out", "c_out"}

    hidden = 64
    obs, info = env.reset(seed=13)
    h = np.zeros((1, 1, hidden), dtype=np.float32)
    c = np.zeros((1, 1, hidden), dtype=np.float32)
    values = []
    for _ in range(5):
        obs_batch = np.asarray(obs, dtype=np.float32)[None, :]
        value, h, c = vsess.run(
            ["value", "h_out", "c_out"],
            {"obs": obs_batch, "h_in": h, "c_in": c},
        )
        assert value.shape == (1, 1)
        assert np.isfinite(value).all()
        values.append(float(value[0, 0]))
        mask = info["action_mask"]
        legal = np.flatnonzero(mask)
        obs, _r, term, trunc, info = env.step(int(legal[0]))
        if term or trunc:
            break
    # State threading is live: the critic state must influence the value.
    assert len(set(values)) > 1

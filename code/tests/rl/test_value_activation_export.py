"""A tanh value head is baked into the ONNX graph and recorded in its metadata.

Native search reads the ONNX ``value`` output directly, so the activation must
live in the graph; legacy (identity) exports must stay byte-for-byte unchanged
in their metadata keys.
"""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import numpy as np
import pytest

pytest.importorskip("torch")
pytest.importorskip("sb3_contrib")
onnx = pytest.importorskip("onnx")
ort = pytest.importorskip("onnxruntime")

import gymnasium as gym  # noqa: E402
import torch  # noqa: E402
from sb3_contrib import MaskablePPO  # noqa: E402

REPO = Path(__file__).resolve().parents[3]


def _export_tool():
    spec = importlib.util.spec_from_file_location("export_onnx_under_test", REPO / "code/tools/export_onnx.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@pytest.mark.parametrize("activation", ["identity", "tanh"])
def test_export_bakes_value_activation(tmp_path, activation):
    from digimon_gym.tensor_profiles import get_tensor_profile
    from model_contract import current_contract

    export = _export_tool()
    profile = get_tensor_profile("standard_lite_v2")

    class Spaces(gym.Env):
        observation_space = gym.spaces.Box(-10.0, 20001.0, (profile.tensor_size,), np.float32)
        action_space = gym.spaces.Discrete(export.ACTION_SPACE_SIZE)

    model = MaskablePPO("MlpPolicy", Spaces(), n_steps=8, batch_size=8, seed=0, device="cpu",
                        policy_kwargs=dict(net_arch=dict(pi=[16], vf=[16])))
    model.digimon_contract = current_contract(profile.id)
    model.digimon_value_semantics = "terminal_outcome"
    if activation == "tanh":
        model.digimon_value_activation = "tanh"
    with torch.no_grad():
        model.policy.value_net.bias.fill_(40.0)  # far outside [-1, 1] before the squash
    checkpoint = tmp_path / "model.zip"
    model.save(str(checkpoint))

    out = tmp_path / "model.onnx"
    export.export_mlp(str(checkpoint), str(out), profile)

    obs = np.random.default_rng(0).uniform(0, 1, (4, profile.tensor_size)).astype(np.float32)
    value = ort.InferenceSession(str(out), providers=["CPUExecutionProvider"]).run(["value"], {"obs": obs})[0]
    with torch.no_grad():
        raw = model.policy.predict_values(torch.as_tensor(obs)).numpy()
    props = {p.key: p.value for p in onnx.load(str(out)).metadata_props}
    meta = json.loads(Path(f"{out}.meta.json").read_text())
    if activation == "tanh":
        assert np.all(np.abs(value) < 1.0 + 1e-6)
        np.testing.assert_allclose(value, np.tanh(raw), atol=1e-5)
        assert props["digimon_value_activation"] == "tanh"
        assert meta["value_activation"] == "tanh"
    else:
        np.testing.assert_allclose(value, raw, atol=1e-4)
        assert np.all(value > 1.0)
        assert "digimon_value_activation" not in props
        assert "value_activation" not in meta

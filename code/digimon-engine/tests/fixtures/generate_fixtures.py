"""Regenerate tiny deterministic ONNX fixtures + expected outputs for Rust inference parity tests.

The numerical core stays small (8 inputs, 16 logits, hidden=4). Public input/output
shapes and embedded metadata use the real current default Rust contract. Exact
zero input padding and masked output padding preserve the existing small-core
parity checks without bypassing production compatibility gates.
Weights are seeded for reproducibility; expected outputs come from onnxruntime so the
Rust tests assert against the same numerics Python sees.

Run from repo root:
    python digimon-engine/tests/fixtures/generate_fixtures.py
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
import onnxruntime as ort
import torch
import torch.nn as nn
import onnx
sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from model_contract import current_contract, embed_onnx_contract
CONTRACT = current_contract("standard_lite_deck_v2")


OBS_SIZE = 8
ACTION_SIZE = 16
HIDDEN_SIZE = 4
SEED = 0xD191


class TinyMlp(nn.Module):
    def __init__(self) -> None:
        super().__init__()
        self.fc = nn.Linear(OBS_SIZE, ACTION_SIZE)

    def forward(self, obs: torch.Tensor) -> torch.Tensor:
        return self.fc(obs)


class TinyMlpValue(nn.Module):
    """Policy+value graph shaped like the task-0.2 MLP export (logits + value
    outputs, dynamic batch axis) for the batched-evaluator parity test."""

    def __init__(self) -> None:
        super().__init__()
        self.fc = nn.Linear(OBS_SIZE, ACTION_SIZE)
        self.vf = nn.Linear(OBS_SIZE, 1)

    def forward(self, obs: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        return self.fc(obs), self.vf(obs)


class TinyLstm(nn.Module):
    def __init__(self) -> None:
        super().__init__()
        self.lstm = nn.LSTM(OBS_SIZE, HIDDEN_SIZE, num_layers=1, batch_first=False)
        self.head = nn.Linear(HIDDEN_SIZE, ACTION_SIZE)

    def forward(
        self,
        obs: torch.Tensor,
        h_in: torch.Tensor,
        c_in: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        seq = obs.unsqueeze(0)
        out, (h_out, c_out) = self.lstm(seq, (h_in, c_in))
        logits = self.head(out.squeeze(0))
        return logits, h_out, c_out


def _masked_argmax(logits: np.ndarray, mask: np.ndarray) -> int:
    masked = logits.copy()
    masked[mask < 0.5] = -1e9
    return int(np.argmax(masked))



def extend_to_current_contract(path: Path) -> None:
    model = onnx.load(str(path))
    for node in model.graph.node:
        for index, name in enumerate(node.input):
            if name == "obs": node.input[index] = "core_obs"
        for index, name in enumerate(node.output):
            if name == "logits": node.output[index] = "core_logits"
    for value in model.graph.input:
        if value.name == "obs": value.type.tensor_type.shape.dim[1].dim_value = CONTRACT["tensor_size"]
    for value in model.graph.output:
        if value.name == "logits": value.type.tensor_type.shape.dim[1].dim_value = CONTRACT["action_space_size"]
    constants = [
        ("slice_start", np.array([0], dtype=np.int64)),
        ("slice_end", np.array([OBS_SIZE], dtype=np.int64)),
        ("slice_axis", np.array([1], dtype=np.int64)),
        ("pad_axes", np.array([0, 0, 0, CONTRACT["action_space_size"] - ACTION_SIZE], dtype=np.int64)),
        ("pad_logit", np.array(-1e9, dtype=np.float32)),
    ]
    for name, data in constants: model.graph.initializer.append(onnx.numpy_helper.from_array(data, name))
    model.graph.node.insert(0, onnx.helper.make_node("Slice", ["obs", "slice_start", "slice_end", "slice_axis"], ["core_obs"]))
    model.graph.node.append(onnx.helper.make_node("Pad", ["core_logits", "pad_axes", "pad_logit"], ["logits"], mode="constant"))
    onnx.checker.check_model(model)
    onnx.save(model, str(path))
    embed_onnx_contract(path, CONTRACT)

def main() -> None:
    torch.manual_seed(SEED)
    np.random.seed(SEED)

    fixtures_dir = Path(__file__).parent
    mlp_path = fixtures_dir / "mlp_tiny.onnx"
    mlp_value_path = fixtures_dir / "mlp_tiny_value.onnx"
    lstm_path = fixtures_dir / "lstm_tiny.onnx"
    expected_path = fixtures_dir / "expected.json"

    rng = np.random.default_rng(SEED)
    obs = rng.standard_normal(OBS_SIZE).astype(np.float32)
    mask = np.ones(ACTION_SIZE, dtype=np.float32)
    # Disable a handful of actions so masking has something to do.
    mask[[0, 3, 7, 12]] = 0.0

    mlp = TinyMlp()
    torch.onnx.export(
        mlp,
        (torch.from_numpy(obs).unsqueeze(0),),
        str(mlp_path),
        input_names=["obs"],
        output_names=["logits"],
        dynamic_axes=None,
        opset_version=17,
        dynamo=False,
    )

    mlp_value = TinyMlpValue()
    torch.onnx.export(
        mlp_value,
        (torch.from_numpy(obs).unsqueeze(0),),
        str(mlp_value_path),
        input_names=["obs"],
        output_names=["logits", "value"],
        dynamic_axes={"obs": {0: "batch"}, "logits": {0: "batch"}, "value": {0: "batch"}},
        opset_version=17,
        dynamo=False,
    )

    lstm = TinyLstm()
    h0 = torch.zeros(1, 1, HIDDEN_SIZE)
    c0 = torch.zeros(1, 1, HIDDEN_SIZE)
    torch.onnx.export(
        lstm,
        (torch.from_numpy(obs).unsqueeze(0), h0, c0),
        str(lstm_path),
        input_names=["obs", "h_in", "c_in"],
        output_names=["logits", "h_out", "c_out"],
        dynamic_axes=None,
        opset_version=17,
        dynamo=False,
    )

    for path in [mlp_path, mlp_value_path, lstm_path]:
        extend_to_current_contract(path)
    lite_contract = current_contract("standard_lite_v2")
    lite_path = fixtures_dir / "mlp_lite_value.onnx"
    lite = onnx.load(str(mlp_value_path))
    next(v for v in lite.graph.input if v.name == "obs").type.tensor_type.shape.dim[1].dim_value = lite_contract["tensor_size"]
    onnx.save(lite, str(lite_path))
    embed_onnx_contract(lite_path, lite_contract)
    obs = np.pad(obs, (0, CONTRACT["tensor_size"] - OBS_SIZE))
    mask = np.pad(mask, (0, CONTRACT["action_space_size"] - ACTION_SIZE))

    mlp_sess = ort.InferenceSession(str(mlp_path), providers=["CPUExecutionProvider"])
    (mlp_logits,) = mlp_sess.run(["logits"], {"obs": obs.reshape(1, -1)})
    mlp_action = _masked_argmax(mlp_logits[0], mask)

    # Batched policy+value fixture: two rows, second is a scaled copy so the
    # per-row outputs differ and row-major layout bugs are caught.
    batch_obs = np.stack([obs, obs * 0.5]).astype(np.float32)
    mask2 = np.ones(ACTION_SIZE, dtype=np.float32)
    mask2[[1, 2, 4, 5, 6, 8, 9, 10, 11, 13, 14, 15]] = 0.0  # only 0,3,7,12 legal
    mask2 = np.pad(mask2, (0, CONTRACT["action_space_size"] - ACTION_SIZE))
    mv_sess = ort.InferenceSession(str(mlp_value_path), providers=["CPUExecutionProvider"])
    mv_logits, mv_values = mv_sess.run(["logits", "value"], {"obs": batch_obs})

    lstm_sess = ort.InferenceSession(str(lstm_path), providers=["CPUExecutionProvider"])
    h = np.zeros((1, 1, HIDDEN_SIZE), dtype=np.float32)
    c = np.zeros((1, 1, HIDDEN_SIZE), dtype=np.float32)
    lstm_logits_step1, h1, c1 = lstm_sess.run(
        ["logits", "h_out", "c_out"],
        {"obs": obs.reshape(1, -1), "h_in": h, "c_in": c},
    )
    lstm_action_step1 = _masked_argmax(lstm_logits_step1[0], mask)
    # Second step uses the state from step 1 to exercise state threading in Rust.
    lstm_logits_step2, _, _ = lstm_sess.run(
        ["logits", "h_out", "c_out"],
        {"obs": obs.reshape(1, -1), "h_in": h1, "c_in": c1},
    )
    lstm_action_step2 = _masked_argmax(lstm_logits_step2[0], mask)

    expected = {
        "shapes": {
            "obs_size": CONTRACT["tensor_size"],
            "action_size": CONTRACT["action_space_size"],
            "hidden_size": HIDDEN_SIZE,
        },
        "obs": obs.tolist(),
        "mask": mask.tolist(),
        "mlp": {
            "logits": mlp_logits[0].tolist(),
            "action": mlp_action,
        },
        "lstm": {
            "step1_logits": lstm_logits_step1[0].tolist(),
            "step1_action": lstm_action_step1,
            "step2_logits": lstm_logits_step2[0].tolist(),
            "step2_action": lstm_action_step2,
        },
        "mlp_value": {
            "batch_obs": batch_obs.tolist(),
            "masks": [mask.tolist(), mask2.tolist()],
            "logits": mv_logits.tolist(),
            "values": mv_values.reshape(-1).tolist(),
        },
    }
    expected_path.write_text(json.dumps(expected, indent=2))
    print(f"Wrote {mlp_path}, {mlp_value_path}, {lstm_path}, {expected_path}")


if __name__ == "__main__":
    main()

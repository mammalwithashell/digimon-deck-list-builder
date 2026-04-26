"""Export a random-weights ONNX policy for bootstrapping the desktop AI.

The desktop alpha ships with an empty model manifest, but the inference plumbing
(manifest fetch -> SHA-verified download -> onnxruntime) is live. This tool
produces a minimum-viable ONNX policy with uninitialized random weights so an
"AI opponent" can play legal-random moves via the action mask before any real
training has happened.

No checkpoint needed - torch.nn.Linear layers with default init are exported
straight to ONNX. Output shapes and node names match tools/export_onnx.py so
digimon_gym/engine/onnx_policy.py loads the file without any changes.

Usage:
    python tools/export_random_onnx.py --type mlp  --output /tmp/random-mlp.onnx
    python tools/export_random_onnx.py --type lstm --output /tmp/random-lstm.onnx
"""

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn

from digimon_engine import ACTION_SPACE_SIZE, TENSOR_SIZE

HIDDEN_SIZE = 256
LSTM_HIDDEN_SIZE = 256


class RandomMlp(nn.Module):
    """MLP with random weights: obs -> logits."""

    def __init__(self) -> None:
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(TENSOR_SIZE, HIDDEN_SIZE),
            nn.Tanh(),
            nn.Linear(HIDDEN_SIZE, HIDDEN_SIZE),
            nn.Tanh(),
            nn.Linear(HIDDEN_SIZE, ACTION_SPACE_SIZE),
        )

    def forward(self, obs: torch.Tensor) -> torch.Tensor:
        return self.net(obs)


class RandomLstm(nn.Module):
    """LSTM with random weights, mirroring LstmActorWrapper I/O signature."""

    def __init__(self) -> None:
        super().__init__()
        self.features = nn.Sequential(
            nn.Linear(TENSOR_SIZE, LSTM_HIDDEN_SIZE),
            nn.Tanh(),
        )
        self.lstm = nn.LSTM(LSTM_HIDDEN_SIZE, LSTM_HIDDEN_SIZE, num_layers=1)
        self.head = nn.Sequential(
            nn.Linear(LSTM_HIDDEN_SIZE, LSTM_HIDDEN_SIZE),
            nn.Tanh(),
            nn.Linear(LSTM_HIDDEN_SIZE, ACTION_SPACE_SIZE),
        )

    def forward(
        self,
        obs: torch.Tensor,
        h: torch.Tensor,
        c: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        features = self.features(obs)
        lstm_in = features.unsqueeze(0)
        lstm_out, (h_out, c_out) = self.lstm(lstm_in, (h, c))
        lstm_out = lstm_out.squeeze(0)
        logits = self.head(lstm_out)
        return logits, h_out, c_out


def _inference_mode(model: nn.Module) -> nn.Module:
    model.train(False)
    return model


def export_mlp(output_path: str) -> None:
    model = _inference_mode(RandomMlp())
    dummy_obs = torch.randn(1, TENSOR_SIZE, dtype=torch.float32)

    with torch.no_grad():
        logits_shape = tuple(model(dummy_obs).shape)
    if logits_shape != (1, ACTION_SPACE_SIZE):
        raise ValueError(
            f"MLP produced logits shape {logits_shape}, expected (1, {ACTION_SPACE_SIZE})"
        )

    torch.onnx.export(
        model,
        (dummy_obs,),
        output_path,
        input_names=["obs"],
        output_names=["logits"],
        dynamic_axes={"obs": {0: "batch"}, "logits": {0: "batch"}},
        opset_version=17,
        dynamo=False,
    )
    print(f"Exported random MLP model to {output_path}")
    _verify_mlp(output_path)


def export_lstm(output_path: str) -> None:
    model = _inference_mode(RandomLstm())
    dummy_obs = torch.randn(1, TENSOR_SIZE, dtype=torch.float32)
    dummy_h = torch.zeros(1, 1, LSTM_HIDDEN_SIZE, dtype=torch.float32)
    dummy_c = torch.zeros(1, 1, LSTM_HIDDEN_SIZE, dtype=torch.float32)

    with torch.no_grad():
        logits_shape = tuple(model(dummy_obs, dummy_h, dummy_c)[0].shape)
    if logits_shape != (1, ACTION_SPACE_SIZE):
        raise ValueError(
            f"LSTM produced logits shape {logits_shape}, expected (1, {ACTION_SPACE_SIZE})"
        )

    torch.onnx.export(
        model,
        (dummy_obs, dummy_h, dummy_c),
        output_path,
        input_names=["obs", "h_in", "c_in"],
        output_names=["logits", "h_out", "c_out"],
        dynamic_axes={
            "obs": {0: "batch"},
            "logits": {0: "batch"},
        },
        opset_version=17,
        dynamo=False,
    )
    print(f"Exported random LSTM model to {output_path}")
    _verify_lstm(output_path)


def _verify_mlp(onnx_path: str) -> None:
    from digimon_gym.inference.onnx_policy import load_onnx_policy

    policy = load_onnx_policy(onnx_path, model_type="auto")
    obs = np.random.randn(TENSOR_SIZE).astype(np.float32)
    mask = np.ones(ACTION_SPACE_SIZE, dtype=np.float32)
    action = policy.predict(obs, mask)
    if not (0 <= action < ACTION_SPACE_SIZE):
        raise ValueError(f"MLP predict returned out-of-range action {action}")
    print(f"  MLP verification: loads via onnx_policy, sample action={action}")


def _verify_lstm(onnx_path: str) -> None:
    from digimon_gym.inference.onnx_policy import load_onnx_policy

    policy = load_onnx_policy(onnx_path, model_type="auto")
    obs = np.random.randn(TENSOR_SIZE).astype(np.float32)
    mask = np.ones(ACTION_SPACE_SIZE, dtype=np.float32)
    action = policy.predict(obs, mask)
    if not (0 <= action < ACTION_SPACE_SIZE):
        raise ValueError(f"LSTM predict returned out-of-range action {action}")
    print(f"  LSTM verification: loads via onnx_policy, sample action={action}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Export a random-weights ONNX policy for desktop bootstrapping."
    )
    parser.add_argument("--type", choices=["mlp", "lstm"], default="mlp")
    parser.add_argument("--output", required=True, help="Output .onnx path")
    parser.add_argument("--seed", type=int, default=None, help="Optional RNG seed")
    args = parser.parse_args()

    if args.seed is not None:
        torch.manual_seed(args.seed)

    Path(args.output).parent.mkdir(parents=True, exist_ok=True)

    if args.type == "mlp":
        export_mlp(args.output)
    else:
        export_lstm(args.output)


if __name__ == "__main__":
    main()

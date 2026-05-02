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
import json
import sys
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn

from digimon_engine import ACTION_SPACE_SIZE, EMBEDDING_DIM, REGISTRY_CAPACITY, TENSOR_SIZE

CODE_ROOT = Path(__file__).resolve().parents[1]
if str(CODE_ROOT) not in sys.path:
    sys.path.insert(0, str(CODE_ROOT))

from digimon_gym.tensor_profiles import get_tensor_profile

HIDDEN_SIZE = 256
LSTM_HIDDEN_SIZE = 256


class RandomMlp(nn.Module):
    """MLP with random weights: obs -> logits."""

    def __init__(self, tensor_size: int = TENSOR_SIZE) -> None:
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(tensor_size, HIDDEN_SIZE),
            nn.Tanh(),
            nn.Linear(HIDDEN_SIZE, HIDDEN_SIZE),
            nn.Tanh(),
            nn.Linear(HIDDEN_SIZE, ACTION_SPACE_SIZE),
        )

    def forward(self, obs: torch.Tensor) -> torch.Tensor:
        return self.net(obs)


class RandomLstm(nn.Module):
    """LSTM with random weights, mirroring LstmActorWrapper I/O signature."""

    def __init__(self, tensor_size: int = TENSOR_SIZE) -> None:
        super().__init__()
        self.features = nn.Sequential(
            nn.Linear(tensor_size, LSTM_HIDDEN_SIZE),
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


def write_export_metadata(output_path: str | Path, observation_layout) -> None:
    """Write ONNX profile metadata next to the exported model."""
    metadata = {
        "observation_profile": observation_layout.id,
        "tensor_version": observation_layout.tensor_version,
        "feature_schema_version": observation_layout.feature_schema_version,
        "tensor_size": observation_layout.tensor_size,
        "tensor_layout_hash": observation_layout.layout_hash,
        "action_space_size": ACTION_SPACE_SIZE,
        "card_registry_capacity": REGISTRY_CAPACITY,
        "embedding_dim": EMBEDDING_DIM,
    }
    meta_path = Path(f"{output_path}.meta.json")
    meta_path.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n")
    print(f"Wrote ONNX metadata to {meta_path}")


def export_mlp(output_path: str, observation_layout=None) -> None:
    observation_layout = observation_layout or get_tensor_profile()
    tensor_size = observation_layout.tensor_size
    model = _inference_mode(RandomMlp(tensor_size))
    dummy_obs = torch.randn(1, tensor_size, dtype=torch.float32)

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
    _verify_mlp(output_path, tensor_size)
    write_export_metadata(output_path, observation_layout)


def export_lstm(output_path: str, observation_layout=None) -> None:
    observation_layout = observation_layout or get_tensor_profile()
    tensor_size = observation_layout.tensor_size
    model = _inference_mode(RandomLstm(tensor_size))
    dummy_obs = torch.randn(1, tensor_size, dtype=torch.float32)
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
    _verify_lstm(output_path, tensor_size)
    write_export_metadata(output_path, observation_layout)


def _verify_mlp(onnx_path: str, tensor_size: int = TENSOR_SIZE) -> None:
    from digimon_gym.inference.onnx_policy import load_onnx_policy

    policy = load_onnx_policy(onnx_path, model_type="auto")
    obs = np.random.randn(tensor_size).astype(np.float32)
    mask = np.ones(ACTION_SPACE_SIZE, dtype=np.float32)
    action = policy.predict(obs, mask)
    if not (0 <= action < ACTION_SPACE_SIZE):
        raise ValueError(f"MLP predict returned out-of-range action {action}")
    print(f"  MLP verification: loads via onnx_policy, sample action={action}")


def _verify_lstm(onnx_path: str, tensor_size: int = TENSOR_SIZE) -> None:
    from digimon_gym.inference.onnx_policy import load_onnx_policy

    policy = load_onnx_policy(onnx_path, model_type="auto")
    obs = np.random.randn(tensor_size).astype(np.float32)
    mask = np.ones(ACTION_SPACE_SIZE, dtype=np.float32)
    action = policy.predict(obs, mask)
    if not (0 <= action < ACTION_SPACE_SIZE):
        raise ValueError(f"LSTM predict returned out-of-range action {action}")
    print(f"  LSTM verification: loads via onnx_policy, sample action={action}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Export a random-weights ONNX policy for desktop bootstrapping."
    )
    parser.add_argument("--type", choices=["mlp", "lstm"], default="mlp")
    parser.add_argument("--output", required=True, help="Output .onnx path")
    parser.add_argument("--seed", type=int, default=None, help="Optional RNG seed")
    parser.add_argument(
        "--tensor-profile",
        default=None,
        help="Observation profile for dummy input shape and metadata.",
    )
    return parser


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()

    if args.seed is not None:
        torch.manual_seed(args.seed)

    Path(args.output).parent.mkdir(parents=True, exist_ok=True)
    observation_layout = get_tensor_profile(args.tensor_profile)

    if args.type == "mlp":
        export_mlp(args.output, observation_layout)
    else:
        export_lstm(args.output, observation_layout)


if __name__ == "__main__":
    main()

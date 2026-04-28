"""Export SB3 MaskablePPO / MaskableRecurrentPPO models to ONNX format.

Requires PyTorch + SB3 — run on a dev machine, NOT on end-user desktops.
The resulting .onnx files can be loaded with onnxruntime (no PyTorch needed).

Usage:
    python tools/export_onnx.py --type mlp --input models/mlp_agent.zip --output models/mlp_agent.onnx
    python tools/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn

from digimon_engine import ACTION_SPACE_SIZE, TENSOR_SIZE


CODE_ROOT = Path(__file__).resolve().parents[1]
if str(CODE_ROOT) not in sys.path:
    sys.path.insert(0, str(CODE_ROOT))

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")


# ---------------------------------------------------------------------------
# Wrapper modules that isolate the actor forward pass for tracing
# ---------------------------------------------------------------------------

class MlpActorWrapper(nn.Module):
    """Wraps SB3 MLP policy's actor path for ONNX export.

    Forward: obs (batch, TENSOR_SIZE) -> logits (batch, ACTION_SPACE_SIZE)
    """

    def __init__(self, policy):
        super().__init__()
        self.features_extractor = policy.features_extractor
        self.pi_features_extractor = getattr(policy, "pi_features_extractor", None)
        self.mlp_extractor_policy = policy.mlp_extractor.policy_net
        self.action_net = policy.action_net

    def forward(self, obs: torch.Tensor) -> torch.Tensor:
        extractor = self.pi_features_extractor or self.features_extractor
        features = extractor(obs)
        latent_pi = self.mlp_extractor_policy(features)
        return self.action_net(latent_pi)


class LstmActorWrapper(nn.Module):
    """Wraps recurrent policy's actor path (LSTM + MLP head) for ONNX export.

    Forward: obs (1, TENSOR_SIZE), h (1, 1, 256), c (1, 1, 256)
          -> logits (1, ACTION_SPACE_SIZE), h_out (1, 1, 256), c_out (1, 1, 256)
    """

    def __init__(self, policy):
        super().__init__()
        self.features_extractor = policy.features_extractor
        self.lstm_actor = policy.lstm_actor
        self.mlp_extractor_policy = policy.mlp_extractor.policy_net
        self.action_net = policy.action_net

    def forward(
        self,
        obs: torch.Tensor,
        h: torch.Tensor,
        c: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        features = self.features_extractor(obs)
        # LSTM expects (seq_len, batch, input_size)
        lstm_in = features.unsqueeze(0)  # (1, batch, TENSOR_SIZE)
        lstm_out, (h_out, c_out) = self.lstm_actor(lstm_in, (h, c))
        lstm_out = lstm_out.squeeze(0)  # (batch, 256)
        latent_pi = self.mlp_extractor_policy(lstm_out)
        logits = self.action_net(latent_pi)
        return logits, h_out, c_out


# ---------------------------------------------------------------------------
# Export functions
# ---------------------------------------------------------------------------

def export_mlp(sb3_zip_path: str, output_path: str) -> None:
    """Export MaskablePPO MLP model to ONNX."""
    from sb3_contrib import MaskablePPO

    model = MaskablePPO.load(sb3_zip_path, device="cpu")
    policy = model.policy
    policy.eval()

    wrapper = MlpActorWrapper(policy)
    wrapper.eval()

    dummy_obs = torch.randn(1, TENSOR_SIZE, dtype=torch.float32)

    with torch.no_grad():
        logits_shape = tuple(wrapper(dummy_obs).shape)
    if logits_shape != (1, ACTION_SPACE_SIZE):
        raise ValueError(
            f"MLP checkpoint produces logits shape {logits_shape}, expected "
            f"(1, {ACTION_SPACE_SIZE}). The checkpoint was trained against a "
            f"stale tensor/action layout and must be retrained."
        )

    torch.onnx.export(
        wrapper,
        (dummy_obs,),
        output_path,
        input_names=["obs"],
        output_names=["logits"],
        dynamic_axes={"obs": {0: "batch"}, "logits": {0: "batch"}},
        opset_version=17,
        dynamo=False,
    )
    print(f"Exported MLP model to {output_path}")

    # Verify round-trip
    _verify_mlp(policy, wrapper, output_path)


def export_lstm(sb3_zip_path: str, output_path: str) -> None:
    """Export MaskableRecurrentPPO LSTM model to ONNX."""
    from digimon_gym.agents.maskable_recurrent import MaskableRecurrentPPO

    model = MaskableRecurrentPPO.load(sb3_zip_path, device="cpu")
    policy = model.policy
    policy.eval()

    wrapper = LstmActorWrapper(policy)
    wrapper.eval()

    dummy_obs = torch.randn(1, TENSOR_SIZE, dtype=torch.float32)
    hidden_size = policy.lstm_actor.hidden_size
    dummy_h = torch.zeros(1, 1, hidden_size, dtype=torch.float32)
    dummy_c = torch.zeros(1, 1, hidden_size, dtype=torch.float32)

    with torch.no_grad():
        logits_shape = tuple(wrapper(dummy_obs, dummy_h, dummy_c)[0].shape)
    if logits_shape != (1, ACTION_SPACE_SIZE):
        raise ValueError(
            f"LSTM checkpoint produces logits shape {logits_shape}, expected "
            f"(1, {ACTION_SPACE_SIZE}). The checkpoint was trained against a "
            f"stale tensor/action layout and must be retrained."
        )

    torch.onnx.export(
        wrapper,
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
    print(f"Exported LSTM model to {output_path}")

    # Verify round-trip
    _verify_lstm(policy, wrapper, output_path)


# ---------------------------------------------------------------------------
# Verification helpers
# ---------------------------------------------------------------------------

def _verify_mlp(policy, wrapper, onnx_path: str) -> None:
    """Verify ONNX output matches PyTorch output."""
    import onnxruntime as ort

    dummy_obs = torch.randn(1, TENSOR_SIZE, dtype=torch.float32)

    with torch.no_grad():
        pt_logits = wrapper(dummy_obs).numpy()

    sess = ort.InferenceSession(onnx_path)
    ort_logits = sess.run(["logits"], {"obs": dummy_obs.numpy()})[0]

    if ort_logits.shape != (1, ACTION_SPACE_SIZE):
        raise ValueError(
            f"Exported ONNX logits shape {ort_logits.shape} != "
            f"(1, {ACTION_SPACE_SIZE})"
        )

    max_diff = np.max(np.abs(pt_logits - ort_logits))
    print(f"  MLP verification: max logit diff = {max_diff:.6e}")
    assert max_diff < 1e-4, f"MLP output mismatch: max diff {max_diff}"


def _verify_lstm(policy, wrapper, onnx_path: str) -> None:
    """Verify ONNX output matches PyTorch output."""
    import onnxruntime as ort

    dummy_obs = torch.randn(1, TENSOR_SIZE, dtype=torch.float32)
    hidden_size = policy.lstm_actor.hidden_size
    dummy_h = torch.zeros(1, 1, hidden_size, dtype=torch.float32)
    dummy_c = torch.zeros(1, 1, hidden_size, dtype=torch.float32)

    with torch.no_grad():
        pt_logits, pt_h, pt_c = wrapper(dummy_obs, dummy_h, dummy_c)
        pt_logits = pt_logits.numpy()
        pt_h = pt_h.numpy()
        pt_c = pt_c.numpy()

    sess = ort.InferenceSession(onnx_path)
    ort_out = sess.run(
        ["logits", "h_out", "c_out"],
        {"obs": dummy_obs.numpy(), "h_in": dummy_h.numpy(), "c_in": dummy_c.numpy()},
    )

    if ort_out[0].shape != (1, ACTION_SPACE_SIZE):
        raise ValueError(
            f"Exported ONNX logits shape {ort_out[0].shape} != "
            f"(1, {ACTION_SPACE_SIZE})"
        )

    max_logit_diff = np.max(np.abs(pt_logits - ort_out[0]))
    max_h_diff = np.max(np.abs(pt_h - ort_out[1]))
    max_c_diff = np.max(np.abs(pt_c - ort_out[2]))
    print(f"  LSTM verification: logits diff={max_logit_diff:.6e}, h diff={max_h_diff:.6e}, c diff={max_c_diff:.6e}")
    assert max_logit_diff < 1e-4, f"LSTM logit mismatch: {max_logit_diff}"


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Export SB3 models to ONNX")
    parser.add_argument("--type", choices=["mlp", "lstm"], required=True)
    parser.add_argument("--input", required=True, help="Path to SB3 .zip checkpoint")
    parser.add_argument("--output", required=True, help="Output .onnx path")
    args = parser.parse_args()

    Path(args.output).parent.mkdir(parents=True, exist_ok=True)

    if args.type == "mlp":
        export_mlp(args.input, args.output)
    else:
        export_lstm(args.input, args.output)


if __name__ == "__main__":
    main()

"""Lightweight ONNX-based policies for trained agent inference.

No PyTorch required — only numpy + onnxruntime.
These wrap exported .onnx models from scripts/export_onnx.py.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np


def _get_session(onnx_path: str):
    """Create an ONNX Runtime inference session."""
    import onnxruntime as ort

    return ort.InferenceSession(
        onnx_path,
        providers=["CPUExecutionProvider"],
    )


def _masked_argmax(logits: np.ndarray, mask: np.ndarray) -> int:
    """Apply action mask to logits and return the greedy action.

    Args:
        logits: shape (2120,) raw logits from the policy network
        mask: shape (2120,) where 1 = valid, 0 = invalid
    """
    masked = logits.copy()
    masked[mask < 0.5] = -1e9
    return int(np.argmax(masked))


class OnnxMlpPolicy:
    """ONNX MLP policy: obs → logits → masked argmax."""

    def __init__(self, onnx_path: str):
        if not Path(onnx_path).exists():
            raise FileNotFoundError(f"ONNX model not found: {onnx_path}")
        self.session = _get_session(onnx_path)

    def predict(self, obs: np.ndarray, action_mask: np.ndarray) -> int:
        """Run inference and return the selected action index.

        Args:
            obs: shape (981,) float32 game state tensor
            action_mask: shape (2120,) int8/float32 mask (1=valid)
        """
        obs_batch = obs.reshape(1, -1).astype(np.float32)
        (logits,) = self.session.run(["logits"], {"obs": obs_batch})
        return _masked_argmax(logits[0], action_mask)


class OnnxLstmPolicy:
    """ONNX LSTM policy with state threading across steps.

    Call reset() at episode boundaries to clear LSTM state.
    """

    def __init__(self, onnx_path: str, hidden_size: int = 256):
        if not Path(onnx_path).exists():
            raise FileNotFoundError(f"ONNX model not found: {onnx_path}")
        self.session = _get_session(onnx_path)
        self.hidden_size = hidden_size
        self.reset()

    def predict(self, obs: np.ndarray, action_mask: np.ndarray) -> int:
        """Run inference with LSTM state threading.

        Args:
            obs: shape (981,) float32 game state tensor
            action_mask: shape (2120,) int8/float32 mask (1=valid)
        """
        obs_batch = obs.reshape(1, -1).astype(np.float32)
        logits, h_out, c_out = self.session.run(
            ["logits", "h_out", "c_out"],
            {"obs": obs_batch, "h_in": self.h, "c_in": self.c},
        )
        self.h = h_out
        self.c = c_out
        return _masked_argmax(logits[0], action_mask)

    def reset(self) -> None:
        """Reset LSTM state at episode boundary."""
        self.h = np.zeros((1, 1, self.hidden_size), dtype=np.float32)
        self.c = np.zeros((1, 1, self.hidden_size), dtype=np.float32)


def load_onnx_policy(onnx_path: str, model_type: str = "mlp") -> OnnxMlpPolicy | OnnxLstmPolicy:
    """Load an ONNX policy by type.

    Args:
        onnx_path: path to the .onnx file
        model_type: "mlp" or "lstm"
    """
    if model_type == "lstm":
        return OnnxLstmPolicy(onnx_path)
    return OnnxMlpPolicy(onnx_path)

"""Lightweight ONNX-based policies for trained agent inference.

No PyTorch required — only numpy + onnxruntime.
These wrap exported .onnx models from tools/export_onnx.py.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np


def _get_session(onnx_path: str):
    """Create an ONNX Runtime inference session."""
    try:
        import onnxruntime as ort
    except ImportError:
        raise ImportError(
            "onnxruntime is required for trained agent inference. "
            "Install it with: pip install onnxruntime"
        )
    try:
        return ort.InferenceSession(
            onnx_path,
            providers=["CPUExecutionProvider"],
        )
    except Exception as e:
        raise RuntimeError(
            f"Failed to load ONNX model '{onnx_path}': {e}. "
            "The model file may be corrupted or incompatible."
        ) from e


def _masked_argmax(logits: np.ndarray, mask: np.ndarray) -> int:
    """Apply action mask to logits and return the greedy action.

    Args:
        logits: shape (2120,) raw logits from the policy network
        mask: shape (2120,) where 1 = valid, 0 = invalid
    """
    masked = logits.copy()
    masked[mask < 0.5] = -1e9
    return int(np.argmax(masked))


def _detect_model_type(session) -> str:
    """Detect model type by inspecting ONNX output names."""
    output_names = {o.name for o in session.get_outputs()}
    if {"logits", "h_out", "c_out"} <= output_names:
        return "lstm"
    if "logits" in output_names:
        return "mlp"
    raise ValueError(
        f"Unrecognized ONNX model outputs: {output_names}. "
        "Expected 'logits' (MLP) or 'logits','h_out','c_out' (LSTM)."
    )


class OnnxMlpPolicy:
    """ONNX MLP policy: obs → logits → masked argmax."""

    def __init__(self, onnx_path: str, session=None):
        if not Path(onnx_path).exists():
            raise FileNotFoundError(f"ONNX model not found: {onnx_path}")
        self.session = session or _get_session(onnx_path)

    def predict(
        self,
        obs: np.ndarray,
        action_mask: np.ndarray,
        deterministic: bool = True,
    ) -> int:
        """Run inference and return the selected action index.

        Args:
            obs: shape (981,) float32 game state tensor
            action_mask: shape (2120,) int8/float32 mask (1=valid)
        """
        obs_batch = obs.reshape(1, -1).astype(np.float32)
        (logits,) = self.session.run(["logits"], {"obs": obs_batch})
        return _masked_argmax(logits[0], action_mask)

    def predict_with_state(
        self,
        obs: np.ndarray,
        action_mask: np.ndarray,
        state=None,
        episode_start=None,
        deterministic: bool = True,
    ):
        return self.predict(obs, action_mask, deterministic=deterministic), state


class OnnxLstmPolicy:
    """ONNX LSTM policy with state threading across steps.

    Call reset() at episode boundaries to clear LSTM state.
    """

    def __init__(self, onnx_path: str, hidden_size: int = 256, session=None):
        if not Path(onnx_path).exists():
            raise FileNotFoundError(f"ONNX model not found: {onnx_path}")
        self.session = session or _get_session(onnx_path)
        self.hidden_size = hidden_size
        self.reset()

    def predict(
        self,
        obs: np.ndarray,
        action_mask: np.ndarray,
        deterministic: bool = True,
    ) -> int:
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

    def predict_with_state(
        self,
        obs: np.ndarray,
        action_mask: np.ndarray,
        state=None,
        episode_start=None,
        deterministic: bool = True,
    ):
        if episode_start is not None and bool(np.asarray(episode_start).reshape(-1)[0]):
            self.reset()
        if state is not None:
            self.h, self.c = state
        action = self.predict(obs, action_mask, deterministic=deterministic)
        return action, (self.h, self.c)

    def reset(self) -> None:
        """Reset LSTM state at episode boundary."""
        self.h = np.zeros((1, 1, self.hidden_size), dtype=np.float32)
        self.c = np.zeros((1, 1, self.hidden_size), dtype=np.float32)


def load_onnx_policy(onnx_path: str, model_type: str = "auto") -> OnnxMlpPolicy | OnnxLstmPolicy:
    """Load an ONNX policy by type.

    Args:
        onnx_path: path to the .onnx file
        model_type: "mlp", "lstm", or "auto" (detect from model outputs)
    """
    if not Path(onnx_path).exists():
        raise FileNotFoundError(f"ONNX model not found: {onnx_path}")
    if model_type == "auto":
        session = _get_session(onnx_path)
        model_type = _detect_model_type(session)
        # Pass pre-created session to avoid double-loading
        if model_type == "lstm":
            return OnnxLstmPolicy(onnx_path, session=session)
        return OnnxMlpPolicy(onnx_path, session=session)
    if model_type == "lstm":
        return OnnxLstmPolicy(onnx_path)
    return OnnxMlpPolicy(onnx_path)


class OnnxPolicy:
    """Compatibility wrapper that auto-detects MLP vs LSTM ONNX policies."""

    def __init__(self, onnx_path: str, model_type: str = "auto", hidden_size: int = 256):
        if model_type == "auto":
            session = _get_session(onnx_path)
            detected = _detect_model_type(session)
            if detected == "lstm":
                self.policy = OnnxLstmPolicy(
                    onnx_path,
                    hidden_size=hidden_size,
                    session=session,
                )
            else:
                self.policy = OnnxMlpPolicy(onnx_path, session=session)
        elif model_type == "lstm":
            self.policy = OnnxLstmPolicy(onnx_path, hidden_size=hidden_size)
        else:
            self.policy = OnnxMlpPolicy(onnx_path)

    def predict(
        self,
        obs: np.ndarray,
        action_mask: np.ndarray,
        deterministic: bool = True,
    ) -> int:
        return self.policy.predict(
            obs,
            action_mask=action_mask,
            deterministic=deterministic,
        )

    def predict_with_state(
        self,
        obs: np.ndarray,
        action_mask: np.ndarray,
        state=None,
        episode_start=None,
        deterministic: bool = True,
    ):
        return self.policy.predict_with_state(
            obs,
            action_mask=action_mask,
            state=state,
            episode_start=episode_start,
            deterministic=deterministic,
        )

    def reset(self) -> None:
        reset = getattr(self.policy, "reset", None)
        if reset is not None:
            reset()

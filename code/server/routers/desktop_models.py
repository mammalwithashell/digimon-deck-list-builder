"""Local ONNX model access and inference cache for the browser-dev workflow.

Mirrors the desktop Tauri model manager (`code/src-tauri/src/models.rs`),
reading from the same OS data directory (`%APPDATA%/digimon-tcg/models` on
Windows) so models staged for the desktop launcher are immediately usable in
the browser-dev UI too.

Engine-only router — no database or auth dependencies.
"""

from __future__ import annotations

import json
import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import onnxruntime as ort
import platformdirs
from fastapi import APIRouter, HTTPException

import digimon_engine

router = APIRouter(tags=["desktop-models"])

# Matches `models_cache_root()` in `code/src-tauri/src/main.rs` (`dirs::data_dir()/digimon-tcg`).
MODELS_APP_NAME = "digimon-tcg"
DEFAULT_TRAINED_PROFILE = "standard_lite_v2"


def _models_dir() -> Path:
    root = Path(
        platformdirs.user_data_dir(MODELS_APP_NAME, appauthor=False, roaming=True)
    )
    return root / "models"


def _sanitize_id(model_id: str) -> str:
    return "".join(
        c if (c.isalnum() or c in "-_") else "_" for c in model_id
    )


def _supported_contracts() -> dict[str, dict[str, Any]]:
    contracts: dict[str, dict[str, Any]] = {}
    for profile_id in ("standard_lite_v2", "standard_lite_deck_v2"):
        try:
            layout = digimon_engine.get_observation_layout(profile_id)
        except Exception:
            continue
        contracts[profile_id] = {
            "observation_profile": layout["profile_id"],
            "tensor_version": int(layout["tensor_version"]),
            "feature_schema_version": str(layout["feature_schema_version"]),
            "tensor_layout_hash": str(layout["layout_hash"]),
            "tensor_size": int(layout["tensor_size"]),
            "action_space_size": int(digimon_engine.ACTION_SPACE_SIZE),
            "action_schema_version": int(digimon_engine.ACTION_SCHEMA_VERSION),
            "engine_commit": None,
        }
    return contracts


def current_engine_contract() -> dict[str, Any]:
    contracts = _supported_contracts()
    if DEFAULT_TRAINED_PROFILE in contracts:
        return contracts[DEFAULT_TRAINED_PROFILE]
    return next(iter(contracts.values()))


def _validate_contract(contract: dict[str, Any]) -> dict[str, Any]:
    supported = _supported_contracts()
    profile = contract.get("observation_profile")
    if not profile or profile not in supported:
        raise ValueError(
            f"Unsupported observation_profile {profile!r}; supported: {sorted(supported)}"
        )
    expected = supported[profile]
    for key in (
        "observation_profile",
        "tensor_version",
        "feature_schema_version",
        "tensor_layout_hash",
        "tensor_size",
        "action_space_size",
        "action_schema_version",
    ):
        if contract.get(key) != expected[key]:
            raise ValueError(
                f"Model contract mismatch on {key}: expected {expected[key]!r}, got {contract.get(key)!r}"
            )
    return expected


@dataclass
class _CachedSession:
    model_id: str
    name: str
    model_type: str  # "mlp" | "lstm"
    hidden_size: int
    observation_profile: str
    tensor_size: int
    action_space_size: int
    session: ort.InferenceSession


_SESSION_CACHE: dict[str, _CachedSession] = {}


class OnnxGamePolicy:
    """Per-game policy instance sharing an ONNX session with isolated LSTM state."""

    def __init__(self, cached: _CachedSession):
        self.cached = cached
        self.observation_profile = cached.observation_profile
        self.name = cached.name
        self.reset()

    def reset(self) -> None:
        if self.cached.model_type == "lstm":
            self.h = np.zeros((1, 1, self.cached.hidden_size), dtype=np.float32)
            self.c = np.zeros((1, 1, self.cached.hidden_size), dtype=np.float32)
        else:
            self.h = None
            self.c = None

    def predict(self, obs: np.ndarray, action_mask: np.ndarray) -> int:
        obs_arr = np.asarray(obs, dtype=np.float32).reshape(1, -1)
        mask_arr = np.asarray(action_mask, dtype=np.float32).reshape(-1)
        if obs_arr.shape[1] != self.cached.tensor_size:
            raise ValueError(
                f"Observation size mismatch: model expects {self.cached.tensor_size}, got {obs_arr.shape[1]}"
            )
        if self.cached.model_type == "lstm":
            logits, h_out, c_out = self.cached.session.run(
                ["logits", "h_out", "c_out"],
                {"obs": obs_arr, "h_in": self.h, "c_in": self.c},
            )
            self.h = h_out
            self.c = c_out
        else:
            (logits,) = self.cached.session.run(["logits"], {"obs": obs_arr})
        scores = np.asarray(logits[0], dtype=np.float32).copy()
        scores[mask_arr < 0.5] = -1e9
        return int(np.argmax(scores))


def _load_session(model_id: str) -> tuple[dict[str, Any], _CachedSession]:
    model_dir = _models_dir() / _sanitize_id(model_id)
    meta_path = model_dir / "meta.json"
    onnx_path = model_dir / "policy.onnx"
    if not meta_path.exists() or not onnx_path.exists():
        raise HTTPException(
            status_code=404, detail=f"Local model '{model_id}' not found"
        )
    try:
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as exc:
        raise HTTPException(status_code=500, detail=f"Failed to read model metadata: {exc}")

    cached = _SESSION_CACHE.get(model_id)
    if cached is not None:
        return meta, cached

    try:
        sess = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])
        raw_contract = sess.get_modelmeta().custom_metadata_map.get("digimon_contract")
        if raw_contract:
            contract = json.loads(raw_contract)
        else:
            contract = meta
        validated = _validate_contract(contract)
    except HTTPException:
        raise
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Incompatible model '{model_id}': {exc}")

    output_names = {o.name for o in sess.get_outputs()}
    is_lstm = {"logits", "h_out", "c_out"} <= output_names
    hidden_size = 0
    if is_lstm:
        h_input = next((i for i in sess.get_inputs() if i.name == "h_in"), None)
        if h_input is not None and len(h_input.shape) >= 3 and isinstance(h_input.shape[-1], int):
            hidden_size = int(h_input.shape[-1])
        else:
            hidden_size = 256

    cached = _CachedSession(
        model_id=model_id,
        name=str(meta.get("name") or model_id),
        model_type="lstm" if is_lstm else "mlp",
        hidden_size=hidden_size,
        observation_profile=validated["observation_profile"],
        tensor_size=validated["tensor_size"],
        action_space_size=validated["action_space_size"],
        session=sess,
    )
    _SESSION_CACHE[model_id] = cached
    return meta, cached


def create_policy_for_model(model_id: str) -> OnnxGamePolicy:
    """Create a fresh per-game ONNX policy for `model_id`."""
    _, cached = _load_session(model_id)
    return OnnxGamePolicy(cached)


@router.get("/desktop-models/contract")
def get_desktop_engine_contract() -> dict[str, Any]:
    """Return the engine training contract used to check local model compatibility."""
    return current_engine_contract()


@router.get("/desktop-models/local")
def list_desktop_local_models() -> list[dict[str, Any]]:
    """List every cached local model under `%APPDATA%/digimon-tcg/models`, newest first."""
    models_dir = _models_dir()
    if not models_dir.exists():
        return []
    out: list[dict[str, Any]] = []
    for entry in models_dir.iterdir():
        if not entry.is_dir():
            continue
        meta_path = entry / "meta.json"
        if not meta_path.exists():
            continue
        try:
            meta = json.loads(meta_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            continue
        out.append(meta)
    out.sort(key=lambda m: int(m.get("downloaded_at") or 0), reverse=True)
    return out


@router.post("/desktop-models/{model_id}/load")
def load_desktop_local_model(model_id: str) -> dict[str, Any]:
    """Validate and preload a cached local ONNX model into memory."""
    meta, _ = _load_session(model_id)
    return meta


@router.delete("/desktop-models/{model_id}")
def delete_desktop_local_model(model_id: str) -> bool:
    """Evict and remove a cached local model."""
    _SESSION_CACHE.pop(model_id, None)
    model_dir = _models_dir() / _sanitize_id(model_id)
    if not model_dir.exists():
        return False
    shutil.rmtree(model_dir)
    return True

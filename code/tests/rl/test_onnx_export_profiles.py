from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

import pytest


REPO_ROOT = Path(__file__).resolve().parents[3]


def _load_tool(module_name: str):
    path = REPO_ROOT / "code" / "tools" / f"{module_name}.py"
    spec = importlib.util.spec_from_file_location(module_name, path)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _profile() -> SimpleNamespace:
    # Synthetic fixture; keep it consistent with the real standard_lite_v2
    # layout (Task S1.4: schema v2.2, tensor_size 8410).
    return SimpleNamespace(
        id="standard_lite_v2",
        tensor_version=2,
        feature_schema_version="standard_lite_v2.2",
        tensor_size=8410,
        layout_hash="sha256:test-layout",
    )


def _deck_profile() -> SimpleNamespace:
    return SimpleNamespace(
        id="standard_lite_deck_v2",
        tensor_version=2,
        feature_schema_version="standard_lite_deck_v2.1",
        tensor_size=8850,
        layout_hash="sha256:test-deck-layout",
    )


def test_export_onnx_parser_accepts_tensor_profile() -> None:
    export_onnx = _load_tool("export_onnx")

    args = export_onnx.build_parser().parse_args(
        [
            "--type",
            "mlp",
            "--input",
            "model.zip",
            "--output",
            "model.onnx",
            "--tensor-profile",
            "standard_lite_v2",
        ]
    )

    assert args.tensor_profile == "standard_lite_v2"


def test_export_onnx_parser_defaults_tensor_profile_to_none() -> None:
    export_onnx = _load_tool("export_onnx")

    args = export_onnx.build_parser().parse_args(
        ["--type", "mlp", "--input", "model.zip", "--output", "model.onnx"]
    )

    assert args.tensor_profile is None


def test_export_random_onnx_parser_accepts_tensor_profile() -> None:
    export_random_onnx = _load_tool("export_random_onnx")

    args = export_random_onnx.build_parser().parse_args(
        [
            "--type",
            "lstm",
            "--output",
            "random.onnx",
            "--tensor-profile",
            "standard_lite_v2",
        ]
    )

    assert args.tensor_profile == "standard_lite_v2"


def test_export_random_onnx_parser_defaults_tensor_profile_to_none() -> None:
    export_random_onnx = _load_tool("export_random_onnx")

    args = export_random_onnx.build_parser().parse_args(
        ["--type", "mlp", "--output", "random.onnx"]
    )

    assert args.tensor_profile is None


def test_export_random_onnx_direct_script_help_runs_from_repo_root() -> None:
    result = subprocess.run(
        [
            sys.executable,
            str(REPO_ROOT / "code" / "tools" / "export_random_onnx.py"),
            "--help",
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr
    assert "--tensor-profile" in result.stdout


def test_export_onnx_writes_profile_metadata(tmp_path) -> None:
    export_onnx = _load_tool("export_onnx")
    output_path = tmp_path / "model.onnx"

    export_onnx.write_export_metadata(output_path, _profile())

    metadata = json.loads((tmp_path / "model.onnx.meta.json").read_text())
    assert metadata == {
        "observation_profile": "standard_lite_v2",
        "tensor_version": 2,
        "feature_schema_version": "standard_lite_v2.2",
        "tensor_size": 8410,
        "tensor_layout_hash": "sha256:test-layout",
        "action_space_size": export_onnx.ACTION_SPACE_SIZE,
        "card_registry_capacity": export_onnx.REGISTRY_CAPACITY,
        "embedding_dim": export_onnx.EMBEDDING_DIM,
    }


def test_export_onnx_checkpoint_observation_profile_match_passes() -> None:
    export_onnx = _load_tool("export_onnx")
    model = SimpleNamespace(observation_space=SimpleNamespace(shape=(8410,)))

    export_onnx.validate_checkpoint_observation_profile(model, _profile())


def test_export_onnx_checkpoint_observation_profile_mismatch_fails() -> None:
    export_onnx = _load_tool("export_onnx")
    model = SimpleNamespace(observation_space=SimpleNamespace(shape=(1375,)))

    with pytest.raises(ValueError) as exc_info:
        export_onnx.validate_checkpoint_observation_profile(model, _profile())

    message = str(exc_info.value)
    assert "SB3 checkpoint observation size 1375" in message
    assert "requested tensor profile 'standard_lite_v2'" in message
    assert "expected tensor size 8410" in message
    assert "Pass the correct --tensor-profile or retrain" in message


def test_export_random_onnx_writes_profile_metadata(tmp_path) -> None:
    export_random_onnx = _load_tool("export_random_onnx")
    output_path = tmp_path / "random.onnx"

    export_random_onnx.write_export_metadata(output_path, _profile())

    metadata = json.loads((tmp_path / "random.onnx.meta.json").read_text())
    assert metadata == {
        "observation_profile": "standard_lite_v2",
        "tensor_version": 2,
        "feature_schema_version": "standard_lite_v2.2",
        "tensor_size": 8410,
        "tensor_layout_hash": "sha256:test-layout",
        "action_space_size": export_random_onnx.ACTION_SPACE_SIZE,
        "card_registry_capacity": export_random_onnx.REGISTRY_CAPACITY,
        "embedding_dim": export_random_onnx.EMBEDDING_DIM,
    }


def test_export_onnx_writes_standard_lite_deck_v2_metadata(tmp_path) -> None:
    export_onnx = _load_tool("export_onnx")
    output_path = tmp_path / "deck-model.onnx"

    export_onnx.write_export_metadata(output_path, _deck_profile())

    metadata = json.loads((tmp_path / "deck-model.onnx.meta.json").read_text())
    assert metadata["observation_profile"] == "standard_lite_deck_v2"
    assert metadata["feature_schema_version"] == "standard_lite_deck_v2.1"
    assert metadata["tensor_size"] == 8850
    assert metadata["tensor_layout_hash"] == "sha256:test-deck-layout"

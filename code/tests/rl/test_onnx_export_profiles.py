from __future__ import annotations

import importlib.util
import json
from pathlib import Path
from types import SimpleNamespace


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
    return SimpleNamespace(
        id="standard_lite_v2",
        tensor_version=2,
        feature_schema_version="standard_lite_v2.1",
        tensor_size=8320,
        layout_hash="sha256:test-layout",
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


def test_export_onnx_writes_profile_metadata(tmp_path) -> None:
    export_onnx = _load_tool("export_onnx")
    output_path = tmp_path / "model.onnx"

    export_onnx.write_export_metadata(output_path, _profile())

    metadata = json.loads((tmp_path / "model.onnx.meta.json").read_text())
    assert metadata == {
        "observation_profile": "standard_lite_v2",
        "tensor_version": 2,
        "feature_schema_version": "standard_lite_v2.1",
        "tensor_size": 8320,
        "tensor_layout_hash": "sha256:test-layout",
        "action_space_size": export_onnx.ACTION_SPACE_SIZE,
        "card_registry_capacity": export_onnx.REGISTRY_CAPACITY,
        "embedding_dim": export_onnx.EMBEDDING_DIM,
    }


def test_export_random_onnx_writes_profile_metadata(tmp_path) -> None:
    export_random_onnx = _load_tool("export_random_onnx")
    output_path = tmp_path / "random.onnx"

    export_random_onnx.write_export_metadata(output_path, _profile())

    metadata = json.loads((tmp_path / "random.onnx.meta.json").read_text())
    assert metadata == {
        "observation_profile": "standard_lite_v2",
        "tensor_version": 2,
        "feature_schema_version": "standard_lite_v2.1",
        "tensor_size": 8320,
        "tensor_layout_hash": "sha256:test-layout",
        "action_space_size": export_random_onnx.ACTION_SPACE_SIZE,
        "card_registry_capacity": export_random_onnx.REGISTRY_CAPACITY,
        "embedding_dim": export_random_onnx.EMBEDDING_DIM,
    }

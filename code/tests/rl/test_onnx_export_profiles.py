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


def _profile(profile_id="standard_lite_v2") -> SimpleNamespace:
    from digimon_gym.tensor_profiles import get_tensor_profile
    return get_tensor_profile(profile_id)


def _deck_profile():
    return _profile("standard_lite_deck_v2")


def _empty_graph(path):
    import onnx
    onnx.save(onnx.helper.make_model(onnx.helper.make_graph([], "metadata_fixture", [], [])), str(path))


def _checkpoint(size):
    from model_contract import current_contract
    contract = current_contract("standard_lite_v2")
    return SimpleNamespace(digimon_contract=contract, observation_space=SimpleNamespace(shape=(size,)), action_space=SimpleNamespace(n=contract["action_space_size"]))


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

    _empty_graph(output_path)
    export_onnx.write_export_metadata(output_path, _profile(), training_contract=_checkpoint(_profile().tensor_size).digimon_contract)

    metadata = json.loads((tmp_path / "model.onnx.meta.json").read_text())
    _assert_embedded_tuple_matches_sidecar(output_path, metadata)
    assert metadata == {
        "observation_profile": "standard_lite_v2",
        "tensor_version": 2,
        "feature_schema_version": _profile().feature_schema_version,
        "tensor_size": _profile().tensor_size,
        "tensor_layout_hash": _profile().layout_hash,
        "action_space_size": export_onnx.ACTION_SPACE_SIZE,
        "action_schema_version": __import__("digimon_engine").ACTION_SCHEMA_VERSION,
        "card_registry_capacity": export_onnx.REGISTRY_CAPACITY,
        "embedding_dim": export_onnx.EMBEDDING_DIM,
    }


def test_export_onnx_checkpoint_observation_profile_match_passes() -> None:
    export_onnx = _load_tool("export_onnx")
    model = _checkpoint(_profile().tensor_size)

    export_onnx.validate_checkpoint_observation_profile(model, _profile())


def test_export_onnx_checkpoint_observation_profile_mismatch_fails() -> None:
    export_onnx = _load_tool("export_onnx")
    model = _checkpoint(1375)

    with pytest.raises(ValueError) as exc_info:
        export_onnx.validate_checkpoint_observation_profile(model, _profile())

    assert "Checkpoint spaces do not match its training contract" in str(exc_info.value)


def test_export_random_onnx_writes_profile_metadata(tmp_path) -> None:
    export_random_onnx = _load_tool("export_random_onnx")
    output_path = tmp_path / "random.onnx"

    _empty_graph(output_path)
    export_random_onnx.write_export_metadata(output_path, _profile())

    metadata = json.loads((tmp_path / "random.onnx.meta.json").read_text())
    _assert_embedded_tuple_matches_sidecar(output_path, metadata)
    assert metadata == {
        "observation_profile": "standard_lite_v2",
        "tensor_version": 2,
        "feature_schema_version": _profile().feature_schema_version,
        "tensor_size": _profile().tensor_size,
        "tensor_layout_hash": _profile().layout_hash,
        "action_space_size": export_random_onnx.ACTION_SPACE_SIZE,
        "action_schema_version": __import__("digimon_engine").ACTION_SCHEMA_VERSION,
        "card_registry_capacity": export_random_onnx.REGISTRY_CAPACITY,
        "embedding_dim": export_random_onnx.EMBEDDING_DIM,
    }


def test_export_onnx_writes_standard_lite_deck_v2_metadata(tmp_path) -> None:
    export_onnx = _load_tool("export_onnx")
    output_path = tmp_path / "deck-model.onnx"

    _empty_graph(output_path)
    export_onnx.write_export_metadata(output_path, _deck_profile(), training_contract=__import__("model_contract").current_contract(_deck_profile().id))

    metadata = json.loads((tmp_path / "deck-model.onnx.meta.json").read_text())
    assert metadata["observation_profile"] == "standard_lite_deck_v2"
    assert metadata["feature_schema_version"] == _deck_profile().feature_schema_version
    assert metadata["tensor_size"] == _deck_profile().tensor_size
    assert metadata["tensor_layout_hash"] == _deck_profile().layout_hash


def test_export_metadata_rejects_old_actions_without_relabeling_weights(tmp_path):
    from model_contract import current_contract
    export = _load_tool("export_onnx")
    artifact = tmp_path / "old.onnx"
    _empty_graph(artifact)
    original = artifact.read_bytes()
    contract = current_contract(_profile().id)
    contract["action_schema_version"] += 1
    with pytest.raises(ValueError):
        export.write_export_metadata(artifact, _profile(), training_contract=contract)
    assert artifact.read_bytes() == original
    assert not Path(str(artifact)+".meta.json").exists()


def _assert_embedded_tuple_matches_sidecar(path, sidecar):
    import onnx
    from model_contract import FIELDS, current_contract
    embedded = json.loads({p.key: p.value for p in onnx.load(str(path)).metadata_props}["digimon_contract"])
    assert embedded == {key: sidecar[key] for key in FIELDS}
    assert embedded == current_contract(sidecar["observation_profile"])


@pytest.mark.parametrize("bad", ["missing", None, True, "mismatch"])
def test_export_action_field_alone_refuses_without_rewriting_artifact(tmp_path, bad):
    from model_contract import current_contract
    export = _load_tool("export_onnx")
    artifact = tmp_path / "legacy.onnx"
    _empty_graph(artifact)
    original = artifact.read_bytes()
    metadata = current_contract(_profile().id)
    if bad == "missing":
        del metadata["action_schema_version"]
    else:
        metadata["action_schema_version"] = metadata["action_schema_version"] + 1 if bad == "mismatch" else bad
    model = SimpleNamespace(digimon_contract=metadata, observation_space=SimpleNamespace(shape=(_profile().tensor_size,)), action_space=SimpleNamespace(n=export.ACTION_SPACE_SIZE))
    with pytest.raises(ValueError, match="action_schema_version"):
        export.validate_checkpoint_observation_profile(model, _profile())
    with pytest.raises(ValueError, match="action_schema_version"):
        export.write_export_metadata(artifact, _profile(), training_contract=metadata)
    assert artifact.read_bytes() == original
    assert not Path(str(artifact) + ".meta.json").exists()
    assert model.digimon_contract == metadata


def test_random_export_real_binding_embedded_contract_round_trip(tmp_path):
    import onnxruntime
    from model_contract import current_contract, validate_session
    export = _load_tool("export_random_onnx")
    path = tmp_path / "fresh-real.onnx"
    export.export_mlp(str(path), _profile())
    metadata = json.loads(Path(str(path) + ".meta.json").read_text())
    _assert_embedded_tuple_matches_sidecar(path, metadata)
    session = onnxruntime.InferenceSession(str(path), providers=["CPUExecutionProvider"])
    assert validate_session(session) == current_contract(_profile().id)


@pytest.mark.parametrize("bad", [None, "missing", True, "mismatch", "valid"])
def test_lstm_export_gates_checkpoint_before_writing_both_contracts(tmp_path, monkeypatch, bad):
    import torch
    import digimon_gym.agents.maskable_recurrent as recurrent
    from model_contract import FIELDS, current_contract
    export = _load_tool("export_onnx")
    profile = _profile()
    contract = current_contract(profile.id)
    if bad == "missing":
        del contract["action_schema_version"]
    elif bad != "valid":
        contract["action_schema_version"] = contract["action_schema_version"] + 1 if bad == "mismatch" else bad
    model = SimpleNamespace(digimon_contract=contract,
        observation_space=SimpleNamespace(shape=(profile.tensor_size,)),
        action_space=SimpleNamespace(n=export.ACTION_SPACE_SIZE),
        policy=SimpleNamespace(eval=lambda: None, lstm_actor=SimpleNamespace(hidden_size=4)))
    monkeypatch.setattr(recurrent.MaskableRecurrentPPO, "load", lambda *a, **k: model)
    class Wrapper(torch.nn.Module):
        def __init__(self, policy):
            super().__init__()
        def forward(self, obs, h, c):
            return torch.zeros((1, export.ACTION_SPACE_SIZE)), h, c
    monkeypatch.setattr(export, "LstmActorWrapper", Wrapper)
    monkeypatch.setattr(export, "LstmValueWrapper", Wrapper)
    monkeypatch.setattr(export, "_verify_lstm", lambda *a: None)
    monkeypatch.setattr(export, "_verify_lstm_value", lambda *a: None)
    written = []
    def graph_export(wrapper, inputs, path, **kwargs):
        written.append(str(path))
        _empty_graph(Path(path))
    monkeypatch.setattr(export.torch.onnx, "export", graph_export)
    actor = tmp_path / "actor.onnx"
    critic = tmp_path / "actor.value.onnx"
    if bad != "valid":
        with pytest.raises(ValueError, match="action_schema_version"):
            export.export_lstm("checkpoint.zip", str(actor), profile)
        assert written == []
        assert list(tmp_path.iterdir()) == []
        assert model.digimon_contract == contract
    else:
        export.export_lstm("checkpoint.zip", str(actor), profile)
        assert written == [str(actor), str(critic)]
        for artifact in (actor, critic):
            metadata = json.loads(Path(str(artifact) + ".meta.json").read_text())
            _assert_embedded_tuple_matches_sidecar(artifact, metadata)
            assert {key: metadata[key] for key in FIELDS} == contract
        assert json.loads(Path(str(actor) + ".meta.json").read_text())["value_head"] == critic.name

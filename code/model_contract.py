"""Strict model compatibility against the live Rust observation registry.

No web/server/training imports: inference, export and hosted consumers share this gate.
"""
from __future__ import annotations
import json
from collections.abc import Mapping
from pathlib import Path

FIELDS = ("observation_profile", "tensor_version", "feature_schema_version", "tensor_size", "tensor_layout_hash", "action_space_size", "action_schema_version")
INTEGER_FIELDS = {"tensor_version", "tensor_size", "action_space_size", "action_schema_version"}
ONNX_KEY = "digimon_contract"

def _typed(metadata):
    if not isinstance(metadata, Mapping):
        raise ValueError("Model is missing its seven-field training contract")
    result = {}
    for key in FIELDS:
        value = metadata.get(key)
        if key in INTEGER_FIELDS:
            if type(value) is not int or value <= 0:
                raise ValueError(f"Model contract {key} must be a positive integer")
        elif type(value) is not str or not value.strip():
            raise ValueError(f"Model contract {key} must be a nonempty string")
        result[key] = value
    return result

def current_contract(profile_id):
    if type(profile_id) is not str or not profile_id:
        raise ValueError("An explicit observation profile is required")
    import digimon_engine
    try:
        layout = digimon_engine.get_observation_layout(profile_id)
    except Exception as exc:
        raise ValueError(f"Unknown or unavailable Rust observation profile {profile_id!r}") from exc
    def field(key):
        return layout[key] if isinstance(layout, Mapping) else getattr(layout, key)
    try:
        action_version = digimon_engine.ACTION_SCHEMA_VERSION
    except AttributeError as exc:
        raise ValueError("Rust extension lacks ACTION_SCHEMA_VERSION; rebuild the matching engine binding") from exc
    expected = _typed(dict(observation_profile=(field("profile_id") if isinstance(layout, Mapping) else field("id")), tensor_version=field("tensor_version"), feature_schema_version=field("feature_schema_version"), tensor_size=field("tensor_size"), tensor_layout_hash=field("layout_hash"), action_space_size=digimon_engine.ACTION_SPACE_SIZE, action_schema_version=action_version))
    if expected["observation_profile"] != profile_id:
        raise ValueError("Model profile must use the canonical Rust profile ID")
    return expected

def validate_contract(metadata, expected_profile=None):
    contract = _typed(metadata)
    expected = current_contract(expected_profile or contract["observation_profile"])
    mismatch = [key for key in FIELDS if contract[key] != expected[key]]
    if mismatch:
        raise ValueError("Incompatible model contract: " + ", ".join(mismatch))
    return contract

def contract_from_row(row):
    return {key: getattr(row, key, None) for key in FIELDS}

def validate_checkpoint(model, expected_profile=None):
    contract = validate_contract(getattr(model, "digimon_contract", None), expected_profile)
    shape = getattr(getattr(model, "observation_space", None), "shape", None)
    actions = getattr(getattr(model, "action_space", None), "n", None)
    if shape is None or tuple(shape) != (contract["tensor_size"],) or actions != contract["action_space_size"]:
        raise ValueError("Checkpoint spaces do not match its training contract")
    return contract

def validate_session(session, expected=None):
    try:
        raw = session.get_modelmeta().custom_metadata_map[ONNX_KEY]
        contract = validate_contract(json.loads(raw))
    except (AttributeError, KeyError, TypeError, json.JSONDecodeError) as exc:
        raise ValueError("ONNX is missing valid embedded training contract metadata") from exc
    if expected is not None and contract != validate_contract(expected):
        raise ValueError("ONNX contract differs from its confirmed manifest record")
    obs = next((v for v in session.get_inputs() if v.name == "obs"), None)
    logits = next((v for v in session.get_outputs() if v.name == "logits"), None)
    for value, key, label in [(obs, "tensor_size", "obs input"), (logits, "action_space_size", "logits output")]:
        if (value is None or not value.shape or len(value.shape) != 2
                or getattr(value, "type", None) != "tensor(float)"
                or type(value.shape[-1]) is not int or value.shape[-1] != contract[key]):
            raise ValueError(f"ONNX {label} must be rank-2 float32 with {key} matching its embedded training contract")
    return contract

def validate_onnx_file(path, expected=None):
    import onnxruntime
    return validate_session(onnxruntime.InferenceSession(str(path), providers=["CPUExecutionProvider"]), expected)

def embed_onnx_contract(path, metadata):
    contract = validate_contract(metadata)
    import onnx
    model = onnx.load(str(path))
    others = [(p.key, p.value) for p in model.metadata_props if p.key != ONNX_KEY]
    del model.metadata_props[:]
    for key, value in others + [(ONNX_KEY, json.dumps(contract, sort_keys=True))]:
        entry = model.metadata_props.add(); entry.key = key; entry.value = value
    onnx.save(model, str(path))
    return contract

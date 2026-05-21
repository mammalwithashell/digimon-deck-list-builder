"""ONNX fixture builder for admin_models tests.

Builds three minimal ONNX models using raw protobuf serialization (no torch/onnx
package required — only onnxruntime is needed to load the produced files).  Models
are cached on disk in this directory and built once per test session.

ONNX protobuf field numbers referenced below:
  ModelProto:    ir_version=1, opset_import=8, graph=7
  GraphProto:    node=1, name=2, initializer=5, input=11, output=12
  NodeProto:     input=1, output=2, name=3, op_type=4
  TensorProto:   dims=1, data_type=2, raw_data=9, name=8  (data_type 1 = FLOAT)
  ValueInfoProto: name=1, type=2
  TypeProto:     tensor_type.field=1
  TypeProto.Tensor: elem_type=1, shape=2
  TensorShapeProto: dim=1
  TensorShapeProto.Dimension: dim_value=1, dim_param=2
  OperatorSetIdProto: domain=1, version=2
"""

from __future__ import annotations

from pathlib import Path

import pytest

FIXTURES_DIR = Path(__file__).parent
VALID_MLP = FIXTURES_DIR / "valid_mlp.onnx"
VALID_LSTM = FIXTURES_DIR / "valid_lstm.onnx"
INVALID_NO_OBS = FIXTURES_DIR / "invalid_no_obs.onnx"


# ---------------------------------------------------------------------------
# Minimal protobuf serialisation helpers
# ---------------------------------------------------------------------------

def _varint(v: int) -> bytes:
    """Encode a non-negative integer as a protobuf varint."""
    out = []
    while True:
        b = v & 0x7F
        v >>= 7
        if v:
            out.append(b | 0x80)
        else:
            out.append(b)
            break
    return bytes(out)


def _field_varint(num: int, value: int) -> bytes:
    tag = (num << 3) | 0  # wire type 0 = varint
    return _varint(tag) + _varint(value)


def _field_bytes(num: int, data: bytes) -> bytes:
    tag = (num << 3) | 2  # wire type 2 = length-delimited
    return _varint(tag) + _varint(len(data)) + data


def _field_str(num: int, s: str) -> bytes:
    return _field_bytes(num, s.encode())


# ---------------------------------------------------------------------------
# ONNX building blocks
# ---------------------------------------------------------------------------

def _dim(value_or_param) -> bytes:
    """Build a TensorShapeProto.Dimension."""
    if isinstance(value_or_param, str):
        return _field_str(2, value_or_param)   # dim_param
    return _field_varint(1, value_or_param)     # dim_value


def _value_info(name: str, elem_type: int, dims) -> bytes:
    """Build a ValueInfoProto for a float tensor."""
    shape_bytes = b"".join(_field_bytes(1, _dim(d)) for d in dims)
    tensor_type = _field_varint(1, elem_type) + _field_bytes(2, shape_bytes)
    type_proto = _field_bytes(1, tensor_type)
    return _field_str(1, name) + _field_bytes(2, type_proto)


def _tensor_initializer(name: str, dims: tuple[int, ...], raw_data: bytes) -> bytes:
    """Build a TensorProto initializer (FLOAT data_type=1, raw_data)."""
    dims_bytes = b"".join(_field_varint(1, d) for d in dims)
    data_type = _field_varint(2, 1)   # FLOAT
    name_bytes = _field_str(8, name)
    raw = _field_bytes(9, raw_data)
    return dims_bytes + data_type + name_bytes + raw


def _node(op_type: str, inputs: list[str], outputs: list[str], name: str) -> bytes:
    """Build a NodeProto."""
    inp_bytes = b"".join(_field_str(1, i) for i in inputs)
    out_bytes = b"".join(_field_str(2, o) for o in outputs)
    return inp_bytes + out_bytes + _field_str(3, name) + _field_str(4, op_type)


def _graph(
    name: str,
    nodes: list[bytes],
    initializers: list[bytes],
    inputs: list[bytes],
    outputs: list[bytes],
) -> bytes:
    node_bytes = b"".join(_field_bytes(1, n) for n in nodes)
    init_bytes = b"".join(_field_bytes(5, i) for i in initializers)
    in_bytes   = b"".join(_field_bytes(11, i) for i in inputs)
    out_bytes  = b"".join(_field_bytes(12, o) for o in outputs)
    return node_bytes + _field_str(2, name) + init_bytes + in_bytes + out_bytes


def _model(graph_bytes: bytes, opset_version: int = 11) -> bytes:
    ir_version = _field_varint(1, 7)
    opset = _field_str(1, "") + _field_varint(2, opset_version)
    opset_field = _field_bytes(8, opset)
    graph_field = _field_bytes(7, graph_bytes)
    return ir_version + opset_field + graph_field


# ---------------------------------------------------------------------------
# Model builders
# ---------------------------------------------------------------------------

ELEM_FLOAT = 1

# Use a factored (rank-1 bottleneck) approach so models stay under 1MB.
# moto 4.x on Windows corrupts objects > 1MB via chunked HTTP encoding.
# W = w1 @ w2 where w1:(1375,1) and w2:(1,2192) are each only a few KB.
_ZEROS_1375x1 = bytes(1375 * 4)     # 5.5 KB
_ZEROS_1x2192 = bytes(2192 * 4)     # 8.6 KB


def _build_valid_mlp(path: Path) -> None:
    """MLP: obs[None,1375] → hidden[None,1] → logits[None,2192].

    Uses a rank-1 bottleneck so the total model size stays well under 1 MB,
    avoiding a moto chunked-encoding corruption bug on Windows.
    """
    w1 = _tensor_initializer("w1_mlp", (1375, 1), _ZEROS_1375x1)
    w2 = _tensor_initializer("w2_mlp", (1, 2192), _ZEROS_1x2192)
    n1 = _node("MatMul", ["obs", "w1_mlp"], ["hidden_mlp"], "mm1_mlp")
    n2 = _node("MatMul", ["hidden_mlp", "w2_mlp"], ["logits"], "mm2_mlp")
    obs_vi    = _value_info("obs",    ELEM_FLOAT, ["batch", 1375])
    logits_vi = _value_info("logits", ELEM_FLOAT, ["batch", 2192])
    graph = _graph("mlp_graph", [n1, n2], [w1, w2], [obs_vi], [logits_vi])
    path.write_bytes(_model(graph))


def _build_valid_lstm(path: Path) -> None:
    """LSTM-shaped model: obs,h_in,c_in → logits,h_out,c_out.

    Confirms that the endpoint accepts models with extra inputs/outputs beyond
    obs/logits (the LSTM hidden/cell state pass-throughs).
    """
    w1 = _tensor_initializer("w1_lstm", (1375, 1), _ZEROS_1375x1)
    w2 = _tensor_initializer("w2_lstm", (1, 2192), _ZEROS_1x2192)
    n1 = _node("MatMul", ["obs", "w1_lstm"], ["hidden_lstm"], "mm1_lstm")
    n2 = _node("MatMul", ["hidden_lstm", "w2_lstm"], ["logits"], "mm2_lstm")
    h_node = _node("Identity", ["h_in"], ["h_out"], "h_pass")
    c_node = _node("Identity", ["c_in"], ["c_out"], "c_pass")

    obs_vi    = _value_info("obs",    ELEM_FLOAT, ["batch", 1375])
    h_in_vi   = _value_info("h_in",  ELEM_FLOAT, [1, 1, 64])
    c_in_vi   = _value_info("c_in",  ELEM_FLOAT, [1, 1, 64])
    logits_vi = _value_info("logits", ELEM_FLOAT, ["batch", 2192])
    h_out_vi  = _value_info("h_out", ELEM_FLOAT, [1, 1, 64])
    c_out_vi  = _value_info("c_out", ELEM_FLOAT, [1, 1, 64])

    graph = _graph(
        "lstm_graph",
        nodes=[n1, n2, h_node, c_node],
        initializers=[w1, w2],
        inputs=[obs_vi, h_in_vi, c_in_vi],
        outputs=[logits_vi, h_out_vi, c_out_vi],
    )
    path.write_bytes(_model(graph))


def _build_invalid_no_obs(path: Path) -> None:
    """Bad model: input named 'x' (not 'obs') → logits.  Confirm should reject."""
    w1 = _tensor_initializer("w1_bad", (1375, 1), _ZEROS_1375x1)
    w2 = _tensor_initializer("w2_bad", (1, 2192), _ZEROS_1x2192)
    n1 = _node("MatMul", ["x", "w1_bad"], ["hidden_bad"], "mm1_bad")
    n2 = _node("MatMul", ["hidden_bad", "w2_bad"], ["logits"], "mm2_bad")
    x_vi      = _value_info("x",      ELEM_FLOAT, ["batch", 1375])
    logits_vi = _value_info("logits", ELEM_FLOAT, ["batch", 2192])
    graph = _graph("bad_graph", [n1, n2], [w1, w2], [x_vi], [logits_vi])
    path.write_bytes(_model(graph))


# ---------------------------------------------------------------------------
# Session-scoped pytest fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(scope="session")
def valid_mlp_path() -> Path:
    if not VALID_MLP.exists():
        _build_valid_mlp(VALID_MLP)
    return VALID_MLP


@pytest.fixture(scope="session")
def valid_lstm_path() -> Path:
    if not VALID_LSTM.exists():
        _build_valid_lstm(VALID_LSTM)
    return VALID_LSTM


@pytest.fixture(scope="session")
def invalid_no_obs_path() -> Path:
    if not INVALID_NO_OBS.exists():
        _build_invalid_no_obs(INVALID_NO_OBS)
    return INVALID_NO_OBS

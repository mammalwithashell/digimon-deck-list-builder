"""Reject non-executable ONNX I/O despite otherwise authentic metadata."""
import json
from types import SimpleNamespace
import pytest
from model_contract import current_contract, validate_session

class Session:
    def __init__(self, contract, which=None, bad=None):
        self.contract = contract
        self.values = {
            "obs": SimpleNamespace(name="obs", shape=[None, contract["tensor_size"]], type="tensor(float)"),
            "logits": SimpleNamespace(name="logits", shape=[None, contract["action_space_size"]], type="tensor(float)"),
        }
        if which:
            v = self.values[which]
            if bad == "rank1": v.shape = [v.shape[-1]]
            elif bad == "rank3": v.shape = [None, 1, v.shape[-1]]
            elif bad == "unknown_width": v.shape[-1] = "width"
            elif bad == "int64": v.type = "tensor(int64)"
            elif bad == "float64": v.type = "tensor(double)"
            elif bad == "missing_type": del v.type
    def get_modelmeta(self):
        return SimpleNamespace(custom_metadata_map={"digimon_contract": json.dumps(self.contract)})
    def get_inputs(self): return [self.values["obs"]]
    def get_outputs(self): return [self.values["logits"]]

@pytest.mark.parametrize("which", ["obs", "logits"])
@pytest.mark.parametrize("bad", ["rank1", "rank3", "unknown_width", "int64", "float64", "missing_type"])
def test_authentic_metadata_does_not_override_actual_tensor_contract(which, bad):
    c = current_contract("standard_lite_v2")
    with pytest.raises(ValueError):
        validate_session(Session(c, which, bad))

def test_dynamic_batch_rank2_float32_is_accepted():
    c = current_contract("standard_lite_v2")
    assert validate_session(Session(c)) == c

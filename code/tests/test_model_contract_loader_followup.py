import json
from types import SimpleNamespace
from unittest.mock import AsyncMock
import pytest

CONTRACT=dict(observation_profile="standard_lite_v2",tensor_version=2,feature_schema_version="standard_lite_v2.current",tensor_size=8410,tensor_layout_hash="test-live-layout",action_space_size=2192,action_schema_version=1)
@pytest.fixture(autouse=True)
def registry(monkeypatch):
    import digimon_engine
    def layout(profile):
        if profile!="standard_lite_v2":raise ValueError("unknown profile")
        return SimpleNamespace(id=profile,tensor_version=2,feature_schema_version=CONTRACT["feature_schema_version"],tensor_size=8410,layout_hash=CONTRACT["tensor_layout_hash"])
    monkeypatch.setattr(digimon_engine,"get_observation_layout",layout)
    monkeypatch.setattr(digimon_engine,"ACTION_SPACE_SIZE",2192)
    monkeypatch.setattr(digimon_engine,"ACTION_SCHEMA_VERSION",1)
class Session:
    def __init__(self,metadata):self.metadata=metadata;self.runs=0
    def get_modelmeta(self):return SimpleNamespace(custom_metadata_map={} if self.metadata is None else {"digimon_contract":json.dumps(self.metadata)})
    def get_inputs(self):return [SimpleNamespace(name="obs",shape=[None,8410],type="tensor(float)")]
    def get_outputs(self):return [SimpleNamespace(name="logits",shape=[None,2192],type="tensor(float)")]
    def run(self,*args):self.runs+=1;raise AssertionError("invalid model executed")

@pytest.mark.parametrize("metadata",[None,{**CONTRACT,"feature_schema_version":"old"}])
def test_cached_training_opponent_rejects_stale_contract(tmp_path,monkeypatch,metadata):
    from digimon_gym.agents import pilot_training as pt
    import onnxruntime
    path=tmp_path/"pilot.zip"
    (tmp_path/"pilot.opponent.onnx").write_bytes(b"cached")
    session=Session(metadata)
    monkeypatch.setattr(onnxruntime,"InferenceSession",lambda *a,**k:session)
    with pytest.raises(ValueError):pt._make_onnx_mlp_opponent(str(path))
    assert session.runs==0

@pytest.mark.parametrize("metadata",[None,{**CONTRACT,"tensor_layout_hash":"old"}])
def test_architect_sb3_loader_rejects_stale_contract(monkeypatch,metadata):
    from digimon_gym.agents.architect_simulator import _SB3Policy
    import sb3_contrib
    model=SimpleNamespace(digimon_contract=metadata,observation_space=SimpleNamespace(shape=(8410,)),action_space=SimpleNamespace(n=2192))
    monkeypatch.setattr(sb3_contrib.MaskablePPO,"load",lambda *a,**k:model)
    with pytest.raises(ValueError):_SB3Policy("old.zip")

def test_cached_training_opponent_accepts_live_contract(tmp_path,monkeypatch):
    from digimon_gym.agents import pilot_training as pt
    import onnxruntime
    (tmp_path/"pilot.opponent.onnx").write_bytes(b"cached")
    monkeypatch.setattr(onnxruntime,"InferenceSession",lambda *a,**k:Session(CONTRACT))
    assert callable(pt._make_onnx_mlp_opponent(str(tmp_path/"pilot.zip")))

def test_opponent_export_rejects_stale_checkpoint_before_policy_access(tmp_path):
    from digimon_gym.agents.pilot_training import _export_mlp_policy_onnx
    model=SimpleNamespace(digimon_contract={**CONTRACT,"tensor_layout_hash":"old"},observation_space=SimpleNamespace(shape=(8410,)),action_space=SimpleNamespace(n=2192))
    with pytest.raises(ValueError):_export_mlp_policy_onnx(model,str(tmp_path/"out.onnx"))
    assert not (tmp_path/"out.onnx").exists()

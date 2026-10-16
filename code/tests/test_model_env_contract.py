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

@pytest.mark.parametrize("kind",["mlp","lstm"])
def test_torch_opponent_rejects_different_env_profile_before_predict(monkeypatch,kind):
    from digimon_gym.agents import pilot_training as pt
    import numpy as np
    from unittest.mock import Mock
    model=SimpleNamespace(digimon_contract=CONTRACT,observation_space=SimpleNamespace(shape=(8410,)),action_space=SimpleNamespace(n=2192),predict=Mock(side_effect=AssertionError("mismatch inferred")))
    cls=pt.MaskablePPO if kind=="mlp" else pt.MaskableRecurrentPPO
    monkeypatch.setattr(cls,"load",lambda *a,**k:model)
    monkeypatch.setattr(pt,"_onnx_opponent_enabled",lambda:False)
    policy=pt.make_agent_opponent_fn("model.zip",algorithm=kind)
    env=SimpleNamespace(tensor_profile="different-profile",action_mask=lambda:np.ones(2192),runner=SimpleNamespace(get_board_tensor=lambda _:np.zeros(8410)))
    with pytest.raises(ValueError):policy(env)
    model.predict.assert_not_called()


def test_architect_rejects_mixed_profiles_before_matchup():
    from digimon_gym.agents.architect_simulator import _matchup_observation_profile
    with pytest.raises(ValueError):
        _matchup_observation_profile(SimpleNamespace(_contract=CONTRACT),SimpleNamespace(_contract={**CONTRACT,"observation_profile":"standard_full_v2"}))

def test_architect_selects_learned_profile_for_actual_runner():
    from digimon_gym.agents.architect_simulator import _matchup_observation_profile
    assert _matchup_observation_profile(SimpleNamespace(_contract=CONTRACT),SimpleNamespace()) == CONTRACT["observation_profile"]

def test_onnx_opponent_rejects_different_env_profile_before_run(tmp_path,monkeypatch):
    from digimon_gym.agents import pilot_training as pt
    import onnxruntime
    import numpy as np
    (tmp_path/"pilot.opponent.onnx").write_bytes(b"cached")
    session=Session(CONTRACT)
    monkeypatch.setattr(onnxruntime,"InferenceSession",lambda *a,**k:session)
    policy=pt._make_onnx_mlp_opponent(str(tmp_path/"pilot.zip"))
    env=SimpleNamespace(tensor_profile="different-profile",action_mask=lambda:np.ones(2192),runner=SimpleNamespace(get_board_tensor=lambda _:np.zeros(8410)))
    with pytest.raises(ValueError):policy(env)
    assert session.runs==0

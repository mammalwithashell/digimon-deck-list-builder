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
@pytest.mark.parametrize("kind",["missing","old_schema","hash","profile","bool_size"])
def test_inference_rejects_invalid_contract_before_predict(tmp_path,kind):
    from digimon_gym.inference.onnx_policy import OnnxMlpPolicy
    path=tmp_path/"model.onnx";path.write_bytes(b"fixture")
    meta=dict(CONTRACT)
    if kind=="missing":meta=None
    elif kind=="old_schema":meta["action_schema_version"]=CONTRACT["action_schema_version"]+1
    elif kind=="hash":meta["tensor_layout_hash"]="old"
    elif kind=="profile":meta["observation_profile"]="unknown"
    else:meta["tensor_size"]=True
    sess=Session(meta)
    with pytest.raises(ValueError):OnnxMlpPolicy(str(path),session=sess)
    assert sess.runs==0

def test_inference_accepts_exact_live_contract(tmp_path):
    from digimon_gym.inference.onnx_policy import OnnxMlpPolicy
    path=tmp_path/"model.onnx";path.write_bytes(b"fixture")
    OnnxMlpPolicy(str(path),session=Session(CONTRACT))
@pytest.mark.parametrize("metadata",[None,{**CONTRACT,"feature_schema_version":"old"}])
def test_export_refuses_same_shape_checkpoint_without_matching_training_contract(metadata):
    from tools.export_onnx import validate_checkpoint_observation_profile
    model=SimpleNamespace(observation_space=SimpleNamespace(shape=(8410,)),action_space=SimpleNamespace(n=2192))
    if metadata is not None:model.digimon_contract=metadata
    layout=SimpleNamespace(id=CONTRACT["observation_profile"],tensor_size=8410,tensor_version=2,feature_schema_version=CONTRACT["feature_schema_version"],layout_hash=CONTRACT["tensor_layout_hash"])
    with pytest.raises(ValueError):validate_checkpoint_observation_profile(model,layout)

@pytest.mark.asyncio
async def test_hosted_cached_legacy_model_is_rejected_before_download(tmp_path,monkeypatch):
    from server.storage import model_resolver
    row=SimpleNamespace(id="legacy",published=True,state="uploaded",file_sha256="a"*64,spaces_key="models/legacy",tensor_size=8410,action_space_size=2192,action_schema_version=1)
    db=SimpleNamespace(execute=AsyncMock(return_value=SimpleNamespace(scalar_one_or_none=lambda:row)))
    monkeypatch.setattr(model_resolver,"_manifest_cache_dir",lambda:tmp_path)
    (tmp_path/(row.file_sha256+".onnx")).write_bytes(b"old model")
    with pytest.raises(ValueError):await model_resolver.resolve_manifest_model_path(db,"legacy")
@pytest.mark.asyncio
async def test_hosted_create_rejects_stale_contract_before_upload_slot(monkeypatch):
    from fastapi import HTTPException
    from unittest.mock import MagicMock
    from server.db.routers import admin_models
    from server.db.schemas import AIModelCreateRequest
    stale={**CONTRACT,"tensor_layout_hash":"old"}
    request=AIModelCreateRequest(name="stale",model_type="mlp",**stale)
    db=SimpleNamespace(add=MagicMock(),commit=AsyncMock(),refresh=AsyncMock())
    monkeypatch.setattr(admin_models.spaces,"generate_presigned_put",lambda *args:"upload-url")
    with pytest.raises(HTTPException) as exc:await admin_models.create_model(request,None,db)
    assert exc.value.status_code==422
    db.add.assert_not_called()

@pytest.mark.asyncio
async def test_publication_rejects_legacy_row_before_commit():
    from fastapi import HTTPException
    from server.db.routers import admin_models
    from server.db.schemas import AIModelUpdateRequest
    row=SimpleNamespace(state="uploaded",published=False)
    db=SimpleNamespace(execute=AsyncMock(return_value=SimpleNamespace(scalar_one_or_none=lambda:row)),commit=AsyncMock(side_effect=AssertionError("invalid publication reached commit")))
    with pytest.raises(HTTPException) as exc:await admin_models.update_model("legacy",AIModelUpdateRequest(published=True),None,db)
    assert exc.value.status_code==422
    db.commit.assert_not_called()

@pytest.mark.asyncio
async def test_confirm_rejects_artifact_without_embedded_contract(monkeypatch):
    from fastapi import HTTPException
    from server.db.routers import admin_models
    import sys
    row=SimpleNamespace(id="model",state="pending",spaces_key="test",**CONTRACT)
    db=SimpleNamespace(execute=AsyncMock(return_value=SimpleNamespace(scalar_one_or_none=lambda:row)),commit=AsyncMock(),refresh=AsyncMock())
    monkeypatch.setattr(admin_models.spaces,"head_object",lambda *args:None)
    monkeypatch.setattr(admin_models.spaces,"download_and_hash",lambda *args:("a"*64,20))
    monkeypatch.setattr(admin_models.spaces,"delete_object",lambda *args:None)
    monkeypatch.setitem(sys.modules,"onnxruntime",SimpleNamespace(InferenceSession=lambda *args,**kwargs:Session(None)))
    with pytest.raises(HTTPException) as exc:await admin_models.confirm_model("model",None,db)
    assert exc.value.status_code==422
    assert row.state=="failed"


def test_same_shape_old_action_contract_rejected_before_inference(tmp_path):
    from digimon_gym.inference.onnx_policy import OnnxMlpPolicy
    path=tmp_path/"old.onnx";path.write_bytes(b"fixture")
    session=Session({**CONTRACT,"action_schema_version":2})
    with pytest.raises(ValueError):OnnxMlpPolicy(str(path),session=session)
    assert session.runs==0

@pytest.mark.asyncio
async def test_same_shape_old_actions_rejected_before_hosted_upload(monkeypatch):
    from fastapi import HTTPException
    from unittest.mock import MagicMock
    from server.db.routers import admin_models
    from server.db.schemas import AIModelCreateRequest
    request=AIModelCreateRequest(name="old",model_type="mlp",**{**CONTRACT,"action_schema_version":2})
    db=SimpleNamespace(add=MagicMock(),commit=AsyncMock(),refresh=AsyncMock())
    upload=MagicMock(side_effect=AssertionError("stale artifact reached upload"))
    monkeypatch.setattr(admin_models.spaces,"generate_presigned_put",upload)
    with pytest.raises(HTTPException):await admin_models.create_model(request,None,db)
    upload.assert_not_called()


ACTION_DEFECTS = ["missing", None, True, "mismatch"]

def action_defect(kind):
    data = dict(CONTRACT)
    if kind == "missing":
        del data["action_schema_version"]
    else:
        data["action_schema_version"] = CONTRACT["action_schema_version"] + 1 if kind == "mismatch" else kind
    return data

@pytest.mark.parametrize("kind", ACTION_DEFECTS)
def test_inference_rejects_action_field_alone_before_execution(tmp_path, kind):
    from digimon_gym.inference.onnx_policy import OnnxMlpPolicy
    path = tmp_path / "model.onnx"; path.write_bytes(b"fixture")
    session = Session(action_defect(kind))
    with pytest.raises(ValueError, match="action_schema_version"):
        OnnxMlpPolicy(str(path), session=session)
    assert session.runs == 0

@pytest.mark.parametrize("kind", ACTION_DEFECTS)
@pytest.mark.asyncio
async def test_create_rejects_action_field_alone_before_upload(kind, monkeypatch):
    from pydantic import ValidationError
    from fastapi import HTTPException
    from unittest.mock import MagicMock
    from server.db.routers import admin_models
    from server.db.schemas import AIModelCreateRequest
    data = dict(name="artifact", model_type="mlp", **action_defect(kind))
    if kind != "mismatch":
        with pytest.raises(ValidationError) as exc:
            AIModelCreateRequest(**data)
        assert any(e["loc"] == ("action_schema_version",) for e in exc.value.errors())
        return
    request = AIModelCreateRequest(**data)
    upload = MagicMock(side_effect=AssertionError("invalid metadata obtained upload slot"))
    monkeypatch.setattr(admin_models.spaces, "generate_presigned_put", upload)
    db = SimpleNamespace(add=MagicMock(), commit=AsyncMock(), refresh=AsyncMock())
    with pytest.raises(HTTPException) as exc:
        await admin_models.create_model(request, None, db)
    assert exc.value.status_code == 422
    assert "action_schema_version" in str(exc.value.detail)
    upload.assert_not_called(); db.add.assert_not_called()

@pytest.mark.parametrize("kind", ACTION_DEFECTS)
@pytest.mark.asyncio
async def test_publish_rejects_action_field_alone(kind):
    from fastapi import HTTPException
    from server.db.routers import admin_models
    from server.db.schemas import AIModelUpdateRequest
    row = SimpleNamespace(state="uploaded", published=False, **action_defect(kind))
    db = SimpleNamespace(execute=AsyncMock(return_value=SimpleNamespace(scalar_one_or_none=lambda: row)), commit=AsyncMock())
    with pytest.raises(HTTPException) as exc:
        await admin_models.update_model("legacy", AIModelUpdateRequest(published=True), None, db)
    assert exc.value.status_code == 422
    assert "action_schema_version" in str(exc.value.detail)
    db.commit.assert_not_called()

@pytest.mark.parametrize("kind", ACTION_DEFECTS)
@pytest.mark.asyncio
async def test_confirm_rejects_action_field_alone_before_artifact_download(kind, monkeypatch):
    from fastapi import HTTPException
    from unittest.mock import MagicMock
    from server.db.routers import admin_models
    row = SimpleNamespace(id="model", state="pending", spaces_key="test", **action_defect(kind))
    db = SimpleNamespace(execute=AsyncMock(return_value=SimpleNamespace(scalar_one_or_none=lambda: row)), commit=AsyncMock(), refresh=AsyncMock())
    download = MagicMock(side_effect=AssertionError("invalid declaration downloaded artifact"))
    monkeypatch.setattr(admin_models.spaces, "download_and_hash", download)
    with pytest.raises(HTTPException) as exc:
        await admin_models.confirm_model("model", None, db)
    assert exc.value.status_code == 422
    assert "action_schema_version" in str(exc.value.detail)
    download.assert_not_called()

@pytest.mark.parametrize("kind", ACTION_DEFECTS)
@pytest.mark.asyncio
async def test_resolver_rejects_action_field_alone_even_cached(kind, tmp_path, monkeypatch):
    from server.storage import model_resolver
    row = SimpleNamespace(id="legacy", published=True, state="uploaded", file_sha256="a"*64, spaces_key="models/legacy", **action_defect(kind))
    db = SimpleNamespace(execute=AsyncMock(return_value=SimpleNamespace(scalar_one_or_none=lambda: row)))
    monkeypatch.setattr(model_resolver, "_manifest_cache_dir", lambda: tmp_path)
    (tmp_path / (row.file_sha256 + ".onnx")).write_bytes(b"cached legacy")
    with pytest.raises(ValueError, match="action_schema_version"):
        await model_resolver.resolve_manifest_model_path(db, "legacy")

@pytest.mark.parametrize("kind", ACTION_DEFECTS)
@pytest.mark.asyncio
async def test_manifest_filters_only_action_defect_and_retains_compatible_row(kind, monkeypatch):
    from datetime import datetime, timezone
    from fastapi import Response
    from server.db.routers import admin_models
    fields = dict(name="policy", model_type="mlp", deck_id=None, starter_deck=None, engine_commit="commit", trained_at=datetime.now(timezone.utc), file_sha256="a"*64, file_size_bytes=20, spaces_key="model", notes=None, published=True, state="uploaded")
    bad = SimpleNamespace(id="old", **fields, **action_defect(kind))
    good = SimpleNamespace(id="current", **fields, **CONTRACT)
    db = SimpleNamespace(execute=AsyncMock(return_value=SimpleNamespace(scalars=lambda: SimpleNamespace(all=lambda: [bad, good]))))
    monkeypatch.setattr(admin_models.spaces, "public_url", lambda key: "https://example.invalid/" + key)
    result = await admin_models.get_manifest(Response(), db)
    assert [row.id for row in result.models] == ["current"]
    assert result.models[0].action_schema_version == CONTRACT["action_schema_version"]


@pytest.mark.parametrize("kind", ACTION_DEFECTS)
@pytest.mark.asyncio
async def test_confirm_rejects_only_embedded_action_defect(kind, monkeypatch):
    import sys
    from fastapi import HTTPException
    from server.db.routers import admin_models
    row = SimpleNamespace(id="model", state="pending", spaces_key="test", **CONTRACT)
    db = SimpleNamespace(execute=AsyncMock(return_value=SimpleNamespace(scalar_one_or_none=lambda: row)), commit=AsyncMock(), refresh=AsyncMock())
    monkeypatch.setattr(admin_models.spaces, "head_object", lambda *args: None)
    monkeypatch.setattr(admin_models.spaces, "download_and_hash", lambda *args: ("a"*64, 20))
    monkeypatch.setattr(admin_models.spaces, "delete_object", lambda *args: None)
    session = Session(action_defect(kind))
    monkeypatch.setitem(sys.modules, "onnxruntime", SimpleNamespace(InferenceSession=lambda *args, **kwargs: session))
    with pytest.raises(HTTPException) as exc:
        await admin_models.confirm_model("model", None, db)
    assert exc.value.status_code == 422
    assert "action_schema_version" in str(exc.value.detail)
    assert row.state == "failed"
    assert session.runs == 0

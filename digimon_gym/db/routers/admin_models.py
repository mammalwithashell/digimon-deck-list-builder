"""Admin-only endpoints for uploading + managing ONNX models.

The read path lives in `digimon_gym/routers/models.py`. These write
endpoints are gated by `ROLE_ADMIN` — the alpha roster is curated, so
only designated operators can push new models.

Upload flow:
1. Stream the multipart body to a temp file while hashing (we never hold
   the full blob in memory — models are 100-300 MB).
2. Optionally parse the ONNX graph to capture input shapes; skipped if
   the `onnx` Python package isn't installed.
3. Hand the temp file off to `ModelStorage.put` (filesystem copy for
   local, multipart S3 upload for Spaces).
4. Insert a `ModelVersion` row.
"""

from __future__ import annotations

import hashlib
import json
import logging
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

from fastapi import (
    APIRouter,
    Body,
    Depends,
    File,
    Form,
    HTTPException,
    Request,
    UploadFile,
    status,
)
from pydantic import ValidationError
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import selectinload

from digimon_gym.db.auth import ROLE_ADMIN, require_roles
from digimon_gym.db.database import get_db
from digimon_gym.db.models import ModelRecord, ModelVersion, User
from digimon_gym.db.schemas import (
    ModelRecordCreate,
    ModelRecordPublic,
    ModelVersionPublic,
    ModelVersionUploadMeta,
)
from digimon_gym.models_store import get_model_storage
from digimon_gym.routers.models import _record_to_public, _version_to_public

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/admin/models", tags=["admin", "models"])


CHUNK_SIZE = 1 * 1024 * 1024  # 1 MiB — streaming sweet spot for ONNX blobs


async def _stream_to_tempfile(
    upload: UploadFile,
) -> tuple[Path, int, str]:
    """Persist the upload to a temp file, returning `(path, size, sha256_hex)`.

    We keep the temp file on disk until the storage backend has copied it
    elsewhere. Caller is responsible for deleting it.
    """
    fd_path = Path(tempfile.NamedTemporaryFile(delete=False, suffix=".onnx").name)
    hasher = hashlib.sha256()
    size = 0
    try:
        with fd_path.open("wb") as out:
            while True:
                chunk = await upload.read(CHUNK_SIZE)
                if not chunk:
                    break
                out.write(chunk)
                hasher.update(chunk)
                size += len(chunk)
    except Exception:
        fd_path.unlink(missing_ok=True)
        raise
    return fd_path, size, hasher.hexdigest()


def _extract_onnx_shapes(path: Path, declared_arch: str) -> Optional[Dict[str, Any]]:
    """Parse the ONNX graph to capture input shapes. Returns None if the
    `onnx` package isn't installed (defense in depth, not required)."""
    try:
        import onnx  # type: ignore
    except ImportError:
        logger.warning("onnx package not installed; skipping shape extraction")
        return None

    try:
        model = onnx.load(str(path))
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Invalid ONNX file: {exc}")

    def _dim(d) -> Any:
        if d.HasField("dim_value"):
            return d.dim_value
        if d.HasField("dim_param"):
            return d.dim_param
        return None

    inputs = {}
    for inp in model.graph.input:
        shape = inp.type.tensor_type.shape
        inputs[inp.name] = [_dim(d) for d in shape.dim]
    outputs = {o.name: None for o in model.graph.output}

    # Sanity-check arch: LSTM policies export h/c state tensors; MLP does not.
    has_state = any(name in outputs for name in ("h_out", "c_out"))
    if declared_arch == "lstm" and not has_state:
        raise HTTPException(
            status_code=400,
            detail="Declared arch='lstm' but ONNX graph has no h_out/c_out outputs",
        )
    if declared_arch == "mlp" and has_state:
        raise HTTPException(
            status_code=400,
            detail="Declared arch='mlp' but ONNX graph exposes recurrent state outputs",
        )
    return {"inputs": inputs, "outputs": list(outputs.keys())}


@router.post("", response_model=ModelRecordPublic, status_code=status.HTTP_201_CREATED)
async def create_model_record(
    request: Request,
    body: ModelRecordCreate = Body(...),
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    existing = await db.execute(select(ModelRecord).where(ModelRecord.slug == body.slug))
    if existing.scalar_one_or_none():
        raise HTTPException(status_code=409, detail="slug already exists")
    record = ModelRecord(
        slug=body.slug,
        name=body.name,
        arch=body.arch,
        description=body.description,
        deck_pool=body.deck_pool,
        min_engine_version=body.min_engine_version,
        is_public=1 if body.is_public else 0,
        created_by_user_id=user.id,
    )
    db.add(record)
    await db.commit()
    await db.refresh(record, attribute_names=["versions"])
    return _record_to_public(request, record)


@router.get("", response_model=List[ModelRecordPublic])
async def list_models_admin(
    request: Request,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    """Admin view includes private + deprecated records that the public
    catalog hides."""
    stmt = (
        select(ModelRecord)
        .options(selectinload(ModelRecord.versions))
        .order_by(ModelRecord.updated_at.desc())
    )
    rows = (await db.execute(stmt)).scalars().all()
    return [_record_to_public(request, r) for r in rows]


@router.post(
    "/{slug}/versions",
    response_model=ModelVersionPublic,
    status_code=status.HTTP_201_CREATED,
)
async def upload_model_version(
    request: Request,
    slug: str,
    file: UploadFile = File(...),
    meta: str = Form(...),
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    try:
        meta_obj = ModelVersionUploadMeta.model_validate_json(meta)
    except ValidationError as exc:
        raise HTTPException(status_code=400, detail=json.loads(exc.json()))

    record = (
        await db.execute(select(ModelRecord).where(ModelRecord.slug == slug))
    ).scalar_one_or_none()
    if record is None:
        raise HTTPException(status_code=404, detail="Model not found")

    clash = (
        await db.execute(
            select(ModelVersion).where(
                ModelVersion.model_id == record.id,
                ModelVersion.version == meta_obj.version,
            )
        )
    ).scalar_one_or_none()
    if clash is not None:
        raise HTTPException(
            status_code=409,
            detail=f"Version {meta_obj.version} already uploaded for {slug}",
        )

    temp_path, size, sha = await _stream_to_tempfile(file)
    try:
        shapes = _extract_onnx_shapes(temp_path, record.arch)
        storage = get_model_storage()
        storage_key = f"{record.slug}/{meta_obj.version}/model.onnx"
        await storage.put(storage_key, temp_path, "application/octet-stream")
    finally:
        temp_path.unlink(missing_ok=True)

    version_row = ModelVersion(
        model_id=record.id,
        version=meta_obj.version,
        storage_key=storage_key,
        size_bytes=size,
        sha256=sha,
        min_engine_version=meta_obj.min_engine_version,
        onnx_input_shapes=shapes,
        changelog=meta_obj.changelog,
        released_at=datetime.now(timezone.utc),
    )
    db.add(version_row)
    await db.commit()
    await db.refresh(version_row)
    return _version_to_public(request, record, version_row)


@router.delete("/{slug}/versions/{version}", status_code=status.HTTP_204_NO_CONTENT)
async def deprecate_model_version(
    slug: str,
    version: str,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    """Soft-delete: marks `is_deprecated=1`. The blob stays in storage so
    clients that already pinned this version keep working."""
    stmt = (
        select(ModelVersion)
        .join(ModelRecord, ModelVersion.model_id == ModelRecord.id)
        .where(ModelRecord.slug == slug, ModelVersion.version == version)
    )
    row = (await db.execute(stmt)).scalar_one_or_none()
    if row is None:
        raise HTTPException(status_code=404, detail="Version not found")
    row.is_deprecated = 1
    await db.commit()

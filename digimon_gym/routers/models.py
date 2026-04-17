"""Public ONNX model catalog.

Read-only, unauthenticated. Lists every `ModelRecord` + every
non-hidden `ModelVersion` so the desktop client can pick/pin/download
a model. Each version's `url` is ready-to-GET — CDN edge for Spaces,
the API's own `/models/{slug}/{version}/blob` for local storage.

Rate limited at a generous cap — the manifest is cheap JSON, but without
a ceiling a single misconfigured client could blow through Droplet
bandwidth hammering a stale manifest.
"""

from __future__ import annotations

from pathlib import Path
from typing import List, Optional

from fastapi import APIRouter, Depends, HTTPException, Request, status
from fastapi.responses import FileResponse
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import selectinload

from digimon_gym.db.database import get_db
from digimon_gym.db.models import ModelRecord, ModelVersion
from digimon_gym.db.schemas import ModelRecordPublic, ModelVersionPublic
from digimon_gym.limiter import limiter
from digimon_gym.models_store import get_model_storage

router = APIRouter(prefix="/models", tags=["models"])


def _version_to_public(
    request: Request, record: ModelRecord, version: ModelVersion
) -> ModelVersionPublic:
    storage = get_model_storage()
    cdn_url = storage.public_url(version.storage_key)
    if cdn_url is None:
        # Local backend: clients download through the API itself.
        path_url = request.url_for(
            "download_model_blob",
            slug=record.slug,
            version=version.version,
        )
        cdn_url = str(path_url)
    return ModelVersionPublic(
        version=version.version,
        size_bytes=version.size_bytes,
        sha256=version.sha256,
        url=cdn_url,
        min_engine_version=version.min_engine_version,
        onnx_input_shapes=version.onnx_input_shapes,
        changelog=version.changelog,
        is_deprecated=bool(version.is_deprecated),
        released_at=version.released_at,
    )


def _record_to_public(
    request: Request, record: ModelRecord
) -> ModelRecordPublic:
    versions_public = [
        _version_to_public(request, record, v) for v in record.versions
    ]
    # `record.versions` is ordered `released_at DESC` by the relationship;
    # `latest` is the first non-deprecated version in that order.
    latest = next(
        (v for v in versions_public if not v.is_deprecated),
        versions_public[0] if versions_public else None,
    )
    return ModelRecordPublic(
        slug=record.slug,
        name=record.name,
        arch=record.arch,
        description=record.description,
        deck_pool=record.deck_pool,
        min_engine_version=record.min_engine_version,
        is_deprecated=bool(record.is_deprecated),
        latest=latest,
        versions=versions_public,
    )


@router.get("", response_model=List[ModelRecordPublic])
@limiter.limit("120/minute")
async def list_models(
    request: Request,
    include_deprecated: bool = False,
    db: AsyncSession = Depends(get_db),
):
    stmt = select(ModelRecord).options(selectinload(ModelRecord.versions))
    stmt = stmt.where(ModelRecord.is_public == 1)
    if not include_deprecated:
        stmt = stmt.where(ModelRecord.is_deprecated == 0)
    stmt = stmt.order_by(ModelRecord.name)
    rows = (await db.execute(stmt)).scalars().all()
    return [_record_to_public(request, r) for r in rows]


@router.get("/{slug}", response_model=ModelRecordPublic)
@limiter.limit("120/minute")
async def get_model(
    request: Request,
    slug: str,
    db: AsyncSession = Depends(get_db),
):
    stmt = (
        select(ModelRecord)
        .options(selectinload(ModelRecord.versions))
        .where(ModelRecord.slug == slug)
    )
    record = (await db.execute(stmt)).scalar_one_or_none()
    if record is None or not record.is_public:
        raise HTTPException(status_code=404, detail="Model not found")
    return _record_to_public(request, record)


@router.get(
    "/{slug}/{version}/blob",
    name="download_model_blob",
    response_class=FileResponse,
)
@limiter.limit("120/minute")
async def download_model_blob(
    request: Request,
    slug: str,
    version: str,
    db: AsyncSession = Depends(get_db),
):
    """Stream the ONNX blob for `(slug, version)` via `FileResponse`.

    Only used when `MODEL_STORAGE_BACKEND=local`. Production deployments
    put a CDN URL in the manifest so this route is never hit on the hot
    path. `FileResponse` supports Range requests out of the box, which
    lets interrupted downloads resume cleanly.
    """
    storage = get_model_storage()
    stmt = (
        select(ModelVersion)
        .join(ModelRecord, ModelVersion.model_id == ModelRecord.id)
        .where(ModelRecord.slug == slug, ModelVersion.version == version)
    )
    version_row = (await db.execute(stmt)).scalar_one_or_none()
    if version_row is None:
        raise HTTPException(status_code=404, detail="Version not found")

    local_path: Optional[Path] = storage.local_path(version_row.storage_key)
    if local_path is None:
        # Shouldn't happen: we only register `local_path`-backed blobs
        # for the local backend. If Spaces is mixed in, the client should
        # have followed `public_url` from the manifest instead of hitting
        # this route.
        raise HTTPException(
            status_code=status.HTTP_501_NOT_IMPLEMENTED,
            detail="Direct blob download is only available for the local storage backend",
        )
    return FileResponse(
        path=str(local_path),
        media_type="application/octet-stream",
        filename=f"{slug}-{version}.onnx",
    )

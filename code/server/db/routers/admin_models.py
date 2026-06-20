"""Admin model management router: admin CRUD + public manifest endpoint."""

from __future__ import annotations

import asyncio
import os
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

from botocore.exceptions import ClientError
from fastapi import APIRouter, Depends, HTTPException, Query, Response, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from server.db.auth import get_current_user, require_admin_or_api_key
from server.db.database import get_db
from server.db.models import AIModel, Deck, User
from server.db.schemas import (
    AIModelConfirmResponse,
    AIModelCreateRequest,
    AIModelCreateResponse,
    AIModelResponse,
    AIModelUpdateRequest,
    ListAIModelsResponse,
    ManifestModel,
    ManifestResponse,
    PrepareModelResponse,
)
from digimon_engine import get_models_dir
from server.storage import spaces
from server.storage.model_resolver import resolve_manifest_model_path

admin_router = APIRouter(prefix="/admin/models", tags=["admin-models"])
public_router = APIRouter(prefix="/models", tags=["models-public"])


def _utcnow() -> datetime:
    return datetime.now(timezone.utc)


def _to_admin_response(
    row: AIModel,
    uploader_username: str | None,
    deck_name: str | None,
) -> AIModelResponse:
    return AIModelResponse(
        id=row.id,
        name=row.name,
        model_type=row.model_type,
        tensor_size=row.tensor_size,
        action_space_size=row.action_space_size,
        engine_commit=row.engine_commit,
        trained_at=row.trained_at,
        file_sha256=row.file_sha256,
        file_size_bytes=row.file_size_bytes,
        spaces_key=row.spaces_key,
        deck_id=row.deck_id,
        uploaded_by=row.uploaded_by,
        published=row.published,
        state=row.state,
        notes=row.notes,
        created_at=row.created_at,
        updated_at=row.updated_at,
        uploader_username=uploader_username,
        deck_name=deck_name,
    )


# ── Admin endpoints ───────────────────────────────────────────────────────


@admin_router.post(
    "",
    response_model=AIModelCreateResponse,
    status_code=status.HTTP_201_CREATED,
)
async def create_model(
    request: AIModelCreateRequest,
    current_user: Optional[User] = Depends(require_admin_or_api_key),
    db: AsyncSession = Depends(get_db),
) -> AIModelCreateResponse:
    """Create an AIModel row in pending state and return a presigned PUT URL."""
    import uuid as _uuid_mod
    model_id = str(_uuid_mod.uuid4())
    spaces_key = f"models/{model_id}/policy.onnx"

    row = AIModel(
        id=model_id,
        name=request.name,
        model_type=request.model_type,
        engine_commit=request.engine_commit,
        trained_at=request.trained_at,
        deck_id=request.deck_id,
        notes=request.notes,
        state="pending",
        published=False,
        uploaded_by=current_user.id if current_user else None,
        spaces_key=spaces_key,
    )
    db.add(row)
    await db.commit()
    await db.refresh(row)

    upload_url = await asyncio.to_thread(
        spaces.generate_presigned_put, spaces_key, 900
    )
    return AIModelCreateResponse(
        id=row.id,
        upload_url=upload_url,
        spaces_key=spaces_key,
        expires_in=900,
    )


@admin_router.post(
    "/{model_id}/confirm",
    response_model=AIModelConfirmResponse,
)
async def confirm_model(
    model_id: str,
    _: Optional[User] = Depends(require_admin_or_api_key),
    db: AsyncSession = Depends(get_db),
) -> AIModelConfirmResponse:
    """Confirm upload: stream sha256, inspect ONNX, populate model fields."""
    result = await db.execute(select(AIModel).where(AIModel.id == model_id))
    row = result.scalar_one_or_none()
    if row is None:
        raise HTTPException(status_code=404, detail="Model not found")

    # Verify object exists in Spaces
    try:
        await asyncio.to_thread(spaces.head_object, row.spaces_key)
    except ClientError as exc:
        code = exc.response.get("Error", {}).get("Code", "")
        if code in ("404", "NoSuchKey"):
            row.state = "failed"
            await db.commit()
            raise HTTPException(
                status_code=422,
                detail="Object not found at key",
            )
        raise

    # Single-pass download: write to temp file while computing sha256.
    tmp_fd, tmp_path = tempfile.mkstemp(suffix=".onnx")
    os.close(tmp_fd)
    try:
        sha256_hex, total_bytes = await asyncio.to_thread(
            spaces.download_and_hash, row.spaces_key, tmp_path
        )

        import onnxruntime  # local import so desktop can still import router module

        async def _fail(detail: str) -> HTTPException:
            row.state = "failed"
            await db.commit()
            await asyncio.to_thread(spaces.delete_object, row.spaces_key)
            return HTTPException(status_code=422, detail=detail)

        try:
            sess = await asyncio.to_thread(
                onnxruntime.InferenceSession,
                tmp_path,
                providers=["CPUExecutionProvider"],
            )
        except Exception as exc:
            raise await _fail(f"Failed to load ONNX model: {exc}")

        obs_input = next((i for i in sess.get_inputs() if i.name == "obs"), None)
        logits_output = next(
            (o for o in sess.get_outputs() if o.name == "logits"), None
        )

        if obs_input is None:
            raise await _fail("Missing 'obs' input in ONNX model")
        if logits_output is None:
            raise await _fail("Missing 'logits' output in ONNX model")

        try:
            tensor_size = int(obs_input.shape[-1])
            action_space_size = int(logits_output.shape[-1])
        except (TypeError, ValueError):
            raise await _fail("ONNX shape not static")

    finally:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass

    # Success — update model fields
    row.state = "uploaded"
    row.file_sha256 = sha256_hex
    row.file_size_bytes = total_bytes
    row.tensor_size = tensor_size
    row.action_space_size = action_space_size
    await db.commit()
    await db.refresh(row)

    return AIModelConfirmResponse(
        id=row.id,
        state=row.state,
        file_sha256=row.file_sha256,
        file_size_bytes=row.file_size_bytes,
        tensor_size=row.tensor_size,
        action_space_size=row.action_space_size,
    )


@admin_router.patch("/{model_id}", response_model=AIModelResponse)
async def update_model(
    model_id: str,
    request: AIModelUpdateRequest,
    _: Optional[User] = Depends(require_admin_or_api_key),
    db: AsyncSession = Depends(get_db),
) -> AIModelResponse:
    """Update editable model fields. Publishing requires state='uploaded'."""
    result = await db.execute(select(AIModel).where(AIModel.id == model_id))
    row = result.scalar_one_or_none()
    if row is None:
        raise HTTPException(status_code=404, detail="Model not found")

    if request.published is True and row.state != "uploaded":
        raise HTTPException(
            status_code=409,
            detail="Cannot publish a model that is not in 'uploaded' state",
        )

    if request.name is not None:
        row.name = request.name
    if request.deck_id is not None:
        row.deck_id = request.deck_id
    if request.published is not None:
        row.published = request.published
    if request.notes is not None:
        row.notes = request.notes
    if request.engine_commit is not None:
        row.engine_commit = request.engine_commit
    if request.trained_at is not None:
        row.trained_at = request.trained_at

    await db.commit()
    await db.refresh(row)

    # Resolve uploader_username
    uploader_username: str | None = None
    if row.uploaded_by:
        user_result = await db.execute(
            select(User.username).where(User.id == row.uploaded_by)
        )
        uploader_username = user_result.scalar_one_or_none()

    # Resolve deck_name
    deck_name: str | None = None
    if row.deck_id:
        deck_result = await db.execute(
            select(Deck.name).where(Deck.id == row.deck_id)
        )
        deck_name = deck_result.scalar_one_or_none()

    return _to_admin_response(row, uploader_username, deck_name)


@admin_router.delete("/{model_id}", status_code=status.HTTP_204_NO_CONTENT, response_model=None)
async def delete_model(
    model_id: str,
    _: Optional[User] = Depends(require_admin_or_api_key),
    db: AsyncSession = Depends(get_db),
) -> None:
    """Delete a model row and best-effort remove the Spaces object."""
    result = await db.execute(select(AIModel).where(AIModel.id == model_id))
    row = result.scalar_one_or_none()
    if row is None:
        raise HTTPException(status_code=404, detail="Model not found")

    try:
        await asyncio.to_thread(spaces.delete_object, row.spaces_key)
    except ClientError:
        pass  # best-effort

    await db.delete(row)
    await db.commit()
    return None


@admin_router.get("", response_model=ListAIModelsResponse)
async def list_models(
    state: Optional[str] = Query(None),
    published: Optional[bool] = Query(None),
    model_type: Optional[str] = Query(None),
    deck_id: Optional[str] = Query(None),
    _: Optional[User] = Depends(require_admin_or_api_key),
    db: AsyncSession = Depends(get_db),
) -> ListAIModelsResponse:
    """List models with optional filters. Results ordered newest first."""
    q = select(AIModel)
    if state is not None:
        q = q.where(AIModel.state == state)
    if published is not None:
        q = q.where(AIModel.published == published)
    if model_type is not None:
        q = q.where(AIModel.model_type == model_type)
    if deck_id is not None:
        q = q.where(AIModel.deck_id == deck_id)

    q = q.order_by(AIModel.created_at.desc())
    result = await db.execute(q)
    rows = result.scalars().all()

    # Resolve usernames and deck names in bulk
    user_ids = {r.uploaded_by for r in rows if r.uploaded_by}
    deck_ids = {r.deck_id for r in rows if r.deck_id}

    username_map: dict[str, str] = {}
    if user_ids:
        users_result = await db.execute(
            select(User.id, User.username).where(User.id.in_(user_ids))
        )
        username_map = {uid: uname for uid, uname in users_result.all()}

    deck_name_map: dict[str, str] = {}
    if deck_ids:
        decks_result = await db.execute(
            select(Deck.id, Deck.name).where(Deck.id.in_(deck_ids))
        )
        deck_name_map = {did: dname for did, dname in decks_result.all()}

    models = [
        _to_admin_response(
            row,
            username_map.get(row.uploaded_by) if row.uploaded_by else None,
            deck_name_map.get(row.deck_id) if row.deck_id else None,
        )
        for row in rows
    ]
    return ListAIModelsResponse(models=models)


# ── Public manifest endpoint ──────────────────────────────────────────────


@public_router.get("/manifest.json", response_model=ManifestResponse)
async def get_manifest(
    response: Response,
    db: AsyncSession = Depends(get_db),
) -> ManifestResponse:
    """Public endpoint: returns all published, uploaded models as a manifest."""
    q = (
        select(AIModel)
        .where(AIModel.published == True)  # noqa: E712
        .where(AIModel.state == "uploaded")
        .order_by(AIModel.trained_at.desc())
    )
    result = await db.execute(q)
    rows = result.scalars().all()

    deck_ids = {r.deck_id for r in rows if r.deck_id}
    deck_name_map: dict[str, str] = {}
    if deck_ids:
        decks_result = await db.execute(
            select(Deck.id, Deck.name).where(Deck.id.in_(deck_ids))
        )
        deck_name_map = {did: dname for did, dname in decks_result.all()}

    manifest_models = [
        ManifestModel(
            id=row.id,
            name=row.name,
            model_type=row.model_type,
            tensor_size=row.tensor_size,
            action_space_size=row.action_space_size,
            engine_commit=row.engine_commit,
            trained_at=row.trained_at,
            file_sha256=row.file_sha256,
            file_size_bytes=row.file_size_bytes,
            url=spaces.public_url(row.spaces_key),
            deck_id=row.deck_id,
            deck_name=deck_name_map.get(row.deck_id) if row.deck_id else None,
            notes=row.notes,
        )
        for row in rows
    ]

    response.headers["Cache-Control"] = "public, max-age=60"
    return ManifestResponse(
        generated_at=_utcnow(),
        models=manifest_models,
    )


@public_router.post("/{model_id}/prepare", response_model=PrepareModelResponse)
async def prepare_model(
    model_id: str,
    _: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> PrepareModelResponse:
    """Stage a manifest model into the server-local ONNX dir so that a
    subsequent POST /games with player_model=<returned filename> can load it.

    The resolver writes to `MANIFEST_MODEL_CACHE_DIR`, which must be
    configured to match (or be a subdir of) `ONNX_MODELS_DIR` so that
    `/games` can find the file by its returned filename. Safe on repeated
    calls: the file is keyed by sha256 and hits the cache.
    """
    try:
        path, downloaded = await resolve_manifest_model_path(db, model_id)
    except FileNotFoundError as exc:
        raise HTTPException(status_code=404, detail=str(exc))

    filename = Path(path).name  # "<sha256>.onnx"
    # Ensure get_models_dir sees this file.
    models_dir = get_models_dir()
    if Path(path).parent != models_dir:
        target = models_dir / filename
        if not target.exists():
            models_dir.mkdir(parents=True, exist_ok=True)
            import shutil
            shutil.copy2(path, target)
    return PrepareModelResponse(filename=filename, downloaded=downloaded)

"""Assets router: upload, list, delete custom card backs and board skins."""

from __future__ import annotations

import os
import uuid
from pathlib import Path
from typing import List

from fastapi import APIRouter, Depends, File, HTTPException, UploadFile
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from server.db.auth import get_current_user
from server.db.database import get_db
from server.db.models import User, UserAsset, UserPreferences
from server.db.schemas import (
    AssetResponse,
    PreferencesResponse,
    UpdatePreferencesRequest,
)

router = APIRouter(prefix="/assets", tags=["assets"])

ASSETS_ROOT = Path("data/user_assets")
ALLOWED_MIME = {"image/png", "image/jpeg", "image/webp"}
MAX_FILE_SIZE = {
    "card_back": 5 * 1024 * 1024,   # 5 MB
    "board_skin": 10 * 1024 * 1024,  # 10 MB
    "avatar": 2 * 1024 * 1024,       # 2 MB
}
USER_QUOTA_BYTES = 150 * 1024 * 1024  # 150 MB total per user


@router.post("/upload", response_model=AssetResponse)
async def upload_asset(
    asset_type: str,
    name: str,
    file: UploadFile = File(...),
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    if asset_type not in MAX_FILE_SIZE:
        raise HTTPException(status_code=400, detail=f"Invalid asset_type. Must be one of: {list(MAX_FILE_SIZE.keys())}")

    if file.content_type not in ALLOWED_MIME:
        raise HTTPException(status_code=400, detail=f"File type {file.content_type} not allowed. Use PNG, JPEG, or WebP.")

    # Read file content
    content = await file.read()
    file_size = len(content)

    if file_size > MAX_FILE_SIZE[asset_type]:
        max_mb = MAX_FILE_SIZE[asset_type] / (1024 * 1024)
        raise HTTPException(status_code=400, detail=f"File too large. Max for {asset_type}: {max_mb:.0f} MB")

    # Check user quota
    result = await db.execute(
        select(UserAsset.file_size_bytes).where(UserAsset.owner_id == user.id)
    )
    current_usage = sum(r[0] or 0 for r in result.all())
    if current_usage + file_size > USER_QUOTA_BYTES:
        raise HTTPException(status_code=400, detail="Storage quota exceeded (150 MB)")

    # Save to filesystem
    asset_id = str(uuid.uuid4())
    ext = file.filename.rsplit(".", 1)[-1] if file.filename and "." in file.filename else "png"
    asset_dir = ASSETS_ROOT / user.id / asset_type
    asset_dir.mkdir(parents=True, exist_ok=True)
    file_path = asset_dir / f"{asset_id}.{ext}"
    file_path.write_bytes(content)

    file_url = str(file_path)

    # TODO: Generate thumbnail with Pillow when needed

    asset = UserAsset(
        id=asset_id,
        owner_id=user.id,
        asset_type=asset_type,
        name=name,
        file_url=file_url,
        file_size_bytes=file_size,
        mime_type=file.content_type,
    )
    db.add(asset)
    await db.commit()
    await db.refresh(asset)
    return asset


@router.get("", response_model=List[AssetResponse])
async def list_assets(
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(
        select(UserAsset).where(UserAsset.owner_id == user.id).order_by(UserAsset.created_at.desc())
    )
    return result.scalars().all()


@router.delete("/{asset_id}")
async def delete_asset(
    asset_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(UserAsset).where(UserAsset.id == asset_id))
    asset = result.scalar_one_or_none()
    if not asset:
        raise HTTPException(status_code=404, detail="Asset not found")
    if asset.owner_id != user.id:
        raise HTTPException(status_code=403, detail="Access denied")

    # Delete file from filesystem
    try:
        file_path = Path(asset.file_url)
        if file_path.exists():
            file_path.unlink()
    except OSError:
        pass  # File may already be gone

    await db.delete(asset)
    await db.commit()
    return {"status": "deleted"}


# ── Preferences ─────────────────────────────────────────────────────────

@router.get("/preferences", response_model=PreferencesResponse)
async def get_preferences(
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(UserPreferences).where(UserPreferences.user_id == user.id))
    prefs = result.scalar_one_or_none()
    if not prefs:
        return PreferencesResponse()
    return prefs


@router.put("/preferences", response_model=PreferencesResponse)
async def update_preferences(
    request: UpdatePreferencesRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(UserPreferences).where(UserPreferences.user_id == user.id))
    prefs = result.scalar_one_or_none()

    if not prefs:
        prefs = UserPreferences(user_id=user.id)
        db.add(prefs)

    # Validate that referenced assets exist and belong to user
    if request.active_card_back_id is not None:
        if request.active_card_back_id:
            asset = await db.execute(select(UserAsset).where(UserAsset.id == request.active_card_back_id))
            a = asset.scalar_one_or_none()
            if not a or a.owner_id != user.id or a.asset_type != "card_back":
                raise HTTPException(status_code=400, detail="Invalid card back asset")
        prefs.active_card_back_id = request.active_card_back_id or None

    if request.active_board_skin_id is not None:
        if request.active_board_skin_id:
            asset = await db.execute(select(UserAsset).where(UserAsset.id == request.active_board_skin_id))
            a = asset.scalar_one_or_none()
            if not a or a.owner_id != user.id or a.asset_type != "board_skin":
                raise HTTPException(status_code=400, detail="Invalid board skin asset")
        prefs.active_board_skin_id = request.active_board_skin_id or None

    await db.commit()
    await db.refresh(prefs)
    return prefs

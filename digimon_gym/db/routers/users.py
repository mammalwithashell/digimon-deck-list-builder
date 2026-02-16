"""Users router: profile view, update, search."""

from __future__ import annotations

from typing import List, Optional

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.db.auth import get_current_user
from digimon_gym.db.database import get_db
from digimon_gym.db.models import User
from digimon_gym.db.schemas import UpdateProfileRequest, UserProfile, UserPublic

router = APIRouter(prefix="/users", tags=["users"])


@router.get("/me", response_model=UserProfile)
async def get_my_profile(user: User = Depends(get_current_user)):
    return user


@router.patch("/me", response_model=UserProfile)
async def update_my_profile(
    request: UpdateProfileRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    if request.display_name is not None:
        user.display_name = request.display_name
    if request.avatar_url is not None:
        user.avatar_url = request.avatar_url
    await db.commit()
    await db.refresh(user)
    return user


@router.get("/search", response_model=List[UserPublic])
async def search_users(
    q: str = Query(..., min_length=2, max_length=50),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(
        select(User)
        .where(User.username.ilike(f"%{q}%"), User.is_active == 1)
        .limit(20)
    )
    users = result.scalars().all()
    return [
        UserPublic(
            id=u.id,
            username=u.username,
            display_name=u.display_name,
            avatar_url=u.avatar_url,
        )
        for u in users
    ]


@router.get("/{user_id}", response_model=UserPublic)
async def get_user_profile(
    user_id: str,
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(User).where(User.id == user_id, User.is_active == 1))
    target = result.scalar_one_or_none()
    if not target:
        raise HTTPException(status_code=404, detail="User not found")
    return UserPublic(
        id=target.id,
        username=target.username,
        display_name=target.display_name,
        avatar_url=target.avatar_url,
    )

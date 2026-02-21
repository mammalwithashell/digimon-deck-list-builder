"""Authentication router: register, login, refresh, logout."""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.db.auth import (
    ACCESS_TOKEN_EXPIRE_MINUTES,
    REFRESH_TOKEN_EXPIRE_DAYS,
    create_access_token,
    create_refresh_token_value,
    get_current_user,
    hash_password,
    hash_token,
    verify_password,
)
from digimon_gym.db.database import get_db
from digimon_gym.db.models import RefreshToken, User, UserPreferences
from digimon_gym.db.schemas import (
    ChangePasswordRequest,
    LoginRequest,
    RefreshRequest,
    RegisterRequest,
    TokenResponse,
    UserProfile,
)

router = APIRouter(prefix="/auth", tags=["auth"])


@router.post("/register", response_model=UserProfile, status_code=status.HTTP_201_CREATED)
async def register(request: RegisterRequest, db: AsyncSession = Depends(get_db)):
    # Check username uniqueness
    existing = await db.execute(select(User).where(User.username == request.username))
    if existing.scalar_one_or_none():
        raise HTTPException(status_code=409, detail="Username already taken")

    # Check email uniqueness
    existing = await db.execute(select(User).where(User.email == request.email))
    if existing.scalar_one_or_none():
        raise HTTPException(status_code=409, detail="Email already registered")

    user = User(
        username=request.username,
        email=request.email,
        password_hash=hash_password(request.password),
        display_name=request.display_name,
    )
    db.add(user)
    await db.flush()  # Ensure user.id is populated

    # Create default preferences row
    prefs = UserPreferences(user_id=user.id)
    db.add(prefs)

    await db.commit()
    await db.refresh(user)
    return user


@router.post("/login", response_model=TokenResponse)
async def login(request: LoginRequest, db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(User).where(User.username == request.username))
    user = result.scalar_one_or_none()

    if not user or not verify_password(request.password, user.password_hash):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid username or password",
        )

    if not user.is_active:
        raise HTTPException(status_code=403, detail="Account is disabled")

    # Update last login
    user.last_login_at = datetime.now(timezone.utc)

    # Create tokens
    access_token = create_access_token(user.id, user.username)
    refresh_value = create_refresh_token_value()

    refresh_record = RefreshToken(
        user_id=user.id,
        token_hash=hash_token(refresh_value),
        expires_at=datetime.now(timezone.utc) + timedelta(days=REFRESH_TOKEN_EXPIRE_DAYS),
    )
    db.add(refresh_record)
    await db.commit()

    return TokenResponse(
        access_token=access_token,
        refresh_token=refresh_value,
    )


@router.post("/refresh", response_model=TokenResponse)
async def refresh_tokens(request: RefreshRequest, db: AsyncSession = Depends(get_db)):
    token_digest = hash_token(request.refresh_token)
    result = await db.execute(
        select(RefreshToken).where(
            RefreshToken.token_hash == token_digest,
            RefreshToken.revoked == 0,
        )
    )
    record = result.scalar_one_or_none()

    if not record:
        raise HTTPException(status_code=401, detail="Invalid refresh token")

    # SQLite returns naive datetimes; compare consistently
    expires = record.expires_at.replace(tzinfo=timezone.utc) if record.expires_at.tzinfo is None else record.expires_at
    if expires < datetime.now(timezone.utc):
        raise HTTPException(status_code=401, detail="Refresh token expired")

    # Revoke old token
    record.revoked = 1

    # Look up user
    user_result = await db.execute(select(User).where(User.id == record.user_id))
    user = user_result.scalar_one_or_none()
    if not user or not user.is_active:
        raise HTTPException(status_code=401, detail="User not found or disabled")

    # Issue new pair
    access_token = create_access_token(user.id, user.username)
    new_refresh_value = create_refresh_token_value()

    new_record = RefreshToken(
        user_id=user.id,
        token_hash=hash_token(new_refresh_value),
        expires_at=datetime.now(timezone.utc) + timedelta(days=REFRESH_TOKEN_EXPIRE_DAYS),
    )
    db.add(new_record)
    await db.commit()

    return TokenResponse(
        access_token=access_token,
        refresh_token=new_refresh_value,
    )


@router.post("/change-password")
async def change_password(
    request: ChangePasswordRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    if not verify_password(request.current_password, user.password_hash):
        raise HTTPException(status_code=400, detail="Current password is incorrect")

    user.password_hash = hash_password(request.new_password)

    # Revoke all existing refresh tokens so other sessions must re-authenticate
    result = await db.execute(
        select(RefreshToken).where(
            RefreshToken.user_id == user.id,
            RefreshToken.revoked == 0,
        )
    )
    for token in result.scalars().all():
        token.revoked = 1

    await db.commit()
    return {"status": "password_changed"}


@router.post("/logout")
async def logout(
    request: RefreshRequest,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(get_current_user),
):
    token_digest = hash_token(request.refresh_token)
    result = await db.execute(
        select(RefreshToken).where(
            RefreshToken.token_hash == token_digest,
            RefreshToken.user_id == user.id,
        )
    )
    record = result.scalar_one_or_none()
    if record:
        record.revoked = 1
        await db.commit()
    return {"status": "logged_out"}

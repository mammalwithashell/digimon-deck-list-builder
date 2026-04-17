"""Authentication router: register, login, refresh, logout."""

from datetime import datetime, timedelta, timezone

from fastapi import APIRouter, Body, Depends, HTTPException, Request, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.config import settings
from digimon_gym.limiter import limiter
from digimon_gym.db.auth import (
    ACCESS_TOKEN_EXPIRE_MINUTES,
    REFRESH_TOKEN_EXPIRE_DAYS,
    ROLE_PLAYER,
    assign_role_to_user,
    create_access_token,
    create_refresh_token_value,
    get_current_user,
    get_user_role_names,
    hash_password,
    hash_token,
    verify_password,
)
from digimon_gym.db.database import get_db
from digimon_gym.db.models import InviteCode, RefreshToken, User, UserPreferences
from digimon_gym.db.schemas import (
    LoginRequest,
    RefreshRequest,
    RegisterRequest,
    TokenResponse,
    UserProfile,
)

router = APIRouter(prefix="/auth", tags=["auth"])


def _to_user_profile(user: User, roles: list[str]) -> UserProfile:
    return UserProfile(
        id=user.id,
        username=user.username,
        email=user.email,
        display_name=user.display_name,
        avatar_url=user.avatar_url,
        roles=roles,
        created_at=user.created_at,
        last_login_at=user.last_login_at,
    )


@router.post("/register", response_model=UserProfile, status_code=status.HTTP_201_CREATED)
@limiter.limit("5/hour")
async def register(
    request: Request,
    body: RegisterRequest = Body(...),
    db: AsyncSession = Depends(get_db),
):
    # Invite-code gate (alpha). We lock the row to make redemption atomic
    # with the new-user insert below; double-spend races would otherwise
    # let two users claim the same code.
    invite_row: InviteCode | None = None
    if settings.invite_codes_required:
        if not body.invite_code:
            raise HTTPException(status_code=400, detail="Invite code is required")
        result = await db.execute(
            select(InviteCode)
            .where(InviteCode.code == body.invite_code)
            .with_for_update()
        )
        invite_row = result.scalar_one_or_none()
        if invite_row is None or invite_row.redeemed_by_user_id is not None:
            raise HTTPException(status_code=400, detail="Invalid or already-used invite code")

    # Check username uniqueness
    existing = await db.execute(select(User).where(User.username == body.username))
    if existing.scalar_one_or_none():
        raise HTTPException(status_code=409, detail="Username already taken")

    # Check email uniqueness
    existing = await db.execute(select(User).where(User.email == body.email))
    if existing.scalar_one_or_none():
        raise HTTPException(status_code=409, detail="Email already registered")

    user = User(
        username=body.username,
        email=body.email,
        password_hash=hash_password(body.password),
        display_name=body.display_name,
    )
    db.add(user)
    await db.flush()  # Ensure user.id is populated

    # Create default preferences row
    prefs = UserPreferences(user_id=user.id)
    db.add(prefs)
    await assign_role_to_user(db, user.id, ROLE_PLAYER)

    if invite_row is not None:
        invite_row.redeemed_by_user_id = user.id
        invite_row.redeemed_at = datetime.now(timezone.utc)

    await db.commit()
    await db.refresh(user)
    roles = sorted(await get_user_role_names(user.id, db))
    return _to_user_profile(user, roles)


@router.post("/login", response_model=TokenResponse)
@limiter.limit("10/minute")
async def login(
    request: Request,
    body: LoginRequest = Body(...),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(User).where(User.username == body.username))
    user = result.scalar_one_or_none()

    if not user or not verify_password(body.password, user.password_hash):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid username or password",
        )

    if not user.is_active:
        raise HTTPException(status_code=403, detail="Account is disabled")

    # Update last login
    user.last_login_at = datetime.now(timezone.utc)

    # Create tokens
    roles = sorted(await get_user_role_names(user.id, db))
    access_token = create_access_token(user.id, user.username, roles=roles)
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
    roles = sorted(await get_user_role_names(user.id, db))
    access_token = create_access_token(user.id, user.username, roles=roles)
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

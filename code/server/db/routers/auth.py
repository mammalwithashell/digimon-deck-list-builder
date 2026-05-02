"""Authentication router: register, login, refresh, logout."""

import secrets
import string
import time
import uuid as _uuid
from collections import deque
from datetime import datetime, timedelta, timezone

from fastapi import APIRouter, Body, Depends, HTTPException, Request, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from server.config import settings
from server.db.auth import (
    ACCESS_TOKEN_EXPIRE_MINUTES,
    REFRESH_TOKEN_EXPIRE_DAYS,
    ROLE_PLAYER,
    assign_role_to_user,
    create_access_token,
    create_guest_access_token,
    create_refresh_token_value,
    get_current_user,
    get_user_role_names,
    hash_password,
    hash_token,
    verify_password,
)
from server.db.database import get_db
from server.db.models import InviteCode, RefreshToken, User, UserPreferences
from server.db.schemas import (
    GuestSessionResponse,
    LoginRequest,
    RefreshRequest,
    RegisterRequest,
    TokenResponse,
    UserProfile,
)
from server.limiter import limiter

router = APIRouter(prefix="/auth", tags=["auth"])


# Per-IP rate limit state for /auth/guest. Kept in-process; OK for the alpha
# single-instance deployment. A production rollout should move this to Redis
# or an edge proxy.
_GUEST_MINT_MAX_PER_WINDOW = 10
_GUEST_MINT_WINDOW_SECONDS = 60
_guest_mint_timestamps: dict[str, deque[float]] = {}


def _rate_limit_guest_mint(client_ip: str) -> bool:
    """Return True if the request is allowed, False if it should be rejected."""
    now = time.monotonic()
    cutoff = now - _GUEST_MINT_WINDOW_SECONDS
    timestamps = _guest_mint_timestamps.setdefault(client_ip, deque())
    while timestamps and timestamps[0] < cutoff:
        timestamps.popleft()
    if len(timestamps) >= _GUEST_MINT_MAX_PER_WINDOW:
        return False
    timestamps.append(now)
    return True


def _reset_guest_rate_limit() -> None:
    """Test hook — wipes the in-memory rate-limit state."""
    _guest_mint_timestamps.clear()


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


def _generate_guest_suffix() -> str:
    alphabet = string.ascii_uppercase + string.digits
    return "".join(secrets.choice(alphabet) for _ in range(4))


@router.post(
    "/guest",
    response_model=GuestSessionResponse,
    status_code=status.HTTP_201_CREATED,
)
async def create_guest(
    request: Request,
    db: AsyncSession = Depends(get_db),
) -> GuestSessionResponse:
    """Create an anonymous guest account and return a long-lived access token.

    Each call mints a distinct guest. Clients are expected to cache the
    token in `localStorage` and reuse it on subsequent launches. Losing
    the token creates a new identity next boot — acceptable because
    guest-owned data (decks) lives on the client.
    """
    client_ip = request.client.host if request.client else "unknown"
    if not _rate_limit_guest_mint(client_ip):
        raise HTTPException(
            status_code=status.HTTP_429_TOO_MANY_REQUESTS,
            detail="Too many guest sessions from this IP; try again in a minute.",
        )
    guest_id = f"guest_{_uuid.uuid4()}"
    suffix = _generate_guest_suffix()
    display_name = f"Guest-{suffix}"
    # Use suffix in the synthetic email to keep the unique constraint happy.
    placeholder_email = f"{guest_id}@guest.invalid"

    user = User(
        id=guest_id,
        username=guest_id,
        email=placeholder_email,
        password_hash=hash_password(secrets.token_urlsafe(32)),
        display_name=display_name,
        is_guest=True,
    )
    db.add(user)
    await db.commit()

    token = create_guest_access_token(user_id=guest_id, display_name=display_name)
    return GuestSessionResponse(
        access_token=token,
        user_id=guest_id,
        display_name=display_name,
    )

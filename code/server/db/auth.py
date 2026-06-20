"""Authentication utilities: password hashing, JWT encode/decode, FastAPI dependency."""

from __future__ import annotations

import hashlib
import hmac
import os
import secrets
from datetime import datetime, timedelta, timezone
from typing import Optional, Sequence, Set

from fastapi import Depends, Header, HTTPException, status
from fastapi.security import OAuth2PasswordBearer
import bcrypt
from jose import JWTError, jwt
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from server.config import settings
from server.db.database import get_db
from server.db.models import RefreshToken, Role, User, UserRole

# - Configuration ---------------------------------------------------------

SECRET_KEY = settings.secret_key
ALGORITHM = "HS256"
ACCESS_TOKEN_EXPIRE_MINUTES = 30
REFRESH_TOKEN_EXPIRE_DAYS = 30

ROLE_ADMIN = "admin"
ROLE_JUDGE = "judge"
ROLE_PLAYER = "player"
DEFAULT_ROLES: tuple[str, ...] = (ROLE_ADMIN, ROLE_JUDGE, ROLE_PLAYER)
DEFAULT_ROLE_DESCRIPTIONS: dict[str, str] = {
    ROLE_ADMIN: "Full administrative access for AI tasks and promotions.",
    ROLE_JUDGE: "Review/triage access for issue validation.",
    ROLE_PLAYER: "Gameplay and bug report submission access.",
}

# ── Password Hashing ───────────────────────────────────────────────────


def hash_password(password: str) -> str:
    salt = bcrypt.gensalt()
    return bcrypt.hashpw(password.encode("utf-8"), salt).decode("utf-8")


def verify_password(plain_password: str, hashed_password: str) -> bool:
    return bcrypt.checkpw(plain_password.encode("utf-8"), hashed_password.encode("utf-8"))


# ── JWT Tokens ──────────────────────────────────────────────────────────

def create_access_token(user_id: str, username: str, roles: Sequence[str] | None = None) -> str:
    expire = datetime.now(timezone.utc) + timedelta(minutes=ACCESS_TOKEN_EXPIRE_MINUTES)
    payload = {
        "sub": user_id,
        "username": username,
        "roles": list(roles or []),
        "exp": expire,
        "type": "access",
    }
    return jwt.encode(payload, SECRET_KEY, algorithm=ALGORITHM)


GUEST_TOKEN_EXPIRE_DAYS = 365


def create_guest_access_token(user_id: str, display_name: str) -> str:
    """Mint a long-lived access token for an anonymous guest session.

    The token is flagged with `is_guest: True` so downstream code can
    cheaply distinguish guests without a DB lookup. Expiry is one year
    because nothing the guest owns is server-side — a token rotation
    would cost the user a new identity for no upside.
    """
    expire = datetime.now(timezone.utc) + timedelta(days=GUEST_TOKEN_EXPIRE_DAYS)
    payload = {
        "sub": user_id,
        "username": display_name,
        "roles": [],
        "exp": expire,
        "type": "access",
        "is_guest": True,
    }
    return jwt.encode(payload, SECRET_KEY, algorithm=ALGORITHM)


def create_refresh_token_value() -> str:
    """Generate a cryptographically random refresh token string."""
    return secrets.token_urlsafe(64)


def hash_token(token: str) -> str:
    """SHA-256 hash a refresh token for storage."""
    return hashlib.sha256(token.encode()).hexdigest()


def decode_access_token(token: str) -> dict:
    """Decode and validate an access token. Raises JWTError on failure."""
    payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
    if payload.get("type") != "access":
        raise JWTError("Invalid token type")
    return payload


# ── FastAPI Dependency ──────────────────────────────────────────────────

oauth2_scheme = OAuth2PasswordBearer(tokenUrl="/auth/login")


async def ensure_role_exists(
    db: AsyncSession,
    role_name: str,
    description: str = "",
) -> Role:
    result = await db.execute(select(Role).where(Role.name == role_name))
    role = result.scalar_one_or_none()
    if role:
        return role

    role = Role(name=role_name, description=description or DEFAULT_ROLE_DESCRIPTIONS.get(role_name, ""))
    db.add(role)
    await db.flush()
    return role


async def assign_role_to_user(db: AsyncSession, user_id: str, role_name: str) -> None:
    role = await ensure_role_exists(db, role_name)
    existing = await db.execute(
        select(UserRole).where(
            UserRole.user_id == user_id,
            UserRole.role_id == role.id,
        )
    )
    if existing.scalar_one_or_none():
        return

    db.add(UserRole(user_id=user_id, role_id=role.id))


async def get_user_role_names(user_id: str, db: AsyncSession) -> Set[str]:
    result = await db.execute(
        select(Role.name)
        .join(UserRole, UserRole.role_id == Role.id)
        .where(UserRole.user_id == user_id)
    )
    return {row[0] for row in result.all()}


async def user_has_any_role(user_id: str, db: AsyncSession, required: Sequence[str]) -> bool:
    if not required:
        return True
    names = await get_user_role_names(user_id, db)
    return bool(names.intersection(required))


def require_roles(*required_roles: str):
    async def _require(
        user: User = Depends(get_current_user),
        db: AsyncSession = Depends(get_db),
    ) -> User:
        if not await user_has_any_role(user.id, db, required_roles):
            raise HTTPException(status_code=403, detail="Insufficient role permissions")
        return user

    return _require


async def get_current_user(
    token: str = Depends(oauth2_scheme),
    db: AsyncSession = Depends(get_db),
) -> User:
    """FastAPI dependency: extract and validate the current user from the access token."""
    credentials_exception = HTTPException(
        status_code=status.HTTP_401_UNAUTHORIZED,
        detail="Could not validate credentials",
        headers={"WWW-Authenticate": "Bearer"},
    )
    try:
        payload = decode_access_token(token)
        user_id: Optional[str] = payload.get("sub")
        if user_id is None:
            raise credentials_exception
    except JWTError:
        raise credentials_exception

    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()
    if user is None or not user.is_active:
        raise credentials_exception
    return user


async def get_optional_user(
    token: Optional[str] = Depends(oauth2_scheme),
    db: AsyncSession = Depends(get_db),
) -> Optional[User]:
    """Like get_current_user but returns None instead of raising for unauthenticated requests."""
    if not token:
        return None
    try:
        return await get_current_user(token=token, db=db)
    except HTTPException:
        return None


def admin_api_key() -> Optional[str]:
    """The operator release key (``ADMIN_API_KEY`` env), or None if unset.

    Lets the model-catalog admin routes be driven without a user login — see
    ``require_admin_or_api_key``. Read at request time (not import) so it can be
    set/rotated without a code reload and monkeypatched in tests."""
    key = os.getenv("ADMIN_API_KEY")
    return key or None


def _bearer_token(authorization: Optional[str]) -> Optional[str]:
    if authorization and authorization[:7].lower() == "bearer ":
        return authorization[7:].strip() or None
    return None


async def require_admin_or_api_key(
    x_admin_key: Optional[str] = Header(default=None, alias="X-Admin-Key"),
    authorization: Optional[str] = Header(default=None),
    db: AsyncSession = Depends(get_db),
) -> Optional[User]:
    """Admin gate for operator routes (model catalog release workflow).

    Accepts a static ``ADMIN_API_KEY`` — via the ``X-Admin-Key`` header or
    ``Authorization: Bearer <key>`` — so model uploads/publishing need no user
    login; otherwise falls back to an admin-role JWT (unchanged behaviour).
    Returns the admin ``User`` for the JWT path, or ``None`` for the key path
    (routes recording an uploader must tolerate a null user).

    The key is a single operator secret, NOT open access: the catalog is
    downloaded + executed by every desktop client, so the endpoint stays gated.
    """
    key = admin_api_key()
    bearer = _bearer_token(authorization)
    if key is not None:
        for candidate in (x_admin_key, bearer):
            if candidate and hmac.compare_digest(candidate, key):
                return None  # authenticated via the operator API key
    # Fall back to an admin-role JWT.
    if not bearer:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Admin auth required: send X-Admin-Key (ADMIN_API_KEY) or an admin bearer token.",
            headers={"WWW-Authenticate": "Bearer"},
        )
    user = await get_current_user(token=bearer, db=db)
    if not await user_has_any_role(user.id, db, [ROLE_ADMIN]):
        raise HTTPException(status_code=403, detail="Insufficient role permissions")
    return user

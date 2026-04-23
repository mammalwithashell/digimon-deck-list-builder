# Tauri v2 Auto-Updater Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Tauri v2 auto-updater for the desktop alpha: hosted-API-managed releases, static JSON manifest in DO Spaces, Ed25519-signed installers, GitHub Actions release pipeline triggered by `desktop-v*` tags.

**Architecture:** DB-backed admin write surface (`/admin/releases/*` in `digimon_gym/db/routers/admin_releases.py`) regenerates a static `updates/<channel>/latest.json` object in DigitalOcean Spaces on every publish/unpublish. Tauri's `tauri-plugin-updater` reads that Spaces URL directly (API independence). Installers are signed with a project-owned Ed25519 key; the public key is baked into `tauri.conf.json`. CI uploads artifacts via presigned PUTs obtained from the hosted API — CI never holds Spaces credentials. Windows installers are self-signed (alpha-acceptable); Linux ships AppImage; macOS out of scope.

**Tech Stack:** FastAPI + SQLAlchemy (async) + Alembic + boto3 + moto[s3] (server); Tauri v2 + `tauri-plugin-updater` + reqwest (desktop); React + `@tauri-apps/plugin-updater` (frontend); GitHub Actions (CI).

**Spec:** [`docs/superpowers/specs/2026-04-21-tauri-auto-updater.md`](../specs/2026-04-21-tauri-auto-updater.md)

---

## File Structure

### New files

| Path | Responsibility |
|---|---|
| `alembic/versions/20260421_0015_app_releases.py` | Migration: create `app_releases` + `app_release_artifacts` tables, indexes, FKs, check constraints |
| `digimon_gym/db/routers/admin_releases.py` | `/admin/releases/*` endpoints + manifest-rewrite helper |
| `tests/api/test_admin_releases.py` | Router + moto-backed integration tests |
| `tools/provision_ci_release_user.py` | One-shot script to create the `ci-desktop-release` admin user + emit an API token |
| `tools/publish_release_smoke.py` | Local smoke test for the full publish flow (optional — runs against dev API) |
| `.github/workflows/desktop-release.yml` | GitHub Actions workflow (matrix build + publish job) |
| `docs/runbooks/desktop-release.md` | How to cut a release, roll back, rotate the updater key |
| `src-tauri/src/updater.rs` | Min-version guard + manifest pre-fetch on app setup |
| `frontend/src/updater/UpdaterBridge.tsx` | React component: subscribes to Tauri updater events, renders toast + modal |
| `frontend/src/updater/types.ts` | Frontend TS types for manifest shape |

### Modified files

| Path | Change |
|---|---|
| `digimon_gym/db/models.py` | Append `AppRelease` + `AppReleaseArtifact` ORM classes after `AIModel` (~line 1010) |
| `digimon_gym/db/schemas.py` | Append `AppRelease*` + `ReleaseManifest*` Pydantic schemas |
| `digimon_gym/storage/spaces.py` | Add `put_object(...)` (plus `put_json(...)` convenience wrapper) |
| `digimon_gym/api.py` | Mount `admin_releases_router` |
| `requirements.txt` | (verify `boto3`, `moto[s3]` already present from model-admin flow) |
| `src-tauri/Cargo.toml` | Add `tauri-plugin-updater = "2"` |
| `src-tauri/src/lib.rs` (or `main.rs`) | Register updater plugin + invoke `updater::check_min_version` in setup |
| `src-tauri/tauri.conf.json` | Add `plugins.updater` block (endpoints + pubkey + installMode) |
| `src-tauri/capabilities/*.json` | Add `updater:default` permission to main window capability |
| `frontend/package.json` | Add `@tauri-apps/plugin-updater` dep |
| `frontend/src/App.tsx` | Mount `<UpdaterBridge />` at root when `IS_DESKTOP` |

### Deferred (not in this plan)

- `frontend/src/pages/AdminReleasesPage.tsx` — admin UI (spec Q6: CI-only write path for now).
- macOS bundle targets (spec non-goal).
- Binary-patch / differential updates.
- Update telemetry beacon.

---

## Phase 1: Server surface (DB + routes + manifest rewrite)

### Task 1: Alembic migration — `app_releases` + `app_release_artifacts`

**Files:**
- Create: `alembic/versions/20260421_0015_app_releases.py`

- [ ] **Step 1: Write the migration**

```python
# alembic/versions/20260421_0015_app_releases.py
"""add app_releases and app_release_artifacts tables

Revision ID: 20260421_0015
Revises: 20260417_0014
Create Date: 2026-04-21
"""
from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260421_0015"
down_revision = "20260417_0014"
branch_labels = None
depends_on = None


def _has_table(table_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return table_name in inspector.get_table_names()


def upgrade() -> None:
    if not _has_table("app_releases"):
        op.create_table(
            "app_releases",
            sa.Column("id", sa.String(), primary_key=True),
            sa.Column("version", sa.String(), nullable=False),
            sa.Column("channel", sa.String(), nullable=False),
            sa.Column("engine_commit", sa.String(), nullable=False),
            sa.Column("min_version", sa.String(), nullable=False),
            sa.Column("release_notes", sa.Text(), nullable=False, server_default=""),
            sa.Column("published", sa.Boolean(), nullable=False, server_default="0"),
            sa.Column("published_at", sa.DateTime(timezone=True), nullable=True),
            sa.Column("state", sa.String(), nullable=False, server_default="pending"),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.CheckConstraint(
                "state IN ('pending', 'uploaded', 'failed')",
                name="ck_app_releases_state",
            ),
            sa.UniqueConstraint("channel", "version", name="uq_app_releases_channel_version"),
        )
        op.create_index("idx_app_releases_channel_pub", "app_releases", ["channel", "published"])

    if not _has_table("app_release_artifacts"):
        op.create_table(
            "app_release_artifacts",
            sa.Column("id", sa.String(), primary_key=True),
            sa.Column(
                "release_id",
                sa.String(),
                sa.ForeignKey("app_releases.id", ondelete="CASCADE"),
                nullable=False,
            ),
            sa.Column("target", sa.String(), nullable=False),
            sa.Column("spaces_key", sa.String(), nullable=False),
            sa.Column("filename", sa.String(), nullable=False),
            sa.Column("file_sha256", sa.String(), nullable=True),
            sa.Column("file_size_bytes", sa.Integer(), nullable=True),
            sa.Column("signature_b64", sa.Text(), nullable=True),
            sa.CheckConstraint(
                "target IN ('windows-x86_64', 'linux-x86_64')",
                name="ck_app_release_artifacts_target",
            ),
            sa.UniqueConstraint("release_id", "target", name="uq_app_release_artifacts_release_target"),
            sa.UniqueConstraint("spaces_key", name="uq_app_release_artifacts_spaces_key"),
        )
        op.create_index("idx_app_release_artifacts_release", "app_release_artifacts", ["release_id"])


def downgrade() -> None:
    if _has_table("app_release_artifacts"):
        op.drop_index("idx_app_release_artifacts_release", table_name="app_release_artifacts")
        op.drop_table("app_release_artifacts")
    if _has_table("app_releases"):
        op.drop_index("idx_app_releases_channel_pub", table_name="app_releases")
        op.drop_table("app_releases")
```

- [ ] **Step 2: Verify head chains correctly**

Run: `python -m alembic heads`
Expected: `20260421_0015 (head)` is the only head. If a merge is needed because another branch landed in the meantime, create a merge migration first.

- [ ] **Step 3: Apply + round-trip**

Run:
```bash
python -m alembic upgrade head
python -m alembic downgrade 20260417_0014
python -m alembic upgrade head
```
Expected: no errors. SQLite inspector shows `app_releases` and `app_release_artifacts` tables present after final upgrade.

- [ ] **Step 4: Commit**

```bash
git add alembic/versions/20260421_0015_app_releases.py
git commit -m "feat(migrations): add app_releases and app_release_artifacts tables"
```

---

### Task 2: ORM classes — `AppRelease` + `AppReleaseArtifact`

**Files:**
- Modify: `digimon_gym/db/models.py` (append after `class AIModel`, ~line 1010)

- [ ] **Step 1: Append ORM classes**

```python
# digimon_gym/db/models.py — append after AIModel (and its relationships)

# ── App Releases (Tauri auto-updater) ─────────────────────────────────────

class AppRelease(Base):
    __tablename__ = "app_releases"
    __table_args__ = (
        CheckConstraint(
            "state IN ('pending', 'uploaded', 'failed')",
            name="ck_app_releases_state",
        ),
        UniqueConstraint("channel", "version", name="uq_app_releases_channel_version"),
        Index("idx_app_releases_channel_pub", "channel", "published"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    version = Column(String, nullable=False)          # SemVer, e.g. "0.2.0-alpha.3"
    channel = Column(String, nullable=False)          # "alpha" for now
    engine_commit = Column(String, nullable=False)    # short git SHA from CI
    min_version = Column(String, nullable=False)      # SemVer floor for kill-switch
    release_notes = Column(Text, nullable=False, default="")
    published = Column(Boolean, nullable=False, default=False)
    published_at = Column(DateTime(timezone=True), nullable=True)
    state = Column(String, nullable=False, default="pending")  # 'pending'|'uploaded'|'failed'
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    updated_at = Column(DateTime(timezone=True), default=_utcnow, onupdate=_utcnow, nullable=False)

    artifacts = relationship(
        "AppReleaseArtifact",
        back_populates="release",
        cascade="all, delete-orphan",
        lazy="selectin",
    )


class AppReleaseArtifact(Base):
    __tablename__ = "app_release_artifacts"
    __table_args__ = (
        CheckConstraint(
            "target IN ('windows-x86_64', 'linux-x86_64')",
            name="ck_app_release_artifacts_target",
        ),
        UniqueConstraint("release_id", "target", name="uq_app_release_artifacts_release_target"),
        UniqueConstraint("spaces_key", name="uq_app_release_artifacts_spaces_key"),
        Index("idx_app_release_artifacts_release", "release_id"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    release_id = Column(String, ForeignKey("app_releases.id", ondelete="CASCADE"), nullable=False)
    target = Column(String, nullable=False)           # 'windows-x86_64' | 'linux-x86_64'
    spaces_key = Column(String, nullable=False)       # "releases/<release_id>/<filename>"
    filename = Column(String, nullable=False)
    file_sha256 = Column(String, nullable=True)       # set on confirm
    file_size_bytes = Column(Integer, nullable=True)  # set on confirm
    signature_b64 = Column(Text, nullable=True)       # base64 Ed25519 signature

    release = relationship("AppRelease", back_populates="artifacts")
```

- [ ] **Step 2: Verify import works**

Run:
```bash
python -c "from digimon_gym.db.models import AppRelease, AppReleaseArtifact; print(AppRelease.__table__.columns.keys())"
```
Expected: prints the full column list without ImportError.

- [ ] **Step 3: Commit**

```bash
git add digimon_gym/db/models.py
git commit -m "feat(db): AppRelease and AppReleaseArtifact ORM classes"
```

---

### Task 3: Pydantic schemas

**Files:**
- Modify: `digimon_gym/db/schemas.py` (append near end)

- [ ] **Step 1: Append schemas**

```python
# digimon_gym/db/schemas.py — append near end of file

# ── App Releases (Tauri auto-updater) ─────────────────────────────────────

from typing import Literal


class AppReleaseArtifactCreate(BaseModel):
    target: Literal["windows-x86_64", "linux-x86_64"]
    # filename is server-derived from version + target; not accepted on input


class AppReleaseCreateRequest(BaseModel):
    version: str                         # SemVer
    channel: Literal["alpha", "beta", "stable"] = "alpha"
    engine_commit: str
    min_version: str                     # SemVer floor
    release_notes: str = ""
    targets: list[Literal["windows-x86_64", "linux-x86_64"]]


class AppReleaseArtifactUploadSlot(BaseModel):
    target: str
    upload_url: str
    spaces_key: str
    filename: str
    expires_in: int


class AppReleaseCreateResponse(BaseModel):
    release_id: str
    version: str
    channel: str
    artifacts: list[AppReleaseArtifactUploadSlot]


class AppReleaseConfirmRequest(BaseModel):
    signature_b64: str                   # base64 Ed25519 signature from cargo tauri signer


class AppReleaseConfirmResponse(BaseModel):
    release_id: str
    target: str
    file_sha256: str
    file_size_bytes: int


class AppReleaseArtifactResponse(BaseModel):
    target: str
    spaces_key: str
    filename: str
    file_sha256: str | None
    file_size_bytes: int | None
    signature_b64: str | None

    class Config:
        from_attributes = True


class AppReleaseResponse(BaseModel):
    id: str
    version: str
    channel: str
    engine_commit: str
    min_version: str
    release_notes: str
    published: bool
    published_at: datetime | None
    state: str
    created_at: datetime
    updated_at: datetime
    artifacts: list[AppReleaseArtifactResponse]

    class Config:
        from_attributes = True


class ListAppReleasesResponse(BaseModel):
    releases: list[AppReleaseResponse]


class AppReleaseUpdateRequest(BaseModel):
    release_notes: str | None = None
    min_version: str | None = None


# ── Release manifest (consumed by Tauri's updater plugin) ─────────────────

class ReleaseManifestPlatform(BaseModel):
    signature: str
    url: str


class ReleaseManifest(BaseModel):
    """Shape served at updates/<channel>/latest.json in Spaces.

    Conforms to Tauri v2's updater manifest with project-specific extensions
    (min_version, engine_commit, channel, release_id)."""
    version: str
    pub_date: str                                              # ISO 8601 UTC
    notes: str
    platforms: dict[str, ReleaseManifestPlatform]              # keyed by target
    min_version: str
    engine_commit: str
    channel: str
    release_id: str


class UnpublishResponse(BaseModel):
    channel: str
    current_version: str | None


class RegenerateManifestResponse(BaseModel):
    channel: str
    current_version: str | None
    manifest: ReleaseManifest | None
```

Note: `BaseModel` and `datetime` are already imported at the top of `schemas.py`; only `Literal` needs to be added to the existing typing imports (check and add if absent).

- [ ] **Step 2: Verify schemas parse**

Run:
```bash
python -c "from digimon_gym.db.schemas import AppReleaseCreateRequest, ReleaseManifest; r = AppReleaseCreateRequest(version='0.2.0', engine_commit='fbf8288', min_version='0.1.0', targets=['windows-x86_64', 'linux-x86_64']); print(r.channel, r.targets)"
```
Expected: `alpha ['windows-x86_64', 'linux-x86_64']`

- [ ] **Step 3: Commit**

```bash
git add digimon_gym/db/schemas.py
git commit -m "feat(schemas): AppRelease and ReleaseManifest pydantic schemas"
```

---

### Task 4: Spaces wrapper — `put_object` / `put_json`

**Files:**
- Modify: `digimon_gym/storage/spaces.py`
- Test: `tests/storage/test_spaces_put_object.py`

- [ ] **Step 1: Write failing test**

```python
# tests/storage/test_spaces_put_object.py
from __future__ import annotations

import json
import os

import pytest

from digimon_gym.storage import spaces

moto = pytest.importorskip("moto")
from moto import mock_aws  # noqa: E402


@pytest.fixture
def moto_spaces(monkeypatch):
    monkeypatch.setenv("SPACES_ENDPOINT", "https://nyc3.digitaloceanspaces.com")
    monkeypatch.setenv("SPACES_BUCKET", "test-bucket")
    monkeypatch.setenv("SPACES_REGION", "us-east-1")
    monkeypatch.setenv("SPACES_KEY", "k")
    monkeypatch.setenv("SPACES_SECRET", "s")
    spaces._client.cache_clear()
    with mock_aws():
        import boto3
        boto3.client(
            "s3",
            endpoint_url=os.environ["SPACES_ENDPOINT"],
            region_name=os.environ["SPACES_REGION"],
            aws_access_key_id="k",
            aws_secret_access_key="s",
        ).create_bucket(Bucket="test-bucket")
        yield
    spaces._client.cache_clear()


def test_put_object_stores_bytes_with_headers(moto_spaces):
    spaces.put_object(
        key="updates/alpha/latest.json",
        body=b'{"hello": "world"}',
        content_type="application/json",
        cache_control="public, max-age=60",
        acl="public-read",
    )
    head = spaces.head_object("updates/alpha/latest.json")
    assert head["ContentLength"] == 18
    assert head["ContentType"] == "application/json"
    assert head["CacheControl"] == "public, max-age=60"


def test_put_json_serializes_dict(moto_spaces):
    spaces.put_json(
        key="updates/alpha/latest.json",
        data={"version": "0.2.0"},
        cache_max_age=60,
    )
    body = b"".join(spaces.iter_object_chunks("updates/alpha/latest.json"))
    assert json.loads(body) == {"version": "0.2.0"}
```

- [ ] **Step 2: Run test; confirm it fails**

Run: `python -m pytest tests/storage/test_spaces_put_object.py -v`
Expected: FAIL with `AttributeError: module 'digimon_gym.storage.spaces' has no attribute 'put_object'`.

- [ ] **Step 3: Implement `put_object` + `put_json`**

```python
# digimon_gym/storage/spaces.py — append after delete_object()

import json as _json


def put_object(
    key: str,
    body: bytes,
    content_type: str = "application/octet-stream",
    cache_control: str | None = None,
    acl: str | None = None,
) -> None:
    """Upload bytes to a Spaces object. Used for manifest JSON rewrites.

    Unlike presigned PUT, this path runs server-side and sets object-level
    metadata (Content-Type, Cache-Control, ACL) in one call.
    """
    extra: dict = {"Bucket": _bucket(), "Key": key, "Body": body, "ContentType": content_type}
    if cache_control is not None:
        extra["CacheControl"] = cache_control
    if acl is not None:
        extra["ACL"] = acl
    _client().put_object(**extra)


def put_json(key: str, data: dict, cache_max_age: int = 60) -> None:
    """Serialize ``data`` as JSON (sorted keys, UTF-8) and upload with a
    sensible Cache-Control header. Public-read ACL is applied so Tauri's
    updater can fetch anonymously."""
    body = _json.dumps(data, sort_keys=True, ensure_ascii=False).encode("utf-8")
    put_object(
        key=key,
        body=body,
        content_type="application/json",
        cache_control=f"public, max-age={cache_max_age}",
        acl="public-read",
    )
```

- [ ] **Step 4: Run test; confirm it passes**

Run: `python -m pytest tests/storage/test_spaces_put_object.py -v`
Expected: both tests PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon_gym/storage/spaces.py tests/storage/test_spaces_put_object.py
git commit -m "feat(storage): add put_object and put_json to spaces wrapper"
```

---

### Task 5: Router skeleton + `POST /admin/releases` (create)

**Files:**
- Create: `digimon_gym/db/routers/admin_releases.py`
- Test: `tests/api/test_admin_releases.py`

- [ ] **Step 1: Write the failing test**

```python
# tests/api/test_admin_releases.py
from __future__ import annotations

import os
import pytest
from httpx import AsyncClient, ASGITransport

from moto import mock_aws  # noqa: E402


@pytest.fixture
def spaces_env(monkeypatch):
    monkeypatch.setenv("SPACES_ENDPOINT", "https://nyc3.digitaloceanspaces.com")
    monkeypatch.setenv("SPACES_BUCKET", "test-bucket")
    monkeypatch.setenv("SPACES_REGION", "us-east-1")
    monkeypatch.setenv("SPACES_KEY", "k")
    monkeypatch.setenv("SPACES_SECRET", "s")
    from digimon_gym.storage import spaces
    spaces._client.cache_clear()
    yield
    spaces._client.cache_clear()


@pytest.fixture
async def client_with_admin(spaces_env, admin_auth_headers):
    """admin_auth_headers comes from tests/api/conftest.py (existing fixture
    used by test_admin_models.py)."""
    from digimon_gym.api import app
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as ac:
        yield ac, admin_auth_headers


@pytest.mark.asyncio
async def test_create_release_returns_presigned_urls_per_target(client_with_admin):
    client, headers = client_with_admin
    with mock_aws():
        import boto3
        boto3.client(
            "s3",
            endpoint_url=os.environ["SPACES_ENDPOINT"],
            region_name=os.environ["SPACES_REGION"],
            aws_access_key_id="k",
            aws_secret_access_key="s",
        ).create_bucket(Bucket="test-bucket")

        resp = await client.post(
            "/admin/releases",
            headers=headers,
            json={
                "version": "0.2.0-alpha.3",
                "channel": "alpha",
                "engine_commit": "fbf8288",
                "min_version": "0.1.0",
                "release_notes": "fix deckbuilder crash",
                "targets": ["windows-x86_64", "linux-x86_64"],
            },
        )
    assert resp.status_code == 201
    body = resp.json()
    assert "release_id" in body
    assert body["version"] == "0.2.0-alpha.3"
    assert len(body["artifacts"]) == 2
    targets = {a["target"] for a in body["artifacts"]}
    assert targets == {"windows-x86_64", "linux-x86_64"}
    for a in body["artifacts"]:
        assert a["upload_url"].startswith("https://")
        assert "X-Amz-Signature" in a["upload_url"]
        assert a["expires_in"] == 900


@pytest.mark.asyncio
async def test_create_release_rejects_non_admin(spaces_env, player_auth_headers):
    from digimon_gym.api import app
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as ac:
        resp = await ac.post(
            "/admin/releases",
            headers=player_auth_headers,
            json={
                "version": "0.2.0",
                "engine_commit": "abc",
                "min_version": "0.1.0",
                "targets": ["windows-x86_64"],
            },
        )
        assert resp.status_code == 403
```

- [ ] **Step 2: Run test; confirm it fails**

Run: `python -m pytest tests/api/test_admin_releases.py::test_create_release_returns_presigned_urls_per_target -v`
Expected: FAIL (likely 404: route not registered).

- [ ] **Step 3: Implement router + create endpoint**

```python
# digimon_gym/db/routers/admin_releases.py
"""Admin release management router for Tauri auto-updater.

Writes: /admin/releases/*  (all require ROLE_ADMIN)
The public read surface is the static manifest at
  <SPACES_CDN_URL or endpoint>/updates/<channel>/latest.json
which is regenerated by this module on publish/unpublish.
"""
from __future__ import annotations

from datetime import datetime, timezone

from botocore.exceptions import ClientError
from fastapi import APIRouter, Depends, HTTPException, Query, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.db.auth import ROLE_ADMIN, get_current_user, require_roles
from digimon_gym.db.database import get_db
from digimon_gym.db.models import AppRelease, AppReleaseArtifact, User
from digimon_gym.db.schemas import (
    AppReleaseArtifactUploadSlot,
    AppReleaseConfirmRequest,
    AppReleaseConfirmResponse,
    AppReleaseCreateRequest,
    AppReleaseCreateResponse,
    AppReleaseResponse,
    AppReleaseUpdateRequest,
    ListAppReleasesResponse,
    RegenerateManifestResponse,
    ReleaseManifest,
    ReleaseManifestPlatform,
    UnpublishResponse,
)
from digimon_gym.storage import spaces

admin_router = APIRouter(prefix="/admin/releases", tags=["admin-releases"])

UPLOAD_URL_TTL = 900  # seconds

# Spec'd in 2026-04-21-tauri-auto-updater.md §Manifest contract
_PRODUCT_SLUG = "digimon-tcg"
_TARGET_EXTENSION = {
    "windows-x86_64": "x86_64-setup.exe",
    "linux-x86_64": "x86_64.AppImage",
}


def _utcnow() -> datetime:
    return datetime.now(timezone.utc)


def _artifact_filename(version: str, target: str) -> str:
    ext = _TARGET_EXTENSION[target]
    return f"{_PRODUCT_SLUG}-{version}-{ext}"


def _artifact_spaces_key(release_id: str, filename: str) -> str:
    return f"releases/{release_id}/{filename}"


# ── Create ────────────────────────────────────────────────────────────────

@admin_router.post(
    "",
    response_model=AppReleaseCreateResponse,
    status_code=status.HTTP_201_CREATED,
)
async def create_release(
    request: AppReleaseCreateRequest,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
) -> AppReleaseCreateResponse:
    # Dupe check (UNIQUE (channel, version) enforces at the DB level, but
    # we want a clean 409 not a 500 on IntegrityError).
    existing = await db.scalar(
        select(AppRelease).where(
            AppRelease.channel == request.channel,
            AppRelease.version == request.version,
        )
    )
    if existing is not None:
        raise HTTPException(
            status_code=409,
            detail=f"release already exists for channel={request.channel} version={request.version}",
        )

    release = AppRelease(
        version=request.version,
        channel=request.channel,
        engine_commit=request.engine_commit,
        min_version=request.min_version,
        release_notes=request.release_notes,
        state="pending",
        published=False,
    )
    db.add(release)
    await db.flush()  # populate release.id

    slots: list[AppReleaseArtifactUploadSlot] = []
    for target in request.targets:
        filename = _artifact_filename(request.version, target)
        key = _artifact_spaces_key(release.id, filename)
        artifact = AppReleaseArtifact(
            release_id=release.id,
            target=target,
            spaces_key=key,
            filename=filename,
        )
        db.add(artifact)
        upload_url = spaces.generate_presigned_put(
            key=key,
            expires_in=UPLOAD_URL_TTL,
            content_type="application/octet-stream",
        )
        slots.append(
            AppReleaseArtifactUploadSlot(
                target=target,
                upload_url=upload_url,
                spaces_key=key,
                filename=filename,
                expires_in=UPLOAD_URL_TTL,
            )
        )

    await db.commit()

    return AppReleaseCreateResponse(
        release_id=release.id,
        version=release.version,
        channel=release.channel,
        artifacts=slots,
    )
```

- [ ] **Step 4: Mount router in api.py**

```python
# digimon_gym/api.py — near where admin_models is mounted (~line 73)
from digimon_gym.db.routers import admin_releases as admin_releases_router
# ...
app.include_router(admin_releases_router.admin_router)
```

- [ ] **Step 5: Run test; confirm it passes**

Run: `python -m pytest tests/api/test_admin_releases.py -v`
Expected: both tests PASS.

- [ ] **Step 6: Commit**

```bash
git add digimon_gym/db/routers/admin_releases.py digimon_gym/api.py tests/api/test_admin_releases.py
git commit -m "feat(api): POST /admin/releases with presigned-PUT slots per target"
```

---

### Task 6: `POST /admin/releases/{id}/artifacts/{target}/confirm`

**Files:**
- Modify: `digimon_gym/db/routers/admin_releases.py`
- Modify: `tests/api/test_admin_releases.py`

- [ ] **Step 1: Write failing test**

```python
# tests/api/test_admin_releases.py — append

@pytest.mark.asyncio
async def test_confirm_hashes_and_stores_signature(client_with_admin):
    client, headers = client_with_admin
    with mock_aws():
        import boto3
        s3 = boto3.client(
            "s3",
            endpoint_url=os.environ["SPACES_ENDPOINT"],
            region_name=os.environ["SPACES_REGION"],
            aws_access_key_id="k",
            aws_secret_access_key="s",
        )
        s3.create_bucket(Bucket="test-bucket")

        create = await client.post(
            "/admin/releases",
            headers=headers,
            json={
                "version": "0.2.0",
                "engine_commit": "abc1234",
                "min_version": "0.1.0",
                "targets": ["windows-x86_64"],
            },
        )
        body = create.json()
        release_id = body["release_id"]
        win_key = next(a["spaces_key"] for a in body["artifacts"] if a["target"] == "windows-x86_64")

        # Simulate CI uploading the installer bytes
        s3.put_object(Bucket="test-bucket", Key=win_key, Body=b"fake-installer-bytes")

        confirm = await client.post(
            f"/admin/releases/{release_id}/artifacts/windows-x86_64/confirm",
            headers=headers,
            json={"signature_b64": "c29tZS1zaWduYXR1cmU="},  # base64("some-signature")
        )
    assert confirm.status_code == 200
    cbody = confirm.json()
    assert cbody["target"] == "windows-x86_64"
    assert cbody["file_size_bytes"] == len(b"fake-installer-bytes")
    # sha256("fake-installer-bytes")
    import hashlib
    assert cbody["file_sha256"] == hashlib.sha256(b"fake-installer-bytes").hexdigest()


@pytest.mark.asyncio
async def test_confirm_unknown_target_returns_404(client_with_admin):
    client, headers = client_with_admin
    with mock_aws():
        import boto3
        boto3.client("s3", endpoint_url=os.environ["SPACES_ENDPOINT"],
                     region_name=os.environ["SPACES_REGION"],
                     aws_access_key_id="k", aws_secret_access_key="s"
                     ).create_bucket(Bucket="test-bucket")
        create = await client.post(
            "/admin/releases",
            headers=headers,
            json={"version": "0.3.0", "engine_commit": "x", "min_version": "0.1.0",
                  "targets": ["windows-x86_64"]},
        )
        rid = create.json()["release_id"]
        resp = await client.post(
            f"/admin/releases/{rid}/artifacts/linux-x86_64/confirm",
            headers=headers,
            json={"signature_b64": "xx"},
        )
    assert resp.status_code == 404
```

- [ ] **Step 2: Run tests; confirm new ones fail**

Run: `python -m pytest tests/api/test_admin_releases.py -v`
Expected: the two new tests FAIL with 404 or similar.

- [ ] **Step 3: Implement the confirm endpoint**

```python
# digimon_gym/db/routers/admin_releases.py — append after create_release

@admin_router.post(
    "/{release_id}/artifacts/{target}/confirm",
    response_model=AppReleaseConfirmResponse,
)
async def confirm_artifact(
    release_id: str,
    target: str,
    request: AppReleaseConfirmRequest,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
) -> AppReleaseConfirmResponse:
    artifact = await db.scalar(
        select(AppReleaseArtifact).where(
            AppReleaseArtifact.release_id == release_id,
            AppReleaseArtifact.target == target,
        )
    )
    if artifact is None:
        raise HTTPException(status_code=404, detail="artifact not found")

    # HEAD first so we 422 cleanly on missing upload rather than erroring inside
    # the streaming hash.
    try:
        spaces.head_object(artifact.spaces_key)
    except ClientError as e:
        raise HTTPException(
            status_code=422,
            detail=f"Spaces object {artifact.spaces_key} not present (CI upload failed?): {e}",
        )

    try:
        sha256_hex, total_bytes = spaces.stream_sha256(artifact.spaces_key)
    except ClientError as e:
        raise HTTPException(status_code=422, detail=f"stream_sha256 failed: {e}")

    artifact.file_sha256 = sha256_hex
    artifact.file_size_bytes = total_bytes
    artifact.signature_b64 = request.signature_b64
    await db.commit()

    return AppReleaseConfirmResponse(
        release_id=release_id,
        target=target,
        file_sha256=sha256_hex,
        file_size_bytes=total_bytes,
    )
```

- [ ] **Step 4: Run tests; confirm they pass**

Run: `python -m pytest tests/api/test_admin_releases.py -v`
Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon_gym/db/routers/admin_releases.py tests/api/test_admin_releases.py
git commit -m "feat(api): confirm endpoint streams sha256 and stores signature"
```

---

### Task 7: Manifest-rewrite helper + `POST /publish`

**Files:**
- Modify: `digimon_gym/db/routers/admin_releases.py`
- Modify: `tests/api/test_admin_releases.py`

- [ ] **Step 1: Write failing test**

```python
# tests/api/test_admin_releases.py — append

@pytest.mark.asyncio
async def test_publish_writes_manifest_to_spaces(client_with_admin):
    client, headers = client_with_admin
    with mock_aws():
        import boto3, hashlib, json
        s3 = boto3.client("s3", endpoint_url=os.environ["SPACES_ENDPOINT"],
                          region_name=os.environ["SPACES_REGION"],
                          aws_access_key_id="k", aws_secret_access_key="s")
        s3.create_bucket(Bucket="test-bucket")

        create = await client.post(
            "/admin/releases",
            headers=headers,
            json={
                "version": "0.2.0",
                "engine_commit": "abc1234",
                "min_version": "0.1.0",
                "release_notes": "first alpha",
                "targets": ["windows-x86_64", "linux-x86_64"],
            },
        )
        body = create.json()
        rid = body["release_id"]

        for art in body["artifacts"]:
            s3.put_object(Bucket="test-bucket", Key=art["spaces_key"], Body=b"x" * 1024)
            await client.post(
                f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                headers=headers,
                json={"signature_b64": f"sig-{art['target']}"},
            )

        pub = await client.post(f"/admin/releases/{rid}/publish", headers=headers)
        assert pub.status_code == 200

        manifest_obj = s3.get_object(Bucket="test-bucket", Key="updates/alpha/latest.json")
        manifest = json.loads(manifest_obj["Body"].read())

    assert manifest["version"] == "0.2.0"
    assert manifest["channel"] == "alpha"
    assert manifest["release_id"] == rid
    assert manifest["engine_commit"] == "abc1234"
    assert manifest["min_version"] == "0.1.0"
    assert manifest["notes"] == "first alpha"
    assert set(manifest["platforms"].keys()) == {"windows-x86_64", "linux-x86_64"}
    for target, plat in manifest["platforms"].items():
        assert plat["signature"] == f"sig-{target}"
        assert plat["url"].endswith(f"/updates/alpha/latest.json") is False  # it's an installer URL
        assert f"releases/{rid}" in plat["url"]


@pytest.mark.asyncio
async def test_publish_refuses_if_artifact_unconfirmed(client_with_admin):
    client, headers = client_with_admin
    with mock_aws():
        import boto3
        boto3.client("s3", endpoint_url=os.environ["SPACES_ENDPOINT"],
                     region_name=os.environ["SPACES_REGION"],
                     aws_access_key_id="k", aws_secret_access_key="s"
                     ).create_bucket(Bucket="test-bucket")
        create = await client.post(
            "/admin/releases",
            headers=headers,
            json={"version": "0.2.1", "engine_commit": "x", "min_version": "0.1.0",
                  "targets": ["windows-x86_64"]},
        )
        rid = create.json()["release_id"]
        # No upload / confirm — fast-path to publish
        pub = await client.post(f"/admin/releases/{rid}/publish", headers=headers)
    assert pub.status_code == 409


@pytest.mark.asyncio
async def test_publish_unpublishes_previous_on_same_channel(client_with_admin):
    client, headers = client_with_admin
    with mock_aws():
        import boto3
        s3 = boto3.client("s3", endpoint_url=os.environ["SPACES_ENDPOINT"],
                          region_name=os.environ["SPACES_REGION"],
                          aws_access_key_id="k", aws_secret_access_key="s")
        s3.create_bucket(Bucket="test-bucket")

        async def _cut(version):
            create = await client.post(
                "/admin/releases",
                headers=headers,
                json={"version": version, "engine_commit": "e", "min_version": "0.1.0",
                      "targets": ["windows-x86_64"]},
            )
            body = create.json()
            rid = body["release_id"]
            for art in body["artifacts"]:
                s3.put_object(Bucket="test-bucket", Key=art["spaces_key"], Body=b"x")
                await client.post(
                    f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                    headers=headers, json={"signature_b64": "sig"},
                )
            await client.post(f"/admin/releases/{rid}/publish", headers=headers)
            return rid

        first = await _cut("0.2.0")
        second = await _cut("0.2.1")

        listing = await client.get("/admin/releases?channel=alpha", headers=headers)
    rows = listing.json()["releases"]
    by_id = {r["id"]: r for r in rows}
    assert by_id[first]["published"] is False
    assert by_id[second]["published"] is True
```

- [ ] **Step 2: Run tests; confirm they fail**

Run: `python -m pytest tests/api/test_admin_releases.py -v`
Expected: three new tests FAIL (no `/publish` route, no `GET /admin/releases`).

- [ ] **Step 3: Implement helper + publish + list**

```python
# digimon_gym/db/routers/admin_releases.py — append

async def _build_manifest(db: AsyncSession, release: AppRelease) -> dict:
    """Serialize a release + its artifacts into the Tauri-compatible
    manifest JSON shape defined in the spec."""
    # Ensure artifacts are loaded (selectin lazy should have done this, but
    # be explicit for safety when called from long transactions).
    platforms: dict[str, dict] = {}
    for art in release.artifacts:
        if art.file_sha256 is None or art.signature_b64 is None:
            raise HTTPException(
                status_code=409,
                detail=f"artifact {art.target} is unconfirmed; confirm it before publish",
            )
        platforms[art.target] = {
            "signature": art.signature_b64,
            "url": spaces.public_url(art.spaces_key),
        }
    return {
        "version": release.version,
        "pub_date": (release.published_at or _utcnow()).isoformat().replace("+00:00", "Z"),
        "notes": release.release_notes,
        "platforms": platforms,
        "min_version": release.min_version,
        "engine_commit": release.engine_commit,
        "channel": release.channel,
        "release_id": release.id,
    }


def _manifest_key(channel: str) -> str:
    return f"updates/{channel}/latest.json"


async def _rewrite_channel_manifest(db: AsyncSession, channel: str) -> dict | None:
    """Regenerate updates/<channel>/latest.json from the newest published
    release on that channel. Returns the manifest dict, or None (and deletes
    the Spaces object) if no published release exists.
    """
    release = await db.scalar(
        select(AppRelease)
        .where(AppRelease.channel == channel, AppRelease.published == True)  # noqa: E712
        .order_by(AppRelease.published_at.desc())
        .limit(1)
    )
    key = _manifest_key(channel)
    if release is None:
        try:
            spaces.delete_object(key)
        except ClientError:
            pass  # 404 is fine
        return None
    manifest = await _build_manifest(db, release)
    spaces.put_json(key, manifest, cache_max_age=60)
    return manifest


@admin_router.post(
    "/{release_id}/publish",
    response_model=AppReleaseResponse,
)
async def publish_release(
    release_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
) -> AppReleaseResponse:
    release = await db.get(AppRelease, release_id)
    if release is None:
        raise HTTPException(status_code=404, detail="release not found")

    # Precondition: every declared artifact has sha256 + signature populated.
    for art in release.artifacts:
        if art.file_sha256 is None or art.signature_b64 is None:
            raise HTTPException(
                status_code=409,
                detail=f"artifact {art.target} is unconfirmed",
            )

    # Unpublish any other row on this channel.
    other = await db.scalars(
        select(AppRelease).where(
            AppRelease.channel == release.channel,
            AppRelease.published == True,  # noqa: E712
            AppRelease.id != release.id,
        )
    )
    for row in other.all():
        row.published = False

    release.state = "uploaded"
    release.published = True
    release.published_at = _utcnow()
    await db.flush()

    await _rewrite_channel_manifest(db, release.channel)
    await db.commit()
    await db.refresh(release)
    return AppReleaseResponse.model_validate(release)


@admin_router.get(
    "",
    response_model=ListAppReleasesResponse,
)
async def list_releases(
    channel: str | None = Query(default=None),
    published: bool | None = Query(default=None),
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
) -> ListAppReleasesResponse:
    stmt = select(AppRelease).order_by(AppRelease.created_at.desc())
    if channel is not None:
        stmt = stmt.where(AppRelease.channel == channel)
    if published is not None:
        stmt = stmt.where(AppRelease.published == published)
    rows = (await db.scalars(stmt)).all()
    return ListAppReleasesResponse(
        releases=[AppReleaseResponse.model_validate(r) for r in rows]
    )
```

- [ ] **Step 4: Run tests; confirm they pass**

Run: `python -m pytest tests/api/test_admin_releases.py -v`
Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon_gym/db/routers/admin_releases.py tests/api/test_admin_releases.py
git commit -m "feat(api): publish release endpoint + manifest rewrite"
```

---

### Task 8: Unpublish, PATCH, DELETE, regenerate-manifest

**Files:**
- Modify: `digimon_gym/db/routers/admin_releases.py`
- Modify: `tests/api/test_admin_releases.py`

- [ ] **Step 1: Write failing tests**

```python
# tests/api/test_admin_releases.py — append

@pytest.mark.asyncio
async def test_unpublish_rewrites_manifest_to_previous(client_with_admin):
    client, headers = client_with_admin
    with mock_aws():
        import boto3, json
        s3 = boto3.client("s3", endpoint_url=os.environ["SPACES_ENDPOINT"],
                          region_name=os.environ["SPACES_REGION"],
                          aws_access_key_id="k", aws_secret_access_key="s")
        s3.create_bucket(Bucket="test-bucket")

        async def _cut(version):
            create = await client.post(
                "/admin/releases", headers=headers,
                json={"version": version, "engine_commit": "e", "min_version": "0.1.0",
                      "targets": ["windows-x86_64"]},
            )
            body = create.json()
            rid = body["release_id"]
            for art in body["artifacts"]:
                s3.put_object(Bucket="test-bucket", Key=art["spaces_key"], Body=b"x")
                await client.post(
                    f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                    headers=headers, json={"signature_b64": f"sig-{version}"},
                )
            await client.post(f"/admin/releases/{rid}/publish", headers=headers)
            return rid

        first = await _cut("0.2.0")
        second = await _cut("0.2.1")

        unpub = await client.post(f"/admin/releases/{second}/unpublish", headers=headers)
        assert unpub.status_code == 200
        assert unpub.json()["current_version"] == "0.2.0"

        manifest = json.loads(
            s3.get_object(Bucket="test-bucket", Key="updates/alpha/latest.json")["Body"].read()
        )
    assert manifest["version"] == "0.2.0"


@pytest.mark.asyncio
async def test_unpublish_only_release_deletes_manifest(client_with_admin):
    client, headers = client_with_admin
    with mock_aws():
        import boto3
        s3 = boto3.client("s3", endpoint_url=os.environ["SPACES_ENDPOINT"],
                          region_name=os.environ["SPACES_REGION"],
                          aws_access_key_id="k", aws_secret_access_key="s")
        s3.create_bucket(Bucket="test-bucket")

        create = await client.post(
            "/admin/releases", headers=headers,
            json={"version": "0.2.0", "engine_commit": "e", "min_version": "0.1.0",
                  "targets": ["windows-x86_64"]},
        )
        body = create.json()
        rid = body["release_id"]
        for art in body["artifacts"]:
            s3.put_object(Bucket="test-bucket", Key=art["spaces_key"], Body=b"x")
            await client.post(
                f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                headers=headers, json={"signature_b64": "s"},
            )
        await client.post(f"/admin/releases/{rid}/publish", headers=headers)

        unpub = await client.post(f"/admin/releases/{rid}/unpublish", headers=headers)
        assert unpub.status_code == 200
        assert unpub.json()["current_version"] is None

        import botocore.exceptions
        with pytest.raises(botocore.exceptions.ClientError):
            s3.get_object(Bucket="test-bucket", Key="updates/alpha/latest.json")


@pytest.mark.asyncio
async def test_patch_release_notes_rewrites_manifest_if_published(client_with_admin):
    client, headers = client_with_admin
    with mock_aws():
        import boto3, json
        s3 = boto3.client("s3", endpoint_url=os.environ["SPACES_ENDPOINT"],
                          region_name=os.environ["SPACES_REGION"],
                          aws_access_key_id="k", aws_secret_access_key="s")
        s3.create_bucket(Bucket="test-bucket")

        create = await client.post(
            "/admin/releases", headers=headers,
            json={"version": "0.2.0", "engine_commit": "e", "min_version": "0.1.0",
                  "release_notes": "typo", "targets": ["windows-x86_64"]},
        )
        body = create.json()
        rid = body["release_id"]
        for art in body["artifacts"]:
            s3.put_object(Bucket="test-bucket", Key=art["spaces_key"], Body=b"x")
            await client.post(
                f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                headers=headers, json={"signature_b64": "s"},
            )
        await client.post(f"/admin/releases/{rid}/publish", headers=headers)

        patch = await client.patch(
            f"/admin/releases/{rid}",
            headers=headers,
            json={"release_notes": "fixed typo"},
        )
        assert patch.status_code == 200

        manifest = json.loads(
            s3.get_object(Bucket="test-bucket", Key="updates/alpha/latest.json")["Body"].read()
        )
    assert manifest["notes"] == "fixed typo"


@pytest.mark.asyncio
async def test_delete_refuses_published(client_with_admin):
    client, headers = client_with_admin
    with mock_aws():
        import boto3
        s3 = boto3.client("s3", endpoint_url=os.environ["SPACES_ENDPOINT"],
                          region_name=os.environ["SPACES_REGION"],
                          aws_access_key_id="k", aws_secret_access_key="s")
        s3.create_bucket(Bucket="test-bucket")
        create = await client.post(
            "/admin/releases", headers=headers,
            json={"version": "0.2.0", "engine_commit": "e", "min_version": "0.1.0",
                  "targets": ["windows-x86_64"]},
        )
        rid = create.json()["release_id"]
        for art in create.json()["artifacts"]:
            s3.put_object(Bucket="test-bucket", Key=art["spaces_key"], Body=b"x")
            await client.post(
                f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                headers=headers, json={"signature_b64": "s"},
            )
        await client.post(f"/admin/releases/{rid}/publish", headers=headers)
        del_resp = await client.delete(f"/admin/releases/{rid}", headers=headers)
    assert del_resp.status_code == 409


@pytest.mark.asyncio
async def test_regenerate_manifest_matches_current_published(client_with_admin):
    client, headers = client_with_admin
    with mock_aws():
        import boto3, json
        s3 = boto3.client("s3", endpoint_url=os.environ["SPACES_ENDPOINT"],
                          region_name=os.environ["SPACES_REGION"],
                          aws_access_key_id="k", aws_secret_access_key="s")
        s3.create_bucket(Bucket="test-bucket")
        create = await client.post(
            "/admin/releases", headers=headers,
            json={"version": "0.2.0", "engine_commit": "e", "min_version": "0.1.0",
                  "targets": ["windows-x86_64"]},
        )
        rid = create.json()["release_id"]
        for art in create.json()["artifacts"]:
            s3.put_object(Bucket="test-bucket", Key=art["spaces_key"], Body=b"x")
            await client.post(
                f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                headers=headers, json={"signature_b64": "s"},
            )
        await client.post(f"/admin/releases/{rid}/publish", headers=headers)

        # Nuke the manifest manually, then call regenerate
        s3.delete_object(Bucket="test-bucket", Key="updates/alpha/latest.json")
        regen = await client.post(
            "/admin/releases/regenerate-manifest?channel=alpha",
            headers=headers,
        )
    assert regen.status_code == 200
    assert regen.json()["current_version"] == "0.2.0"
```

- [ ] **Step 2: Run tests; confirm they fail**

Run: `python -m pytest tests/api/test_admin_releases.py -v`
Expected: the five new tests FAIL.

- [ ] **Step 3: Implement endpoints**

```python
# digimon_gym/db/routers/admin_releases.py — append

@admin_router.post(
    "/{release_id}/unpublish",
    response_model=UnpublishResponse,
)
async def unpublish_release(
    release_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
) -> UnpublishResponse:
    release = await db.get(AppRelease, release_id)
    if release is None:
        raise HTTPException(status_code=404, detail="release not found")
    release.published = False
    await db.flush()
    manifest = await _rewrite_channel_manifest(db, release.channel)
    await db.commit()
    return UnpublishResponse(
        channel=release.channel,
        current_version=manifest["version"] if manifest else None,
    )


@admin_router.patch(
    "/{release_id}",
    response_model=AppReleaseResponse,
)
async def update_release(
    release_id: str,
    request: AppReleaseUpdateRequest,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
) -> AppReleaseResponse:
    release = await db.get(AppRelease, release_id)
    if release is None:
        raise HTTPException(status_code=404, detail="release not found")
    if request.release_notes is not None:
        release.release_notes = request.release_notes
    if request.min_version is not None:
        release.min_version = request.min_version
    await db.flush()
    if release.published:
        await _rewrite_channel_manifest(db, release.channel)
    await db.commit()
    await db.refresh(release)
    return AppReleaseResponse.model_validate(release)


@admin_router.delete(
    "/{release_id}",
    status_code=status.HTTP_204_NO_CONTENT,
)
async def delete_release(
    release_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    release = await db.get(AppRelease, release_id)
    if release is None:
        raise HTTPException(status_code=404, detail="release not found")
    if release.published:
        raise HTTPException(
            status_code=409,
            detail="release is published; unpublish before delete",
        )
    # Best-effort cleanup of Spaces artifacts
    for art in release.artifacts:
        try:
            spaces.delete_object(art.spaces_key)
        except ClientError:
            pass
    await db.delete(release)
    await db.commit()


@admin_router.post(
    "/regenerate-manifest",
    response_model=RegenerateManifestResponse,
)
async def regenerate_manifest(
    channel: str = Query(...),
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
) -> RegenerateManifestResponse:
    manifest = await _rewrite_channel_manifest(db, channel)
    return RegenerateManifestResponse(
        channel=channel,
        current_version=manifest["version"] if manifest else None,
        manifest=ReleaseManifest.model_validate(manifest) if manifest else None,
    )
```

- [ ] **Step 4: Run tests; confirm they pass**

Run: `python -m pytest tests/api/test_admin_releases.py -v`
Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon_gym/db/routers/admin_releases.py tests/api/test_admin_releases.py
git commit -m "feat(api): unpublish, patch, delete, regenerate-manifest"
```

---

## Phase 2: Tauri desktop integration

### Task 9: Generate Ed25519 updater key + document custody

**Files:**
- Create: `docs/runbooks/desktop-release.md` (new file; scaffolds the runbook — fuller content in Task 16)

- [ ] **Step 1: Generate the key locally**

Run:
```bash
mkdir -p ~/.tauri
cargo install tauri-cli --version "^2" --locked   # if not already installed
cargo tauri signer generate -w ~/.tauri/digimon-updater.key
```
Expected: prompts for a password; outputs a private key file at `~/.tauri/digimon-updater.key` and prints the base64-encoded **public** key to stdout (and writes `.pub` alongside).

- [ ] **Step 2: Record pubkey**

Copy the printed public key string. Save it to a scratch file temporarily — it goes into `tauri.conf.json` in Task 10 and into the runbook in Task 16.

- [ ] **Step 3: Scaffold runbook**

Create `docs/runbooks/desktop-release.md` with this minimal content (fleshed out in Task 16):

```markdown
# Desktop Release Runbook

## Updater key custody

- Private key file: `~/.tauri/digimon-updater.key` (password-encrypted, lives on maintainer machine only)
- Password: stored in 1Password vault "Digimon TCG" entry "Tauri Updater Key"
- Public key: committed in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`
- CI secrets: `TAURI_UPDATER_PRIVATE_KEY` (file contents), `TAURI_UPDATER_KEY_PASSWORD` (password)

## Key rotation

Rotation invalidates every deployed alpha build (Tauri verifies against the
baked-in pubkey, which can only change in a new native binary). Procedure:
1. Generate a new key: `cargo tauri signer generate -w ~/.tauri/digimon-updater-v2.key`.
2. Update `src-tauri/tauri.conf.json` `plugins.updater.pubkey`.
3. Update GHA secrets to the new private key + password.
4. Cut a new release (e.g. `desktop-v0.3.0`). CI builds and signs with v2.
5. Email alpha testers: "Please download the new installer manually" — the
   old pubkey-verifying apps cannot update to the v2-signed binary.

## Cut a release (full flow)

(Filled in by Task 16.)

## Roll back

(Filled in by Task 16.)
```

- [ ] **Step 4: Add CI secrets via GitHub CLI**

Run (reads the key file and password from the local machine):
```bash
gh secret set TAURI_UPDATER_PRIVATE_KEY < ~/.tauri/digimon-updater.key
# Password prompt — paste the password manually:
gh secret set TAURI_UPDATER_KEY_PASSWORD
```
Expected: `✓ Set secret TAURI_UPDATER_PRIVATE_KEY for <repo>` and similar for the password.

- [ ] **Step 5: Commit runbook scaffold**

```bash
git add docs/runbooks/desktop-release.md
git commit -m "docs: scaffold desktop-release runbook with key custody section"
```

---

### Task 10: `tauri.conf.json` + capability + Cargo deps

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/capabilities/default.json` (or whichever capability file grants main window permissions — check with `ls src-tauri/capabilities/`)
- Modify: `frontend/package.json`

- [ ] **Step 1: Add plugin to Cargo.toml**

```toml
# src-tauri/Cargo.toml — in [dependencies] section
tauri-plugin-updater = "2"
```

- [ ] **Step 2: Add JS plugin to frontend**

Run: `cd frontend && npm install @tauri-apps/plugin-updater`

- [ ] **Step 3: Add `plugins.updater` block to tauri.conf.json**

Paste the Ed25519 public key from Task 9 as the value of `pubkey`.

```json
// src-tauri/tauri.conf.json — add as sibling of "app" / "bundle"
"plugins": {
  "updater": {
    "active": true,
    "endpoints": [
      "https://digimon-tcg-releases.nyc3.cdn.digitaloceanspaces.com/updates/alpha/latest.json"
    ],
    "pubkey": "<PASTE-ED25519-PUBLIC-KEY-HERE>",
    "windows": {
      "installMode": "passive"
    }
  }
}
```

Note: the Spaces URL in `endpoints` must exactly match what the server will write (Task 7's `spaces.public_url(_manifest_key(channel))`). Verify at implementation time by curling `/admin/releases/regenerate-manifest?channel=alpha` against a dev API and reading the printed `manifest["platforms"][...]["url"]` host — use the same host for the updater endpoint.

- [ ] **Step 4: Grant updater capability**

Inspect existing capability files:
```bash
ls src-tauri/capabilities/
```
In the capability JSON that applies to the main window, add `"updater:default"` to the `permissions` array. Example edit:

```json
{
  "identifier": "main-capability",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "updater:default"
  ]
}
```

- [ ] **Step 5: Verify build compiles**

Run: `cd src-tauri && cargo check`
Expected: zero errors. A warning about unused dependency is acceptable; the plugin is wired in Task 11.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/tauri.conf.json src-tauri/capabilities/*.json frontend/package.json frontend/package-lock.json
git commit -m "feat(desktop): add tauri-plugin-updater dep and config"
```

---

### Task 11: Rust-side updater wiring + min-version guard

**Files:**
- Create: `src-tauri/src/updater.rs`
- Modify: `src-tauri/src/lib.rs` (or `main.rs` — whichever registers plugins)

- [ ] **Step 1: Locate the plugin-registration site**

Run: `grep -rn "tauri::Builder\|\.plugin(" src-tauri/src/`
Expected: find the `tauri::Builder::default().plugin(...)` chain. Note the filename (commonly `lib.rs` or `main.rs`).

- [ ] **Step 2: Write `updater.rs`**

```rust
// src-tauri/src/updater.rs
//! Min-version guard: fetch the channel manifest on startup and, if the
//! running app's version is below the manifest's `min_version`, emit a
//! `updater:force-update` event so the frontend can render a blocking modal.
//!
//! This is separate from Tauri's own updater check — we want the min-version
//! decision made *before* any normal update prompt, because the whole point
//! is "the user cannot continue on this version."

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};

const MANIFEST_URL: &str = "https://digimon-tcg-releases.nyc3.cdn.digitaloceanspaces.com/updates/alpha/latest.json";

#[derive(Debug, Deserialize)]
struct MinVersionPeek {
    min_version: String,
    version: String,
}

/// Spawn the min-version check in a background Tokio task.
/// Failure modes (network error, bad JSON, missing file) are logged and
/// ignored — we never block the user on a transient failure.
pub fn spawn_min_version_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = check_min_version(&app).await {
            log::warn!("min-version check failed (ignoring): {e}");
        }
    });
}

async fn check_min_version(app: &AppHandle) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()?;
    let resp = client.get(MANIFEST_URL).send().await?;
    if !resp.status().is_success() {
        return Err(format!("manifest HTTP {}", resp.status()).into());
    }
    let peek: MinVersionPeek = resp.json().await?;

    let running = app.package_info().version.to_string();
    if version_lt(&running, &peek.min_version) {
        log::warn!(
            "running version {} is below manifest min_version {} — forcing update to {}",
            running, peek.min_version, peek.version
        );
        app.emit("updater:force-update", &peek)?;
    }
    Ok(())
}

/// SemVer-ish comparison. Uses the `semver` crate for correctness since
/// manifest versions may have prerelease suffixes like `-alpha.3`.
fn version_lt(a: &str, b: &str) -> bool {
    match (semver::Version::parse(a), semver::Version::parse(b)) {
        (Ok(va), Ok(vb)) => va < vb,
        _ => false,  // if either side is unparseable, don't force-update
    }
}

#[cfg(test)]
mod tests {
    use super::version_lt;

    #[test]
    fn prerelease_ordering() {
        assert!(version_lt("0.2.0-alpha.2", "0.2.0-alpha.3"));
        assert!(version_lt("0.2.0-alpha.3", "0.2.0"));
        assert!(!version_lt("0.2.0", "0.2.0"));
        assert!(!version_lt("0.3.0", "0.2.0"));
    }

    #[test]
    fn unparseable_does_not_force() {
        assert!(!version_lt("not-a-version", "0.2.0"));
        assert!(!version_lt("0.2.0", "not-a-version"));
    }
}
```

- [ ] **Step 3: Register plugin + invoke guard in the builder**

Edit the Tauri builder site (likely `src-tauri/src/lib.rs`):

```rust
// src-tauri/src/lib.rs — inside the pub fn run() or equivalent

mod updater;

pub fn run() {
    tauri::Builder::default()
        // ... existing plugins ...
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // ... existing setup ...
            let handle = app.handle().clone();
            updater::spawn_min_version_check(handle);
            Ok(())
        })
        // ... existing invoke_handler etc ...
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Add `semver` + `log` to Cargo.toml if absent**

```toml
# src-tauri/Cargo.toml — under [dependencies]
semver = "1"
log = "0.4"
```

- [ ] **Step 5: Run cargo tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: the two new `version_lt` tests PASS along with all existing Tauri-layer tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/updater.rs src-tauri/src/lib.rs
git commit -m "feat(desktop): min-version guard + updater plugin registration"
```

---

### Task 12: Frontend updater bridge (toast + modal)

**Files:**
- Create: `frontend/src/updater/types.ts`
- Create: `frontend/src/updater/UpdaterBridge.tsx`
- Modify: `frontend/src/App.tsx`

- [ ] **Step 1: Define TS types**

```typescript
// frontend/src/updater/types.ts
export interface ForceUpdatePayload {
  min_version: string;
  version: string;
}
```

- [ ] **Step 2: Implement UpdaterBridge**

```typescript
// frontend/src/updater/UpdaterBridge.tsx
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type { ForceUpdatePayload } from "./types";

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === "desktop";

type AvailableUpdate = {
  version: string;
  date: string | null;
  body: string | null;
};

export function UpdaterBridge() {
  const [forced, setForced] = useState<ForceUpdatePayload | null>(null);
  const [available, setAvailable] = useState<AvailableUpdate | null>(null);
  const [installing, setInstalling] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    if (!IS_DESKTOP) return;

    // 1. Listen for Rust-side force-update signal (min_version guard)
    const unlistenForce = listen<ForceUpdatePayload>("updater:force-update", (e) => {
      setForced(e.payload);
      setModalOpen(true);
    });

    // 2. Normal background check
    (async () => {
      try {
        const update = await check();
        if (update) {
          setAvailable({
            version: update.version,
            date: update.date ?? null,
            body: update.body ?? null,
          });
        }
      } catch (err) {
        console.warn("updater check failed:", err);
      }
    })();

    return () => {
      unlistenForce.then((fn) => fn());
    };
  }, []);

  async function applyUpdate() {
    setInstalling(true);
    try {
      const update = await check();
      if (!update) {
        // Manifest changed between our cached state and now; nothing to do.
        setInstalling(false);
        return;
      }
      await update.downloadAndInstall();
      await relaunch();
    } catch (err) {
      console.error("update install failed:", err);
      setInstalling(false);
      alert(`Update failed: ${err}. You can download the latest installer manually.`);
    }
  }

  if (!IS_DESKTOP) return null;

  // Forced-update blocking modal takes priority over the normal toast.
  if (forced) {
    return (
      <div role="dialog" aria-modal="true" className="updater-force-modal">
        <div className="updater-modal-card">
          <h2>Update required</h2>
          <p>
            This version ({forced.min_version} floor) is no longer supported.
            Please update to {forced.version} to continue.
          </p>
          <button disabled={installing} onClick={applyUpdate}>
            {installing ? "Updating…" : "Update now"}
          </button>
        </div>
      </div>
    );
  }

  // Toast: non-blocking "Update available" prompt
  if (available && !modalOpen) {
    return (
      <button
        className="updater-toast"
        onClick={() => setModalOpen(true)}
        aria-label="Update available"
      >
        Update available: {available.version}
      </button>
    );
  }

  // Modal: show release notes, let user install
  if (available && modalOpen) {
    return (
      <div role="dialog" aria-modal="true" className="updater-modal">
        <div className="updater-modal-card">
          <h2>Update to {available.version}</h2>
          {available.body ? <pre className="updater-notes">{available.body}</pre> : null}
          <div className="updater-modal-actions">
            <button disabled={installing} onClick={applyUpdate}>
              {installing ? "Installing…" : "Install and restart"}
            </button>
            <button disabled={installing} onClick={() => setModalOpen(false)}>
              Later
            </button>
          </div>
        </div>
      </div>
    );
  }

  return null;
}
```

- [ ] **Step 3: Also install `@tauri-apps/plugin-process` (needed for `relaunch()`)**

Run: `cd frontend && npm install @tauri-apps/plugin-process`

And expose it on the Rust side:
```toml
# src-tauri/Cargo.toml
tauri-plugin-process = "2"
```
```rust
// src-tauri/src/lib.rs — in the builder chain
.plugin(tauri_plugin_process::init())
```
```json
// src-tauri/capabilities/<main>.json — add to permissions
"process:default",
"process:allow-relaunch"
```

- [ ] **Step 4: Mount the bridge in App.tsx**

```tsx
// frontend/src/App.tsx — near the root render
import { UpdaterBridge } from "./updater/UpdaterBridge";

// Inside the top-level component tree, next to other root-level chrome:
<UpdaterBridge />
```

- [ ] **Step 5: Build check**

Run:
```bash
cd frontend && VITE_BUILD_TARGET=desktop npm run build
cd frontend && VITE_BUILD_TARGET=web npm run build
```
Expected: both builds succeed. In the web build, grep shows no `UpdaterBridge`:
```bash
grep -l "UpdaterBridge" frontend/dist/assets/*.js || echo OK
```
Expected: `OK` (desktop-only code tree-shaken out).

- [ ] **Step 6: Commit**

```bash
git add frontend/src/updater/ frontend/src/App.tsx frontend/package.json frontend/package-lock.json src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/capabilities/*.json
git commit -m "feat(desktop): UpdaterBridge — toast + modal + force-update"
```

---

## Phase 3: CI release pipeline

### Task 13: Provision CI release user + token

**Files:**
- Create: `tools/provision_ci_release_user.py`

- [ ] **Step 1: Write the provisioning script**

```python
# tools/provision_ci_release_user.py
"""One-shot: create (or verify) the `ci-desktop-release` admin user and
print a long-lived JWT for use in GitHub Actions.

Usage:
    python tools/provision_ci_release_user.py --password "$(pwgen -s 32 1)"

Run against the production DB with the appropriate DATABASE_URL env var.
Safe to re-run: if the user exists, we rotate the password to the new value
and print a fresh token.
"""
from __future__ import annotations

import argparse
import asyncio
import os
import sys
from datetime import datetime, timedelta, timezone

import bcrypt
from sqlalchemy import select

from digimon_gym.db.auth import ROLE_ADMIN, create_access_token
from digimon_gym.db.database import async_session_maker
from digimon_gym.db.models import User

CI_USERNAME = "ci-desktop-release"
TOKEN_EXPIRES = timedelta(days=365)


async def run(password: str) -> str:
    async with async_session_maker() as db:
        user = await db.scalar(select(User).where(User.username == CI_USERNAME))
        if user is None:
            user = User(
                username=CI_USERNAME,
                email=f"{CI_USERNAME}@ci.local",
                password_hash=bcrypt.hashpw(password.encode(), bcrypt.gensalt()).decode(),
                roles=[ROLE_ADMIN],
                created_at=datetime.now(timezone.utc),
            )
            db.add(user)
            await db.commit()
            await db.refresh(user)
            print(f"created user {CI_USERNAME} (id={user.id})", file=sys.stderr)
        else:
            user.password_hash = bcrypt.hashpw(password.encode(), bcrypt.gensalt()).decode()
            if ROLE_ADMIN not in (user.roles or []):
                user.roles = [*(user.roles or []), ROLE_ADMIN]
            await db.commit()
            print(f"refreshed password for existing user {CI_USERNAME}", file=sys.stderr)

        token = create_access_token(
            subject=user.id,
            expires_delta=TOKEN_EXPIRES,
        )
    return token


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--password", required=True, help="password for CI user")
    args = p.parse_args()
    token = asyncio.run(run(args.password))
    # stdout is the token only, so `gh secret set CI_ADMIN_TOKEN < tok.txt` works.
    print(token)


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Verify the User / auth imports match the codebase**

Run: `grep -n "create_access_token\|class User\|password_hash\|roles" digimon_gym/db/auth.py digimon_gym/db/models.py | head -20`

If the signature of `create_access_token` differs from the script (e.g., takes different kwargs, returns a dict, or `User.roles` uses a different column type / JSON serialization), adjust the script to match. The script is intentionally thin — it MUST match the real auth module shape before running.

- [ ] **Step 3: Dry-run against dev DB**

Run (with local dev `DATABASE_URL`):
```bash
python tools/provision_ci_release_user.py --password "dev-test-password"
```
Expected: prints a JWT string to stdout, stderr says "created user ci-desktop-release".

- [ ] **Step 4: Upload token + URL to GitHub Secrets**

```bash
python tools/provision_ci_release_user.py --password "$(openssl rand -base64 32)" > /tmp/ci_token.txt
gh secret set CI_ADMIN_TOKEN < /tmp/ci_token.txt
rm /tmp/ci_token.txt
gh secret set HOSTED_API_URL --body "https://api.digimon-tcg.example.com"
```
(Adjust the API URL to the real production hostname.)

- [ ] **Step 5: Commit**

```bash
git add tools/provision_ci_release_user.py
git commit -m "feat(tools): provision_ci_release_user.py for CI admin token"
```

---

### Task 14: GitHub Actions workflow

**Files:**
- Create: `.github/workflows/desktop-release.yml`

- [ ] **Step 1: Write the workflow**

```yaml
# .github/workflows/desktop-release.yml
name: desktop-release

on:
  push:
    tags:
      - "desktop-v*"

jobs:
  build:
    strategy:
      fail-fast: true
      matrix:
        include:
          - runner: windows-latest
            target: windows-x86_64
            bundle_glob: "src-tauri/target/release/bundle/nsis/*-setup.exe"
            sig_suffix: ".sig"
          - runner: ubuntu-latest
            target: linux-x86_64
            bundle_glob: "src-tauri/target/release/bundle/appimage/*.AppImage"
            sig_suffix: ".sig"
    runs-on: ${{ matrix.runner }}
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri -> target

      - uses: actions/setup-node@v4
        with:
          node-version: "20"
          cache: "npm"
          cache-dependency-path: frontend/package-lock.json

      - name: Install Linux system deps
        if: matrix.runner == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev \
            libayatana-appindicator3-dev librsvg2-dev libssl-dev

      - name: Install frontend deps
        working-directory: frontend
        run: npm ci

      - name: Build desktop frontend
        working-directory: frontend
        env:
          VITE_BUILD_TARGET: desktop
        run: npm run build

      - name: Build + sign Tauri bundle
        working-directory: src-tauri
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_UPDATER_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_UPDATER_KEY_PASSWORD }}
        run: cargo tauri build

      - name: Collect artifact
        id: collect
        shell: bash
        run: |
          INSTALLER=$(ls ${{ matrix.bundle_glob }} | head -n1)
          echo "installer=$INSTALLER" >> "$GITHUB_OUTPUT"
          echo "signature=${INSTALLER}${{ matrix.sig_suffix }}" >> "$GITHUB_OUTPUT"
          echo "Installer: $INSTALLER"
          ls -lh "$INSTALLER" "${INSTALLER}${{ matrix.sig_suffix }}"

      - name: Upload workflow artifact
        uses: actions/upload-artifact@v4
        with:
          name: desktop-${{ matrix.target }}
          path: |
            ${{ steps.collect.outputs.installer }}
            ${{ steps.collect.outputs.signature }}
          if-no-files-found: error
          retention-days: 7

  publish:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Resolve version + engine_commit
        id: meta
        shell: bash
        run: |
          TAG="${GITHUB_REF_NAME}"                 # e.g. desktop-v0.2.0-alpha.3
          VERSION="${TAG#desktop-v}"               # e.g. 0.2.0-alpha.3
          ENGINE_COMMIT=$(git rev-parse --short HEAD)
          # Extract release notes from annotated tag body (git tag -a -m "..."),
          # falling back to a one-liner if empty.
          NOTES=$(git tag -l --format='%(contents)' "$TAG")
          if [ -z "$NOTES" ]; then NOTES="Release $VERSION"; fi
          # min_version defaults to X.Y.0 for X.Y.Z-* — bumped manually via PATCH
          MAJ_MIN=$(echo "$VERSION" | sed -E 's/([0-9]+\.[0-9]+)\..*/\1.0/')
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"
          echo "engine_commit=$ENGINE_COMMIT" >> "$GITHUB_OUTPUT"
          echo "min_version=$MAJ_MIN" >> "$GITHUB_OUTPUT"
          # Multiline output via heredoc
          {
            echo "notes<<NOTES_EOF"
            echo "$NOTES"
            echo "NOTES_EOF"
          } >> "$GITHUB_OUTPUT"

      - name: Create release on hosted API
        id: create
        env:
          HOSTED_API_URL: ${{ secrets.HOSTED_API_URL }}
          CI_ADMIN_TOKEN: ${{ secrets.CI_ADMIN_TOKEN }}
        shell: bash
        run: |
          set -euo pipefail
          BODY=$(jq -n \
            --arg version "${{ steps.meta.outputs.version }}" \
            --arg engine_commit "${{ steps.meta.outputs.engine_commit }}" \
            --arg min_version "${{ steps.meta.outputs.min_version }}" \
            --arg notes "${{ steps.meta.outputs.notes }}" \
            '{
               version: $version,
               channel: "alpha",
               engine_commit: $engine_commit,
               min_version: $min_version,
               release_notes: $notes,
               targets: ["windows-x86_64", "linux-x86_64"]
             }')
          RESP=$(curl -fsS -X POST "$HOSTED_API_URL/admin/releases" \
            -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
            -H "Content-Type: application/json" \
            -d "$BODY")
          echo "$RESP" > /tmp/create.json
          RELEASE_ID=$(jq -r '.release_id' /tmp/create.json)
          echo "release_id=$RELEASE_ID" >> "$GITHUB_OUTPUT"

      - name: Upload artifacts to Spaces via presigned PUTs
        env:
          HOSTED_API_URL: ${{ secrets.HOSTED_API_URL }}
          CI_ADMIN_TOKEN: ${{ secrets.CI_ADMIN_TOKEN }}
          RELEASE_ID: ${{ steps.create.outputs.release_id }}
        shell: bash
        run: |
          set -euo pipefail
          for row in $(jq -c '.artifacts[]' /tmp/create.json); do
            TARGET=$(echo "$row" | jq -r '.target')
            URL=$(echo "$row" | jq -r '.upload_url')
            FILENAME=$(echo "$row" | jq -r '.filename')

            # Find the locally-downloaded installer + sig for this target
            ART_DIR="artifacts/desktop-$TARGET"
            case "$TARGET" in
              windows-x86_64) LOCAL=$(ls "$ART_DIR"/*-setup.exe | head -n1) ;;
              linux-x86_64)   LOCAL=$(ls "$ART_DIR"/*.AppImage | head -n1) ;;
              *) echo "unknown target $TARGET" >&2; exit 1 ;;
            esac
            SIG_FILE="${LOCAL}.sig"

            echo "Uploading $LOCAL -> $URL"
            curl -fsS -X PUT --data-binary "@${LOCAL}" \
              -H "Content-Type: application/octet-stream" \
              -H "x-amz-acl: public-read" \
              "$URL"

            SIG_B64=$(cat "$SIG_FILE" | tr -d '\n')

            echo "Confirming $TARGET"
            curl -fsS -X POST \
              "$HOSTED_API_URL/admin/releases/$RELEASE_ID/artifacts/$TARGET/confirm" \
              -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
              -H "Content-Type: application/json" \
              -d "$(jq -n --arg sig "$SIG_B64" '{signature_b64: $sig}')"
          done

      - name: Publish release
        env:
          HOSTED_API_URL: ${{ secrets.HOSTED_API_URL }}
          CI_ADMIN_TOKEN: ${{ secrets.CI_ADMIN_TOKEN }}
          RELEASE_ID: ${{ steps.create.outputs.release_id }}
        run: |
          curl -fsS -X POST \
            "$HOSTED_API_URL/admin/releases/$RELEASE_ID/publish" \
            -H "Authorization: Bearer $CI_ADMIN_TOKEN"

      - name: Create GitHub release entry
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        shell: bash
        run: |
          gh release create "$GITHUB_REF_NAME" \
            --title "Desktop ${{ steps.meta.outputs.version }}" \
            --notes "${{ steps.meta.outputs.notes }}" \
            artifacts/desktop-windows-x86_64/* \
            artifacts/desktop-linux-x86_64/*
```

- [ ] **Step 2: Lint the workflow**

Run:
```bash
gh workflow view desktop-release.yml || true
```
Any GitHub-side parse errors will surface here (the file does need to be committed + pushed first for full validation, but CLI flags any obvious issues). Also run a local YAML parse:
```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/desktop-release.yml'))"
```
Expected: no exceptions.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/desktop-release.yml
git commit -m "feat(ci): desktop-release workflow for desktop-v* tags"
```

---

### Task 15: End-to-end smoke test (local, manual)

**Files:**
- Create: `tools/publish_release_smoke.py`

This is a one-shot manual verification script — the automated tests cover the unit behaviors; this lets you confirm the full flow against a real dev DO Spaces bucket + local hosted API before trusting CI.

- [ ] **Step 1: Write the smoke script**

```python
# tools/publish_release_smoke.py
"""Smoke-test the /admin/releases flow end-to-end against a running hosted API.

Usage (local dev):
    SPACES_* env vars set, hosted API running on :8000:

    python tools/publish_release_smoke.py \\
        --api http://localhost:8000 \\
        --token "$CI_ADMIN_TOKEN" \\
        --version 0.0.1-smoke.1 \\
        --windows-installer /tmp/fake-installer.exe \\
        --windows-sig /tmp/fake-installer.exe.sig \\
        --linux-installer /tmp/fake-installer.AppImage \\
        --linux-sig /tmp/fake-installer.AppImage.sig

The "installers" can be arbitrary bytes for smoke purposes — we're not
verifying signatures here, just the hosted API + Spaces round trip.
"""
from __future__ import annotations

import argparse
import base64
import json
import pathlib
import sys
import urllib.request

import requests


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--api", required=True)
    p.add_argument("--token", required=True)
    p.add_argument("--version", required=True)
    p.add_argument("--engine-commit", default="smoke")
    p.add_argument("--min-version", default="0.0.0")
    p.add_argument("--windows-installer", required=True)
    p.add_argument("--windows-sig", required=True)
    p.add_argument("--linux-installer", required=True)
    p.add_argument("--linux-sig", required=True)
    args = p.parse_args()

    headers = {"Authorization": f"Bearer {args.token}"}

    # 1. Create
    create = requests.post(
        f"{args.api}/admin/releases",
        headers=headers,
        json={
            "version": args.version,
            "channel": "alpha",
            "engine_commit": args.engine_commit,
            "min_version": args.min_version,
            "release_notes": f"smoke test {args.version}",
            "targets": ["windows-x86_64", "linux-x86_64"],
        },
    )
    create.raise_for_status()
    body = create.json()
    release_id = body["release_id"]
    print(f"Created release {release_id}")

    targets = {
        "windows-x86_64": (args.windows_installer, args.windows_sig),
        "linux-x86_64": (args.linux_installer, args.linux_sig),
    }

    for art in body["artifacts"]:
        target = art["target"]
        installer, sig_path = targets[target]

        # 2. Upload via presigned PUT
        with open(installer, "rb") as f:
            put = urllib.request.Request(
                art["upload_url"],
                data=f.read(),
                method="PUT",
                headers={"Content-Type": "application/octet-stream", "x-amz-acl": "public-read"},
            )
            with urllib.request.urlopen(put) as resp:
                assert resp.status in (200, 204), f"upload {target}: {resp.status}"
        print(f"Uploaded {target}")

        # 3. Confirm
        sig_b64 = pathlib.Path(sig_path).read_text().strip()
        confirm = requests.post(
            f"{args.api}/admin/releases/{release_id}/artifacts/{target}/confirm",
            headers=headers,
            json={"signature_b64": sig_b64},
        )
        confirm.raise_for_status()
        print(f"Confirmed {target}: sha256={confirm.json()['file_sha256']}")

    # 4. Publish
    pub = requests.post(
        f"{args.api}/admin/releases/{release_id}/publish",
        headers=headers,
    )
    pub.raise_for_status()
    print(f"Published {release_id}")

    # 5. Fetch public manifest
    manifest_url = f"{args.api}/admin/releases".replace("/admin/releases", "") + "/updates/alpha/latest.json"
    print("Manifest URL (via API convention):", manifest_url)
    # Actual fetch should hit the Spaces URL; for smoke we just print the body:
    info = requests.post(
        f"{args.api}/admin/releases/regenerate-manifest?channel=alpha",
        headers=headers,
    )
    info.raise_for_status()
    print(json.dumps(info.json(), indent=2))


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run smoke test against local API + dev Spaces bucket**

Prerequisite: local hosted API running with real DO Spaces env vars pointing at a **dev bucket** (not production). Generate fake installers:

```bash
dd if=/dev/urandom of=/tmp/fake-installer.exe bs=1024 count=100
echo "fake-sig-windows" | base64 > /tmp/fake-installer.exe.sig
dd if=/dev/urandom of=/tmp/fake-installer.AppImage bs=1024 count=100
echo "fake-sig-linux" | base64 > /tmp/fake-installer.AppImage.sig

python tools/provision_ci_release_user.py --password smokepass > /tmp/tok.txt

python tools/publish_release_smoke.py \
  --api http://localhost:8000 \
  --token "$(cat /tmp/tok.txt)" \
  --version 0.0.1-smoke.1 \
  --windows-installer /tmp/fake-installer.exe \
  --windows-sig /tmp/fake-installer.exe.sig \
  --linux-installer /tmp/fake-installer.AppImage \
  --linux-sig /tmp/fake-installer.AppImage.sig
```

Expected: prints `Created ...`, `Uploaded ...` x2, `Confirmed ...` x2, `Published ...`, and the regenerated manifest JSON with both platforms populated and `url` pointing at the dev Spaces bucket.

- [ ] **Step 3: Curl the live manifest**

```bash
# URL pattern from spaces.public_url(); adjust to your dev bucket host
curl -v "https://<dev-bucket>.<region>.digitaloceanspaces.com/updates/alpha/latest.json"
```
Expected: 200 OK, JSON body matches what regenerate-manifest returned. `Cache-Control: public, max-age=60` header present.

- [ ] **Step 4: Tear down dev data**

```bash
# Unpublish + delete via curl; verify manifest object disappears
RELEASE_ID=$(curl -s -H "Authorization: Bearer $(cat /tmp/tok.txt)" \
  "http://localhost:8000/admin/releases?channel=alpha" | jq -r '.releases[0].id')
curl -X POST -H "Authorization: Bearer $(cat /tmp/tok.txt)" \
  "http://localhost:8000/admin/releases/$RELEASE_ID/unpublish"
curl -X DELETE -H "Authorization: Bearer $(cat /tmp/tok.txt)" \
  "http://localhost:8000/admin/releases/$RELEASE_ID"
rm /tmp/tok.txt /tmp/fake-installer.*
```

- [ ] **Step 5: Commit the smoke script**

```bash
git add tools/publish_release_smoke.py
git commit -m "feat(tools): publish_release_smoke.py for manual end-to-end check"
```

---

## Phase 4: Docs

### Task 16: Flesh out the release runbook

**Files:**
- Modify: `docs/runbooks/desktop-release.md`

- [ ] **Step 1: Expand runbook**

Replace the Task 9 scaffold content with:

````markdown
# Desktop Release Runbook

## Updater key custody

- Private key file: `~/.tauri/digimon-updater.key` (password-encrypted).
- Password: 1Password → "Digimon TCG" → "Tauri Updater Key".
- Public key: committed in `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).
- GitHub Actions secrets: `TAURI_UPDATER_PRIVATE_KEY` (file contents),
  `TAURI_UPDATER_KEY_PASSWORD` (password), `CI_ADMIN_TOKEN`, `HOSTED_API_URL`.

## Cut a new release

1. Ensure `main` is green and the change is in `main`.
2. Bump `version` in `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`
   (keep them synchronized — Tauri uses the Cargo version at runtime).
3. Commit version bump.
4. Create an annotated tag with release notes in the body:
   ```bash
   git tag -a desktop-v0.2.0-alpha.3 -m "Fix: deckbuilder crash on whitespace import.
   Add: Beelzemon gauntlet preset."
   git push origin desktop-v0.2.0-alpha.3
   ```
5. Watch CI: `gh run watch`. On success the new manifest is live in Spaces
   within ~60s of the publish step.
6. Verify from a fresh dev machine: launch the current installed alpha; confirm
   the "Update available" toast appears; click through to install.

## Roll back a broken release

### Scenario A: broken build will still launch

Testers can self-recover by updating forward. Rollback is "cut a new release
from the last good commit":

1. `git checkout <last-good-commit>`
2. Bump version to something strictly greater than the broken one
   (e.g. if broken was `0.2.0-alpha.3`, cut `0.2.0-alpha.4`).
3. Tag: `git tag -a desktop-v0.2.0-alpha.4 -m "Revert 0.2.0-alpha.3 regression"`.
4. Push the tag; CI publishes. Testers update forward next launch.

Also unpublish the broken release so new installers from scratch don't pull it:
```bash
curl -X POST -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
  "$HOSTED_API_URL/admin/releases/$BROKEN_RELEASE_ID/unpublish"
```

### Scenario B: broken build crashes before the updater can run

The normal update path is dead. Use the `min_version` kill-switch:

1. Cut + publish a new good release as in Scenario A (e.g. `0.2.0-alpha.4`).
2. Bump `min_version` on the *broken* (now unpublished) release — or on the
   *new* release, doesn't matter as only the currently-published one's
   `min_version` is served:
   ```bash
   curl -X PATCH -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
     -H "Content-Type: application/json" \
     "$HOSTED_API_URL/admin/releases/$NEW_RELEASE_ID" \
     -d '{"min_version": "0.2.0-alpha.4"}'
   ```
3. Running broken installs will see `manifest.min_version > running.version`
   on their next launch and get the force-update modal (rendered from
   `updater:force-update` event — pre-empts any crash-on-launch bug that
   happens *after* Rust setup).

Caveat: if the crash happens during Rust plugin init itself (very rare), the
min-version check never runs. At that point the only recourse is to email
testers the new installer URL for manual reinstall.

## Rotate the updater private key

Rotation bricks every already-installed tester's auto-update path. Do not
rotate casually. Procedure if the key leaks:

1. Generate new key: `cargo tauri signer generate -w ~/.tauri/digimon-updater-v2.key`.
2. Update `src-tauri/tauri.conf.json`'s `plugins.updater.pubkey`.
3. Update GHA secrets `TAURI_UPDATER_PRIVATE_KEY` + `TAURI_UPDATER_KEY_PASSWORD`.
4. Bump major-ish version so SemVer clearly distinguishes (e.g. `0.3.0-alpha.1`).
5. Cut a release via the normal flow.
6. Email alpha tester list: "Please download the new installer manually from
   <GitHub release URL>. Auto-update will not work across this version."

## Common issues

| Symptom | Likely cause | Fix |
|---|---|---|
| CI publish step 401s | `CI_ADMIN_TOKEN` expired or user revoked | Re-run `tools/provision_ci_release_user.py`, update `CI_ADMIN_TOKEN` secret |
| Tauri build signing fails with "bad password" | GHA secret mismatch | Re-set `TAURI_UPDATER_KEY_PASSWORD` |
| Testers don't see the update | Spaces CDN caching the old manifest | `Cache-Control: max-age=60`; wait 60s. If persistent: `POST /admin/releases/regenerate-manifest?channel=alpha` and verify the Spaces URL directly |
| Windows SmartScreen blocks install | Self-signed installer (expected for alpha) | Tester clicks "More info → Run anyway". Documented UX cost until OV cert is purchased. |
````

- [ ] **Step 2: Commit**

```bash
git add docs/runbooks/desktop-release.md
git commit -m "docs(runbook): full desktop-release procedure + rollback + rotation"
```

---

## Phase 5: Full-system verification

### Task 17: End-to-end dry run on a real test tag

**Files:** None (this is operational verification).

- [ ] **Step 1: Push a test tag**

```bash
# Use a clearly-scratchpad version so real testers don't get confused
git tag -a desktop-v0.0.1-ci-test.1 -m "CI smoke test — ignore"
git push origin desktop-v0.0.1-ci-test.1
```

- [ ] **Step 2: Watch CI run**

```bash
gh run watch
```
Expected: both `build` matrix jobs complete (Windows + Linux), `publish` job runs, exits 0.

- [ ] **Step 3: Verify Spaces manifest**

```bash
curl "https://<bucket>.<region>.digitaloceanspaces.com/updates/alpha/latest.json" | jq
```
Expected: valid JSON matching the spec's manifest contract. `version` = `0.0.1-ci-test.1`. Both `platforms` keys present. `engine_commit` = current short SHA.

- [ ] **Step 4: Install the Windows installer on a clean VM (or the current dev machine)**

Download the `-setup.exe` from the GitHub release page (`gh release view desktop-v0.0.1-ci-test.1`), install, launch. The app should open normally.

- [ ] **Step 5: Trigger an update**

1. Bump version locally to `0.0.1-ci-test.2`.
2. Tag + push.
3. On the machine running `-ci-test.1`, relaunch. Confirm:
   - No force-update modal (min_version stays at `0.0.0` from CI default).
   - "Update available" toast appears within ~5 seconds.
   - Clicking shows a modal with the tag's annotation body as release notes.
   - "Install and restart" successfully downloads + verifies signature + relaunches into `-ci-test.2`.

- [ ] **Step 6: Exercise rollback**

```bash
CI_TEST_RELEASE_ID=$(curl -s -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
  "$HOSTED_API_URL/admin/releases?channel=alpha" | jq -r '.releases[0].id')
curl -X POST -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
  "$HOSTED_API_URL/admin/releases/$CI_TEST_RELEASE_ID/unpublish"
curl -s "https://<bucket>.<region>.digitaloceanspaces.com/updates/alpha/latest.json" | jq '.version'
```
Expected: version reverts to `-ci-test.1` (or 404 if that one's also deleted).

- [ ] **Step 7: Cleanup test tags/releases**

```bash
curl -X POST -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
  "$HOSTED_API_URL/admin/releases/$CI_TEST_RELEASE_ID/unpublish"
curl -X DELETE -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
  "$HOSTED_API_URL/admin/releases/$CI_TEST_RELEASE_ID"
# ... repeat for the other test release
gh release delete desktop-v0.0.1-ci-test.1 --yes --cleanup-tag
gh release delete desktop-v0.0.1-ci-test.2 --yes --cleanup-tag
```

- [ ] **Step 8: Document findings in the runbook if anything surprised you**

If any step diverged from the runbook, PATCH the runbook in `docs/runbooks/desktop-release.md` and commit. Otherwise:

```bash
# No commit needed — operational verification only.
```

---

## Self-review checklist

- [ ] All spec sections mapped to tasks: Context ✓, Manifest contract (Task 7's `_build_manifest`), Server surface (Tasks 1–8), Tauri integration (Tasks 9–12), Release pipeline (Tasks 13–15), Rollback + kill-switch (Task 8 `unpublish` + Task 11 force-update + runbook), Security (Task 9 custody + Task 14 presigned-only CI access).
- [ ] No placeholders: every step has actual code or commands.
- [ ] Type consistency: `AppRelease.published`, `AppReleaseArtifact.signature_b64`, `ReleaseManifest` fields used identically across Tasks 2/3/5/6/7/8.
- [ ] Admin UI deferred: spec §Admin UI explicitly out-of-scope; plan respects that and notes it in File Structure §Deferred.
- [ ] Tests precede implementation in every code task.
- [ ] Each task ends with a commit.

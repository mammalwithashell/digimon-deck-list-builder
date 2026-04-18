# Alpha Desktop Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Tauri desktop build that lets anonymous users play PvP via the existing matchmaking queue, play vs AI online (server-side inference against the DO-Spaces-backed manifest), and play vs AI offline (downloaded ONNX).

**Architecture:** Frontend rides on the existing desktop Tauri build (`VITE_BUILD_TARGET=desktop`). A thin guest-identity layer mints a long-lived JWT on first launch; decks live client-side in `app_data_dir` via new Tauri commands; the hosted FastAPI grows a `POST /auth/guest` endpoint, an inline-deck variant of the matchmaking queue, and manifest-id resolution for `POST /games`. The Models page becomes the hub for AI opponents: each manifest entry renders either a **Try online** button (creates a server-side game) or a **Download** / **Play offline** pair based on local state. Everything else is UX cleanup (Home redesign, Lobby trim, DeckBuilder lock-to-standard, Layout nav prune).

**Tech Stack:** Rust (Tauri + digimon-engine), TypeScript (React + Zustand), Python (FastAPI + SQLAlchemy), JWT (python-jose), pytest, vitest, cargo test.

**Spec:** [`docs/superpowers/specs/2026-04-18-alpha-desktop-release-design.md`](../specs/2026-04-18-alpha-desktop-release-design.md)

---

## File Structure

### Backend — create

- `tests/api/test_auth_guest.py` — guest endpoint + matchmaking integration
- `tests/api/test_games_manifest_resolution.py` — `/games` resolves manifest_id → Spaces
- `tests/api/test_matchmaking_inline_deck.py` — queue accepts inline deck payload

### Backend — modify

- `digimon_gym/db/models.py` — add `User.is_guest` column (default `False`)
- `digimon_gym/db/auth.py` — add `create_guest_access_token(...)` with 365-day expiry
- `digimon_gym/db/schemas.py` — add `GuestSessionResponse` pydantic model
- `digimon_gym/db/routers/auth.py` — add `POST /auth/guest` endpoint
- `digimon_gym/api.py` — include the router hookup if not already wired (verify only)
- `digimon_gym/engine/model_utils.py` — add `resolve_manifest_model_path(db, manifest_id)` that streams from Spaces to a sha256-keyed local cache
- `digimon_gym/routers/games.py` — accept `player{1,2}_model` either as a filename or a manifest UUID; prefer manifest resolution when the value is a UUID
- `digimon_gym/routers/matchmaking.py` — accept `{main_deck, egg_deck, game_mode}` inline in `QueueRequest`, bypassing the DB `Deck` lookup

### Frontend — create

- `src-tauri/src/deck_storage.rs` — Tauri commands `decks_list`, `decks_get`, `decks_put`, `decks_delete` backed by JSON files under `app_data_dir()/decks/`
- `frontend/src/storage/deckStore.ts` — thin TS wrapper matching the existing `DeckSummary` / `DeckResponse` shapes from `deckApi.ts`
- `frontend/src/bootstrap/guest.ts` — on boot, mint a guest JWT if none cached
- `frontend/src/bootstrap/guest.test.ts` — unit tests
- `frontend/src/components/home/AlphaBanner.tsx` — small banner with "Alpha" tag and a link to patch notes
- `tests/tauri/deck_storage_test.rs` (or inline `#[cfg(test)]` in `deck_storage.rs`) — round-trip test

### Frontend — modify

- `src-tauri/src/main.rs` — register new deck-storage commands in the `.invoke_handler`
- `frontend/src/stores/authStore.ts` — `hydrate()` delegates to the guest bootstrap on desktop
- `frontend/src/components/auth/AuthGuard.tsx` — on desktop, gate on "guest token exists" rather than "user is logged in"
- `frontend/src/pages/HomePage.tsx` — five-card grid + AlphaBanner
- `frontend/src/pages/ModelsPage.tsx` — one unified table; each row has state-dependent buttons (Try online, Download, Play offline, Delete); drop the manifest-URL text input
- `frontend/src/pages/GamePage.tsx` — handle `mode=vsai` (WS-driven, no Tauri calls for the opponent)
- `frontend/src/pages/LobbyPage.tsx` — sources decks from `deckStore`; sends matchmaking queue request with inline deck payload; hides the Browse tab
- `frontend/src/pages/DeckBuilderPage.tsx` — save/load via `deckStore`; lock `game_mode` to `"standard"`; remove format picker
- `frontend/src/components/layout/Layout.tsx` — strip admin/training nav items in desktop build
- `frontend/src/api/gameApi.ts` — pass `player{1,2}_model` as-is (it's already a string; the backend decides whether it's a filename or manifest_id)

### Environment

- `frontend/.env.desktop` (or equivalent) — add `VITE_MODELS_MANIFEST_URL` baked in to the desktop build
- `src-tauri/tauri.conf.json` — add `"fs:default"` and `"core:path:default"` capabilities if not already granted (for deck_storage file IO)

---

## Phase 1 — Backend plumbing

### Task 1: Add `User.is_guest` flag

**Files:**
- Modify: `digimon_gym/db/models.py` (around line 54, next to `rating`)
- Modify: any Alembic/migration mechanism if present; otherwise this is a dev-db schema update

- [ ] **Step 1: Read the current `User` model**

Run: `rg -n "class User" digimon_gym/db/models.py`
Expected: line number around 39–55.

- [ ] **Step 2: Write the failing test**

Create `tests/api/test_auth_guest.py`:

```python
"""Guest-user endpoint + integration with matchmaking."""
from __future__ import annotations

import pytest
from httpx import AsyncClient

from digimon_gym.db.models import User


@pytest.mark.asyncio
async def test_user_model_has_is_guest_flag(db_session) -> None:
    user = User(
        username="test_flag_user",
        email="flag@example.com",
        password_hash="dummy",
    )
    db_session.add(user)
    await db_session.flush()
    assert user.is_guest is False, "new users default to is_guest=False"
```

Run: `pytest tests/api/test_auth_guest.py::test_user_model_has_is_guest_flag -v`
Expected: FAIL — `AttributeError: 'User' object has no attribute 'is_guest'`.

- [ ] **Step 3: Add the column**

In `digimon_gym/db/models.py`, inside `class User`, after the `rating` column:

```python
    is_guest = Column(Boolean, default=False, nullable=False)
```

Add `Boolean` to the existing `from sqlalchemy import (...)` block if not already there.

- [ ] **Step 4: Run the test**

Run: `pytest tests/api/test_auth_guest.py::test_user_model_has_is_guest_flag -v`
Expected: PASS.

If the dev DB is a persisted file (SQLite), recreate it: `rm -f digimon_tcg.db && python -c "from digimon_gym.db.database import init_db; import asyncio; asyncio.run(init_db())"`.

- [ ] **Step 5: Commit**

```bash
git add digimon_gym/db/models.py tests/api/test_auth_guest.py
git commit -m "feat: add User.is_guest flag for anonymous guest sessions"
```

---

### Task 2: Add `create_guest_access_token` helper

**Files:**
- Modify: `digimon_gym/db/auth.py`

- [ ] **Step 1: Write the failing test**

Append to `tests/api/test_auth_guest.py`:

```python
from datetime import datetime, timedelta, timezone
from jose import jwt

from digimon_gym.db.auth import (
    ALGORITHM,
    SECRET_KEY,
    create_guest_access_token,
)


def test_create_guest_access_token_has_year_long_expiry() -> None:
    token = create_guest_access_token(user_id="guest_123", display_name="Guest-abcd")
    payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
    assert payload["sub"] == "guest_123"
    assert payload["username"] == "Guest-abcd"
    assert payload["type"] == "access"
    assert payload["is_guest"] is True
    exp = datetime.fromtimestamp(payload["exp"], tz=timezone.utc)
    now = datetime.now(timezone.utc)
    # Between 360 and 370 days in the future.
    assert timedelta(days=360) <= (exp - now) <= timedelta(days=370)
```

Run: `pytest tests/api/test_auth_guest.py::test_create_guest_access_token_has_year_long_expiry -v`
Expected: FAIL — `ImportError: cannot import name 'create_guest_access_token'`.

- [ ] **Step 2: Implement the helper**

In `digimon_gym/db/auth.py`, below `create_access_token`:

```python
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
```

- [ ] **Step 3: Run the test**

Run: `pytest tests/api/test_auth_guest.py::test_create_guest_access_token_has_year_long_expiry -v`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add digimon_gym/db/auth.py tests/api/test_auth_guest.py
git commit -m "feat: add create_guest_access_token (365-day expiry)"
```

---

### Task 3: Add `POST /auth/guest` endpoint

**Files:**
- Modify: `digimon_gym/db/schemas.py` (add `GuestSessionResponse`)
- Modify: `digimon_gym/db/routers/auth.py`

- [ ] **Step 1: Write the failing test**

Append to `tests/api/test_auth_guest.py`:

```python
@pytest.mark.asyncio
async def test_post_auth_guest_creates_new_guest_user(client: AsyncClient, db_session) -> None:
    resp = await client.post("/auth/guest")
    assert resp.status_code == 201, resp.text
    body = resp.json()
    assert "access_token" in body
    assert body["user_id"].startswith("guest_")
    assert body["display_name"].startswith("Guest-")
    assert len(body["display_name"]) == len("Guest-") + 4  # 4-char suffix

    # The row exists and is flagged as a guest.
    from sqlalchemy import select
    row = (await db_session.execute(
        select(User).where(User.id == body["user_id"])
    )).scalar_one()
    assert row.is_guest is True
    assert row.rating == 1500.0


@pytest.mark.asyncio
async def test_post_auth_guest_is_idempotent_per_call(client: AsyncClient) -> None:
    """Each call mints a fresh guest — no session stickiness."""
    a = (await client.post("/auth/guest")).json()
    b = (await client.post("/auth/guest")).json()
    assert a["user_id"] != b["user_id"]
```

Run: `pytest tests/api/test_auth_guest.py -v -k post_auth_guest`
Expected: FAIL — 404 or missing route.

- [ ] **Step 2: Add the response schema**

In `digimon_gym/db/schemas.py`, near the other token/user schemas:

```python
class GuestSessionResponse(BaseModel):
    access_token: str
    token_type: str = "bearer"
    user_id: str
    display_name: str
```

- [ ] **Step 3: Add the endpoint**

In `digimon_gym/db/routers/auth.py`, at the bottom of the file:

```python
import secrets
import string
import uuid as _uuid

from digimon_gym.db.auth import create_guest_access_token


def _generate_guest_suffix() -> str:
    alphabet = string.ascii_uppercase + string.digits
    return "".join(secrets.choice(alphabet) for _ in range(4))


@router.post(
    "/guest",
    response_model=GuestSessionResponse,
    status_code=status.HTTP_201_CREATED,
)
async def create_guest(db: AsyncSession = Depends(get_db)) -> GuestSessionResponse:
    """Create an anonymous guest account and return a long-lived access token.

    Each call mints a distinct guest. Clients are expected to cache the
    token in `localStorage` and reuse it on subsequent launches. Losing
    the token creates a new identity next boot — acceptable because
    guest-owned data (decks) lives on the client.
    """
    guest_id = f"guest_{_uuid.uuid4()}"
    suffix = _generate_guest_suffix()
    display_name = f"Guest-{suffix}"
    # Use suffix in the synthetic email to keep the unique constraint happy.
    placeholder_email = f"{guest_id}@guest.invalid"

    user = User(
        id=guest_id,
        username=guest_id,
        email=placeholder_email,
        password_hash="!disabled-guest",
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
```

Add the schema import at the top:

```python
from digimon_gym.db.schemas import (
    GuestSessionResponse,
    LoginRequest,
    ...
)
```

- [ ] **Step 4: Run the test**

Run: `pytest tests/api/test_auth_guest.py -v -k post_auth_guest`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon_gym/db/schemas.py digimon_gym/db/routers/auth.py tests/api/test_auth_guest.py
git commit -m "feat: POST /auth/guest — anonymous guest sessions with 365-day JWTs"
```

---

### Task 4: Matchmaking accepts inline deck payload

**Files:**
- Modify: `digimon_gym/routers/matchmaking.py`
- Create: `tests/api/test_matchmaking_inline_deck.py`

- [ ] **Step 1: Read the current `QueueRequest` + `enqueue`**

Run: `rg -n "class QueueRequest|async def enqueue" digimon_gym/routers/matchmaking.py`
Expected: two matches.

- [ ] **Step 2: Write the failing test**

Create `tests/api/test_matchmaking_inline_deck.py`:

```python
"""Matchmaking queue with an inline deck payload (guest path)."""
from __future__ import annotations

import pytest
from httpx import AsyncClient

from digimon_gym.routers import matchmaking as mm


@pytest.fixture(autouse=True)
def _clean_mm_state():
    mm.reset_state()
    yield
    mm.reset_state()


@pytest.mark.asyncio
async def test_queue_accepts_inline_deck_for_guest(client: AsyncClient) -> None:
    # Mint a guest token.
    guest = (await client.post("/auth/guest")).json()
    headers = {"Authorization": f"Bearer {guest['access_token']}"}

    body = {
        "queue_type": "casual",
        "main_deck": ["BT1-001"] * 50,
        "egg_deck": ["BT1-002"] * 5,
        "game_mode": "standard",
        "opponent_tier_filter": "any",
    }
    resp = await client.post("/matchmaking/queue", json=body, headers=headers)
    assert resp.status_code in (200, 201), resp.text
    payload = resp.json()
    assert payload["status"] == "waiting"
    ticket_id = payload["ticket_id"]

    ticket = mm.tickets[ticket_id]
    assert ticket.deck == ["BT1-001"] * 50 + ["BT1-002"] * 5
    assert ticket.game_mode == "standard"
    assert ticket.user_id == guest["user_id"]


@pytest.mark.asyncio
async def test_queue_rejects_request_missing_both_deck_id_and_inline(client: AsyncClient) -> None:
    guest = (await client.post("/auth/guest")).json()
    headers = {"Authorization": f"Bearer {guest['access_token']}"}
    resp = await client.post(
        "/matchmaking/queue",
        json={"queue_type": "casual"},
        headers=headers,
    )
    assert resp.status_code == 400
    assert "deck" in resp.json()["detail"].lower()
```

Run: `pytest tests/api/test_matchmaking_inline_deck.py -v`
Expected: FAIL — 422 (pydantic validation) or deck_id missing.

- [ ] **Step 3: Widen `QueueRequest`**

In `digimon_gym/routers/matchmaking.py`, replace the existing `QueueRequest` class with:

```python
class QueueRequest(BaseModel):
    queue_type: QueueType
    # Either deck_id (resolves a server-side Deck row) or the inline
    # triple below. Inline is the guest path; deck_id is the accounts path
    # for post-alpha.
    deck_id: Optional[str] = None
    main_deck: Optional[list[str]] = None
    egg_deck: Optional[list[str]] = None
    game_mode: Optional[str] = None
    opponent_tier_filter: TierFilter = "any"
```

- [ ] **Step 4: Update `enqueue` to handle the inline path**

Replace the body of `enqueue` up to the `QueueTicket(...)` construction with:

```python
    _prune_stale_tickets()

    if user.id in user_to_ticket:
        raise HTTPException(status.HTTP_409_CONFLICT, "User already has an active ticket")

    # Resolve the deck: prefer inline payload, else DB lookup.
    self_tier: Optional[str] = None
    if request.main_deck is not None:
        if request.game_mode is None:
            raise HTTPException(
                status.HTTP_400_BAD_REQUEST,
                "Inline deck requires game_mode",
            )
        card_ids = list(request.main_deck) + list(request.egg_deck or [])
        if not card_ids:
            raise HTTPException(status.HTTP_400_BAD_REQUEST, "Deck is empty")
        game_mode = request.game_mode
        # Tier classifier is optional for guests — leave as None for alpha.
    elif request.deck_id is not None:
        result = await db.execute(select(Deck).where(Deck.id == request.deck_id))
        deck = result.scalar_one_or_none()
        if deck is None or deck.owner_id != user.id:
            raise HTTPException(status.HTTP_404_NOT_FOUND, "Deck not found")
        import json
        card_ids = json.loads(deck.main_deck) + json.loads(deck.egg_deck or "[]")
        if not card_ids:
            raise HTTPException(status.HTTP_400_BAD_REQUEST, "Deck is empty")
        game_mode = deck.game_mode
        self_tier = deck.meta_tier
    else:
        raise HTTPException(
            status.HTTP_400_BAD_REQUEST,
            "Request must include either deck_id or inline deck (main_deck + game_mode)",
        )

    rating = None
    if request.queue_type == "ranked":
        rating = float(getattr(user, "rating", 1500.0) or 1500.0)

    ticket = QueueTicket(
        ticket_id=str(uuid4()),
        user_id=user.id,
        display_name=user.display_name or user.username,
        queue_type=request.queue_type,
        deck=card_ids,
        game_mode=game_mode,
        self_tier=self_tier,
        opponent_tier_filter=request.opponent_tier_filter,
        rating=rating,
        created_at=datetime.now(timezone.utc),
    )
```

(Leave the rest of `enqueue` — the `find_match`/`_promote_to_matched` block — unchanged.)

- [ ] **Step 5: Run the tests**

Run: `pytest tests/api/test_matchmaking_inline_deck.py -v`
Expected: PASS.

Run: `pytest tests/ -k matchmaking -v` (regression smoke on existing matchmaking tests).
Expected: no regressions.

- [ ] **Step 6: Commit**

```bash
git add digimon_gym/routers/matchmaking.py tests/api/test_matchmaking_inline_deck.py
git commit -m "feat(matchmaking): accept inline deck payload for guest queue requests"
```

---

### Task 5: `POST /models/{id}/prepare` resolves manifest → cached ONNX path

Keeps `games.py` as an engine-only router (CLAUDE.md Working Rule #11).
The frontend calls this endpoint first to obtain a filename, then passes
the filename to the unchanged `POST /games`.

**Files:**
- Modify: `digimon_gym/engine/model_utils.py`
- Modify: `digimon_gym/db/routers/admin_models.py` (add to `public_router`)
- Modify: `digimon_gym/db/schemas.py` (add `PrepareModelResponse`)
- Create: `tests/api/test_games_manifest_resolution.py`

- [ ] **Step 1: Write the failing test**

Create `tests/api/test_games_manifest_resolution.py`:

```python
"""POST /games resolves player{1,2}_model as a manifest UUID by streaming
the ONNX blob out of DO Spaces into a sha256-keyed local cache."""
from __future__ import annotations

import hashlib
import uuid as _uuid
from pathlib import Path
from unittest.mock import patch

import pytest
from httpx import AsyncClient
from sqlalchemy import select

from digimon_gym.db.models import AIModel
from digimon_gym.engine.model_utils import (
    resolve_manifest_model_path,
    _manifest_cache_dir,
)


def _fake_onnx_bytes() -> bytes:
    # Not a real ONNX file; the test uses a mocked `download_and_hash`
    # that returns the sha256 of whatever bytes we configure.
    return b"fake-onnx-model-payload"


@pytest.mark.asyncio
async def test_resolve_manifest_writes_to_sha_keyed_cache(db_session, tmp_path, monkeypatch) -> None:
    model_id = str(_uuid.uuid4())
    payload = _fake_onnx_bytes()
    expected_sha = hashlib.sha256(payload).hexdigest()
    row = AIModel(
        id=model_id,
        name="test",
        model_type="mlp",
        tensor_size=1375,
        action_space_size=2168,
        spaces_key=f"models/{model_id}/policy.onnx",
        file_sha256=expected_sha,
        file_size_bytes=len(payload),
        state="uploaded",
        published=True,
    )
    db_session.add(row)
    await db_session.commit()

    monkeypatch.setenv("MANIFEST_MODEL_CACHE_DIR", str(tmp_path))

    def fake_download(key: str, dest: str) -> tuple[str, int]:
        Path(dest).write_bytes(payload)
        return expected_sha, len(payload)

    with patch("digimon_gym.storage.spaces.download_and_hash", side_effect=fake_download):
        path_str = await resolve_manifest_model_path(db_session, model_id)

    cached = Path(path_str)
    assert cached.exists()
    assert cached.parent == _manifest_cache_dir()
    assert cached.name == f"{expected_sha}.onnx"
    assert cached.read_bytes() == payload


@pytest.mark.asyncio
async def test_resolve_manifest_hits_cache_on_second_call(db_session, tmp_path, monkeypatch) -> None:
    model_id = str(_uuid.uuid4())
    payload = _fake_onnx_bytes()
    expected_sha = hashlib.sha256(payload).hexdigest()
    db_session.add(AIModel(
        id=model_id, name="t", model_type="mlp",
        tensor_size=1375, action_space_size=2168,
        spaces_key=f"models/{model_id}/policy.onnx",
        file_sha256=expected_sha, file_size_bytes=len(payload),
        state="uploaded", published=True,
    ))
    await db_session.commit()
    monkeypatch.setenv("MANIFEST_MODEL_CACHE_DIR", str(tmp_path))

    calls = {"n": 0}

    def fake_download(key: str, dest: str) -> tuple[str, int]:
        calls["n"] += 1
        Path(dest).write_bytes(payload)
        return expected_sha, len(payload)

    with patch("digimon_gym.storage.spaces.download_and_hash", side_effect=fake_download):
        await resolve_manifest_model_path(db_session, model_id)
        await resolve_manifest_model_path(db_session, model_id)

    assert calls["n"] == 1, "second call should hit the on-disk cache"


@pytest.mark.asyncio
async def test_resolve_manifest_rejects_unknown_id(db_session) -> None:
    with pytest.raises(FileNotFoundError) as excinfo:
        await resolve_manifest_model_path(db_session, "does-not-exist")
    assert "manifest" in str(excinfo.value).lower()
```

Run: `pytest tests/api/test_games_manifest_resolution.py -v`
Expected: FAIL — `ImportError: cannot import name 'resolve_manifest_model_path'`.

- [ ] **Step 2: Implement the resolver**

In `digimon_gym/engine/model_utils.py`, append:

```python
import asyncio
import os
import re

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession


_UUID_RE = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", re.I)


def looks_like_manifest_id(s: str) -> bool:
    """True if `s` parses as a UUID (the shape of AIModel.id)."""
    return bool(_UUID_RE.match(s))


def _manifest_cache_dir() -> Path:
    """Server-side cache for manifest-fetched ONNX blobs, keyed by sha256."""
    return Path(os.environ.get("MANIFEST_MODEL_CACHE_DIR", "/tmp/digimon-models"))


async def resolve_manifest_model_path(db: AsyncSession, manifest_id: str) -> str:
    """Resolve an AIModel row id to a local `.onnx` path.

    First call for a given sha256 streams the blob out of DO Spaces. Every
    subsequent call (same sha256) returns the cached path without hitting
    Spaces.
    """
    from digimon_gym.db.models import AIModel  # lazy: engine shouldn't hard-dep on db
    from digimon_gym.storage import spaces

    row = (await db.execute(
        select(AIModel).where(AIModel.id == manifest_id)
    )).scalar_one_or_none()
    if row is None or not row.published or row.state != "uploaded":
        raise FileNotFoundError(
            f"manifest model not found or not published: {manifest_id}"
        )

    cache_dir = _manifest_cache_dir()
    cache_dir.mkdir(parents=True, exist_ok=True)
    cache_path = cache_dir / f"{row.file_sha256}.onnx"
    if cache_path.exists():
        return str(cache_path)

    # Cold: stream out of Spaces.
    await asyncio.to_thread(
        spaces.download_and_hash, row.spaces_key, str(cache_path)
    )
    return str(cache_path)
```

- [ ] **Step 3: Run the resolver tests**

Run: `pytest tests/api/test_games_manifest_resolution.py::test_resolve_manifest_writes_to_sha_keyed_cache tests/api/test_games_manifest_resolution.py::test_resolve_manifest_hits_cache_on_second_call tests/api/test_games_manifest_resolution.py::test_resolve_manifest_rejects_unknown_id -v`
Expected: PASS.

- [ ] **Step 4: Write the endpoint test**

Append to `tests/api/test_games_manifest_resolution.py`:

```python
@pytest.mark.asyncio
async def test_prepare_model_returns_filename_and_caches(
    client: AsyncClient, db_session, tmp_path, monkeypatch
) -> None:
    model_id = str(_uuid.uuid4())
    payload = _fake_onnx_bytes()
    sha = hashlib.sha256(payload).hexdigest()
    db_session.add(AIModel(
        id=model_id, name="t", model_type="mlp",
        tensor_size=1375, action_space_size=2168,
        spaces_key=f"models/{model_id}/policy.onnx",
        file_sha256=sha, file_size_bytes=len(payload),
        state="uploaded", published=True,
    ))
    await db_session.commit()
    monkeypatch.setenv("MANIFEST_MODEL_CACHE_DIR", str(tmp_path))
    monkeypatch.setenv("ONNX_MODELS_DIR", str(tmp_path))  # so /games sees the same dir

    def fake_dl(key, dest):
        Path(dest).write_bytes(payload)
        return sha, len(payload)

    with patch("digimon_gym.storage.spaces.download_and_hash", side_effect=fake_dl):
        resp = await client.post(f"/models/{model_id}/prepare")

    assert resp.status_code == 200, resp.text
    body = resp.json()
    assert body["filename"] == f"{sha}.onnx"
    assert body["cached"] is True
    # The file is under the configured models dir so /games can resolve it.
    assert (tmp_path / body["filename"]).exists()


@pytest.mark.asyncio
async def test_prepare_model_404_on_unknown_id(client: AsyncClient) -> None:
    resp = await client.post("/models/00000000-0000-0000-0000-000000000000/prepare")
    assert resp.status_code == 404
```

Run: `pytest tests/api/test_games_manifest_resolution.py -v -k prepare_model`
Expected: FAIL — endpoint doesn't exist.

- [ ] **Step 5: Add the endpoint**

In `digimon_gym/db/schemas.py`, near `ManifestModel`:

```python
class PrepareModelResponse(BaseModel):
    filename: str
    cached: bool
```

In `digimon_gym/db/routers/admin_models.py`, import the resolver and add an endpoint on the existing `public_router`:

```python
from digimon_gym.engine.model_utils import (
    get_models_dir,
    resolve_manifest_model_path,
)


@public_router.post("/{model_id}/prepare", response_model=PrepareModelResponse)
async def prepare_model(
    model_id: str,
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
        path = await resolve_manifest_model_path(db, model_id)
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
    return PrepareModelResponse(filename=filename, cached=True)
```

Add the schema to the import block at the top of `admin_models.py`:

```python
from digimon_gym.db.schemas import (
    ...,
    PrepareModelResponse,
)
```

Also add `from pathlib import Path` if not already imported.

- [ ] **Step 6: Run the endpoint test**

Run: `pytest tests/api/test_games_manifest_resolution.py -v -k prepare_model`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add digimon_gym/engine/model_utils.py digimon_gym/db/routers/admin_models.py digimon_gym/db/schemas.py tests/api/test_games_manifest_resolution.py
git commit -m "feat(models): POST /models/{id}/prepare stages manifest ONNX for /games"
```

---

## Phase 2 — Frontend foundation: Tauri deck storage + guest bootstrap

### Task 6: Tauri `deck_storage` commands

**Files:**
- Create: `src-tauri/src/deck_storage.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write the Rust unit tests (inline)**

Create `src-tauri/src/deck_storage.rs`:

```rust
//! Local deck storage for the desktop build. Decks are per-app JSON files
//! under `app_data_dir()/decks/<deck_id>.json`. Listing scans the dir.
//!
//! Shapes mirror `frontend/src/api/deckApi.ts::DeckResponse` so the TS
//! side can treat them identically to server-returned decks.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub game_mode: String,
    pub main_deck: Vec<String>,
    pub egg_deck: Vec<String>,
    #[serde(default)]
    pub main_deck_alt_arts: Vec<bool>,
    #[serde(default)]
    pub egg_deck_alt_arts: Vec<bool>,
    #[serde(default)]
    pub commander_id: Option<String>,
    #[serde(default)]
    pub is_valid: bool,
    #[serde(default)]
    pub validation_errors: Vec<String>,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub meta_tier: Option<String>,
    #[serde(default)]
    pub meta_archetype: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckSummary {
    pub id: String,
    pub name: String,
    pub game_mode: String,
    pub main_deck_size: usize,
    pub egg_deck_size: usize,
    #[serde(default)]
    pub meta_tier: Option<String>,
    #[serde(default)]
    pub meta_archetype: Option<String>,
    pub updated_at: String,
}

fn decks_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir unavailable: {e}"))?;
    let dir = base.join("decks");
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("create decks dir: {e}"))?;
    }
    Ok(dir)
}

fn read_deck_file(path: &Path) -> Option<Deck> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[tauri::command]
pub fn decks_list(app: AppHandle) -> Result<Vec<DeckSummary>, String> {
    let dir = decks_dir(&app)?;
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("read decks dir: {e}"))? {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if let Some(deck) = read_deck_file(&entry.path()) {
            out.push(DeckSummary {
                id: deck.id,
                name: deck.name,
                game_mode: deck.game_mode,
                main_deck_size: deck.main_deck.len(),
                egg_deck_size: deck.egg_deck.len(),
                meta_tier: deck.meta_tier,
                meta_archetype: deck.meta_archetype,
                updated_at: deck.updated_at,
            });
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

#[tauri::command]
pub fn decks_get(app: AppHandle, deck_id: String) -> Result<Deck, String> {
    let path = decks_dir(&app)?.join(format!("{deck_id}.json"));
    read_deck_file(&path).ok_or_else(|| format!("deck not found: {deck_id}"))
}

#[tauri::command]
pub fn decks_put(app: AppHandle, deck: Deck) -> Result<Deck, String> {
    let dir = decks_dir(&app)?;
    // Assign an ID for new decks.
    let mut deck = deck;
    if deck.id.is_empty() {
        deck.id = Uuid::new_v4().to_string();
    }
    let now = chrono::Utc::now().to_rfc3339();
    if deck.created_at.is_empty() {
        deck.created_at = now.clone();
    }
    deck.updated_at = now;
    let path = dir.join(format!("{}.json", deck.id));
    let json = serde_json::to_vec_pretty(&deck)
        .map_err(|e| format!("serialize deck: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write deck: {e}"))?;
    Ok(deck)
}

#[tauri::command]
pub fn decks_delete(app: AppHandle, deck_id: String) -> Result<bool, String> {
    let path = decks_dir(&app)?.join(format!("{deck_id}.json"));
    match fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("delete deck: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Mini harness: implement the IO against an explicit dir so we don't need a Tauri AppHandle.
    fn write_deck(dir: &Path, deck: &Deck) {
        let path = dir.join(format!("{}.json", deck.id));
        fs::write(path, serde_json::to_vec(deck).unwrap()).unwrap();
    }

    fn sample_deck(id: &str) -> Deck {
        Deck {
            id: id.into(),
            owner_id: "guest_abc".into(),
            name: format!("deck-{id}"),
            description: String::new(),
            game_mode: "standard".into(),
            main_deck: vec!["BT1-001".into(); 50],
            egg_deck: vec!["BT1-002".into(); 5],
            main_deck_alt_arts: vec![],
            egg_deck_alt_arts: vec![],
            commander_id: None,
            is_valid: true,
            validation_errors: vec![],
            is_public: false,
            tags: vec![],
            meta_tier: None,
            meta_archetype: None,
            created_at: "2026-04-18T00:00:00Z".into(),
            updated_at: "2026-04-18T00:00:00Z".into(),
        }
    }

    #[test]
    fn list_returns_empty_when_no_decks() {
        let tmp = TempDir::new().unwrap();
        let entries: Vec<_> = fs::read_dir(tmp.path()).unwrap().collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn round_trip_single_deck() {
        let tmp = TempDir::new().unwrap();
        let deck = sample_deck("d1");
        write_deck(tmp.path(), &deck);
        let back = read_deck_file(&tmp.path().join("d1.json")).unwrap();
        assert_eq!(back.id, "d1");
        assert_eq!(back.main_deck.len(), 50);
    }

    #[test]
    fn malformed_json_returns_none_not_panic() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("broken.json"), b"{not json").unwrap();
        assert!(read_deck_file(&tmp.path().join("broken.json")).is_none());
    }
}
```

- [ ] **Step 2: Register the module + commands**

In `src-tauri/src/main.rs`, at the top with the other `mod` declarations:

```rust
mod deck_storage;
```

In the `.invoke_handler(tauri::generate_handler![...])` block, append:

```rust
            deck_storage::decks_list,
            deck_storage::decks_get,
            deck_storage::decks_put,
            deck_storage::decks_delete,
```

- [ ] **Step 3: Add deps if missing**

Check `src-tauri/Cargo.toml` has `uuid`, `chrono`, `tempfile` (dev-dep). If not, add:

```toml
[dependencies]
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml deck_storage -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/deck_storage.rs src-tauri/src/main.rs src-tauri/Cargo.toml
git commit -m "feat(desktop): local deck storage via Tauri commands"
```

---

### Task 7: Frontend `deckStore` wrapper

**Files:**
- Create: `frontend/src/storage/deckStore.ts`

- [ ] **Step 1: Write the wrapper**

Create `frontend/src/storage/deckStore.ts`:

```typescript
// Client-side deck storage for desktop builds. Backed by Tauri commands
// in `src-tauri/src/deck_storage.rs`. The shapes match `deckApi.ts` so
// callers can swap behind a single import.

import { invoke } from '@tauri-apps/api/core';

export interface Deck {
  id: string;
  owner_id: string;
  name: string;
  description: string;
  game_mode: string;
  main_deck: string[];
  egg_deck: string[];
  main_deck_alt_arts?: boolean[];
  egg_deck_alt_arts?: boolean[];
  commander_id: string | null;
  is_valid: boolean;
  validation_errors: string[];
  is_public: boolean;
  tags: string[];
  meta_tier?: string | null;
  meta_archetype?: string | null;
  created_at: string;
  updated_at: string;
}

export interface DeckSummary {
  id: string;
  name: string;
  game_mode: string;
  main_deck_size: number;
  egg_deck_size: number;
  meta_tier?: string | null;
  meta_archetype?: string | null;
  updated_at: string;
}

export async function listDecks(): Promise<DeckSummary[]> {
  return invoke<DeckSummary[]>('decks_list');
}

export async function getDeck(deckId: string): Promise<Deck> {
  return invoke<Deck>('decks_get', { deckId });
}

export async function putDeck(deck: Partial<Deck> & {
  name: string;
  game_mode: string;
  main_deck: string[];
  egg_deck: string[];
}): Promise<Deck> {
  // Fill required fields so the Rust struct deserializes cleanly.
  const now = new Date().toISOString();
  const full: Deck = {
    id: deck.id ?? '',
    owner_id: deck.owner_id ?? 'guest',
    name: deck.name,
    description: deck.description ?? '',
    game_mode: deck.game_mode,
    main_deck: deck.main_deck,
    egg_deck: deck.egg_deck,
    main_deck_alt_arts: deck.main_deck_alt_arts ?? [],
    egg_deck_alt_arts: deck.egg_deck_alt_arts ?? [],
    commander_id: deck.commander_id ?? null,
    is_valid: deck.is_valid ?? false,
    validation_errors: deck.validation_errors ?? [],
    is_public: deck.is_public ?? false,
    tags: deck.tags ?? [],
    meta_tier: deck.meta_tier ?? null,
    meta_archetype: deck.meta_archetype ?? null,
    created_at: deck.created_at ?? now,
    updated_at: now,
  };
  return invoke<Deck>('decks_put', { deck: full });
}

export async function deleteDeck(deckId: string): Promise<boolean> {
  return invoke<boolean>('decks_delete', { deckId });
}
```

- [ ] **Step 2: Quick compile check via frontend**

Run: `cd frontend && npm run typecheck`
Expected: no errors in `storage/deckStore.ts`. (If there is no `typecheck` script, use `npx tsc --noEmit`.)

- [ ] **Step 3: Commit**

```bash
git add frontend/src/storage/deckStore.ts
git commit -m "feat(desktop-fe): deckStore wrapper over Tauri deck commands"
```

---

### Task 8: Guest bootstrap

**Files:**
- Create: `frontend/src/bootstrap/guest.ts`
- Create: `frontend/src/bootstrap/guest.test.ts`
- Modify: `frontend/src/stores/authStore.ts`

- [ ] **Step 1: Write the failing test**

Create `frontend/src/bootstrap/guest.test.ts`:

```typescript
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ensureGuestSession, GUEST_TOKEN_KEY, GUEST_USER_ID_KEY, GUEST_NAME_KEY } from './guest';

describe('ensureGuestSession', () => {
  const originalFetch = globalThis.fetch;
  beforeEach(() => {
    localStorage.clear();
  });
  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it('mints a new guest token if localStorage is empty', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 201,
      json: async () => ({
        access_token: 'guest-jwt',
        user_id: 'guest_abc',
        display_name: 'Guest-ABCD',
      }),
    }) as unknown as typeof fetch;

    const session = await ensureGuestSession();

    expect(session).toEqual({
      token: 'guest-jwt',
      userId: 'guest_abc',
      displayName: 'Guest-ABCD',
    });
    expect(localStorage.getItem(GUEST_TOKEN_KEY)).toBe('guest-jwt');
    expect(localStorage.getItem(GUEST_USER_ID_KEY)).toBe('guest_abc');
    expect(localStorage.getItem(GUEST_NAME_KEY)).toBe('Guest-ABCD');
    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
  });

  it('reuses cached token on subsequent boots without hitting the network', async () => {
    localStorage.setItem(GUEST_TOKEN_KEY, 'existing');
    localStorage.setItem(GUEST_USER_ID_KEY, 'guest_existing');
    localStorage.setItem(GUEST_NAME_KEY, 'Guest-EEEE');
    const fetchMock = vi.fn();
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const session = await ensureGuestSession();

    expect(session.token).toBe('existing');
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('throws if the network mint fails so the UI can show an offline banner', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false, status: 500, text: async () => 'boom',
    }) as unknown as typeof fetch;
    await expect(ensureGuestSession()).rejects.toThrow(/guest session/i);
    expect(localStorage.getItem(GUEST_TOKEN_KEY)).toBeNull();
  });
});
```

Run: `cd frontend && npx vitest run src/bootstrap/guest.test.ts`
Expected: FAIL — module `./guest` does not exist.

- [ ] **Step 2: Implement the bootstrap**

Create `frontend/src/bootstrap/guest.ts`:

```typescript
/**
 * Anonymous guest-session bootstrap for the desktop alpha build.
 *
 * On first launch, POST /auth/guest and cache the JWT + display name in
 * localStorage. On subsequent launches, the cached token is reused.
 *
 * Policy: we never silently re-mint a token on 401. Losing the guest
 * user_id silently mid-session would be surprising; if a 401 comes back
 * for a decodable token, the caller surfaces it as an auth error.
 */

export const GUEST_TOKEN_KEY = 'guest_access_token';
export const GUEST_USER_ID_KEY = 'guest_user_id';
export const GUEST_NAME_KEY = 'guest_display_name';

export interface GuestSession {
  token: string;
  userId: string;
  displayName: string;
}

const API_BASE = (import.meta.env.VITE_API_URL as string | undefined) ?? '';

interface GuestResponse {
  access_token: string;
  user_id: string;
  display_name: string;
}

export async function ensureGuestSession(): Promise<GuestSession> {
  const cachedToken = localStorage.getItem(GUEST_TOKEN_KEY);
  const cachedId = localStorage.getItem(GUEST_USER_ID_KEY);
  const cachedName = localStorage.getItem(GUEST_NAME_KEY);
  if (cachedToken && cachedId && cachedName) {
    return { token: cachedToken, userId: cachedId, displayName: cachedName };
  }

  const resp = await fetch(`${API_BASE}/auth/guest`, { method: 'POST' });
  if (!resp.ok) {
    throw new Error(`Failed to mint guest session (${resp.status})`);
  }
  const body = (await resp.json()) as GuestResponse;
  localStorage.setItem(GUEST_TOKEN_KEY, body.access_token);
  localStorage.setItem(GUEST_USER_ID_KEY, body.user_id);
  localStorage.setItem(GUEST_NAME_KEY, body.display_name);
  return {
    token: body.access_token,
    userId: body.user_id,
    displayName: body.display_name,
  };
}
```

- [ ] **Step 3: Run tests**

Run: `cd frontend && npx vitest run src/bootstrap/guest.test.ts`
Expected: PASS.

- [ ] **Step 4: Wire into `authStore.hydrate()`**

In `frontend/src/stores/authStore.ts`, replace the existing `hydrate` implementation with:

```typescript
  hydrate: async () => {
    const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

    if (IS_DESKTOP) {
      // Desktop alpha: mint/reuse a guest session. No account flows.
      const { ensureGuestSession } = await import('@/bootstrap/guest');
      try {
        const session = await ensureGuestSession();
        // The existing axios interceptor reads `access_token`; mirror the
        // guest token there so authenticated calls "just work" without
        // rewriting the interceptor.
        localStorage.setItem('access_token', session.token);
        set({
          accessToken: session.token,
          refreshToken: null,
          isAuthenticated: true,
          user: {
            id: session.userId,
            username: session.displayName,
            email: '',
            roles: [],
          },
        });
      } catch {
        // Offline: HomePage will render an offline banner and disable PvP + Try Online.
        localStorage.removeItem('access_token');
        set({
          accessToken: null,
          refreshToken: null,
          isAuthenticated: false,
          user: null,
        });
      }
      return;
    }

    // Web build (unchanged)
    const accessToken = localStorage.getItem('access_token');
    const refreshToken = localStorage.getItem('refresh_token');
    let user: User | null = null;
    if (accessToken) {
      try {
        user = await authApi.getMe();
      } catch {
        user = null;
      }
    }
    set({
      accessToken,
      refreshToken,
      isAuthenticated: !!accessToken,
      user,
    });
  },
```

- [ ] **Step 5: Commit**

```bash
git add frontend/src/bootstrap/guest.ts frontend/src/bootstrap/guest.test.ts frontend/src/stores/authStore.ts
git commit -m "feat(desktop-fe): guest session bootstrap on app hydrate"
```

---

### Task 9: `AuthGuard` gates on guest token in desktop

**Files:**
- Modify: `frontend/src/components/auth/AuthGuard.tsx`

- [ ] **Step 1: Read the current AuthGuard**

Run: `cat frontend/src/components/auth/AuthGuard.tsx`

- [ ] **Step 2: Update the gate**

Replace the existing gate with:

```typescript
import { Navigate, Outlet } from 'react-router-dom';
import { useAuthStore } from '@/stores/authStore';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

export function AuthGuard() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  if (isAuthenticated) {
    return <Outlet />;
  }
  if (IS_DESKTOP) {
    // Desktop: hydrate mints a guest session on boot; if it failed (offline),
    // fall through to home so the offline banner can render.
    return <Navigate to="/" replace />;
  }
  return <Navigate to="/login" replace />;
}
```

- [ ] **Step 3: Commit**

```bash
git add frontend/src/components/auth/AuthGuard.tsx
git commit -m "feat(desktop-fe): AuthGuard accepts guest sessions"
```

---

### Task 10: DeckBuilder — switch to deckStore + lock to standard

**Files:**
- Modify: `frontend/src/pages/DeckBuilderPage.tsx`

- [ ] **Step 1: Locate the save/load + format-picker code**

Run: `rg -n "listDecks|getDeck|createDeck|updateDeck|game_mode" frontend/src/pages/DeckBuilderPage.tsx`
Note the line ranges where the API calls live and where the game-mode picker UI lives.

- [ ] **Step 2: Replace `@/api/deckApi` imports with `@/storage/deckStore` for desktop**

At the top of `DeckBuilderPage.tsx`:

```typescript
const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';
import * as deckApi from '@/api/deckApi';
import * as deckStore from '@/storage/deckStore';

// Desktop uses the local deck store; web falls back to the hosted API.
const decks = IS_DESKTOP ? deckStore : deckApi;
```

Then use `decks.listDecks()`, `decks.getDeck(id)`, etc. For save flows, desktop goes through `deckStore.putDeck({...})` which returns the saved deck. Web's `createDeck` / `updateDeck` keep their existing shape.

- [ ] **Step 3: Remove the format picker in desktop**

Locate the JSX that renders the `<select>` / radio group for `game_mode`. Wrap its render in:

```tsx
{!IS_DESKTOP && (
  /* existing format picker JSX */
)}
```

In state initialization, set `gameMode` to `'standard'` and don't allow changes on desktop. Remove any validation that depends on other formats.

- [ ] **Step 4: Typecheck**

Run: `cd frontend && npx tsc --noEmit`
Expected: no errors in `DeckBuilderPage.tsx`.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/pages/DeckBuilderPage.tsx
git commit -m "feat(desktop-fe): DeckBuilder uses deckStore; locked to standard format"
```

---

### Task 11: LobbyPage — deckStore + inline deck matchmaking + hide Browse

**Files:**
- Modify: `frontend/src/pages/LobbyPage.tsx`
- Modify: `frontend/src/api/matchmaking.ts`
- Modify: `frontend/src/hooks/useMatchmaking.ts` (if it crafts `QueueRequest`)

- [ ] **Step 1: Widen the `QueueRequest` TS type**

In `frontend/src/api/matchmaking.ts`, replace `QueueRequest` with:

```typescript
export interface QueueRequest {
  queue_type: QueueType;
  // One of these two shapes:
  deck_id?: string;
  main_deck?: string[];
  egg_deck?: string[];
  game_mode?: string;
  opponent_tier_filter?: TierFilter;
}
```

- [ ] **Step 2: Replace `deckApiMod` with `deckStore` in LobbyPage (desktop only)**

At the top of `LobbyPage.tsx`:

```typescript
const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';
import * as deckApiMod from '@/api/deckApi';
import * as deckStore from '@/storage/deckStore';
const decks = IS_DESKTOP ? deckStore : deckApiMod;
```

Replace all `deckApiMod.listDecks()` / `deckApiMod.getDeck(...)` with `decks.listDecks()` / `decks.getDeck(...)`. The returned summary and deck shapes are compatible (both expose `id`, `name`, `main_deck`, `egg_deck`).

- [ ] **Step 3: Build the inline-deck queue request in the Play tab**

Find the `handleQueue` callback. Rewrite it:

```typescript
const handleQueue = useCallback(async () => {
  if (!playDeckId) return;
  // Always send the inline deck shape in desktop — the guest user has no
  // server-side Deck row to reference.
  const deck = await decks.getDeck(playDeckId);
  void matchmaking.enqueue({
    queue_type: playQueueType,
    main_deck: deck.main_deck,
    egg_deck: deck.egg_deck,
    game_mode: deck.game_mode,
    opponent_tier_filter: playQueueType === 'casual' ? playTierFilter : undefined,
  });
}, [matchmaking, playDeckId, playQueueType, playTierFilter]);
```

- [ ] **Step 4: Hide the Browse tab for alpha**

In the tab state and the tab-button map, remove `'browse'`:

```typescript
const [tab, setTab] = useState<'play' | 'create' | 'join'>('play');

{(['play', 'create', 'join'] as const).map((t) => (
  /* ... */
))}
```

Delete the Browse tab JSX block and the `refreshGames` / `games` / `loadingGames` state + effect. Remove the `import type { LobbyGame }` line.

- [ ] **Step 5: Typecheck + smoke**

Run: `cd frontend && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/api/matchmaking.ts frontend/src/pages/LobbyPage.tsx
git commit -m "feat(desktop-fe): Lobby uses deckStore + inline matchmaking payload; hide Browse"
```

---

## Phase 3 — Frontend UI polish

### Task 12: HomePage redesign + alpha banner

**Files:**
- Create: `frontend/src/components/home/AlphaBanner.tsx`
- Modify: `frontend/src/pages/HomePage.tsx`

- [ ] **Step 1: Build the banner**

Create `frontend/src/components/home/AlphaBanner.tsx`:

```typescript
import { Link } from 'react-router-dom';

export function AlphaBanner() {
  return (
    <div className="mb-6 rounded border border-amber-600 bg-amber-900/20 p-3 text-sm text-amber-100">
      <span className="mr-2 rounded bg-amber-600 px-1.5 py-0.5 text-xs font-bold uppercase text-amber-950">
        Alpha
      </span>
      Card-effect coverage is incomplete. See{' '}
      <Link to="/patch-notes" className="underline hover:text-amber-200">
        patch notes
      </Link>{' '}
      for what works today.
    </div>
  );
}
```

- [ ] **Step 2: Redesign HomePage**

Replace the body of `frontend/src/pages/HomePage.tsx`:

```typescript
import { Link } from 'react-router-dom';
import { AlphaBanner } from '@/components/home/AlphaBanner';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

interface HomeCard {
  to: string;
  title: string;
  desc: string;
  primary?: boolean;
}

const CARDS: HomeCard[] = [
  { to: '/lobby', title: 'Find Match', desc: 'Queue up for casual or ranked play', primary: true },
  { to: '/models', title: 'Play vs AI', desc: 'Try or download AI opponents', primary: true },
  { to: '/deckbuilder', title: 'Deck Builder', desc: 'Build and manage your decks' },
  { to: '/patch-notes', title: 'Patch Notes', desc: 'What is new, what is known-broken' },
];

export function HomePage() {
  return (
    <div className="mx-auto max-w-4xl px-4 py-10">
      <AlphaBanner />
      <h1 className="mb-2 text-4xl font-bold text-gray-100">Digimon TCG Simulator</h1>
      <p className="mb-8 text-gray-400">
        {IS_DESKTOP
          ? 'Play, build decks, and face AI opponents. Multiplayer is online.'
          : 'Play, build decks, and train AI agents.'}
      </p>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        {CARDS.map((c) => (
          <Link
            key={c.to}
            to={c.to}
            className={`block rounded-lg border p-6 transition-all ${
              c.primary
                ? 'border-blue-600 bg-blue-900/20 hover:border-blue-500 hover:bg-blue-900/30'
                : 'border-gray-700 bg-gray-800 hover:border-gray-600 hover:bg-gray-750'
            }`}
          >
            <h2 className="mb-2 text-xl font-semibold text-gray-100">{c.title}</h2>
            <p className="text-sm text-gray-400">{c.desc}</p>
          </Link>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Visual smoke**

Run: `cd frontend && npm run dev` and open the home route.
Expected: four cards render, alpha banner visible, patch notes link works.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/components/home/AlphaBanner.tsx frontend/src/pages/HomePage.tsx
git commit -m "feat(desktop-fe): redesign home page with alpha banner and four-card grid"
```

---

### Task 13: ModelsPage — merged table with Try Online

**Files:**
- Modify: `frontend/src/pages/ModelsPage.tsx`
- Modify: `frontend/src/api/gameApi.ts` (add `createVsAiGame` helper)

- [ ] **Step 1: Add `createVsAiGame` helper**

In `frontend/src/api/gameApi.ts`, append:

```typescript
/** Stage a manifest model on the server, then create a server-side
 *  vs-AI game. Used by the Models page `Try online` button. */
export async function createVsAiGame(params: {
  modelId: string;
  userDeck: { main_deck: string[]; egg_deck: string[] };
  opponentDeck: { main_deck: string[]; egg_deck: string[] };
}): Promise<{ game_id: string }> {
  // Step 1: ask the server to stage the ONNX blob by model_id.
  const { data: prepared } = await client.post<{ filename: string; cached: boolean }>(
    `/models/${params.modelId}/prepare`,
  );
  // Step 2: create the game against the staged filename.
  const { data } = await client.post<{ game_id: string }>('/games', {
    deck1: [...params.userDeck.egg_deck, ...params.userDeck.main_deck],
    deck2: [...params.opponentDeck.egg_deck, ...params.opponentDeck.main_deck],
    player1_type: 'human',
    player2_type: 'agent',
    player1_policy: 'human',
    player2_policy: 'trained',
    player2_model: prepared.filename,
  });
  return data;
}
```

- [ ] **Step 2: Read the current ModelsPage for row state**

Run: `rg -n "manifest|local|isCompatible|handleDownload|handleDelete" frontend/src/pages/ModelsPage.tsx | head -40`

- [ ] **Step 3: Rewrite ModelsPage with a unified table**

Replace `frontend/src/pages/ModelsPage.tsx` with the following (keeps the existing download/delete/activate plumbing, adds Try Online, drops the manifest-URL input):

```typescript
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  deleteModel as deleteModelApi,
  downloadModel,
  engineContract,
  fetchManifest,
  isCompatible,
  listLocal,
  loadCached,
  type EngineContract,
  type LocalModelMeta,
  type ManifestModel,
} from '@/api/desktopModelsApi';
import * as deckStore from '@/storage/deckStore';
import { createVsAiGame } from '@/api/gameApi';

const MANIFEST_URL =
  (import.meta.env.VITE_MODELS_MANIFEST_URL as string | undefined) ?? '';

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

interface MergedRow {
  id: string;
  manifest?: ManifestModel;
  local?: LocalModelMeta;
}

export function ModelsPage() {
  const navigate = useNavigate();
  const [contract, setContract] = useState<EngineContract | null>(null);
  const [manifest, setManifest] = useState<ManifestModel[]>([]);
  const [local, setLocal] = useState<LocalModelMeta[]>([]);
  const [busyIds, setBusyIds] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [decks, setDecks] = useState<deckStore.DeckSummary[]>([]);
  const [playDeckId, setPlayDeckId] = useState<string>('');

  const refresh = useCallback(async () => {
    setError(null);
    try {
      const [c, m, l, d] = await Promise.all([
        engineContract(),
        MANIFEST_URL ? fetchManifest(MANIFEST_URL) : Promise.resolve([] as ManifestModel[]),
        listLocal(),
        deckStore.listDecks(),
      ]);
      setContract(c);
      setManifest(m);
      setLocal(l);
      setDecks(d);
      if (d.length > 0 && !playDeckId) setPlayDeckId(d[0].id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [playDeckId]);

  useEffect(() => {
    void refresh();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const rows = useMemo<MergedRow[]>(() => {
    const byId = new Map<string, MergedRow>();
    for (const m of manifest) byId.set(m.id, { id: m.id, manifest: m });
    for (const l of local) {
      const row = byId.get(l.id) ?? { id: l.id };
      row.local = l;
      byId.set(l.id, row);
    }
    return Array.from(byId.values());
  }, [manifest, local]);

  const withBusy = useCallback(async (id: string, fn: () => Promise<void>) => {
    setBusyIds((prev) => new Set(prev).add(id));
    try {
      await fn();
    } finally {
      setBusyIds((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    }
  }, []);

  const handleDownload = (m: ManifestModel) =>
    withBusy(m.id, async () => {
      await downloadModel(m);
      await refresh();
    });

  const handleDelete = (id: string) =>
    withBusy(id, async () => {
      if (!window.confirm(`Delete cached model ${id}?`)) return;
      await deleteModelApi(id);
      await refresh();
    });

  const handleActivate = (id: string) => withBusy(id, () => loadCached(id).then());

  const handleTryOnline = async (row: MergedRow) => {
    if (!playDeckId) {
      setError('Select a deck to play with first.');
      return;
    }
    await withBusy(row.id, async () => {
      try {
        const deck = await deckStore.getDeck(playDeckId);
        // For now the opponent uses the same deck — the manifest entry may
        // advertise an intended deck in a future iteration.
        const { game_id } = await createVsAiGame({
          modelId: row.id,
          userDeck: { main_deck: deck.main_deck, egg_deck: deck.egg_deck },
          opponentDeck: { main_deck: deck.main_deck, egg_deck: deck.egg_deck },
        });
        navigate(`/game/${game_id}?mode=vsai&player=1`);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    });
  };

  return (
    <div className="p-6 text-gray-100">
      <h1 className="mb-2 text-2xl font-bold">AI Models</h1>
      <p className="mb-4 text-sm text-gray-400">
        Try models online, or download them for offline play. Models whose
        tensor/action shapes don&apos;t match this build are greyed out.
      </p>

      {contract && (
        <div className="mb-6 text-xs text-gray-400">
          Engine contract: tensor={contract.tensor_size}, actions=
          {contract.action_space_size}
          {contract.engine_commit
            ? `, commit=${contract.engine_commit.slice(0, 10)}`
            : ''}
        </div>
      )}

      {error && (
        <div className="mb-4 rounded border border-red-500/40 bg-red-500/10 p-3 text-sm text-red-200">
          {error}
        </div>
      )}

      <div className="mb-6 flex items-end gap-3">
        <div>
          <label className="mb-1 block text-xs text-gray-400">Your deck</label>
          <select
            value={playDeckId}
            onChange={(e) => setPlayDeckId(e.target.value)}
            className="rounded border border-gray-600 bg-gray-700 px-3 py-2 text-sm text-white"
          >
            {decks.length === 0 ? (
              <option value="">No decks yet — build one in the Deck Builder</option>
            ) : (
              decks.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))
            )}
          </select>
        </div>
        <button
          onClick={() => void refresh()}
          className="rounded bg-gray-700 px-3 py-2 text-sm hover:bg-gray-600"
        >
          Refresh
        </button>
      </div>

      {loading ? (
        <p className="text-sm text-gray-400">Loading…</p>
      ) : rows.length === 0 ? (
        <p className="text-sm text-gray-500">
          No models available. Check that <code>VITE_MODELS_MANIFEST_URL</code> is set
          and the server is reachable.
        </p>
      ) : (
        <table className="w-full text-sm">
          <thead className="text-xs uppercase text-gray-400">
            <tr>
              <th className="py-2 pr-4 text-left">Name</th>
              <th className="py-2 pr-4 text-left">Type</th>
              <th className="py-2 pr-4 text-left">Size</th>
              <th className="py-2 pr-4 text-left">Status</th>
              <th className="py-2 text-right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => {
              const busy = busyIds.has(row.id);
              const compatible =
                !contract ||
                !row.manifest ||
                isCompatible(row.manifest, contract);
              const have = !!row.local;
              const name = row.manifest?.name ?? row.local?.name ?? row.id;
              const type = (row.manifest?.model_type ?? row.local?.model_type ?? '').toUpperCase();
              const size = row.manifest?.file_size_bytes ?? row.local?.file_size_bytes ?? 0;
              return (
                <tr key={row.id} className="border-t border-gray-700">
                  <td className="py-2 pr-4">{name}</td>
                  <td className="py-2 pr-4">{type}</td>
                  <td className="py-2 pr-4 text-gray-400">{formatBytes(size)}</td>
                  <td className="py-2 pr-4">
                    {have ? (
                      <span className="text-xs text-green-400">downloaded</span>
                    ) : compatible ? (
                      <span className="text-xs text-gray-400">online only</span>
                    ) : (
                      <span className="text-xs text-red-400">incompatible</span>
                    )}
                  </td>
                  <td className="py-2 text-right">
                    <div className="flex justify-end gap-2">
                      {row.manifest && compatible && (
                        <button
                          disabled={busy || !playDeckId}
                          onClick={() => void handleTryOnline(row)}
                          className="rounded bg-blue-600 px-2 py-1 text-xs hover:bg-blue-500 disabled:opacity-40"
                        >
                          {busy ? '…' : 'Try online'}
                        </button>
                      )}
                      {row.manifest && !have && compatible && (
                        <button
                          disabled={busy}
                          onClick={() => void handleDownload(row.manifest!)}
                          className="rounded bg-gray-700 px-2 py-1 text-xs hover:bg-gray-600 disabled:opacity-40"
                        >
                          Download
                        </button>
                      )}
                      {have && (
                        <>
                          <button
                            disabled={busy}
                            onClick={() => void handleActivate(row.id)}
                            className="rounded bg-green-700 px-2 py-1 text-xs hover:bg-green-600 disabled:opacity-40"
                          >
                            Activate
                          </button>
                          <button
                            disabled={busy}
                            onClick={() => void handleDelete(row.id)}
                            className="rounded bg-red-700 px-2 py-1 text-xs hover:bg-red-600 disabled:opacity-40"
                          >
                            Delete
                          </button>
                        </>
                      )}
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Typecheck + visual smoke**

Run: `cd frontend && npx tsc --noEmit`
Expected: no errors.

Manually: `npm run dev`, open `/models`, confirm the merged table renders (may be empty locally without manifest URL).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/pages/ModelsPage.tsx frontend/src/api/gameApi.ts
git commit -m "feat(desktop-fe): unified Models page with Try Online button"
```

---

### Task 14: GamePage handles `mode=vsai`

**Files:**
- Modify: `frontend/src/pages/GamePage.tsx`

- [ ] **Step 1: Find the mode-branching code**

Run: `rg -n "isPvpMode|mode=" frontend/src/pages/GamePage.tsx | head -20`

- [ ] **Step 2: Add a `vsai` branch**

In `GamePage.tsx`, replace:

```typescript
const isPvpMode = searchParams.get('mode') === 'pvp';
const isSpectator = searchParams.get('role') === 'spectator';
```

with:

```typescript
const mode = searchParams.get('mode');
const isPvpMode = mode === 'pvp';
const isVsAiOnline = mode === 'vsai';
const isSpectator = searchParams.get('role') === 'spectator';
// WebSocket is used for both PvP and vs-AI-online: the server streams
// state and runs the AI turn internally for the 'vsai' mode.
const useWebSocket = isPvpMode || isVsAiOnline || isSpectator;
```

Then replace the `wsOptions` guard:

```typescript
const wsOptions = useMemo<UseWebSocketGameOptions | null>(() => {
  if (!urlGameId || !useWebSocket) return null;
  return {
    gameId: urlGameId,
    role: isSpectator ? 'spectator' : 'player',
    onStateUpdate: (payload) => {
      store.setGameState(payload.state);
      store.setActionMask(payload.action_mask ?? []);
      if (payload.logs) store.appendLogs(payload.logs);
      if (payload.events) store.appendEvents(payload.events);
      if (payload.your_player_id != null) {
        store.setPlayerLabels({
          [payload.your_player_id]: 'You',
          [payload.your_player_id === 1 ? 2 : 1]: isVsAiOnline ? 'AI' : 'Opponent',
        });
      }
    },
    onGameOver: () => {},
    onError: (msg) => console.error('WebSocket error:', msg),
  };
}, [urlGameId, useWebSocket, isSpectator, isVsAiOnline]); // eslint-disable-line react-hooks/exhaustive-deps
```

Replace the `sendAction` ternary:

```typescript
const sendAction = useWebSocket ? ws.sendAction : httpSendAction;
```

- [ ] **Step 3: Typecheck**

Run: `cd frontend && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/pages/GamePage.tsx
git commit -m "feat(desktop-fe): GamePage handles mode=vsai via WebSocket"
```

---

### Task 15: Layout — drop admin/training links for desktop

**Files:**
- Modify: `frontend/src/components/layout/Layout.tsx`

- [ ] **Step 1: Locate the nav items**

Run: `rg -n "admin|training|gauntlet|arena|barracks" frontend/src/components/layout/Layout.tsx`

- [ ] **Step 2: Gate each admin/training link on `!IS_DESKTOP`**

At the top of `Layout.tsx`:

```typescript
const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';
```

Every nav item under `/admin/*` or `/training` gets wrapped. Example — what the existing admin nav probably looks like:

```tsx
<Link to="/admin/barracks">Barracks</Link>
<Link to="/admin/arena">Arena</Link>
<Link to="/admin/gauntlet">Gauntlet</Link>
<Link to="/admin/deck-pools">Deck Pools</Link>
<Link to="/admin/issues">Issues</Link>
<Link to="/admin/tasks">Tasks</Link>
<Link to="/admin/promotions">Promotions</Link>
<Link to="/admin/patch-notes">Patch Notes Admin</Link>
<Link to="/admin/models">Models Admin</Link>
```

Becomes:

```tsx
{!IS_DESKTOP && (
  <>
    <Link to="/admin/barracks">Barracks</Link>
    <Link to="/admin/arena">Arena</Link>
    <Link to="/admin/gauntlet">Gauntlet</Link>
    <Link to="/admin/deck-pools">Deck Pools</Link>
    <Link to="/admin/issues">Issues</Link>
    <Link to="/admin/tasks">Tasks</Link>
    <Link to="/admin/promotions">Promotions</Link>
    <Link to="/admin/patch-notes">Patch Notes Admin</Link>
    <Link to="/admin/models">Models Admin</Link>
  </>
)}
```

For the `Login` / `Register` links that appear when logged out, wrap similarly:

```tsx
{!IS_DESKTOP && !isAuthenticated && (
  <>
    <Link to="/login">Login</Link>
    <Link to="/register">Register</Link>
  </>
)}
```

Add a desktop-only `/models` link (the AI Models hub):

```tsx
{IS_DESKTOP && <Link to="/models">AI Models</Link>}
```

Keep `Home`, `Lobby`, `Deck Builder`, and `Patch Notes` always visible.

- [ ] **Step 3: Typecheck + visual**

Run: `cd frontend && npx tsc --noEmit`
Expected: no errors.

`npm run dev` and check the desktop build's sidebar is trimmed.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/components/layout/Layout.tsx
git commit -m "feat(desktop-fe): hide admin/training nav in desktop build"
```

---

### Task 16: Bake manifest URL + backend URL into desktop env

**Files:**
- Create/Modify: `frontend/.env.desktop`
- Modify: `frontend/vite.config.ts` (verify desktop build reads the env)

- [ ] **Step 1: Create `.env.desktop`**

Create `frontend/.env.desktop`:

```
VITE_BUILD_TARGET=desktop
VITE_API_URL=https://api.digimon-tcg.example
VITE_MODELS_MANIFEST_URL=https://api.digimon-tcg.example
```

Adjust the URL values to the actual production host before shipping.

- [ ] **Step 2: Verify the Tauri build script picks it up**

Inspect `src-tauri/tauri.conf.json` `beforeBuildCommand` / `beforeDevCommand`. If they call `npm run build` / `npm run dev`, make sure those scripts set `--mode desktop` (which tells Vite to load `.env.desktop`). Example `package.json` scripts:

```json
"scripts": {
  "dev:desktop": "vite --mode desktop",
  "build:desktop": "vite build --mode desktop"
}
```

And in `src-tauri/tauri.conf.json`:

```json
"build": {
  "beforeDevCommand": "npm run dev:desktop",
  "beforeBuildCommand": "npm run build:desktop"
}
```

- [ ] **Step 3: Build + sanity**

Run: `cd src-tauri && cargo tauri dev` (short-lived; confirm app launches and hits the configured API).

- [ ] **Step 4: Commit**

```bash
git add frontend/.env.desktop frontend/package.json src-tauri/tauri.conf.json
git commit -m "chore(desktop): bake API + manifest URLs into .env.desktop"
```

---

## Phase 4 — End-to-end verification

### Task 17: Manual smoke checklist

**Files:** none — documentation.

- [ ] **Step 1: Create `docs/RELEASE_SMOKE_ALPHA.md`**

```markdown
# Alpha desktop smoke checklist

## First launch (clean app-data dir)
- [ ] App launches and lands on Home
- [ ] Alpha banner is visible
- [ ] `localStorage.guest_access_token` is populated after Home paints
- [ ] Navigating to `/lobby`, `/deckbuilder`, `/models` works without redirect

## Deck Builder
- [ ] Create a standard deck, save it
- [ ] Re-open — deck list shows the saved deck
- [ ] No format picker visible

## Models page
- [ ] Manifest rows render (requires reachable API + DO Spaces)
- [ ] "Try online" on an undownloaded row navigates to `/game/<id>?mode=vsai`
  and the game starts
- [ ] Download a model, delete it, re-download — local row state updates

## Matchmaking
- [ ] From two separate app installs (or one app + one web client), queue
  into casual with the same deck, game starts
- [ ] Cancel a ticket — UI returns to the queue entry screen
```

- [ ] **Step 2: Commit**

```bash
git add docs/RELEASE_SMOKE_ALPHA.md
git commit -m "docs: alpha desktop smoke checklist"
```

- [ ] **Step 3: Run the checklist**

Execute every item. File a task for each failure; do not mark the feature shipped until the checklist passes on a clean machine.

---

## Self-review notes

**Spec coverage:**
- `POST /auth/guest` → Task 3 ✅
- Guest token long-lived → Task 2 ✅
- `POST /models/{id}/prepare` (Spaces → server cache for `/games`) → Task 5 ✅
- Matchmaking inline deck → Task 4 ✅
- Tauri deck storage → Task 6 ✅
- Frontend deckStore → Task 7 ✅
- Guest bootstrap → Task 8 ✅
- AuthGuard → Task 9 ✅
- DeckBuilder standard-locked → Task 10 ✅
- Lobby inline deck + hide Browse → Task 11 ✅
- HomePage redesign + alpha banner → Task 12 ✅
- ModelsPage merged table + Try online → Task 13 ✅
- GamePage mode=vsai → Task 14 ✅
- Layout desktop nav → Task 15 ✅
- Baked env URLs → Task 16 ✅
- Manual smoke → Task 17 ✅

**Known deferrals / follow-ups:**
- Engine-shape check on the server side of Try Online (currently only the client check gates the button). Follow-up ticket: reject `/games` with `player2_model=<manifest_id>` if the manifest row's shape doesn't match the running engine tensor/action sizes.
- Admin-uploaded deck for the AI opponent: right now the Try Online flow reuses the user's deck as the AI opponent's deck. A post-alpha improvement is to respect `ManifestModel.deck_id`.
- Desktop PvP re-using the WS matchmaking flow depends on the hosted API being reachable; offline mode only supports vs-AI-offline. The banner call-out covers this.

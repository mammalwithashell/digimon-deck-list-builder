# Plan: Admin Model Management API + UI (ONNX distribution)

## Context

The Tauri desktop app needs to fetch ONNX agents (MLP/LSTM policies, ~50–200MB each) from the hosted API at runtime. The original plan ("GitHub Actions pushes a manifest.json to Spaces") is being replaced with a DB-backed admin surface: admins upload `.onnx` files via an admin UI, associate each with a deck and engine commit, toggle a `published` flag, and the public desktop client reads a live manifest served by the hosted API. Binary bodies still live in DigitalOcean Spaces — the hosted API only proxies presigned URLs, never the file itself.

This plan covers the **server + admin-UI side**. The companion Rust-side plan (not yet written — see `.claude/plans/` when authored) consumes the manifest shape defined here as its contract.

### Decisions locked in from clarifying questions

- **Router location:** `digimon_gym/db/routers/admin_models.py`, not `digimon_gym/routers/` (rule 11 forbids DB imports from engine-only routers; patch-notes convention from commit `aa83d5f` is the template).
- **sha256 computation:** server streams the Spaces object in 8MB chunks through `hashlib.sha256` during confirm — constant memory, no temp files.
- **Upload transport:** presigned PUT. Browser → Spaces direct. Server never sees file bytes on upload, only on confirm (for hashing + onnxruntime inspection).
- **Manifest contract:** this plan defines it canonically. Rust downloader will cite it.

### Template / sibling work

The patch-notes system (commit `aa83d5f`, files below) is the structural template. Reuse its patterns verbatim where they apply:

| Concern | Template file |
|---|---|
| Migration | `alembic/versions/20260414_0013_patch_notes.py` |
| Model | `digimon_gym/db/models.py:932-960` (`Release`, `KnownIssue`) |
| Pydantic schemas | `digimon_gym/db/schemas.py` (Create/Update/Response pattern) |
| Admin router | `digimon_gym/db/routers/patch_notes.py` |
| Router mount | `digimon_gym/api.py:72` |
| Admin UI page | `frontend/src/pages/AdminPatchNotesPage.tsx` |
| API client | `frontend/src/api/patchNotesApi.ts` |
| Route guard | `frontend/src/App.tsx:55-68` (`!IS_DESKTOP` + `RoleGuard`) |
| Tests | `tests/api/test_patch_notes.py` |

---

## Manifest contract (authoritative)

Served by `GET /models/manifest.json` (public, no auth). Shape the desktop client depends on:

```json
{
  "generated_at": "2026-04-17T20:15:00Z",
  "models": [
    {
      "id": "b3f2…",                            // uuid (ai_models.id)
      "name": "MLP vs Beelzemon (standard)",
      "model_type": "mlp",                      // "mlp" | "lstm"
      "tensor_size": 1375,                      // observation dim (parity with TENSOR_SPEC.md)
      "action_space_size": 2168,                // action dim (parity with ACTION_SPEC.md)
      "engine_commit": "fbf8288",               // git SHA of engine at training time
      "trained_at": "2026-04-10T04:12:00Z",
      "file_sha256": "d2a4e1…",                 // lowercase hex, 64 chars
      "file_size_bytes": 127348920,
      "url": "https://<bucket>.<region>.digitaloceanspaces.com/models/b3f2…/policy.onnx",
      "deck_id": "e9c1…",                       // uuid or null
      "deck_name": "Beelzemon BT14",            // denormalized for display; null if deck_id null
      "notes": "…optional markdown…"
    }
  ]
}
```

- Only rows with `published=true` appear.
- `url` is a **stable public Spaces URL** (objects live under a public-read prefix; no presigned GET needed initially, but the wrapper exposes presigned GET for future auth-gated premium models).
- Ordering: `trained_at DESC` so the newest model per deck is first; client dedupes as it wishes.

---

## Approach

### 1. DB schema + migration

**New model** — append to `digimon_gym/db/models.py` (after `KnownIssue`, ~line 961):

```python
class AIModel(Base):
    __tablename__ = "ai_models"
    __table_args__ = (
        CheckConstraint(
            "model_type IN ('mlp', 'lstm')",
            name="ck_ai_models_model_type",
        ),
        CheckConstraint(
            "state IN ('pending', 'uploaded', 'failed')",
            name="ck_ai_models_state",
        ),
        UniqueConstraint("spaces_key", name="uq_ai_models_spaces_key"),
        Index("idx_ai_models_deck_id", "deck_id"),
        Index("idx_ai_models_published", "published"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    name = Column(String, nullable=False)
    model_type = Column(String, nullable=False)       # 'mlp' | 'lstm'
    tensor_size = Column(Integer, nullable=True)      # set on confirm
    action_space_size = Column(Integer, nullable=True)
    engine_commit = Column(String, nullable=True)     # admin-entered at create
    trained_at = Column(DateTime(timezone=True), nullable=True)
    file_sha256 = Column(String, nullable=True)       # set on confirm
    file_size_bytes = Column(Integer, nullable=True)
    spaces_key = Column(String, nullable=False)       # e.g. "models/<uuid>/policy.onnx"
    deck_id = Column(String, ForeignKey("decks.id", ondelete="SET NULL"), nullable=True)
    uploaded_by = Column(String, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    published = Column(Boolean, nullable=False, default=False)
    state = Column(String, nullable=False, default="pending")  # 'pending'|'uploaded'|'failed'
    notes = Column(Text, nullable=True)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    updated_at = Column(DateTime(timezone=True), default=_utcnow, onupdate=_utcnow, nullable=False)

    deck = relationship("Deck", foreign_keys=[deck_id])
    uploader = relationship("User", foreign_keys=[uploaded_by])
```

**Migration** — new file `alembic/versions/20260417_0014_ai_models.py`, following the `_has_table` idempotent pattern from `20260414_0013_patch_notes.py`. `down_revision = "20260414_0013"` (chains off patch-notes — verify against the latest head at implementation time; there are two `_0013` migrations on the same date, so resolve the chain tip with `alembic heads` before writing).

**Prerequisite check** — Decks already live in the `decks` table (`digimon_gym/db/models.py:90`). FK is straightforward. No deck_library.json promotion needed. `deck_library.json` is separate static content for the deckbuilder UI and unrelated to user-owned decks.

### 2. Spaces wrapper

**New module** — `digimon_gym/storage/__init__.py` + `digimon_gym/storage/spaces.py`.

```python
# digimon_gym/storage/spaces.py
"""Thin boto3 client for DigitalOcean Spaces (S3-compatible).

Keyed off env vars: SPACES_ENDPOINT, SPACES_BUCKET, SPACES_REGION,
SPACES_KEY, SPACES_SECRET. Raises RuntimeError if any are unset at call time
(fail loud, not on import, so desktop_main.py can still import the module
transitively via db.routers — though in practice the router never imports it).
"""
from __future__ import annotations

import hashlib
import os
from functools import lru_cache
from typing import Iterator

import boto3
from botocore.client import Config


def _require_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"{name} is not set")
    return value


@lru_cache(maxsize=1)
def _client():
    return boto3.client(
        "s3",
        endpoint_url=_require_env("SPACES_ENDPOINT"),
        region_name=_require_env("SPACES_REGION"),
        aws_access_key_id=_require_env("SPACES_KEY"),
        aws_secret_access_key=_require_env("SPACES_SECRET"),
        config=Config(signature_version="s3v4"),
    )


def _bucket() -> str:
    return _require_env("SPACES_BUCKET")


def generate_presigned_put(key: str, expires_in: int = 900, content_type: str = "application/octet-stream") -> str:
    return _client().generate_presigned_url(
        "put_object",
        Params={"Bucket": _bucket(), "Key": key, "ContentType": content_type, "ACL": "public-read"},
        ExpiresIn=expires_in,
        HttpMethod="PUT",
    )


def generate_presigned_get(key: str, expires_in: int = 900) -> str:
    return _client().generate_presigned_url(
        "get_object",
        Params={"Bucket": _bucket(), "Key": key},
        ExpiresIn=expires_in,
        HttpMethod="GET",
    )


def head_object(key: str) -> dict:
    """Returns {'ContentLength': int, 'ETag': str, ...} or raises ClientError 404."""
    return _client().head_object(Bucket=_bucket(), Key=key)


def delete_object(key: str) -> None:
    _client().delete_object(Bucket=_bucket(), Key=key)


def iter_object_chunks(key: str, chunk_size: int = 8 * 1024 * 1024) -> Iterator[bytes]:
    """Stream an object body in chunks. Used by confirm endpoint for sha256."""
    body = _client().get_object(Bucket=_bucket(), Key=key)["Body"]
    try:
        while True:
            chunk = body.read(chunk_size)
            if not chunk:
                return
            yield chunk
    finally:
        body.close()


def stream_sha256(key: str) -> tuple[str, int]:
    """Return (sha256_hex, total_bytes) by streaming the object once."""
    h = hashlib.sha256()
    total = 0
    for chunk in iter_object_chunks(key):
        h.update(chunk)
        total += len(chunk)
    return h.hexdigest(), total


def public_url(key: str) -> str:
    """Stable public URL — <bucket>.<region>.digitaloceanspaces.com/<key>.
    The presence of SPACES_ENDPOINT (with scheme+host) gives us the origin.
    """
    endpoint = _require_env("SPACES_ENDPOINT").rstrip("/")
    return f"{endpoint}/{_bucket()}/{key}"
```

**Object key convention:** `models/<ai_models.id>/policy.onnx`. UUID namespacing avoids collisions when the same model is re-uploaded after a failed confirm.

**Unit tests** — `tests/storage/test_spaces.py` using `moto`:
- `test_generate_presigned_put_returns_signed_url` — asserts URL contains `X-Amz-Signature`
- `test_head_object_roundtrip` — PUT a blob, HEAD it, check ContentLength
- `test_stream_sha256_matches_hashlib` — PUT known bytes, stream_sha256 equals `hashlib.sha256(bytes).hexdigest()`
- `test_delete_object_removes` — PUT, delete, HEAD → ClientError 404
- `test_iter_object_chunks_across_boundary` — PUT 17MB of random bytes with seeded RNG, assert chunks reassemble to same bytes
- `test_missing_env_var_raises_at_call_time` — unset `SPACES_ENDPOINT`, call `head_object`, assert RuntimeError (not ImportError)

Add `moto[s3]` and `boto3` to `requirements.txt` (hosted API only, not `requirements-desktop.txt` or `requirements-training.txt`).

### 3. Admin router + public manifest endpoint

**New file** — `digimon_gym/db/routers/admin_models.py`. Both admin-write and the public manifest GET live in this file (single module, different prefixes) because both need DB access.

Pydantic schemas (append to `digimon_gym/db/schemas.py`):

- `AIModelCreateRequest`: `name: str`, `model_type: Literal["mlp","lstm"]`, `engine_commit: str | None`, `trained_at: datetime | None`, `deck_id: str | None`, `notes: str | None`
- `AIModelCreateResponse`: `id: str`, `upload_url: str`, `spaces_key: str`, `expires_in: int`
- `AIModelConfirmResponse`: `id: str`, `state: str`, `file_sha256: str`, `file_size_bytes: int`, `tensor_size: int`, `action_space_size: int`
- `AIModelUpdateRequest`: `name: str | None`, `deck_id: str | None`, `published: bool | None`, `notes: str | None`, `engine_commit: str | None`, `trained_at: datetime | None`
- `AIModelResponse` (admin view): all columns including `state`, `spaces_key`, `published`, `uploader_username`, `deck_name`
- `ManifestModel` + `ManifestResponse`: the public shape defined above
- `ListAIModelsResponse`: `{"models": list[AIModelResponse]}`

Endpoints:

| Method + path | Auth | Behavior |
|---|---|---|
| `POST /admin/models` | admin | Create DB row (state=`pending`), generate presigned PUT for `models/<id>/policy.onnx`, return URL + id + expires_in (900s) |
| `POST /admin/models/{id}/confirm` | admin | HEAD the key → populate `file_size_bytes`; stream-hash → populate `file_sha256`; download-once to a temp file (actually: use `iter_object_chunks` to stream into an `NamedTemporaryFile(suffix=".onnx", delete=False)`; run `onnxruntime.InferenceSession` on the file for tensor/action shape extraction; `os.unlink` in `finally`). Set `state='uploaded'`. Idempotent: re-running on an already-confirmed row re-hashes and updates columns. On any error, set `state='failed'`, delete the Spaces object, return 422 with the exception message. |
| `PATCH /admin/models/{id}` | admin | Update mutable fields: `name`, `deck_id`, `published`, `notes`, `engine_commit`, `trained_at`. Refuse `published=true` if `state != 'uploaded'` → 409. |
| `DELETE /admin/models/{id}` | admin | Delete Spaces object (best-effort — swallow 404), then delete DB row. |
| `GET /admin/models?state=&published=&deck_id=&type=` | admin | List with optional filters. Join `users` for `uploader_username`, `decks` for `deck_name`. |
| `GET /models/manifest.json` | **public** | Return `ManifestResponse` with `published=true AND state='uploaded'`. `url = spaces.public_url(row.spaces_key)`. No auth dependency. Set `Cache-Control: public, max-age=60` so short bursts don't hammer the DB. |

**Key implementation notes:**

- Router file mounts **two** APIRouters to keep prefixes clean:
  ```python
  admin_router = APIRouter(prefix="/admin/models", tags=["admin-models"])
  public_router = APIRouter(prefix="/models", tags=["models-public"])
  ```
  Export `admin_router` and `public_router`; `api.py` mounts both (`app.include_router(admin_models_router.admin_router)` and `admin_models_router.public_router`).

- Confirm endpoint tensor/action extraction:
  ```python
  sess = onnxruntime.InferenceSession(tmp_path, providers=["CPUExecutionProvider"])
  # MLP export convention (see tools/export_onnx.py): input "obs" shape [-1, tensor_size], output "logits" [-1, action_space]
  # LSTM export convention: inputs are "obs" + "h_in" + "c_in"; shape extraction still uses "obs" and "logits"
  obs_input = next(i for i in sess.get_inputs() if i.name == "obs")
  logits_output = next(o for o in sess.get_outputs() if o.name == "logits")
  tensor_size = int(obs_input.shape[-1])
  action_space_size = int(logits_output.shape[-1])
  ```
  Wrap in try/except → 422 "ONNX model doesn't match expected export convention (missing 'obs' input or 'logits' output)".

- Auth: `from digimon_gym.db.auth import ROLE_ADMIN, require_roles`; inject `_: User = Depends(require_roles(ROLE_ADMIN))` on all `/admin/*` endpoints (see `digimon_gym/db/routers/patch_notes.py:74`). Public manifest endpoint has no auth dep.

- Desktop safety: the module lives under `digimon_gym/db/routers/`, which `desktop_main.py` already avoids importing (rule 8). `api.py` mounts it alongside `patch_notes_router` (line 72).

- Design-for-later: the presigned GET wrapper exists but is unused. When/if auth-gated premium models ship, swap `public_url(key)` for `generate_presigned_get(key, expires_in=3600)` inside the manifest builder, and gate `GET /models/manifest.json` behind auth for those rows. Don't build the gating now.

### 4. Admin UI

**New files:**

- `frontend/src/api/modelsApi.ts` — typed client functions mirroring `patchNotesApi.ts`. Includes `uploadWithProgress(url: string, file: File, onProgress: (pct: number) => void)` using `XMLHttpRequest` (not `fetch` — need `xhr.upload.onprogress` for a bandwidth-accurate bar on 200MB uploads).
- `frontend/src/pages/AdminModelsPage.tsx` — list view. Filters: state (all/pending/uploaded/failed), published (all/true/false), model_type, deck. Each row: name, type chip, deck chip, state chip, published toggle, sha256 short, size, uploaded_by, trained_at, Edit/Delete buttons. Pagination via client-side slicing initially (list endpoint is unpaginated for now — OK, admin-only, expected <100 rows).
- `frontend/src/pages/AdminModelsUploadModal.tsx` — three-step modal:
  1. Metadata form (name, type, engine_commit, trained_at, deck picker reusing the deckbuilder's deck-select component, notes). Submit → `POST /admin/models` → receive `{id, upload_url}`.
  2. File picker + progress bar. `XMLHttpRequest.open("PUT", upload_url)` with the file as body. Show percentage. On success → step 3.
  3. Confirm button → `POST /admin/models/{id}/confirm`. Spinner ~5-15s (streaming hash on hosted droplet + onnx inspection). On success show summary card (sha256, size, tensor_size, action_space_size), close modal, refresh list. On failure show error, offer "Delete pending row" (DELETE `/admin/models/{id}`).
- `frontend/src/pages/AdminModelsEditModal.tsx` — edit name/deck/notes/published/engine_commit/trained_at. `PATCH /admin/models/{id}`.

**Modified files:**

- `frontend/src/App.tsx:17-25` — add `const AdminModelsPage = lazy(...)` entry.
- `frontend/src/App.tsx:55-68` — add `<Route path="/admin/models" element={suspended(AdminModelsPage)} />` inside the `!IS_DESKTOP` + `RoleGuard` block. Rule 13 compliance confirmed.
- `frontend/src/components/layout/NavBar.tsx` — add "Models" link to the admin dropdown (mirror how "Patch Notes" was added in commit `aa83d5f` line 11 of NavBar.tsx).

**Deck picker reuse:** look for the existing deck select inside `frontend/src/pages/DeckBuilderPage.tsx` or `frontend/src/components/deck/` — if there's an exported `<DeckSelect>` component, reuse it; otherwise inline a simple `<select>` fed by `GET /decks` (existing endpoint). Don't build a new deck picker.

### 5. Desktop client knock-on (manifest URL resolution)

The Rust-side plan (to be written) must resolve the manifest URL this way:

```rust
fn resolve_model_manifest_url() -> String {
    std::env::var("DIGIMON_MANIFEST_URL")
        .unwrap_or_else(|_| "https://api.digimon-tcg-sim.example.com/models/manifest.json".to_string())
}
```

- Prod default: production hosted API.
- Dev: `DIGIMON_MANIFEST_URL=http://localhost:8000/models/manifest.json`.
- Staging: `DIGIMON_MANIFEST_URL=https://staging-api.../models/manifest.json`.

Binary URLs in the manifest `url` field point directly at Spaces (public-read objects) — desktop downloads them without hitting the API. This plan just makes sure the manifest endpoint ships before the Rust downloader lands.

---

## Critical files

### New

| Path | Purpose |
|---|---|
| `alembic/versions/20260417_0014_ai_models.py` | Migration — creates `ai_models` table + indexes/checks |
| `digimon_gym/storage/__init__.py` | Package marker |
| `digimon_gym/storage/spaces.py` | boto3 wrapper (presigned URLs, head, delete, streaming hash, public_url) |
| `digimon_gym/db/routers/admin_models.py` | Admin CRUD + public manifest endpoint |
| `tests/storage/__init__.py` | Package marker |
| `tests/storage/test_spaces.py` | moto-backed unit tests for the wrapper |
| `tests/api/test_admin_models.py` | Router tests: auth, two-phase flow, confirm happy/failure paths, manifest filtering |
| `frontend/src/api/modelsApi.ts` | Typed API client + XHR progress helper |
| `frontend/src/pages/AdminModelsPage.tsx` | List + filters |
| `frontend/src/pages/AdminModelsUploadModal.tsx` | Three-step upload wizard |
| `frontend/src/pages/AdminModelsEditModal.tsx` | Edit published/deck/notes |

### Modified

| Path | Change |
|---|---|
| `digimon_gym/db/models.py` (after line 960) | Append `class AIModel(Base)` |
| `digimon_gym/db/schemas.py` | Append `AIModel*` and `Manifest*` schemas |
| `digimon_gym/api.py` (near line 73) | `app.include_router(admin_models_router.admin_router)` + `app.include_router(admin_models_router.public_router)` |
| `requirements.txt` | Add `boto3`, `moto[s3]` (move moto to a test section if one exists) |
| `frontend/src/App.tsx` (lines 17-25 and 55-68) | Lazy import + route entry (desktop-gated) |
| `frontend/src/components/layout/NavBar.tsx` | Admin "Models" link |

### Out of scope (flag in PR description)

- Rust downloader (separate plan)
- Code signing of `.onnx` artifacts
- CDN / Cloudflare caching in front of Spaces
- Auth-gated premium models (endpoints designed to accommodate; not implemented)
- Webhook when a new model is published (could notify desktop clients via existing WS; defer)
- `deck_library.json` → DB promotion (not required; user-owned decks already DB-backed)

---

## Verification

Run these in order; each must pass before moving on.

### 1. Migration

```bash
alembic upgrade head
python -c "from digimon_gym.db.models import AIModel; print(AIModel.__table__.columns.keys())"
alembic downgrade 20260414_0013 && alembic upgrade head   # round-trip
```

Expected: column list includes `id, name, model_type, tensor_size, action_space_size, engine_commit, trained_at, file_sha256, file_size_bytes, spaces_key, deck_id, uploaded_by, published, state, notes, created_at, updated_at`. Downgrade/upgrade round-trip leaves no orphan constraints (sqlite inspector returns clean).

### 2. Spaces wrapper

```bash
python -m pytest tests/storage/test_spaces.py -v
```

Expected: all 6 tests pass. Moto doesn't need real credentials — tests set dummy env vars via `monkeypatch` (same pattern as the patch-notes test fixture).

### 3. Admin router

```bash
python -m pytest tests/api/test_admin_models.py -v
```

Required test cases (mirror `tests/api/test_patch_notes.py` style):

- `test_create_returns_presigned_put_and_pending_row`
- `test_confirm_streams_hash_and_extracts_tensor_shape` — upload a small pre-built ONNX fixture to moto, confirm, verify `tensor_size` + `action_space_size` + `file_sha256` populated
- `test_confirm_fails_if_onnx_missing_obs_input` — fixture ONNX with wrong input names → 422, row state=`failed`, object deleted
- `test_confirm_is_idempotent` — call twice, second call succeeds, same hash
- `test_patch_published_requires_state_uploaded` — PATCH `published=true` on `pending` row → 409
- `test_delete_removes_spaces_object_and_row`
- `test_non_admin_403_on_create_patch_delete`
- `test_list_filters_by_state_and_published_and_deck`
- `test_manifest_excludes_unpublished_and_pending` — create 3 rows (published+uploaded, published+pending, unpublished+uploaded); manifest returns only the first
- `test_manifest_is_public_no_auth_header_needed`
- `test_manifest_shape_matches_contract` — schema-validate against the contract in this plan (hand-rolled `assert` on keys)

Fixture — pre-built ONNX files in `tests/api/fixtures/`:
- `valid_mlp.onnx` — 2-layer MLP, input `obs` shape `[-1, 1375]`, output `logits` shape `[-1, 2168]`. Build once via a `conftest.py` `torch.onnx.export` helper guarded by `pytest.importorskip("torch")` and cached.
- `valid_lstm.onnx` — LSTM with `obs`/`h_in`/`c_in` inputs, `logits`/`h_out`/`c_out` outputs.
- `invalid_no_obs.onnx` — input named `x` instead of `obs`.

### 4. Frontend

```bash
cd frontend && npm run build                 # prod build — admin UI included
cd frontend && VITE_BUILD_TARGET=desktop npm run build   # desktop build — admin UI tree-shaken
```

Expected: `npm run build` output includes a `AdminModelsPage-*.js` chunk; desktop build does not (grep bundle output). Also run `npm run lint` and the existing Vitest suite (`npm test` if configured).

### 5. Manual smoke (local dev, required before marking done)

```bash
# Terminal 1 — hosted API with real boto3 pointing at a DO Spaces dev bucket
SPACES_ENDPOINT=https://nyc3.digitaloceanspaces.com \
SPACES_BUCKET=digimon-tcg-dev \
SPACES_REGION=nyc3 \
SPACES_KEY=... SPACES_SECRET=... \
python -m uvicorn digimon_gym.api:app --host 0.0.0.0 --port 8000
# NOTE: no --reload — see MEMORY.md zombie-process warning.

# Terminal 2
cd frontend && npm run dev
```

Then in the browser as admin user (`mammal` / `testlogin` per memory):

1. Navigate to `/admin/models`, click Upload.
2. Fill form, pick a real exported ONNX (`tools/export_onnx.py --type mlp --input models/mlp_agent.zip --output /tmp/test.onnx`), submit.
3. Watch progress bar reach 100%.
4. Click Confirm, watch spinner, verify extracted `tensor_size=1375`, `action_space_size=2168`.
5. Toggle Published.
6. `curl http://localhost:8000/models/manifest.json` as unauthenticated user → the row appears with correct `url`.
7. `curl -I <url from manifest>` → 200 OK from Spaces.
8. Delete the model, re-curl manifest → row gone; HEAD on Spaces URL → 404.

### 6. Desktop tree-shake audit

```bash
VITE_BUILD_TARGET=desktop npm --prefix frontend run build
grep -l "AdminModels" frontend/dist/assets/*.js || echo OK
```

Expected: "OK" (no admin page strings in the desktop bundle). Rule 13 compliance.

---

## Risks

### Shape drift (highest)

The manifest's `tensor_size` and `action_space_size` must match the engine actually running on the client. If the engine rewrites its observation tensor (new card fields, etc.) and a model with the old shape is marked `published=true`, the desktop client will crash at `InferenceSession.Run` time. Mitigations, ranked:

1. **`engine_commit` column + client-side check.** Desktop client knows its own engine commit (compile-time); if manifest `engine_commit` doesn't match, skip the model with a warning. Implemented server-side by storing `engine_commit` at create time (admin types it; we don't infer it). Client-side filter is the Rust plan's job — but server must serve the field.
2. **Publish guard.** The `PATCH /admin/models/{id}` handler could cross-reference `tensor_size` and `action_space_size` against running-engine constants (`digimon_gym.engine.game.tensor.TENSOR_SIZE`, `digimon_gym.engine.game.action.ACTION_SPACE_SIZE`) and refuse `published=true` on mismatch with a warning. Implement this — admins override via notes + retrain rather than publishing broken models.
3. **Documentation.** Add a one-line note in `docs/ARCHITECTURE.md` pointing at the manifest contract and the shape-drift hazard so the next person editing `TENSOR_SPEC.md` remembers to invalidate old models.

### Secondary risks

- **Presigned URL expiry vs slow uploads.** 900s (15min) TTL. A 200MB upload on a 20Mbit home connection takes ~80s — fine. On a 2Mbit connection it takes ~800s — tight. If this bites, bump `expires_in` to 3600s and revisit.
- **Public-read ACL on uploaded objects.** We set `ACL=public-read` in the presigned PUT params; the signer enforces the client honors it. Sanity-check in the confirm handler: after HEAD, if the object's ACL isn't public-read, re-apply it (`put_object_acl`). Cheap insurance.
- **Spaces outage during confirm.** Confirm does HEAD + streaming-get + onnx-inspect. Any of these can fail mid-flight. Row gets `state='failed'`, admin retries. Don't rollback the DB row on failure — keep the failed row visible so admins can debug.
- **moto version skew.** Pin `moto[s3]>=4.2` to ensure `generate_presigned_url` + `head_object` behave like real AWS; older moto didn't always sign v4.
- **Large test fixtures.** The ONNX fixture files are small (~500KB each for a toy net) — fine to commit. If they balloon, build them on-the-fly in `conftest.py` via `torch.onnx.export` with a module-scoped fixture.
- **Race on deck deletion.** `deck_id` FK is `ON DELETE SET NULL` — if an admin deletes a deck associated with a published model, the model stays published with `deck_id=null`. Manifest shows `deck_name: null`. Acceptable.

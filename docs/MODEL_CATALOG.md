# ONNX Model Catalog

End-to-end reference for the ONNX model distribution system: how operators
upload new policies to the hosted API, how desktop clients discover +
download them, and how developers reason about the cache.

Related: [DEPLOYMENT.md](DEPLOYMENT.md) covers DigitalOcean Spaces
provisioning; [TOOLS.md §7.1](TOOLS.md) documents the `export_onnx.py`
CLI this workflow depends on.

## Overview

```
┌────────────────┐   POST .onnx + meta   ┌────────────────┐
│  operator      │ ────────────────────▶ │  hosted API    │
│  (CI or laptop)│                        │  /admin/models │
└────────────────┘                        └───────┬────────┘
                                                  │ put(key, path)
                                                  ▼
                                         ┌────────────────┐
                                         │ ModelStorage   │
                                         │ local | Spaces │
                                         └───────┬────────┘
                                                  │ public_url or FileResponse
                                                  ▼
┌────────────────┐   GET /models/manifest  ┌─────────────┐   HTTP GET+Range
│  desktop UI    │ ◀───── sidecar ───────▶ │ /models      │ ─────────────────▶ CDN/API
│  (Settings →   │                         │              │                    serves .onnx
│   Models)      │  POST /models/download  │              │ ◀──── sha256 verify ──
└────────────────┘                         └─────────────┘
                                                  │
                                                  ▼
                                         app_data_dir/models/{slug}/{version}/model.onnx
```

Two storage backends, selected by `MODEL_STORAGE_BACKEND`:

- `local` — `LocalModelStorage` writes to a filesystem root. Downloads
  flow through `GET /models/{slug}/{version}/blob`, which uses FastAPI's
  `FileResponse` (Range-aware for resumable downloads). Good for dev +
  single-host alpha.
- `spaces` — `SpacesModelStorage` (`aioboto3`) writes to a DigitalOcean
  Spaces bucket. `public_url(key)` returns the CDN URL clients GET
  directly — read traffic bypasses the Droplet.

Both implement the same `ModelStorage` Protocol
(`digimon_gym/models_store/storage.py`). Swap-in replacement; no client
code changes between backends.

## Data model

Two tables, migration `20260417_0016_add_models_catalog`:

### `model_records`
One row per *logical* model (e.g. `greedy-meta-gauntlet`). Versions
stack up under it.

| Column | Notes |
|---|---|
| `slug` | URL-safe unique identifier (`[a-z0-9-]{2,64}`). Clients pass this to `createGame` as the model reference. |
| `name` / `description` | Free-form for display. |
| `arch` | `mlp` or `lstm`. Validated at upload. |
| `min_engine_version` | Record-level floor. Per-version override lives on `model_versions`. |
| `deck_pool` | Optional tag, e.g. `"meta-2026-Q2"`. |
| `is_public` / `is_deprecated` | Visibility flags. |

### `model_versions`
One row per uploaded `.onnx`. Blob bytes live in `ModelStorage`; DB
stores only metadata.

| Column | Notes |
|---|---|
| `version` | Semver string (`\d+\.\d+\.\d+(?:[-+]...)?`). Unique within a record. |
| `storage_key` | Opaque locator into `ModelStorage`. By convention `{slug}/{version}/model.onnx`. |
| `sha256` | Authoritative integrity value. Clients verify after download; upload handler refuses to register if computed hash doesn't match. |
| `size_bytes` | Bytes on disk. Used by the UI for progress + footprint display. |
| `onnx_input_shapes` | `{"inputs": {name: shape}, "outputs": [names]}`. Captured via `onnx.load` when the package is installed; null otherwise. |
| `min_engine_version` | Optional per-version floor — tighter than the record's baseline. |
| `is_deprecated` | Soft-delete flag. Blobs stay so pinned clients keep working. |

## Public read API

All read endpoints are public + rate-limited (120/min). Manifest JSON
is cheap; rate limit is there to stop a misconfigured client from
cratering the CDN origin.

### `GET /models`
Returns a list of `ModelRecordPublic`. Private and deprecated records
are hidden by default; pass `?include_deprecated=true` to see deprecated.

### `GET /models/{slug}`
Single record detail. 404 if the slug is unknown or `is_public=0`.

### `GET /models/{slug}/{version}/blob`
Streams the ONNX bytes. **Only reachable when
`MODEL_STORAGE_BACKEND=local`** — Spaces deployments put a CDN URL in
the manifest so this route is never hit on the hot path.
`FileResponse` handles Range requests natively; interrupted downloads
can resume.

Each `ModelVersionPublic` in the response includes a `url` field the
client can GET directly — CDN edge in production, the `/blob` route
locally. The desktop cache code treats these uniformly.

## Admin upload flow

Gated by `require_roles(ROLE_ADMIN)`. Two calls per model:

### 1. Create the record
```bash
curl -X POST https://api.example.com/admin/models \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "slug": "greedy-meta",
    "name": "Greedy Meta Gauntlet",
    "arch": "mlp",
    "description": "Baseline opponent, late-Q2 meta",
    "min_engine_version": "0.1.0",
    "is_public": true
  }'
```

### 2. Upload a version
The `meta` form field carries release metadata as stringified JSON. The
`.onnx` file is the other half of the multipart body.

```bash
curl -X POST https://api.example.com/admin/models/greedy-meta/versions \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -F 'meta={"version":"1.0.0","changelog":"First release","min_engine_version":"0.1.0"}' \
  -F 'file=@models/greedy_meta_v1.onnx'
```

The server:

1. Streams the upload to a temp file while hashing (never holds the
   full blob in memory — models are 100-300 MB).
2. Runs `onnx.load` to extract input/output shapes and cross-check the
   declared arch. Skipped with a log warning if the `onnx` package
   isn't installed; hash + size remain authoritative.
3. Hands the temp file to `ModelStorage.put` (filesystem copy or
   multipart S3 upload).
4. Inserts a `model_versions` row.
5. Returns the manifest entry.

### 3. Deprecate a version (soft-delete)
```bash
curl -X DELETE https://api.example.com/admin/models/greedy-meta/versions/1.0.0 \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

The blob stays in storage; the row gets `is_deprecated=1`. Clients that
had already pinned this version keep working; new `latest` resolves to
the newest non-deprecated version.

## CI upload from `export_onnx.py`

The CLI can emit a `{output}.meta.json` sidecar matching the upload
handler's `meta` field, collapsing export + upload to a scripted pair:

```bash
python tools/export_onnx.py \
  --type mlp \
  --input models/greedy_meta.zip \
  --output models/greedy_meta.onnx \
  --emit-metadata \
  --engine-version 0.2.0 \
  --min-engine-version 0.1.0 \
  --changelog "Retrained on Q2 meta"

# Upload both
curl -X POST https://api.example.com/admin/models/greedy-meta/versions \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -F "meta=$(cat models/greedy_meta.onnx.meta.json)" \
  -F "file=@models/greedy_meta.onnx"
```

The sidecar also contains `sha256` and `size_bytes` fields so you can
sanity-check the file before uploading — the server recomputes both
and rejects mismatches, but catching a corrupted export client-side
saves a round trip.

## Desktop cache

The sidecar exposes three catalog routes that the frontend drives. All
talk to the hosted API through `catalog_api_base`; fall back to
bundled-only when unset.

| Route | Purpose |
|---|---|
| `GET /models/manifest` | Merged view: bundled baseline + remote catalog + cache status. Pass `?refresh=true` to bypass the 5-minute manifest cache. |
| `POST /models/download` | `{slug, version}`. Streams the blob into the cache with sha256 verification. Resumable via HTTP `Range`. |
| `POST /models/delete` | `{slug, version}`. Removes the cached files + empty parent dir. |

Resolution order inside the sidecar (`resolve_model_path` in
`digimon_gym/engine/model_utils.py`):

1. **Slug lookup** — if `reference` names a directory under the cache
   dir, load the newest version (or `prefer_version` if set).
2. **Bundled filename** — if `reference` matches a file in the
   installer-baked `ONNX_MODELS_DIR`, use it. `.onnx` extension
   optional.
3. **FileNotFoundError** — raised with the list of bundled filenames
   for error messages.

### Cache layout
```
{app_data_dir}/models/
├── greedy-meta/
│   ├── 1.0.0/
│   │   └── model.onnx
│   └── 1.1.0/
│       ├── model.onnx
│       └── model.onnx.part     # if a prior download was interrupted
└── lstm-gauntlet/
    └── 2.0.0/
        └── model.onnx
```

`app_data_dir` resolves per OS (Tauri's `app_data_dir()`):
- Windows: `%APPDATA%\com.digimon-tcg.desktop\models\`
- macOS: `~/Library/Application Support/com.digimon-tcg.desktop/models/`
- Linux: `~/.config/com.digimon-tcg.desktop/models/`

### Integrity guarantees
- Download path always runs through `hashlib.sha256` as bytes arrive;
  final hash must match the manifest entry. Mismatch raises
  `IntegrityError` and deletes the quarantined `.part` file.
- If `model.onnx` already exists at the target size but the hash is
  wrong (disk corruption, partial prior run, tampering), it's deleted
  and re-downloaded.
- `Range` resumption verifies the server actually returned `206 Partial
  Content`; a stale cache that returns `200` triggers a full restart.

## Environment variables

| Var | Where | Required | Notes |
|---|---|---|---|
| `MODEL_STORAGE_BACKEND` | hosted API | yes | `local` or `spaces`. |
| `MODEL_STORAGE_LOCAL_ROOT` | hosted API | local only | Filesystem root. Default `data/models_store`. |
| `SPACES_ENDPOINT_URL` | hosted API | spaces | e.g. `https://nyc3.digitaloceanspaces.com`. |
| `SPACES_REGION` | hosted API | spaces | e.g. `nyc3`. |
| `SPACES_BUCKET` | hosted API | spaces | Bucket name. |
| `SPACES_ACCESS_KEY` / `SPACES_SECRET_KEY` | hosted API | spaces | r/w credentials. |
| `SPACES_CDN_BASE_URL` | hosted API | optional | Override the default `{bucket}.{region}.cdn.digitaloceanspaces.com` hostname. |
| `ONNX_MODELS_DIR` | sidecar | yes | Installer-bundled baseline dir. Set by `create_desktop_app` via `--models-dir`. |
| `ONNX_MODELS_CACHE_DIR` | sidecar | optional | Per-user writable cache. Set via `--models-cache-dir`; Tauri passes `app_data_dir()/models`. |
| `DIGIMON_API_BASE` | sidecar | optional | Hosted API base URL for `/models` manifest fetches. Unset → bundled-only mode. |

## Debugging

- **"ONNX model not found"** after an upload — check the `storage_key`
  matches on disk: `ls $MODEL_STORAGE_LOCAL_ROOT/{slug}/{version}/`.
  For Spaces, `aws s3 ls s3://$SPACES_BUCKET/{slug}/{version}/`.
- **Client downloads but "sha256 mismatch"** — the blob in storage
  doesn't match the DB row. Usually means a fresh upload clobbered an
  existing storage key. Re-upload a new `version` rather than
  overwriting.
- **Desktop shows "not downloaded" but the file is there** — cache
  entry is keyed by `{slug}/{version}/model.onnx`. If the sidecar was
  launched without `--models-cache-dir`, or the Tauri path resolver
  changed, cached models become invisible. Check
  `ONNX_MODELS_CACHE_DIR` in the sidecar env.
- **Admin upload returns 400 "Declared arch=…"** — the ONNX graph
  doesn't have the output names the declared arch expects (`logits,
  h_out, c_out` for LSTM; `logits` only for MLP). Re-run
  `export_onnx.py` with the correct `--type`.

## Tests

- `tests/api/test_models_catalog.py` — end-to-end upload/download
  round trip, duplicate-slug/version rejection, deprecation semantics.
- `tests/engine/test_model_catalog_client.py` — pure-logic coverage of
  `merge_manifest`, `resolve_model`, sha256 integrity, and cached
  version deletion. Mocks `httpx.stream` so no network needed.

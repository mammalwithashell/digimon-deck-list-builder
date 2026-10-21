# Design — add-model-distribution-tiers

## Context

Today (see `docs/MODEL_CATALOG.md`):

- Operators upload `.onnx` files through `/admin/models`; the server issues a presigned PUT to Spaces with `ACL: public-read` (`server/storage/spaces.py::generate_presigned_put`) at key `models/<id>/policy.onnx`.
- `GET /models/manifest.json` (unauthenticated) lists every `published` + `uploaded` row whose contract validates, each with `url = spaces.public_url(key)`, `file_sha256` and `file_size_bytes`.
- The desktop app (`src-tauri/src/models.rs`) fetches the manifest, downloads, verifies the SHA-256, caches under `data_dir/digimon-tcg/models/`, and runs the model through the Rust engine's `OnnxPolicy` (MLP or LSTM, masked argmax).
- The hosted API can already run a model server-side for its own games: `POST /models/{id}/prepare` resolves the blob to a local cache via `storage/model_resolver.py`.
- Admin UI: `AdminModelsPage.tsx` with upload and edit modals.

The goal: the owner's newest models are playable by everyone but never distributed as files, while other models remain downloadable for offline play.

## Goals / Non-Goals

**Goals**
- A per-model tier, `download` or `stream`, that the owner controls, with stream as the safe default.
- Stream model files are never readable from outside the server: not via the CDN, the manifest, or any API response.
- Streamed play makes exactly the moves local play of the same model would make.
- One client rule for choosing local versus streamed inference, used by desktop now and the web client later.
- A dashboard the owner can run the catalog from: tiers, visibility, usage, limits, history.

**Non-Goals**
- The WebAssembly browser engine and onnxruntime-web (separate change).
- Recalling files already downloaded before a model was moved to `stream`.
- Preventing a determined user from training an imitation of a stream model by playing many games against it. Auth and rate limits raise the cost; nothing short of not offering the bot prevents it.
- Model compression or quantization.

## Decisions

### D1. Two buckets, not per-object ACLs

Stream models live in a separate private bucket (`SPACES_PRIVATE_BUCKET`) that has no CDN and no public objects. Download models are copied into the existing public bucket.

*Why not a single bucket with per-object `private` / `public-read` ACLs?* It works, but one wrong ACL on one object silently publishes a protected model, and the CDN can keep serving a cached copy after an ACL flip. A bucket that never holds public objects makes "stream files are private" a property of where they are stored, not of every write getting the ACL right. Copies are server-side (`copy_object`), so promotion does not route the file through the API host.

- Uploads always go to the private bucket (presigned PUT without `public-read`).
- `ai_models.spaces_key` keeps pointing at the private (canonical) object. A new nullable `public_spaces_key` is set only while the model is `download`.
- The server resolver (`model_resolver.py`) reads from the private bucket for every model, so server-side play works regardless of tier.

### D2. Promotion is one-way in effect; demotion is best-effort

- **Promote (`stream` → `download`)**: copy private → public, set `public_spaces_key`, set `distribution = download`, write an audit row. From this point the file should be considered public forever.
- **Demote (`download` → `stream`)**: set `distribution = stream`, delete the public object, purge the CDN path, clear `public_spaces_key`, write an audit row. Copies already downloaded remain out there; the dashboard says so before confirming.

### D3. Manifest shape per tier

The manifest stays public and lists every published, contract-valid model. Each entry gains `distribution`. `url`, `file_sha256` and `file_size_bytes` are present only for `download` entries. Stream entries keep the contract fields (`observation_profile`, `tensor_size`, `action_space_size`, `tensor_layout_hash`, `feature_schema_version`, `action_schema_version`) so clients can tell whether they can drive the model at all.

The SHA-256 is withheld for stream models: it is not secret on its own, but it would confirm a leaked file's identity.

Older desktop builds that expect `url` on every entry must skip stream entries instead of failing. Today's `models.rs` deserializes `url` as required, so the shipped desktop release needs a compatible client (task 5.1) **before** the server starts emitting stream entries. Until then the server can hide stream entries from clients that do not send `X-Client-Capabilities: model-tiers` (D8).

### D4. Inference endpoint: stateless, binary-friendly

```
POST /bots/{model_id}/act
Authorization: Bearer <token>
Content-Type: application/json   (or application/x-digimon-act for the binary form)

{ "contract": { "tensor_layout_hash", "action_schema_version", "tensor_size", "action_space_size" },
  "obs":  <tensor_size float32>,
  "mask": <action_space_size 0/1>,
  "h": <hidden float32> | null, "c": <hidden float32> | null }     // LSTM only; null = start of game

→ 200 { "action": <int>, "h": [...], "c": [...] }
→ 409 contract mismatch · 404 unknown/unpublished · 429 rate-limited (Retry-After) · 503 model loading
```

- **Stateless.** The LSTM state goes back and forth with each call. The server keeps no per-game session, so restarts, deploys and multiple API instances are safe. A client can tamper with its own state; that only weakens the bot it is playing against.
- **Payload size.** `standard_lite_v2` observations are 8,410 floats: about 34 KB raw, more as JSON text. JSON is the reference format for simplicity and debugging. A binary form (little-endian float32 blocks with a small header) is accepted for production clients and cuts the request to about 34 KB + mask bits. Both decode to the same arrays.
- **Model residency.** The server keeps an LRU of loaded sessions keyed by `file_sha256`, filled through the existing resolver. A cold load returns `503` with `Retry-After` rather than blocking the worker.

### D5. Action selection reuses the Rust policy (parity)

The endpoint does not re-implement inference in Python. `digimon-engine-py` gains a stateless entry point over the existing Rust `OnnxPolicy`:

```
load_policy(path) -> PolicyHandle
PolicyHandle.predict(obs, mask, h=None, c=None) -> (action, h_out, c_out)
```

It uses the same `masked_argmax` and the same output-name contract (`logits`, `h_out`, `c_out`) as desktop and server-side games. A parity test runs one recorded observation sequence through the desktop path and the endpoint and requires identical actions.

### D6. Auth and rate limits

- Auth: any authenticated user, including guest tokens (`is_guest`). Stream bots are part of the game, not a paid feature. The owner can restrict a model to registered users with a per-model `requires_account` flag if guests abuse it.
- Limits: per user per model, enforced with the existing `slowapi` limiter keyed by `user_id:model_id`. Default `ai_models.rate_limit_per_minute = 600`, which is ample for interactive play: a bot turn is tens of decisions and the UI paces bot actions. Overridable per model from the dashboard. A global per-user cap across all models stops one account from fanning out.
- 429 responses carry `Retry-After`. Clients pause the bot and show a notice rather than failing the game.

### D7. Usage statistics without a hot write per call

Each call increments in-process counters keyed by `(model_id, day)`: calls, rate-limited calls, distinct users (a capped set per day), latency sum and max. A background task flushes them into `model_usage_daily` every 60 seconds with an upsert. Losing up to a minute of counts on a crash is acceptable for a dashboard. The dashboard reads daily rows; no per-call table exists.

### D8. Client selection rule (desktop now, web later)

```
if model.distribution == "download" and cached(model.sha) and contract_ok: run locally
elif model.distribution == "download" and can_download:                 download, then run locally
elif model.distribution == "stream" and online and contract_ok:         stream via /bots/{id}/act
else:                                                                    unavailable (show reason)
```

Desktop implements this in Rust: a `RemotePolicy` implementing the same policy trait as `OnnxPolicy`, called from the agent loop, using the HTTP client `models.rs` already has. The web client will follow the same rule once it has a browser engine; until then web games keep running server-side (where the server can use any tier directly).

Clients that understand tiers send `X-Client-Capabilities: model-tiers` on the manifest request. The server omits stream entries for clients that don't, protecting desktop builds released before this change.

### D9. Audit log

`model_distribution_audits(id, model_id, actor_user_id, action, from_value, to_value, note, created_at)`. `action` is one of `promote`, `demote`, `publish`, `unpublish`, `deprecate`, `undeprecate`, `rate_limit_change`, `upload`. Written in the same transaction as the change it records. The dashboard shows it per model and globally.

## Migration plan

1. Create the private bucket; deploy with `SPACES_PRIVATE_BUCKET` set.
2. Alembic migration: add columns (`distribution` default `'stream'`, `public_spaces_key` null, `rate_limit_per_minute` default 600) and the two new tables.
3. One-off script (`code/tools/migrate_models_to_private.py`), idempotent: for each row, copy the public object into the private bucket at the same key and verify the SHA-256. It leaves the public copy and sets `public_spaces_key` to it, but marks the row `stream`. Then, in a second explicit step (`--delete-public`), it removes public copies of rows still `stream` and purges the CDN. The two steps let the owner promote the models they want to keep downloadable *before* public copies disappear.
4. Ship the tier-aware desktop build (task 5) before stream entries are visible to old clients. D3's capability header covers the gap.

Rollback: columns are additive. Reverting the server restores the old manifest (all rows with `url`), but rows whose public copy was deleted would 404. Rollback after step 3's `--delete-public` therefore requires re-promoting.

## Risks / Trade-offs

- **Server load moves to us for stream models.** One decision is roughly a millisecond of CPU for these model sizes, so a 2 vCPU droplet serves many concurrent games. Usage stats (D7) show when to scale.
- **Latency per bot decision** (one round trip, about 50–150 ms) is masked by the UI's deliberate bot pacing, but noticeable at "Instant" bot speed.
- **Stream bots need a connection.** Desktop offline play is limited to download models.
- **Imitation by querying** (non-goal) remains possible; per-account limits and auth slow it down.
- **Already-downloaded files** of models later moved to `stream` cannot be recalled.

## Open Questions

1. Should stream bots be available to guest accounts by default, or registered accounts only? (Design assumes guests allowed, with a per-model `requires_account` switch.)
2. Which of today's published models should be promoted to `download` before `--delete-public` runs? The owner decides in the dashboard after step 3 of the migration.
3. Should the desktop app fall back automatically to the best compatible `download` model when a stream model is unreachable, or ask the player? (Design assumes ask.)

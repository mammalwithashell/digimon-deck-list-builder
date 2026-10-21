# Tasks — add-model-distribution-tiers

Ordered so stream models are protected as early as possible. Phases 1–2 alone stop new uploads from being public; phase 5.1 must ship before stream entries are shown to old desktop clients (design D3/D8).

## 1. Data model and storage

- [ ] 1.1 Alembic migration: `ai_models.distribution` (`'download'|'stream'`, not null, default `'stream'`, check constraint), `ai_models.public_spaces_key` (nullable), `ai_models.rate_limit_per_minute` (int, default 600), `ai_models.requires_account` (bool, default false); new tables `model_distribution_audits` and `model_usage_daily` (`model_id`, `day`, `calls`, `rate_limited`, `distinct_users`, `latency_ms_sum`, `latency_ms_max`, unique on `(model_id, day)`)
- [ ] 1.2 SQLAlchemy models + Pydantic schemas for the new columns/tables; admin responses include `distribution`, `requires_account`, `rate_limit_per_minute`
- [ ] 1.3 `storage/spaces.py`: bucket-aware helpers (`private` vs `public`), `copy_to_public(key)`, `delete_public(key)`, `purge_cdn(path)` via the DigitalOcean CDN API; new env vars `SPACES_PRIVATE_BUCKET`, `SPACES_CDN_ENDPOINT_ID`, `DO_API_TOKEN` (purge only)
- [ ] 1.4 `generate_presigned_put`: target the private bucket and drop `ACL: public-read`; upload confirm/hash path reads from the private bucket
- [ ] 1.5 `storage/model_resolver.py`: always resolve from the private (canonical) key, independent of tier
- [ ] 1.6 Tests: upload URL targets private bucket without public ACL; resolver loads a stream model; storage helpers against a stubbed S3 client

## 2. Manifest and tier actions (server)

- [ ] 2.1 Manifest: add `distribution`; emit `url`/`file_sha256`/`file_size_bytes` only for `download`; omit stream entries unless the request declares `X-Client-Capabilities: model-tiers`
- [ ] 2.2 Admin API: `POST /admin/models/{id}/promote` (body must echo the model name) and `POST /admin/models/{id}/demote`; promotion copies then flips tier in one transaction with the audit row; failed copy leaves the row unchanged
- [ ] 2.3 Admin API: publish/unpublish, deprecate/undeprecate, `requires_account`, `rate_limit_per_minute` updates via `PATCH`, each writing an audit row in the same transaction
- [ ] 2.4 Admin API: `GET /admin/models/{id}/usage?days=30`, `GET /admin/models/audit?model_id=` (read-only; no update/delete routes)
- [ ] 2.5 Guard test: no public or user endpoint response contains a URL or `file_sha256` for a stream model (walk the routes' response models)
- [ ] 2.6 Tests: manifest shape per tier and per capability header; promote/demote happy paths and failure path; audit rows written for every action

## 3. Bindings: stateless policy entry point

- [ ] 3.1 `digimon-engine-py`: expose `load_policy(path)` returning a handle with `predict(obs, mask, h=None, c=None) -> (action, h_out, c_out)` over the existing Rust `OnnxPolicy` (MLP ignores state; LSTM uses given state, zeros when None)
- [ ] 3.2 Rust: add the stateless predict path to the LSTM/MLP policies without changing the stateful API used by desktop and training
- [ ] 3.3 Tests: binding returns legal actions under the mask; LSTM state round trip equals the stateful path step for step

## 4. Move endpoint (server)

- [ ] 4.1 `routers/bots.py`: `POST /bots/{model_id}/act` — auth (guest or registered; `requires_account` → 403 for guests), contract check (409 with mismatched fields), length checks (422), 404 for unknown/unpublished
- [ ] 4.2 Request decoding: JSON reference format plus the binary `application/x-digimon-act` form (float32 little-endian blocks + header); both decode to identical arrays
- [ ] 4.3 Session cache: LRU of loaded policies keyed by `file_sha256`, filled via the resolver; cold load returns 503 + `Retry-After` instead of blocking
- [ ] 4.4 Rate limiting: per `user_id:model_id` at the model's `rate_limit_per_minute`, plus a global per-user cap; 429 with `Retry-After`
- [ ] 4.5 Usage counters: in-process per `(model_id, day)`, flushed to `model_usage_daily` every 60 s with upsert; flush on shutdown
- [ ] 4.6 Parity test: a recorded observation/mask sequence through the endpoint vs local Rust inference — identical actions, states within tolerance (MLP and LSTM fixtures)
- [ ] 4.7 Tests: auth (401/403), rate limit isolation between users and models, restart-mid-game statelessness, 404 for unpublished

## 5. Desktop client

- [ ] 5.1 `src-tauri/src/models.rs`: send `X-Client-Capabilities: model-tiers`; deserialize `distribution` and optional `url`/`file_sha256`; never attempt to download a stream entry
- [ ] 5.2 `RemotePolicy` implementing the agent loop's policy trait: calls `/bots/{id}/act` with the stored token, threads LSTM state, resets at game boundaries (rule 10), binary request form
- [ ] 5.3 Policy selection per design D8 in `engine_commands.rs` / `inference_state.rs`; clear errors for offline, 401/403, 409, 429 (pause + notice + retry after `Retry-After`)
- [ ] 5.4 Tests: selection matrix (download cached / not cached / stream online / stream offline / incompatible); `RemotePolicy` against a mock server including 429 handling

## 6. Frontend: model picker

- [ ] 6.1 Model picker shows tier with an "online only" badge for stream models; disabled state with a reason when unavailable (offline, account required, incompatible)
- [ ] 6.2 In-game notice for a paused bot (rate limited) and for a lost connection to a stream bot, with "keep waiting" / "concede" options
- [ ] 6.3 Unit tests for the picker states and the in-game notices

## 7. Admin dashboard (frontend)

- [ ] 7.1 `AdminModelsPage`: tier column with distinct styling, filters (tier, published, compatible), 7-day call sparkline per row
- [ ] 7.2 Promotion dialog: one-way warning, type-the-model-name confirmation, disabled confirm until it matches; demotion dialog with the "existing copies cannot be recalled" warning
- [ ] 7.3 Model detail view: publish/unpublish, deprecate, `requires_account`, rate-limit editor; 30-day usage charts (calls, distinct users, rate-limited, latency)
- [ ] 7.4 Audit log views: per-model history in the detail view and a global log page, read-only
- [ ] 7.5 Upload modal: states that new uploads are stream-only until promoted
- [ ] 7.6 Tests: promotion confirm gating, filters, audit rendering; admin-only route guard

## 8. Migration and rollout

- [ ] 8.1 `code/tools/migrate_models_to_private.py`: default step copies each object to the private bucket with checksum verification and marks rows `stream` (idempotent); `--delete-public` step removes public copies of rows still `stream` and purges the CDN; `--dry-run` prints the plan
- [ ] 8.2 Rollout runbook in `docs/runbooks/api-deploy.md`: create private bucket → deploy → run default migration → ship tier-aware desktop (5.1) → owner promotes keepers in the dashboard → run `--delete-public`
- [ ] 8.3 Update `docs/MODEL_CATALOG.md` (tiers, endpoint, dashboard) and `docs/DEPLOYMENT.md` (new bucket and env vars)
- [ ] 8.4 Verify end to end on a staging stack: upload → stream play from desktop → promote → download play offline → demote → URL no longer serves the file

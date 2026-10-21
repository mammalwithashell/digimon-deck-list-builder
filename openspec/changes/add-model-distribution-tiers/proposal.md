# add-model-distribution-tiers

## Why

Every published model is downloadable by anyone today. `GET /models/manifest.json` needs no login and lists each model with a public Spaces CDN URL, and every upload is written with `ACL: public-read`. That suits models we are happy to give away (offline desktop play, small or older bots), but the owner wants the **newest and strongest models to stay theirs**: playable by everyone, never handed out as a file.

A file that reaches a client can be copied — encryption or obfuscation only delays a determined user, because the client must decrypt it to run it. The only real protection is that the file never leaves the server: the client sends the game situation and receives the bot's move. This change adds that as a second distribution tier alongside today's downloads, and gives the owner a dashboard to decide which model sits in which tier.

## What Changes

- **Two distribution tiers per model.** A new `distribution` field on `ai_models`: `download` (public file, cached and run locally, works offline) or `stream` (file stays on the server; moves are served by an inference endpoint). **New uploads default to `stream`.**
- **Private storage for stream models.** Uploads land in a private Spaces bucket with no CDN. Promoting a model to `download` copies it to the public bucket; demoting deletes the public copy and purges the CDN. **BREAKING (storage):** the presigned upload no longer sets `public-read`.
- **Manifest lists every published model, but only `download` entries carry `url`, `file_sha256` and `file_size_bytes`.** Stream entries carry the same contract metadata plus `distribution: "stream"`, so clients can list and validate them without being able to fetch them.
- **Bot inference endpoint** `POST /bots/{model_id}/act`: takes the observation, the legal-action mask and (for LSTM models) the recurrent state, and returns the chosen action and the next state. Stateless on the server, login required, rate-limited per account, contract-checked against the client's engine. Action selection reuses the Rust engine's policy code through the PyO3 bindings, so a streamed move is the same move a local run of that model would make.
- **Client policy selection.** Desktop (Tauri) picks local inference for a cached `download` model and the streaming endpoint otherwise, behind the same policy interface the agent loop already uses. The shared frontend model picker shows tier, an "online" badge for stream models, and handles stream-model errors (offline, rate-limited, incompatible).
- **Model management dashboard** (admin): extends the existing `/admin/models` page with a tier column and filter, a one-way promotion flow with an explicit confirmation, demotion, publish/unpublish and deprecate, per-model usage and rate-limit statistics, per-model rate-limit overrides, and an audit log of every tier and visibility change.
- **Migration.** Existing rows become `stream` and their objects move to private storage (safe by default). The owner then promotes the models they are happy to give away from the dashboard. Files that desktop users already downloaded stay out there; this change cannot recall them.

## Capabilities

### New Capabilities

- `model-distribution-tiers`: the `download` / `stream` tier model — storage placement and visibility per tier, manifest shape per tier, upload defaults, promotion/demotion semantics (one-way exposure), and the data migration for existing models.
- `bot-inference-streaming`: the server-side move endpoint — request/response contract, stateless recurrent-state round trip, auth, per-account rate limiting, contract validation, action-selection parity with local inference, and client selection between local and streamed play (desktop now; web through the same interface once a browser engine exists).
- `model-admin-dashboard`: the owner's model-management surface — tier and visibility controls, the promotion confirmation, deprecation, usage and rate-limit statistics, per-model limits, and the audit log.

### Modified Capabilities

(none — no existing spec covers the model catalog, its manifest, or the admin model page; `docs/MODEL_CATALOG.md` is updated as documentation)

## Impact

- **Database**: Alembic migration adding `ai_models.distribution`, `ai_models.public_spaces_key`, `ai_models.rate_limit_per_minute`, `ai_models.requires_account`; new tables `model_distribution_audits` and `model_usage_daily`.
- **Storage**: a second, private Spaces bucket (`SPACES_PRIVATE_BUCKET`, no CDN); `server/storage/spaces.py` gains bucket-aware helpers, server-side copy, and CDN purge. Deploy needs the new bucket and env vars (`docs/DEPLOYMENT.md`).
- **Server**: `db/routers/admin_models.py` (manifest shape, admin tier actions, stats, audit), new `routers/bots.py` (inference endpoint), `storage/model_resolver.py` (read from either bucket), `server/limiter.py` (per-user, per-model limits).
- **Bindings**: `digimon-engine-py` exposes a stateless `predict(obs, mask, h, c) -> (action, h, c)` over the existing Rust `OnnxPolicy`.
- **Desktop**: `src-tauri/src/models.rs` (manifest tier field, skip download for stream models), a remote policy in the agent loop (`engine_commands.rs` / `inference_state.rs`).
- **Frontend**: model picker (tier badge, stream error states), `AdminModelsPage` + modals (tier, promotion dialog, stats, audit log).
- **Docs**: `docs/MODEL_CATALOG.md`, `docs/DEPLOYMENT.md`.
- **Out of scope**: the in-browser (WebAssembly) engine and onnxruntime-web local inference for the website — a separate change. This change defines the client-selection rule the web client will follow and makes streaming usable from any client that can produce an observation (including the hosted API's own server-side games, which already load models from storage). Also out of scope: watermarking models, quantization, and preventing imitation-by-querying beyond rate limits.

# Environment Variables

This document catalogs every environment variable consumed by the project,
grouped by subsystem. `.env.example` at the repo root is the canonical seed
file — the FastAPI app auto-loads it at startup, so **no manual shell export
is required** for server-side vars. Frontend (Vite) vars must be set in
`frontend/.env.local` or passed at build time.

Legend:
- **Required** — service fails loudly (typically `RuntimeError` at call time)
  when unset. Fail-on-first-use is preferred over fail-on-import so modules
  stay importable without credentials.
- **Optional** — has a sensible default or only activates a feature branch.
- **Default** column — value used when the var is unset.

---

## Model publishing — DigitalOcean Spaces

Used by the hosted API's `/admin/models` and public `/models/manifest.json`
endpoints, and by the desktop client when it fetches the manifest. See
[ARCHITECTURE.md → Publishing Models (Ops)](ARCHITECTURE.md#publishing-models-ops)
for the end-to-end flow.

| Var | Required | Default | Used by | Purpose |
|-----|----------|---------|---------|---------|
| `SPACES_ENDPOINT` | yes† | — | `digimon_gym/storage/spaces.py` | Spaces origin, e.g. `https://nyc3.digitaloceanspaces.com`. Consumed by boto3 and the manifest URL fallback. |
| `SPACES_BUCKET` | yes | — | `digimon_gym/storage/spaces.py` | Bucket name, e.g. `digimon-tcg-models`. |
| `SPACES_REGION` | yes | — | `digimon_gym/storage/spaces.py` | Region slug, e.g. `nyc3`. Required for signature-v4. |
| `SPACES_KEY` | yes | — | `digimon_gym/storage/spaces.py` | Spaces access key (admin-level IAM **not** required — per-object `ACL="public-read"` suffices). |
| `SPACES_SECRET` | yes | — | `digimon_gym/storage/spaces.py` | Spaces secret key. |
| `SPACES_CDN_URL` | optional | — (falls back to origin) | `digimon_gym/storage/spaces.py:public_url()` | CDN base URL, e.g. `https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com`. When set, manifest `url` fields become `{SPACES_CDN_URL}/{key}` (no bucket path). When unset, manifest falls back to path-style `{SPACES_ENDPOINT}/{bucket}/{key}`. Switching between modes requires no desktop client change — SHA256+size verification matches either way. Trailing slash and whitespace-only values are normalized/ignored. |

† `SPACES_ENDPOINT` is only dereferenced by `public_url()` when
`SPACES_CDN_URL` is unset. A read-only frontend service that never issues
presigned PUTs can run with only `SPACES_CDN_URL` set, but the hosted API
(which presigns + streams downloads for SHA hashing on confirm) needs the
full block.

**Dev vs. prod:** leave `SPACES_CDN_URL` unset in dev to hit the origin
directly; set it in staging/prod to serve through the edge cache.

---

## Database & auth

| Var | Required | Default | Used by | Purpose |
|-----|----------|---------|---------|---------|
| `DATABASE_URL` | optional | `sqlite+aiosqlite:///./data/app.db` | `digimon_gym/db/database.py` | SQLAlchemy async URL. Production should point at PostgreSQL, e.g. `postgresql+asyncpg://user:pw@host/db`. |

---

## AI pipeline (hosted API `/admin/*` routes)

All variables below are consumed by `digimon_gym/ai/`. The pipeline is
opt-in — nothing here is required for core gameplay, PvP, or RL training.

### Provider selection & credentials

| Var | Required | Default | Purpose |
|-----|----------|---------|---------|
| `AI_PROVIDER` | optional | `openai` | Provider routing. Set to `anthropic` to use Claude models via `ANTHROPIC_API_KEY`. |
| `OPENAI_API_KEY` | required if `AI_PROVIDER=openai` | — | OpenAI credentials. Also used by `digimon_gym/ai/retrieval.py` for Pinecone-adjacent embedding calls. |
| `ANTHROPIC_API_KEY` | required if `AI_PROVIDER=anthropic` | — | Anthropic credentials. |
| `PINECONE_API_KEY` | optional | — | Required only for the `/implement-archetype` sub-agent retrieval flow (see `docs/TOOLS.md` §5). |

### OpenAI model names & pricing (per-million tokens, USD)

| Var | Default | Purpose |
|-----|---------|---------|
| `OPENAI_MODEL_REVIEW` | `gpt-4.1` | Model for batch review tasks. |
| `OPENAI_MODEL_QA` | `gpt-4.1-mini` | Model for QA analysis. |
| `OPENAI_MODEL_ENGINE` | `gpt-4.1-mini` | Model for engine-audit tasks. |
| `OPENAI_MODEL_AUTOFIX` | falls back to `OPENAI_MODEL_REVIEW` | Model for script autofix. |
| `OPENAI_PRICE_GPT41_INPUT` | `2.0` | Input price for gpt-4.1 (USD per 1M tokens). |
| `OPENAI_PRICE_GPT41_OUTPUT` | `8.0` | Output price for gpt-4.1. |
| `OPENAI_PRICE_GPT41_MINI_INPUT` | `0.4` | Input price for gpt-4.1-mini. |
| `OPENAI_PRICE_GPT41_MINI_OUTPUT` | `1.6` | Output price for gpt-4.1-mini. |
| `OPENAI_MAX_OUTPUT_TOKENS_REVIEW` | `1200` | Output cap for review tasks. |
| `OPENAI_MAX_OUTPUT_TOKENS_QA` | `800` | Output cap for QA tasks. |
| `OPENAI_MAX_OUTPUT_TOKENS_ENGINE` | `800` | Output cap for engine-audit tasks. |
| `OPENAI_MAX_OUTPUT_TOKENS_AUTOFIX` | `6000` | Output cap for autofix tasks. |

### Anthropic model names & pricing (per-million tokens, USD)

| Var | Default | Purpose |
|-----|---------|---------|
| `ANTHROPIC_MODEL_REVIEW` | `claude-sonnet-4-6` | Model for batch review tasks. |
| `ANTHROPIC_MODEL_QA` | `claude-sonnet-4-6` | Model for QA tasks. |
| `ANTHROPIC_MODEL_ENGINE` | `claude-sonnet-4-6` | Model for engine audit. |
| `ANTHROPIC_MODEL_AUTOFIX` | `claude-sonnet-4-6` | Model for autofix. |
| `ANTHROPIC_PRICE_SONNET_INPUT` | `3.0` | Input price for Sonnet. |
| `ANTHROPIC_PRICE_SONNET_OUTPUT` | `15.0` | Output price for Sonnet. |
| `ANTHROPIC_PRICE_OPUS_INPUT` | `15.0` | Input price for Opus. |
| `ANTHROPIC_PRICE_OPUS_OUTPUT` | `75.0` | Output price for Opus. |
| `ANTHROPIC_PRICE_HAIKU_INPUT` | `0.8` | Input price for Haiku. |
| `ANTHROPIC_PRICE_HAIKU_OUTPUT` | `4.0` | Output price for Haiku. |
| `ANTHROPIC_MAX_OUTPUT_TOKENS_REVIEW` | `1200` | Output cap for review. |
| `ANTHROPIC_MAX_OUTPUT_TOKENS_QA` | `800` | Output cap for QA. |
| `ANTHROPIC_MAX_OUTPUT_TOKENS_ENGINE` | `800` | Output cap for engine audit. |
| `ANTHROPIC_MAX_OUTPUT_TOKENS_AUTOFIX` | `6000` | Output cap for autofix. |

### AI pipeline orchestration

| Var | Default | Used by | Purpose |
|-----|---------|---------|---------|
| `AI_TASK_MAX_COST_USD` | `5.0` | `digimon_gym/db/routers/admin_ai.py`, `digimon_gym/ai/worker.py` | Hard cap per task. Over-budget runs abort before issuing the LLM call. |
| `AI_AUTOFIX_CONTEXT_CHARS_PER_FILE` | `8000` | `digimon_gym/ai/autofix_apply.py` | Per-file context window cap. |
| `AI_AUTOFIX_CONTEXT_CHARS_TOTAL` | `16000` | `digimon_gym/ai/autofix_apply.py` | Total context window cap across all files in a task. |
| `AI_BATCH_FAILURE_MIN_SAMPLE` | `10` | `digimon_gym/ai/batch_orchestrator.py` | Minimum sample size before a batch run can be auto-aborted on failure rate. |
| `AI_APPLY_MAIN_ALLOWED` | `0` | `digimon_gym/ai/batch_orchestrator.py`, `digimon_gym/ai/set_run_orchestrator.py` | Set to `1` to permit the AI pipeline to write directly to `main` (worktrees only otherwise). |
| `AI_WORKER_POLL_SECONDS` | `2.0` | `digimon_gym/ai/worker.py` | Worker poll interval. |
| `AI_WORKER_STALE_SECONDS` | `1800` | `digimon_gym/ai/worker.py` | Stale-task reclaim threshold (seconds). |
| `AI_WORKER_MAX_CONCURRENT` | `1` | `digimon_gym/ai/worker.py` | Max parallel AI tasks. |
| `AI_WORKER_DISABLED` | `0` | `digimon_gym/api.py` | Set to `1` to skip starting the AI worker on app startup. |
| `AI_WORKTREE_DIR` | `$TMPDIR/ai_worktrees` | `digimon_gym/ai/git_adapter.py` | Where the pipeline clones isolated worktrees. |

---

## Training worker (hosted API — long-running RL jobs)

| Var | Default | Purpose |
|-----|---------|---------|
| `TRAINING_WORKER_DISABLED` | `0` | Set to `1` to skip starting the training worker on app startup. |
| `TRAINING_WORKER_POLL_SECONDS` | `5.0` | Poll interval for claiming queued training jobs. |
| `TRAINING_WORKER_STALE_SECONDS` | `7200` | Stale-job reclaim threshold. |
| `TRAINING_WORKER_MAX_CONCURRENT` | `1` | Max parallel training jobs. |
| `TRAINING_WORKER_DEVICES` | *(empty)* | Optional comma-separated device list (e.g. `cuda:0,cuda:1`); empty lets SB3 auto-select. |

---

## Matchmaking

| Var | Default | Purpose |
|-----|---------|---------|
| `MATCHMAKING_DISABLED` | `0` | Set to `1` to skip starting the matchmaking background task on app startup. |
| `MATCHMAKING_RANKED_ENABLED` | `0` | Gate for the ranked public queue (`digimon_gym/routers/matchmaking.py`). Leave unset/`0` during alpha — the jank/casual/sweat queues still run. Set to `1` to expose ranked. |

---

## Engine backend selection

| Var | Default | Purpose |
|-----|---------|---------|
| `DIGIMON_BACKEND` | `py` | Selects the game engine driving `DigimonEnv`. Set to `rust` to route through the PyO3 `digimon-engine` binding (requires `maturin develop` in `digimon-engine-py/`). Anything else falls back to the Python engine. |

---

## Model files & paths

| Var | Default | Used by | Purpose |
|-----|---------|---------|---------|
| `ONNX_MODELS_DIR` | `models` | `digimon_gym/engine/model_utils.py` | Local directory where exported ONNX models are written/read. |
| `ARCHITECT_MODELS_DIR` | `architect_runs` | `digimon_gym/routers/deck_optimizer.py` | Output dir for the Architect/Q-DeckRec agent runs. |
| `ARCHETYPE_ALIASES_PATH` | `digimon_gym/data/archetype_aliases.json` | `digimon_gym/digilab_client.py`, `digimon_gym/agents/gauntlet.py` | Override path for the archetype-alias lookup table. |

---

## DigiLab / meta stats (optional enrichment)

| Var | Required | Purpose |
|-----|----------|---------|
| `MOTHERDUCK_TOKEN` | optional | MotherDuck (DuckDB-hosted) token for pulling meta-stat tables. Unset → meta enrichment is skipped. |
| `DIGILAB_CONN_STR` | optional | Override for the DigiLab connection string (defaults to the constant in `digilab_client.py`). |

---

## Debug / misc

| Var | Default | Purpose |
|-----|---------|---------|
| `DEBUG_MODE` | `0` | Set to `1` to enable extra debug routes/logs on the hosted API. |
| `PYTHONIOENCODING` | — | Set to `utf-8` on Windows when handling card text with fullwidth chars (see `MEMORY.md`). |

---

## Frontend (Vite build-time)

Vite vars must be prefixed `VITE_` to be exposed to the browser. Set in
`frontend/.env.local` for dev; pass at build time for production bundles.

| Var | Default | Used by | Purpose |
|-----|---------|---------|---------|
| `VITE_BUILD_TARGET` | *(empty, treated as web)* | `frontend/src/App.tsx` and all desktop gating paths | Set to `desktop` to tree-shake admin/training UI for the Tauri build. Any other value (or unset) produces the hosted web bundle. |
| `VITE_API_URL` | `/api` | `frontend/src/api/client.ts`, `useWebSocketGame.ts`, `useMatchmaking.ts` | Base URL for REST + WebSocket calls. Leave as default for same-origin deployments; point at the hosted API host for split frontend/backend. |
| `VITE_MODELS_MANIFEST_URL` | *(empty)* | `frontend/src/pages/ModelsPage.tsx` | Override for the manifest host. Empty → relative path (`/models/manifest.json`). Useful for pointing the desktop build at a specific hosted API. |

---

## Rust desktop build (compile-time)

The Tauri app has no runtime env vars — gameplay, inference, and the model
cache all run through `digimon-engine` directly. Two compile-time constants
are read via Rust macros:

| Var | Where | Purpose |
|-----|-------|---------|
| `DIGIMON_ENGINE_COMMIT` | `src-tauri/src/models.rs` (`option_env!`) | Engine git SHA baked into the binary, used by the model compatibility gate to display/record which engine build the manifest was validated against. Typically injected by CI at build time. |
| `CARGO_PKG_VERSION` | `src-tauri/src/models.rs` (`env!`) | Version string in the HTTP `User-Agent` when fetching the manifest. Set by Cargo automatically. |

---

## Test-only overrides

Set by fixtures, not expected in deployed environments:

- `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_DEFAULT_REGION` — moto
  uses these to accept credentials in `tests/storage/test_spaces.py` and
  `tests/api/test_admin_models.py`.

---

## Adding a new env var

1. Consume it via `os.environ.get("NAME", default)` (optional) or
   `_require_env("NAME")` (required-at-call-time).
2. Add a row to the appropriate table above.
3. Add a commented entry to `.env.example` with a safe example value (never
   a real secret).
4. If the var gates a feature, confirm tests cover both branches (set and
   unset) — see `tests/storage/test_spaces.py::test_public_url_*` for the
   pattern used by the Spaces CDN toggle.

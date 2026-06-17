# Deployment — Hosted API

This doc covers shipping the hosted API (`digimon_gym.api:app`) to a public
endpoint. Desktop sidecar packaging is separate (`docs/TOOLS.md`).

> **How to actually run a deploy / rollback / restore:** see the procedural
> runbook [`docs/runbooks/api-deploy.md`](runbooks/api-deploy.md) (or the
> `deploy-hosted-api` skill for the happy path).

## Target: DigitalOcean

The recommended topology for alpha:

- **Droplet** (Basic, 2 vCPU / 4 GB is plenty for alpha) — runs the API
  container.
- **Managed Postgres** (smallest tier) — authoritative user / deck / invite
  data.
- **Spaces bucket + Spaces CDN** — stores ONNX model blobs, serves
  downloads directly to the desktop client.
- **Managed Redis** *(optional)* — backs slowapi rate limit counters so
  limits survive process restarts. In-memory is acceptable for single-host
  alpha.

Swap in your own provider freely; nothing below is DO-specific except where
noted.

## Required environment variables

| Var | Required | Notes |
|-----|----------|-------|
| `ENVIRONMENT` | yes | `production` in prod; `development` disables the secret-placeholder guard. |
| `SECRET_KEY` | yes | JWT signing HMAC. Generate once with `python -c "import secrets; print(secrets.token_urlsafe(64))"`. **Must not** equal `CHANGE-ME-IN-PRODUCTION`. |
| `DATABASE_URL` | yes | Async SQLAlchemy URL, e.g. `postgresql+asyncpg://user:pw@host:5432/digimon`. |
| `CORS_ORIGINS` | yes | Comma-separated list of allowed origins. Include the Tauri bundle origin (`tauri://localhost`) and the web frontend. |
| `INVITE_CODES_REQUIRED` | yes (alpha) | `true` to gate `/auth/register`. Mint codes via `POST /admin/invite-codes`. |
| `ENGINE_VERSION` | yes | Semver. Clients whose major differs are closed on WS connect. Bump when tensor/action spec changes. |
| `RATE_LIMIT_STORAGE_URI` | recommended | `redis://…`. Omit for in-process limiter. |
| `SENTRY_DSN` | recommended | Enables Sentry error capture. |
| `LOG_LEVEL` | optional | Default `INFO`. |
| `MODEL_STORAGE_BACKEND` | required for models | `local` or `spaces`. |
| `SPACES_ENDPOINT_URL` / `SPACES_REGION` / `SPACES_BUCKET` / `SPACES_ACCESS_KEY` / `SPACES_SECRET_KEY` / `SPACES_CDN_BASE_URL` | required when `MODEL_STORAGE_BACKEND=spaces` | See the DO Spaces control panel. |

The API fails to start if `ENVIRONMENT != development` and any required
value is missing or left at its placeholder (see
`Settings.assert_production_ready` in `digimon_gym/config.py`).

## Local stack (docker-compose)

```bash
docker compose up --build
# Postgres on 5432, Redis on 6379, API on 8000
```

The compose file pre-wires every setting for local dev; use it as a
reference for production env.

## Production Droplet

```bash
# One-shot, on the Droplet
docker build -f Dockerfile.hosted -t digimon-api:alpha .
docker run -d --name digimon-api \
  -p 8000:8000 \
  --env-file /etc/digimon/api.env \
  --restart unless-stopped \
  digimon-api:alpha

# Migrations (run once per deploy)
docker exec digimon-api alembic upgrade head

# Mint a few invite codes
curl -X POST https://api.example.com/admin/invite-codes \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"count": 20, "note": "alpha wave 1"}'
```

Put Caddy / Nginx in front for TLS termination. WebSocket upgrades just
need `proxy_set_header Upgrade $http_upgrade; proxy_set_header Connection
"upgrade";` — the API itself exposes plain HTTP.

## Readiness / liveness

- `GET /healthz` — liveness, always returns 200 if the process is up.
- `GET /readyz` — readiness, returns 503 until DB + storage are reachable.
  Point your load balancer's health check at `/readyz`.

## Models in DO Spaces

1. Create a Spaces bucket in a region close to your players; enable the
   CDN option on the bucket — note the CDN hostname for
   `SPACES_CDN_BASE_URL`.
2. Generate a Spaces access key (r/w on the bucket).
3. Set the `SPACES_*` env vars on the API container.
4. Upload the first model via `POST /admin/models` + `POST
   /admin/models/{slug}/versions`.

Model download traffic hits the CDN edge, not the Droplet — Droplet egress
stays cheap even as the community grows.

Full reference for the catalog (admin endpoints, CI upload pattern,
desktop cache layout, integrity guarantees, environment variables):
see [MODEL_CATALOG.md](MODEL_CATALOG.md).

## First-user bootstrap

There's no admin-role self-assignment in the API. After the first user
registers normally:

```bash
docker exec -it digimon-api python - <<'PY'
import asyncio
from digimon_gym.db.database import get_session_factory
from digimon_gym.db.auth import assign_role_to_user, ROLE_ADMIN

async def main():
    Session = get_session_factory()
    async with Session() as s:
        # Replace with the user id you want to promote
        await assign_role_to_user(s, "<user-id>", ROLE_ADMIN)
        await s.commit()

asyncio.run(main())
PY
```

Then log in as that user and mint invite codes for the rest of the alpha
cohort.

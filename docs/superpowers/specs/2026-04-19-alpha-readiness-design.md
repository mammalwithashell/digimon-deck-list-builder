# Alpha Readiness Design

**Date:** 2026-04-19
**Scope:** Closed friends-and-family alpha of the Digimon TCG simulator.
**Out of scope:** Card effect script correctness (covered by separate archetype-QA campaign).

## Goals

Audit and close the gaps that block inviting ~5–20 friends-and-family testers to a best-effort-uptime alpha. The alpha distribution is a Windows `.msi` (plus Mac/Linux as byproducts) from GitHub Releases; testers play as anonymous guests against each other via room-code PvP or against an AI on the hosted API. There is **no public hosted web app** for testers in this milestone — the desktop binary is the only supported client.

## Definition of Alpha-Ready

1. A developer can `git push origin main` and the hosted API deploys to a DigitalOcean droplet within ~8 minutes, with Playwright tests gating the merge.
2. A tester can download the `.msi` from GitHub Releases, launch it, sign in anonymously, create a room code, have a second tester join by code, and play a turn to completion.
3. The desktop app can fetch a model from `/models/manifest.json`, download it from DO Spaces, cache it locally, and use it to play vs AI offline.
4. If the droplet dies, Postgres can be restored from the latest Spaces backup in under 15 minutes following a documented runbook.

## Existing Foundation (do not rebuild)

- **Playwright config** — `frontend/playwright.config.ts`, with four engine-behavior tests: `game-loads`, `digivolution`, `memory-accounting`, `timing-regression`.
- **Desktop release workflow** — `.github/workflows/desktop-release.yml` builds `.msi`/`.dmg`/AppImage on `v*` tags and publishes to GitHub Releases.
- **Room-code matchmaking** — `digimon_gym/routers/lobby.py` implements create-room, join-by-code, and lobby listing.
- **Model hosting pipeline** — `digimon_gym/storage/spaces.py` + `digimon_gym/db/routers/admin_models.py` handle upload → confirm → publish; public `/models/manifest.json` endpoint lists published models; `src-tauri/src/models.rs` fetches, SHA-verifies, and caches.
- **Anonymous guest login** — shipped in the alpha desktop release (commit `071c7017`).
- **Database** — Alembic migrations current through `20260225_0010`, SQLAlchemy async, Postgres target.

## Deliverables

### A. Playwright test coverage

Keep the 4 existing engine-behavior tests. Add 3 user-journey tests under `frontend/e2e/`:

1. **`guest-onboarding.spec.ts`** — open app → "Continue as guest" → home page renders → deck builder reachable.
2. **`room-code-pvp.spec.ts`** — two browser contexts in one test: context A creates room (captures code), context B joins via code, both see the board, both submit one action, turn resolves.
3. **`try-online-vs-ai.spec.ts`** — guest → "Try Online vs AI" → game starts against server-side AI → game ends (concede or natural) → returns to home.

All three run against the web frontend bundle (same React code the Tauri app ships). The Tauri-binary-only paths — offline ONNX vs AI, model-manifest fetch + download — are covered instead by two Rust integration tests under `src-tauri/tests/`:

- `it_model_download.rs` — manifest fetch, download, SHA verification, cache-hit on second run.
- `it_offline_game.rs` — offline game vs the null-agent ONNX completes legally.

### B. Playwright test authoring process (this plan only)

The 3 new journey tests are authored by a **separate, fresh-context Opus agent** spawned via `mcp__ccd_session__spawn_task` from the implementing session. The spawned agent:

- inherits a worktree branched from the feature branch,
- has no prior conversation context — reads the changed files to understand the feature,
- authors the Playwright test under `frontend/e2e/`, runs it against a local dev server, iterates until green,
- **owns the feature post-handoff**: if Playwright reveals broken behavior, the tester agent fixes the feature code too,
- opens its own PR back to the parent feature branch.

The implementing agent does not write Playwright tests for the feature it just built. This is scoped to this plan — it is not a general project-wide policy.

### C. Server deployment: Dockerfile, droplet, GHA auto-deploy

**New files:**

- `Dockerfile` (multi-stage, replaces `Dockerfile.hosted`)
- `docker-compose.prod.yml`
- `Caddyfile`
- `.github/workflows/deploy-api.yml`
- `ops/deploy/README.md`
- `ops/backup/pg_backup.sh`

**Dockerfile — three stages:**

1. **Rust builder** (`rust:1.78-slim`) — `cargo build --release -p digimon-engine`, then `maturin build --release --out /wheels` on `digimon-engine-py`.
2. **Python wheel builder** (`python:3.11-slim`) — `pip wheel -r requirements.txt -w /wheels`.
3. **Runtime** (`python:3.11-slim`) — installs wheels including the PyO3 wheel from stage 1, copies `digimon_gym/` + `alembic/` + `alembic.ini`, runs as non-root, `HEALTHCHECK` hits `/health`, `CMD` runs `alembic upgrade head && uvicorn digimon_gym.api:app --host 0.0.0.0 --port 8000`.

Target image size ~400 MB. Tagged `ghcr.io/<owner>/digimon-api:<git-sha>` and `:latest` on push to main.

**docker-compose.prod.yml — four services on the droplet:**

- `api` — our image, env-file driven, `depends_on: postgres`.
- `postgres` — `postgres:16-alpine`, named volume `postgres_data`, healthcheck.
- `caddy` — `caddy:2-alpine`, volumes for `Caddyfile` + cert cache, ports 80/443.
- `backup` — `postgres:16-alpine` with `ops/backup/pg_backup.sh` on cron `0 4 * * *` UTC.

**Caddyfile:**

```
api.yourdomain.com {
  reverse_proxy api:8000
  encode gzip
}
```

Caddy handles ACME TLS automatically.

**deploy-api.yml — on push to main, single workflow with sequential jobs:**

1. **Fast tests:** lint + `pytest -m "not slow"`.
2. **Build image:** `docker buildx build` the API image, push to GHCR as `:<sha>` and `:latest`.
3. **Playwright e2e:** pull the freshly-built image, bring up `api` + `postgres` via a test compose profile inside the GHA runner, wait for `/health`, run `npx playwright test` against `localhost`. Upload HTML report as an artifact on failure.
4. **Deploy:** `appleboy/ssh-action` → `cd /opt/digimon && docker compose pull api && docker compose up -d api && docker image prune -f`.
5. **Post-deploy healthcheck:** GHA curls `https://api.yourdomain.com/health`; workflow fails on non-200.

The same workflow runs on PRs, but with the deploy + healthcheck jobs gated behind `if: github.ref == 'refs/heads/main'`. PRs get the test signal without deploying.

No auto-rollback. Rollback is documented as a one-command SSH step: edit compose to pin `image: ghcr.io/.../digimon-api:<known-good-sha>`, `docker compose up -d api`.

**Droplet spec:** $12/mo, 2 GB RAM, 50 GB SSD, Ubuntu 24.04, NYC3 region (same as Spaces — in-region bucket reads are free). Bootstrap steps (one-time, documented in `ops/deploy/README.md`):

- install Docker + docker compose plugin,
- create `deploy` user in the `docker` group, no sudo,
- clone `docker-compose.prod.yml` + `Caddyfile` + `ops/` to `/opt/digimon`,
- install the deploy SSH public key into `/home/deploy/.ssh/authorized_keys`,
- enable `unattended-upgrades` for security patches,
- set DNS `A` record for `api.yourdomain.com` → droplet IP.

**GitHub Actions secrets:**

- `DROPLET_SSH_KEY` — private half of a dedicated keypair (not the developer's personal key).
- `DROPLET_HOST` — droplet's public IP.
- `DROPLET_USER` — `deploy`.
- GHCR push uses the built-in `GITHUB_TOKEN`.

### D. Database plan

**Persistence:** Postgres 16 in the compose stack on the droplet, named volume. Alembic migrations run on every container boot (`alembic upgrade head` in the API `CMD`).

**Backup:** `ops/backup/pg_backup.sh` runs daily via cron inside the `backup` compose service:

```sh
#!/bin/sh
set -eu
TS=$(date -u +%Y%m%dT%H%M%SZ)
pg_dump -h postgres -U $POSTGRES_USER $POSTGRES_DB \
  | gzip -9 \
  | aws --endpoint-url "$SPACES_ENDPOINT" s3 cp - \
      "s3://$SPACES_BUCKET/backups/digimon-$TS.sql.gz"
```

**Retention:** a Spaces lifecycle rule (set once via `doctl` or the DO web UI) auto-deletes `backups/` objects older than 14 days. No cleanup logic lives in our code.

**Restore runbook** (in `ops/deploy/README.md`): three commands — `aws s3 cp` the desired backup, `gunzip`, `psql` restore into a fresh volume. The runbook MUST be exercised end-to-end before alpha opens; a backup that has never been restored is not a backup.

### E. Model serving to desktop

Existing pipeline is complete; what remains:

1. **Publish a null-agent placeholder** — a fresh SB3 MaskablePPO with untrained weights, exported through `tools/export_onnx.py`. After action masking, random logits produce legal random play — a sufficient placeholder opponent for alpha. Upload via the admin publish flow. Name it `null-agent-v0`; notes field: "Untrained baseline; random legal actions."
2. **Add `tools/export_null_agent.py`** — ~15-line throwaway script that instantiates the policy with the current `policy_kwargs` and calls `export_onnx`, so a future contributor can reproduce without guessing config.
3. **Set `SPACES_CDN_URL` in prod** — desktop clients hit the CDN, not origin.
4. **Confirm no lifecycle rule on Spaces `models/` prefix** — backups expire; models do not.

### F. Desktop installer distribution

Extend `.github/workflows/desktop-release.yml`:

1. **Bake `VITE_API_URL` at build time** — env: `VITE_API_URL: https://api.yourdomain.com` before `npm run build`. Without this the installer ships pointing at `/api` and talks to nothing.
2. **Add release notes template** — `.github/RELEASE_NOTES.md` pulled into the GH Release body. Must mention: Windows SmartScreen warning is expected (unsigned), how to report bugs, known issues.

**Code signing:** explicitly skipped for this milestone. Windows testers click through SmartScreen; Mac testers right-click → Open. Revisit for public beta.

**Auto-updates:** explicitly skipped. Testers re-download when a new `.msi` is posted.

**Release cadence:** bump `Cargo.toml` + `package.json` versions, tag `vX.Y.Z`, push tag. The developer controls when testers get new bits.

### G. Matchmaking scope

Room-code lobby (`digimon_gym/routers/lobby.py`) is the only matchmaking surface advertised for this alpha. The `/matchmaking` queues (jank/casual/sweat) are built but left unadvertised — a population of ~20 will not fill them and the UI would feel broken. Do not remove the code; just don't link to it from the alpha home page.

### H. Alpha readiness checklist

Lives in `docs/ALPHA_READINESS.md`. Every item must be checked before inviting testers:

- [ ] `Dockerfile` multi-stage builds clean locally (`docker build . -t test && docker run test alembic upgrade head`).
- [ ] `docker-compose.prod.yml` brings up API + Postgres + Caddy + backup on a scratch droplet.
- [ ] DNS `A` record `api.yourdomain.com` → droplet IP; Caddy gets a cert on first boot.
- [ ] GHA `deploy-api.yml` deploys on push to main; post-deploy healthcheck green.
- [ ] Nightly `pg_backup.sh` runs; restore runbook exercised end-to-end at least once.
- [ ] Playwright CI green on the 3 new journey tests + 4 existing engine tests.
- [ ] `src-tauri/tests/it_model_download.rs` + `it_offline_game.rs` green in CI.
- [ ] `null-agent-v0` published and visible in `/models/manifest.json`.
- [ ] Tagged `v0.1.0-alpha`; `desktop-release.yml` produced `.msi`/`.dmg`/AppImage with `VITE_API_URL` baked in.
- [ ] Installed `.msi` on a clean Windows VM connects to prod API and completes a PvP room-code game.
- [ ] `ops/deploy/README.md` complete: bootstrap, restore runbook, rollback steps.
- [ ] Secrets set on droplet: `SPACES_*`, `JWT_SECRET`, `POSTGRES_PASSWORD`. No defaults left.

## Explicitly Out of Scope

- Public hosted web app (desktop-only distribution).
- Ranked matchmaking (`MATCHMAKING_RANKED_ENABLED` stays 0).
- Code signing and auto-updates for the desktop binary.
- Observability / alerting beyond GHA healthcheck (no Prometheus, no uptime pinger).
- Rate limiting (not needed at ~20 testers).
- Card effect script audit (separate campaign).
- Applying the "spawn fresh tester agent" process to features beyond the 3 Playwright journey tests in this plan.

## Risks

- **Rust build time in CI** (~5–8 min) will dominate deploy latency. Acceptable for F&F push cadence; revisit if it hurts.
- **Null-agent placeholder will feel weak** to any tester who plays vs AI. Mitigated by honest naming in the manifest; real training remains a parallel track.
- **No alerting** means a droplet OOM at 2am is invisible until a tester reports it. Acceptable for best-effort uptime; revisit if F&F testers demand more.
- **Unsigned Windows `.msi`** will trigger SmartScreen. Mitigated by release notes; acceptable for F&F.

## Implementation Handoff

This spec is the input to the `superpowers:writing-plans` skill, which will produce a step-by-step implementation plan with ordering, dependencies, and review checkpoints.

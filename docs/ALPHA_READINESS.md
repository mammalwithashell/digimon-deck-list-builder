# Alpha Readiness Checklist

Every item MUST be checked before inviting the first F&F tester.

## Server

- [ ] `Dockerfile` multi-stage builds clean locally: `docker build -t test .`
- [ ] `docker compose -f docker-compose.prod.yml -f docker-compose.override.local.yml up` brings up API + Postgres; `curl localhost:8000/health` returns 200
- [ ] DuckDNS record `inbetweentheatre.duckdns.org` points at droplet IP; Caddy serves a valid cert (`curl -I https://inbetweentheatre.duckdns.org/health`)
- [ ] GHA `deploy-api.yml` runs green on last push to `main`; post-deploy healthcheck passes
- [ ] Droplet disk usage < 50% (`ssh deploy@host df -h /`)

## Database

- [ ] Nightly `pg_backup.sh` has run at least once; `.sql.gz` visible in `s3://<bucket>/backups/`
- [ ] Restore runbook exercised end-to-end (notes in `ops/deploy/README.md`)
- [ ] Spaces lifecycle rule: 14-day expiry on `backups/` prefix (confirm via DO web UI)

## Tests

- [ ] Playwright CI green: 4 existing engine tests + 3 new journey tests (`guest-onboarding`, `room-code-pvp`, `try-online-vs-ai`)
- [ ] Rust integration tests green: `it_model_download.rs`, `it_offline_game.rs`
- [ ] Fast-tests job green: `pytest -m "not slow"`

## Models

- [ ] `null-agent-v0` published via admin; visible in `https://inbetweentheatre.duckdns.org/models/manifest.json`
- [ ] `SPACES_CDN_URL` set in prod `.env`; manifest URL resolves via CDN
- [ ] No lifecycle rule on `models/` prefix (confirm via DO web UI)

## Desktop

- [ ] Tagged `v0.1.0-alpha`; `desktop-release.yml` produced `.msi` + `.dmg` + `.AppImage`
- [ ] `.msi` installed on a clean Windows VM connects to prod API (check network calls) and completes a PvP room-code game with yourself
- [ ] GH Release body renders the `RELEASE_NOTES.md` template with `{{VERSION}}` substituted

## Secrets hygiene

- [ ] Droplet `/opt/digimon/.env`: real `SPACES_*`, `JWT_SECRET`, `POSTGRES_PASSWORD`, `TRAINING_WORKER_DISABLED=1`, `AI_WORKER_DISABLED=1`. No defaults from any example file.
- [ ] GH Actions secrets: `DROPLET_SSH_KEY`, `DROPLET_HOST`, `DROPLET_USER` present and unique to this project
- [ ] Deploy SSH keypair is dedicated (not a developer's personal key)

## Ops

- [ ] `ops/deploy/README.md` complete: bootstrap, rollback, restore. Updated with any bootstrap-time deviations.
- [ ] Last rollback drill (manual `docker compose` edit + `up -d`) exercised at least once

## Known-disabled features (F&F tradeoffs)

These admin-facing surfaces are intentionally offline in the alpha image:
- `/admin/training/*` (gauntlet, training jobs)
- `/admin/ai/*` (AI-pipeline card fixing)
- `/deck-optimizer/*` (architect-backed deck optimization)

F&F testers do not touch these paths. If you (as admin) need them, spin up a dev server from a full `requirements.txt` install.

---

When all boxes ticked: tag `v0.1.0-alpha`, push tag, share the GH Release link with testers.

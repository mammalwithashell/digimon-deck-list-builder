---
name: deploy-hosted-api
description: Deploy the hosted API server (build the FastAPI image and roll it out to the DigitalOcean droplet). Use WHENEVER the user wants to deploy / ship / release / push / roll out / update the hosted API, the server, the backend, or prod; redeploy or restart the droplet; publish a new API image; or roll the API back to a known-good build. Dispatches .github/workflows/build-api-image.yml with deploy=true (build + push image, then pull + restart the droplet, then healthcheck). Rollback and Postgres restore are covered too. The authoritative runbook is docs/runbooks/api-deploy.md. NOT for the desktop app (use cut-desktop-release) and NOT for the landing page (landing-page.yml).
---

# Deploy the hosted API

Ship the FastAPI server image to the DigitalOcean droplet. The recurring deploy
is one manually-dispatched workflow:
[`.github/workflows/build-api-image.yml`](../../../.github/workflows/build-api-image.yml)
builds + pushes the image to GHCR, and (with `deploy=true`) SSHes the droplet to
`docker compose pull` + `up -d` + healthcheck. The authoritative runbook —
bootstrap, rollback, restore — is
[`docs/runbooks/api-deploy.md`](../../../docs/runbooks/api-deploy.md).

This is an **outward-facing, hard-to-reverse** action (it restarts prod for real
users). Confirm with the user that `main` carries the intended changes before
dispatching, and that DB migrations in the image are expected (the API runs them
on boot).

## Preconditions

- The change is **merged to `main`** — the image builds from the dispatched ref
  (default `main`). Don't deploy a feature branch.
- `gh` is authenticated with workflow-dispatch rights on the repo.
- The droplet secrets (`DROPLET_HOST`, `DROPLET_USER`, `DROPLET_SSH_KEY`) are set
  in the repo (already configured for the live droplet).

## Steps

### 1. Dispatch the build-and-deploy

```bash
gh workflow run build-api-image.yml -f deploy=true
```

Omit `-f deploy=true` (or pass `deploy=false`) to **only build + push** the image
without touching the droplet — useful to stage an image before a deliberate
rollout.

### 2. Watch the run

```bash
gh run list --workflow=build-api-image.yml --limit 1
gh run watch <run-id>      # or open the Actions URL
```

The `build-push` job tags the image with the 12-char git SHA **and** `latest`.
The `deploy` job (only when `deploy=true`) pulls on the droplet, recreates the
stack, and runs a post-deploy healthcheck.

### 3. Verify prod is healthy

```bash
curl -sf https://inbetweentheatre.duckdns.org/health && echo " <- API up"
```

Expected: a healthy response. The `deploy` job itself gates on `/health` (it
polls for ~2 min and fails the run if the API never comes up), so a green run
already means this passed — this curl is a belt-and-suspenders external check.

## Rollback

Every build tags the image with its git SHA, so rollback = re-pin the droplet to
a known-good SHA:

```bash
ssh <DROPLET_USER>@<DROPLET_HOST>
cd /opt/digimon
export API_IMAGE=ghcr.io/mammalwithashell/digimon-api:<known-good-sha>
docker compose -f docker-compose.prod.yml pull api
docker compose -f docker-compose.prod.yml up -d api
curl -sf https://inbetweentheatre.duckdns.org/health
```

Full procedure (and pinning the tag in compose) is in the runbook.

## Gotchas

- **Deployed a branch by mistake.** The image builds from the dispatched ref;
  default is `main`. Merge first; don't dispatch from a feature branch.
- **Migrations.** The API runs Alembic migrations on boot — a deploy is also a
  migration. Confirm the migration is intended.
- **Torch-free image.** The server image drops torch + heavy AI wheels, so
  `/admin/training/*` and `/admin/ai/*` 500 by design
  (`TRAINING_WORKER_DISABLED=1`, `AI_WORKER_DISABLED=1`). That's expected, not a
  failed deploy.
- **Restore ≠ deploy.** Recovering Postgres from a Spaces backup is a separate
  procedure in the runbook; don't conflate it with a code rollout.

## Reference

`docs/runbooks/api-deploy.md` is authoritative — one-time droplet bootstrap, the
rollback procedure, the Postgres restore runbook, and the tooling/secrets table
all live there. `docs/DEPLOYMENT.md` covers the topology and required env vars.

# Alpha Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the hosted API deployable via GHA→droplet auto-deploy, add Playwright journey + Tauri integration tests that gate merges, publish a null-agent model, and ship a `.msi` that points at the prod API — so friends-and-family testers can install from GitHub Releases and play room-code PvP.

**Architecture:** DigitalOcean droplet ($12/mo, 2 GB) runs `docker compose` with four services (API, Postgres, Caddy, Postgres backup). GitHub Actions builds a multi-stage Dockerfile on push to `main`, pushes the image to GHCR, SSHes into the droplet, pulls and restarts. Caddy handles TLS. Nightly `pg_dump` pipes to DO Spaces. Desktop binaries are built by the existing `desktop-release.yml` on `v*` tags and bake the prod API URL at build time.

**Tech Stack:** Docker multi-stage build (Rust + Python runtime), docker-compose v2, Caddy 2, Postgres 16, DigitalOcean droplet + Spaces, GitHub Actions, Playwright, Tauri v2.

---

## Spec reference

All design decisions live in [`docs/superpowers/specs/2026-04-19-alpha-readiness-design.md`](../specs/2026-04-19-alpha-readiness-design.md). If a task here conflicts with the spec, the spec wins.

## File Structure

**New files:**

- `Dockerfile` — multi-stage: Rust builder → Python wheel builder → runtime. Replaces `Dockerfile.hosted`.
- `docker-compose.prod.yml` — droplet topology: api + postgres + caddy + backup.
- `Caddyfile` — single site block, reverse-proxy to api:8000.
- `ops/backup/pg_backup.sh` — nightly `pg_dump | gzip | s3 cp` script.
- `ops/deploy/README.md` — droplet bootstrap + rollback + restore runbook.
- `.github/workflows/deploy-api.yml` — test → build → push → ssh-deploy.
- `tools/export_null_agent.py` — instantiate MaskablePPO with untrained weights, export to ONNX.
- `src-tauri/tests/it_model_download.rs` — manifest fetch + SHA check + cache hit.
- `src-tauri/tests/it_offline_game.rs` — offline game vs null-agent completes legally.
- `frontend/e2e/guest-onboarding.spec.ts`
- `frontend/e2e/room-code-pvp.spec.ts`
- `frontend/e2e/try-online-vs-ai.spec.ts`
- `.github/RELEASE_NOTES.md` — template pulled into GH Release bodies.
- `docs/ALPHA_READINESS.md` — final go/no-go checklist.

**Modified files:**

- `.github/workflows/desktop-release.yml` — bake `VITE_API_URL` at build time.

**Deleted files:**

- `Dockerfile.hosted` — superseded by new `Dockerfile`.

---

## Task 1: Multi-stage Dockerfile for the API

**Files:**
- Create: `Dockerfile`
- Delete (end of Task 4): `Dockerfile.hosted`

- [ ] **Step 1: Create `Dockerfile`**

```dockerfile
# syntax=docker/dockerfile:1.6

# ── Stage 1: Rust builder (produces the PyO3 wheel) ───────────────────────
FROM rust:1.78-slim AS rust-builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3.11 python3.11-dev python3-pip pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN pip3 install --no-cache-dir --break-system-packages maturin==1.5.1
COPY Cargo.toml Cargo.lock ./
COPY digimon-engine/ digimon-engine/
COPY digimon-engine-py/ digimon-engine-py/
RUN cd digimon-engine-py && maturin build --release --out /wheels

# ── Stage 2: Python wheel builder ─────────────────────────────────────────
FROM python:3.11-slim AS py-builder
WORKDIR /build
COPY requirements.txt .
RUN pip wheel --no-cache-dir -r requirements.txt -w /wheels
COPY --from=rust-builder /wheels/*.whl /wheels/

# ── Stage 3: Runtime ──────────────────────────────────────────────────────
FROM python:3.11-slim AS runtime
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1
RUN useradd -u 1001 -m app
WORKDIR /app
COPY --from=py-builder /wheels /wheels
RUN pip install --no-cache-dir /wheels/*.whl && rm -rf /wheels
COPY digimon_gym/ digimon_gym/
COPY alembic/ alembic/
COPY alembic.ini .
USER app
EXPOSE 8000
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8000/health').read()" || exit 1
CMD ["sh", "-c", "alembic upgrade head && uvicorn digimon_gym.api:app --host 0.0.0.0 --port 8000"]
```

- [ ] **Step 2: Build the image locally**

```bash
docker build -t digimon-api:local .
```
Expected: build completes in 5-10 min; final image ~400 MB.

- [ ] **Step 3: Smoke-run (without DB — migrations will fail, but startup should get far)**

```bash
docker run --rm digimon-api:local python -c "import digimon_engine; import digimon_gym.api; print('imports OK')"
```
Expected: prints `imports OK`.

- [ ] **Step 4: Commit**

```bash
git add Dockerfile
git commit -m "build: multi-stage Dockerfile for hosted API"
```

---

## Task 2: docker-compose.prod.yml

**Files:**
- Create: `docker-compose.prod.yml`

- [ ] **Step 1: Create `docker-compose.prod.yml`**

```yaml
services:
  postgres:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_USER: ${POSTGRES_USER}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: ${POSTGRES_DB}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${POSTGRES_USER}"]
      interval: 10s
      timeout: 5s
      retries: 5

  api:
    image: ${API_IMAGE:-ghcr.io/OWNER/digimon-api:latest}
    restart: unless-stopped
    env_file: .env
    environment:
      DATABASE_URL: postgresql+asyncpg://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/${POSTGRES_DB}
    depends_on:
      postgres:
        condition: service_healthy
    expose:
      - "8000"

  caddy:
    image: caddy:2-alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    depends_on:
      - api

  backup:
    image: postgres:16-alpine
    restart: unless-stopped
    env_file: .env
    environment:
      POSTGRES_USER: ${POSTGRES_USER}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: ${POSTGRES_DB}
      PGPASSWORD: ${POSTGRES_PASSWORD}
    volumes:
      - ./ops/backup/pg_backup.sh:/usr/local/bin/pg_backup.sh:ro
    entrypoint:
      - sh
      - -c
      - |
        apk add --no-cache aws-cli dcron && \
        echo "0 4 * * * /usr/local/bin/pg_backup.sh" | crontab - && \
        crond -f -d 8
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  postgres_data:
  caddy_data:
  caddy_config:
```

- [ ] **Step 2: Replace `OWNER` placeholder**

Edit line with `ghcr.io/OWNER/digimon-api:latest` — replace `OWNER` with the actual GitHub org/user. Grep to verify no other `OWNER` placeholder remains.

- [ ] **Step 3: Commit**

```bash
git add docker-compose.prod.yml
git commit -m "build: prod docker-compose with api+postgres+caddy+backup"
```

---

## Task 3: Caddyfile

**Files:**
- Create: `Caddyfile`

- [ ] **Step 1: Create `Caddyfile`**

```caddy
api.yourdomain.com {
  reverse_proxy api:8000
  encode gzip
  header {
    Strict-Transport-Security "max-age=31536000; includeSubDomains"
    X-Content-Type-Options "nosniff"
    Referrer-Policy "strict-origin-when-cross-origin"
  }
}
```

- [ ] **Step 2: Replace `api.yourdomain.com`**

Replace with the actual hostname the developer plans to use for the prod API. Decide this now — it needs to match the DNS record set in Task 8 and the `VITE_API_URL` in Task 17.

- [ ] **Step 3: Commit**

```bash
git add Caddyfile
git commit -m "build: Caddyfile for reverse proxy + TLS"
```

---

## Task 4: Remove superseded `Dockerfile.hosted`

**Files:**
- Delete: `Dockerfile.hosted`

- [ ] **Step 1: Verify nothing references it**

```bash
grep -rn "Dockerfile.hosted" --exclude-dir=.git
```
Expected: no results (or only the file itself).

- [ ] **Step 2: Delete and commit**

```bash
git rm Dockerfile.hosted
git commit -m "build: remove broken Dockerfile.hosted (superseded by multi-stage Dockerfile)"
```

---

## Task 5: Postgres backup script

**Files:**
- Create: `ops/backup/pg_backup.sh`

- [ ] **Step 1: Create `ops/backup/pg_backup.sh`**

```sh
#!/bin/sh
set -eu

: "${POSTGRES_USER:?required}"
: "${POSTGRES_DB:?required}"
: "${SPACES_ENDPOINT:?required}"
: "${SPACES_BUCKET:?required}"
: "${SPACES_KEY:?required}"
: "${SPACES_SECRET:?required}"

export AWS_ACCESS_KEY_ID="$SPACES_KEY"
export AWS_SECRET_ACCESS_KEY="$SPACES_SECRET"

TS=$(date -u +%Y%m%dT%H%M%SZ)
KEY="backups/digimon-${TS}.sql.gz"

pg_dump -h postgres -U "$POSTGRES_USER" "$POSTGRES_DB" \
  | gzip -9 \
  | aws --endpoint-url "$SPACES_ENDPOINT" s3 cp - "s3://${SPACES_BUCKET}/${KEY}"

echo "backup complete: s3://${SPACES_BUCKET}/${KEY}"
```

- [ ] **Step 2: `chmod +x`**

```bash
chmod +x ops/backup/pg_backup.sh
```

- [ ] **Step 3: Dry-run the script against a local postgres**

Spin up the compose stack locally with test secrets (use `.env.local` with dummy Spaces creds — this step is just to confirm the script runs until the `aws` call, which will fail with InvalidAccessKeyId; that's expected).

```bash
docker compose -f docker-compose.prod.yml --profile backup run --rm backup \
  sh /usr/local/bin/pg_backup.sh
```
Expected: prints the `pg_dump | gzip | aws` pipeline, `aws` call fails with an auth error (good — it means the pipeline got there).

- [ ] **Step 4: Commit**

```bash
git add ops/backup/pg_backup.sh
git commit -m "ops: nightly pg_dump to Spaces"
```

---

## Task 6: Local-stack smoke test

**Files:** none new. This task verifies Tasks 1–5 work together.

- [ ] **Step 1: Create a `.env.local` for testing (gitignored)**

```sh
cat > .env.local <<'EOF'
POSTGRES_USER=digimon
POSTGRES_PASSWORD=localdevpassword
POSTGRES_DB=digimon
JWT_SECRET=localdevsecret
SPACES_ENDPOINT=https://nyc3.digitaloceanspaces.com
SPACES_BUCKET=digimon-tcg-models
SPACES_REGION=nyc3
SPACES_KEY=dummy
SPACES_SECRET=dummy
SPACES_CDN_URL=https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com
API_IMAGE=digimon-api:local
EOF
```

Verify `.env.local` is in `.gitignore` (add it if missing).

- [ ] **Step 2: Comment out the `caddy` service temporarily**

Caddy will fail locally trying to acquire a cert for a real domain. Comment out the `caddy:` block in `docker-compose.prod.yml` for this smoke test, or use an override file.

Simpler: create `docker-compose.override.local.yml`:

```yaml
services:
  api:
    ports:
      - "8000:8000"
  caddy:
    profiles: ["noop"]  # disabled for local smoke
```

- [ ] **Step 3: Bring up the stack**

```bash
docker compose --env-file .env.local \
  -f docker-compose.prod.yml -f docker-compose.override.local.yml \
  up -d postgres api
```

Wait ~20 seconds for Postgres → migrations → API.

- [ ] **Step 4: Hit `/health`**

```bash
curl -sf http://localhost:8000/health
```
Expected: `{"status":"ok"}`.

- [ ] **Step 5: Tear down**

```bash
docker compose --env-file .env.local \
  -f docker-compose.prod.yml -f docker-compose.override.local.yml \
  down -v
```

- [ ] **Step 6: Commit override file**

```bash
git add docker-compose.override.local.yml .gitignore
git commit -m "build: local smoke override for docker-compose.prod"
```

---

## Task 7: Ops deployment README (bootstrap + rollback + restore)

**Files:**
- Create: `ops/deploy/README.md`

- [ ] **Step 1: Create `ops/deploy/README.md`**

```markdown
# Deploying the hosted API

## One-time droplet bootstrap

Replace `api.yourdomain.com` with your chosen hostname throughout.

```bash
# 1. Generate a dedicated deploy keypair on your laptop
ssh-keygen -t ed25519 -f ~/.ssh/digimon_deploy -C "gha-deploy" -N ""

# 2. Register the public key with DigitalOcean
doctl compute ssh-key import digimon-deploy \
  --public-key-file ~/.ssh/digimon_deploy.pub
# Note the key id that's returned.

# 3. Provision the droplet
doctl compute droplet create digimon-api \
  --region nyc3 \
  --size s-1vcpu-2gb \
  --image ubuntu-24-04-x64 \
  --ssh-keys <key-id-from-step-2> \
  --wait

# 4. Note the droplet's public IP
doctl compute droplet list --format ID,Name,PublicIPv4

# 5. Create DNS A record
# In your DNS provider, add:  api.yourdomain.com  A  <droplet-ip>  TTL=300
# Wait for propagation: `dig api.yourdomain.com` should return the IP.
```

SSH in once as root to finish setup:

```bash
ssh -i ~/.ssh/digimon_deploy root@<droplet-ip>

# On the droplet:
apt-get update && apt-get install -y ca-certificates curl
install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg \
  -o /etc/apt/keyrings/docker.asc
chmod a+r /etc/apt/keyrings/docker.asc
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo $VERSION_CODENAME) stable" \
  | tee /etc/apt/sources.list.d/docker.list > /dev/null
apt-get update
apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

# Non-root deploy user
useradd -m -s /bin/bash deploy
usermod -aG docker deploy
install -d -m700 -o deploy -g deploy /home/deploy/.ssh
cp /root/.ssh/authorized_keys /home/deploy/.ssh/authorized_keys
chown deploy:deploy /home/deploy/.ssh/authorized_keys
chmod 600 /home/deploy/.ssh/authorized_keys

# Unattended security updates
apt-get install -y unattended-upgrades
dpkg-reconfigure -plow unattended-upgrades

# App directory
mkdir -p /opt/digimon
chown deploy:deploy /opt/digimon
```

Copy deploy files from your laptop:

```bash
scp -i ~/.ssh/digimon_deploy \
  docker-compose.prod.yml Caddyfile \
  deploy@<droplet-ip>:/opt/digimon/
scp -i ~/.ssh/digimon_deploy -r ops/ \
  deploy@<droplet-ip>:/opt/digimon/
```

Create `/opt/digimon/.env` on the droplet with real secrets:

```sh
POSTGRES_USER=digimon
POSTGRES_PASSWORD=<generate with: openssl rand -hex 24>
POSTGRES_DB=digimon
JWT_SECRET=<generate with: openssl rand -hex 32>
SPACES_ENDPOINT=https://nyc3.digitaloceanspaces.com
SPACES_BUCKET=digimon-tcg-models
SPACES_REGION=nyc3
SPACES_KEY=<from DO Spaces access keys page>
SPACES_SECRET=<from DO Spaces access keys page>
SPACES_CDN_URL=https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com
MATCHMAKING_RANKED_ENABLED=0
```

First boot:

```bash
cd /opt/digimon
docker login ghcr.io -u <your-github-user> -p <a GHCR read token>
docker compose -f docker-compose.prod.yml pull
docker compose -f docker-compose.prod.yml up -d
docker compose logs -f api      # watch migrations finish
curl -sf https://api.yourdomain.com/health
```

## Rollback

Every deploy tags the image with the git SHA. To roll back to a known-good SHA:

```bash
ssh deploy@<droplet-ip>
cd /opt/digimon
# Option A — override via env
export API_IMAGE=ghcr.io/OWNER/digimon-api:<known-good-sha>
docker compose -f docker-compose.prod.yml pull api
docker compose -f docker-compose.prod.yml up -d api
# Option B — edit docker-compose.prod.yml to pin the image tag, commit, redeploy.
```

## Restore Postgres from Spaces backup

```bash
ssh deploy@<droplet-ip>
cd /opt/digimon
# List available backups
docker compose -f docker-compose.prod.yml run --rm backup \
  aws --endpoint-url "$SPACES_ENDPOINT" s3 ls s3://$SPACES_BUCKET/backups/

# Download the desired backup
docker compose -f docker-compose.prod.yml run --rm backup sh -c '
  aws --endpoint-url "$SPACES_ENDPOINT" s3 cp \
    s3://$SPACES_BUCKET/backups/digimon-<TIMESTAMP>.sql.gz \
    /tmp/restore.sql.gz && \
  gunzip /tmp/restore.sql.gz && \
  cat /tmp/restore.sql'  > ./restore.sql

# Stop the API, drop and recreate the database, replay the dump
docker compose -f docker-compose.prod.yml stop api
docker compose -f docker-compose.prod.yml exec postgres \
  psql -U $POSTGRES_USER -c "DROP DATABASE IF EXISTS $POSTGRES_DB;"
docker compose -f docker-compose.prod.yml exec postgres \
  psql -U $POSTGRES_USER -c "CREATE DATABASE $POSTGRES_DB;"
docker compose -f docker-compose.prod.yml exec -T postgres \
  psql -U $POSTGRES_USER -d $POSTGRES_DB < ./restore.sql
docker compose -f docker-compose.prod.yml start api
curl -sf https://api.yourdomain.com/health
```

This runbook MUST be exercised end-to-end once before alpha opens. See Task 9.

## Spaces lifecycle rule (one-time)

Set a 14-day auto-delete rule on the `backups/` prefix of the Spaces bucket via the DO web UI (Spaces → Bucket → Settings → Lifecycle rules). Models under `models/` are NOT subject to any rule.
```

- [ ] **Step 2: Replace `OWNER` and `api.yourdomain.com`**

Grep for `OWNER` and `api.yourdomain.com` in `ops/deploy/README.md` and replace with real values.

- [ ] **Step 3: Commit**

```bash
git add ops/deploy/README.md
git commit -m "ops: deployment runbook (bootstrap, rollback, restore)"
```

---

## Task 8: Execute droplet bootstrap (manual, tracked)

**Files:** none new; this task is the developer running the bootstrap steps and noting completion.

- [ ] **Step 1: Follow `ops/deploy/README.md` "One-time droplet bootstrap"**

Run every command in the "One-time droplet bootstrap" section. Track deviations (anything that failed or needed adjustment) in a section at the bottom of the README under `## Bootstrap notes (YYYY-MM-DD)`.

- [ ] **Step 2: Verify droplet is reachable as `deploy`**

```bash
ssh -i ~/.ssh/digimon_deploy deploy@<droplet-ip> docker ps
```
Expected: empty `docker ps` output (no containers yet; this confirms SSH + docker-group membership work).

- [ ] **Step 3: Set GH Actions secrets**

```bash
gh secret set DROPLET_SSH_KEY < ~/.ssh/digimon_deploy
gh secret set DROPLET_HOST --body "<droplet-ip>"
gh secret set DROPLET_USER --body "deploy"
```

Verify: `gh secret list` shows the three secrets.

- [ ] **Step 4: Commit bootstrap notes**

If you added deviation notes to the README, commit them.

```bash
git add ops/deploy/README.md
git commit -m "ops: bootstrap notes for initial droplet" || echo "no changes"
```

---

## Task 9: Exercise restore runbook once

**Files:** none new.

- [ ] **Step 1: Seed the live database with a recognizable test row**

```bash
ssh deploy@<droplet-ip>
cd /opt/digimon
docker compose -f docker-compose.prod.yml exec postgres \
  psql -U $POSTGRES_USER -d $POSTGRES_DB \
  -c "CREATE TABLE IF NOT EXISTS restore_test (id SERIAL, note TEXT);"
docker compose -f docker-compose.prod.yml exec postgres \
  psql -U $POSTGRES_USER -d $POSTGRES_DB \
  -c "INSERT INTO restore_test (note) VALUES ('pre-restore-canary');"
```

- [ ] **Step 2: Trigger a backup immediately**

```bash
docker compose -f docker-compose.prod.yml exec backup /usr/local/bin/pg_backup.sh
```
Expected: `backup complete: s3://...`.

- [ ] **Step 3: Delete the canary row, then follow the restore runbook**

```bash
docker compose -f docker-compose.prod.yml exec postgres \
  psql -U $POSTGRES_USER -d $POSTGRES_DB \
  -c "DROP TABLE restore_test;"
```

Then follow the "Restore Postgres from Spaces backup" section of `ops/deploy/README.md` top to bottom.

- [ ] **Step 4: Confirm canary returned**

```bash
docker compose -f docker-compose.prod.yml exec postgres \
  psql -U $POSTGRES_USER -d $POSTGRES_DB \
  -c "SELECT note FROM restore_test;"
```
Expected: `pre-restore-canary`.

- [ ] **Step 5: Clean up + note in README**

Drop the `restore_test` table. Add a line to the README bootstrap-notes section: `Restore runbook exercised on YYYY-MM-DD — end-to-end restore succeeded.`

```bash
git add ops/deploy/README.md
git commit -m "ops: restore runbook verified end-to-end"
```

---

## Task 10: `deploy-api.yml` GitHub Actions workflow

**Files:**
- Create: `.github/workflows/deploy-api.yml`

- [ ] **Step 1: Create the workflow**

```yaml
name: Deploy API

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

permissions:
  contents: read
  packages: write

jobs:
  fast-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with: { python-version: "3.11" }
      - run: pip install -r requirements.txt
      - run: python -m pytest tests -m "not slow" -x -q

  build:
    needs: fast-tests
    runs-on: ubuntu-latest
    outputs:
      image_tag: ${{ steps.meta.outputs.tag }}
    steps:
      - uses: actions/checkout@v4
        with: { submodules: recursive }
      - id: meta
        run: echo "tag=${GITHUB_SHA::12}" >> "$GITHUB_OUTPUT"
      - uses: docker/setup-buildx-action@v3
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: |
            ghcr.io/${{ github.repository_owner }}/digimon-api:${{ steps.meta.outputs.tag }}
            ghcr.io/${{ github.repository_owner }}/digimon-api:latest
          cache-from: type=gha
          cache-to: type=gha,mode=max

  e2e:
    needs: build
    runs-on: ubuntu-latest
    env:
      API_IMAGE: ghcr.io/${{ github.repository_owner }}/digimon-api:${{ needs.build.outputs.image_tag }}
      POSTGRES_USER: digimon
      POSTGRES_PASSWORD: cipassword
      POSTGRES_DB: digimon
      JWT_SECRET: cisecret
      SPACES_ENDPOINT: https://nyc3.digitaloceanspaces.com
      SPACES_BUCKET: ci-unused
      SPACES_REGION: nyc3
      SPACES_KEY: ci-unused
      SPACES_SECRET: ci-unused
    steps:
      - uses: actions/checkout@v4
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - name: Start stack
        run: |
          docker compose -f docker-compose.prod.yml -f docker-compose.override.ci.yml up -d postgres api
          # wait for /health
          for i in $(seq 1 60); do
            curl -sf http://localhost:8000/health && exit 0
            sleep 2
          done
          echo "API did not come up"; docker compose logs; exit 1
      - uses: actions/setup-node@v4
        with: { node-version: "lts/*" }
      - working-directory: frontend
        run: |
          npm ci
          npx playwright install --with-deps chromium
          npm run build
          VITE_API_URL=http://localhost:8000 npx playwright test
      - name: Upload Playwright report
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: playwright-report
          path: frontend/playwright-report

  deploy:
    needs: e2e
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: appleboy/ssh-action@v1.0.3
        with:
          host: ${{ secrets.DROPLET_HOST }}
          username: ${{ secrets.DROPLET_USER }}
          key: ${{ secrets.DROPLET_SSH_KEY }}
          script: |
            set -e
            cd /opt/digimon
            docker compose -f docker-compose.prod.yml pull api
            docker compose -f docker-compose.prod.yml up -d api
            docker image prune -f
      - name: Post-deploy healthcheck
        run: |
          for i in $(seq 1 30); do
            curl -sf https://api.yourdomain.com/health && exit 0
            sleep 3
          done
          exit 1
```

- [ ] **Step 2: Create `docker-compose.override.ci.yml`**

```yaml
services:
  api:
    ports:
      - "8000:8000"
  caddy:
    profiles: ["noop"]
  backup:
    profiles: ["noop"]
```

- [ ] **Step 3: Replace `api.yourdomain.com`**

Grep for and replace with the real hostname chosen in Task 3.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/deploy-api.yml docker-compose.override.ci.yml
git commit -m "ci: deploy-api workflow (test → build → e2e → ssh-deploy)"
```

- [ ] **Step 5: Push and confirm the workflow runs green**

Push the branch, open a PR. The `fast-tests`, `build`, and `e2e` jobs must pass. Deploy is skipped on PRs.

After merging to main, the `deploy` job runs and the post-deploy healthcheck passes.

---

## Task 11: `tools/export_null_agent.py`

**Files:**
- Create: `tools/export_null_agent.py`

- [ ] **Step 1: Create the script**

```python
"""Export a freshly-initialized (untrained) MaskablePPO to ONNX.

Used to publish the `null-agent-v0` placeholder for the F&F alpha.
Masked action selection over random logits produces legal random play —
a sufficient placeholder opponent until real models are trained.

Usage:
    python tools/export_null_agent.py --output models/null_agent.onnx
"""
from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
from sb3_contrib import MaskablePPO
from stable_baselines3.common.vec_env import DummyVecEnv

from digimon_gym.digimon_gym import DigimonEnv
from tools.export_onnx import export_mlp  # reuse the MLP export path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    # Build the env to get observation/action spaces right
    env = DummyVecEnv([lambda: DigimonEnv()])

    # Instantiate an MLP MaskablePPO with default policy_kwargs — no training
    model = MaskablePPO("MlpPolicy", env, verbose=0)

    # Save the zip so export_mlp can consume it
    zip_path = args.output.with_suffix(".zip")
    model.save(str(zip_path))

    # Reuse the existing export path
    export_mlp(str(zip_path), str(args.output))

    # Clean up the intermediate zip
    zip_path.unlink()
    print(f"null-agent ONNX written to {args.output}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run it**

```bash
python tools/export_null_agent.py --output models/null_agent.onnx
```
Expected: prints `null-agent ONNX written to models/null_agent.onnx`; the file is ~1–5 MB.

- [ ] **Step 3: Sanity-check the ONNX file loads**

```bash
python -c "
import onnxruntime as ort
import numpy as np
sess = ort.InferenceSession('models/null_agent.onnx')
out = sess.run(None, {'obs': np.zeros((1, 1375), dtype=np.float32)})
print('output shape:', out[0].shape)
"
```
Expected: `output shape: (1, 2168)`.

- [ ] **Step 4: Commit**

```bash
git add tools/export_null_agent.py
git commit -m "tools: export untrained MaskablePPO as null-agent placeholder"
```

---

## Task 12: Publish `null-agent-v0` via admin flow

**Files:** none new. This is a manual operational task — developer runs commands and the model appears in `/models/manifest.json`.

- [ ] **Step 1: Export fresh null-agent**

```bash
python tools/export_null_agent.py --output models/null_agent.onnx
```

- [ ] **Step 2: Publish via admin endpoints**

Detailed flow is in `digimon_gym/db/routers/admin_models.py` (POST `/admin/models` → PUT to pre-signed URL → confirm → publish). Quick recipe:

```bash
# Log in as admin to get a token
TOKEN=$(curl -sf -X POST https://api.yourdomain.com/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"<admin-user>","password":"<admin-pass>"}' | jq -r .access_token)

# Register + upload + confirm + publish
# (see admin_models.py for exact payload shapes — model_type="mlp",
#  tensor_size=1375, action_space_size=2168, name="null-agent-v0",
#  notes="Untrained baseline; random legal actions.")
```

Alternatively: use the admin UI if available.

- [ ] **Step 3: Verify manifest**

```bash
curl -sf https://api.yourdomain.com/models/manifest.json | jq .
```
Expected: an entry with `name: "null-agent-v0"` and a `url` pointing at Spaces (or the CDN URL).

- [ ] **Step 4: Commit nothing; note in alpha readiness checklist**

No code change. Mark the checklist item in `docs/ALPHA_READINESS.md` (Task 20) once the manifest entry exists.

---

## Task 13: Playwright journey test — `guest-onboarding.spec.ts`

**Files:**
- Create: `frontend/e2e/guest-onboarding.spec.ts` (via spawned tester chip)

This task is **authored by a spawned tester chip**, not by the implementing subagent. The executing agent calls `mcp__ccd_session__spawn_task` with the prompt below; the user clicks to spawn; the spawned agent works in its own worktree and opens a PR back to this branch.

- [ ] **Step 1: Spawn the tester chip**

Call `mcp__ccd_session__spawn_task` with:

- `title`: "Author guest-onboarding Playwright test"
- `tldr`: "A fresh agent writes the anonymous-guest e2e test for the alpha-readiness branch and PRs it back."
- `prompt`:

```
You are writing a Playwright e2e test for the digimon-deck-list-builder-1 frontend.

You have no prior conversation context. Start by reading:
- frontend/playwright.config.ts — how tests run, baseURL, auth fixtures
- frontend/e2e/fixtures/ — existing test fixtures (auth, debug-game)
- frontend/e2e/game-loads.spec.ts — existing test shape to follow
- frontend/src/bootstrap/guest.ts — how anonymous-guest login works
- frontend/src/pages/ — routes touched by the guest flow

Your task: create `frontend/e2e/guest-onboarding.spec.ts` that exercises the
anonymous-guest entry path end-to-end:

1. Navigate to the app's root.
2. Click "Continue as guest" (or whatever the UI affordance is — read the code to confirm).
3. Assert the user lands on the home page (some deterministic element visible).
4. Navigate to the deck builder. Assert the deck builder loads.

Pre-reqs for running tests:
- `cd frontend && npm ci && npx playwright install chromium`
- Dev server: `cd frontend && npm run dev` in a background process.
- The API must be running — use docker-compose as described in ops/deploy/README.md
  or docs/superpowers/specs/2026-04-19-alpha-readiness-design.md.

You OWN this feature post-handoff: if the test reveals the guest flow is broken,
fix the frontend code too. Do NOT mock around broken UI.

When green, open a PR back to the current feature branch (check `git branch` —
you're in a worktree branched from it). Title: "test: guest-onboarding e2e".

The alpha-readiness design spec is at
docs/superpowers/specs/2026-04-19-alpha-readiness-design.md — read section A
("Playwright test coverage") and section B ("Playwright test authoring process").
```

- [ ] **Step 2: Wait for tester PR**

The spawned agent works independently. Check back periodically — when a PR titled `test: guest-onboarding e2e` appears, proceed.

- [ ] **Step 3: Review and merge the tester PR**

```bash
gh pr view <PR#> --web   # review
gh pr checks <PR#>       # confirm CI green
gh pr merge <PR#> --merge
```

- [ ] **Step 4: Pull the merged changes into this worktree**

```bash
git pull origin <feature-branch>
```

---

## Task 14: Playwright journey test — `room-code-pvp.spec.ts`

**Files:**
- Create: `frontend/e2e/room-code-pvp.spec.ts` (via spawned tester chip)

Same pattern as Task 13 — spawn a tester chip.

- [ ] **Step 1: Spawn the tester chip**

Call `mcp__ccd_session__spawn_task` with:

- `title`: "Author room-code-pvp Playwright test"
- `tldr`: "A fresh agent writes the two-browser room-code PvP e2e test for the alpha-readiness branch."
- `prompt`:

```
You are writing a Playwright e2e test for the digimon-deck-list-builder-1 frontend.

You have no prior conversation context. Start by reading:
- frontend/playwright.config.ts
- frontend/e2e/fixtures/ and frontend/e2e/page-objects/
- frontend/e2e/game-loads.spec.ts (to copy the shape)
- digimon_gym/routers/lobby.py — server endpoints: POST /lobby/create, GET /lobby/public, POST /lobby/join/{code}
- frontend/src/pages/LobbyPage.tsx and frontend/src/pages/GamePage.tsx
- frontend/src/api/client.ts — how the frontend calls the lobby endpoints

Your task: create `frontend/e2e/room-code-pvp.spec.ts` exercising the
two-player room-code flow inside one test using TWO browser contexts:

1. Context A (host): anonymous guest → create a lobby with a deck → capture the join code.
2. Context B (guest): anonymous guest → join the lobby using the code.
3. Both contexts: assert the game board is visible.
4. Both contexts: assert each can see the other's state (without peeking at private info — check public fields only).
5. One context submits a legal action (e.g., pass turn or hatch). Both contexts observe the turn resolve.
6. (Optional but recommended) end the test by both contexts closing gracefully.

Use `browser.newContext()` twice inside a single test block. Keep both
contexts open concurrently. Auth each context as a distinct anonymous guest.

You OWN this feature post-handoff: if the test reveals the room-code flow is
broken (missing field, racy WebSocket, etc.), fix the frontend code. Do NOT
mock around broken UI.

When green, open a PR back to the current feature branch. Title: "test: room-code-pvp e2e".

Spec: docs/superpowers/specs/2026-04-19-alpha-readiness-design.md, sections A and B.
```

- [ ] **Step 2: Wait for tester PR, review, merge** (same as Task 13 Step 3)

- [ ] **Step 3: Pull merged changes into this worktree** (same as Task 13 Step 4)

---

## Task 15: Playwright journey test — `try-online-vs-ai.spec.ts`

**Files:**
- Create: `frontend/e2e/try-online-vs-ai.spec.ts` (via spawned tester chip)

- [ ] **Step 1: Spawn the tester chip**

Call `mcp__ccd_session__spawn_task` with:

- `title`: "Author try-online-vs-ai Playwright test"
- `tldr`: "A fresh agent writes the Try-Online-vs-AI e2e test for the alpha-readiness branch."
- `prompt`:

```
You are writing a Playwright e2e test for the digimon-deck-list-builder-1 frontend.

You have no prior conversation context. Start by reading:
- frontend/playwright.config.ts, frontend/e2e/fixtures/
- frontend/e2e/game-loads.spec.ts for shape
- Grep frontend/src for "Try Online vs AI" or similar — the UI surface added in commit 071c7017
- digimon_gym/routers/games.py — server-side AI-vs-player game creation
- The null-agent model must be published to manifest.json for this test to work;
  if it's not, the test should skip with a helpful message.

Your task: create `frontend/e2e/try-online-vs-ai.spec.ts`:

1. Anonymous guest.
2. Click "Try Online vs AI" (or the exact UI affordance — read the code).
3. Assert a game starts against an AI opponent. Game board loads.
4. Submit at least one legal action.
5. Either play to natural end OR concede — assert the game ends and the user
   returns to home.

You OWN this feature post-handoff: if the AI-vs-player flow is broken, fix
the frontend code. If the server rejects the model_id — that's a separate
problem — surface a skip, don't mock the response.

When green, open a PR back to the current feature branch. Title: "test: try-online-vs-ai e2e".

Spec: docs/superpowers/specs/2026-04-19-alpha-readiness-design.md, sections A and B.
```

- [ ] **Step 2: Wait for tester PR, review, merge**
- [ ] **Step 3: Pull merged changes into this worktree**

---

## Task 16: Rust integration test — `it_model_download.rs`

**Files:**
- Create: `src-tauri/tests/it_model_download.rs`

- [ ] **Step 1: Read the existing model-download code**

```bash
cat src-tauri/src/models.rs
```
Understand: manifest fetch, download to `data_dir()`, SHA-256 verification, cache hit on second call.

- [ ] **Step 2: Write the failing test**

```rust
// src-tauri/tests/it_model_download.rs

use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_download_verify_and_cache() {
    // 1. Stand up a local HTTP server serving a small fake ONNX + a manifest pointing at it.
    //    Use wiremock or a hand-rolled hyper server.
    // 2. Point the models module at the local URL + a tempdir cache root.
    // 3. Call download_model("null-agent-v0").
    // 4. Assert the file exists, SHA matches manifest.
    // 5. Call download_model again; assert no network hit (check the mock call count).

    let cache_dir = tempdir().unwrap();
    // ... construct manifest with a known SHA for test fixture bytes ...
    // ... hit the cache-miss path, then the cache-hit path ...

    unimplemented!("flesh out with actual fixtures; fail first");
}
```

- [ ] **Step 3: Run and confirm it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test it_model_download
```
Expected: `unimplemented!` panic.

- [ ] **Step 4: Add a helper that makes the test possible**

If `src-tauri/src/models.rs` doesn't expose a way to inject a custom manifest URL and cache root, add one. Minimum change: a `#[cfg(test)] fn download_model_for_test(url, cache_dir, name)` wrapper. Prefer refactoring the real code to accept a struct (ModelCache) that holds both, so test and prod go through the same path.

- [ ] **Step 5: Flesh out the test with a real HTTP fixture**

Use `wiremock = "0.6"` (add to `[dev-dependencies]`):

```rust
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_download_verify_and_cache() {
    let fake_onnx: Vec<u8> = b"not a real onnx, just bytes".to_vec();
    let sha = sha256_hex(&fake_onnx);

    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/null-agent-v0.onnx"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fake_onnx.clone()))
        .expect(1)  // exactly one network hit
        .mount(&mock).await;

    let manifest = json!({
        "models": [{
            "id": "null-agent-v0",
            "url": format!("{}/null-agent-v0.onnx", mock.uri()),
            "sha256": sha,
            "size_bytes": fake_onnx.len(),
        }]
    });
    // ... mount manifest endpoint, point ModelCache at mock.uri() + tempdir, download twice, assert ...
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
```

- [ ] **Step 6: Run and confirm it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test it_model_download
```
Expected: PASS, one test.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/tests/it_model_download.rs src-tauri/src/models.rs src-tauri/Cargo.toml
git commit -m "test(tauri): manifest fetch, SHA verify, cache hit"
```

---

## Task 17: Rust integration test — `it_offline_game.rs`

**Files:**
- Create: `src-tauri/tests/it_offline_game.rs`

- [ ] **Step 1: Read offline-gameplay code**

```bash
cat src-tauri/src/engine_commands.rs src-tauri/src/inference_state.rs
```

- [ ] **Step 2: Write the failing test**

```rust
// src-tauri/tests/it_offline_game.rs

#[tokio::test]
async fn test_offline_game_vs_null_agent_completes_legally() {
    // 1. Create a game via the same invoke path as engine_commands.rs
    //    (HeadlessRunner or similar — use the library API, don't invoke Tauri IPC).
    // 2. Attach a null-agent ONNX session (load from tests/fixtures/null_agent.onnx
    //    — tiny hand-crafted ONNX that outputs zeros for any input).
    // 3. Loop: while !game.is_over() { submit legal action (masked argmax over random logits) }
    // 4. Assert game ends with a winner (not a panic, not an illegal action).

    unimplemented!("needs null_agent.onnx fixture and a driver loop");
}
```

- [ ] **Step 3: Generate a tiny ONNX fixture**

Add a build-time helper or commit a small hand-built ONNX that takes shape `(1, 1375)` → `(1, 2168)` and outputs zeros. Either:

- Commit `src-tauri/tests/fixtures/null_agent.onnx` (bytes committed to repo — only a few KB).
- Or generate it in a `build.rs`-style helper using `tract-onnx` to build the graph. Simpler: commit the fixture.

A 3-line Python helper produces the fixture once:

```python
# one-off, NOT checked in as a tool
import torch, torch.nn as nn, torch.onnx
m = nn.Sequential(nn.Linear(1375, 2168))
for p in m.parameters(): p.data.zero_()
torch.onnx.export(m, torch.zeros(1, 1375), "null_agent.onnx",
                  input_names=["obs"], output_names=["logits"],
                  dynamic_axes={"obs": {0: "b"}, "logits": {0: "b"}})
```

- [ ] **Step 4: Flesh out the test, run, iterate to green**

Use `onnxruntime` crate via the existing inference path. Mask illegal actions via the game's `action_mask`.

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test it_offline_game
```
Expected: PASS.

- [ ] **Step 5: Wire into CI**

Add a Rust-test job to `.github/workflows/deploy-api.yml`:

```yaml
  tauri-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { submodules: recursive }
      - uses: dtolnay/rust-toolchain@stable
      - uses: swatinem/rust-cache@v2
        with: { workspaces: './src-tauri -> target' }
      - run: sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
      - run: cargo test --manifest-path src-tauri/Cargo.toml
```

Add `tauri-tests` as a dependency of the `deploy` job's `needs:` list, so deploy waits on it.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tests/ .github/workflows/deploy-api.yml
git commit -m "test(tauri): offline game vs null-agent completes legally"
```

---

## Task 18: Bake `VITE_API_URL` in desktop-release.yml

**Files:**
- Modify: `.github/workflows/desktop-release.yml`

- [ ] **Step 1: Read the current workflow**

```bash
cat .github/workflows/desktop-release.yml
```
Locate the `Build frontend` step (likely `npm run build`).

- [ ] **Step 2: Add `VITE_API_URL` env before the build step**

Add after the `Install frontend dependencies` step, before or inline with the build:

```yaml
      - name: Build frontend with prod API URL
        env:
          VITE_API_URL: https://api.yourdomain.com
        working-directory: frontend
        run: npm run build
```

If the build already happens inside the Tauri action (`tauri-apps/tauri-action@v0`), pass env at the job level:

```yaml
    env:
      VITE_API_URL: https://api.yourdomain.com
```

- [ ] **Step 3: Replace `api.yourdomain.com` with the real hostname**

- [ ] **Step 4: Test by cutting a dry-run tag**

```bash
git tag v0.0.0-dryrun
git push origin v0.0.0-dryrun
```

Watch `desktop-release.yml` in Actions. Verify the workflow runs and the built `.msi` artifact, when installed, makes network calls to `https://api.yourdomain.com` (not localhost or `/api`).

If the dry-run works, delete the tag + release:

```bash
git push --delete origin v0.0.0-dryrun
git tag -d v0.0.0-dryrun
gh release delete v0.0.0-dryrun
```

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/desktop-release.yml
git commit -m "build(desktop): bake VITE_API_URL at build time"
```

---

## Task 19: `.github/RELEASE_NOTES.md` template

**Files:**
- Create: `.github/RELEASE_NOTES.md`

- [ ] **Step 1: Create the template**

```markdown
# Digimon TCG Simulator — {{VERSION}}

**Alpha build.** Bugs are expected. Please report them.

## What's new
- (edit per release)

## How to install

### Windows
Download `Digimon-TCG-Simulator_{{VERSION}}_x64_en-US.msi` below.

Windows SmartScreen will warn you: "Windows protected your PC." Click **More info → Run anyway**. This is expected — the binary is unsigned (we're not paying for a code-signing cert yet).

### Mac
Download `.dmg`. On first launch, right-click the app → **Open** (Gatekeeper blocks double-click for unsigned apps).

### Linux
Download `.AppImage`, `chmod +x`, run.

## How to play

- Launch the app. Click **Continue as Guest**.
- **Play a friend:** Create a room → share the 6-character code.
- **Play the AI:** Click **Try Online vs AI**. The AI is currently a random-legal-action baseline — it won't be hard. Real models are on the way.

## Known issues
- (edit per release)

## Reporting bugs
Open an issue at https://github.com/{{OWNER}}/{{REPO}}/issues with:
- OS + version
- What you were doing
- What happened vs what you expected
- Screenshot if visual
```

- [ ] **Step 2: Replace `{{OWNER}}/{{REPO}}`**

- [ ] **Step 3: Wire into desktop-release.yml**

If the workflow uses `tauri-apps/tauri-action`, it supports `releaseBody: ${{ github.event.repository.description }}`. Point it at this file instead:

```yaml
- name: Read release notes template
  id: notes
  run: |
    echo "body<<EOF" >> "$GITHUB_OUTPUT"
    sed "s/{{VERSION}}/${GITHUB_REF#refs/tags/}/g" .github/RELEASE_NOTES.md >> "$GITHUB_OUTPUT"
    echo "EOF" >> "$GITHUB_OUTPUT"
# ...
- uses: tauri-apps/tauri-action@v0
  with:
    releaseBody: ${{ steps.notes.outputs.body }}
```

Place the `Read release notes` step before the `tauri-action` step.

- [ ] **Step 4: Commit**

```bash
git add .github/RELEASE_NOTES.md .github/workflows/desktop-release.yml
git commit -m "docs: release notes template for GH Releases"
```

---

## Task 20: `docs/ALPHA_READINESS.md` — final checklist

**Files:**
- Create: `docs/ALPHA_READINESS.md`

- [ ] **Step 1: Create the checklist**

```markdown
# Alpha Readiness Checklist

Every item MUST be checked before inviting the first F&F tester.

## Server

- [ ] `Dockerfile` multi-stage builds clean locally: `docker build -t test .`
- [ ] `docker compose -f docker-compose.prod.yml -f docker-compose.override.local.yml up` brings up API + Postgres; `curl localhost:8000/health` returns 200
- [ ] DNS `A` record `api.yourdomain.com` → droplet IP; Caddy serves a valid cert (`curl -I https://api.yourdomain.com/health`)
- [ ] GHA `deploy-api.yml` runs green on last push to `main`; post-deploy healthcheck passes
- [ ] Droplet disk usage < 50% (`ssh deploy@host df -h /`)

## Database

- [ ] Nightly `pg_backup.sh` has run at least once and the `.sql.gz` is visible in `s3://<bucket>/backups/`
- [ ] Restore runbook exercised end-to-end (notes in `ops/deploy/README.md`)
- [ ] Spaces lifecycle rule: 14-day expiry on `backups/` prefix (confirm via DO web UI)

## Tests

- [ ] Playwright CI green: 4 existing engine tests + 3 new journey tests (`guest-onboarding`, `room-code-pvp`, `try-online-vs-ai`)
- [ ] Rust integration tests green: `it_model_download.rs`, `it_offline_game.rs`
- [ ] Fast-tests job green: `pytest -m "not slow"`

## Models

- [ ] `null-agent-v0` published via admin; visible in `https://api.yourdomain.com/models/manifest.json`
- [ ] `SPACES_CDN_URL` set in prod `.env`; manifest URL resolves via CDN
- [ ] No lifecycle rule on `models/` prefix (confirm via DO web UI)

## Desktop

- [ ] Tagged `v0.1.0-alpha`; `desktop-release.yml` produced `.msi` + `.dmg` + `.AppImage`
- [ ] `.msi` installed on a clean Windows VM connects to prod API (check network calls) and completes a PvP room-code game with yourself
- [ ] GH Release body renders the `RELEASE_NOTES.md` template

## Secrets hygiene

- [ ] Droplet `/opt/digimon/.env`: real `SPACES_*`, `JWT_SECRET`, `POSTGRES_PASSWORD`. No defaults from any example file.
- [ ] GH Actions secrets: `DROPLET_SSH_KEY`, `DROPLET_HOST`, `DROPLET_USER` present and unique to this project
- [ ] Deploy SSH keypair is dedicated (not a developer's personal key)

## Ops

- [ ] `ops/deploy/README.md` complete: bootstrap, rollback, restore. Updated with any bootstrap-time deviations.
- [ ] Last rollback drill (manual `docker compose` edit + `up -d`) exercised at least once

---

When all boxes ticked: tag `v0.1.0-alpha`, push tag, share GH Release link with testers.
```

- [ ] **Step 2: Commit**

```bash
git add docs/ALPHA_READINESS.md
git commit -m "docs: alpha readiness go/no-go checklist"
```

---

## Task 21: Final end-to-end verification

**Files:** none new. This is the go/no-go gate.

- [ ] **Step 1: Walk the checklist top to bottom**

Open `docs/ALPHA_READINESS.md`. For each item, run the verification command or manual check. Tick boxes in the file as you go (commit the ticked file at the end).

- [ ] **Step 2: Install the `.msi` on a clean Windows VM**

- Spin up a fresh Windows VM (Hyper-V, VirtualBox, or a cloud Windows instance).
- Download the `.msi` from the `v0.1.0-alpha` GH Release.
- Install, click through SmartScreen, launch.
- Click **Continue as Guest**.
- Create a lobby; note the code.
- In a separate context (second VM or the developer's machine via desktop app), join via the code.
- Play one turn to completion.
- Close and reopen the app; verify the AI opponent works (network call succeeds, model downloads if it's a first launch).

- [ ] **Step 3: Tick the final checklist item, commit**

```bash
git add docs/ALPHA_READINESS.md
git commit -m "docs: alpha readiness — all boxes ticked"
```

- [ ] **Step 4: Ship**

Share the GH Release URL with F&F testers. Monitor droplet CPU/disk for the first week (`ssh deploy@host htop` and `df -h`).

---

## Notes

- **Order matters.** Tasks 1–10 set up the server; 11–12 publish the model (must happen before task 15's try-online-vs-ai test can run end-to-end); 13–15 author Playwright via spawned chips (can run in parallel with each other); 16–17 do the Tauri-side Rust tests; 18–19 ship the desktop binary; 20–21 close out.
- **Parallelizable:** Tasks 13, 14, 15 each spawn an independent tester chip and can all be in flight simultaneously. Tasks 16–17 are independent of the Playwright work.
- **Rollback at any point:** revert the feature branch; no prod impact until Task 10 Step 5 lands the workflow on main.

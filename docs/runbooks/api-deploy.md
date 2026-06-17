# Deploying the hosted API

> **Procedural runbook** (bootstrap / deploy / rollback / restore). For the
> hosted-API topology, required env vars, and provider choices, see
> [`docs/DEPLOYMENT.md`](../DEPLOYMENT.md). The happy-path deploy recipe is the
> `deploy-hosted-api` skill.

Prod host: `inbetweentheatre.duckdns.org` → DO droplet in NYC3.
Image registry: `ghcr.io/mammalwithashell/digimon-api`.

## One-time droplet bootstrap

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

# 5. Point DuckDNS at the droplet
# Visit https://www.duckdns.org, log in, set inbetweentheatre.duckdns.org
# to the droplet IP. Verify: `dig inbetweentheatre.duckdns.org +short`.
```

SSH in once as root to finish setup:

```bash
ssh -i ~/.ssh/digimon_deploy root@<droplet-ip>

# On the droplet — install Docker Engine + compose plugin
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
scp -i ~/.ssh/digimon_deploy -r scripts/ \
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
TRAINING_WORKER_DISABLED=1
AI_WORKER_DISABLED=1
```

`TRAINING_WORKER_DISABLED=1` and `AI_WORKER_DISABLED=1` are required because
the server image drops torch and the heavy AI-pipeline wheels. The `/admin/training/*`
and `/admin/ai/*` endpoints will 500 if hit; F&F testers never touch them.

First boot:

```bash
cd /opt/digimon
docker login ghcr.io -u mammalwithashell -p <a GHCR read token>
docker compose -f docker-compose.prod.yml pull
docker compose -f docker-compose.prod.yml up -d
docker compose logs -f api      # watch migrations finish
curl -sf https://inbetweentheatre.duckdns.org/health
```

## Rollback

Every deploy tags the image with the git SHA. To roll back to a known-good SHA:

```bash
ssh deploy@<droplet-ip>
cd /opt/digimon
export API_IMAGE=ghcr.io/mammalwithashell/digimon-api:<known-good-sha>
docker compose -f docker-compose.prod.yml pull api
docker compose -f docker-compose.prod.yml up -d api
# Or: edit docker-compose.prod.yml to pin the image tag, commit, redeploy.
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
curl -sf https://inbetweentheatre.duckdns.org/health
```

This runbook MUST be exercised end-to-end once before alpha opens. See Task 9 in
`docs/superpowers/plans/2026-04-19-alpha-readiness.md`.

## Spaces lifecycle rule (one-time)

Set a 14-day auto-delete rule on the `backups/` prefix of the Spaces bucket via
the DO web UI (Spaces → Bucket → Settings → Lifecycle rules). Models under
`models/` are NOT subject to any rule.

## Tooling used for bootstrap + deploy

All bootstrap + deploy steps in this repo use CLIs (no web UIs except DuckDNS +
GitHub package-visibility settings). Future developers replaying this from
scratch need these installed and authenticated before running anything above.

| Tool | Purpose | Auth state |
|------|---------|------------|
| `doctl` | DigitalOcean droplet + SSH key provisioning. | `doctl auth init` once with a DO API token (Account → API → Tokens). |
| `gh` | Repo secret setting, PR creation, status checks. | `gh auth login` once. |
| `ssh` + `scp` | Droplet access via dedicated `digimon_deploy` keypair (not the developer's personal key). | Key at `~/.ssh/digimon_deploy`; public half registered with DO via `doctl compute ssh-key import`. |
| `docker` + `docker compose` | Multi-stage Dockerfile build; prod compose stack on droplet. | Docker Desktop locally; daemon + compose plugin installed on droplet. |
| `openssl` | Generating `POSTGRES_PASSWORD` + `JWT_SECRET` on the droplet directly (kept out of local shell history). | n/a |
| Caddy 2 | Automatic Let's Encrypt TLS for `inbetweentheatre.duckdns.org`. Runs as a compose service, no config beyond `Caddyfile`. | LE cert acquired automatically on first HTTPS request. |
| DuckDNS | Free dynamic DNS (`*.duckdns.org`) — used instead of paying for a domain. | Sign in with GitHub at duckdns.org, set the subdomain's IP to the droplet's public IP. |
| DO Spaces | S3-compatible object storage: `models/` prefix (no expiry) for RL models, `backups/` prefix (14-day lifecycle rule) for Postgres dumps. | `SPACES_KEY` + `SPACES_SECRET` via DO → API → Spaces Keys → Generate New Key. |
| GHCR | Container registry for the API image, pushed by GHA on every merge. | `${{ secrets.GITHUB_TOKEN }}` in workflow writes; package made public after first build so droplet pulls without auth. |

## Development patterns established this session

- **Torch split.** `requirements-server.txt` excludes torch + SB3 for the API image. Admin training endpoints (`/admin/training/*`, `/admin/ai/*`, `/deck-optimizer/*`) will 500 if called; gated off via `TRAINING_WORKER_DISABLED=1` + `AI_WORKER_DISABLED=1`. If admin features are needed, run a dev server from the full `requirements.txt`.
- **LF line endings.** `.gitattributes` forces LF on `*.sh`, `*.yml`, `Dockerfile`, `Caddyfile` so files authored on Windows still run on Linux runners / droplet.
- **Dedicated deploy keypair.** Don't reuse the developer's personal SSH key. GHA stores the private half; if logs leak, rotate just that key.
- **Rust integration tests use `wiremock`.** For testing HTTP clients (model manifest fetch, Spaces download), mount a local `wiremock::MockServer` and assert exact call counts via `.expect(n)`. No manual port picking — wiremock auto-assigns.
- **`#[cfg(test)]` hacks discouraged.** If Rust code needs a test seam, prefer a small refactor (struct accepts injected config) over sprinkling test-only code paths in the real module. Done in Task 16 for `ModelsManager`.
- **Tauri testing requires a `frontend/dist/index.html` stub** so `tauri::generate_context!()` compiles under `cargo test`. The stub is committed (1 line); `npm run build` overwrites it harmlessly.
- **Playwright test authorship was delegated** to fresh-context agents via `mcp__ccd_session__spawn_task`. The agent that implemented a feature does NOT write its own Playwright test — a different agent reads the code cold and authors the test. Scoped to the three alpha-readiness journey tests only; not a general project policy.

## Bootstrap notes (2026-04-19)

Record of what actually happened during the initial droplet bootstrap. Update
this section on every re-bootstrap (disaster recovery, droplet migration, etc.).

- **Droplet:** `digimon-api` (DO ID `565912379`) — 2 GB / 1 vCPU / 50 GB SSD, Ubuntu 24.04, NYC3. Public IP `64.225.21.6`.
- **SSH key:** `digimon-deploy` (DO ID `55736461`, fingerprint `84:c5:62:60:3f:29:bb:ca:5d:21:3e:e5:f2:e5:fe:4a`). Private half at `~/.ssh/digimon_deploy`; pushed to GH repo secret `DROPLET_SSH_KEY`.
- **DNS:** `inbetweentheatre.duckdns.org` A-record manually pointed at `64.225.21.6` via duckdns.org.
- **Docker versions on droplet:** Docker Engine 29.4.0, Docker Compose v5.1.3. Installed via Docker's official apt repo (https://download.docker.com/linux/ubuntu).
- **Spaces bucket:** `digimon-tcg-models` in NYC3 — created fresh during this bootstrap. CDN URL `https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com`. Lifecycle rule on `backups/` (14-day expiry) **still to be set via DO web UI** as of 2026-04-19.
- **Secrets rotation:** `POSTGRES_PASSWORD` + `JWT_SECRET` generated on the droplet via `openssl rand -hex`. `SPACES_KEY` / `SPACES_SECRET` provided by user during bootstrap; if these leak, rotate via DO web UI and update `/opt/digimon/.env`.
- **GHA secrets set:** `DROPLET_SSH_KEY`, `DROPLET_HOST=64.225.21.6`, `DROPLET_USER=deploy`.
- **Skipped:** Local `docker build` validation of the multi-stage Dockerfile (Docker Desktop not running during this session — per user "1b" decision, relying on CI's build job to validate).
- **Deferred:** First `docker compose pull` on droplet — awaiting PR #339's build job to push the first image to GHCR. GHCR package visibility must be flipped to Public (GH → packages → `digimon-api` → settings) before the droplet can pull without auth.

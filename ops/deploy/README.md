# Deploying the hosted API

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

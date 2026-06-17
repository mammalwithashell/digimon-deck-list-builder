# Consolidate `ops/` into `docs/` + add training / API-deploy skills — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dissolve the top-level `ops/` tree into `docs/` (prose) + `scripts/` and the repo root (live artifacts), and add two recipe skills — `start-training-run` and `deploy-hosted-api` — following the `cut-desktop-release` pattern.

**Architecture:** Mechanical reorg with verification gates instead of unit tests (grep for stale paths, `docker compose config` resolves, skill files load). Live references are repointed in the same task as each move so the repo never sits in a broken state. Historical/archived records are left untouched. Skills are thin recipes pointing at authoritative runbooks under `docs/`.

**Tech Stack:** git, Docker Compose, GitHub Actions (`gh`), Markdown skills under `.claude/skills/`.

**Spec:** [`docs/superpowers/specs/2026-06-16-ops-docs-skills-consolidation-design.md`](2026-06-16-ops-docs-skills-consolidation-design.md)

**Working directory:** this worktree (`.claude/worktrees/trusting-dijkstra-61cdc0`). Run all commands from the worktree root. Confirm before starting: `git rev-parse --show-toplevel` must end in `.claude/worktrees/trusting-dijkstra-61cdc0`.

---

### Task 1: Relocate live runtime artifacts + repoint every live reference

Moves the two live artifacts and fixes every non-historical path that points at them, in one atomic commit.

**Files:**
- Move: `ops/backup/pg_backup.sh` → `scripts/pg_backup.sh`
- Move: `ops/training/docker-compose.watch.yml` → `docker-compose.watch.yml` (repo root)
- Modify: `docker-compose.prod.yml:53`
- Modify: `docker-compose.override.local.yml:7`
- Modify: `CLAUDE.md` (lines 93, 255, 347)
- Modify: `docs/CLOUD_TRAINING.md` (lines ~509, ~519, ~525)

- [ ] **Step 1: Move the two artifacts with history preserved**

```bash
git mv ops/backup/pg_backup.sh scripts/pg_backup.sh
git mv ops/training/docker-compose.watch.yml docker-compose.watch.yml
```

- [ ] **Step 2: Repoint the backup mount in the prod compose**

In `docker-compose.prod.yml`, replace the `backup` service's volume line.

Old:
```yaml
      - ./ops/backup/pg_backup.sh:/usr/local/bin/pg_backup.sh:ro
```
New:
```yaml
      - ./scripts/pg_backup.sh:/usr/local/bin/pg_backup.sh:ro
```

- [ ] **Step 3: Repoint the local-override comment**

In `docker-compose.override.local.yml`, the comment on line ~7.

Old:
```yaml
# `.env.local` is gitignored — see ops/deploy/README.md for the template.
```
New:
```yaml
# `.env.local` is gitignored — see docs/runbooks/api-deploy.md for the template.
```

(`api-deploy.md` is created in Task 2; this comment is the forward reference.)

- [ ] **Step 4: Repoint the three CLAUDE.md mentions**

In `CLAUDE.md`:

Line ~93 (Project Layout tree) — old:
```
├── ops/, scripts/                 # Deploy + operational scripts
```
new:
```
├── scripts/                       # Deploy + operational scripts
```

Line ~255 (Commands → Cloud training) — old:
```
docker compose -f ops/training/docker-compose.watch.yml up -d   # TensorBoard sidecar over ./runs
```
new:
```
docker compose -f docker-compose.watch.yml up -d   # TensorBoard sidecar over ./runs
```

Line ~347 (Documentation → Cloud training bullet) — replace the substring `the \`ops/training/docker-compose.watch.yml\` TensorBoard sidecar` with `the \`docker-compose.watch.yml\` TensorBoard sidecar`.

- [ ] **Step 5: Repoint the CLOUD_TRAINING.md cloud-host commands**

In `docs/CLOUD_TRAINING.md`, section B.3/B.4. The watcher compose now lives at the workspace root, so its relative `./runs` mount resolves against `~/digimon-training/` (where runs actually live).

Line ~509 — old:
```bash
mkdir -p ~/digimon-training/{runs,models,data,training_jobs,ops/training}
```
new:
```bash
mkdir -p ~/digimon-training/{runs,models,data,training_jobs}
```

Line ~519 — old:
```bash
rsync -az ops/training/ digimon-train:~/digimon-training/ops/training/
```
new:
```bash
rsync -az docker-compose.watch.yml digimon-train:~/digimon-training/
```

Line ~525 — old:
```bash
docker compose -f ops/training/docker-compose.watch.yml up -d
```
new:
```bash
docker compose -f docker-compose.watch.yml up -d
```

- [ ] **Step 6: Verify the compose files still resolve**

Run:
```bash
docker compose -f docker-compose.watch.yml config >/dev/null && echo "watch OK"
docker compose -f docker-compose.prod.yml config 2>/dev/null | grep -q 'scripts/pg_backup.sh' && echo "prod backup path OK"
```
Expected: `watch OK` and `prod backup path OK`. (If `docker` is unavailable in the environment, instead confirm by reading the two files that no `ops/` path remains in either.)

- [ ] **Step 7: Verify no live reference to the moved artifacts remains**

The `ops/` READMEs are still present (moved in Tasks 2–3) and reference the old
paths, so exclude `ops/` itself here — this gate checks only references *outside*
`ops/`.

Run:
```bash
git grep -n "ops/backup\|ops/training/docker-compose" -- ':!docs/superpowers' ':!openspec' ':!ops'
```
Expected: no output.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "Relocate ops/ live artifacts (pg_backup.sh -> scripts/, watch compose -> root) + repoint refs

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: Relocate the hosted-API runbook into `docs/runbooks/`

Move the procedural deploy runbook and cross-link it with the conceptual `DEPLOYMENT.md`.

**Files:**
- Move: `ops/deploy/README.md` → `docs/runbooks/api-deploy.md`
- Modify: `docs/runbooks/api-deploy.md` (add cross-link header)
- Modify: `docs/DEPLOYMENT.md` (add pointer to the runbook)

- [ ] **Step 1: Move the runbook with history preserved**

```bash
git mv ops/deploy/README.md docs/runbooks/api-deploy.md
```

- [ ] **Step 1b: Fix the stale `ops/` path inside the runbook**

The bootstrap section copies deploy files to the droplet. It used to `scp` the
whole `ops/` dir because the prod compose mounted `./ops/backup/pg_backup.sh`;
now that mount is `./scripts/pg_backup.sh`, so the droplet needs `scripts/`
instead. In `docs/runbooks/api-deploy.md`:

Old:
```bash
scp -i ~/.ssh/digimon_deploy -r ops/ \
  deploy@<droplet-ip>:/opt/digimon/
```
New:
```bash
scp -i ~/.ssh/digimon_deploy -r scripts/ \
  deploy@<droplet-ip>:/opt/digimon/
```

This is the only `ops/` reference inside the runbook (verify with
`git grep -n "ops/" docs/runbooks/api-deploy.md` → no output after the edit).

- [ ] **Step 2: Add a cross-link header to the runbook**

In `docs/runbooks/api-deploy.md`, the file currently starts with:
```markdown
# Deploying the hosted API
```
Replace that line with:
```markdown
# Deploying the hosted API

> **Procedural runbook** (bootstrap / deploy / rollback / restore). For the
> hosted-API topology, required env vars, and provider choices, see
> [`docs/DEPLOYMENT.md`](../DEPLOYMENT.md). The happy-path deploy recipe is the
> `deploy-hosted-api` skill.
```

- [ ] **Step 3: Add a pointer from DEPLOYMENT.md**

In `docs/DEPLOYMENT.md`, after the opening paragraph that ends `Desktop sidecar packaging is separate (\`docs/TOOLS.md\`).`, add a new line:
```markdown

> **How to actually run a deploy / rollback / restore:** see the procedural
> runbook [`docs/runbooks/api-deploy.md`](runbooks/api-deploy.md) (or the
> `deploy-hosted-api` skill for the happy path).
```

- [ ] **Step 4: Verify**

Run:
```bash
test -f docs/runbooks/api-deploy.md && echo "runbook present"
git grep -n "api-deploy.md" docs/DEPLOYMENT.md docker-compose.override.local.yml
```
Expected: `runbook present`, and the grep shows the two pointers (DEPLOYMENT.md + the override comment from Task 1).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "Move hosted-API deploy runbook ops/deploy/README.md -> docs/runbooks/api-deploy.md

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Fold the training-watcher README into CLOUD_TRAINING.md and delete `ops/`

**Files:**
- Modify: `docs/CLOUD_TRAINING.md` (append a watcher subsection)
- Delete: `ops/training/README.md`
- Delete: `ops/` (now empty)

- [ ] **Step 1: Append the watcher infra subsection to CLOUD_TRAINING.md**

At the end of `docs/CLOUD_TRAINING.md`, append:

```markdown

## TensorBoard watcher sidecar

The watcher is the **observation** layer for a cloud training host. The trainer
container is a one-shot `docker run` (it exits loudly on completion — we do *not*
want a 13-hour job silently restart-looping on a transient failure). The watcher
is the opposite shape: long-lived and declarative, so it is a Compose service.

`docker-compose.watch.yml` (repo root) defines a single `tensorboard` service:
upstream `tensorflow/tensorflow:latest`, `tensorboard --logdir /runs --bind_all
--port 6006`, mounting `./runs:/runs:ro` (read-only — the watcher cannot corrupt
trainer output) with `restart: unless-stopped`. Bring it up once per host at
provisioning time (`docker compose -f docker-compose.watch.yml up -d`); it
survives trainer restarts.

**Reach:** port 6006 must only be reachable over Tailscale. The cloud-provider
firewall (Hetzner / DO Cloud Firewall) blocks inbound `:6006` from the public
internet; the WireGuard tunnel is the only legitimate path.
```

- [ ] **Step 2: Delete the README and the now-empty ops/ tree**

```bash
git rm ops/training/README.md
# ops/deploy/README.md and the two live artifacts already moved in Tasks 1-2,
# so ops/ should now be empty of tracked files.
git status --porcelain ops/ || true
rmdir ops/training ops/backup ops/deploy ops 2>/dev/null || true
```

- [ ] **Step 3: Final gate — no live `ops/` reference anywhere**

Run the specific gate (catches every real reference — all `ops/` content lived
under these three subtrees):
```bash
git grep -nE "ops/(deploy|training|backup)" -- ':!docs/superpowers' ':!openspec'
```
Expected: no output.

Then a broad sweep to eyeball anything missed:
```bash
git grep -n "ops/" -- ':!docs/superpowers' ':!openspec'
```
Expected: no infra-path hits. Any remaining match must be an incidental substring
(e.g. a word like `develops/`), not a path into the old `ops/` tree. Confined
real mentions are only in the design spec + this plan under `docs/superpowers/`
and historical `openspec/changes/`, both intentionally left as-is.

Also confirm the directory is gone:
```bash
test ! -d ops && echo "ops/ removed"
```
Expected: `ops/ removed`.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "Fold training-watcher README into CLOUD_TRAINING.md; remove ops/ tree

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: Author the `start-training-run` skill

Cloud-first recipe pointing at the training runbooks. Author/validate with the `superpowers:writing-skills` skill, but the full content is below.

**Files:**
- Create: `.claude/skills/start-training-run/SKILL.md`

- [ ] **Step 1: Write the skill file**

Create `.claude/skills/start-training-run/SKILL.md` with exactly:

````markdown
---
name: start-training-run
description: Start a Digimon RL training run — on a cloud host (RunPod GPU or Hetzner/DO CPU) or locally. Use WHENEVER the user wants to start / launch / kick off / spin up a training run, train an agent or model, begin a new RL run, run pilot_training, train an LSTM/MLP pilot, stand up a training pod or droplet, or continue training from a checkpoint. Covers picking the host, launching via scripts/train_remote.sh or code/tools/run_training_job.py (cloud) or pilot_training (local), bringing up the TensorBoard watcher, and monitoring/mirroring runs. Cloud-first; the authoritative runbooks are docs/CLOUD_TRAINING.md and docs/TRAINING_RUNBOOK.md. NOT for evaluating an already-trained model (that is anchored eval — docs/MODEL_EVALUATION.md) and NOT for deploying the API (deploy-hosted-api) or the desktop app (cut-desktop-release).
---

# Start a training run

Launch an RL training run for the Digimon agent. **Cloud is the primary path**
(GPU for LSTM / frozen-pool play, CPU for MLP-vs-greedy); local is for quick
iteration. The authoritative runbooks are
[`docs/CLOUD_TRAINING.md`](../../../docs/CLOUD_TRAINING.md) (provisioning,
end-to-end) and [`docs/TRAINING_RUNBOOK.md`](../../../docs/TRAINING_RUNBOOK.md)
(wrapper chain, gauntlet, reward profiles). This skill is the happy-path recipe;
consult the runbooks for anything off the path.

A training run can burn many GPU-hours on a paid host. Confirm host, deck pool,
timesteps, and opponent with the user before launching, and confirm where
artifacts will be persisted (see the **ephemeral storage** gotcha).

## Decide: cloud or local

- **Cloud (default).** Real runs. Path A = RunPod GPU (LSTM, frozen-pool).
  Path B = Hetzner/DO CPU (MLP-vs-greedy, cheaper). Pick per
  `docs/CLOUD_TRAINING.md`.
- **Local.** Smoke / quick iteration only — for a real signal you still need the
  anchored-eval frame below.

## Cloud path

1. **Provision / pick the host** per `docs/CLOUD_TRAINING.md` (RunPod via
   `runpodctl`, or a Hetzner/DO droplet). Join it to the tailnet.
2. **Stage data + start the watcher** (from your laptop / the host):
   ```bash
   rsync -az data/ <host>:~/digimon-training/data/
   rsync -az training_jobs/ <host>:~/digimon-training/training_jobs/
   rsync -az docker-compose.watch.yml <host>:~/digimon-training/
   # on the host, once:
   docker compose -f docker-compose.watch.yml up -d   # TensorBoard on :6006 (tailnet only)
   ```
3. **Launch the trainer.** Either:
   - DO GPU droplet helper: `./scripts/train_remote.sh <droplet_ip> <training_jobs/your_job.yaml> [--publish]`, or
   - the job runner directly: `python code/tools/run_training_job.py <job_config>` (see its `--help`).
4. **Monitor.** TensorBoard at `http://<host>:6006` (tailnet); mirror runs back
   with `scripts/sync_cloud_runs.sh`; inspect with the training MCP
   (`python -m digimon_training_mcp --runs-dir ./runs --models-dir ./models`).

## Local path

```bash
python -m digimon_gym.agents.pilot_training --timesteps 500000
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
python -m digimon_gym.agents.pilot_training --opponent pool --opponent-pool-manifest pool.json --timesteps 1000000
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000
python -m digimon_gym.agents.pilot_training --archetypes rocks,ts-olympos --timesteps 500000
```

`--match-format bo3` (one episode = one best-of-three match) is the default; pass
`--match-format single` for the legacy one-game episode.

## Judging the result (do NOT trust the in-run win rate)

The in-run / mirror eval win rate is **not a cross-mode learning signal** and is
degenerate under self-play. Rank a model only with **anchored evaluation** —
against fixed references (greedy floor + frozen champions), seat-balanced:
`python code/tools/anchored_eval_cli.py --deck-pool-snapshot <run>/deck_pool_snapshot.json --n <adequate-n>`
(see `docs/MODEL_EVALUATION.md`). Training runs also log an in-training anchored
panel (`pilot/anchored/*`), but promotion decisions come from the post-hoc frame.

## Gotchas (each has cost a real run)

- **Ephemeral storage.** Launching with `cd /app` writes artifacts to ephemeral
  `/app/models` (NOT persistent `/workspace`). **Mirror or download the run
  before terminating the pod**, or it is gone.
- **`opponent="self-play"` is retired** and fails at startup. Use
  `opponent="pool"` with a `champion_admin.py emit-pool` manifest.
- **Resume-from-checkpoint:** `run_training_job.py` cannot continue from a
  checkpoint without a small `init_from` patch; the bare CLI also traps on a
  missing `default.yaml`. See the runbook before relying on resume.
- **Reachability:** TensorBoard `:6006` is tailnet-only by design — the cloud
  firewall blocks it publicly. Reach it over Tailscale, not a public IP.
- **Building the engine for training?** Rust target dirs are isolated per
  worktree; a shared `CARGO_TARGET_DIR` causes phantom compile errors. See
  CLAUDE.md rule 31.

## Reference

`docs/CLOUD_TRAINING.md` (provisioning + the two paths), `docs/TRAINING_RUNBOOK.md`
(wrapper chain, gauntlet, standing eval cadence), `docs/MODEL_EVALUATION.md`
(why the in-run number lies; the anchored frame), `AGENTS.md` (architecture).
````

- [ ] **Step 2: Verify the skill file is well-formed**

Run:
```bash
test -f .claude/skills/start-training-run/SKILL.md && echo "present"
head -3 .claude/skills/start-training-run/SKILL.md | grep -q "name: start-training-run" && echo "frontmatter OK"
```
Expected: `present` and `frontmatter OK`.

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/start-training-run/SKILL.md
git commit -m "Add start-training-run skill (cloud-first RL training recipe)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: Author the `deploy-hosted-api` skill

Recipe pointing at the relocated `docs/runbooks/api-deploy.md`. Author/validate with `superpowers:writing-skills`; full content below.

**Files:**
- Create: `.claude/skills/deploy-hosted-api/SKILL.md`

- [ ] **Step 1: Write the skill file**

Create `.claude/skills/deploy-hosted-api/SKILL.md` with exactly:

````markdown
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

Expected: a healthy response. The deploy job already polls `/health` for ~2 min,
so a green run usually means this passes; verify anyway.

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
````

- [ ] **Step 2: Verify the skill file is well-formed**

Run:
```bash
test -f .claude/skills/deploy-hosted-api/SKILL.md && echo "present"
head -3 .claude/skills/deploy-hosted-api/SKILL.md | grep -q "name: deploy-hosted-api" && echo "frontmatter OK"
```
Expected: `present` and `frontmatter OK`.

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/deploy-hosted-api/SKILL.md
git commit -m "Add deploy-hosted-api skill (hosted API image deploy recipe)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: Final verification sweep

- [ ] **Step 1: Confirm acceptance criteria**

```bash
# 1. ops/ is gone
test ! -d ops && echo "ops/ removed"
# 2. no live ops/ references
git grep -n "ops/" -- ':!docs/superpowers' ':!openspec' || echo "no live ops/ refs"
# 3. artifacts in new homes
test -f scripts/pg_backup.sh && test -f docker-compose.watch.yml && test -f docs/runbooks/api-deploy.md && echo "artifacts relocated"
# 4. skills exist
test -f .claude/skills/start-training-run/SKILL.md && test -f .claude/skills/deploy-hosted-api/SKILL.md && echo "skills present"
```
Expected: `ops/ removed`, `no live ops/ refs`, `artifacts relocated`, `skills present`.

- [ ] **Step 2: Confirm the tree is clean and history is intact**

```bash
git status --porcelain        # expect empty (all committed)
git log --oneline -6          # the six task commits + design commit
git log --follow --oneline -- scripts/pg_backup.sh | head -3   # history preserved across the move
```

- [ ] **Step 3 (optional): integrate the branch**

If the work is complete and verified, use the `superpowers:finishing-a-development-branch` skill to choose how to integrate (merge / PR / cleanup). Do not push or open a PR unless the user asks.

---

## Notes for the executor

- **Skills are committed, not symlinked per-worktree.** Creating `.claude/skills/<name>/SKILL.md` and committing it is the publish step; it surfaces in every worktree via the base repo. New skills are not loaded into the *current* session until it restarts — that is expected; the files being present + committed is the deliverable.
- **Do not touch** `docs/superpowers/archive/*` or `openspec/changes/add-cloud-training-pipeline/*` — they document past state by design.
- **Windows/CRLF:** git will warn `LF will be replaced by CRLF` on `.md`/`.sh`/`.yml` writes; that is the repo's normal `.gitattributes` behavior, not an error.

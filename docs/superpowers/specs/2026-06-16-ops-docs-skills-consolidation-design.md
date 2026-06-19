# Consolidate `ops/` into `docs/` + add training / API-deploy skills

- **Date:** 2026-06-16
- **Status:** Approved (design); pending implementation plan
- **Topic:** Single documentation tree; promote operational runbooks to skills

## Problem

Operational knowledge is split across two top-level trees. `docs/` holds the
project's documentation, but `ops/` *also* holds prose runbooks
(`ops/deploy/README.md`, `ops/training/README.md`) interleaved with live runtime
artifacts (`ops/backup/pg_backup.sh`, `ops/training/docker-compose.watch.yml`).
There is no single place to look for "how do I deploy / train", and the only
operational runbook that has been promoted to a skill so far is the desktop
release (`cut-desktop-release` ↔ `docs/runbooks/desktop-release.md`).

Goals:
1. **One docs path.** All prose documentation lives under `docs/`; `ops/` is
   dissolved.
2. **Promote runbooks to skills**, following the existing recipe-skill pattern,
   for the two recurring operations that lack a skill: **starting a training
   run** and **deploying the hosted API**.

Non-goal: the desktop release flow is already a skill (`cut-desktop-release`)
and is left as-is.

## Decisions (confirmed with user)

- **Skills to add:** `start-training-run` and `deploy-hosted-api`. Desktop is
  already covered by `cut-desktop-release`.
- **`ops/` layout:** dissolve `ops/` fully — prose → `docs/`, live artifacts →
  `scripts/` (shell) and the repo root (compose).
- **Training skill scope:** both cloud and local, **cloud-first**.

## Approach

Mirror the established pattern: a **thin recipe skill** (the happy path) that
references a **single authoritative runbook** under `docs/runbooks/`, rather
than a self-contained skill that duplicates runbook prose. One source of truth;
the skill is the recipe, the runbook is the contract. Helper scripts (if any)
live under the skill's own `scripts/` dir (as `cut-desktop-release` does with
`bump_version.py`) — but none are anticipated here, since the existing
`scripts/train_remote.sh`, `code/tools/run_training_job.py`, and the
`build-api-image.yml` workflow already are the helpers.

## Part 1 — Dissolve `ops/`

Use `git mv` for every move to preserve history. `.gitattributes` enforces LF on
`*.sh` / `*.yml` by glob (path-independent), so line endings are unaffected.

| Current path | Destination | Rationale |
|---|---|---|
| `ops/backup/pg_backup.sh` | `scripts/pg_backup.sh` | Live script (mounted into the `backup` compose service). Belongs alongside the other `scripts/*.sh`. |
| `ops/training/docker-compose.watch.yml` | `docker-compose.watch.yml` (repo root) | Live compose file. Sits with `docker-compose.prod.yml` and `docker-compose.override.local.yml`. |
| `ops/deploy/README.md` | `docs/runbooks/api-deploy.md` | Procedural hosted-API runbook (bootstrap / deploy / rollback / restore). Pairs with the new `deploy-hosted-api` skill, mirroring `desktop-release.md` ↔ `cut-desktop-release`. |
| `ops/training/README.md` | folded into `docs/CLOUD_TRAINING.md` | ~40 lines explaining the TensorBoard watcher sidecar; `CLOUD_TRAINING.md` already cross-references it. Folding it in genuinely consolidates rather than relocating sprawl. |

After the moves, the `ops/` directory is removed entirely.

### `api-deploy.md` vs `DEPLOYMENT.md`

`docs/DEPLOYMENT.md` already documents hosted-API **topology / env vars**
(the "what/where"). The relocated runbook is **procedural** (the "how to run
it"). Keep them separate and **cross-link** (DEPLOYMENT.md → api-deploy.md for
"how", api-deploy.md → DEPLOYMENT.md for "what/where"). No large merge — this
matches the desktop split (a conceptual spec plus a procedural runbook).

### References to update (live only)

- `docker-compose.prod.yml` — the backup-service mount path
  `./ops/backup/pg_backup.sh:/usr/local/bin/pg_backup.sh:ro` →
  `./scripts/pg_backup.sh:...`.
- `docker-compose.override.local.yml` (line ~7) — comment pointing at
  `ops/deploy/README.md` → `docs/runbooks/api-deploy.md`.
- `CLAUDE.md` — the `docker compose -f ops/training/docker-compose.watch.yml`
  command (~line 255), the Cloud-training bullet that names
  `ops/training/docker-compose.watch.yml` (~line 347), the **Project Layout**
  tree (drop the `ops/` entry), and **rule 24**'s list of permitted top-level
  dirs (remove `ops/`).
- `docs/CLOUD_TRAINING.md` — the `mkdir -p …/ops/training`, the
  `rsync … ops/training/`, and the `docker compose -f
  ops/training/docker-compose.watch.yml up -d` invocations (~lines 509/519/525),
  plus the folded-in watcher README content.

A final repo-wide grep for `ops/` confirms no live (non-archived) reference is
missed.

### Deliberately left unchanged (historical records)

- `docs/superpowers/archive/*` (archived specs/plans).
- `openspec/changes/add-cloud-training-pipeline/*` (proposal / spec / tasks).

These describe state **as implemented at the time**; rewriting their paths would
falsify the historical record. They are documentation of the past, not live
pointers.

## Part 2 — Skill: `start-training-run`

`.claude/skills/start-training-run/SKILL.md` — cloud-first recipe referencing
`docs/CLOUD_TRAINING.md` (authoritative) and `docs/TRAINING_RUNBOOK.md`.

- **Cloud path (primary):** choose a host (RunPod GPU for LSTM/self-play-style
  pool play; Hetzner/DO CPU for MLP-vs-greedy), launch via
  `scripts/train_remote.sh` or `code/tools/run_training_job.py`, bring up the
  TensorBoard watcher (`docker compose -f docker-compose.watch.yml up -d`),
  monitor via `scripts/sync_cloud_runs.sh` + the training-inspection MCP.
- **Local path:** `python -m digimon_gym.agents.pilot_training` with the common
  flags (`--timesteps`, `--lstm`, `--opponent pool`, `--gauntlet`,
  `--archetypes`, `--match-format bo3`).
- **Gotchas section** (hard-won; sourced from project memory):
  - Launching with `cd /app` writes run artifacts to **ephemeral**
    `/app/models` (not persistent `/workspace`) — **mirror or download before
    terminating the pod.**
  - `run_training_job.py` cannot continue-from-checkpoint without a small patch
    (the `init_from` gap).
  - `opponent="self-play"` is **retired** and fails at startup — use
    `opponent="pool"` with an `emit-pool` manifest.
  - **In-run / mirror win rate is not a cross-mode learning signal** (rule 30);
    rank models with anchored evaluation (greedy + frozen champions,
    seat-balanced).
  - `bo3` is the default Gym episode shape (rule 26).

## Part 3 — Skill: `deploy-hosted-api`

`.claude/skills/deploy-hosted-api/SKILL.md` — recipe referencing the relocated
`docs/runbooks/api-deploy.md`.

- **Happy path:** dispatch `.github/workflows/build-api-image.yml` with
  `deploy=true` (build the image, then pull + restart the droplet) → watch the
  run → `curl -sf https://<host>/health`. (The exact `workflow_dispatch` input
  name is verified against the workflow when authoring the skill.)
- **Rollback:** re-pin the droplet to a known-good git-SHA image tag.
- **Restore:** Postgres restore from the Spaces backup (procedure stays in the
  runbook; skill links to it).
- One-time droplet **bootstrap** stays in the runbook (referenced, not inlined
  into the skill — it is not a recurring operation).
- The skill **description** explicitly disambiguates from `cut-desktop-release`:
  this deploys the **hosted API image**, not the desktop app.

## Skill conventions (both new skills)

- Frontmatter: `name` (matches the directory) + a trigger-rich `description`
  (the description is what routes the skill; enumerate the phrasings a user
  would use — "start a training run", "kick off training", "deploy the API",
  "ship the server", etc.).
- Body: preconditions → numbered happy-path steps → gotchas → rollback →
  reference (pointer to the authoritative runbook).
- Authored with the `superpowers:writing-skills` skill so structure/triggering
  match repo conventions.
- Skills are git-tracked under `.claude/skills/`; they appear in every worktree
  via the base repo, so committing them is the publish step.

## Out of scope

- Desktop deploy (already `cut-desktop-release`).
- The landing page (`landing-page.yml`).
- Any rewrite of historical/archived/openspec records.
- A deep merge of `DEPLOYMENT.md` and `api-deploy.md` (cross-link only).

## Acceptance criteria

1. `ops/` no longer exists; `pg_backup.sh`, `docker-compose.watch.yml`,
   `api-deploy.md`, and the watcher prose are in their new homes with history
   preserved.
2. A repo-wide grep for `ops/` returns only archived/openspec/historical hits.
3. `docker compose -f docker-compose.watch.yml config` and the `backup` service
   in `docker-compose.prod.yml config` both resolve with the new paths.
4. `start-training-run` and `deploy-hosted-api` exist with trigger-rich
   descriptions and reference their authoritative runbooks; neither duplicates
   runbook prose wholesale.
5. `CLAUDE.md` (layout tree + rule 24) and `docs/CLOUD_TRAINING.md` reflect the
   new paths.

## Risks

- **Missed live reference** → a broken mount or a stale command. Mitigated by
  the repo-wide grep gate (criterion 2) and the compose-config check
  (criterion 3).
- **Skill mis-triggering** (e.g. `deploy-hosted-api` firing for a desktop
  release). Mitigated by explicit disambiguation in both new descriptions and
  in `cut-desktop-release` (which already says "NOT for the hosted API image").

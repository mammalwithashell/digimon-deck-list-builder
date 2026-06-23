# Desktop Release Runbook

How to cut, publish, and roll back a Tauri v2 desktop alpha release. The auto-updater design spec lives at [`docs/superpowers/specs/2026-04-21-tauri-auto-updater.md`](../superpowers/specs/2026-04-21-tauri-auto-updater.md); the implementation plan is at [`docs/superpowers/plans/2026-04-21-tauri-auto-updater.md`](../superpowers/plans/2026-04-21-tauri-auto-updater.md).

---

## Design reference

Consolidated inventory of everything the release flow touches. Each row is a stable contract — changing any of these is a migration, not a config tweak.

### Git tag convention

| Tag prefix | Triggers | Workflow |
|---|---|---|
| `desktop-vX.Y.Z[-suffix]` | `.github/workflows/desktop-release.yml` | Build Windows + Linux installers, sign, upload to Spaces, publish manifest |
| `api-vX.Y.Z` | _(reserved; not yet wired)_ | API server deploys. In practice the API image ships via the manually-dispatched `.github/workflows/build-api-image.yml` (run with `deploy=true` to also pull+restart the droplet) |
| `engine-vX.Y.Z` | _(reserved; not yet wired)_ | Engine-only releases |

Suffixes follow SemVer prerelease rules (`-alpha.N`, `-beta.N`). The updater's "is this newer?" check uses the `semver` crate, which correctly orders `0.2.0-alpha.2 < 0.2.0-alpha.3 < 0.2.0`.

Tag body = release notes. The tag **must be annotated** (`git tag -a`) — the publish job re-fetches the tag object explicitly (`git fetch origin "refs/tags/$TAG:refs/tags/$TAG" --force`) because `actions/checkout` only fetches it as a lightweight ref, which would make `%(contents)` return the merge-commit message instead of your notes (this is exactly what shipped in the 0.1.0 manifest). A lightweight tag falls back to the `.github/RELEASE_NOTES.md` template. Testers see the notes verbatim in the update modal — keep it plain text, one fact per line, no markdown (Tauri v2 doesn't render markdown in update notes).

### Build outputs + bundle config

Contracts that bit us during the pipeline revival — each looks like a config tweak but breaks the release if regressed:

- **Bundles land in the workspace-root `target/`**, not `code/src-tauri/target/`. `src-tauri` is a member of the root Cargo workspace, so `cargo tauri build` writes installers to `target/release/bundle/nsis/*-setup.exe` (Windows) and `target/release/bundle/appimage/*.AppImage` (Linux), resolved relative to the repo root. The workflow's `bundle_glob` matrix values encode this.
- **`tauri.conf.json` hooks must use the structured `{script, cwd}` form.** A plain-string `beforeBuildCommand`/`beforeDevCommand` runs with cwd = the `frontendDist` directory (`code/frontend/dist`), not the frontend package root, so `npm run ...` can't find `package.json`. The committed config uses `{"script": "npm run build:desktop", "cwd": "../frontend"}` (cwd relative to `code/src-tauri/`).
- **`bundle.createUpdaterArtifacts: true` is required** for `cargo tauri build` to emit the `.sig` files next to the installers. Without it the build succeeds but the publish job fails collecting `${INSTALLER}.sig`, and the manifest can't carry the Ed25519 signatures the updater verifies.

### Release channels

| Channel | Manifest URL path | Used by | Status |
|---|---|---|---|
| `alpha` | `updates/alpha/latest.json` | Current alpha testers | Active |
| `beta` | `updates/beta/latest.json` | Reserved | Not yet used |
| `stable` | `updates/stable/latest.json` | Reserved | Not yet used |

Only `alpha` is active. Path shape is future-proof — adding a channel is a static-file operation, not a schema change. Channel identifier is compile-baked into the desktop build via `code/src-tauri/src/updater.rs:MANIFEST_URL` and `tauri.conf.json:plugins.updater.endpoints`. Switching channels means building a different binary; you can't flip a tester mid-install.

### URL shapes

| Concern | URL pattern | Set by |
|---|---|---|
| Manifest (Tauri reads this) | `https://<spaces-cdn-host>/updates/<channel>/latest.json` | `code/server/db/routers/admin_releases.py:_manifest_key` + `spaces.public_url` |
| Installer artifact | `https://<spaces-cdn-host>/releases/<release_id>/<filename>` | `admin_releases.py:_artifact_spaces_key` |
| Filename (Windows) | `digimon-tcg-<version>-x86_64-setup.exe` | `admin_releases.py:_artifact_filename` |
| Filename (Linux) | `digimon-tcg-<version>-x86_64.AppImage` | same |
| Hosted API — create | `POST {HOSTED_API_URL}/admin/releases` | |
| Hosted API — confirm | `POST {HOSTED_API_URL}/admin/releases/{id}/artifacts/{target}/confirm` | |
| Hosted API — publish | `POST {HOSTED_API_URL}/admin/releases/{id}/publish` | |
| Hosted API — unpublish | `POST {HOSTED_API_URL}/admin/releases/{id}/unpublish` | |
| Hosted API — patch | `PATCH {HOSTED_API_URL}/admin/releases/{id}` | |
| Hosted API — delete | `DELETE {HOSTED_API_URL}/admin/releases/{id}` | |
| Hosted API — list | `GET {HOSTED_API_URL}/admin/releases?channel=&published=` | |
| Hosted API — regenerate manifest | `POST {HOSTED_API_URL}/admin/releases/regenerate-manifest?channel=` | |

All `/admin/releases/*` endpoints require `ROLE_ADMIN` via `Bearer` JWT. The manifest URL is public-read (no auth).

The `<spaces-cdn-host>` resolves via `code/server/storage/spaces.py:public_url()`:
- If `SPACES_CDN_URL` env var is set: `{SPACES_CDN_URL}/<key>` (DO Spaces CDN, preferred for throughput)
- Else: `{SPACES_ENDPOINT}/{SPACES_BUCKET}/<key>` (direct origin)

The desktop client's `tauri.conf.json` `plugins.updater.endpoints[0]` **must** exactly match the server's computed URL. If you ever change `SPACES_CDN_URL` or the bucket, you must rebuild the desktop or it will 404 fetching the manifest.

### Environment variables

#### Hosted API server (production droplet)

| Variable | Purpose | Example |
|---|---|---|
| `DATABASE_URL` | Postgres connection string | `postgresql+asyncpg://user:pass@host/digimon` |
| `SECRET_KEY` | JWT signing key | random 64-byte hex |
| `SPACES_ENDPOINT` | DO Spaces origin | `https://nyc3.digitaloceanspaces.com` |
| `SPACES_REGION` | DO region | `nyc3` |
| `SPACES_BUCKET` | Bucket name | `digimon-tcg-models` |
| `SPACES_KEY` | DO Spaces access key | secret |
| `SPACES_SECRET` | DO Spaces secret key | secret |
| `SPACES_CDN_URL` | Preferred public host (optional) | `https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com` |

Server reads these lazily — missing env vars raise `RuntimeError` at first Spaces call (see `spaces.py:_require_env`). The release admin API calls Spaces on every create/confirm/publish/unpublish, so any missing var will surface on the first operation after server start.

#### Desktop client (compile-time + runtime)

| Variable | Purpose | Set where |
|---|---|---|
| `VITE_BUILD_TARGET` | `desktop` vs `web` — tree-shakes admin/training UI | CI, local dev (`npm run dev:desktop`) |
| Tauri `env!("CARGO_PKG_VERSION")` | Runtime self-version for min-version comparison | `Cargo.toml:[package].version` |

No runtime env vars consumed by the updater itself — the manifest URL + pubkey are compile-baked into the binary (`code/src-tauri/tauri.conf.json`, `code/src-tauri/src/updater.rs:MANIFEST_URL`). This is intentional: a tester can't be tricked into pointing at an attacker's manifest via env-var injection.

#### GitHub Actions secrets

| Secret | Consumed by | Set via |
|---|---|---|
| `TAURI_UPDATER_PRIVATE_KEY` | `cargo tauri build` (bundle signing) | `Get-Content -Raw $HOME\.tauri\digimon-updater.key \| gh secret set TAURI_UPDATER_PRIVATE_KEY` |
| `TAURI_UPDATER_KEY_PASSWORD` | `cargo tauri build` (decrypts the key) | `gh secret set TAURI_UPDATER_KEY_PASSWORD` (interactive paste) |
| `HOSTED_API_URL` | `.github/workflows/desktop-release.yml` publish job | `gh secret set HOSTED_API_URL --body "https://..."` |
| `CI_ADMIN_TOKEN` | same | Provisioned **inside the API container** on the droplet (the tool needs the prod DB): `docker compose exec -T api python tools/provision_ci_release_user.py --password "$P"`, then pipe/paste the printed JWT into `gh secret set CI_ADMIN_TOKEN` |
| `GITHUB_TOKEN` | `gh release create` at workflow end | Auto-provisioned by GitHub Actions |

**Secrets explicitly NOT in CI** (by design — least-privilege):
- `SPACES_KEY`, `SPACES_SECRET` — CI uploads via presigned PUTs obtained from the hosted API.
- `SECRET_KEY`, `DATABASE_URL` — CI never touches the DB directly.

### Secret inventory by location

| Location | Holds | Rotation trigger |
|---|---|---|
| `$HOME\.tauri\digimon-updater.key` (maintainer machine) | Ed25519 private key (password-encrypted) | Key leak; pre-production launch |
| 1Password "Digimon TCG" → "Tauri Updater Key" | Private key password | With the key |
| GitHub Actions `TAURI_UPDATER_PRIVATE_KEY` + `TAURI_UPDATER_KEY_PASSWORD` | Signing key for CI builds | With the key |
| `code/src-tauri/tauri.conf.json:plugins.updater.pubkey` | Public key (committed) | With the key, major version bump |
| DB `users` row `ci-desktop-release` | CI user's bcrypt password hash | Annually or on leak |
| GitHub Actions `CI_ADMIN_TOKEN` | CI user's long-lived JWT (365d) | Annually or on leak; re-provision via the tool |

### Database schema

Two tables, cascade-linked. See `alembic/versions/20260421_0015_app_releases.py` for the authoritative migration.

| Table | PK | Notable columns | Purpose |
|---|---|---|---|
| `app_releases` | `id` (uuid) | `version`, `channel`, `engine_commit`, `min_version`, `release_notes`, `published`, `published_at`, `state` | One row per release per channel |
| `app_release_artifacts` | `id` (uuid) | `release_id` (FK, CASCADE), `target`, `spaces_key`, `filename`, `file_sha256`, `file_size_bytes`, `signature_b64` | One row per platform-specific installer |

Invariants:
- `UNIQUE(channel, version)` — can't re-publish the same version string.
- `UNIQUE(release_id, target)` — one installer per platform per release.
- `UNIQUE(spaces_key)` — no two artifact rows point at the same Spaces object.
- `state IN ('pending', 'uploaded', 'failed')` — enforced by CHECK.
- `target IN ('windows-x86_64', 'linux-x86_64')` — enforced by CHECK; adding macOS is a migration.
- `file_sha256`, `file_size_bytes`, `signature_b64` nullable by construction (populated on `/confirm`). There's no CHECK enforcing "if state='uploaded' then these must be NOT NULL" — the router layer owns that invariant.

### Manifest contract (what Tauri reads from Spaces)

Canonical shape at `https://<spaces-cdn-host>/updates/<channel>/latest.json`. Served public-read with `Cache-Control: public, max-age=60`. A verified live example (the shipped 0.1.0 manifest) is at <https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com/updates/alpha/latest.json>.

```json
{
  "version": "0.2.0-alpha.3",
  "pub_date": "2026-04-21T18:40:00Z",
  "notes": "Fix: deckbuilder crash.",
  "platforms": {
    "windows-x86_64": { "signature": "<base64>", "url": "https://.../releases/<uuid>/digimon-tcg-0.2.0-alpha.3-x86_64-setup.exe" },
    "linux-x86_64":   { "signature": "<base64>", "url": "https://.../releases/<uuid>/digimon-tcg-0.2.0-alpha.3-x86_64.AppImage" }
  },
  "min_version": "0.1.0",
  "engine_commit": "fbf8288",
  "channel": "alpha",
  "release_id": "<uuid>"
}
```

| Field | Consumed by | Notes |
|---|---|---|
| `version` | Tauri plugin (compared to running) | SemVer; must be strictly greater for update to propose |
| `pub_date` | Display only | ISO 8601 UTC, `Z` suffix |
| `notes` | Update modal body | Plain text, newline-separated |
| `platforms.<target>.signature` | Tauri plugin (Ed25519 verify) | Base64 output of `cargo tauri signer sign` |
| `platforms.<target>.url` | Tauri plugin (download) | Public-read Spaces URL |
| `min_version` | `code/src-tauri/src/updater.rs` (startup guard) | SemVer floor; running below triggers force-update modal |
| `engine_commit` | Display + future gating | 7-char git SHA at CI build time |
| `channel` | Logging + admin UI | Redundant with URL path |
| `release_id` | Telemetry, logging | UUID of the DB row |

### Update UX state machine

```
app launch
   │
   ├─ min-version guard (Rust, 3s timeout)
   │   ├─ manifest.min_version > running: emit "updater:force-update" ──→ blocking modal ──→ downloadAndInstall ──→ relaunch
   │   └─ ok: no event
   │
   └─ background check (JS, on App mount)
       ├─ plugin.check() returns update: setState, show corner toast ──→ user clicks ──→ modal with notes ──→ install ──→ relaunch
       └─ no update: no UI
```

Two independent paths; force-update takes precedence over the normal toast/modal. Both go through the same `downloadAndInstall()` → `relaunch()` sequence (which Ed25519-verifies the installer before applying).

### Spec / plan / code map

| Concern | Spec § | Plan Task | Primary code |
|---|---|---|---|
| Decisions + rationale | `Decisions` | _(n/a)_ | _(n/a)_ |
| Manifest JSON shape | `Manifest contract` | Task 7 | `admin_releases.py:_build_manifest` |
| Admin API surface | `Server surface` | Tasks 5–8 | `code/server/db/routers/admin_releases.py` |
| DB tables | `Server surface → DB schema` | Tasks 1–2 | `alembic/.../20260421_0015_app_releases.py`, `code/server/db/models.py` |
| Tauri wiring | `Tauri integration` | Tasks 10–12 | `code/src-tauri/tauri.conf.json`, `code/src-tauri/src/updater.rs`, `code/frontend/src/updater/` |
| CI pipeline | `Release pipeline` | Tasks 13–14 | `.github/workflows/desktop-release.yml`, `tools/provision_ci_release_user.py` |
| Rollback + kill-switch | `Rollback + kill-switch` | Task 8 (unpublish) + Task 11 (min_version) | `admin_releases.py:unpublish_release`, `code/src-tauri/src/updater.rs:check_min_version` |

---

## Updater key custody

- Private key file: `$HOME/.tauri/digimon-updater.key` (password-encrypted, maintainer machine only).
- Password: 1Password → "Digimon TCG" → "Tauri Updater Key".
- Public key: committed in `code/src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.
- GitHub Actions secrets:
  - `TAURI_UPDATER_PRIVATE_KEY` — file contents
  - `TAURI_UPDATER_KEY_PASSWORD` — key passphrase
  - `HOSTED_API_URL` — hosted API base URL (e.g. `https://api.digimon-tcg.example.com`)
  - `CI_ADMIN_TOKEN` — long-lived JWT for the `ci-desktop-release` admin user (provision with `tools/provision_ci_release_user.py`)

On Windows PowerShell, `~` in command-line paths passed to native binaries (including `cargo`) is taken literally as a directory name — use `$HOME` instead.

---

## One-time setup

```powershell
# 1. Install tauri CLI if missing
cargo install tauri-cli --version "^2" --locked

# 2. Generate the Ed25519 key (prompts for password twice)
cargo tauri signer generate -w $HOME\.tauri\digimon-updater.key

# 3. Upload to GitHub Actions secrets
Get-Content -Raw $HOME\.tauri\digimon-updater.key | gh secret set TAURI_UPDATER_PRIVATE_KEY
gh secret set TAURI_UPDATER_KEY_PASSWORD                # interactive paste
gh secret set HOSTED_API_URL --body "https://api.digimon-tcg.example.com"

# 4. Paste the printed public key into code/src-tauri/tauri.conf.json
#    plugins.updater.pubkey — commit that change

# 5. Provision the CI admin user + long-lived token.
#    The provisioning tool needs the production DB, so run it INSIDE the API
#    container on the droplet (not on your machine):
ssh <droplet> 'cd /opt/digimon && docker compose -f docker-compose.prod.yml exec -T api \
  python tools/provision_ci_release_user.py --password "$(openssl rand -base64 32)"'
# Copy the printed JWT into the GHA secret:
gh secret set CI_ADMIN_TOKEN                             # interactive paste
```

The Spaces URL in `plugins.updater.endpoints` must match where the server writes the manifest (see `code/server/storage/spaces.py:public_url()` — which prefers `SPACES_CDN_URL` on the server, else `SPACES_ENDPOINT/<bucket>/`). The default configured is `https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com/updates/alpha/latest.json` — adjust if your bucket host differs.

The hosted API itself must be running the release-admin code before the publish job can succeed. The API image is published via the manually-dispatched [`build-api-image.yml`](../../.github/workflows/build-api-image.yml) workflow — `gh workflow run build-api-image.yml -f deploy=true` builds, pushes to GHCR, and pulls+restarts the droplet.

---

## Cut a new release

1. Ensure `main` is green and the change you want to ship is merged.
2. Refresh the implemented-cards allowlist so cards implemented since the last release actually ship:
   ```bash
   python code/tools/build_tested_cards.py   # rewrites data/tested_cards.json from the live engine pool
   ```
   This snapshot is `include_str!`-baked into the desktop binary at compile time (`code/digimon-engine/src/deck_tools.rs`), so without this step the deck builder rejects newly-implemented cards as "not available in the alpha release." Commit the result together with the version bump (step 4) — the same committed file also feeds the hosted API and browser builds, so it must stay the single source of truth.
3. Bump the version in both files (they must stay in sync):
   - `code/src-tauri/tauri.conf.json` — the `version` field at the top
   - `code/src-tauri/Cargo.toml` — the `[package].version` field
4. Commit the version bump together with the refreshed `data/tested_cards.json`.
5. Create an annotated tag with release notes in the body. The body becomes the update-modal text your testers see.
   ```bash
   git tag -a desktop-v0.2.0-alpha.3 -m "Fix: deckbuilder crash on whitespace import.
   Add: Beelzemon gauntlet preset."
   git push origin desktop-v0.2.0-alpha.3
   ```
6. Watch CI: `gh run watch`. Both build jobs (Windows + Linux) must succeed, then the `publish` job calls the hosted API to create/upload/confirm/publish in sequence.
7. Verify by refetching the manifest:
   ```bash
   curl https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com/updates/alpha/latest.json | jq
   ```
   Should show the new version, both `platforms` populated, and `pub_date` within the last few minutes. `Cache-Control: public, max-age=60` header is set; propagation ≤ 60 seconds.
8. Verify a running tester sees the update. Easiest check: launch the previously-installed alpha on your own machine, confirm the "Update available" toast appears within ~5s of launch, click through to install.

---

## Roll back a broken release

### Scenario A: broken build still launches

Testers can self-recover by updating forward. The rollback is "cut a new release from the last good commit":

1. `git checkout <last-good-commit>`
2. Bump `code/src-tauri/tauri.conf.json` and `code/src-tauri/Cargo.toml` to a version strictly greater than the broken one (e.g., broken `0.2.0-alpha.3` → cut `0.2.0-alpha.4`).
3. Tag + push:
   ```bash
   git tag -a desktop-v0.2.0-alpha.4 -m "Revert 0.2.0-alpha.3 — deckbuilder regression"
   git push origin desktop-v0.2.0-alpha.4
   ```
4. CI publishes. Testers update forward on next launch.
5. Also unpublish the broken release so any fresh-install flow doesn't reach for it:
   ```bash
   BROKEN_ID=$(curl -s -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
     "$HOSTED_API_URL/admin/releases?channel=alpha" \
     | jq -r '.releases[] | select(.version == "0.2.0-alpha.3") | .id')
   curl -X POST -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
     "$HOSTED_API_URL/admin/releases/$BROKEN_ID/unpublish"
   ```

Unpublish by itself re-promotes the most recent prior published release on the channel (the admin API's unpublish logic handles this) — so if you only have the one broken version in the wild, `unpublish` without a follow-up release just deletes the manifest. Either cut a new release OR accept the alpha is parked.

### Scenario B: broken build crashes before the updater can run

Normal update path is dead. Use the `min_version` kill-switch:

1. Cut + publish a new known-good release as in Scenario A.
2. PATCH the published release's `min_version` above the broken version:
   ```bash
   NEW_ID=$(curl -s -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
     "$HOSTED_API_URL/admin/releases?channel=alpha&published=true" \
     | jq -r '.releases[0].id')
   curl -X PATCH -H "Authorization: Bearer $CI_ADMIN_TOKEN" \
     -H "Content-Type: application/json" \
     "$HOSTED_API_URL/admin/releases/$NEW_ID" \
     -d '{"min_version": "0.2.0-alpha.4"}'
   ```
3. Running broken installs now see `manifest.min_version > running.version` on next launch and hit the force-update modal (emitted by the Rust-side min-version guard BEFORE any game code runs, so it survives most in-app crashes).

Caveat: if the crash is during Rust plugin init itself (pre-setup), the min-version check never runs. At that point the only recourse is to email testers the new installer URL for manual reinstall.

---

## Rotate the updater private key

Rotation invalidates every already-installed tester's auto-update path — Tauri verifies updates against the pubkey baked into the binary they're running, and that pubkey can only change in a new native build. **Do not rotate casually.** If the key has leaked:

1. Generate a new key:
   ```powershell
   cargo tauri signer generate -w $HOME\.tauri\digimon-updater-v2.key
   ```
2. Update `code/src-tauri/tauri.conf.json`'s `plugins.updater.pubkey`.
3. Update GHA secrets:
   ```powershell
   Get-Content -Raw $HOME\.tauri\digimon-updater-v2.key | gh secret set TAURI_UPDATER_PRIVATE_KEY
   gh secret set TAURI_UPDATER_KEY_PASSWORD   # paste the new password
   ```
4. Bump version clearly past the current release (e.g., `0.3.0-alpha.1`).
5. Cut the release via the normal flow.
6. Email alpha tester list: "Please download the new installer manually from [GitHub release URL]. Auto-update will not work across this version."
7. Delete the old private key file + 1Password password entry once every tester has confirmed they've reinstalled.

---

## Common issues

| Symptom | Likely cause | Fix |
|---|---|---|
| CI publish step 401s | `CI_ADMIN_TOKEN` expired (annual) or user revoked | Re-provision in-container on the droplet: `docker compose -f docker-compose.prod.yml exec -T api python tools/provision_ci_release_user.py --password "$(openssl rand -base64 32)"`, then `gh secret set CI_ADMIN_TOKEN` with the printed JWT |
| Manifest `notes` show a merge-commit message ("Merge pull request #...") instead of your release notes | Tag pushed as lightweight, or the publish job didn't fetch the annotated tag object (`actions/checkout` fetches tags as lightweight refs; the workflow's "Fetch annotated tag object" step exists to fix this) | Ensure the tag was created with `git tag -a -m "..."`. Verify the object type: `git cat-file -t desktop-vX.Y.Z` must print `tag`, not `commit`. To repair a shipped release: `PATCH /admin/releases/{id} {"release_notes": "..."}` then regenerate the manifest |
| Build job can't find the installer (`Collect artifact paths` globs match nothing) | Looking under `code/src-tauri/target/` — but `src-tauri` is a root-workspace member, so bundles land in the **workspace-root** `target/release/bundle/...` | Use the workspace-root paths (the workflow matrix `bundle_glob`s already do) |
| `beforeBuildCommand` fails with `npm` unable to find `package.json` | Hook written as a plain string — plain-string hooks run with cwd = the `frontendDist` dir, not the frontend package root | Use the structured form in `tauri.conf.json`: `{"script": "npm run build:desktop", "cwd": "../frontend"}` |
| Publish job fails on missing `.sig` files | `bundle.createUpdaterArtifacts` absent or false — the build emits installers but no updater signatures | Set `"createUpdaterArtifacts": true` under `bundle` in `tauri.conf.json` |
| Publish step 404s on `/admin/releases` | Droplet running a stale API image without the release-admin router | Dispatch `build-api-image.yml` with `deploy=true` to build + deploy the current image |
| Tauri build signing fails "bad password" | GHA secret mismatch with key file | Re-set `TAURI_UPDATER_KEY_PASSWORD` from 1Password |
| Testers don't see the new release | Spaces CDN cache (60s) | Wait one minute. If persistent: `curl -X POST -H "Authorization: Bearer $CI_ADMIN_TOKEN" "$HOSTED_API_URL/admin/releases/regenerate-manifest?channel=alpha"` and `curl` the manifest URL to confirm the rewrite landed |
| Windows SmartScreen blocks installer | Self-signed installer (expected for alpha) | Tester clicks "More info → Run anyway". Documented UX cost until an OV/EV cert is purchased |
| Linux AppImage won't self-update | AppImage not made executable | `chmod +x digimon-tcg-*-x86_64.AppImage` and relaunch |
| `desktop-v*` tag fires workflow but build fails on `frontendDist not found` | Frontend not rebuilt for desktop before Tauri build step | CI already runs `VITE_BUILD_TARGET=desktop npm run build` before `cargo tauri build`; if it still fails, check the frontend build step's logs for the actual error |

---

## Local smoke test (no CI)

Useful when you're changing the admin API or Spaces flow and want to verify end-to-end without a real CI run.

```bash
# Terminal 1 — local hosted API with real Spaces credentials (use a dev bucket!)
export SPACES_ENDPOINT=https://nyc3.digitaloceanspaces.com
export SPACES_BUCKET=digimon-tcg-dev
export SPACES_REGION=nyc3
export SPACES_KEY=...
export SPACES_SECRET=...
python -m uvicorn digimon_gym.api:app --host 0.0.0.0 --port 8000

# Terminal 2
dd if=/dev/urandom of=/tmp/fake-installer.exe      bs=1024 count=100
echo "fake-sig-windows" | base64 > /tmp/fake-installer.exe.sig
dd if=/dev/urandom of=/tmp/fake-installer.AppImage bs=1024 count=100
echo "fake-sig-linux"   | base64 > /tmp/fake-installer.AppImage.sig

TOKEN=$(python tools/provision_ci_release_user.py --password smokepass)

python tools/publish_release_smoke.py \
  --api http://localhost:8000 \
  --token "$TOKEN" \
  --version 0.0.1-smoke.1 \
  --windows-installer /tmp/fake-installer.exe \
  --windows-sig       /tmp/fake-installer.exe.sig \
  --linux-installer   /tmp/fake-installer.AppImage \
  --linux-sig         /tmp/fake-installer.AppImage.sig

# Teardown
RID=$(curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/admin/releases?channel=alpha" | jq -r '.releases[0].id')
curl -X POST   -H "Authorization: Bearer $TOKEN" "http://localhost:8000/admin/releases/$RID/unpublish"
curl -X DELETE -H "Authorization: Bearer $TOKEN" "http://localhost:8000/admin/releases/$RID"
```

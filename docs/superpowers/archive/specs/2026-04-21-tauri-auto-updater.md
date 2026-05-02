# Tauri v2 Auto-Updater + Code-Signing — Design Spec

**Date:** 2026-04-21
**Status:** Spec — not yet implemented
**Sibling precedent:** [`.claude/plans/model-admin-api.md`](../../../.claude/plans/model-admin-api.md) (ONNX model distribution; same Spaces + hosted-API pattern)

---

## Context / Goal

The desktop alpha ships via Tauri v2 (`src-tauri/`, Python-free at runtime). Today every alpha tester re-downloads installers manually when we fix something — a tight feedback loop is the entire point of alpha, and manual re-install kills it. We need an in-app auto-updater, a release pipeline that takes "I pushed a git tag" to "testers get a prompt on next launch," and enough code-signing discipline that Tauri's updater (which mandates signed payloads) actually works. We are not paying for EV/OV Windows certs or an Apple Developer ID during alpha; we are paying for the Ed25519 updater keypair Tauri generates locally, because it's free and required.

The auto-updater should share architectural DNA with the ONNX model distribution flow: binary artifacts in DigitalOcean Spaces, metadata in Postgres on the hosted API, Spaces URLs cited in a manifest the desktop reads. The two flows stay separate (different tables, different endpoints, different manifests) but follow the same shape so future-us isn't learning two patterns.

### Non-goals

- Paid code-signing certs (Windows EV/OV, Apple Developer ID). Alpha is self-signed / unsigned at the installer level.
- macOS support.
- Linux `.deb` or `.rpm` distribution. AppImage only.
- Differential / binary-patch updates. Full-installer replace each time.
- Multi-channel UI (beta/stable). Channel path is namespaced for future expansion; only `alpha` is active.
- Staged rollouts / percentage-based pushes.
- Admin UI for release management. CI publishes; one-off manual intervention is DB SQL + a manifest-regenerate endpoint.
- Engine-commit-mismatched-model refusal in the desktop client. The `engine_commit` field is exposed for future use and admin visibility; client-side filtering is owned by the ONNX flow, not this spec.

---

## Decisions

| # | Topic | Decision | Rationale |
|---|---|---|---|
| 1 | Manifest hosting | Hybrid: static `updates/<channel>/latest.json` in Spaces (Tauri reads this); DB-backed admin API rewrites it on publish/unpublish. | Tauri update path stays up even when hosted API is down; admin writes still get audit trail + DB semantics. |
| 2 | Channels | URL shape `updates/<channel>/latest.json`; only `alpha` active. | Adding `beta`/`stable` later is a new static file, not a schema change. |
| 3 | Windows installer signing | Self-signed for alpha; document SmartScreen click-through. | EV/OV cost + lead time unjustified for <50 alpha testers. Migration path to OV cert documented as follow-up. |
| 3b | Tauri updater key | Ed25519 keypair generated via `cargo tauri signer generate`. Mandatory — not optional. | Tauri updater refuses unsigned payloads regardless of installer cert. Private key lives only on the release-signing GitHub Actions secret; public key baked into `tauri.conf.json`. |
| 4 | macOS | Out of scope. | No paid Apple Developer ID; unsigned macOS updater is blocked by Gatekeeper at apply-time. Revisit for beta. |
| 5 | Linux | AppImage only. | Tauri updater self-replaces AppImages natively; Steam Deck target works. |
| 6 | Release pipeline | GitHub Actions, `desktop-vX.Y.Z[-alpha]` tag trigger. | Shared CI pattern with API server; no manual click-through; tag convention distinguishes desktop vs API releases. |
| 7a | Rollback | `POST /admin/releases/{id}/unpublish` sets `published=false` and rewrites Spaces manifest to point at newest remaining `published=true` row. | Instant, no CI involved. |
| 7b | Kill-switch | `min_version` field in manifest; Tauri-side wrapper refuses to run below it. | Nuclear option for a build that can't self-update itself. |
| 8a | Update UX | Silent background check on launch → corner notification → user-initiated modal with release notes → install + restart. | Transparent to testers; no surprise restarts; release notes build trust. |
| 8b | Engine-commit gating | Release row carries `engine_commit` from CI (`git rev-parse HEAD` at build time); exposed in manifest. Client does not enforce. | Admin visibility + future cross-manifest compatibility guarantee. Actual engine/model matching stays owned by the ONNX flow. |

---

## Manifest contract (authoritative)

Served as a **static file** at `https://<bucket>.<region>.digitaloceanspaces.com/updates/<channel>/latest.json`. Public-read ACL, `Cache-Control: public, max-age=60` set on the Spaces object. Tauri's updater plugin fetches this URL directly; the hosted API is not in the read path.

Tauri's updater plugin expects a specific JSON shape — this spec uses Tauri's v2 native shape for the top-level fields (`version`, `pub_date`, `platforms`, `notes`) so the plugin can consume the manifest with zero custom parsing, and augments with project-specific fields (`min_version`, `engine_commit`, `channel`, `release_id`) that our own code reads.

```json
{
  "version": "0.2.0-alpha.3",
  "pub_date": "2026-04-21T18:40:00Z",
  "notes": "Fix: deckbuilder crash when importing .dck with trailing whitespace.\nAdd: Beelzemon BT14 gauntlet preset.",
  "platforms": {
    "windows-x86_64": {
      "signature": "dW50cnVzdGVk…",
      "url": "https://<bucket>.<region>.digitaloceanspaces.com/releases/<release_id>/digimon-tcg-0.2.0-alpha.3-x86_64-setup.exe"
    },
    "linux-x86_64": {
      "signature": "dW50cnVzdGVk…",
      "url": "https://<bucket>.<region>.digitaloceanspaces.com/releases/<release_id>/digimon-tcg-0.2.0-alpha.3-x86_64.AppImage"
    }
  },
  "min_version": "0.1.0",
  "engine_commit": "fbf8288",
  "channel": "alpha",
  "release_id": "b3f2…"
}
```

**Field semantics:**

- `version` — [SemVer](https://semver.org/) string. Must be strictly greater than the running app's version for the updater to propose an update. Prerelease suffix (`-alpha.N`) is meaningful to Tauri's SemVer comparison.
- `pub_date` — ISO 8601 UTC. Display-only; not used for update logic.
- `notes` — Plain-text release notes, newline-separated. Rendered verbatim in the update modal. Markdown is **not** rendered (Tauri v2 does not); keep notes plain.
- `platforms.<target>.signature` — Base64-encoded Ed25519 signature over the artifact bytes, produced by `cargo tauri signer sign`. Tauri's updater plugin verifies this against the pubkey baked into `tauri.conf.json` before applying.
- `platforms.<target>.url` — Stable public Spaces URL for the signed installer. Never presigned; objects are public-read.
- `min_version` — SemVer string. Project-specific. The desktop wrapper checks this at startup against its own compile-time version; if running version is below `min_version`, the app shows a blocking "Please update to continue" modal and refuses to proceed past the splash screen until the updater completes. See [Rollback + kill-switch](#rollback--kill-switch).
- `engine_commit` — Git SHA (short, 7 chars) of `digimon-engine/` at build time. Project-specific. Exposed for admin visibility and future cross-manifest checks; no enforcement in this spec.
- `channel` — Literal channel name (`"alpha"`). Redundant with the URL path but convenient for logging / admin UI.
- `release_id` — UUID primary key of the DB row. Lets the desktop report "I'm running release X" in telemetry / bug reports.

**Ordering:** one release per channel at a time. The static JSON is fully overwritten on each publish/unpublish; there is no historical array.

**Target identifier conventions** (match Tauri v2's platform naming):
- `windows-x86_64` — `.msi` or `-setup.exe` NSIS installer
- `linux-x86_64` — `.AppImage`

**Artifact naming convention** at the Spaces URL:
- `digimon-tcg-<version>-x86_64-setup.exe` (Windows NSIS; `.msi` also acceptable — whichever Tauri produces by default for our config)
- `digimon-tcg-<version>-x86_64.AppImage` (Linux)
- Companion `.sig` files live next to each artifact at `<artifact-url>.sig` for auditability, though Tauri's updater reads the signature from the manifest JSON, not the sidecar.

---

## Server surface

### Module location

`digimon_gym/db/routers/admin_releases.py`. Rule 11 forbids DB imports from engine-only routers; this is DB-backed, so it lives under `db/routers/` alongside `admin_models.py` and `patch_notes.py`. `admin_models.py` is the structural template.

There is no public read endpoint. Tauri reads the manifest directly from Spaces. The hosted API's only read-side responsibility is internal admin list views (which are deferred along with the admin UI).

### DB schema

New table `app_releases`. Migration file: `alembic/versions/20260421_0015_app_releases.py` (verify head with `alembic heads` at implementation time; chain off the latest revision).

```
app_releases
────────────────────────────────────────────
id                 str  PK (uuid)
version            str  not null  -- SemVer, e.g. "0.2.0-alpha.3"
channel            str  not null  -- "alpha"
engine_commit      str  not null  -- short git SHA
min_version        str  not null  -- SemVer for kill-switch
release_notes      text not null default ''
published          bool not null default false
published_at       datetime nullable
state              str  not null default 'pending'
                     -- 'pending' | 'uploaded' | 'failed'
                     -- 'pending' means row exists, artifacts not yet confirmed
                     -- 'uploaded' means all per-platform artifacts confirmed + hashed + signed
created_at         datetime not null default utcnow
updated_at         datetime not null default utcnow onupdate utcnow

Constraints:
  UNIQUE (channel, version)      -- can't re-publish the same version string
  CHECK  state IN ('pending','uploaded','failed')
  INDEX  (channel, published)
```

And a child table `app_release_artifacts` — one row per platform-specific artifact of a release:

```
app_release_artifacts
────────────────────────────────────────────
id                 str  PK (uuid)
release_id         str  FK app_releases.id ON DELETE CASCADE
target             str  not null      -- 'windows-x86_64' | 'linux-x86_64'
spaces_key         str  not null      -- "releases/<release_id>/<filename>"
file_sha256        str  not null      -- lowercase hex, 64 chars
file_size_bytes    int  not null
signature_b64      text not null      -- base64 Ed25519 signature
filename           str  not null      -- e.g. "digimon-tcg-0.2.0-alpha.3-x86_64-setup.exe"

Constraints:
  UNIQUE (release_id, target)
  UNIQUE (spaces_key)
  CHECK  target IN ('windows-x86_64', 'linux-x86_64')
```

The child table lets one `app_releases` row describe a multi-platform release (Windows + Linux AppImage) without denormalizing columns per platform. When `/publish` runs, it reads `app_release_artifacts` for that release and emits the `platforms` object in the Spaces manifest.

### Endpoints

All endpoints under `/admin/releases/*` require `ROLE_ADMIN` via `require_roles(ROLE_ADMIN)` (same pattern as `admin_models.py`). CI has a dedicated admin-role user with a long-lived API token; the token lives in a GitHub Actions secret. See [Release pipeline](#release-pipeline).

| Method + path | Behavior |
|---|---|
| `POST /admin/releases` | Create a release row. Body: `{version, channel, engine_commit, min_version, release_notes, targets: ["windows-x86_64", "linux-x86_64"]}`. Server creates the `app_releases` row in `state='pending'` plus one `app_release_artifacts` placeholder row per target (with `spaces_key` set to `releases/<release_id>/<filename-to-be-uploaded>`; filename derived server-side from version + target). Returns `{release_id, artifacts: [{target, upload_url, spaces_key, expires_in}, ...]}`, one presigned PUT per target (15min TTL). |
| `POST /admin/releases/{id}/artifacts/{target}/confirm` | Body: `{signature_b64}`. Server HEADs the Spaces object → populates `file_size_bytes`; streams sha256 via `stream_sha256(spaces_key)` → populates `file_sha256`; stores `signature_b64` verbatim (CI computed it with `cargo tauri signer sign`). Does **not** verify the signature server-side — the trust chain is that CI has the private key and the desktop verifies before applying; server is a pass-through. Idempotent. On HEAD failure → 422, artifact row unchanged. |
| `POST /admin/releases/{id}/publish` | Preconditions: release `state='pending'`, all declared `app_release_artifacts` for this release have non-null `file_sha256` and `signature_b64`. Atomically: set `app_releases.state='uploaded'`, `published=true`, `published_at=utcnow()`; mark any other release on the same `channel` as `published=false`; rewrite `updates/<channel>/latest.json` in Spaces (PUT with `Cache-Control: public, max-age=60`, `ACL=public-read`) from the newly-published row. Returns the freshly-written manifest JSON. |
| `POST /admin/releases/{id}/unpublish` | Set `published=false`. Find the next-newest `published=true` row on the same channel (ordered by `published_at DESC`); if one exists, rewrite the Spaces manifest to that row. If none exists, **delete** the Spaces manifest object (`delete_object`). Returns `{channel, current_version: str | null}`. This is the primary rollback path. |
| `PATCH /admin/releases/{id}` | Mutable fields: `release_notes`, `min_version`. If the release is currently published, rewrite the Spaces manifest after the update. Version/channel/engine_commit are immutable post-create. |
| `DELETE /admin/releases/{id}` | Precondition: `published=false`. CASCADE deletes `app_release_artifacts`; best-effort deletes each Spaces object for the release (swallow 404); deletes DB row. Refuse on published releases — unpublish first. |
| `GET /admin/releases?channel=&published=` | List. For the internal admin view (UI deferred; curl access is fine for alpha). |
| `POST /admin/releases/regenerate-manifest?channel=alpha` | Rewrite `updates/<channel>/latest.json` from current DB state without a publish transition. Recovery path for the case where the Spaces manifest and the DB diverge (e.g., Spaces rollback, manual tampering). |

**Manifest-rewrite helper** — a module-private function in `admin_releases.py`:

```
def _rewrite_channel_manifest(db, channel: str) -> dict | None:
    """Find the newest published release for channel, serialize to the
    manifest contract, PUT to updates/<channel>/latest.json in Spaces.
    Returns the manifest dict, or None if no published release exists
    (in which case the Spaces object is deleted)."""
```

Called from `publish`, `unpublish`, `PATCH` (when published), and `regenerate-manifest`. Uses `spaces.put_object(key, body, content_type="application/json", cache_control="public, max-age=60", acl="public-read")` — one new method added to the `spaces` wrapper from the ONNX plan.

### Spaces wrapper additions

The existing `digimon_gym/storage/spaces.py` (from `model-admin-api.md`) gains:

- `put_object(key, body, content_type, cache_control=None, acl=None)` — direct upload, used for the manifest JSON rewrite. Not presigned.

Everything else (presigned PUT, HEAD, stream_sha256, delete, public_url) is reused unchanged.

---

## Admin UI

**Deferred.** Per Q6, CI owns publish. Manual intervention (unpublish, patch notes edit, regenerate-manifest) is curl + admin token during alpha. Once alpha testers multiply or a release needs hand-holding, build `AdminReleasesPage.tsx` mirroring `AdminModelsPage.tsx` — but that's a follow-up.

The server endpoints are designed for the future UI from day one (list, patch, unpublish all exist). The `IS_DESKTOP` + `RoleGuard` tree-shake pattern (rule 13) applies when the UI lands.

---

## Tauri integration

### Plugin dependencies

Add to `src-tauri/Cargo.toml`:

- `tauri-plugin-updater = "2"` — Rust side, handles check / download / verify / apply.

Add to `frontend/package.json` (desktop entry points only; tree-shaken from web build):

- `@tauri-apps/plugin-updater` — JS binding for check / install.

### `tauri.conf.json` additions

New top-level `plugins.updater` block:

```json
{
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://<bucket>.<region>.digitaloceanspaces.com/updates/alpha/latest.json"
      ],
      "pubkey": "<ed25519-pubkey-base64>",
      "windows": {
        "installMode": "passive"
      }
    }
  }
}
```

- `endpoints` — Tauri's updater supports an array; it probes in order. For alpha we have one. The channel (`alpha`) is hardcoded at build time; flipping to `beta`/`stable` later means a second build config.
- `pubkey` — base64-encoded Ed25519 public key. Generated once, checked into source control, never rotated without a coordinated migration. See [Key custody](#security-considerations).
- `windows.installMode: "passive"` — NSIS runs with a minimal progress UI, no user input required, reboot suppressed.

The `bundle.targets` field (currently `"all"`) stays as-is on the release-build branch; CI selects per-runner with `--target` flags.

Capability file (`src-tauri/capabilities/*.json`) gains `updater:default` for the main window.

### Updater key generation + custody

One-time, done locally by the project maintainer:

```
cargo tauri signer generate -w ~/.tauri/digimon-updater.key
```

Produces:
- Private key file `~/.tauri/digimon-updater.key` (password-encrypted at generation time).
- Public key (printed to stdout; also a `.pub` sidecar).

**Custody rules** (spec'd; operationalized as a follow-up checklist):

- Private key password is stored in 1Password under "Digimon TCG — Tauri Updater Key".
- Private key file contents are copy-pasted into GitHub Actions secret `TAURI_UPDATER_PRIVATE_KEY`.
- Password is copy-pasted into GitHub Actions secret `TAURI_UPDATER_KEY_PASSWORD`.
- The local key file is kept on the maintainer's machine as a backup; it never leaves that machine in cleartext.
- The public key is pasted into `tauri.conf.json` and committed.
- **Rotation**: if the private key leaks, every tester must reinstall manually with the new pubkey baked in (Tauri cannot update past a pubkey change because the new update binary is verified against the *old* pubkey the running app knows about). Document this as "alpha key rotation is equivalent to cutting a new alpha and emailing the tester list." Not automatable.

### Startup flow in the desktop app

Pseudocode — wiring lives in `src-tauri/src/lib.rs` setup hook and a new `frontend/src/updater/` module (desktop-only, behind `IS_DESKTOP`):

1. App launches, splash screen renders.
2. **Min-version guard.** On Rust-side setup, fetch `updates/alpha/latest.json` with a 3-second timeout. Parse `min_version`. Compare against the running app's `env!("CARGO_PKG_VERSION")`. If running version < `min_version`, emit a Tauri event `updater:force-update` to the frontend, which shows a blocking modal: "This version is no longer supported. Please update." — with a single "Update now" button that invokes the normal updater install flow and exits. No way to dismiss. If the manifest fetch fails (network, Spaces down), skip the min-version check — do not block the user on a network error.
3. **Background check.** If not force-updating, kick off `tauri::updater::builder().check()` in a Tokio task. When it resolves, emit `updater:available` or `updater:none` to the frontend.
4. **UX** (per Q8A):
   - If `updater:available`: show a corner toast "Update available → click to view" (using existing toast component). Not modal.
   - On click: open a modal showing version, release notes (from manifest `notes`), and an "Install and restart" button.
   - On "Install and restart": invoke `update.download_and_install()`; updater verifies Ed25519 signature before applying; on success the app restarts into the new version.
5. **Episode-level concerns** — updater check happens once per app launch, not periodically. Rationale: alpha testers launch the app fresh when they want to play; background re-checks add no value and complicate "is a game in progress" edge-cases. A follow-up can add "check every 6 hours if the window is idle" once we're past alpha.

### Client-side engine_commit exposure

The manifest's `engine_commit` field is read by the desktop for *display only* in an "About" panel ("Alpha build 0.2.0-alpha.3, engine fbf8288"). It is not used for any gating decision in this spec. The ONNX model flow (separate) does its own engine-commit matching against models it fetches from `GET /models/manifest.json`.

---

## Release pipeline

### Tag convention

- Desktop releases: `desktop-vX.Y.Z[-alpha.N]` (e.g., `desktop-v0.2.0-alpha.3`).
- API server releases (future, noted here for consistency): `api-vX.Y.Z`.
- Engine releases (future): `engine-vX.Y.Z`.

Only tags matching `refs/tags/desktop-v*` trigger the desktop workflow. This keeps the desktop pipeline disjoint from the API-server pipeline the user plans to add; both can live in `.github/workflows/` without cross-triggering.

### GitHub Actions workflow shape

New file: `.github/workflows/desktop-release.yml`. Trigger: `on: push: tags: ['desktop-v*']`.

Single workflow with a matrix over `windows-latest` and `ubuntu-latest`. High-level shape:

**Job 1: `build` (matrix over windows/linux)**
1. Checkout with submodules (DCGO not needed; engine is in-tree).
2. Setup Rust toolchain, Node.
3. `npm ci` in `frontend/`.
4. `VITE_BUILD_TARGET=desktop npm run build` in `frontend/`.
5. `cargo tauri build --target <runner-specific>` in `src-tauri/`.
   - Env: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` set from GHA secrets `TAURI_UPDATER_PRIVATE_KEY` / `TAURI_UPDATER_KEY_PASSWORD`. Tauri's build step signs each bundle artifact with these and emits `.sig` files next to the installer.
6. Capture artifacts:
   - Windows: `src-tauri/target/release/bundle/nsis/*-setup.exe` + `.sig`
   - Linux: `src-tauri/target/release/bundle/appimage/*.AppImage` + `.sig`
7. Upload as GHA artifacts for job 2.

**Job 2: `publish` (needs: build, runs on ubuntu-latest)**
1. Download artifacts from job 1.
2. Parse version from `${{ github.ref_name }}` (strip `desktop-v` prefix).
3. Read engine commit from `digimon-engine/Cargo.toml` via `git rev-parse HEAD -- digimon-engine/` (short form, 7 chars).
4. Resolve `min_version` and `release_notes` from tag annotations or a `RELEASE_NOTES.md` file at the tag (TBD in implementation — pick whichever ergonomics test better; spec stays agnostic).
5. `POST /admin/releases` with the hosted API (base URL from secret `HOSTED_API_URL`, auth from secret `CI_ADMIN_TOKEN`). Receive `{release_id, artifacts: [...]}`.
6. For each artifact (windows, linux):
   - `curl -X PUT --data-binary @<installer> <presigned_url>` — upload to Spaces.
   - Read `.sig` file contents (base64 already, per Tauri's signer output).
   - `POST /admin/releases/{id}/artifacts/<target>/confirm` with `{signature_b64}`.
7. `POST /admin/releases/{id}/publish`. Server regenerates the Spaces manifest.
8. Post a GitHub release note to the tag via `gh release create` with the same release notes + artifact download links (for people who want to manually install / sideload).

**Secrets required (one-time setup):**

- `TAURI_UPDATER_PRIVATE_KEY` — Ed25519 private key file contents.
- `TAURI_UPDATER_KEY_PASSWORD` — password for the above.
- `SPACES_KEY` / `SPACES_SECRET` — not needed in CI if the hosted API generates presigned URLs; CI only needs HTTP access to the hosted API. Decision: **CI does not get Spaces credentials directly.** It uses presigned PUTs from the hosted API, same as the model upload flow. Enforces least-privilege.
- `HOSTED_API_URL` — production hosted API base URL.
- `CI_ADMIN_TOKEN` — long-lived JWT for a dedicated CI admin user (`ci-desktop-release`). Provisioned manually once; rotated if leaked. Scoped to `ROLE_ADMIN` because `/admin/releases/*` requires it; a future `ROLE_RELEASE_PUBLISHER` narrower role is a follow-up.

### Failure modes

- Build fails on one runner (e.g., Windows) but succeeds on the other. Job 2 doesn't run because `needs: build` waits for all matrix jobs. No partial release. Maintainer fixes + retags (e.g., `desktop-v0.2.0-alpha.4`).
- Upload succeeds on one platform, fails on another during job 2. DB row exists, one artifact confirmed, `/publish` call fails precondition (not all artifacts confirmed). Maintainer either retries job 2, or calls `DELETE /admin/releases/{id}` via curl and retags.
- `/publish` succeeds but manifest rewrite to Spaces fails partway. Server logs the error and returns 500; DB still marks `published=true` but Spaces manifest is stale. Recovery: `POST /admin/releases/regenerate-manifest?channel=alpha`.

---

## Rollback + kill-switch

### Primary rollback (unpublish)

A tester reports the current alpha is broken. Maintainer runs:

```
curl -X POST "$HOSTED_API_URL/admin/releases/$RELEASE_ID/unpublish" \
  -H "Authorization: Bearer $CI_ADMIN_TOKEN"
```

Server:
1. Sets `published=false` on the row.
2. Queries `SELECT id FROM app_releases WHERE channel='alpha' AND published=true ORDER BY published_at DESC LIMIT 1`.
3. If a row exists: rewrites `updates/alpha/latest.json` from that row.
4. If no row exists (first release was the broken one): deletes the Spaces manifest object.

Testers running the broken build: next launch, updater check either sees the older version as "newer" (because `max_version` has regressed — Tauri's updater *does* allow forward-to-previous if the manifest advertises an older version number, since all it compares is `manifest.version > running.version`...) — wait, Tauri's default behavior is `manifest.version > running.version`, so this scenario requires the older release to have a version *greater* than... hm.

**Corner case resolution:** Tauri's updater applies an update when `manifest.version != running.version` *and* the signature verifies and `manifest.version > running.version` per SemVer. So a straight "republish the older version as-is" won't trigger an update from the running broken build. **Workflow in practice:** after unpublishing the broken release, immediately cut a new release with a fresh version number that is the older good build's code rebuilt with a higher version string (e.g., if good was `0.2.0-alpha.2` and broken was `0.2.0-alpha.3`, the rollback is `0.2.0-alpha.4` built from the same commit as `-alpha.2`). CI workflow supports this: push `desktop-v0.2.0-alpha.4` pointing at the good commit, normal pipeline runs, testers auto-update forward to the good code.

This means **unpublish alone doesn't help running testers**; it just stops *new* installs from getting the broken version. Unpublish is correct for "stop the bleeding"; the actual rollback-for-running-testers is "cut a new version from the last good commit." Spec this in the runbook section of an implementation follow-up.

### Kill-switch (nuclear)

If the current broken build is so broken it can't run its updater (e.g., crash on launch before updater check, or updater code itself is the bug), the min-version guard is the escape hatch.

Maintainer: `PATCH /admin/releases/{id_of_broken_release} {"min_version": "<version_higher_than_broken>"}` followed by a new CI release at that higher version.

Since the manifest is already advertising the broken release and the min-version guard runs **before** the updater check, a tester launching the broken build sees:
1. Rust setup hook fetches manifest.
2. `manifest.min_version = 0.2.0-alpha.4` > running `0.2.0-alpha.3`.
3. Emit `updater:force-update`; frontend shows blocking modal.
4. User clicks "Update now"; Tauri updater downloads+verifies+applies the manifest's current `version` (which must be a higher good build).

Min-version can only be bumped *after* a good build is available; bumping it without a working newer version just bricks the tester. Spec'd as "always cut the good version first, then PATCH min_version up."

---

## Security considerations

| Threat | Mitigation |
|---|---|
| Attacker tampers with Spaces manifest (e.g., compromised Spaces key). | Tauri updater verifies Ed25519 signature on the installer *before* applying. A tampered manifest pointing at a malicious installer fails signature check; updater refuses to install. Pubkey is compile-baked into the running app. Attacker would need the private key to produce a valid signature. |
| Attacker tampers with the installer binary in Spaces. | Same. Signature is over the bytes. |
| Private key leak. | Rotation requires a new alpha email-out (see [Key custody](#updater-key-generation--custody)). Mitigation: secret lives only in GHA + maintainer 1Password; never on CI runners' filesystem after job completion (GHA scrubs). No `cargo tauri signer sign` runs outside CI for releases. |
| Manifest hosted over HTTP. | Not done. Spaces URLs are HTTPS-only. `endpoints` in `tauri.conf.json` are `https://` — Tauri rejects `http://` for updater endpoints by default. |
| CI admin token leak. | Token scoped to admin role but can only call `/admin/releases/*` effectively (other admin endpoints exist but don't matter for this surface). Rotation: revoke via DB (`UPDATE users SET token_version = token_version + 1 WHERE username = 'ci-desktop-release'`) — existing JWT auth pattern invalidates old tokens on version bump. Follow-up: narrower `ROLE_RELEASE_PUBLISHER` scope. |
| Spaces credentials in CI. | **Avoided by design.** CI uses presigned PUTs from the hosted API; never receives SPACES_KEY / SPACES_SECRET. Least-privilege. |
| Malicious self-signed Windows installer (from Q3). | Out of scope — this is an alpha UX trade-off, not a supply-chain issue. Testers are trusted; SmartScreen warning is documented. The Ed25519 signature over the installer still protects the *update path* even with a self-signed installer cert. |
| Downgrade attack (attacker pins testers to an older vulnerable version). | `min_version` guard prevents running below floor regardless of what manifest advertises as `version`. Attacker who can rewrite the manifest can also rewrite `min_version`, but cannot forge signatures — the worst they achieve is a denial-of-service (refuse to let testers run), not an exploit. |
| Manifest cached too long on CDN. | Spaces objects have `Cache-Control: public, max-age=60`. One-minute window between publish and testers seeing it. Acceptable for alpha. |
| DB / Spaces drift. | `POST /admin/releases/regenerate-manifest` is the recovery path. Admin can always force the Spaces manifest to match current DB state. |

---

## Open questions / follow-ups

1. **Release notes source.** Tag annotation (`git tag -a -m "..."`) vs. `RELEASE_NOTES.md` at tag vs. PR body via `gh pr view`. Pick during implementation based on which is easiest to author without breaking CI when authors forget.
2. **Admin UI (`AdminReleasesPage.tsx`).** Deferred; spec server endpoints are already shaped for it.
3. **Windows OV cert migration path.** When alpha tester count crosses ~50 or we open public beta, buy an OV cert from Sectigo/DigiCert. `cargo tauri build` can call `signtool.exe` via a post-bundle hook. No schema changes needed; installer just gets a second signature layer on top of the Tauri Ed25519 one. Document the cert purchase + CA setup in a follow-up runbook.
4. **macOS inclusion.** When/if we pay for Apple Developer ID ($99/yr), add `macos-x86_64` and `macos-aarch64` targets to the manifest `platforms` object, add a macOS runner to the GHA matrix, and add a notarization step (`xcrun notarytool submit`) before the upload step.
5. **Narrower CI role (`ROLE_RELEASE_PUBLISHER`).** Instead of `ROLE_ADMIN`, create a scoped role that can only hit `/admin/releases/*`. Reduces blast radius if the CI token leaks.
6. **Periodic update check after launch.** Spec is "once per launch." If alpha testers keep the app open for long sessions, add a 6-hour idle-window re-check. Trivially additive.
7. **Update telemetry.** Beacon back to the hosted API when a desktop finishes applying an update, so admin can see "N testers now on 0.2.0-alpha.3." Probably a separate endpoint (`POST /admin/releases/{id}/telemetry/applied`). Out of scope for this spec.
8. **Differential updates.** Tauri v2 supports binary-patch updates experimentally. Not worth it for alpha (installers are small, full re-downloads are fine). Revisit if installer size crosses ~100MB.
9. **Staged rollouts.** Publish to 10% of testers first, then 100% if no crashes. Would need a per-tester stable ID + a `rollout_percent` field. Out of scope; alpha is small enough to not need it.
10. **Runbook document.** This spec describes the system; the actual "how to cut a release" / "how to roll back" / "how to rotate keys" operational doc is a separate follow-up under `docs/runbooks/`.

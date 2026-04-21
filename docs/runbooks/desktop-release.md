# Desktop Release Runbook

How to cut, publish, and roll back a Tauri v2 desktop alpha release. The auto-updater design spec lives at [`docs/superpowers/specs/2026-04-21-tauri-auto-updater.md`](../superpowers/specs/2026-04-21-tauri-auto-updater.md).

---

## Updater key custody

- Private key file: `$HOME/.tauri/digimon-updater.key` (password-encrypted, maintainer machine only).
- Password: 1Password → "Digimon TCG" → "Tauri Updater Key".
- Public key: committed in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.
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

# 4. Paste the printed public key into src-tauri/tauri.conf.json
#    plugins.updater.pubkey — commit that change

# 5. Provision the CI admin user + long-lived token
$PASS = -join ((33..126) | Get-Random -Count 32 | ForEach-Object {[char]$_})
python tools/provision_ci_release_user.py --password $PASS | gh secret set CI_ADMIN_TOKEN
Remove-Variable PASS
```

The Spaces URL in `plugins.updater.endpoints` must match where the server writes the manifest (see `digimon_gym/storage/spaces.py:public_url()` — which prefers `SPACES_CDN_URL` on the server, else `SPACES_ENDPOINT/<bucket>/`). The default configured is `https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com/updates/alpha/latest.json` — adjust if your bucket host differs.

---

## Cut a new release

1. Ensure `main` is green and the change you want to ship is merged.
2. Bump the version in both files (they must stay in sync):
   - `src-tauri/tauri.conf.json` — the `version` field at the top
   - `src-tauri/Cargo.toml` — the `[package].version` field
3. Commit the version bump.
4. Create an annotated tag with release notes in the body. The body becomes the update-modal text your testers see.
   ```bash
   git tag -a desktop-v0.2.0-alpha.3 -m "Fix: deckbuilder crash on whitespace import.
   Add: Beelzemon gauntlet preset."
   git push origin desktop-v0.2.0-alpha.3
   ```
5. Watch CI: `gh run watch`. Both build jobs (Windows + Linux) must succeed, then the `publish` job calls the hosted API to create/upload/confirm/publish in sequence.
6. Verify by refetching the manifest:
   ```bash
   curl https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com/updates/alpha/latest.json | jq
   ```
   Should show the new version, both `platforms` populated, and `pub_date` within the last few minutes. `Cache-Control: public, max-age=60` header is set; propagation ≤ 60 seconds.
7. Verify a running tester sees the update. Easiest check: launch the previously-installed alpha on your own machine, confirm the "Update available" toast appears within ~5s of launch, click through to install.

---

## Roll back a broken release

### Scenario A: broken build still launches

Testers can self-recover by updating forward. The rollback is "cut a new release from the last good commit":

1. `git checkout <last-good-commit>`
2. Bump `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` to a version strictly greater than the broken one (e.g., broken `0.2.0-alpha.3` → cut `0.2.0-alpha.4`).
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
2. Update `src-tauri/tauri.conf.json`'s `plugins.updater.pubkey`.
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
| CI publish step 401s | `CI_ADMIN_TOKEN` expired (annual) or user revoked | Re-run `python tools/provision_ci_release_user.py --password "$(openssl rand -base64 32)" \| gh secret set CI_ADMIN_TOKEN` |
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

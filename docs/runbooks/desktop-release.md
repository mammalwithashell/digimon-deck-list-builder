# Desktop Release Runbook

> Scaffolding only — the "Cut a release" and "Roll back" sections are filled in by Task 16.

## Updater key custody

- Private key file: `~/.tauri/digimon-updater.key` (password-encrypted, lives on maintainer machine only).
- Password: stored in 1Password → "Digimon TCG" → "Tauri Updater Key".
- Public key: committed in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.
- GitHub Actions secrets:
  - `TAURI_UPDATER_PRIVATE_KEY` — file contents
  - `TAURI_UPDATER_KEY_PASSWORD` — key passphrase
  - `HOSTED_API_URL` — production hosted API base URL
  - `CI_ADMIN_TOKEN` — provisioned by `tools/provision_ci_release_user.py` (Task 13)

## Key generation (one-time, already done)

```powershell
cargo install tauri-cli --version "^2" --locked
cargo tauri signer generate -w $HOME\.tauri\digimon-updater.key

# Upload secrets
Get-Content -Raw $HOME\.tauri\digimon-updater.key | gh secret set TAURI_UPDATER_PRIVATE_KEY
gh secret set TAURI_UPDATER_KEY_PASSWORD          # interactive paste
gh secret set HOSTED_API_URL --body "https://api.digimon-tcg.example.com"
```

Note: on Windows PowerShell, `~` in command-line paths passed to native binaries is taken literally — use `$HOME` instead.

## Key rotation

Rotation bricks every already-installed tester's auto-update path (Tauri verifies against the baked-in pubkey). Do not rotate casually. Procedure if the key leaks:

1. Generate new key: `cargo tauri signer generate -w $HOME\.tauri\digimon-updater-v2.key`.
2. Update `src-tauri/tauri.conf.json`'s `plugins.updater.pubkey`.
3. Update GHA secrets `TAURI_UPDATER_PRIVATE_KEY` + `TAURI_UPDATER_KEY_PASSWORD`.
4. Bump major-ish version so SemVer clearly distinguishes (e.g. `0.3.0-alpha.1`).
5. Cut a release via the normal flow.
6. Email alpha tester list: "Please download the new installer manually from `<GitHub release URL>`. Auto-update will not work across this version."

## Cut a new release

_(Filled in by Task 16.)_

## Roll back a broken release

_(Filled in by Task 16.)_

---
name: cut-desktop-release
description: Cut and publish a Tauri desktop ALPHA release of the Digimon TCG app — bump the version, tag, and push so the desktop-release CI builds the Windows+Linux installers and publishes the auto-updater manifest. Use this WHENEVER the user wants to ship/cut/publish a desktop release, "update the alpha client", push a new desktop build to testers, release the Tauri app, bump the desktop version for a release, or kick off the desktop-release workflow — even if they don't name the version. This is the verified release recipe (matches docs/runbooks/desktop-release.md); prefer it over hand-running git/gh commands, because a release that skips the version bump or uses a lightweight tag ships a broken or mislabeled update to real testers. NOT for the hosted API image (build-api-image.yml) or the landing page.
---

# Cut a desktop alpha release

Ship a new version of the Tauri desktop app to alpha testers via the auto-updater.

**The whole release is driven by one thing: pushing an annotated `desktop-vX.Y.Z` git tag.** That fires [`.github/workflows/desktop-release.yml`](../../../.github/workflows/desktop-release.yml), which builds + signs the Windows and Linux installers, uploads them to Spaces, and publishes the updater manifest to the **alpha** channel. Installed clients see an "Update available" toast on next launch.

**Why the version bump is non-negotiable.** The built app reports its own version from `Cargo.toml`/`tauri.conf.json` (compile-baked), while the manifest version comes from the tag. The updater only offers an update when the manifest version is *strictly greater* than the running app's. If you tag `desktop-v0.3.3` but leave the app at `0.3.2`, the new build still reports `0.3.2` → testers either never get the update or get stuck in an update loop (download → still 0.3.2 → offered again). So the tag version and the in-app version MUST match, which means bumping the version files in the same commit you tag.

This is an **outward-facing, hard-to-reverse** action (it publishes to real testers). Confirm the version and release notes with the user before pushing the tag. Rollback is "cut a newer release" + unpublish — see the runbook.

## Preconditions

- `main` is green and the change(s) you want to ship are **merged to main**. The release builds from the tagged commit, so cut from `main`'s tip, not a feature branch.
- `gh` is authenticated and you can push to `main` + tags (the bump commit goes to `main`; if branch protection blocks a direct push, open a tiny version-bump PR, merge it, then tag the merge commit).
- You're cutting the **alpha** channel (the only active channel). Channel is compile-baked; you can't switch a tester mid-install.

## Steps

### 1. Sync to main's tip and pick the version

```bash
git fetch origin main
git checkout -b release/desktop-vX.Y.Z origin/main   # cut from main, not a feature branch
git tag --list 'desktop-v*' --sort=-creatordate | head -3   # see the latest released version
```

Pick the next version by SemVer from the latest `desktop-v*` tag: **patch** (`0.3.2 → 0.3.3`) for bugfixes/small features, **minor** for larger features. Alpha prereleases use suffixes (`-alpha.N`, ordered correctly by the updater's semver check). Confirm the choice with the user.

### 2. Refresh the implemented-cards allowlist (so newly-implemented cards actually ship)

The deck builder's import gate rejects any card not in `data/tested_cards.json` ("not available in the alpha release"). That snapshot is **`include_str!`-baked into the desktop binary at compile time** (`code/digimon-engine/src/deck_tools.rs`), so any card implemented since the last snapshot stays import-blocked in the shipped build until the snapshot is regenerated **and committed** (the release builds from the tagged commit). Regenerate it from the live engine pool:

```bash
python code/tools/build_tested_cards.py   # queries the engine's registered-effect pool, rewrites data/tested_cards.json
```

This builds `digimon-engine-cli` once (a few minutes) and rewrites the file only if the pool changed — it prints `wrote data/tested_cards.json (N implemented cards)` if cards were added, or `no changes` if the snapshot was already fresh. Leave any change **staged**; it ships in the bump commit below (step 4). Don't regenerate it in CI-only — the same committed file feeds the hosted API and browser builds, so it must be the single committed source of truth.

### 3. Bump the version in all three files (they must stay in sync)

`code/src-tauri/tauri.conf.json`, `code/src-tauri/Cargo.toml`, and the root `Cargo.lock` (`digimon-tcg` entry) all carry the version. Use the helper so the `Cargo.lock` edit hits only the `digimon-tcg` package:

```bash
python .claude/skills/cut-desktop-release/scripts/bump_version.py X.Y.Z
```

It prints the old→new change for each file. Verify with `git diff`.

### 4. Commit the bump (+ refreshed allowlist) to main

Use the established commit-message convention (matches prior bumps). Stage the allowlist alongside the version files so the refreshed snapshot lands in the same tagged commit:

```bash
git add code/src-tauri/tauri.conf.json code/src-tauri/Cargo.toml Cargo.lock data/tested_cards.json
git commit -m "Bump desktop app version to X.Y.Z for the desktop-vX.Y.Z release

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
git push origin HEAD:main
```

If the push is rejected by branch protection, push the branch and open a PR for just the bump, merge it, then continue from the merged commit.

### 5. Create an annotated tag with plain-text release notes, and push it

The tag **must be annotated** (`git tag -a`) — the publish job re-fetches the tag object and reads its body as the update-modal notes. A lightweight tag falls back to a template and can ship a merge-commit message as the notes (this actually happened on 0.1.0).

Write the notes as **plain text, one fact per line, no markdown** — Tauri v2 renders them verbatim in the update modal, so `#`/`*`/backticks show up literally. Lead with user-facing Fix:/New: lines.

```bash
cat > /tmp/desktop-release-notes.txt <<'EOF'
Fix: <one user-facing fix per line>.
New: <one user-facing addition per line>.
EOF
git tag -a desktop-vX.Y.Z -F /tmp/desktop-release-notes.txt
git cat-file -t desktop-vX.Y.Z      # MUST print "tag" (annotated), not "commit"
git push origin desktop-vX.Y.Z
```

Derive the notes from what shipped since the last tag (`git log --oneline desktop-v<prev>..HEAD`), but phrase them for testers, not as commit subjects.

### 6. Watch the build + publish

```bash
gh run list --workflow=desktop-release.yml --limit 1
gh run watch <run-id>      # or open the Actions URL
```

Both build legs (Windows + Linux) must go green, then the `publish` job creates → uploads → confirms → publishes to the hosted API. (`workflow_dispatch` and `desktop-ci-*` tags build but do NOT publish — only `desktop-v*` publishes.)

### 7. Verify the release reached testers

```bash
curl -s https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com/updates/alpha/latest.json | jq '{version, pub_date, platforms: (.platforms | keys)}'
```

Confirm `version` is the new one, both `platforms` are present, and `pub_date` is recent (CDN propagation ≤ 60s). For the full smoke test, launch the previously-installed alpha and confirm the update toast appears.

## Gotchas (each looks minor, each breaks a release)

- **Skipped the allowlist refresh (step 2)** → cards implemented since the last release are import-blocked in the shipped build ("not available in the alpha release"), because `data/tested_cards.json` is compiled into the binary. Always regenerate + commit it before tagging.
- **Lightweight tag** → notes show a merge-commit message. Always `git tag -a`; verify `git cat-file -t` prints `tag`.
- **Version files out of sync** (or `Cargo.lock` missed) → updater confusion / loop. The helper script keeps all three aligned.
- **Markdown in notes** → renders literally. Plain text only.
- **Tagged a feature branch / pre-merge commit** → ships unmerged or partial code. Cut from `main`'s tip.
- **Publish job 401** → `CI_ADMIN_TOKEN` expired; **404 on `/admin/releases`** → droplet running a stale API image. Both are in the runbook's "Common issues".

## Rollback

If a release is bad: cut a strictly-greater version from the last good commit (testers update forward), and `unpublish` the broken one; if it crashes before the updater runs, use the `min_version` kill-switch. Full procedures (with the exact `/admin/releases` curls) are in the runbook.

## After publishing

Consider running the `update-landing-screenshots` skill so the landing-page
gallery reflects this build's UI. It launches the real desktop app, recaptures
each mainstay page in both themes, and republishes the gallery.

## Reference

`docs/runbooks/desktop-release.md` is the authoritative source — the tag/version/notes contracts, the manifest shape, CI secrets, rollback, and key rotation all live there. This skill is the happy-path recipe; consult the runbook for anything off the path.

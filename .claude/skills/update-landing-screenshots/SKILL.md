---
name: update-landing-screenshots
description: Recapture the desktop-app screenshots on the landing page and republish them. Launches the REAL Tauri desktop app (not a browser — backend-fed pages like decks/models/a live game only populate in the real app), drives it through the mainstay pages in both light and dark themes via the dev-only debug-bridge navigate hook, captures each window with PrintWindow, writes the WebP assets under code/landing/assets/screenshots/, and commits + pushes (triggering the landing-page Pages deploy). Use WHENEVER the user wants to update / refresh / regenerate the landing-page screenshots or gallery, recapture the desktop app shots, or "take new screenshots for the site" — and as a companion to cut-desktop-release so the gallery tracks the shipped build. NOT for the hosted API or the desktop release itself.
---

# Update the landing-page screenshots

Companion to `cut-desktop-release`. Recaptures the gallery in `code/landing/`
from the **real** desktop app. The capture is fully scriptable (PowerShell
`PrintWindow`); navigation + theme are driven through the dev-only debug bridge
(`/navigate`). This is an **outward-facing** action — the final step pushes to
`main` and publishes to the live site.

## Pages captured (5 × 2 themes = 10 WebP)

| asset stem | route | notes |
|---|---|---|
| `launcher` | `/` | front door, populated decks |
| `game-board` | `/game/rust-local` | **stage `fixtures/hero-board.json` first** |
| `deck-builder` | `/deckbuilder/new` | |
| `deck-library` | `/deckbuilder` | |
| `ai-models` | `/models` | desktop-only (Tauri invoke) |

## Recipe

### 1. Preconditions
- On `main`'s tip (assets should reflect the shipped build); `gh` authenticated.
- Frontend deps present: `test -d code/frontend/node_modules || (cd code/frontend && npm install)`.
- Free the `src-tauri` build lock (no other `cargo tauri dev` running).

### 2. Launch the app with the bridge (reuse `run-desktop`)
```bash
cd code/frontend && npm run dev:desktop      # background; wait for :5173 LISTEN
cd code/src-tauri && DIGIMON_DEBUG_BRIDGE=1 cargo tauri dev --features debug-bridge \
  --config '{"build":{"beforeDevCommand":""}}'   # background; wait for the window
```
Bridge port: read `~/AppData/Roaming/digimon-tcg/debug_bridge.json` (default 5174).

### 3. Capture loop
For `theme` in `dark`, `light`; for each page in the table:
```bash
# menu pages:
curl -s -X POST http://127.0.0.1:$PORT/navigate -H 'Content-Type: application/json' \
  -d "{\"route\":\"$ROUTE\",\"theme\":\"$THEME\"}"
sleep 1
# game board ONLY (stage before navigating so GamePage's mount-fetch finds it):
curl -s -X POST http://127.0.0.1:$PORT/stage -H 'Content-Type: application/json' \
  --data-binary @.claude/skills/update-landing-screenshots/fixtures/hero-board.json
curl -s -X POST http://127.0.0.1:$PORT/navigate -d "{\"route\":\"/game/rust-local\",\"theme\":\"$THEME\"}" -H 'Content-Type: application/json'
sleep 1
# capture + convert:
powershell -File .claude/skills/update-landing-screenshots/scripts/capture_window.ps1 -OutPath "$TMP/$STEM-$THEME.png"
python .claude/skills/update-landing-screenshots/scripts/to_webp.py "$TMP/$STEM-$THEME.png" \
  "code/landing/assets/screenshots/$STEM-$THEME.webp" --width 960
```
**`Read` each WebP** and confirm the right page + theme + a clean crop before moving on (the no-approximations habit: verify, don't assume). Re-capture any that are wrong (wrong theme = settle longer; bad crop = pass `--crop L T R B`).

### 4. Wire `index.html` (first run only)
The gallery section references the fixed asset paths above. If it's absent
(first run), add it per the plan/Task 8. On recaptures, only the WebPs change.

### 5. Publish
```bash
git add code/landing/assets/screenshots/*.webp code/landing/index.html
git commit -m "chore(landing): refresh desktop screenshots"
git push origin HEAD:main      # triggers .github/workflows/landing-page.yml
```
Confirm the deploy: `gh run list --workflow=landing-page.yml --limit 1`.

### 6. Teardown
Stop `cargo tauri dev` + the dev server.

## Gotchas
- **Browser ≠ desktop:** a Playwright shot of `:5173` renders the right chrome but empty backend data (decks/models) and can't show invoke-only pages. Always use the real app.
- **Dev server must serve the right frontend:** start `dev:desktop` from the checkout whose frontend you're shipping. In a worktree, junction the base `node_modules` in first (`New-Item -ItemType Junction`), or the dev server serves the wrong tree.
- **Game board shows the setup form:** `/stage` must precede the `/game/rust-local` navigate; add a settle if needed.
- **Native title bar in the shot:** `capture_window.ps1` already crops to the client area; if a sliver remains, pass `--crop` to `to_webp.py`.
- **Stale exe:** never screenshot a prebuilt `digimon-tcg.exe` — it may bundle an old web dist. Always launch via `cargo tauri dev` so it loads the current `:5173` desktop frontend.
- **Stack overflow during `cargo tauri dev` build/test:** set `RUST_MIN_STACK=268435456` (deep engine calls overflow the default stack in this env).

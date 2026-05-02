# Alpha Desktop Release — Smoke Checklist

Run this checklist on a clean machine before shipping the alpha Tauri
installer. Every item must pass, or the failure must be filed as a
blocker before release.

## Build verification

- [ ] `cd src-tauri && cargo tauri build` produces an installer without errors
- [ ] `cargo test --manifest-path code/src-tauri/Cargo.toml deck_storage` passes (5 tests)
- [ ] `cargo test --manifest-path code/digimon-engine/Cargo.toml` passes (engine regression)
- [ ] `cd frontend && npx tsc --noEmit` is clean
- [ ] `cd frontend && npx vitest run` passes (currently: `guest.test.ts`, 4 tests)
- [ ] `python -m pytest tests/api/ -v` passes (backend regression; run only when backend changes land)
- [ ] `JWT_SECRET_KEY` env var is set on the production server (not the default dev key)
- [ ] `code/frontend/.env.desktop` has real URLs (not `api.digimon-tcg.example`)

## First launch (clean app-data dir)

On Windows: delete `%APPDATA%/com.digimon-tcg.desktop/`
On macOS: delete `~/Library/Application Support/com.digimon-tcg.desktop/`
On Linux: delete `~/.local/share/com.digimon-tcg.desktop/`

- [ ] App launches and lands on HomePage
- [ ] AlphaBanner is visible at the top of Home
- [ ] Side nav shows: Home, Lobby, Deck Builder, AI Models, Patch Notes (no admin/training entries)
- [ ] `localStorage.guest_access_token`, `guest_user_id`, `guest_display_name` are populated (open devtools)
- [ ] Navigating to `/lobby`, `/deckbuilder`, `/models` works without a redirect to `/login`
- [ ] UserMenu (top right) shows the guest display name with a Logout button, NOT a Log In button

## Offline boot (guest mint fails)

Disable network, delete app-data, relaunch:

- [ ] App still opens without crashing
- [ ] Home renders (guest mint failed — check devtools for the error)
- [ ] AI Models page renders the "No decks yet" empty state and any previously downloaded models
- [ ] "Find Match" click does NOT crash (may show an error when the queue POST fails)

Re-enable network:

- [ ] Relaunching online mints a fresh guest (new user_id, new decks storage — this is expected data-loss-on-offline-first-launch)

## Deck Builder

- [ ] Open `/deckbuilder` — no format picker visible (standard is hardcoded)
- [ ] Build a valid standard deck (50 main + 5 egg using cards with behavioral tests)
- [ ] Save the deck — returns to the deck list, the new deck is visible
- [ ] Re-open Deck Builder — the saved deck is listed
- [ ] Edit the deck name, save again — `updated_at` refreshes
- [ ] Delete the deck — removed from list, not recoverable

## AI Models

Requires a reachable API server with at least one published model in the DO Spaces manifest.

- [ ] Manifest rows render (name, type, size column populated)
- [ ] Engine-compatible models are clickable; incompatible rows are greyed out with "incompatible" badge
- [ ] "Try online" on an undownloaded row:
  - [ ] Button label flips to "Preparing…" during the call
  - [ ] Navigates to `/game/<id>?mode=vsai&player=1`
  - [ ] A game starts (state streams in via WebSocket)
  - [ ] The opponent makes moves autonomously
- [ ] "Download" on a manifest row:
  - [ ] Downloads the ONNX file (progress visible or at least button disables)
  - [ ] After success, row state flips to "downloaded"
  - [ ] "Activate" button now available
- [ ] "Activate" a downloaded model — no visible error; model is now loaded for offline play
- [ ] "Delete" a downloaded model — row reverts to manifest-only state

## Matchmaking (requires a second client)

Start a second build (same or different machine) and sign in as a different guest:

- [ ] From both clients, navigate to `/lobby` → Play tab
- [ ] Pick a deck in each, click "Find Match" on casual queue
- [ ] Both tickets pair within a few seconds; both clients navigate to `/game/<id>?mode=pvp`
- [ ] The game renders on both sides; actions from one client appear on the other via WebSocket
- [ ] Cancel a ticket on one client before pairing — UI returns to the queue entry screen; backend ticket is removed

Create + Join-by-code flow:

- [ ] Client A navigates to Create tab, picks a deck, clicks Create — shows a 6-character join code
- [ ] Client B enters the code on the Join tab, picks a deck, clicks Join — both navigate to the game
- [ ] Browse tab is NOT visible (alpha cut)

## Known limitations to confirm (not bugs)

- [ ] Decks do not sync between two desktop installs of the same guest (expected: client-local storage)
- [ ] Losing localStorage (manual clear, uninstall) drops the guest identity (expected: no server anchor)
- [ ] Admin UI is absent from the nav and from desktop build bundle (expected: tree-shaken)
- [ ] Ranked queue works but rating is not displayed in UI (deferred to post-alpha)
- [ ] No replay/spectator flows (deferred to post-alpha)

## Signoff

| Item | Owner | Date | Notes |
| --- | --- | --- | --- |
| Build artifacts produced | | | |
| All checklist items green | | | |
| Blockers filed | | | |
| Installer signed | | | |
| Ship decision | | | |

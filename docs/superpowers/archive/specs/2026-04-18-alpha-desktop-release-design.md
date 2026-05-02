# Alpha desktop release — design

Date: 2026-04-18
Status: design, not yet implemented
Scope: frontend + small backend glue for the alpha desktop release

## Goal

Ship a Tauri desktop build that lets anonymous users:

1. Play **PvP** via the existing matchmaking queue.
2. Play **vs AI online** — the hosted FastAPI runs the Rust engine + ONNX
   inference server-side, so users can try a model without downloading it.
3. Play **vs AI offline** — download an ONNX model from the DO-Spaces-backed
   manifest and run inference locally via the Rust engine embedded in Tauri.
4. Build **standard-format** decks.

No accounts for this release. Identity is a short-lived guest JWT minted on
first launch.

Card-script completeness is explicitly out of scope for alpha — whatever the
current Rust engine supports is what ships.

## Non-goals

- Web app as a release target (hosted web build can stay buildable; it is
  not distributed).
- Accounts, login/register, persistent rating, profile pages.
- Replays, spectator mode, public lobby browse.
- Admin / training / arena / gauntlet UI (already tree-shaken out of
  `VITE_BUILD_TARGET=desktop`).

## Background — what is already built

- **Matchmaking backend** (casual + ranked): `digimon_gym/routers/matchmaking.py`,
  `digimon_gym/routers/ws_matchmaking.py`. Tier filters, rating windows,
  lobby handoff.
- **Lobby frontend**: `frontend/src/pages/LobbyPage.tsx` with Play / Create /
  Join / Browse tabs.
- **Deck meta-tier classifier** with tier badges in the queue UI.
- **Desktop ONNX download flow**: `frontend/src/pages/ModelsPage.tsx` +
  `frontend/src/api/desktopModelsApi.ts` + `src-tauri/src/models.rs`.
  Manifest fetch, SHA-verified download, local cache, activate/delete.
- **Admin model upload** → DO Spaces via presigned PUT (`digimon_gym/storage/spaces.py`,
  `digimon_gym/db/routers/admin_models.py`). Files live in Spaces; FastAPI
  serves `/models/manifest.json`.
- **Server-side game creation with an ONNX policy**: `POST /games` in
  `digimon_gym/routers/games.py` already supports `player2_policy=trained`
  with a local model path.
- **Deck builder gated to cards with behavioral tests** (#318), defaults to
  `game_mode: 'standard'`.

## Overall approach — model-centric UI

The Models page becomes the single hub for AI opponents. Each manifest
entry renders a row whose buttons depend on its state:

| State | Buttons |
| --- | --- |
| Not downloaded | Try online · Download |
| Downloaded | Try online · Play offline · Delete |
| Active (loaded for offline play) | Active badge · Try online · Play offline · Delete |

Home surfaces: Find Match · Play vs AI · Deck Builder · Models · Patch Notes.

Rejected alternatives:

- *Split "Play Online" / "Play Offline" entry points.* Fragments the
  try-before-you-download story; more clicks.
- *Automatic online/offline dispatch based on connectivity.* Cute but
  surprising — users should see which path the game is using.

## In scope

- Tauri desktop build as the only release artifact.
- Anonymous guest identity (`POST /auth/guest`) minted on first launch,
  stored in `localStorage`. Token is long-lived; no refresh flow needed.
- Deck lists live **client-side** in Tauri `app_data_dir`, not on the
  server. Matchmaking and server-side games take the deck as an inline
  payload.
- PvP via matchmaking (casual + ranked UI survives; ranked rating display is
  hidden until accounts land).
- Vs-AI online — server-side inference against models in the DO Spaces
  manifest.
- Vs-AI offline — existing Tauri/ONNX-local path.
- Deck builder locked to `game_mode: "standard"`.
- Baked-in hosted-API and manifest URLs from Vite env vars; no user-entered
  URL in UI.
- Small "alpha" banner on Home linking to the patch notes page.

## Out of scope

- Accounts, login/register, persistent rating, profile page.
- Replays, spectator, public lobby browse. (PvP is matchmaking + join-by-code.)
- Admin UI, training, arena, gauntlet.
- Web app distribution.

## Components and data flow

### Frontend (desktop build)

- **`frontend/src/bootstrap/guest.ts`** *(new)* — on app boot, if no token is
  cached, `POST /auth/guest`, store `{token, display_name, user_id}` in
  `localStorage`. Called from `authStore.hydrate()`.
- **`frontend/src/pages/HomePage.tsx`** — replace the three-card grid with
  Find Match, Play vs AI, Deck Builder, Models, Patch Notes. Add an alpha
  banner component that links to the tested-cards list + patch notes.
- **`frontend/src/pages/ModelsPage.tsx`** — remove the manifest-URL text
  input; read it from `VITE_MODELS_MANIFEST_URL`. Merge the "Remote manifest"
  and "Downloaded" sections into one model-centric table. Add a `Try online`
  button that posts to `/games` with `player2_policy=trained,
  player2_model=<manifest_id>` and navigates to `/game/<game_id>?mode=vsai`.
- **`frontend/src/pages/GamePage.tsx`** — handle `mode=vsai`: connect via the
  existing game WS channel (same path PvP uses), no Tauri `invoke` for the
  opponent's actions. The offline-AI path (Tauri + Rust inference) stays as
  it is today.
- **`frontend/src/pages/LobbyPage.tsx`** — hide the Browse tab for alpha;
  keep Play (matchmaking), Create, Join.
- **`frontend/src/pages/DeckBuilderPage.tsx`** — lock `game_mode` to
  `standard`; remove the format picker. Save/load path switches from the
  hosted `/decks` API to the local deck store (below).
- **`frontend/src/storage/deckStore.ts`** *(new)* — thin wrapper over a
  Tauri command pair (`decks_list`, `decks_get`, `decks_put`, `decks_delete`)
  backed by JSON files under `app_data_dir()/decks/`. Same `DeckSummary` /
  `Deck` shapes the existing `deckApi.ts` returns, so the Lobby and Deck
  Builder pages can swap the source behind a single import. No network
  roundtrip, no server schema for guest decks, no token anchoring concern.
- **`src-tauri/src/deck_storage.rs`** *(new)* — the Rust side of those
  commands. JSON-on-disk is sufficient; lists are small (tens of decks).
- **`frontend/src/components/layout/Layout.tsx`** — drop admin/training
  links from the side nav.
- **`frontend/src/components/auth/AuthGuard.tsx`** — on desktop, gate on
  "guest token exists" rather than "user is logged in".

### Backend gaps

Small, needed for the frontend design to work.

- **`POST /auth/guest`** — issues a **long-lived JWT** (1 year) whose `sub`
  is `guest_<uuid>` and whose display name is `Guest-<4chars>`. Backed by a
  row in a `guest_users` table (or a new `User.is_guest` flag on the
  existing `users` table — implementer's call) so `get_current_user` returns
  a usable object exposing `id`, `display_name`, and `rating` (default 1500)
  to the matchmaking router. The token is long-lived so the user isn't
  interrupted mid-session by a refresh flow; nothing important is anchored
  to the guest `user_id` on the server (decks live client-side — see below),
  so losing the token is harmless.
- **`POST /models/{id}/prepare`** — new endpoint on the DB-coupled
  `admin_models` public router that resolves a manifest row ID to a local
  ONNX filename (downloading from DO Spaces on cache miss,
  `/tmp/digimon-models/<sha256>.onnx`) and returns
  `{filename, cached}`. `games.py` stays engine-only per Working Rule #11;
  the frontend calls `prepare` first, then posts to the unchanged `/games`
  with `player2_model=<returned filename>`. Cache is keyed by sha256 so
  repeated requests hit warm.
- **Matchmaking queue accepts a raw deck payload** — today
  `POST /matchmaking/queue` takes a `deck_id` and hits the DB. Add an
  alternative body shape that carries the deck inline (`main_deck`,
  `egg_deck`, `game_mode`), skipping the DB lookup. The tier classifier
  already takes a card-ID list, so it runs against the inline payload
  unchanged. Accounts-based flows can keep using `deck_id` when that lands
  post-alpha.

## Error handling and edge cases

- **No network on first launch** — `POST /auth/guest` fails. Show a retry
  banner on Home. Vs-AI offline with already-downloaded models still works.
  PvP and Try Online are disabled with an "offline" tooltip.
- **Engine-shape mismatch** — the manifest row is already flagged
  "incompatible" client-side. Extend the disable state to cover Try Online
  too (server would fail with the same check).
- **Server has no cached ONNX yet** — `POST /games` with a manifest_id does
  a blocking fetch from Spaces on cache miss. Frontend shows a "warming up
  model…" spinner for up to ~15s before the WS starts streaming state.
  If the fetch takes longer, surface the error and let the user retry.
- **Guest token missing or unreadable** — bootstrap mints a fresh one via
  `POST /auth/guest`, creating a new guest user. No data loss because decks
  are local. A 401 on a *present, decodable* token surfaces as an auth
  error rather than silently re-minting, so the user sees a real problem
  rather than their identity quietly changing mid-session.
- **Matchmaking WS disconnect** — existing ghost-ticket cleanup already
  drops the ticket on disconnect; no changes.

## Testing

- **Frontend unit**: `bootstrap/guest.ts` — new token on first boot, cached
  token reused on subsequent boots, missing token re-mints, 401 on an
  existing token surfaces as an error rather than silently re-minting.
- **Frontend unit**: `GamePage` `mode=vsai` branch connects to WS without
  calling Tauri commands for the opponent.
- **Frontend unit**: `ModelsPage` renders the correct button set per row
  state.
- **Tauri unit** (`cargo test --manifest-path src-tauri/Cargo.toml`):
  `deck_storage` round-trips a deck list through `app_data_dir`, deletes
  are visible on the next list, malformed JSON on disk doesn't panic.
- **Backend pytest**: `POST /auth/guest` mints a usable token and
  matchmaking accepts it.
- **Backend pytest**: `POST /games` with a manifest_id triggers a Spaces
  download, caches it, and a second request hits the cache.
- **Manual desktop smoke**: install → first launch → play offline vs
  downloaded model → Try Online vs undownloaded model → queue for match vs
  a second build of the app.

## Open questions

None at the time of writing. Any that surface during plan-writing get added
here.

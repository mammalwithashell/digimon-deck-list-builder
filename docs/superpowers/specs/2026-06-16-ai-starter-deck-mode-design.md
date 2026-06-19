# AI Starter Deck Mode — Design

**Date:** 2026-06-16
**Status:** Approved design, pending implementation plan
**Topic:** A new game mode where a player picks one of the 6 official starter decks and plays against a trained AI model that pilots a randomly chosen starter deck. Ships the 6 starter decks as built-in read-only decks and lays the "released model" substrate so the first model release lights the mode up with no client change.

## Goal

For the first model release, give players a one-click path:

1. From the mode/queue menu, select a new **AI Starter Deck** tile (placed right after **Bot Match**).
2. Choose one of the **6 official starter decks** (ST-1 … ST-6) — and only those.
3. Play a game against the **AI model**, which pilots a **randomly chosen** one of the 6 starter decks. The player does **not** choose the AI's deck.

Two enabling requirements:

- **Ship the app with the 6 starter decks** as built-in, read-only decks (always present, not editable/deletable).
- **Make a format exception for ST-2**, whose deck contains a card that is *Limited* in the standard modern banlist (ST2-13 "Hammer Spark"), so the unaltered ST-2 starter deck is legal in this mode.

The AI model itself does **not exist yet**. This change builds the *substrate* so that:

- The mode ships and is playable **today** against the greedy CPU.
- When the first model is published to the hosted manifest and flagged as the starter model, the mode begins using it with **no client-side change**.

## Non-goals

- Training or publishing the actual model (separate workstream).
- Online/ranked matchmaking, Elo, or leaderboards for this mode.
- Changing in-game *rules* for starter play — starter decks play under normal `Rules::standard()`. The "starter" format only relaxes **deck legality** (the banlist), nothing else.
- Trained-model inference in the hosted browser build. Trained-model play is **desktop-only** (ONNX inference lives in the Tauri shell). The browser build renders the tile/flow and uses the greedy CPU.
- Editable starter decks. They are read-only built-ins; the user cannot rename, edit, or delete them.

## Current state (what exists today)

- **Mode menu:** `code/frontend/src/pages/ModeSelectPage.tsx` renders an `OPPONENTS` array of three tiles: `quick`, `room`, `bot`. Selecting a tile stores `opponentMode` in the Zustand `playFlowStore`. A 6-card **format grid** is shown below the tiles. "ENTER FORMAT" routes to `/play/deck` (bot/quick) or `/play/room`.
- **Bot match flow:** `DeckSelectPage.tsx` → `createBotGame()` in `code/frontend/src/features/play/playApi.ts`. The opponent is currently a **greedy CPU mirroring the player's own deck** (`opponentDeck: deck`). Desktop calls `invoke('rust_create_game', { player_kinds: ['human','greedy'], player_model_ids: [null, null] })`; browser POSTs `/games` with `player2_policy: 'greedy'`.
- **Trained-model path (separate):** `code/frontend/src/pages/ModelsPage.tsx` already plays vs a trained ONNX model but requires manual model activation and also mirrors the player's deck (`TODO(alpha+1)` to use `manifest.deck_id`). This is a different entry point; we are **not** repurposing it.
- **Decks (desktop):** `Deck` struct in `code/src-tauri/src/deck_storage.rs`; decks persist as `{app_data_dir}/decks/{deck_id}.json`. There is **no** mechanism to ship/seed default decks — users create them at runtime. The frontend deck type is `DeckResponse` in `code/frontend/src/types/deck.ts`.
- **Starter decklists (data):** All 6 ST-1…ST-6 decklists exist in `data/deck_library.json` (`format: "starter"`). `data/starter_decks.json` exists but contains **only ST-3**. All 96 ST-1…ST-6 cards are in `data/tested_cards.json` and were battle-tested for training readiness (2026-06-14, verdicts in `qa/qa-reports/validated_cards_dsl.json`).
- **Deck validation / formats:** `code/digimon-engine/src/deck_tools.rs` `validate_deck_for_descriptor` enforces size, copy limits, and **format restrictions (banlist)**. Restrictions live in `data/deck_formats.json`; `official_eng` lists **ST2-13 as Limited (1 copy)**. Tauri entry: `rust_validate_deck_raw(main, egg, game_mode)` in `code/src-tauri/src/deck_commands.rs`. Hosted entry: `code/server/routers/deck_tools.py`.
- **AI game creation (desktop):** `rust_create_game(deck1, deck2, player_kinds, player_model_ids, seed)` in `code/src-tauri/src/engine_commands.rs`. `PlayerKind` ∈ `{Human, Greedy, Trained}`; `Trained` requires a non-null `model_id`, validated at creation. The agent loop (`run_agent_steps`) drives greedy/trained turns; trained calls `inference.predict(model_id, obs, mask)`.
- **Models:** `code/src-tauri/src/models.rs` fetches `/models/manifest.json`, SHA-verifies + caches ONNX to `{app_data_dir}/models/<id>/policy.onnx`. `ManifestModel` has an unused `deck_id`/`deck_name`. There is **no "default/released" model concept**.

## Design

### 1. New `ai_starter` opponent mode (frontend)

- Add a fourth tile to `OPPONENTS` in `ModeSelectPage.tsx`, **immediately after `bot`**:
  `{ id: 'ai_starter', name: 'AI STARTER DECK', sub: 'Pick a starter, face the AI', meta: 'AI OPPONENT' }` (final display copy may be tuned during implementation).
- Add `'ai_starter'` to the `OpponentMode` type and the `playFlowStore`.
- When `ai_starter` is selected, the **format grid is hidden/disabled** — this mode *is* the "starter" format. The action bar routes directly to the starter-deck picker (no format choice).
- The deck picker for this mode is **scoped to the 6 built-in starter decks only** (the player's own custom decks are not shown). Selecting one starts the game via the new `createAiStarterGame()` path.
- Both build targets render the tile and flow; the AI opponent differs by build/model availability (see §4).

### 2. Ship the 6 starter decks as built-in read-only (data + desktop)

- **Data:** Expand `data/starter_decks.json` to contain **all 6** ST-1…ST-6 decks, generated from `data/deck_library.json`. Schema per deck:
  ```json
  {
    "version": 2,
    "starter_decks": {
      "ST-1": {
        "id": "starter_st1",
        "name": "Starter Deck Gaia Red",
        "set": "ST1",
        "game_mode": "starter",
        "egg_deck": ["ST1-01", ...],
        "main_deck": ["ST1-03", ...]
      },
      "ST-2": { ... },  "ST-3": { ... }, "ST-4": { ... }, "ST-5": { ... }, "ST-6": { ... }
    }
  }
  ```
  Egg vs main split is derived from card metadata (Digi-Egg cards → `egg_deck`). The generator is a small committed script/tool so the file can be regenerated if a decklist changes.
- **Desktop loading:** A new Tauri command (e.g. `rust_list_starter_decks()`) returns the 6 decks as `DeckResponse`-shaped objects with a `read_only`/`is_builtin: true` flag and `game_mode: "starter"`. They are loaded from the bundled `data/starter_decks.json` (shipped in the app bundle), **not** from `{app_data_dir}/decks/`.
- **Read-only enforcement:** The deck builder UI treats `is_builtin` decks as locked (no edit/delete/rename). They also appear in the library as locked entries. The AI-starter mode picker reads from `rust_list_starter_decks()`.
- **Browser build:** the hosted build can serve the same 6 decks from the bundled data (or a `/decks/starter` endpoint) so the tile is functional there too; trained-model play remains desktop-only.

### 3. ST-2 limited-card exception (engine/data)

- Add a `starter` format descriptor to `data/deck_formats.json` with **no banlist restriction** (or an empty restriction) so ST2-13 at its printed **×4** (the official `starter_st2_cocytus_blue` deck runs 4 copies vs the standard Limited cap of 1) is legal. Deck size / egg cap match standard starter construction (the ST-2 deck is 50 main + 4 egg).
- Built-in starter decks carry `game_mode: "starter"`, so any legality check (library display, deck picker, or a defensive validate-on-create) treats them as legal.
- **Standard format is unchanged** — ST2-13 stays Limited there. The exception is contained to the `starter` format / this mode.
- In-game rules are unaffected: game creation continues to use `Rules::standard()`. (`starter` only changes the deck-legality descriptor, not `Rules`.)

### 4. AI opponent — random starter deck + released-model substrate (desktop)

- **AI deck choice:** On game start, the AI's deck is chosen **uniformly at random from the 6** starter decks. The choice is **derived from the game seed** so games are reproducible (the existing bot-match seed input is reused). It may occasionally coincide with the player's pick (accepted).
- **Game creation:** A new `createAiStarterGame({ deck, seed })` resolves the random AI starter deck, then calls (desktop) `rust_create_game(deck1 = player, deck2 = randomStarter, player_kinds, player_model_ids, seed)`.
- **Model resolution substrate:**
  - The hosted `/models/manifest.json` gains a top-level optional pointer `starter_ai_model_id: string | null` (mirrored in `models.rs`). The server sets it when a starter model is published.
  - On entering the mode, desktop resolves `starter_ai_model_id` → finds the matching `ManifestModel` → auto-downloads + loads it (SHA-verified, cached) → uses `PlayerKind::Trained` with that `model_id`. Tensor/action-shape validation (`validate_shapes`) guards mismatches.
  - **Fallback:** if `starter_ai_model_id` is null, unresolvable, not downloadable, or the build can't run ONNX (browser), player 2 falls back to **`PlayerKind::Greedy`**. The mode is fully playable today against the greedy CPU.
  - No client change is needed when the first model is published — flipping the manifest pointer lights it up.

### 5. Scope & build targets

- **Primary target: desktop (Tauri).** Trained-model inference is desktop-only.
- The tile + flow + starter decks + `starter` format work in both builds.
- Browser build uses greedy for the AI until/unless hosted trained-model serving is added (out of scope).

## Data contracts (summary)

- `data/starter_decks.json` — `version: 2`, all 6 decks with `id`, `name`, `set`, `game_mode: "starter"`, `egg_deck[]`, `main_deck[]`.
- `data/deck_formats.json` — new `starter` format with no restriction; standard format unchanged.
- `/models/manifest.json` — new optional top-level `starter_ai_model_id: string | null`.
- `ManifestModel` / manifest parsing in `code/src-tauri/src/models.rs` — read the new pointer.

## UX flow

```
ModeSelectPage
  └─ tiles: [QUICK] [ROOM] [BOT] [AI STARTER DECK]   ← new tile, after BOT
        │  (selecting AI STARTER DECK hides the format grid)
        ▼
  StarterDeckPicker  (6 built-in read-only decks only)
        │  player picks ST-n
        ▼
  createAiStarterGame({ deck, seed })
        │  AI deck = uniform-random of 6 (seed-derived)
        │  AI player kind = Trained(starter_ai_model_id)  OR  Greedy (fallback)
        ▼
  GamePage  (normal game UI; opponent is the AI)
```

## Testing

- **Rust (engine):** unit test that the `starter` format accepts the unaltered ST-2 deck (ST2-13 ×4 legal) while standard still rejects it; test that `rust_list_starter_decks()` returns 6 read-only decks with correct egg/main split and `game_mode: "starter"`.
- **Rust (tauri):** test seed-derived AI deck selection is deterministic and within the 6; test `rust_create_game` accepts the trained/greedy kinds for this mode and that greedy fallback engages when `model_id` is absent.
- **Frontend:** test mode routing (`ai_starter` hides the format grid and routes to the starter picker); test the picker shows exactly the 6 built-in decks and no custom decks; test `createAiStarterGame` wires player/AI decks + seed.
- **Manual (desktop):** run the desktop app, select the new tile, pick each of the 6 decks, confirm a game starts vs the greedy CPU (no model published yet), and confirm the ST-2 deck is selectable/legal.

## Decisions resolved

- **AI deck:** uniformly random among the 6 each game (seed-derived); may mirror the player's pick.
- **Model source:** no model yet — build the substrate; greedy fallback until a starter model is flagged in the manifest.
- **Deck shipping:** built-in, read-only (non-deletable, non-editable), always present.
- **Format grid:** hidden when `ai_starter` is selected.

## Rollout / sequencing

1. Data: expand `data/starter_decks.json` (6 decks) + add `starter` format to `data/deck_formats.json`.
2. Engine: `starter` format legality + (defensive) validation; tests.
3. Tauri: `rust_list_starter_decks()` (read-only built-ins) + seed-derived AI deck selection + greedy/trained wiring; tests.
4. Models: `starter_ai_model_id` manifest pointer + desktop resolution/fallback.
5. Frontend: `ai_starter` tile, format-grid hide, starter-only picker, `createAiStarterGame`; tests.
6. Manual desktop verification.

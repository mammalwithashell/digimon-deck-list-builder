## Why

While playtesting a game in the UI you cannot read what a card actually does — there is no way to right-click a card and see a large preview of its art and printed text. The reference client (DCGO) supports this for both hand cards and field permanents, and its absence makes in-app QA of archetypes slow and error-prone (cards on the board are tiny and many show placeholder art). The data needed already exists in the frontend (CDN art via `useCardImage`, printed text via `digimonCardApi.getCardById`); it is simply not wired into the game board.

## What Changes

- Add an **in-game right-click (`onContextMenu`) gesture** that opens a large card-preview overlay, for:
  - **Hand cards** (right-click a card in the hand) — previews that card.
  - **Field permanents** (right-click a Digimon/Tamer/Option on the field) — previews the permanent's top card, and surfaces the full **digivolution stack** so each source card can also be previewed.
- The preview shows **full-size card art** (`useCardImage` → digimoncard.io CDN) and the **full printed card text** (name, cost, DP, level, traits, main effect, inherited effect) sourced from card metadata by `cardId` — independent of engine runtime state.
- Introduce a **reusable, game-context preview component** that takes a `cardId` (and, for permanents, the source `cardId`s) — decoupled from the deck-builder-bound `CardDetail`/`useDeckBuilderStore`.
- Preserve existing left-click behavior (play, attack, target selection); right-click is preview-only and never submits a game action.
- Dismissable via the existing overlay close affordance / click-away / `Esc`.

Out of scope (tracked separately, not part of this change):
- The empty in-game **action log** (Rust binding `get_last_log()` stub) — separate change.
- **Runtime-modified** stack state in the inspector (current DP after modifiers, which inherited effects are *currently active*) — depends on engine `serialization.rs` text stubs; the printed-card preview here deliberately uses static metadata instead.

## Capabilities

### New Capabilities
- `ingame-card-preview`: Right-click-to-preview of cards during a game — large art + printed text for hand cards and field permanents, including per-source preview of a permanent's digivolution stack.

### Modified Capabilities
<!-- None. No existing spec covers in-game card preview / game-board interaction. -->

## Impact

- **Frontend only.** No engine, binding, server, or API changes. No changes to action masking, the action space, or game state.
- Affected areas (React): `components/board/HandZone.tsx`, `components/board/PermanentSlot.tsx` (+ `GameBoard.tsx` plumbing), `components/game/CardOverlay.tsx` (the existing field stack inspector), `pages/GamePage.tsx` (right-click handler + overlay state), and a new reusable preview component under `components/shared/` or `components/game/`.
- Reuses existing primitives: `Card` (`size="xl"`), `hooks/useCardImage.ts`, `api/digimonCardApi.ts` (`getCardById`), `utils/cardImages.ts` (`getCardImageUrl`).
- External dependency unchanged: card art is loaded from the digimoncard.io CDN (already the app's image source); printed metadata from the same source the deck builder already uses. No new offline assets required.
- No engine/server tests affected; verification is via frontend unit tests (gesture → preview state) and manual/Playwright in-app checks.

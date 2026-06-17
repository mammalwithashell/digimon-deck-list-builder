# Deck Icon Placeholder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Let users choose a representative card image for a saved deck, default new and legacy decks to a stable heuristic icon, and show real card thumbnails in deck lists and the deck builder contents rail.

**Architecture:** Reuse the existing `commander_id` persistence field as the explicit deck icon card id for all formats. Summaries expose both `commander_id` and computed `deck_icon_card_id`; the computed value is explicit-if-valid, otherwise a heuristic fallback. The deck builder store tracks the explicit selected icon, while list surfaces render the computed summary icon.

**Tech Stack:** React, Zustand, Vitest, FastAPI/Pydantic, Tauri Rust deck storage.

---

### Task 1: Add Tests For Icon Selection

- [ ] Add Vitest coverage for the frontend deck icon heuristic.
- [ ] Add Zustand coverage for loading, selecting, and clearing an explicit icon when the last copy leaves the deck.
- [ ] Add API coverage for standard-format decks accepting `commander_id` as a visual icon and returning `deck_icon_card_id`.
- [ ] Add Rust deck-storage coverage for summary icon fallback and explicit icon preservation.

Run:

```powershell
cd code/frontend
npm test -- src/features/deck-builder/deckBuilderView.test.ts src/stores/deckBuilderStore.test.ts
cd ../..
python -m pytest code/tests/api/test_decks_library.py -q
cd code/src-tauri
cargo test deck_summary
```

### Task 2: Implement Shared Frontend Heuristic

- [ ] Export `selectDeckIconCardId(mainDeck, eggDeck, explicitId?)` from `deckBuilderView.ts`.
- [ ] Prefer explicit ids that are still present in the deck.
- [ ] Otherwise sort main-deck entries using the DCGO-inspired order: Digimon before Tamer before Option, higher level, color order, higher play cost, higher DP, then stable card id.
- [ ] Fall back to the first egg id when no main-deck card is available.

### Task 3: Persist Explicit Icon In The Builder

- [ ] Add `deckIconCardId` and `setDeckIconCardId` to `deckBuilderStore`.
- [ ] Load existing `commander_id` into that state.
- [ ] Save `commander_id` from the explicit icon if the card is still in the deck.
- [ ] Clear the explicit icon if the final matching card is removed.

### Task 4: Render Icons In Builder And Lists

- [ ] Replace the right rail color swatch with a card thumbnail.
- [ ] Add a row action to set the deck icon.
- [ ] Add a compact selected/auto icon preview above deck contents.
- [ ] Render `deck_icon_card_id` images in launcher recent decks, deck select cards, and deck library sleeves with existing fallbacks on missing images.

### Task 5: Expose Summary Fields

- [ ] Add `commander_id` and `deck_icon_card_id` to deck summary schemas/types.
- [ ] Compute summary icons in FastAPI, local desktop fallback, and Tauri storage.
- [ ] Relax standard-format API validation so `commander_id` is visual metadata, while still rejecting ids that are not in the deck.

### Task 6: Verify

- [ ] Run the focused frontend tests.
- [ ] Run focused API tests for deck library behavior.
- [ ] Run focused Tauri Rust deck-storage tests.
- [ ] Run a frontend build or typecheck.
- [ ] Use the in-app browser to confirm deck builder thumbnails and icon selection are visually coherent.

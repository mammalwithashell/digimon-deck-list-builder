## 1. Make the deck builder embeddable

- [x] 1.1 In `pages/DeckBuilderPage.tsx`, rename the component to `DeckBuilderWorkbench` accepting `{ embedded?: boolean; initialDeckId?: string | null; onSaved?: (deckId: string) => void; onClose?: () => void }`, and add a thin `export function DeckBuilderPage()` wrapper that renders `<DeckBuilderWorkbench />` (page mode).
- [x] 1.2 Resolve the deck to load from `initialDeckId` when `embedded`, else the route param; gate the `/new` clear effect and import-query handling to page mode only.
- [x] 1.3 Branch `handleSave`: page mode keeps existing navigation; embedded mode persists then calls `onSaved(savedId)`. Render outer wrapper as `.deck-builder-page > .deck-builder-app` in page mode and `.deck-builder-app` (overlay-framed) in embedded mode.
- [x] 1.4 In embedded mode, replace the top-bar HOME/LIBRARY/QUIT controls with a CANCEL/close control (wired to `onClose`); keep SAVE/VALIDATE/IMPORT/CLEAR. (The confirm-to-discard guard is centralized in `DeckEditOverlay` so it also covers ESC/backdrop.)

## 2. Edit overlay component

- [x] 2.1 Add `components/deckbuilder/DeckEditOverlay.tsx` — a portal backdrop + frame (state-driven, like `ImportExport`) that renders `<DeckBuilderWorkbench embedded initialDeckId onSaved onClose />` when open; backdrop/ESC close routes through the same dirty-guarded `onClose`.
- [x] 2.2 Add overlay CSS (frame height ~96vh so the builder scrolls internally; backdrop dim; consistent with builder chrome incl. `[data-theme="light"]`).

## 3. Wire EDIT into the play flow

- [x] 3.1 In `pages/DeckSelectPage.tsx`, add an EDIT control on the selected deck (and/or in the confirm bar), enabled only when a deck is selected, for all opponent modes.
- [x] 3.2 Add overlay open/close state; EDIT opens `DeckEditOverlay` with the selected deck id.
- [x] 3.3 Implement `onSaved`: re-fetch `library.listDecks()`, re-select the saved deck by id, close the overlay (legality re-checks on render). `onClose` just closes.

## 4. Tests

- [x] 4.1 `DeckBuilderWorkbench` embedded mode: renders CANCEL instead of HOME/LIBRARY/QUIT, and a save invokes `onSaved` rather than navigating.
- [x] 4.2 `DeckEditOverlay`: renders the builder when open, nothing when closed; dirty-guarded close.
- [x] 4.3 `DeckSelectPage`: EDIT is disabled with no selection and opens the overlay for the selected deck; an `onSaved` refresh keeps the deck selected.

## 5. Verification

- [x] 5.1 Frontend tests for the touched files green; `tsc -b` clean. (11 new tests pass; full suite 240/241, the 1 failure is a pre-existing CardOverlay card-text test outside this change.)
- [x] 5.2 Live: from CHOOSE DECK (bot flow), EDIT opens the full builder overlay with the deck loaded and embedded chrome (← CANCEL / DONE, no HOME/LIBRARY/QUIT); an in-overlay edit (50→51) marks the deck dirty; CANCEL prompts to discard and, on confirm, returns to CHOOSE DECK with the deck still selected and format/mode preserved; the discard persisted nothing (deck verified still 50 via the API). The SAVE→persist→refresh integration is covered by the unit tests in 4.1/4.3 rather than exercised live, to avoid mutating real saved decks.

## Why

Right before a match — on the CHOOSE DECK screen, the last step before a bot game, the quick-match queue, or a room — players often spot a card they want to swap. Today the only edit path is the **NEW DECK** link, which is a full-page navigation to the deck builder; there is no way to edit the *existing* deck you just selected, and leaving the play flow to do it is jarring. Players should be able to tweak the deck they are about to play, in place, without losing their spot in the queue flow.

## What Changes

- Add an **EDIT** affordance to the selected deck on the CHOOSE DECK screen (`DeckSelectPage`) for **all** opponent modes (bot, quick-match, room).
- EDIT opens the **full deck builder in an overlay** over the play flow — every builder capability (filters, the new GRID/DETAIL/DECKLIST views, validation, import) is available, not a reduced editor.
- Saving in the overlay persists the deck, re-checks its legality for the queue format, refreshes the selected deck's summary, keeps it selected, and returns the player to CHOOSE DECK — no full-page navigation.
- Cancelling discards in-overlay changes, with an unsaved-changes confirmation.
- The play-flow selections (format, opponent mode) are preserved throughout.
- The deck builder gains an **embedded mode** (rendered inside the overlay) that swaps its page navigation chrome (HOME/LIBRARY/QUIT) for overlay close/save controls; its page route is unchanged.

## Capabilities

### New Capabilities
- `deck-edit-before-queue`: Editing the about-to-be-played deck from the play flow — the EDIT trigger on CHOOSE DECK across all modes, the full deck builder rendered in an embedded overlay, the save-in-place flow (persist → legality re-check → refresh → return), the discard/unsaved-changes behaviour, and preservation of the play-flow selections.

### Modified Capabilities
<!-- None. No existing spec governs the play deck-select flow or the deck builder page shell; the deck-builder card-browsing capability (view modes, filters) is reused unchanged inside the overlay. -->

## Impact

- **Frontend (play flow + deck builder):**
  - `pages/DeckBuilderPage.tsx` — the builder body becomes a reusable `DeckBuilderWorkbench` with an `embedded` mode (props: `initialDeckId`, `onSaved`, `onClose`); `DeckBuilderPage` stays as the thin route wrapper. Embedded mode swaps page-nav chrome for overlay controls and routes SAVE/CANCEL through callbacks instead of `navigate`.
  - New `components/deckbuilder/DeckEditOverlay.tsx` (+ css) — a portal/backdrop modal wrapping `DeckBuilderWorkbench`, matching the existing state-driven modal pattern (`ImportExport`).
  - `pages/DeckSelectPage.tsx` — EDIT button on the selected deck; overlay open/close state; `onSaved` re-fetches the deck list and keeps the edited deck selected.
- **No backend, engine, RL, or hosted-web changes.** Reuses the existing `/desktop-decks` (desktop) / deck API persistence via `deckBuilderAdapter` / `deckLibraryAdapter`.
- **Tests:** `DeckBuilderWorkbench` embedded-mode chrome/callbacks; `DeckEditOverlay` open/close + portal; `DeckSelectPage` EDIT opens the overlay and a save refreshes the selected deck.

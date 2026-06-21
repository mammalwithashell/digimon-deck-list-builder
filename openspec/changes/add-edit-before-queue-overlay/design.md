## Context

The play flow is: `/play` (mode + format) → CHOOSE DECK (`pages/DeckSelectPage.tsx`) → "USE THIS DECK" (`handleConfirm`), which branches to a bot game (`createBotGame` → `/game/:id`), the quick-match queue (`/play/matching`), or a room (`/play/room/new`). The format and opponent mode live in `playFlowStore` (Zustand), so they already survive navigation. The only existing edit path from CHOOSE DECK is a **NEW DECK** link to `/deckbuilder/new?returnTo=play` — a full-page navigation; there is no edit-existing path and no overlay.

The deck builder is `pages/DeckBuilderPage.tsx` (~720 lines): one component that reads route params, loads a deck, owns `deckBuilderStore` (main/egg decks, dirty flag, validation), and renders the `.bld` shell (top bar, filters, 3-column main, `ImportExport` modal). Modals in this codebase are **state-driven**, not route-based (`<ImportExport isOpen onClose>`). The builder CSS is scoped to both `.deck-builder-page` and `.deck-builder-app`, so either wrapper class pulls in the full styling and theme variables.

## Goals / Non-Goals

**Goals:**
- Edit the selected deck from CHOOSE DECK, in an overlay, for all modes.
- Reuse the entire deck builder (no reduced editor) inside the overlay.
- Save-in-place: persist → legality re-check → refresh selection → return; cancel discards with a guard.
- Preserve play-flow selections.

**Non-Goals:**
- No backend/persistence changes (reuse existing deck adapters and `/desktop-decks`).
- No route-based modal machinery (match the existing state-driven modal pattern).
- No change to the builder's own page route or to which cards/effects exist.
- No new reduced/quick editor — the full builder is the editor.

## Decisions

### Reuse the builder via an `embedded` mode, not a rewrite
Rename the builder component to `DeckBuilderWorkbench(props)` and keep `DeckBuilderPage` as a thin wrapper that renders it with no props (page mode). The workbench gains `{ embedded?, initialDeckId?, onSaved?, onClose? }`. Router hooks (`useParams`/`useLocation`/`useNavigate`) are still called unconditionally (the overlay mounts inside the app router) but their values are only *used* in page mode; embedded mode uses `initialDeckId` and the callbacks. This reuses 100% of the builder with mostly additive branching, far lower risk than cutting a 700-line body into a new file.

*Alternatives considered:* (a) a full extraction into a separate `DeckBuilderWorkbench.tsx` by moving the body — more churn and merge risk for no behavioural gain; (b) a route-based modal (`/play/deck/edit/:id` with background location) — introduces a router pattern the app doesn't use; rejected for consistency.

### Overlay is a state-driven portal modal (matches `ImportExport`)
`components/deckbuilder/DeckEditOverlay.tsx` renders a backdrop + frame via a portal and mounts `<DeckBuilderWorkbench embedded initialDeckId={deckId} onSaved onClose />`. `DeckSelectPage` owns the open/close state and the `deckId` (the selected deck). The frame gives the builder a definite height (e.g. `92vh`) so its internal scrolling engages; the embedded wrapper uses `.deck-builder-app` for full styling.

### Embedded chrome and save/cancel semantics
In embedded mode the top bar omits HOME/LIBRARY/QUIT and shows a CLOSE/CANCEL control; SAVE/VALIDATE/IMPORT/CLEAR remain. `handleSave` branches: page mode keeps its navigation; embedded mode persists, then calls `onSaved(savedId)`. `DeckSelectPage.onSaved` re-fetches `library.listDecks()`, re-selects the saved deck, and closes the overlay — legality (`canUseDeckForFormat`) re-evaluates on the next render, so a now-illegal deck disables USE THIS DECK automatically. CANCEL/close consults `deckBuilderStore.isDirty`; if dirty, a confirm gates the close.

### Store lifecycle
The overlay loads the deck into the singleton `deckBuilderStore` on open (the existing route-load effect, driven by `initialDeckId`). On close it clears the store so a later visit to the real builder page reloads cleanly. `DeckSelectPage` itself does not read `deckBuilderStore`, so there is no cross-talk while the overlay is closed.

## Risks / Trade-offs

- **A `Page`-named component rendered in a modal reads oddly.** → Renamed to `DeckBuilderWorkbench`; `DeckBuilderPage` remains only as the route wrapper, so intent is clear.
- **Singleton `deckBuilderStore` shared between the page and the overlay.** → Only one is mounted at a time in this flow; the overlay clears the store on close. No concurrent editing path exists.
- **Unsaved-changes loss on accidental close.** → `isDirty`-gated confirm on cancel/backdrop/close.
- **Overlay sizing/scroll regressions.** → The frame supplies a definite height; the builder's existing internal-scroll contract (`.bld` rows `56px auto 1fr`) is unchanged. Below 1120px the builder already switches to page-scroll, which the overlay tolerates.
- **Stale selection after save.** → `onSaved` re-fetches the deck list and re-selects by id, so counts/icon/legality refresh.

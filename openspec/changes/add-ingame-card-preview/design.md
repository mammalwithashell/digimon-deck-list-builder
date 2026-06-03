## Context

The in-game board renders cards small and many show placeholder/back art, so a player testing an archetype can't read a card's effect text. The reference client (DCGO) lets you right-click any card — in hand or on the field — to see a large art + full-text preview, and for a field permanent it also shows the digivolution stack.

The frontend already has every primitive needed:
- `hooks/useCardImage.ts` → `utils/cardImages.ts::getCardImageUrl(cardId)` loads full art from the digimoncard.io CDN.
- `components/shared/Card.tsx` renders a card image and supports `size="xl"`.
- `api/digimonCardApi.ts::getCardById(cardId)` returns full printed metadata (`DigimonCardData`: name, cost, DP, level, traits, effect text, inherited text).
- `components/game/CardOverlay.tsx` already renders a field permanent's digivolution stack (color-coded, top-first) — it is currently opened via a left-click fall-through in `GamePage.handleSlotClick` and stores `inspectedPerm: PermanentInfo`.
- `components/shared/CardDetail.tsx` renders art + full text — but is coupled to `useDeckBuilderStore` (`selectedCardId`, `addCardToDeck`), so it is not directly reusable in-game.
- Right-click (`onContextMenu`) is already used in the deck builder, confirming the pattern.

What's missing is purely wiring: an in-game right-click gesture and a deck-builder-independent preview surface keyed by `cardId`.

Critically, the preview is a view of the **printed card**, which is static metadata — so it does **not** depend on the engine's `serialization.rs` effect-text (currently stubbed to `""`) nor on the binding's `get_last_log()` (also stubbed). Those are separate concerns and explicitly out of scope.

## Goals / Non-Goals

**Goals:**
- Right-click a hand card → large preview (art + printed text).
- Right-click a field permanent (own or opponent) → large preview of the top card, plus the digivolution stack with each source previewable.
- Source all preview content from static metadata by `cardId`, independent of engine runtime state.
- Right-click is preview-only; it never submits a game action and never alters left-click behavior.

**Non-Goals:**
- Showing **runtime-modified** state in the preview (current DP after modifiers, which inherited effects are *currently active*, temporary keywords). That needs engine `serialization.rs` work and is a separate change.
- Fixing the empty in-game **action log** (`get_last_log()` stub) — separate change.
- Any engine, PyO3 binding, FastAPI, or action-space change.
- Offline/bundled card art (continue using the CDN, as the rest of the app does).

## Decisions

**1. Add a dedicated game preview component rather than reusing `CardDetail` directly.**
`CardDetail` reads `selectedCardId` and calls `addCardToDeck` from `useDeckBuilderStore`; using it in-game would couple gameplay to the deck-builder store and risk deck mutations. Instead, introduce a small presentational component (e.g. `GameCardPreview`) that takes a `cardId` prop, fetches via `getCardById`, and renders `<Card size="xl" />` + printed text. *Alternative considered:* refactor `CardDetail` to accept a `cardId` prop and drop the store dependency — rejected for now to avoid touching the deck-builder render path in a gameplay change, though the new component should be written so `CardDetail` could later delegate to it.

**2. Reuse and extend the existing `CardOverlay` for the field-permanent case instead of a second overlay.**
`CardOverlay` already renders the stack top-first with color-coded entries and per-source rows. Extend it to (a) show the large art preview of the currently focused card and (b) let a source row be focused to preview that source by its `cardId`. The hand-card case opens the same preview surface with a single card and no stack. *Alternative considered:* a brand-new overlay for both cases — rejected to avoid duplicating the stack-rendering already in `CardOverlay`.

**3. Trigger via `onContextMenu` at the card/slot leaf components, routed up through existing callbacks.**
Hand: add `onCardContextMenu(index)` alongside the existing `onCardClick`/`onCardHoverIndex` in `HandZone`/`Card`. Field: add an `onSlotContextMenu(isOpponent, slotIndex)` alongside `onSlotClick` in `GameBoard`/`PlayerHalf`/`PermanentSlot`. Each handler calls `event.preventDefault()` then sets the preview state in `GamePage`. *Alternative considered:* a global board-level context-menu handler that hit-tests coordinates — rejected as brittle versus per-element handlers.

**4. Preview state lives in `GamePage` as a single nullable descriptor.**
Replace/augment the current `inspectedPerm: PermanentInfo | null` with a preview descriptor that can represent either a bare `cardId` (hand) or a permanent (top card + ordered source `cardId`s + focused source). This keeps one overlay and one dismissal path. The permanent's source `cardId`s already arrive in the serialized state (`PermanentInfo.sources[].cardId` / the stack), so no engine change is needed to enumerate the stack.

**5. Right-click never mutates game state.**
The context-menu handlers only set local preview state and `preventDefault()`. They do not touch `handleAction`, the action mask, attacker selection, or digivolve mode. Left-click paths are untouched.

## Risks / Trade-offs

- **[Right-click could be intercepted or feel non-obvious on some platforms/Tauri webview]** → Keep the existing left-click "inspect" fall-through as a secondary path, and ensure `preventDefault()` reliably suppresses the native menu; optionally surface a hint. Acceptance is the spec's gesture scenarios.
- **[CDN art latency / failure makes the big preview blank]** → The spec requires loading/error placeholders; reuse `useCardImage`'s `isLoading`/`hasError`. No correctness impact on gameplay.
- **[Printed metadata can differ from the engine's actual card behavior]** (e.g., `card_overrides.json` corrections) → The preview is explicitly the *printed* card for human reading, not an authority on engine resolution; acceptable and documented. Runtime-accurate state is a separate (non-goal) track.
- **[Opponent hand is hidden]** → Right-click preview applies only to visible cards: the local player's hand and any face-up field permanent (own or opponent). Face-down/hidden cards are not previewable.
- **[Scope creep toward the runtime inspector / log]** → Explicitly fenced off in Non-Goals; this change touches frontend only.

## Migration Plan

Pure additive frontend change; no data migration, no API/version changes. Ships behind no flag (low risk, view-only). Rollback is reverting the frontend diff. Verified by frontend unit tests for the gesture→preview-state mapping and manual/Playwright checks in a running game (right-click hand card and field permanent, confirm preview + that no action is submitted).

## Open Questions

- Should the field-permanent preview default-focus the **top card** (simplest) or auto-list the stack expanded? (Lean: top card focused, stack listed and selectable.)
- Should `CardDetail` be refactored to delegate to the new `GameCardPreview` in this change, or left as a follow-up to avoid touching the deck-builder render path? (Lean: follow-up.)
- Is a keyboard affordance (e.g., hover + a key) wanted in addition to right-click, for trackpad users? (Out of scope unless requested.)

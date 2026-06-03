## 1. Reusable game card preview component

- [ ] 1.1 Add a presentational `GameCardPreview` component (e.g. `components/game/GameCardPreview.tsx`) that takes a `cardId` prop, fetches printed metadata via `getCardById`, and renders `<Card size="xl" />` plus full printed text (name, cost, DP, level, traits, main effect, inherited effect). No deck-builder store dependency.
- [ ] 1.2 Handle loading and error states using `useCardImage`'s `isLoading`/`hasError` and a metadata-fetch failure path (placeholder, never crash).
- [ ] 1.3 Unit test: given a `cardId`, the component requests the right art URL and renders the metadata text once resolved; error path renders a placeholder.

## 2. Preview state + dismissal in GamePage

- [ ] 2.1 Introduce a single nullable preview descriptor in `GamePage` that represents either a bare hand `cardId` or a permanent (top `cardId` + ordered source `cardId`s + focused source index). Replace/augment the existing `inspectedPerm` state and its single dismissal path.
- [ ] 2.2 Wire dismissal: Escape key, an explicit close control, and click-away all close the overlay and restore prior board state.
- [ ] 2.3 Ensure opening/closing the preview never calls `handleAction`, never changes the action mask, and never alters attacker-selection or digivolve-mode state.

## 3. Right-click gesture wiring

- [ ] 3.1 Hand: add an `onCardContextMenu(index)` callback through `Card` → `HandZone` → `GameBoard` → `GamePage`; the leaf handler calls `event.preventDefault()` and opens the preview for the local player's hand card. Do not wire it for the opponent's (hidden) hand.
- [ ] 3.2 Field: add an `onSlotContextMenu(isOpponent, slotIndex)` callback through `PermanentSlot` → `PlayerHalf`/`BattleArea` → `GameBoard` → `GamePage`; the leaf handler calls `event.preventDefault()` and opens the permanent preview for own or opponent face-up permanents.
- [ ] 3.3 Confirm `event.preventDefault()` reliably suppresses the native context menu in both the browser-dev and Tauri webview runtimes.

## 4. Field permanent preview + digivolution stack

- [ ] 4.1 Extend `CardOverlay` (or compose it with `GameCardPreview`) so a field-permanent preview shows the large art of the focused card plus the digivolution stack listed top-card-first.
- [ ] 4.2 Make each source row in the stack focusable so selecting it previews that source card by its `cardId` (including its inherited effect text from metadata).
- [ ] 4.3 Source the stack's `cardId`s from the already-serialized permanent sources (no engine change); verify a multi-card stack lists every source in order.

## 5. Verification

- [ ] 5.1 Frontend unit tests for the gesture→preview-state mapping (hand right-click sets a hand preview; field right-click sets a permanent preview; neither submits an action).
- [ ] 5.2 Manual/Playwright check in a running game: right-click a hand card and a field permanent, confirm the large art + printed text render, the stack is browsable for permanents, and game state/action mask are unchanged.
- [ ] 5.3 Confirm left-click behavior (play, attack, target selection, digivolve) is unchanged by the addition.

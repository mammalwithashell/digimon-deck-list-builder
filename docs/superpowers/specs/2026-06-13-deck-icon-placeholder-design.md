# Deck Icon Placeholder — Design

**Date:** 2026-06-13
**Status:** Approved (brainstorming) — ready for implementation plan
**Author:** Codex (with james)

## Problem

Deck lists currently have no user-controlled representative card image. The deck
builder's right-side deck contents rail shows compact color swatches instead of
card thumbnails, so a card in the deck is harder to visually recognize than a
card in the pool. Saved deck cards and play deck-selection cards also fall back
to text glyphs or empty placeholders instead of a meaningful card image.

The goal is to add a faithful DCGO-style deck icon feature:

1. The user can pick any card in the deck as the deck icon.
2. Deck-list rows in the builder show card thumbnails on the right rail.
3. Deck summary/list surfaces can render a representative image without fetching
   each full deck.
4. Decks without an explicit icon get a deterministic default that resembles
   DCGO's "key card" behavior.

## DCGO Reference

DCGO stores an explicit `KeyCardId` on `DeckData`. The deck code optionally
serializes this key card as its sixth comma-separated field, and `DeckInfoPrefab`
renders `deckData.KeyCard` as the deck tile image.

When `KeyCardId` is unset, DCGO chooses a default from the main deck. It sorts
main-deck cards by card kind priority, then by higher level, color order, higher
play cost, higher DP, and stable card index. If the main deck is empty, it falls
back to the first Digitama card. The editor exposes this through
`OnClickSetDeckIconButton()`, which opens a deck-only card list with the prompt
"Choose a deck icon card" and writes the selected card's `CardIndex`.

Relevant source files:

- Base DCGO checkout: `Assets/Scripts/Script/DeckData.cs`
- Deck tile image rendering: `Assets/Scripts/Script/DeckInfoPrefab.cs`
- Deck-only picker panel: `Assets/Scripts/Script/DeckListPanel.cs`
- Editor action: `Assets/Scripts/Script/EditDeck.cs`

## Current App Shape

The app already has a persisted deck field named `commander_id`:

- Frontend deck types: `code/frontend/src/types/deck.ts`
- Desktop local storage: `code/src-tauri/src/deck_storage.rs`
- Browser-dev local deck mirror: `code/server/routers/desktop_decks.py`
- Hosted DB schemas/router: `code/server/db/schemas.py` and
  `code/server/db/routers/decks.py`

Today that field is treated as EDH-specific on the hosted API, is not included
in `DeckSummary`, and is not threaded through the deck builder save path. For
this feature, the product concept should be "deck icon" for all deck formats.
To avoid a migration in this change, the implementation continues serializing
the field as `commander_id` while the frontend API exposes `deckIconCardId`
helper naming internally. A database-column rename is out of scope for this
design.

## Recommended UX

Use the hybrid rail direction approved from the mockup:

- Show a compact deck-icon block at the top of the deck contents rail.
- Replace each deck row's color swatch with a tiny card-art thumbnail.
- Add a small icon button on each deck row to set that card as the deck icon.
- Keep row add/remove controls as they are.
- Mark the selected icon row and the rail image with the same visual indicator.

This keeps the deck list scannable, gives a constant "current icon" affordance,
and avoids forcing users into a modal for the common case. This design does not
include a modal deck-only picker.

## Data Model

### Full Deck

Persist one card ID:

- Existing wire field: `commander_id: string | null`
- Frontend helper name: `deckIconCardId` where useful

The saved value is valid only if the card ID is present in the current main or
egg deck. If the selected card is removed, the UI should display the fallback
icon and save `null` or a newly selected explicit icon on the next save.

### Deck Summary

Add a summary-level representative image field:

```ts
deck_icon_card_id: string | null
```

The value is computed as:

1. Saved `commander_id` if it is present in the deck.
2. The deterministic fallback card ID.
3. `null` for empty decks.

This lets deck library and play deck-selection surfaces render thumbnails from
summary data without fetching every full deck.

Summaries also include `commander_id` for compatibility, but rendering must
prefer `deck_icon_card_id` because it is always safe to display.

## Default Icon Heuristic

Implement one shared frontend helper for builder state and one backend/storage
equivalent for summaries:

1. If an explicit saved icon exists and appears in `mainDeck` or `eggDeck`, use
   it.
2. Otherwise choose from main deck entries by:
   - card kind priority: Digimon, Tamer, Option, other
   - higher level first
   - primary color order: Red, Blue, Yellow, Green, Black, Purple, White, other
   - higher play cost first
   - higher DP first
   - stable card ID
3. If no main-deck card exists, choose the first egg by stable card ID.
4. If the deck is empty, return `null`.

This is close to DCGO while fitting the app's card metadata. The only deliberate
difference is that Tamers sort ahead of Options because they are more often
archetype-defining deck identity cards in our current list UI.

## Frontend Changes

### Deck Builder Store

Add state and actions:

- `deckIconCardId: string | null`
- `setDeckIconCardId(cardId: string | null)`
- `loadDeck(..., deckIconCardId?: string | null)`
- `clearDeck()` resets the icon

When a card is removed, if no remaining entry has that card ID, clear the
explicit icon.

### Builder Adapter

Thread the icon through save/load:

- `getBuilderDeck()` reads `deck.commander_id`
- `saveBuilderDeck()` writes `commander_id`
- Desktop storage keeps existing icon when not supplied only for library-field
  updates; builder saves should send the current builder icon explicitly

### Builder View

Add helper functions near existing deck-view helpers:

- `selectDeckIconCardId(mainDeck, eggDeck, explicitId)`
- `deckEntrySortForIcon(entry)`
- `deckEntryImageCard(entry)`

Render:

- top rail deck-icon block above tabs or directly below the deck heading
- deck row thumbnail using `BuilderCardImage`
- row icon button with accessible label `Set <card name> as deck icon`
- selected state indicator when `entry.cardId === deckIconCardId`

The card image component should remain lazy-loaded. Row thumbnails are fewer
than the pool and should not cause the pool-wide eager-loading issue noted in
`DeckBuilderPage.tsx`.

### Deck Library And Play Selection

Use `deck.deck_icon_card_id` to render a small card-art tile instead of initials.
Fallback to initials only when the field is null or the image fails to load.

## Backend And Desktop Storage

### Desktop Tauri Storage

Update `DeckSummary` and `deck_summary(deck)` in
`code/src-tauri/src/deck_storage.rs` to include `commander_id` and
`deck_icon_card_id`. Compute `deck_icon_card_id` with the same saved-or-fallback
rule used by the frontend. If Tauri storage cannot access full card metadata
directly without adding an undesirable dependency, use the ID-only fallback
ordering for summaries in this module: saved valid `commander_id`, otherwise the
lexicographically first main-deck card ID, otherwise the lexicographically first
egg card ID. The builder itself still uses the full metadata heuristic.

### Browser-Dev Desktop Mirror

Mirror the summary fields in `code/server/routers/desktop_decks.py` so browser
testing sees the same shape as Tauri.

### Hosted API

Update:

- `DeckSummary` schema with `commander_id` and `deck_icon_card_id`
- `_deck_to_summary()`
- `CreateDeckRequest` and `UpdateDeckRequest` handling so non-EDH decks can
  save a representative card ID

The existing hard validation that rejects `commander_id` outside EDH is relaxed
and replaced with deck-icon validation: when provided, the card ID must appear
in the deck. This feature is visual-only and must not affect engine legality or
gameplay.

## Error Handling

- Missing image: show the card name/initials fallback already used by card image
  components.
- Saved icon no longer in deck: render fallback and clear explicit icon when the
  user saves.
- Empty deck: render the neutral placeholder and no selected row indicator.
- Alt-art rows: icon selection is card-ID based; it uses the row's
  `isAltArt` only for the builder rail preview while the summary image uses the
  base card ID.

## Testing

Add focused tests before implementation:

- `deckBuilderView.test.ts`
  - explicit icon wins when present
  - invalid explicit icon falls back
  - high-level main-deck Digimon wins over rookies
  - egg is used only when main deck is empty
  - deterministic tie-break by card ID
- `deckBuilderStore` tests or nearest store coverage
  - loading sets the icon
  - removing the last copy of the icon card clears it
- Desktop storage tests in `deck_storage.rs`
  - summaries include `commander_id` and `deck_icon_card_id`
- Browser-dev route/API tests where existing coverage is available
  - `/desktop-decks` summaries include the new fields
- Hosted deck router/schema tests if DB route tests are active in the current
  suite
  - list summaries expose the visual icon field for standard decks

## Out Of Scope

- Migrating `commander_id` to a new database column in this pass.
- Art-specific saved icons.
- A modal deck-only picker.
- Gameplay semantics for EDH commander cards.
- Changing deck validation or engine rules.

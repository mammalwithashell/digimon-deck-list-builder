## Why

A round of live polish on the desktop deck surfaces left several rough edges worth capturing as spec: the Deck Library had a redundant "armory" descriptor panel duplicating the page title, the Import action was buried in that panel, and opening a deck for editing took multiple clicks; the deck builder still showed a disabled, non-functional sideboard (SIDE pill + tab), its top counts bar ran the card-type tallies straight into the per-level tallies with no separation, and the deck-contents list lumped Lv6 and Lv7 Digimon into a single "LV6+ / MEGA" bucket even though levels only go up to 7 and can be split cleanly.

## What Changes

- **Deck Library:** remove the "// ARMORY / DECK LIBRARY" descriptor panel, move the **Import** action up to the top bar beside Home and New Deck, and make **double-clicking a deck open it in the builder** (single-click still selects/previews).
- **Deck builder — sideboard:** remove the side deck entirely (the SIDE counts pill, the disabled SIDE tab, and the dead "sideboard not supported" branch); the builder is Main + Egg only.
- **Deck builder — counts bar:** add visual separation between the card-type counts (EGG/DIGIMON/TAMER/OPTION) and the per-level counts (L2…L7+).
- **Deck builder — level grouping:** split the deck-contents list's "LV6+ / MEGA" bucket into distinct **LV6** and **LV7** sections.

## Capabilities

### New Capabilities
- `deck-library-navigation`: How the Deck Library presents its primary actions and opens decks — top-bar Home/Import/New Deck (no separate descriptor panel) and double-click-to-edit on a deck tile (single-click selects).
- `deck-builder-chrome`: The deck builder's composition and counts presentation — Main+Egg only (no sideboard), a separated counts bar (type tallies vs per-level tallies), and a deck-contents list grouped by exact Digimon level (Lv2–Lv7, no combined Lv6+ bucket).

### Modified Capabilities
<!-- None. No existing spec governs the Deck Library page layout or the deck builder's chrome/composition; `deck-builder-card-browsing` covers only the card-pool views/filters and `deck-builder-format-selection` only format legality, both unchanged. -->

## Impact

- **Frontend only:**
  - `pages/DeckLibraryPage.tsx` — remove the `library-hero` panel, add an Import button to `library-actions`, add `onOpen` (double-click → `/deckbuilder/:id`) to `DeckTile`; drop the now-unused `Link` import.
  - `pages/DeckBuilderPage.tsx` — remove the SIDE pill, SIDE tab, and `activeSection === 'side'` branch; narrow `activeSection` to `'main' | 'egg'`.
  - `pages/DeckBuilderPage.css` — add the inert `.bld-count.split` rule (margin) that separates the counts groups.
  - `features/deck-builder/deckBuilderView.ts` — add an `lv7` section key/definition, relabel `lv6` to "LV6 / MEGA", add "LV7 / ULTRA", and map level 6 → lv6 / level ≥ 7 → lv7.
- **No backend, engine, RL, or hosted-web changes.** Some now-dead `.library-hero*` CSS remains (matches nothing) and the always-0 `BuilderCounts.side` field is retained to avoid unrelated churn.
- **Tests:** `deckBuilderView.test.ts` gains a Lv6/Lv7 split case; existing builder/library tests stay green.

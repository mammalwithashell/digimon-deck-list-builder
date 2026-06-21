> Retroactive change: all items were implemented and verified against the running desktop build before this spec was written.

## 1. Deck Library navigation

- [x] 1.1 Remove the `library-hero` ("// ARMORY / DECK LIBRARY") descriptor panel from `pages/DeckLibraryPage.tsx`; drop the now-unused `Link` import.
- [x] 1.2 Add an **Import** button to the top-bar `library-actions` (→ `/deckbuilder/new?import=1`), beside Home and New Deck.
- [x] 1.3 Add an `onOpen` prop to `DeckTile` and wire `onDoubleClick` → `/deckbuilder/:id` (both tile lists); keep single-click = select; stop the pin control's double-click from propagating. Add a "click to select, double-click to edit" tooltip.

## 2. Deck builder — remove sideboard

- [x] 2.1 Remove the SIDE counts pill, the disabled SIDE deck-contents tab, and the `activeSection === 'side'` branch from `pages/DeckBuilderPage.tsx`; narrow `activeSection` to `'main' | 'egg'`.

## 3. Deck builder — counts bar spacing

- [x] 3.1 Add the `.bld-count.split` rule (`margin-left`) to `pages/DeckBuilderPage.css` so the type tallies (EGG/DIGIMON/TAMER/OPTION) are separated from the per-level tallies (L2…L7+).

## 4. Deck builder — Lv7 grouping

- [x] 4.1 In `features/deck-builder/deckBuilderView.ts`, add an `lv7` section key + definition ("LV7 / ULTRA"), relabel `lv6` to "LV6 / MEGA", insert `lv7` into `SECTION_ORDER`, and map level 6 → `lv6` / level ≥ 7 → `lv7` in `sectionKeyForEntry`.
- [x] 4.2 Add a `deckBuilderView.test.ts` case asserting a Lv6 + Lv7 deck splits into distinct `LV6 / MEGA` and `LV7 / ULTRA` sections.

## 5. Verification

- [x] 5.1 `deckBuilderView.test.ts` green (10 tests) and `tsc -b` clean.
- [x] 5.2 Live (desktop build via the dev server): Deck Library shows Home/Import/New Deck with no armory panel and double-click opens a deck; the builder shows no SIDE pill/tab, the counts bar has a gap before L2 (`margin-left: 12px`), and Lv6/Lv7 group separately.

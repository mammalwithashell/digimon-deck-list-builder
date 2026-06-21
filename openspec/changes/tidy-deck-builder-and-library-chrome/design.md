## Context

This change retroactively captures a round of live UI polish on the desktop deck surfaces (`pages/DeckLibraryPage.tsx`, `pages/DeckBuilderPage.tsx` + `.css`, `features/deck-builder/deckBuilderView.ts`). The work was implemented and verified directly against the running Tauri build; this spec documents the resulting contracts. All changes are frontend-only and reuse existing patterns (top-bar action buttons, the counts bar, the `groupDeckEntriesForBuilder` section model).

## Goals / Non-Goals

**Goals:**
- Declutter the Deck Library (drop the redundant descriptor panel, relocate Import) and make opening a deck a single gesture (double-click).
- Remove the non-functional sideboard from the builder.
- Make the counts bar legible by separating type tallies from level tallies.
- Group the deck list by exact Digimon level (Lv2–Lv7), not a combined Lv6+ bucket.

**Non-Goals:**
- No backend/persistence/engine changes.
- No change to card-pool filtering or the GRID/DETAIL/DECKLIST view modes (`deck-builder-card-browsing`), nor to format legality (`deck-builder-format-selection`).
- Not removing the always-0 `BuilderCounts.side` data field or the orphaned `.library-hero*` CSS (see Risks).

## Decisions

- **Double-click to edit, single-click to select.** The `DeckTile` keeps its single-click select (loads the banner/analytics) and gains an `onDoubleClick` that navigates to `/deckbuilder/:id`. The pin control stops double-click propagation so pinning never opens the builder. Rationale: preserves the existing preview-on-select UX while adding the faster open gesture; an explicit Edit button already exists on the banner for discoverability/keyboard.
- **Sideboard removed, not hidden.** The SIDE pill, the disabled SIDE tab, and the `activeSection === 'side'` branch are deleted and `activeSection` is narrowed to `'main' | 'egg'`. Rationale: the sideboard was a permanently-disabled stub; deleting it is cleaner than carrying dead UI.
- **Counts separation via the existing `split` marker.** The JSX already tagged the first level count (L2) with a `split` class that had no CSS; this change adds `.bld-count.split { margin-left: 12px }` rather than introducing new markup. The Option cell's existing right border plus the gap reads as the divider.
- **Level grouping adds an `lv7` section.** `sectionKeyForEntry` maps level 6 → `lv6` and level ≥ 7 → `lv7`; `lv6` is relabeled "LV6 / MEGA" and `lv7` is "LV7 / ULTRA". Rationale: levels top out at 7, so the old `≥ 6 → lv6` bucket can split cleanly. "ULTRA" is the community term for Lv7 (超究極体); there is no official Bandai form name, so the label is easily revised.

## Risks / Trade-offs

- **Orphaned CSS / data field left in place.** The `.library-hero*` / `.library-kicker` / `.library-command` rules and the always-0 `BuilderCounts.side` field are now unused. → Left as-is: both are invisible to users, and removing them would mean touching six CSS sites and a counts test for zero behavioral gain. Flagged here so a later sweep can remove them deliberately.
- **Double-click discoverability.** Some users may not try double-click. → Mitigated by the tile's "Click to select, double-click to edit" tooltip and the existing banner Edit button.
- **Lv7 label accuracy.** "ULTRA" is community, not official. → Trivially renamable; isolated to one `SECTION_DEFINITIONS` entry.

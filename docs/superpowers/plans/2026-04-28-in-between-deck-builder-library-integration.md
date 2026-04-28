# In Between Deck Builder Library Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the old deck builder UI with the In Between Theatre deck-builder mock, wired to real deck library loading, saving, importing, validating, and route handoff.

**Architecture:** Keep `DeckLibraryPage` as the deck organization hub and make `/deckbuilder/new`, `/deckbuilder/:id`, and `/deckbuilder/new?import=1` render a mock-derived builder shell. Reuse the existing `useDeckBuilderStore`, `ImportExport`, card API, validation API, and deck library storage paths; add a small builder adapter and view-model helpers so the page can work in web, real Tauri desktop, and desktop-mode Playwright without branching throughout the UI.

**Tech Stack:** React 19, React Router 7, Zustand, TypeScript, Vite desktop mode, Playwright, Vitest, FastAPI/Tauri deck storage adapters.

---

## Scope Check

This plan intentionally covers only the deck builder and its library integration. It does not redesign the battle board, matchmaking, or room lobby. The mock source is `C:\Users\james\Downloads\In Between Theatre(1).zip`, specifically:

- `deck-builder.jsx`
- `deck-builder.css`
- `deck-data.jsx`
- `deck-library.jsx` only for route/link context, not another library rewrite

Current app behavior to preserve:

- `/deckbuilder` remains the deck library.
- `/deckbuilder/new` creates a new deck.
- `/deckbuilder/:id` edits a saved deck from the library.
- `/deckbuilder/new?import=1` opens import/export immediately.
- `/deckbuilder/new?returnTo=play` can return to `/play/deck`.
- Desktop real app uses Tauri deck storage.
- Desktop-mode Playwright without a Tauri bridge uses HTTP mocks.
- Web build uses FastAPI deck routes.

Known design decision: the mock includes a sideboard tab. The real deck model has only `main_deck` and `egg_deck`, so Task 4 renders the side tab disabled with `"SIDEBOARD NOT SUPPORTED IN STANDARD"` rather than adding fake sideboard persistence.

---

## File Structure

### New Files

- `code/frontend/src/features/deck-builder/deckBuilderAdapter.ts`
  - One backend surface for builder load/save/list across web, Tauri desktop, and desktop-mode browser tests.
- `code/frontend/src/features/deck-builder/deckBuilderView.ts`
  - Pure helpers for counts, card presentation, filter matching, deck flattening, and section grouping.
- `code/frontend/src/features/deck-builder/deckBuilderView.test.ts`
  - Unit tests for the helper contract.
- `code/frontend/src/pages/DeckBuilderPage.css`
  - Mock-derived builder CSS, scoped to `.deck-builder-page` and `.deck-builder-app`.
- `code/frontend/e2e/deck-builder.spec.ts`
  - End-to-end coverage for builder route, library edit route, import route, add/remove, save, and return-to-play.

### Modified Files

- `code/frontend/src/pages/DeckBuilderPage.tsx`
  - Replace the old Tailwind layout with the mock-derived builder surface.
  - Keep existing route loading, import modal, validation, and save behavior through the new adapter.
- `code/frontend/src/pages/DeckLibraryPage.tsx`
  - Ensure all builder entry points land on the new builder route: Edit, New Deck, Import.
- `code/frontend/e2e/deck-library.spec.ts`
  - Update assertions so library-to-builder route expects the new builder chrome instead of old controls only.

### Left Alone

- `code/frontend/components/deckbuilder/*`
  - Old components can remain for now. Do not delete in this plan; remove them only in a later cleanup if no imports remain.
- `code/frontend/src/stores/deckBuilderStore.ts`
  - Reuse the existing store; do not create a parallel builder state model.

---

## Task 1: Builder View-Model Helpers

**Files:**
- Create: `code/frontend/src/features/deck-builder/deckBuilderView.ts`
- Create: `code/frontend/src/features/deck-builder/deckBuilderView.test.ts`

- [ ] **Step 1: Write helper tests**

Create `code/frontend/src/features/deck-builder/deckBuilderView.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import type { DigimonCardData } from '@/types/cards';
import type { DeckEntry } from '@/types/deck';
import {
  builderCardColorClass,
  deckEntriesToSlotArrays,
  filterBuilderCards,
  getBuilderCounts,
  groupDeckEntriesForBuilder,
  slotArraysToDeckEntries,
} from './deckBuilderView';

const card = (overrides: Partial<DigimonCardData>): DigimonCardData => ({
  name: 'Agumon',
  type: 'Digimon',
  color: 'Red',
  stage: 'Rookie',
  digi_type: 'Reptile',
  attribute: 'Vaccine',
  level: '3',
  play_cost: '3',
  evolution_cost: '0',
  cardrarity: 'C',
  artist: '',
  dp: '2000',
  cardnumber: 'BT1-001',
  maineffect: '[On Play] Draw 1.',
  soureeffect: '',
  set_name: 'BT1',
  card_sets: ['BT1'],
  image_url: '',
  ...overrides,
});

describe('deckBuilderView', () => {
  it('maps card colors into mock builder classes', () => {
    expect(builderCardColorClass(card({ color: 'Red' }))).toBe('r');
    expect(builderCardColorClass(card({ color: 'Blue' }))).toBe('b');
    expect(builderCardColorClass(card({ color: 'Purple' }))).toBe('p');
    expect(builderCardColorClass(card({ color: 'White' }))).toBe('w');
  });

  it('filters card pool by search, color, type, level, rarity, inherited, and security flags', () => {
    const pool = [
      card({ cardnumber: 'BT1-001', name: 'Agumon', color: 'Red', type: 'Digimon', level: '3', cardrarity: 'C', soureeffect: '[Your Turn] +1000 DP.' }),
      card({ cardnumber: 'BT1-002', name: 'Greymon', color: 'Blue', type: 'Digimon', level: '4', cardrarity: 'R', soureeffect: 'Security: Draw 1.' }),
      card({ cardnumber: 'BT1-003', name: 'Tai Kamiya', color: 'Red', type: 'Tamer', level: '', cardrarity: 'U' }),
    ];

    expect(filterBuilderCards(pool, { search: 'agu', colors: ['Red'], type: 'Digimon', level: '3', rarity: 'all', inheritedOnly: true, securityOnly: false }).map((c) => c.cardnumber)).toEqual(['BT1-001']);
    expect(filterBuilderCards(pool, { search: '', colors: [], type: 'all', level: 'all', rarity: 'R', inheritedOnly: false, securityOnly: true }).map((c) => c.cardnumber)).toEqual(['BT1-002']);
  });

  it('computes main, egg, type, and level counts from deck entries', () => {
    const main: DeckEntry[] = [
      { cardId: 'BT1-001', isAltArt: false, count: 4, cardData: card({ type: 'Digimon', level: '3' }) },
      { cardId: 'BT1-003', isAltArt: false, count: 2, cardData: card({ type: 'Tamer', level: '', cardnumber: 'BT1-003' }) },
    ];
    const egg: DeckEntry[] = [
      { cardId: 'BT1-004', isAltArt: false, count: 4, cardData: card({ type: 'Digi-Egg', level: '2', cardnumber: 'BT1-004' }) },
    ];

    expect(getBuilderCounts(main, egg)).toMatchObject({
      main: 6,
      egg: 4,
      total: 10,
      digimon: 4,
      tamer: 2,
      option: 0,
      lv2: 4,
      lv3: 4,
    });
  });

  it('round-trips saved slot arrays and grouped entries', () => {
    const cardMap = new Map([
      ['BT1-001', card({ cardnumber: 'BT1-001' })],
      ['BT1-002', card({ cardnumber: 'BT1-002', type: 'Digi-Egg', level: '2' })],
    ]);
    const entries = slotArraysToDeckEntries(['BT1-001', 'BT1-001'], [true, false], cardMap);

    expect(entries).toEqual([
      { cardId: 'BT1-001', isAltArt: true, count: 1, cardData: cardMap.get('BT1-001') },
      { cardId: 'BT1-001', isAltArt: false, count: 1, cardData: cardMap.get('BT1-001') },
    ]);
    expect(deckEntriesToSlotArrays(entries)).toEqual({
      ids: ['BT1-001', 'BT1-001'],
      altArts: [true, false],
    });
    expect(groupDeckEntriesForBuilder([{ cardId: 'BT1-002', isAltArt: false, count: 2, cardData: cardMap.get('BT1-002') }])[0]?.label).toBe('LV2 / DIGI-EGG');
  });
});
```

- [ ] **Step 2: Run helper tests and verify they fail**

Run:

```powershell
cd code/frontend
npm test -- src/features/deck-builder/deckBuilderView.test.ts
```

Expected: fail because `deckBuilderView.ts` does not exist.

- [ ] **Step 3: Implement helper module**

Create `code/frontend/src/features/deck-builder/deckBuilderView.ts`:

```ts
import type { DigimonCardData } from '@/types/cards';
import type { DeckEntry } from '@/types/deck';

export interface BuilderCardFilters {
  search: string;
  colors: string[];
  type: string;
  level: string;
  rarity: string;
  inheritedOnly: boolean;
  securityOnly: boolean;
}

export interface BuilderCounts {
  main: number;
  egg: number;
  side: number;
  total: number;
  eggCards: number;
  digimon: number;
  tamer: number;
  option: number;
  lv2: number;
  lv3: number;
  lv4: number;
  lv5: number;
  lv6: number;
  lv7: number;
}

export interface BuilderSection {
  label: string;
  expected: number;
  total: number;
  entries: DeckEntry[];
}

const COLOR_CLASS: Record<string, string> = {
  red: 'r',
  blue: 'b',
  yellow: 'y',
  green: 'g',
  purple: 'p',
  black: 'k',
  white: 'w',
};

export function builderCardColorClass(card: Pick<DigimonCardData, 'color'> | undefined): string {
  return COLOR_CLASS[(card?.color ?? '').toLowerCase()] ?? 'k';
}

export function hasInheritedEffect(card: DigimonCardData): boolean {
  return Boolean(card.soureeffect?.trim());
}

export function hasSecurityEffect(card: DigimonCardData): boolean {
  return /security/i.test(`${card.maineffect ?? ''} ${card.soureeffect ?? ''}`);
}

export function filterBuilderCards(
  cards: DigimonCardData[],
  filters: BuilderCardFilters,
): DigimonCardData[] {
  const search = filters.search.trim().toLowerCase();
  return cards.filter((card) => {
    if (filters.colors.length > 0 && !filters.colors.includes(card.color)) return false;
    if (filters.type !== 'all' && card.type !== filters.type) return false;
    if (filters.level !== 'all' && card.level !== filters.level) return false;
    if (filters.rarity !== 'all' && card.cardrarity !== filters.rarity) return false;
    if (filters.inheritedOnly && !hasInheritedEffect(card)) return false;
    if (filters.securityOnly && !hasSecurityEffect(card)) return false;
    if (!search) return true;
    const haystack = [
      card.name,
      card.cardnumber,
      card.maineffect,
      card.soureeffect,
      card.digi_type,
      card.attribute,
      card.set_name,
    ].join(' ').toLowerCase();
    return haystack.includes(search);
  });
}

export function deckEntryCardCount(entries: DeckEntry[]): number {
  return entries.reduce((sum, entry) => sum + entry.count, 0);
}

export function getBuilderCounts(mainDeck: DeckEntry[], eggDeck: DeckEntry[]): BuilderCounts {
  const counts: BuilderCounts = {
    main: deckEntryCardCount(mainDeck),
    egg: deckEntryCardCount(eggDeck),
    side: 0,
    total: deckEntryCardCount(mainDeck) + deckEntryCardCount(eggDeck),
    eggCards: 0,
    digimon: 0,
    tamer: 0,
    option: 0,
    lv2: 0,
    lv3: 0,
    lv4: 0,
    lv5: 0,
    lv6: 0,
    lv7: 0,
  };

  for (const entry of [...eggDeck, ...mainDeck]) {
    const card = entry.cardData;
    if (!card) continue;
    if (card.type === 'Digi-Egg') counts.eggCards += entry.count;
    else if (card.type === 'Digimon') counts.digimon += entry.count;
    else if (card.type === 'Tamer') counts.tamer += entry.count;
    else if (card.type === 'Option') counts.option += entry.count;

    const level = Number.parseInt(card.level, 10);
    if (level === 2) counts.lv2 += entry.count;
    else if (level === 3) counts.lv3 += entry.count;
    else if (level === 4) counts.lv4 += entry.count;
    else if (level === 5) counts.lv5 += entry.count;
    else if (level === 6) counts.lv6 += entry.count;
    else if (level >= 7) counts.lv7 += entry.count;
  }

  return counts;
}

export function slotArraysToDeckEntries(
  ids: string[],
  altArts: boolean[] = [],
  cardMap: Map<string, DigimonCardData>,
): DeckEntry[] {
  const counts = new Map<string, DeckEntry>();
  ids.forEach((cardId, index) => {
    const isAltArt = Boolean(altArts[index]);
    const key = `${cardId}|${isAltArt ? '1' : '0'}`;
    const existing = counts.get(key);
    if (existing) existing.count += 1;
    else counts.set(key, { cardId, isAltArt, count: 1, cardData: cardMap.get(cardId) });
  });
  return Array.from(counts.values());
}

export function deckEntriesToSlotArrays(entries: DeckEntry[]): { ids: string[]; altArts: boolean[] } {
  const ids: string[] = [];
  const altArts: boolean[] = [];
  for (const entry of entries) {
    for (let i = 0; i < entry.count; i += 1) {
      ids.push(entry.cardId);
      altArts.push(Boolean(entry.isAltArt));
    }
  }
  return { ids, altArts };
}

function entrySortValue(entry: DeckEntry): number {
  const card = entry.cardData;
  if (!card) return 99;
  if (card.type === 'Digi-Egg') return 2;
  if (card.type === 'Tamer') return 90;
  if (card.type === 'Option') return 91;
  return Number.parseInt(card.level, 10) || 99;
}

export function groupDeckEntriesForBuilder(entries: DeckEntry[]): BuilderSection[] {
  const groups: Record<string, { expected: number; entries: DeckEntry[] }> = {
    'LV2 / DIGI-EGG': { expected: 5, entries: [] },
    'LV3 / ROOKIE': { expected: 0, entries: [] },
    'LV4 / CHAMPION': { expected: 0, entries: [] },
    'LV5 / ULTIMATE': { expected: 0, entries: [] },
    'LV6+ / MEGA': { expected: 0, entries: [] },
    TAMER: { expected: 0, entries: [] },
    OPTION: { expected: 0, entries: [] },
    OTHER: { expected: 0, entries: [] },
  };

  for (const entry of entries) {
    const card = entry.cardData;
    const level = Number.parseInt(card?.level ?? '', 10);
    let label = 'OTHER';
    if (card?.type === 'Digi-Egg' || level === 2) label = 'LV2 / DIGI-EGG';
    else if (card?.type === 'Tamer') label = 'TAMER';
    else if (card?.type === 'Option') label = 'OPTION';
    else if (level === 3) label = 'LV3 / ROOKIE';
    else if (level === 4) label = 'LV4 / CHAMPION';
    else if (level === 5) label = 'LV5 / ULTIMATE';
    else if (level >= 6) label = 'LV6+ / MEGA';
    groups[label]!.entries.push(entry);
  }

  return Object.entries(groups)
    .filter(([, group]) => group.entries.length > 0)
    .map(([label, group]) => ({
      label,
      expected: group.expected,
      total: deckEntryCardCount(group.entries),
      entries: [...group.entries].sort((a, b) => {
        const sort = entrySortValue(a) - entrySortValue(b);
        if (sort !== 0) return sort;
        return (a.cardData?.name ?? a.cardId).localeCompare(b.cardData?.name ?? b.cardId);
      }),
    }));
}
```

- [ ] **Step 4: Run helper tests and commit**

Run:

```powershell
cd code/frontend
npm test -- src/features/deck-builder/deckBuilderView.test.ts
```

Expected: pass. Commit:

```powershell
git add code/frontend/src/features/deck-builder/deckBuilderView.ts code/frontend/src/features/deck-builder/deckBuilderView.test.ts
git commit -m "feat: add deck builder view helpers"
```

---

## Task 2: Builder Storage Adapter

**Files:**
- Create: `code/frontend/src/features/deck-builder/deckBuilderAdapter.ts`
- Modify: `code/frontend/src/pages/DeckBuilderPage.tsx`

- [ ] **Step 1: Create the adapter**

Create `code/frontend/src/features/deck-builder/deckBuilderAdapter.ts`:

```ts
import * as deckApi from '@/api/deckApi';
import * as deckLibrary from '@/api/deckLibraryAdapter';
import * as deckStore from '@/storage/deckStore';
import type { DeckResponse, DeckSummary } from '@/types/deck';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

function hasTauriBridge(): boolean {
  return Boolean((globalThis as { isTauri?: boolean }).isTauri);
}

function usesDesktopStore(): boolean {
  return IS_DESKTOP && hasTauriBridge();
}

export async function listBuilderDecks(): Promise<DeckSummary[]> {
  return deckLibrary.listDecks();
}

export async function getBuilderDeck(deckId: string): Promise<DeckResponse> {
  return deckLibrary.getDeck(deckId);
}

export async function saveBuilderDeck(params: {
  deckId: string | null;
  ownerId: string;
  name: string;
  mainDeck: string[];
  eggDeck: string[];
  mainDeckAltArts: boolean[];
  eggDeckAltArts: boolean[];
}): Promise<DeckResponse> {
  if (usesDesktopStore()) {
    const existing = params.deckId ? await deckStore.getDeck(params.deckId) : null;
    return deckStore.putDeck({
      ...(existing ?? {}),
      id: params.deckId ?? undefined,
      owner_id: existing?.owner_id ?? params.ownerId,
      name: params.name,
      game_mode: existing?.game_mode ?? 'standard',
      main_deck: params.mainDeck,
      egg_deck: params.eggDeck,
      main_deck_alt_arts: params.mainDeckAltArts,
      egg_deck_alt_arts: params.eggDeckAltArts,
    });
  }

  if (params.deckId) {
    return deckApi.updateDeck(params.deckId, {
      name: params.name,
      main_deck: params.mainDeck,
      egg_deck: params.eggDeck,
      main_deck_alt_arts: params.mainDeckAltArts,
      egg_deck_alt_arts: params.eggDeckAltArts,
    });
  }

  return deckApi.createDeck({
    name: params.name,
    game_mode: 'standard',
    main_deck: params.mainDeck,
    egg_deck: params.eggDeck,
    main_deck_alt_arts: params.mainDeckAltArts,
    egg_deck_alt_arts: params.eggDeckAltArts,
  });
}
```

- [ ] **Step 2: Update old builder imports to use the adapter**

In `code/frontend/src/pages/DeckBuilderPage.tsx`, replace direct builder load/save imports:

```ts
import * as deckApi from '@/api/deckApi';
import * as deckStore from '@/storage/deckStore';
```

with:

```ts
import * as deckApi from '@/api/deckApi';
import {
  getBuilderDeck,
  saveBuilderDeck,
} from '@/features/deck-builder/deckBuilderAdapter';
```

Then change the route deck loader from:

```ts
const deck = await decks.getDeck(routeDeckId!);
```

to:

```ts
const deck = await getBuilderDeck(routeDeckId!);
```

Replace both desktop and web branches inside `handleSave` with one adapter call:

```ts
const saved = await saveBuilderDeck({
  deckId,
  ownerId: useAuthStore.getState().user?.id ?? 'guest',
  name: deckName,
  mainDeck: mainIds,
  eggDeck: eggIds,
  mainDeckAltArts: mainAlts,
  eggDeckAltArts: eggAlts,
});
setDeckId(saved.id);
if (returnToPlay) {
  navigate('/play/deck', { replace: true });
} else if (!routeDeckId) {
  navigate(`/deckbuilder/${saved.id}`, { replace: true });
}
```

Remove these now-unused lines:

```ts
const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';
const decks = IS_DESKTOP ? deckStore : deckApi;
```

- [ ] **Step 3: Build and commit**

Run:

```powershell
cd code/frontend
npm run build:desktop
```

Expected: TypeScript passes. Commit:

```powershell
git add code/frontend/src/features/deck-builder/deckBuilderAdapter.ts code/frontend/src/pages/DeckBuilderPage.tsx
git commit -m "feat: add deck builder storage adapter"
```

---

## Task 3: Real Builder Route E2E Spec

**Files:**
- Create: `code/frontend/e2e/deck-builder.spec.ts`

- [ ] **Step 1: Write the failing E2E spec**

Create `code/frontend/e2e/deck-builder.spec.ts`:

```ts
import { expect, test, type Page } from '@playwright/test';

const cards = [
  {
    id: 'BT1-001',
    name: 'Agumon',
    type: 'Digimon',
    color: 'Red',
    stage: 'Rookie',
    digi_type: 'Reptile',
    attribute: 'Vaccine',
    level: 3,
    play_cost: 3,
    evolution_cost: 0,
    rarity: 'C',
    artist: '',
    dp: 2000,
    main_effect: '[On Play] Reveal the top 3 cards.',
    source_effect: '[Your Turn] +1000 DP.',
    set_name: ['BT1'],
  },
  {
    id: 'BT1-002',
    name: 'Koromon',
    type: 'Digi-Egg',
    color: 'Red',
    stage: 'In-Training',
    digi_type: 'Lesser',
    attribute: '',
    level: 2,
    play_cost: null,
    evolution_cost: null,
    rarity: 'U',
    artist: '',
    dp: null,
    main_effect: '',
    source_effect: '[Your Turn] Draw 1.',
    set_name: ['BT1'],
  },
];

async function mockBuilderBackend(page: Page) {
  let savedDeck = {
    id: 'deck-1',
    owner_id: 'u1',
    folder_id: null,
    name: 'Blue Flare',
    description: '',
    game_mode: 'standard',
    main_deck: ['BT1-001', 'BT1-001'],
    egg_deck: ['BT1-002'],
    main_deck_alt_arts: [false, false],
    egg_deck_alt_arts: [false],
    commander_id: null,
    is_valid: false,
    validation_errors: [],
    is_public: false,
    is_pinned: false,
    tags: [],
    meta_tier: 'rogue',
    meta_archetype: 'Red Aggro',
    created_at: '2026-04-28T00:00:00.000Z',
    updated_at: '2026-04-28T00:00:00.000Z',
  };

  await page.addInitScript(() => {
    localStorage.setItem('access_token', 'test-token');
    localStorage.setItem('guest_access_token', 'test-token');
    localStorage.setItem('guest_user_id', 'u1');
    localStorage.setItem('guest_display_name', 'tester');
  });
  await page.route('**/auth/guest', (route) =>
    route.fulfill({ json: { access_token: 'test-token', user_id: 'u1', display_name: 'tester' } }),
  );
  await page.route('**/users/me', (route) =>
    route.fulfill({ json: { id: 'u1', username: 'tester', email: 't@example.com', roles: [] } }),
  );
  await page.route('**/decks/tested-cards', (route) =>
    route.fulfill({ json: { card_ids: ['BT1-001', 'BT1-002'], card_count: 2 } }),
  );
  await page.route('**/decks/folders', (route) => route.fulfill({ json: [] }));
  await page.route('**/decks/deck-1', (route) => route.fulfill({ json: savedDeck }));
  await page.route('**/decks', (route) => {
    if (route.request().method() === 'GET') {
      return route.fulfill({
        json: [{
          id: savedDeck.id,
          name: savedDeck.name,
          description: savedDeck.description,
          game_mode: savedDeck.game_mode,
          is_valid: savedDeck.is_valid,
          is_public: savedDeck.is_public,
          is_pinned: savedDeck.is_pinned,
          folder_id: savedDeck.folder_id,
          card_count: savedDeck.main_deck.length + savedDeck.egg_deck.length,
          main_count: savedDeck.main_deck.length,
          egg_count: savedDeck.egg_deck.length,
          tags: savedDeck.tags,
          meta_tier: savedDeck.meta_tier,
          meta_archetype: savedDeck.meta_archetype,
          colors: ['Red'],
          highest_level: 3,
          created_at: savedDeck.created_at,
          updated_at: savedDeck.updated_at,
        }],
      });
    }
    const body = route.request().postDataJSON();
    savedDeck = { ...savedDeck, ...body, id: 'created-deck' };
    return route.fulfill({ json: savedDeck });
  });
  await page.route('**/decks/deck-1/validate', (route) => route.fulfill({ json: { ...savedDeck, is_valid: true, validation_errors: [] } }));
  await page.route('**/decks/validate', (route) => route.fulfill({ json: { is_valid: true, errors: [], warnings: [] } }));
  await page.route('**/decks/parse', (route) => route.fulfill({ json: { main_deck: ['BT1-001'], egg_deck: ['BT1-002'], warnings: [] } }));
  await page.route('https://digimoncard.io/**', (route) => {
    const url = new URL(route.request().url());
    const cardParam = url.searchParams.get('card');
    const nameParam = url.searchParams.get('n')?.toLowerCase();
    const out = cards.filter((card) => {
      if (cardParam) return card.id.startsWith(cardParam);
      if (nameParam) return card.name.toLowerCase().includes(nameParam);
      return true;
    });
    return route.fulfill({ json: out });
  });
}

test.describe('In Between deck builder', () => {
  test('renders the mock builder shell, edits deck counts, validates, and saves', async ({ page }) => {
    await mockBuilderBackend(page);
    await page.goto('/deckbuilder/deck-1');

    await expect(page.getByText('CARD POOL')).toBeVisible();
    await expect(page.getByText('DECK CONTENTS')).toBeVisible();
    await expect(page.getByText('Blue Flare')).toBeVisible();
    await expect(page.getByText('MAIN').first()).toBeVisible();

    await page.getByPlaceholder('NAME, ID, KEYWORD...').fill('Agumon');
    await expect(page.getByRole('button', { name: /Agumon BT1-001/i })).toBeVisible();
    await page.getByRole('button', { name: /Agumon BT1-001/i }).click();
    await expect(page.getByText('3/50')).toBeVisible();

    await page.getByRole('button', { name: 'VALIDATE' }).click();
    await expect(page.getByText('Deck is valid')).toBeVisible();

    await page.getByRole('button', { name: /SAVE/i }).click();
    await expect(page).toHaveURL(/\/deckbuilder\/deck-1$/);
  });

  test('new deck import route opens the mock builder with import modal', async ({ page }) => {
    await mockBuilderBackend(page);
    await page.goto('/deckbuilder/new?import=1');

    await expect(page.getByText('CARD POOL')).toBeVisible();
    await expect(page.getByText('DECK CONTENTS')).toBeVisible();
    await expect(page.getByText('Import / Export Deck')).toBeVisible();
  });

  test('library edit opens the new builder shell', async ({ page }) => {
    await mockBuilderBackend(page);
    await page.goto('/deckbuilder');
    await page.getByRole('button', { name: 'Edit' }).click();
    await expect(page).toHaveURL(/\/deckbuilder\/deck-1$/);
    await expect(page.getByText('CARD POOL')).toBeVisible();
    await expect(page.getByText('DECK CONTENTS')).toBeVisible();
  });
});
```

- [ ] **Step 2: Run E2E and verify it fails**

Run desktop-mode Vite and the spec:

```powershell
cd code/frontend
$p = Start-Process -FilePath npm.cmd -ArgumentList @('run','dev:desktop','--','--host','127.0.0.1','--port','5174') -WorkingDirectory (Get-Location) -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 5
npm run e2e -- deck-builder.spec.ts
Stop-Process -Id $p.Id -Force
```

Expected: fail because the old builder does not expose the mock `CARD POOL` and `DECK CONTENTS` surface.

- [ ] **Step 3: Commit the failing test**

Commit the red spec so execution stays TDD-traceable:

```powershell
git add code/frontend/e2e/deck-builder.spec.ts
git commit -m "test: cover in between deck builder shell"
```

---

## Task 4: Replace DeckBuilderPage With Mock-Derived Real UI

**Files:**
- Modify: `code/frontend/src/pages/DeckBuilderPage.tsx`
- Create: `code/frontend/src/pages/DeckBuilderPage.css`

- [ ] **Step 1: Replace the page imports**

In `code/frontend/src/pages/DeckBuilderPage.tsx`, remove these old UI imports:

```ts
import { CardSearchPanel } from '@/components/deckbuilder/CardSearchPanel';
import { DeckListPanel } from '@/components/deckbuilder/DeckListPanel';
import { DeckStats } from '@/components/deckbuilder/DeckStats';
import { DeckSelector } from '@/components/deckbuilder/DeckSelector';
import { ValidationPanel } from '@/components/deckbuilder/ValidationPanel';
import { CardDetail } from '@/components/shared/CardDetail';
```

Add:

```ts
import { Link } from 'react-router-dom';
import type { DigimonCardData } from '@/types/cards';
import {
  builderCardColorClass,
  deckEntriesToSlotArrays,
  filterBuilderCards,
  getBuilderCounts,
  groupDeckEntriesForBuilder,
  slotArraysToDeckEntries,
  type BuilderCardFilters,
} from '@/features/deck-builder/deckBuilderView';
import { listBuilderDecks, getBuilderDeck, saveBuilderDeck } from '@/features/deck-builder/deckBuilderAdapter';
import './DeckBuilderPage.css';
```

- [ ] **Step 2: Add page-local constants and helpers**

Add below imports:

```ts
const COLORS = ['Red', 'Blue', 'Yellow', 'Green', 'Black', 'Purple', 'White'];
const TYPES = ['all', 'Digimon', 'Tamer', 'Option', 'Digi-Egg'];
const LEVELS = ['all', '2', '3', '4', '5', '6', '7'];
const RARITIES = ['all', 'C', 'U', 'R', 'SR', 'SEC', 'P'];

function cardButtonName(card: DigimonCardData): string {
  return `${card.name} ${card.cardnumber}`;
}

function uniqueCards(cards: Array<DigimonCardData | null | undefined>): DigimonCardData[] {
  const out = new Map<string, DigimonCardData>();
  for (const card of cards) {
    if (!card?.cardnumber) continue;
    const key = `${card.cardnumber}|${card.isAltArt ? '1' : '0'}`;
    out.set(key, card);
  }
  return Array.from(out.values());
}
```

- [ ] **Step 3: Add builder page state**

Inside `DeckBuilderPage`, keep the existing store state but add:

```ts
const [cardPool, setCardPool] = useState<DigimonCardData[]>([]);
const [previewCard, setPreviewCard] = useState<DigimonCardData | null>(null);
const [activeSection, setActiveSection] = useState<'main' | 'egg' | 'side'>('main');
const [builderFilters, setBuilderFilters] = useState<BuilderCardFilters>({
  search: '',
  colors: [],
  type: 'all',
  level: 'all',
  rarity: 'all',
  inheritedOnly: false,
  securityOnly: false,
});
const [notice, setNotice] = useState('');
```

Also destructure these store fields/actions:

```ts
searchResults,
setSearchQuery,
setFilters,
setSearchResults,
setIsSearching,
addCardToDeck,
removeCardFromDeck,
savedDecks,
setSavedDecks,
```

- [ ] **Step 4: Load tested card metadata for the initial pool**

Add this effect after the existing tested-card allowlist effect:

```ts
useEffect(() => {
  if (!testedCardIds || cardPool.length > 0) return;
  let cancelled = false;
  async function loadInitialPool() {
    const ids = Array.from(testedCardIds).slice(0, 80);
    const results = await Promise.allSettled(ids.map((id) => getCardById(id)));
    if (cancelled) return;
    const cards = uniqueCards(results.map((result) => (result.status === 'fulfilled' ? result.value : null)));
    setCardPool(cards);
    setSearchResults(cards);
    setPreviewCard((current) => current ?? cards[0] ?? null);
  }
  void loadInitialPool();
  return () => {
    cancelled = true;
  };
}, [cardPool.length, setSearchResults, testedCardIds]);
```

- [ ] **Step 5: Update route deck loading with helper conversion**

Replace the body after `const deck = await getBuilderDeck(routeDeckId!)` with:

```ts
const ids = [...new Set([...deck.main_deck, ...deck.egg_deck])];
const cardPairs = await Promise.allSettled(
  ids.map(async (id) => [id, await getCardById(id)] as const),
);
const cardMap = new Map<string, DigimonCardData>();
for (const result of cardPairs) {
  if (result.status === 'fulfilled' && result.value[1]) {
    cardMap.set(result.value[0], result.value[1]);
  }
}
const mainEntries = slotArraysToDeckEntries(deck.main_deck, deck.main_deck_alt_arts, cardMap);
const eggEntries = slotArraysToDeckEntries(deck.egg_deck, deck.egg_deck_alt_arts, cardMap);
const loadedCards = uniqueCards([...cardMap.values(), ...cardPool]);
if (!cancelled) {
  loadDeck(deck.id, deck.name, mainEntries, eggEntries);
  setCardPool(loadedCards);
  setSearchResults(loadedCards);
  setPreviewCard((current) => current ?? loadedCards[0] ?? null);
}
```

- [ ] **Step 6: Implement search/filter bridge**

Add this effect:

```ts
useEffect(() => {
  const search = builderFilters.search.trim();
  setSearchQuery(search);
  setFilters({
    color: builderFilters.colors,
    type: builderFilters.type === 'all' ? [] : [builderFilters.type],
    level: builderFilters.level === 'all' ? [] : [builderFilters.level],
    rarity: builderFilters.rarity === 'all' ? '' : builderFilters.rarity,
  });
}, [builderFilters, setFilters, setSearchQuery]);
```

Add this effect to fetch remote search results when the user types:

```ts
useEffect(() => {
  const search = builderFilters.search.trim();
  if (!search) return;
  let cancelled = false;
  async function runSearch() {
    setIsSearching(true);
    try {
      const results = await getCardById(search).then((card) => (card ? [card] : []));
      if (cancelled) return;
      const next = uniqueCards([...results, ...cardPool]);
      setCardPool(next);
      setSearchResults(next);
      setPreviewCard((current) => current ?? next[0] ?? null);
    } finally {
      if (!cancelled) setIsSearching(false);
    }
  }
  if (/^[A-Z]{2,}\d{1,2}-\d{3}$/i.test(search)) {
    void runSearch();
  }
  return () => {
    cancelled = true;
  };
}, [builderFilters.search, cardPool, setIsSearching, setSearchResults]);
```

- [ ] **Step 7: Replace `handleSave` with adapter flattening**

Inside `handleSave`, replace manual `flatMap` arrays with:

```ts
const main = deckEntriesToSlotArrays(mainDeck);
const eggs = deckEntriesToSlotArrays(eggDeck);
const saved = await saveBuilderDeck({
  deckId,
  ownerId: useAuthStore.getState().user?.id ?? 'guest',
  name: deckName,
  mainDeck: main.ids,
  eggDeck: eggs.ids,
  mainDeckAltArts: main.altArts,
  eggDeckAltArts: eggs.altArts,
});
setDeckId(saved.id);
setNotice('Saved');
setIsDirty(false);
void listBuilderDecks().then(setSavedDecks).catch(() => {});
if (returnToPlay) {
  navigate('/play/deck', { replace: true });
} else if (!routeDeckId) {
  navigate(`/deckbuilder/${saved.id}`, { replace: true });
}
```

- [ ] **Step 8: Replace JSX with mock-derived layout**

Replace the old `return (...)` with this structure:

```tsx
const counts = getBuilderCounts(mainDeck, eggDeck);
const visibleCards = filterBuilderCards(searchResults.length ? searchResults : cardPool, builderFilters);
const mainSections = groupDeckEntriesForBuilder(mainDeck);
const eggSections = groupDeckEntriesForBuilder(eggDeck);
const visibleSections = activeSection === 'egg' ? eggSections : mainSections;
const activeCards = activeSection === 'egg' ? eggDeck : mainDeck;
const activeExpected = activeSection === 'egg' ? 5 : 50;

return (
  <div className="deck-builder-page">
    <div className="deck-builder-app">
      <div className="bld">
        <header className="bld-top">
          <div className="left">
            <Link className="back" to={returnToPlay ? '/play/deck' : '/deckbuilder'}>
              ← LIBRARY
            </Link>
            <input
              aria-label="Deck name"
              className="deck-name-input"
              value={deckName}
              onChange={(event) => setDeckName(event.target.value)}
            />
            <span className="pill"><span className="v">{counts.main}</span>/50</span>
            <span className="pill">EGG <span className="v">{counts.egg}</span>/5</span>
            <span className="pill disabled">SIDE <span className="v">0</span>/15</span>
          </div>
          <div className="bld-counts">
            <div className={`bld-count ${counts.egg >= 4 ? 'ok' : ''}`}><span className="v">{counts.eggCards}</span>EGG</div>
            <div className="bld-count"><span className="v player">{counts.digimon}</span>DIGIMON</div>
            <div className="bld-count"><span className="v">{counts.tamer}</span>TAMER</div>
            <div className="bld-count"><span className="v">{counts.option}</span>OPTION</div>
            <div className="bld-count"><span className="v">{counts.lv2}</span>L2</div>
            <div className="bld-count"><span className="v">{counts.lv3}</span>L3</div>
            <div className="bld-count"><span className="v">{counts.lv4}</span>L4</div>
            <div className="bld-count"><span className="v">{counts.lv5}</span>L5</div>
            <div className="bld-count"><span className="v">{counts.lv6}</span>L6</div>
            <div className="bld-count"><span className="v">{counts.lv7}</span>L7+</div>
          </div>
          <div className="right">
            {notice && <span className="bld-notice">{notice}</span>}
            <button type="button" className="btn btn-good" onClick={handleSave} disabled={saving || !isDirty}>
              {saving ? 'SAVING...' : 'SAVE'}
            </button>
            <button type="button" className="btn btn-opp" onClick={handleValidate}>VALIDATE</button>
            <button type="button" className="btn btn-ghost" onClick={() => setShowImport(true)}>IMPORT</button>
            <button type="button" className="btn btn-danger" onClick={clearDeck}>CLEAR</button>
          </div>
        </header>

        <section className="bld-filters" aria-label="Builder filters">
          <div className="bld-filter color-filter">
            <span className="l">COLOR</span>
            <div className="bld-colors">
              <button type="button" className={`chip all ${builderFilters.colors.length === 0 ? 'on' : ''}`} onClick={() => setBuilderFilters((current) => ({ ...current, colors: [] }))}>ALL</button>
              {COLORS.map((color) => (
                <button
                  type="button"
                  key={color}
                  className={`chip ${builderFilters.colors.includes(color) ? 'on' : ''} ${color.toLowerCase()}`}
                  onClick={() => setBuilderFilters((current) => ({
                    ...current,
                    colors: current.colors.includes(color)
                      ? current.colors.filter((item) => item !== color)
                      : [...current.colors, color],
                  }))}
                >
                  {color[0]}
                </button>
              ))}
            </div>
          </div>
          <label className="bld-filter"><span className="l">TYPE</span><select value={builderFilters.type} onChange={(event) => setBuilderFilters((current) => ({ ...current, type: event.target.value }))}>{TYPES.map((type) => <option key={type} value={type}>{type.toUpperCase()}</option>)}</select></label>
          <label className="bld-filter"><span className="l">LEVEL</span><select value={builderFilters.level} onChange={(event) => setBuilderFilters((current) => ({ ...current, level: event.target.value }))}>{LEVELS.map((level) => <option key={level} value={level}>{level === 'all' ? 'ALL' : `LV${level}`}</option>)}</select></label>
          <label className="bld-filter"><span className="l">RARITY</span><select value={builderFilters.rarity} onChange={(event) => setBuilderFilters((current) => ({ ...current, rarity: event.target.value }))}>{RARITIES.map((rarity) => <option key={rarity} value={rarity}>{rarity.toUpperCase()}</option>)}</select></label>
          <label className="bld-filter search"><span className="l">SEARCH</span><input placeholder="NAME, ID, KEYWORD..." value={builderFilters.search} onChange={(event) => setBuilderFilters((current) => ({ ...current, search: event.target.value }))} /></label>
          <label className="check"><input type="checkbox" checked={builderFilters.inheritedOnly} onChange={(event) => setBuilderFilters((current) => ({ ...current, inheritedOnly: event.target.checked }))} />INHERITED ONLY</label>
          <label className="check"><input type="checkbox" checked={builderFilters.securityOnly} onChange={(event) => setBuilderFilters((current) => ({ ...current, securityOnly: event.target.checked }))} />SECURITY ONLY</label>
        </section>

        <main className="bld-main">
          <aside className="bld-preview">
            {previewCard ? (
              <>
                <div className="bld-preview-card">
                  <div className={`frame ${builderCardColorClass(previewCard)}`}>
                    {previewCard.play_cost && <span className="cost">{previewCard.play_cost}</span>}
                    {previewCard.level && <span className="lvl">L{previewCard.level}</span>}
                    <span className="nm">{previewCard.name}</span>
                    <span className="id">{previewCard.cardnumber}</span>
                  </div>
                </div>
                <div className="bld-preview-meta">
                  <div className="row"><span className="k">SET</span><span className="v">{previewCard.set_name || '-'}</span></div>
                  <div className="row"><span className="k">RARITY</span><span className="v">{previewCard.cardrarity || '-'}</span></div>
                  <div className="row"><span className="k">TYPE</span><span className="v">{previewCard.type}</span></div>
                  <div className="row"><span className="k">IN DECK</span><span className="v">x{[...mainDeck, ...eggDeck].filter((entry) => entry.cardId === previewCard.cardnumber).reduce((sum, entry) => sum + entry.count, 0)}</span></div>
                </div>
                <div className="bld-preview-effect"><h6>MAIN EFFECT</h6><p>{previewCard.maineffect || 'No main effect text loaded.'}</p></div>
                {previewCard.soureeffect && <div className="bld-preview-effect"><h6 className="opp">INHERITED EFFECT</h6><p>{previewCard.soureeffect}</p></div>}
              </>
            ) : (
              <div className="bld-empty">SEARCH OR SELECT A CARD</div>
            )}
          </aside>

          <section className="bld-pool">
            <div className="bld-pool-head"><span>CARD POOL · <span className="v">{visibleCards.length}</span> RESULTS</span><div className="legend"><span><i className="in"></i>IN DECK</span><span><i className="hover"></i>HOVER</span></div></div>
            <div className="bld-pool-grid">
              {visibleCards.map((card) => {
                const count = [...mainDeck, ...eggDeck].filter((entry) => entry.cardId === card.cardnumber).reduce((sum, entry) => sum + entry.count, 0);
                return (
                  <button
                    type="button"
                    key={`${card.cardnumber}-${card.isAltArt ? 'alt' : 'base'}`}
                    aria-label={cardButtonName(card)}
                    className={`bld-card ${builderCardColorClass(card)} ${count > 0 ? 'in-deck' : ''} ${previewCard?.cardnumber === card.cardnumber ? 'preview' : ''}`}
                    onMouseEnter={() => setPreviewCard(card)}
                    onFocus={() => setPreviewCard(card)}
                    onClick={() => addCardToDeck(card.cardnumber, card, card.isAltArt ?? false)}
                    onContextMenu={(event) => {
                      event.preventDefault();
                      removeCardFromDeck(card.cardnumber, card.isAltArt ?? false);
                    }}
                  >
                    {card.play_cost && <span className="cost">{card.play_cost}</span>}
                    {card.level && <span className="lvl">L{card.level}</span>}
                    <span className="nm">{card.name}</span>
                    <span className="id">{card.cardnumber}</span>
                    {count > 0 && <span className="ct">x{count}</span>}
                  </button>
                );
              })}
              {visibleCards.length === 0 && <div className="bld-empty pool-empty">NO CARDS MATCH FILTERS</div>}
            </div>
            <div className="bld-pool-foot"><span>CLICK = ADD · RIGHT-CLICK = REMOVE</span></div>
          </section>

          <aside className="bld-deck">
            <div className="bld-deck-head"><span>DECK CONTENTS</span><span className="v">{counts.total}/55</span></div>
            <div className="bld-deck-tabs">
              <button type="button" className={activeSection === 'main' ? 'on' : ''} onClick={() => setActiveSection('main')}>MAIN <span className="ct">{counts.main}</span></button>
              <button type="button" className={activeSection === 'egg' ? 'on' : ''} onClick={() => setActiveSection('egg')}>EGG <span className="ct">{counts.egg}</span></button>
              <button type="button" className="disabled" onClick={() => setActiveSection('side')}>SIDE <span className="ct">0</span></button>
            </div>
            <div className="bld-deck-list">
              {activeSection === 'side' ? (
                <div className="bld-empty">SIDEBOARD NOT SUPPORTED IN STANDARD</div>
              ) : (
                <section className="bld-section">
                  <div className="bld-section-head"><span>{activeSection === 'egg' ? 'EGG DECK' : 'MAIN DECK'}</span><span className="ct">{activeCards.reduce((sum, entry) => sum + entry.count, 0)} / {activeExpected}</span></div>
                  {visibleSections.map((section) => (
                    <div key={section.label} className="bld-subsection">
                      <div className="bld-subsection-head">{section.label} <span>{section.total}</span></div>
                      {section.entries.map((entry) => (
                        <div key={`${entry.cardId}-${entry.isAltArt ? 'alt' : 'base'}`} className="bld-row" onMouseEnter={() => entry.cardData && setPreviewCard(entry.cardData)}>
                          <span className="ct">x{entry.count}</span>
                          <span className={`swatch ${builderCardColorClass(entry.cardData)}`} />
                          <div className="nm">{entry.cardData?.name ?? entry.cardId}<small>{entry.cardId} · {entry.cardData?.type?.toUpperCase() ?? 'CARD'}</small></div>
                          <span className="cost">{entry.cardData?.play_cost ? `C${entry.cardData.play_cost}` : '-'}</span>
                          <span className="lvl">{entry.cardData?.level ? `L${entry.cardData.level}` : entry.cardData?.type === 'Option' ? 'OPT' : 'TMR'}</span>
                          <div className="actions">
                            <button type="button" onClick={() => removeCardFromDeck(entry.cardId, entry.isAltArt)}>-</button>
                            <button type="button" onClick={() => entry.cardData && addCardToDeck(entry.cardId, entry.cardData, entry.isAltArt)}>+</button>
                          </div>
                        </div>
                      ))}
                    </div>
                  ))}
                  {activeCards.length === 0 && <div className="bld-empty">EMPTY</div>}
                </section>
              )}
              <ValidationPanelInline validationResult={validationResult} />
            </div>
          </aside>
        </main>
      </div>
      <ImportExport isOpen={showImport} onClose={() => setShowImport(false)} />
    </div>
  </div>
);
```

Also add this helper component above `DeckBuilderPage`:

```tsx
function ValidationPanelInline({
  validationResult,
}: {
  validationResult: ReturnType<typeof useDeckBuilderStore.getState>['validationResult'];
}) {
  if (!validationResult) return null;
  if (!validationResult.errors.length && !validationResult.warnings.length) {
    return <div className="bld-validation good">Deck is valid</div>;
  }
  return (
    <div className="bld-validation bad">
      {validationResult.errors.map((error, index) => <p key={`e-${index}`}>ERROR: {error.message}</p>)}
      {validationResult.warnings.map((warning, index) => <p key={`w-${index}`}>WARNING: {warning.message}</p>)}
    </div>
  );
}
```

- [ ] **Step 9: Add mock-derived builder CSS**

Create `code/frontend/src/pages/DeckBuilderPage.css` by copying the builder-specific rules from mock `deck-builder.css`, with these exact changes:

1. Use `.deck-builder-page` instead of `.deck-app` for root scoping.
2. Keep the shared `--bg-*`, `--line*`, `--ink-*`, `--player`, `--opp`, `--good`, `--warn`, and `--bad` tokens inside `.deck-builder-page`.
3. Include only the shared button/pill rules and the `BUILDER VIEW` section from the mock.
4. Add this compatibility block at the end:

```css
.deck-builder-page {
  --bg-0: #07080b;
  --bg-1: #0c0e13;
  --bg-2: #11141b;
  --bg-3: #181c25;
  --line: #1f2330;
  --line-2: #2a3040;
  --line-3: #353c50;
  --ink-0: #f1f3f7;
  --ink-1: #c4c9d4;
  --ink-2: #858c9d;
  --ink-3: #555c6d;
  --player: #ff7a18;
  --player-2: #ffb05a;
  --opp: #3aa6ff;
  --opp-2: #7fc8ff;
  --good: #4cd497;
  --warn: #ffd34a;
  --bad: #ff5b5b;
  --display: "Bebas Neue", Impact, sans-serif;
  --body: Inter, system-ui, sans-serif;
  --mono: "JetBrains Mono", ui-monospace, monospace;
  min-height: calc(100vh - 56px);
  background: var(--bg-0);
  color: var(--ink-1);
  font-family: var(--body);
  overflow: hidden;
}

.deck-builder-app {
  height: calc(100vh - 56px);
}

.deck-name-input {
  min-width: 180px;
  max-width: 280px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--ink-0);
  font-family: var(--display);
  font-size: 18px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  outline: none;
}

.deck-name-input:focus {
  border-color: var(--player);
  background: var(--bg-0);
}

.bld-card {
  text-align: left;
}

.bld-card.red,
.bld-card.r,
.bld-colors .chip.red,
.bld-row .swatch.r { background: linear-gradient(160deg, #3a1414, #15060a); border-color: #5a2020; }
.bld-card.blue,
.bld-card.b,
.bld-colors .chip.blue,
.bld-row .swatch.b { background: linear-gradient(160deg, #14243a, #060d1d); border-color: #20355a; }
.bld-card.yellow,
.bld-card.y,
.bld-colors .chip.yellow,
.bld-row .swatch.y { background: linear-gradient(160deg, #3a2c14, #1d1308); border-color: #5a4220; }
.bld-card.green,
.bld-card.g,
.bld-colors .chip.green,
.bld-row .swatch.g { background: linear-gradient(160deg, #14382a, #061f10); border-color: #205a35; }
.bld-card.purple,
.bld-card.p,
.bld-colors .chip.purple,
.bld-row .swatch.p { background: linear-gradient(160deg, #2c1438, #16061f); border-color: #421f5a; }
.bld-card.black,
.bld-card.k,
.bld-colors .chip.black,
.bld-row .swatch.k { background: linear-gradient(160deg, #1a1a1f, #08080b); border-color: #2a2a35; }
.bld-card.white,
.bld-card.w,
.bld-colors .chip.white,
.bld-row .swatch.w { background: linear-gradient(160deg, #2c3038, #14171c); border-color: #454c5a; }

.bld-subsection-head,
.bld-validation,
.bld-empty,
.bld-notice {
  font-family: var(--mono);
  font-size: 10px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.bld-subsection-head {
  display: flex;
  justify-content: space-between;
  padding: 8px 14px;
  color: var(--ink-3);
  background: var(--bg-0);
  border-bottom: 1px solid var(--line);
}

.bld-validation {
  margin: 10px;
  padding: 10px;
  border: 1px solid var(--line-2);
}

.bld-validation.good {
  color: var(--good);
  border-color: rgba(76, 212, 151, 0.45);
}

.bld-validation.bad {
  color: var(--bad);
  border-color: rgba(255, 91, 91, 0.45);
}

.bld-empty {
  padding: 18px;
  color: var(--ink-3);
  text-align: center;
}

.pool-empty {
  grid-column: 1 / -1;
}

.bld-deck-tabs .disabled,
.pill.disabled {
  opacity: 0.55;
}

@media (max-width: 1120px) {
  .bld-top,
  .bld-filters,
  .bld-main {
    display: block;
    height: auto;
  }

  .deck-builder-page,
  .deck-builder-app {
    overflow: visible;
    height: auto;
  }

  .bld-preview,
  .bld-deck {
    border: 0;
  }
}
```

- [ ] **Step 10: Run E2E and build**

Run:

```powershell
cd code/frontend
npm run build:desktop
npm run e2e -- deck-builder.spec.ts
```

Expected: build passes and the new builder spec passes. Commit:

```powershell
git add code/frontend/src/pages/DeckBuilderPage.tsx code/frontend/src/pages/DeckBuilderPage.css code/frontend/e2e/deck-builder.spec.ts
git commit -m "feat: add in between deck builder ui"
```

---

## Task 5: Library-To-Builder Integration Polish

**Files:**
- Modify: `code/frontend/src/pages/DeckLibraryPage.tsx`
- Modify: `code/frontend/e2e/deck-library.spec.ts`

- [ ] **Step 1: Update library e2e to assert new builder UI**

In `code/frontend/e2e/deck-library.spec.ts`, after:

```ts
await page.getByRole('button', { name: 'Edit' }).click();
await expect(page).toHaveURL(/\/deckbuilder\/deck-1$/);
```

add:

```ts
await expect(page.getByText('CARD POOL')).toBeVisible();
await expect(page.getByText('DECK CONTENTS')).toBeVisible();
await expect(page.getByDisplayValue('Blue Flare')).toBeVisible();
```

- [ ] **Step 2: Make library import/new links explicit**

In `code/frontend/src/pages/DeckLibraryPage.tsx`, ensure the hero links remain:

```tsx
<Link to="/deckbuilder/new" className="library-command primary">New Deck</Link>
<Link to="/deckbuilder/new?import=1" className="library-command">Import</Link>
```

Ensure the banner edit button remains:

```tsx
<button type="button" className="primary" onClick={() => navigate(`/deckbuilder/${selectedSummary.id}`)}>Edit</button>
```

If either differs, change it to the snippets above.

- [ ] **Step 3: Run library and builder specs**

Run:

```powershell
cd code/frontend
npm run e2e -- deck-library.spec.ts deck-builder.spec.ts
```

Expected: both specs pass. Commit:

```powershell
git add code/frontend/src/pages/DeckLibraryPage.tsx code/frontend/e2e/deck-library.spec.ts
git commit -m "test: assert library opens in between builder"
```

---

## Task 6: Full Verification And Desktop Build

**Files:**
- Modify only if verification reveals issues in files touched by this plan.

- [ ] **Step 1: Run unit tests**

Run:

```powershell
cd code/frontend
npm test -- src/features/deck-builder/deckBuilderView.test.ts src/features/play/formatCatalog.test.ts src/features/play/playFlowStore.test.ts src/utils/deckLibrary.test.ts
```

Expected: all tests pass.

- [ ] **Step 2: Run frontend builds**

Run:

```powershell
cd code/frontend
npm run build
npm run build:desktop
```

Expected: both builds pass.

- [ ] **Step 3: Run desktop-mode Playwright specs**

Run:

```powershell
cd code/frontend
$p = Start-Process -FilePath npm.cmd -ArgumentList @('run','dev:desktop','--','--host','127.0.0.1','--port','5174') -WorkingDirectory (Get-Location) -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 5
npm run e2e -- launcher.spec.ts deck-library.spec.ts deck-builder.spec.ts play-flow.spec.ts
Stop-Process -Id $p.Id -Force
```

Expected: all specs pass.

- [ ] **Step 4: Build Tauri desktop executable**

Run:

```powershell
cargo tauri build --no-bundle --config '{"build":{"beforeBuildCommand":""}}'
```

Expected: executable builds at:

```text
C:\Users\james\.codex\worktrees\16a1\digimon-deck-list-builder-1\target\release\digimon-tcg.exe
```

- [ ] **Step 5: Manual smoke checklist**

Run:

```powershell
Start-Process -FilePath "C:\Users\james\.codex\worktrees\16a1\digimon-deck-list-builder-1\target\release\digimon-tcg.exe"
```

Verify:

- Launcher `MY DECKS` opens `/deckbuilder` library.
- Library `New Deck` opens the new mock builder shell.
- Library `Import` opens the new mock builder shell with import modal.
- Library `Edit` opens the new mock builder shell with saved deck name and counts.
- Builder search shows card tiles in the mock card pool.
- Clicking a card increments deck count.
- Right-clicking a card or using row `-` decrements count.
- `VALIDATE` shows validation result.
- `SAVE` writes through the deck library and preserves the deck in `/deckbuilder`.
- `Back to Play` still appears and returns to `/play/deck` when opened via `/deckbuilder/new?returnTo=play`.

- [ ] **Step 6: Final commit**

Run:

```powershell
git status --short
git add code/frontend/src/features/deck-builder code/frontend/src/pages/DeckBuilderPage.tsx code/frontend/src/pages/DeckBuilderPage.css code/frontend/src/pages/DeckLibraryPage.tsx code/frontend/e2e/deck-builder.spec.ts code/frontend/e2e/deck-library.spec.ts
git commit -m "feat: wire in between deck builder to library"
```

Expected: working tree clean after commit.

---

## Self-Review

**Spec coverage:** This plan replaces the old builder UI, uses the mock `deck-builder.jsx`/`deck-builder.css` layout, keeps `/deckbuilder` as the real library, wires library New/Import/Edit into the builder, and preserves save/validate/import/play-return behavior.

**Placeholder scan:** The plan avoids `TBD`, open-ended “handle errors,” or “write tests” without code. The only large CSS instruction is a concrete copy/scope operation from a named mock file plus exact compatibility CSS.

**Type consistency:** New helper names are consistent across tasks: `deckBuilderView.ts`, `deckBuilderAdapter.ts`, `BuilderCardFilters`, `getBuilderCounts`, `filterBuilderCards`, `deckEntriesToSlotArrays`, and `slotArraysToDeckEntries`. Route names stay `/deckbuilder`, `/deckbuilder/new`, `/deckbuilder/:id`, and `/deckbuilder/new?import=1`.

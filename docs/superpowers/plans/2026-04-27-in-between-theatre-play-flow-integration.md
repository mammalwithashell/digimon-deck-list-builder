# In Between Theatre Play Flow Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the In Between Theatre mock into the real frontend, backend, desktop storage, and Rust engine flows for format selection, matchmaking, room lobbies, deck selection, deck organization, deck building, and game launch.

**Architecture:** Keep the mock as the visual contract, but do not ship mock state. Add a shared play-flow model that carries `formatId`, `opponentMode`, `deckId`, `queueType`, and `roomCode` between pages. Route all data through existing adapters first: `deckLibraryAdapter`, `lobbyApi`, `useMatchmaking`, and desktop `gameApi`; add a small format catalog API/Tauri command because format metadata must be shared by web and desktop.

**Tech Stack:** React 19, React Router 7, Zustand, Playwright, Vitest, FastAPI, SQLAlchemy, Tauri 2, Rust `digimon-engine`.

---

## Scope Check

This request spans several subsystems: visual shell, deck library/builder, format selection, matchmaking, room lobbies, and game start. Keep it as one plan because the deliverable is one connected play flow, but implement as independent vertical slices. Each task below ends in a working route, test, and commit.

Mock source inspected from `C:\Users\james\Downloads\In Between Theatre(1).zip`, expanded locally during planning to `%TEMP%\in-between-theatre-1-mock`.

Mock pages to map:
- `Launcher.html`: already largely covered by `code/frontend/src/components/launcher/*`; this plan only updates links into the new play flow.
- `In Between - Mode Select.html`: new `/play` format/opponent page.
- `In Between - Deck Select.html`: new `/play/deck` page backed by deck library data.
- `In Between - Matching.html`: new `/play/matching` page backed by `useMatchmaking`.
- `In Between - Room.html`: new `/play/room/:gameId` page backed by `lobbyApi` and `useWebSocketGame`.
- `In Between - Deck Select.html`, `deck-library.jsx`, `deck-builder.jsx`: polish existing `DeckLibraryPage` and `DeckBuilderPage` without forking data models.
- `In Between - Board.html`, `board.jsx`: keep using current `GameBoard`; only wire route entry and launch metadata.

Unsupported rule variants: the UI may show Titan, EDH, No Banlist, Draft, and Tutorial from the mock, but only `standard` is launchable until the engine exposes alternate `Rules`. Disabled cards must show the engine-backed reason `"Engine supports Standard only in this build"` and cannot continue.

---

## File Structure

### New Files

- `code/frontend/src/features/play/formatCatalog.ts`
  - Shared frontend types and helpers for format cards, queue defaults, and deck legality checks.
- `code/frontend/src/features/play/playFlowStore.ts`
  - Zustand store persisted in `sessionStorage`, scoped to the multi-page play flow.
- `code/frontend/src/features/play/playApi.ts`
  - Web/desktop adapter for format catalog, lobby launch, matchmaking launch, and bot-game launch.
- `code/frontend/src/features/play/InBetweenShell.tsx`
  - Mock-derived titlebar/topbar/frame wrapper used by play pages only.
- `code/frontend/src/features/play/InBetweenShell.css`
  - Mock-derived shell tokens, frame, titlebar, crumb, pill, and page grid CSS.
- `code/frontend/src/pages/ModeSelectPage.tsx`
  - `/play`: choose opponent mode and format.
- `code/frontend/src/pages/ModeSelectPage.css`
  - Mock-derived format cards and opponent mode styling.
- `code/frontend/src/pages/DeckSelectPage.tsx`
  - `/play/deck`: choose a legal deck for the selected format/opponent.
- `code/frontend/src/pages/DeckSelectPage.css`
  - Mock-derived format banner and confirm bar styling.
- `code/frontend/src/pages/MatchingPage.tsx`
  - `/play/matching`: real matchmaking wait/matched screen.
- `code/frontend/src/pages/MatchingPage.css`
  - Mock-derived radar/stage/player-card styling.
- `code/frontend/src/pages/RoomLobbyPage.tsx`
  - `/play/room/:gameId`: room code, ready state, deck picker, and launch handoff.
- `code/frontend/src/pages/RoomLobbyPage.css`
  - Mock-derived room lobby layout.
- `code/frontend/e2e/play-flow.spec.ts`
  - End-to-end coverage for `/play -> /play/deck -> /play/matching`.
- `code/frontend/src/features/play/formatCatalog.test.ts`
  - Unit tests for format catalog and deck legality.
- `code/server/routers/formats.py`
  - Web API route for format metadata.
- `code/tests/api/test_formats.py`
  - API tests for format metadata.
- `code/src-tauri/src/format_commands.rs`
  - Desktop command returning the same format metadata shape as the web route.

### Modified Files

- `code/frontend/src/App.tsx`
  - Add routes `/play`, `/play/deck`, `/play/matching`, `/play/room/:gameId`.
- `code/frontend/src/components/launcher/LauncherActions.tsx`
  - Change primary PLAY target from `/game` to `/play`.
- `code/frontend/src/components/launcher/LauncherShell.tsx`
  - Change sidebar Play target from `/game` to `/play`; keep Sandbox on `/game`.
- `code/frontend/src/pages/DeckLibraryPage.tsx`
  - Allow selection mode via optional props so `/play/deck` can reuse library content without duplicating deck fetch logic.
- `code/frontend/src/pages/DeckLibraryPage.css`
  - Add legal/illegal deck state and compact embedded mode styles.
- `code/frontend/src/pages/DeckBuilderPage.tsx`
  - Keep `/deckbuilder/new?import=1` behavior and add a return-to-play affordance when `returnTo=play`.
- `code/frontend/src/api/lobbyApi.ts`
  - Export lobby response types; add `getLobby(gameId)` if room view needs server state not currently exposed.
- `code/frontend/src/api/matchmaking.ts`
  - Export queue labels and add typed `queue_type` mapping helper if needed by `playApi`.
- `code/frontend/src/storage/deckStore.ts`
  - Preserve deck library fields when duplicating/saving from play flow.
- `code/server/api.py`
  - Include `formats.router`.
- `code/src-tauri/src/main.rs`
  - Register `formats_list` command.

---

## Task 1: Shared Format Catalog

**Files:**
- Create: `code/frontend/src/features/play/formatCatalog.ts`
- Create: `code/frontend/src/features/play/formatCatalog.test.ts`
- Create: `code/server/routers/formats.py`
- Create: `code/tests/api/test_formats.py`
- Create: `code/src-tauri/src/format_commands.rs`
- Modify: `code/server/api.py`
- Modify: `code/src-tauri/src/main.rs`

- [x] **Step 1: Write the frontend format catalog tests**

Add `code/frontend/src/features/play/formatCatalog.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { canUseDeckForFormat, formatToQueueType, PLAY_FORMATS } from './formatCatalog';
import type { DeckSummary } from '@/types/deck';

const deck = (overrides: Partial<DeckSummary>): DeckSummary => ({
  id: 'd1',
  name: 'Standard Legal',
  description: '',
  game_mode: 'standard',
  is_valid: true,
  is_public: false,
  is_pinned: false,
  folder_id: null,
  card_count: 54,
  main_count: 50,
  egg_count: 4,
  tags: [],
  meta_tier: 'rogue',
  meta_archetype: 'Test Archetype',
  colors: ['Red'],
  highest_level: 6,
  created_at: '2026-04-27T00:00:00.000Z',
  updated_at: '2026-04-27T00:00:00.000Z',
  ...overrides,
});

describe('formatCatalog', () => {
  it('exposes standard as the only engine-launchable format', () => {
    expect(PLAY_FORMATS.find((f) => f.id === 'standard')?.enabled).toBe(true);
    expect(PLAY_FORMATS.filter((f) => f.enabled).map((f) => f.id)).toEqual(['standard']);
  });

  it('accepts a 50 plus 4 standard deck for standard', () => {
    expect(canUseDeckForFormat(deck({}), 'standard')).toEqual({ ok: true });
  });

  it('rejects incomplete and invalid standard decks', () => {
    expect(canUseDeckForFormat(deck({ main_count: 43, card_count: 47 }), 'standard')).toEqual({
      ok: false,
      reason: 'Standard requires 50 main cards and 0-5 eggs.',
    });
    expect(canUseDeckForFormat(deck({ is_valid: false }), 'standard')).toEqual({
      ok: false,
      reason: 'Deck must pass validation before queueing.',
    });
  });

  it('maps standard quick match to casual queue', () => {
    expect(formatToQueueType('standard')).toBe('casual');
  });
});
```

- [x] **Step 2: Run the frontend test and verify it fails**

Run:

```bash
cd code/frontend
npm test -- src/features/play/formatCatalog.test.ts
```

Expected: fail because `src/features/play/formatCatalog.ts` does not exist.

- [x] **Step 3: Implement the frontend format catalog**

Create `code/frontend/src/features/play/formatCatalog.ts`:

```ts
import type { QueueType } from '@/api/matchmaking';
import type { DeckSummary } from '@/types/deck';

export type PlayFormatId = 'standard' | 'titan' | 'edh' | 'nobanlist' | 'draft' | 'tutorial';
export type OpponentMode = 'quick' | 'room' | 'bot';

export interface PlayFormat {
  id: PlayFormatId;
  name: string;
  tagline: string;
  description: string;
  deckLabel: string;
  populationPct: number;
  enabled: boolean;
  disabledReason?: string;
}

export const ENGINE_STANDARD_ONLY_REASON = 'Engine supports Standard only in this build';

export const PLAY_FORMATS: PlayFormat[] = [
  {
    id: 'standard',
    name: 'STANDARD',
    tagline: 'The official ruleset',
    description: '50-card decks, current banlist, mirrored memory gauge.',
    deckLabel: '50 cards',
    populationPct: 84,
    enabled: true,
  },
  {
    id: 'titan',
    name: 'TITAN',
    tagline: 'Bigger gauges. Bigger threats.',
    description: '75-card deck concept from the mock; disabled until Rules support lands.',
    deckLabel: '75 cards',
    populationPct: 42,
    enabled: false,
    disabledReason: ENGINE_STANDARD_ONLY_REASON,
  },
  {
    id: 'edh',
    name: 'EDH',
    tagline: 'One herald, one of each, four players',
    description: '100-card singleton concept from the mock; disabled until multiplayer Rules support lands.',
    deckLabel: '100 singleton',
    populationPct: 67,
    enabled: false,
    disabledReason: ENGINE_STANDARD_ONLY_REASON,
  },
  {
    id: 'nobanlist',
    name: 'NO BANLIST',
    tagline: 'Every card. Every printing.',
    description: 'Standard shape without restrictions; disabled until validator support lands.',
    deckLabel: '50 cards',
    populationPct: 23,
    enabled: false,
    disabledReason: ENGINE_STANDARD_ONLY_REASON,
  },
  {
    id: 'draft',
    name: 'DRAFT',
    tagline: 'Build from a pod',
    description: 'Limited mode concept from the mock; disabled until draft pool support lands.',
    deckLabel: '40 cards',
    populationPct: 12,
    enabled: false,
    disabledReason: ENGINE_STANDARD_ONLY_REASON,
  },
  {
    id: 'tutorial',
    name: 'TUTORIAL',
    tagline: 'Practice the board',
    description: 'Guided game concept from the mock; disabled until scripted tutorial support lands.',
    deckLabel: 'Starter',
    populationPct: 9,
    enabled: false,
    disabledReason: ENGINE_STANDARD_ONLY_REASON,
  },
];

export function getPlayFormat(formatId: string | null | undefined): PlayFormat {
  return PLAY_FORMATS.find((format) => format.id === formatId) ?? PLAY_FORMATS[0];
}

export function canUseDeckForFormat(
  deck: DeckSummary,
  formatId: PlayFormatId,
): { ok: true } | { ok: false; reason: string } {
  const format = getPlayFormat(formatId);
  if (!format.enabled) {
    return { ok: false, reason: format.disabledReason ?? ENGINE_STANDARD_ONLY_REASON };
  }
  if (!deck.is_valid) return { ok: false, reason: 'Deck must pass validation before queueing.' };
  if (deck.main_count !== 50 || deck.egg_count < 0 || deck.egg_count > 5) {
    return { ok: false, reason: 'Standard requires 50 main cards and 0-5 eggs.' };
  }
  return { ok: true };
}

export function formatToQueueType(formatId: PlayFormatId): QueueType {
  if (formatId === 'standard') return 'casual';
  return 'casual';
}
```

- [x] **Step 4: Add web and desktop format catalog endpoints**

Create `code/server/routers/formats.py`:

```py
from fastapi import APIRouter
from pydantic import BaseModel

router = APIRouter(prefix="/formats", tags=["formats"])


class FormatDto(BaseModel):
    id: str
    name: str
    tagline: str
    description: str
    deck_label: str
    population_pct: int
    enabled: bool
    disabled_reason: str | None = None


ENGINE_STANDARD_ONLY_REASON = "Engine supports Standard only in this build"


@router.get("", response_model=list[FormatDto])
def list_formats() -> list[FormatDto]:
    return [
        FormatDto(
            id="standard",
            name="STANDARD",
            tagline="The official ruleset",
            description="50-card decks, current banlist, mirrored memory gauge.",
            deck_label="50 cards",
            population_pct=84,
            enabled=True,
        ),
        FormatDto(
            id="titan",
            name="TITAN",
            tagline="Bigger gauges. Bigger threats.",
            description="75-card deck concept from the mock; disabled until Rules support lands.",
            deck_label="75 cards",
            population_pct=42,
            enabled=False,
            disabled_reason=ENGINE_STANDARD_ONLY_REASON,
        ),
        FormatDto(
            id="edh",
            name="EDH",
            tagline="One herald, one of each, four players",
            description="100-card singleton concept from the mock; disabled until multiplayer Rules support lands.",
            deck_label="100 singleton",
            population_pct=67,
            enabled=False,
            disabled_reason=ENGINE_STANDARD_ONLY_REASON,
        ),
        FormatDto(
            id="nobanlist",
            name="NO BANLIST",
            tagline="Every card. Every printing.",
            description="Standard shape without restrictions; disabled until validator support lands.",
            deck_label="50 cards",
            population_pct=23,
            enabled=False,
            disabled_reason=ENGINE_STANDARD_ONLY_REASON,
        ),
        FormatDto(
            id="draft",
            name="DRAFT",
            tagline="Build from a pod",
            description="Limited mode concept from the mock; disabled until draft pool support lands.",
            deck_label="40 cards",
            population_pct=12,
            enabled=False,
            disabled_reason=ENGINE_STANDARD_ONLY_REASON,
        ),
        FormatDto(
            id="tutorial",
            name="TUTORIAL",
            tagline="Practice the board",
            description="Guided game concept from the mock; disabled until scripted tutorial support lands.",
            deck_label="Starter",
            population_pct=9,
            enabled=False,
            disabled_reason=ENGINE_STANDARD_ONLY_REASON,
        ),
    ]
```

Create `code/tests/api/test_formats.py`:

```py
from fastapi.testclient import TestClient

from server.api import app


def test_formats_catalog_marks_standard_enabled() -> None:
    client = TestClient(app)
    response = client.get("/formats")
    assert response.status_code == 200
    body = response.json()
    assert body[0]["id"] == "standard"
    assert body[0]["enabled"] is True
    assert [item["id"] for item in body if item["enabled"]] == ["standard"]
```

Modify `code/server/api.py`:

```py
from server.routers import formats

app.include_router(formats.router)
```

Create `code/src-tauri/src/format_commands.rs`:

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FormatDto {
    pub id: &'static str,
    pub name: &'static str,
    pub tagline: &'static str,
    pub description: &'static str,
    pub deck_label: &'static str,
    pub population_pct: u8,
    pub enabled: bool,
    pub disabled_reason: Option<&'static str>,
}

const ENGINE_STANDARD_ONLY_REASON: &str = "Engine supports Standard only in this build";

#[tauri::command]
pub fn formats_list() -> Vec<FormatDto> {
    vec![
        FormatDto {
            id: "standard",
            name: "STANDARD",
            tagline: "The official ruleset",
            description: "50-card decks, current banlist, mirrored memory gauge.",
            deck_label: "50 cards",
            population_pct: 84,
            enabled: true,
            disabled_reason: None,
        },
        FormatDto {
            id: "titan",
            name: "TITAN",
            tagline: "Bigger gauges. Bigger threats.",
            description: "75-card deck concept from the mock; disabled until Rules support lands.",
            deck_label: "75 cards",
            population_pct: 42,
            enabled: false,
            disabled_reason: Some(ENGINE_STANDARD_ONLY_REASON),
        },
        FormatDto {
            id: "edh",
            name: "EDH",
            tagline: "One herald, one of each, four players",
            description: "100-card singleton concept from the mock; disabled until multiplayer Rules support lands.",
            deck_label: "100 singleton",
            population_pct: 67,
            enabled: false,
            disabled_reason: Some(ENGINE_STANDARD_ONLY_REASON),
        },
        FormatDto {
            id: "nobanlist",
            name: "NO BANLIST",
            tagline: "Every card. Every printing.",
            description: "Standard shape without restrictions; disabled until validator support lands.",
            deck_label: "50 cards",
            population_pct: 23,
            enabled: false,
            disabled_reason: Some(ENGINE_STANDARD_ONLY_REASON),
        },
        FormatDto {
            id: "draft",
            name: "DRAFT",
            tagline: "Build from a pod",
            description: "Limited mode concept from the mock; disabled until draft pool support lands.",
            deck_label: "40 cards",
            population_pct: 12,
            enabled: false,
            disabled_reason: Some(ENGINE_STANDARD_ONLY_REASON),
        },
        FormatDto {
            id: "tutorial",
            name: "TUTORIAL",
            tagline: "Practice the board",
            description: "Guided game concept from the mock; disabled until scripted tutorial support lands.",
            deck_label: "Starter",
            population_pct: 9,
            enabled: false,
            disabled_reason: Some(ENGINE_STANDARD_ONLY_REASON),
        },
    ]
}
```

Modify `code/src-tauri/src/main.rs`:

```rust
mod format_commands;

.invoke_handler(tauri::generate_handler![
    format_commands::formats_list,
])
```

- [x] **Step 5: Run tests and commit**

Run:

```bash
cd code/frontend
npm test -- src/features/play/formatCatalog.test.ts
cd ..\..
pytest code/tests/api/test_formats.py -q
cargo test -p digimon-tcg formats_list
```

Expected: frontend test passes, API test passes, Rust command compiles. Commit:

```bash
git add code/frontend/src/features/play/formatCatalog.ts code/frontend/src/features/play/formatCatalog.test.ts code/server/routers/formats.py code/tests/api/test_formats.py code/server/api.py code/src-tauri/src/format_commands.rs code/src-tauri/src/main.rs
git commit -m "feat: add play format catalog"
```

---

## Task 2: Play Flow Store And API Adapter

**Files:**
- Create: `code/frontend/src/features/play/playFlowStore.ts`
- Create: `code/frontend/src/features/play/playApi.ts`
- Test: `code/frontend/src/features/play/playFlowStore.test.ts`

- [x] **Step 1: Write play flow store tests**

Create `code/frontend/src/features/play/playFlowStore.test.ts`:

```ts
import { beforeEach, describe, expect, it } from 'vitest';
import { usePlayFlowStore } from './playFlowStore';

describe('playFlowStore', () => {
  beforeEach(() => {
    sessionStorage.clear();
    usePlayFlowStore.getState().reset();
  });

  it('stores the selected format, opponent mode, and deck id', () => {
    usePlayFlowStore.getState().selectFormat('standard');
    usePlayFlowStore.getState().selectOpponentMode('quick');
    usePlayFlowStore.getState().selectDeck('deck-1');

    expect(usePlayFlowStore.getState()).toMatchObject({
      formatId: 'standard',
      opponentMode: 'quick',
      deckId: 'deck-1',
    });
  });

  it('resets transient queue and room fields without clearing the selected format', () => {
    usePlayFlowStore.getState().selectFormat('standard');
    usePlayFlowStore.getState().setQueue({ ticketId: 'ticket-1', roomCode: 'ABC123' });
    usePlayFlowStore.getState().clearLaunchState();

    expect(usePlayFlowStore.getState().formatId).toBe('standard');
    expect(usePlayFlowStore.getState().ticketId).toBeNull();
    expect(usePlayFlowStore.getState().roomCode).toBeNull();
  });
});
```

- [x] **Step 2: Run the store test and verify it fails**

Run:

```bash
cd code/frontend
npm test -- src/features/play/playFlowStore.test.ts
```

Expected: fail because `playFlowStore.ts` does not exist.

- [x] **Step 3: Implement play flow store**

Create `code/frontend/src/features/play/playFlowStore.ts`:

```ts
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { QueueType } from '@/api/matchmaking';
import type { OpponentMode, PlayFormatId } from './formatCatalog';

interface QueueState {
  ticketId?: string | null;
  roomCode?: string | null;
  gameId?: string | null;
}

interface PlayFlowState {
  formatId: PlayFormatId;
  opponentMode: OpponentMode;
  queueType: QueueType;
  deckId: string | null;
  ticketId: string | null;
  roomCode: string | null;
  gameId: string | null;
  selectFormat: (formatId: PlayFormatId) => void;
  selectOpponentMode: (mode: OpponentMode) => void;
  selectQueueType: (queueType: QueueType) => void;
  selectDeck: (deckId: string | null) => void;
  setQueue: (state: QueueState) => void;
  clearLaunchState: () => void;
  reset: () => void;
}

const initial = {
  formatId: 'standard' as PlayFormatId,
  opponentMode: 'quick' as OpponentMode,
  queueType: 'casual' as QueueType,
  deckId: null,
  ticketId: null,
  roomCode: null,
  gameId: null,
};

export const usePlayFlowStore = create<PlayFlowState>()(
  persist(
    (set) => ({
      ...initial,
      selectFormat: (formatId) => set({ formatId }),
      selectOpponentMode: (opponentMode) => set({ opponentMode }),
      selectQueueType: (queueType) => set({ queueType }),
      selectDeck: (deckId) => set({ deckId }),
      setQueue: ({ ticketId, roomCode, gameId }) =>
        set((state) => ({
          ticketId: ticketId === undefined ? state.ticketId : ticketId,
          roomCode: roomCode === undefined ? state.roomCode : roomCode,
          gameId: gameId === undefined ? state.gameId : gameId,
        })),
      clearLaunchState: () => set({ ticketId: null, roomCode: null, gameId: null }),
      reset: () => set(initial),
    }),
    {
      name: 'in-between-play-flow',
      storage: {
        getItem: (name) => {
          const value = sessionStorage.getItem(name);
          return value ? JSON.parse(value) : null;
        },
        setItem: (name, value) => sessionStorage.setItem(name, JSON.stringify(value)),
        removeItem: (name) => sessionStorage.removeItem(name),
      },
    },
  ),
);
```

- [x] **Step 4: Implement play API adapter**

Create `code/frontend/src/features/play/playApi.ts`:

```ts
import { invoke } from '@tauri-apps/api/core';
import * as deckLibrary from '@/api/deckLibraryAdapter';
import * as gameApi from '@/api/gameApi';
import * as lobbyApi from '@/api/lobbyApi';
import * as matchmaking from '@/api/matchmaking';
import type { DeckResponse } from '@/types/deck';
import type { PlayFormat, PlayFormatId } from './formatCatalog';
import { PLAY_FORMATS, formatToQueueType } from './formatCatalog';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

interface FormatDto {
  id: PlayFormatId;
  name: string;
  tagline: string;
  description: string;
  deck_label: string;
  population_pct: number;
  enabled: boolean;
  disabled_reason?: string | null;
}

function fromDto(dto: FormatDto): PlayFormat {
  return {
    id: dto.id,
    name: dto.name,
    tagline: dto.tagline,
    description: dto.description,
    deckLabel: dto.deck_label,
    populationPct: dto.population_pct,
    enabled: dto.enabled,
    disabledReason: dto.disabled_reason ?? undefined,
  };
}

export async function listFormats(): Promise<PlayFormat[]> {
  if (IS_DESKTOP) {
    try {
      return (await invoke<FormatDto[]>('formats_list')).map(fromDto);
    } catch {
      return PLAY_FORMATS;
    }
  }
  const response = await fetch('/formats');
  if (!response.ok) return PLAY_FORMATS;
  return ((await response.json()) as FormatDto[]).map(fromDto);
}

export async function getDeck(deckId: string): Promise<DeckResponse> {
  return deckLibrary.getDeck(deckId);
}

export async function queueQuickMatch(params: {
  formatId: PlayFormatId;
  deck: DeckResponse;
}): Promise<matchmaking.QueueResponse> {
  return matchmaking.queue({
    queue_type: formatToQueueType(params.formatId),
    main_deck: params.deck.main_deck,
    egg_deck: params.deck.egg_deck,
    game_mode: params.formatId,
  });
}

export async function createRoom(params: {
  formatId: PlayFormatId;
  deck: DeckResponse;
}): Promise<{ game_id: string; join_code: string }> {
  return lobbyApi.createLobby({
    deck: [...params.deck.egg_deck, ...params.deck.main_deck],
    is_public: false,
    allow_spectators: true,
    spectator_mode: 'hidden',
  });
}

export async function createBotGame(params: {
  deck: DeckResponse;
  opponentDeck: DeckResponse;
}): Promise<{ game_id: string }> {
  const response = await gameApi.createGame({
    deck1: [...params.deck.egg_deck, ...params.deck.main_deck],
    deck2: [...params.opponentDeck.egg_deck, ...params.opponentDeck.main_deck],
    player_kinds: ['human', 'greedy'],
    player_model_ids: [null, null],
  });
  return { game_id: response.game_id };
}
```

- [x] **Step 5: Run tests and commit**

Run:

```bash
cd code/frontend
npm test -- src/features/play/playFlowStore.test.ts
npm run build:desktop
```

Expected: tests and desktop build pass. Commit:

```bash
git add code/frontend/src/features/play/playFlowStore.ts code/frontend/src/features/play/playFlowStore.test.ts code/frontend/src/features/play/playApi.ts
git commit -m "feat: add play flow state and adapters"
```

---

## Task 3: In Between Shell And Routes

**Files:**
- Create: `code/frontend/src/features/play/InBetweenShell.tsx`
- Create: `code/frontend/src/features/play/InBetweenShell.css`
- Modify: `code/frontend/src/App.tsx`
- Modify: `code/frontend/src/components/launcher/LauncherActions.tsx`
- Modify: `code/frontend/src/components/launcher/LauncherShell.tsx`
- Test: `code/frontend/e2e/play-flow.spec.ts`

- [ ] **Step 1: Write a failing route smoke test**

Create `code/frontend/e2e/play-flow.spec.ts`:

```ts
import { expect, test } from '@playwright/test';

test.describe('In Between play flow', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('access_token', 'guest-token');
      localStorage.setItem('guest_access_token', 'guest-token');
      localStorage.setItem('guest_user_id', 'guest_abc');
      localStorage.setItem('guest_display_name', 'Guest-ABCD');
    });
    await page.route('**/api/users/me', (route) =>
      route.fulfill({
        json: { id: 'guest_abc', username: 'Guest-ABCD', email: null, roles: [] },
      }),
    );
    await page.route('**/formats', (route) =>
      route.fulfill({
        json: [
          {
            id: 'standard',
            name: 'STANDARD',
            tagline: 'The official ruleset',
            description: '50-card decks, current banlist, mirrored memory gauge.',
            deck_label: '50 cards',
            population_pct: 84,
            enabled: true,
            disabled_reason: null,
          },
        ],
      }),
    );
  });

  test('opens format selection from launcher play', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: /PRIMARY ACTION\s+PLAY/i }).click();
    await expect(page).toHaveURL(/\/play$/);
    await expect(page.getByRole('heading', { name: /CHOOSE YOUR\s+FORMAT/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /QUICK MATCH/i })).toBeVisible();
  });
});
```

- [ ] **Step 2: Run the route smoke test and verify it fails**

Run:

```bash
cd code/frontend
npm run dev:desktop -- --host 127.0.0.1 --port 5174
npm run e2e -- play-flow.spec.ts
```

Expected: fail because `/play` is not routed and launcher PLAY still targets the old route.

- [ ] **Step 3: Implement shared shell**

Create `code/frontend/src/features/play/InBetweenShell.tsx`:

```tsx
import type { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import './InBetweenShell.css';

interface Crumb {
  label: string;
  href?: string;
}

interface InBetweenShellProps {
  title: string;
  stepLabel: string;
  crumbs: Crumb[];
  children: ReactNode;
  rightSlot?: ReactNode;
}

export function InBetweenShell({
  title,
  stepLabel,
  crumbs,
  children,
  rightSlot,
}: InBetweenShellProps) {
  return (
    <div className="ib-flow-frame">
      <div className="ib-flow-titlebar">
        <div className="ib-flow-dots" aria-hidden="true">
          <span className="r" />
          <span className="y" />
          <span className="g" />
        </div>
        <div className="ib-flow-window-title">THE AMPHITHEATER BETWIXT - {title}</div>
        <span className="ib-flow-pill-live">CONNECTED</span>
      </div>
      <div className="ib-flow-body">
        <div className="ib-flow-topbar">
          <nav className="ib-flow-crumb" aria-label="Play flow">
            <span className="idx">{stepLabel}</span>
            {crumbs.map((crumb, index) => (
              <span key={`${crumb.label}-${index}`} className="crumb-part">
                <span className="sep">/</span>
                {crumb.href ? <Link to={crumb.href}>{crumb.label}</Link> : <span>{crumb.label}</span>}
              </span>
            ))}
          </nav>
          <div className="ib-flow-topbar-right">{rightSlot}</div>
        </div>
        {children}
      </div>
    </div>
  );
}
```

Create `code/frontend/src/features/play/InBetweenShell.css` with the mock-derived frame tokens:

```css
.ib-flow-frame {
  min-height: 100vh;
  background: #050608;
  color: #f5f1e8;
  font-family: Geist, Inter, system-ui, sans-serif;
}

.ib-flow-titlebar {
  height: 38px;
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 14px;
  padding: 0 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.12);
  background: linear-gradient(180deg, rgba(255,255,255,0.08), rgba(255,255,255,0.02));
}

.ib-flow-dots {
  display: flex;
  gap: 8px;
}

.ib-flow-dots span {
  width: 11px;
  height: 11px;
  border-radius: 999px;
}

.ib-flow-dots .r { background: #ff5f57; }
.ib-flow-dots .y { background: #ffbd2e; }
.ib-flow-dots .g { background: #28c840; }

.ib-flow-window-title {
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
  letter-spacing: 0.12em;
  color: rgba(245, 241, 232, 0.72);
}

.ib-flow-pill-live {
  border: 1px solid rgba(76, 212, 151, 0.55);
  color: #4cd497;
  padding: 3px 8px;
  font-size: 11px;
  letter-spacing: 0.12em;
}

.ib-flow-body {
  min-height: calc(100vh - 38px);
}

.ib-flow-topbar {
  height: 46px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 22px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
}

.ib-flow-crumb {
  display: flex;
  align-items: center;
  gap: 8px;
  color: rgba(245, 241, 232, 0.55);
}

.ib-flow-crumb .idx {
  border: 1px solid rgba(255, 122, 24, 0.6);
  color: #ff7a18;
  padding: 2px 6px;
}

.ib-flow-crumb a {
  color: rgba(245, 241, 232, 0.72);
  text-decoration: none;
}

.ib-flow-crumb .crumb-part:last-child span:last-child {
  color: #f5f1e8;
}

.ib-flow-topbar-right {
  display: flex;
  align-items: center;
  gap: 10px;
  color: rgba(245, 241, 232, 0.64);
}
```

- [ ] **Step 4: Wire routes and launcher links**

Modify `code/frontend/src/App.tsx`:

```tsx
import { ModeSelectPage } from '@/pages/ModeSelectPage';
import { DeckSelectPage } from '@/pages/DeckSelectPage';
import { MatchingPage } from '@/pages/MatchingPage';
import { RoomLobbyPage } from '@/pages/RoomLobbyPage';

<Route element={<AuthGuard />}>
  <Route path="/play" element={<ModeSelectPage />} />
  <Route path="/play/deck" element={<DeckSelectPage />} />
  <Route path="/play/matching" element={<MatchingPage />} />
  <Route path="/play/room/:gameId" element={<RoomLobbyPage />} />
</Route>
```

Modify `code/frontend/src/components/launcher/LauncherActions.tsx`:

```tsx
<Link className="launcher-action primary" to="/play">
  <span className="launcher-action-label">// PRIMARY ACTION</span>
  <span className="launcher-action-name">PLAY</span>
  <span className="launcher-action-desc">Choose format, deck, and opponent.</span>
  <span className="launcher-action-meta">ENTER</span>
</Link>
```

Modify `code/frontend/src/components/launcher/LauncherShell.tsx`:

```tsx
<Link className="launcher-nav-item" to="/play">Play</Link>
```

- [ ] **Step 5: Run smoke test and commit**

Run:

```bash
cd code/frontend
npm run build:desktop
npm run e2e -- play-flow.spec.ts
```

Expected: pass after `ModeSelectPage` exists in Task 4. If this task is implemented before Task 4, commit after route compilation passes with a minimal page. Commit:

```bash
git add code/frontend/src/features/play/InBetweenShell.tsx code/frontend/src/features/play/InBetweenShell.css code/frontend/src/App.tsx code/frontend/src/components/launcher/LauncherActions.tsx code/frontend/src/components/launcher/LauncherShell.tsx code/frontend/e2e/play-flow.spec.ts
git commit -m "feat: add in between play shell routes"
```

---

## Task 4: Format And Opponent Selection Page

**Files:**
- Create: `code/frontend/src/pages/ModeSelectPage.tsx`
- Create: `code/frontend/src/pages/ModeSelectPage.css`
- Modify: `code/frontend/e2e/play-flow.spec.ts`

- [ ] **Step 1: Extend e2e for format selection**

Append to `code/frontend/e2e/play-flow.spec.ts`:

```ts
test('chooses quick match standard and advances to deck select', async ({ page }) => {
  await page.goto('/play');
  await page.getByRole('button', { name: /QUICK MATCH/i }).click();
  await page.getByRole('button', { name: /STANDARD/i }).click();
  await page.getByRole('button', { name: /ENTER FORMAT/i }).click();
  await expect(page).toHaveURL(/\/play\/deck/);
});
```

- [ ] **Step 2: Run the e2e and verify it fails**

Run:

```bash
cd code/frontend
npm run e2e -- play-flow.spec.ts
```

Expected: fail because `ModeSelectPage` does not render the mock controls.

- [ ] **Step 3: Implement mode select page**

Create `code/frontend/src/pages/ModeSelectPage.tsx`:

```tsx
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import { getPlayFormat, type OpponentMode, type PlayFormat } from '@/features/play/formatCatalog';
import { listFormats } from '@/features/play/playApi';
import './ModeSelectPage.css';

const OPPONENTS: Array<{ id: OpponentMode; name: string; sub: string; meta: string }> = [
  { id: 'quick', name: 'QUICK MATCH', sub: 'Auto-paired ladder', meta: 'MATCHMAKING' },
  { id: 'room', name: 'ROOM MATCH', sub: 'Private code and friends', meta: 'PRIVATE CODE' },
  { id: 'bot', name: 'BOT MATCH', sub: 'CPU practice', meta: 'LOCAL ENGINE' },
];

export function ModeSelectPage() {
  const navigate = useNavigate();
  const [formats, setFormats] = useState<PlayFormat[]>([]);
  const { formatId, opponentMode, selectFormat, selectOpponentMode, clearLaunchState } =
    usePlayFlowStore();
  const selected = getPlayFormat(formatId);

  useEffect(() => {
    clearLaunchState();
    listFormats().then(setFormats).catch(() => setFormats([]));
  }, [clearLaunchState]);

  const visibleFormats = formats.length > 0 ? formats : [selected];

  return (
    <InBetweenShell
      title="CHOOSE FORMAT"
      stepLabel="01"
      crumbs={[{ label: 'HOME', href: '/' }, { label: 'PLAY' }]}
      rightSlot={<span>STEP 1 OF 3</span>}
    >
      <main className="mode-select-main">
        <header className="mode-select-header">
          <div className="welcome">// SELECT A RULESET TO ENTER THE AMPHITHEATER</div>
          <h1>CHOOSE YOUR<br /><em>FORMAT.</em></h1>
          <p>SIX RULESETS - DIFFERENT BANLISTS - DIFFERENT DECK SHAPES - ONE THEATER</p>
        </header>

        <section className="opponent-strip" aria-label="Opponent">
          {OPPONENTS.map((opponent) => (
            <button
              key={opponent.id}
              type="button"
              className={opponentMode === opponent.id ? 'on' : ''}
              onClick={() => selectOpponentMode(opponent.id)}
            >
              <span className="name">{opponent.name}</span>
              <span className="sub">{opponent.sub}</span>
              <span className="meta">{opponent.meta}</span>
            </button>
          ))}
        </section>

        <section className="mode-grid" aria-label="Formats">
          {visibleFormats.map((format, index) => (
            <button
              key={format.id}
              type="button"
              className={`mode-card ${format.id === formatId ? 'selected' : ''}`}
              onClick={() => selectFormat(format.id)}
              disabled={!format.enabled}
            >
              <span className="num">{String(index + 1).padStart(2, '0')} / 06</span>
              <span className="tag">{format.enabled ? '// READY' : '// LOCKED'}</span>
              <span className="sub">{format.tagline}</span>
              <span className="name">{format.name}</span>
              <span className="desc">{format.enabled ? format.description : format.disabledReason}</span>
              <span className="stats">
                <b>{format.deckLabel}</b>
                <i>POPULATION {format.populationPct}%</i>
              </span>
            </button>
          ))}
        </section>

        <div className="mode-action-bar">
          <span>{selected.name} / {opponentMode.toUpperCase()}</span>
          <button type="button" onClick={() => navigate('/play/deck')} disabled={!selected.enabled}>
            ENTER FORMAT
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
```

- [ ] **Step 4: Add mode page CSS**

Create `code/frontend/src/pages/ModeSelectPage.css`:

```css
.mode-select-main {
  padding: 34px 36px 96px;
}

.mode-select-header .welcome {
  color: #ffb05a;
  font-family: "JetBrains Mono", monospace;
  letter-spacing: 0.18em;
  font-size: 12px;
}

.mode-select-header h1 {
  margin: 8px 0;
  font-size: clamp(44px, 8vw, 96px);
  line-height: 0.88;
  letter-spacing: 0;
}

.mode-select-header em {
  color: #ff7a18;
  font-style: normal;
}

.mode-select-header p {
  color: rgba(245, 241, 232, 0.62);
  font-family: "JetBrains Mono", monospace;
  letter-spacing: 0.14em;
}

.opponent-strip {
  margin: 28px 0;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.opponent-strip button,
.mode-card {
  text-align: left;
  background: rgba(255, 255, 255, 0.045);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #f5f1e8;
}

.opponent-strip button {
  display: grid;
  gap: 5px;
  min-height: 92px;
  padding: 16px;
}

.opponent-strip button.on,
.mode-card.selected {
  border-color: #ff7a18;
  box-shadow: 0 0 0 1px rgba(255, 122, 24, 0.55), 0 0 28px rgba(255, 122, 24, 0.14);
}

.opponent-strip .name,
.mode-card .name {
  font-family: "JetBrains Mono", monospace;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.opponent-strip .sub,
.opponent-strip .meta,
.mode-card .sub,
.mode-card .desc,
.mode-card .stats {
  color: rgba(245, 241, 232, 0.6);
}

.mode-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(270px, 1fr));
  gap: 14px;
}

.mode-card {
  min-height: 260px;
  padding: 18px;
  display: grid;
  gap: 10px;
}

.mode-card:disabled {
  opacity: 0.48;
  cursor: not-allowed;
}

.mode-card .tag,
.mode-card .num {
  font-family: "JetBrains Mono", monospace;
  color: #ffb05a;
  font-size: 12px;
}

.mode-card .name {
  font-size: 32px;
  color: #f5f1e8;
}

.mode-card .stats {
  display: flex;
  justify-content: space-between;
  align-items: end;
}

.mode-action-bar {
  position: fixed;
  left: 36px;
  right: 36px;
  bottom: 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 18px;
  border: 1px solid rgba(255, 122, 24, 0.4);
  background: rgba(5, 6, 8, 0.92);
}

.mode-action-bar button {
  background: #ff7a18;
  color: #090604;
  border: 0;
  padding: 12px 18px;
  font-weight: 800;
}
```

- [ ] **Step 5: Run build/e2e and commit**

Run:

```bash
cd code/frontend
npm run build
npm run e2e -- play-flow.spec.ts
```

Expected: pass. Commit:

```bash
git add code/frontend/src/pages/ModeSelectPage.tsx code/frontend/src/pages/ModeSelectPage.css code/frontend/e2e/play-flow.spec.ts
git commit -m "feat: add in between format selection"
```

---

## Task 5: Deck Selection Using Real Deck Library Data

**Files:**
- Create: `code/frontend/src/pages/DeckSelectPage.tsx`
- Create: `code/frontend/src/pages/DeckSelectPage.css`
- Modify: `code/frontend/e2e/play-flow.spec.ts`

- [ ] **Step 1: Extend e2e for deck selection**

Add deck API mocks and assertion to `code/frontend/e2e/play-flow.spec.ts`:

```ts
async function mockDeckLibrary(page: import('@playwright/test').Page) {
  await page.route('**/api/decks/folders', (route) => route.fulfill({ json: [] }));
  await page.route('**/api/decks', (route) =>
    route.fulfill({
      json: [
        {
          id: 'deck-1',
          name: 'Ember Vanguard',
          description: '',
          game_mode: 'standard',
          is_valid: true,
          is_public: false,
          is_pinned: false,
          folder_id: null,
          card_count: 54,
          main_count: 50,
          egg_count: 4,
          tags: [],
          meta_tier: 'rogue',
          meta_archetype: 'Red Aggro',
          colors: ['Red'],
          highest_level: 6,
          created_at: '2026-04-27T00:00:00.000Z',
          updated_at: '2026-04-27T00:00:00.000Z',
        },
      ],
    }),
  );
  await page.route('**/api/decks/deck-1', (route) =>
    route.fulfill({
      json: {
        id: 'deck-1',
        owner_id: 'guest_abc',
        folder_id: null,
        name: 'Ember Vanguard',
        description: '',
        game_mode: 'standard',
        main_deck: Array(50).fill('BT1-001'),
        egg_deck: Array(4).fill('BT1-002'),
        main_deck_alt_arts: [],
        egg_deck_alt_arts: [],
        commander_id: null,
        is_valid: true,
        validation_errors: [],
        is_public: false,
        is_pinned: false,
        tags: [],
        meta_tier: 'rogue',
        meta_archetype: 'Red Aggro',
        created_at: '2026-04-27T00:00:00.000Z',
        updated_at: '2026-04-27T00:00:00.000Z',
      },
    }),
  );
}

test('selects a legal deck and advances to matching', async ({ page }) => {
  await mockDeckLibrary(page);
  await page.goto('/play');
  await page.getByRole('button', { name: /STANDARD/i }).click();
  await page.getByRole('button', { name: /ENTER FORMAT/i }).click();
  await page.getByRole('button', { name: /EMBER VANGUARD/i }).click();
  await page.getByRole('button', { name: /USE THIS DECK/i }).click();
  await expect(page).toHaveURL(/\/play\/matching/);
});
```

- [ ] **Step 2: Run the e2e and verify it fails**

Run:

```bash
cd code/frontend
npm run e2e -- play-flow.spec.ts
```

Expected: fail because `/play/deck` is not implemented.

- [ ] **Step 3: Implement deck select page**

Create `code/frontend/src/pages/DeckSelectPage.tsx`:

```tsx
import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import * as library from '@/api/deckLibraryAdapter';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { canUseDeckForFormat, getPlayFormat } from '@/features/play/formatCatalog';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import type { DeckSummary } from '@/types/deck';
import './DeckSelectPage.css';

export function DeckSelectPage() {
  const navigate = useNavigate();
  const { formatId, opponentMode, deckId, selectDeck } = usePlayFlowStore();
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [search, setSearch] = useState('');
  const format = getPlayFormat(formatId);
  const selected = decks.find((deck) => deck.id === deckId) ?? decks[0] ?? null;
  const selectedLegality = selected ? canUseDeckForFormat(selected, formatId) : null;

  useEffect(() => {
    library.listDecks().then((items) => {
      setDecks(items);
      const firstLegal = items.find((deck) => canUseDeckForFormat(deck, formatId).ok);
      selectDeck(firstLegal?.id ?? items[0]?.id ?? null);
    }).catch(() => setDecks([]));
  }, [formatId, selectDeck]);

  const filtered = useMemo(() => {
    const needle = search.trim().toLowerCase();
    if (!needle) return decks;
    return decks.filter((deck) =>
      `${deck.name} ${deck.meta_archetype ?? ''} ${deck.tags.join(' ')}`.toLowerCase().includes(needle),
    );
  }, [decks, search]);

  const nextPath = opponentMode === 'room' ? '/play/room/new' : opponentMode === 'bot' ? '/game' : '/play/matching';

  return (
    <InBetweenShell
      title="CHOOSE DECK"
      stepLabel="02"
      crumbs={[
        { label: 'PLAY', href: '/play' },
        { label: 'FORMAT', href: '/play' },
        { label: 'CHOOSE DECK' },
      ]}
      rightSlot={<span>{format.name} - {format.deckLabel}</span>}
    >
      <main className="deck-select-main">
        <section className="deck-select-banner">
          <div>
            <span className="label">FORMAT //</span>
            <h1>{format.name}</h1>
            <p>{format.description}</p>
          </div>
          <Link to="/play">CHANGE</Link>
        </section>

        <section className="deck-select-toolbar">
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Decks, archetypes, tags"
          />
          <Link to="/deckbuilder/new?returnTo=play">NEW DECK</Link>
        </section>

        <section className="deck-select-grid">
          {filtered.map((deck) => {
            const legality = canUseDeckForFormat(deck, formatId);
            return (
              <button
                key={deck.id}
                type="button"
                className={`deck-select-card ${deck.id === selected?.id ? 'selected' : ''} ${legality.ok ? '' : 'illegal'}`}
                onClick={() => selectDeck(deck.id)}
              >
                <span className="glyph">{deck.name.split(/\s+/).map((part) => part[0]).slice(0, 2).join('')}</span>
                <span className="name">{deck.name}</span>
                <span className="meta">{deck.main_count}/{deck.egg_count} - {deck.meta_archetype ?? 'Unclassified'}</span>
                <span className={legality.ok ? 'legal' : 'warn'}>
                  {legality.ok ? `LEGAL IN ${format.name}` : legality.reason}
                </span>
              </button>
            );
          })}
        </section>

        <div className="deck-confirm-bar">
          <span>{selected ? selected.name : 'NO DECK SELECTED'}</span>
          <button
            type="button"
            disabled={!selected || !selectedLegality?.ok}
            onClick={() => navigate(nextPath)}
          >
            USE THIS DECK
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
```

- [ ] **Step 4: Add deck select CSS**

Create `code/frontend/src/pages/DeckSelectPage.css`:

```css
.deck-select-main {
  padding: 26px 32px 98px;
}

.deck-select-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border: 1px solid rgba(255, 122, 24, 0.35);
  padding: 18px;
  background: rgba(255, 122, 24, 0.06);
}

.deck-select-banner .label,
.deck-select-toolbar a,
.deck-confirm-bar,
.deck-select-card .meta,
.deck-select-card .legal,
.deck-select-card .warn {
  font-family: "JetBrains Mono", monospace;
}

.deck-select-banner h1 {
  margin: 4px 0;
  color: #ff7a18;
}

.deck-select-banner p {
  margin: 0;
  color: rgba(245, 241, 232, 0.66);
}

.deck-select-banner a,
.deck-select-toolbar a {
  color: #ffb05a;
  text-decoration: none;
}

.deck-select-toolbar {
  margin: 18px 0;
  display: flex;
  gap: 12px;
  align-items: center;
}

.deck-select-toolbar input {
  flex: 1;
  background: rgba(255,255,255,0.06);
  border: 1px solid rgba(255,255,255,0.12);
  color: #f5f1e8;
  padding: 12px;
}

.deck-select-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
}

.deck-select-card {
  min-height: 190px;
  display: grid;
  gap: 9px;
  text-align: left;
  padding: 14px;
  color: #f5f1e8;
  background: rgba(255,255,255,0.045);
  border: 1px solid rgba(255,255,255,0.12);
}

.deck-select-card.selected {
  border-color: #ff7a18;
  box-shadow: 0 0 0 1px rgba(255,122,24,0.5);
}

.deck-select-card.illegal {
  opacity: 0.58;
}

.deck-select-card .glyph {
  width: 62px;
  height: 78px;
  display: grid;
  place-items: center;
  border: 1px solid rgba(255,122,24,0.45);
  color: #ff7a18;
  font-weight: 800;
}

.deck-select-card .name {
  font-size: 18px;
  font-weight: 800;
}

.deck-select-card .meta {
  color: rgba(245,241,232,0.56);
}

.deck-select-card .legal {
  color: #4cd497;
}

.deck-select-card .warn {
  color: #ffcc4a;
}

.deck-confirm-bar {
  position: fixed;
  left: 32px;
  right: 32px;
  bottom: 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 18px;
  background: rgba(5, 6, 8, 0.94);
  border: 1px solid rgba(255, 122, 24, 0.4);
}

.deck-confirm-bar button {
  background: #ff7a18;
  border: 0;
  color: #090604;
  font-weight: 800;
  padding: 12px 18px;
}

.deck-confirm-bar button:disabled {
  opacity: 0.42;
}
```

- [ ] **Step 5: Run e2e and commit**

Run:

```bash
cd code/frontend
npm run build:desktop
npm run e2e -- play-flow.spec.ts
```

Expected: pass. Commit:

```bash
git add code/frontend/src/pages/DeckSelectPage.tsx code/frontend/src/pages/DeckSelectPage.css code/frontend/e2e/play-flow.spec.ts
git commit -m "feat: add in between deck selection"
```

---

## Task 6: Matchmaking Screen Backed By Queue API

**Files:**
- Create: `code/frontend/src/pages/MatchingPage.tsx`
- Create: `code/frontend/src/pages/MatchingPage.css`
- Modify: `code/frontend/e2e/play-flow.spec.ts`

- [ ] **Step 1: Extend e2e for queue call**

Add to `code/frontend/e2e/play-flow.spec.ts`:

```ts
test('queues selected deck for quick match', async ({ page }) => {
  await mockDeckLibrary(page);
  let queuePayload: unknown = null;
  await page.route('**/api/matchmaking/queue', async (route) => {
    queuePayload = route.request().postDataJSON();
    await route.fulfill({ json: { status: 'waiting', ticket_id: 'ticket-1' } });
  });
  await page.goto('/play');
  await page.getByRole('button', { name: /STANDARD/i }).click();
  await page.getByRole('button', { name: /ENTER FORMAT/i }).click();
  await page.getByRole('button', { name: /EMBER VANGUARD/i }).click();
  await page.getByRole('button', { name: /USE THIS DECK/i }).click();
  await expect(page.getByText(/SEARCHING/i)).toBeVisible();
  expect(queuePayload).toMatchObject({
    queue_type: 'casual',
    game_mode: 'standard',
  });
});
```

- [ ] **Step 2: Run e2e and verify it fails**

Run:

```bash
cd code/frontend
npm run e2e -- play-flow.spec.ts
```

Expected: fail because `MatchingPage` is not implemented.

- [ ] **Step 3: Implement matching page**

Create `code/frontend/src/pages/MatchingPage.tsx`:

```tsx
import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { getPlayFormat } from '@/features/play/formatCatalog';
import { getDeck } from '@/features/play/playApi';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import { useMatchmaking } from '@/hooks/useMatchmaking';
import type { DeckResponse } from '@/types/deck';
import './MatchingPage.css';

export function MatchingPage() {
  const navigate = useNavigate();
  const { formatId, deckId, queueType, setQueue } = usePlayFlowStore();
  const matchmaking = useMatchmaking();
  const [deck, setDeck] = useState<DeckResponse | null>(null);
  const [localWait, setLocalWait] = useState(0);
  const format = getPlayFormat(formatId);

  useEffect(() => {
    if (!deckId) {
      navigate('/play/deck', { replace: true });
      return;
    }
    let cancelled = false;
    getDeck(deckId).then((loaded) => {
      if (!cancelled) setDeck(loaded);
    }).catch(() => navigate('/play/deck', { replace: true }));
    return () => {
      cancelled = true;
    };
  }, [deckId, navigate]);

  useEffect(() => {
    if (!deck || matchmaking.status !== 'idle') return;
    void matchmaking.enqueue({
      queue_type: queueType,
      main_deck: deck.main_deck,
      egg_deck: deck.egg_deck,
      game_mode: formatId,
    });
  }, [deck, formatId, matchmaking, queueType]);

  useEffect(() => {
    if (matchmaking.ticketId) setQueue({ ticketId: matchmaking.ticketId });
  }, [matchmaking.ticketId, setQueue]);

  useEffect(() => {
    if (matchmaking.status !== 'waiting' && matchmaking.status !== 'connecting') {
      setLocalWait(0);
      return;
    }
    const id = window.setInterval(() => setLocalWait((value) => value + 1), 1000);
    return () => window.clearInterval(id);
  }, [matchmaking.status]);

  useEffect(() => {
    if (matchmaking.status !== 'matched' || !matchmaking.match) return;
    setQueue({ roomCode: matchmaking.match.join_code, gameId: matchmaking.match.game_id });
    navigate(`/game/${matchmaking.match.game_id}?mode=pvp&player=1`);
  }, [matchmaking.match, matchmaking.status, navigate, setQueue]);

  const elapsed = Math.max(localWait, Math.floor(matchmaking.waitedSeconds));
  const initials = useMemo(
    () => deck?.name.split(/\s+/).map((part) => part[0]).slice(0, 2).join('') ?? '??',
    [deck],
  );

  return (
    <InBetweenShell
      title="MATCHMAKING"
      stepLabel="03"
      crumbs={[
        { label: 'PLAY', href: '/play' },
        { label: 'DECK', href: '/play/deck' },
        { label: 'MATCHING' },
      ]}
      rightSlot={<span>{format.name} - {deck?.name ?? 'LOADING'}</span>}
    >
      <main className="matching-main">
        <header className="matching-header">
          <div className="welcome">// MATCHMAKING SERVICE - NA-WEST RELAY</div>
          <h1>SEARCHING<br /><em>FOR AN OPPONENT.</em></h1>
          <p>SCANNING THE LADDER - BALANCED PAIRING - TYPICAL WAIT 25-45 SECONDS</p>
        </header>

        <section className="matching-stage">
          <article className="match-player-card p1">
            <span className="role">YOU</span>
            <div className="deck-art">{initials}</div>
            <h2>{deck?.name ?? 'Loading deck'}</h2>
            <p>{deck ? `${deck.main_deck.length}/50 main - ${deck.egg_deck.length} eggs` : 'Resolving deck'}</p>
            <span className="ready">READY</span>
          </article>

          <div className="matching-radar">
            <div className="pulse">VS</div>
            <strong>{Math.floor(elapsed / 60)}:{String(elapsed % 60).padStart(2, '0')}</strong>
            <span>{matchmaking.ratingWindow ? `RANGE +/-${matchmaking.ratingWindow}` : 'SCANNING...'}</span>
          </div>

          <article className="match-player-card p2">
            <span className="role">OPPONENT</span>
            <div className="deck-art muted">??</div>
            <h2>SEARCHING...</h2>
            <p>{matchmaking.error ?? 'Awaiting handshake'}</p>
            <span className="ready waiting">{matchmaking.status.toUpperCase()}</span>
          </article>
        </section>

        <div className="matching-actions">
          <button type="button" onClick={() => void matchmaking.cancel()}>
            CANCEL SEARCH
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
```

- [ ] **Step 4: Add matching CSS**

Create `code/frontend/src/pages/MatchingPage.css`:

```css
.matching-main {
  padding: 34px 36px;
}

.matching-header .welcome,
.matching-header p,
.match-player-card .role,
.match-player-card .ready,
.matching-radar {
  font-family: "JetBrains Mono", monospace;
}

.matching-header .welcome {
  color: #ffb05a;
  letter-spacing: 0.18em;
}

.matching-header h1 {
  margin: 8px 0;
  font-size: clamp(42px, 8vw, 88px);
  line-height: 0.88;
}

.matching-header em {
  color: #ff7a18;
  font-style: normal;
}

.matching-stage {
  display: grid;
  grid-template-columns: minmax(240px, 1fr) 260px minmax(240px, 1fr);
  align-items: stretch;
  gap: 18px;
  margin-top: 34px;
}

.match-player-card {
  min-height: 360px;
  border: 1px solid rgba(255,255,255,0.14);
  background: rgba(255,255,255,0.045);
  padding: 18px;
  display: grid;
  gap: 14px;
  align-content: start;
}

.match-player-card.p1 {
  border-color: rgba(255,122,24,0.44);
}

.match-player-card.p2 {
  border-color: rgba(58,166,255,0.35);
}

.deck-art {
  width: 96px;
  height: 128px;
  display: grid;
  place-items: center;
  border: 1px solid #ff7a18;
  color: #ff7a18;
  font-size: 28px;
  font-weight: 900;
}

.deck-art.muted {
  border-color: rgba(245,241,232,0.22);
  color: rgba(245,241,232,0.32);
}

.matching-radar {
  display: grid;
  place-items: center;
  align-content: center;
  gap: 12px;
  border: 1px solid rgba(255,255,255,0.14);
  background: radial-gradient(circle, rgba(255,122,24,0.16), rgba(255,255,255,0.035));
}

.matching-radar .pulse {
  width: 132px;
  height: 132px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  border: 1px solid rgba(255,122,24,0.5);
  color: #ff7a18;
}

.matching-actions {
  margin-top: 22px;
  display: flex;
  justify-content: center;
}

.matching-actions button {
  border: 1px solid rgba(255,82,82,0.55);
  color: #ff7777;
  background: rgba(255,82,82,0.08);
  padding: 12px 18px;
}
```

- [ ] **Step 5: Run e2e and commit**

Run:

```bash
cd code/frontend
npm run build
npm run e2e -- play-flow.spec.ts
```

Expected: pass. Commit:

```bash
git add code/frontend/src/pages/MatchingPage.tsx code/frontend/src/pages/MatchingPage.css code/frontend/e2e/play-flow.spec.ts
git commit -m "feat: add in between matchmaking screen"
```

---

## Task 7: Room Lobby Screen Backed By Lobby API

**Files:**
- Create: `code/frontend/src/pages/RoomLobbyPage.tsx`
- Create: `code/frontend/src/pages/RoomLobbyPage.css`
- Modify: `code/frontend/e2e/play-flow.spec.ts`

- [ ] **Step 1: Add room e2e**

Append:

```ts
test('creates a room from selected deck', async ({ page }) => {
  await mockDeckLibrary(page);
  await page.route('**/api/lobby/create', (route) =>
    route.fulfill({ json: { game_id: 'game-1', join_code: 'ABC123' } }),
  );
  await page.goto('/play');
  await page.getByRole('button', { name: /ROOM MATCH/i }).click();
  await page.getByRole('button', { name: /STANDARD/i }).click();
  await page.getByRole('button', { name: /ENTER FORMAT/i }).click();
  await page.getByRole('button', { name: /EMBER VANGUARD/i }).click();
  await page.getByRole('button', { name: /USE THIS DECK/i }).click();
  await expect(page).toHaveURL(/\/play\/room\/new/);
  await expect(page.getByText('ABC123')).toBeVisible();
});
```

- [ ] **Step 2: Run e2e and verify it fails**

Run:

```bash
cd code/frontend
npm run e2e -- play-flow.spec.ts
```

Expected: fail because `RoomLobbyPage` is not implemented.

- [ ] **Step 3: Implement room lobby page**

Create `code/frontend/src/pages/RoomLobbyPage.tsx`:

```tsx
import { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import * as lobbyApi from '@/api/lobbyApi';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { getPlayFormat } from '@/features/play/formatCatalog';
import { getDeck } from '@/features/play/playApi';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import type { DeckResponse } from '@/types/deck';
import './RoomLobbyPage.css';

export function RoomLobbyPage() {
  const { gameId: routeGameId } = useParams();
  const navigate = useNavigate();
  const { deckId, formatId, roomCode, gameId, setQueue } = usePlayFlowStore();
  const [deck, setDeck] = useState<DeckResponse | null>(null);
  const [creating, setCreating] = useState(routeGameId === 'new');
  const format = getPlayFormat(formatId);

  useEffect(() => {
    if (!deckId) {
      navigate('/play/deck', { replace: true });
      return;
    }
    getDeck(deckId).then(setDeck).catch(() => navigate('/play/deck', { replace: true }));
  }, [deckId, navigate]);

  useEffect(() => {
    if (!deck || routeGameId !== 'new') return;
    let cancelled = false;
    setCreating(true);
    lobbyApi.createLobby({
      deck: [...deck.egg_deck, ...deck.main_deck],
      is_public: false,
      allow_spectators: true,
      spectator_mode: 'hidden',
    }).then((room) => {
      if (cancelled) return;
      setQueue({ gameId: room.game_id, roomCode: room.join_code });
      navigate(`/play/room/${room.game_id}`, { replace: true });
    }).finally(() => {
      if (!cancelled) setCreating(false);
    });
    return () => {
      cancelled = true;
    };
  }, [deck, navigate, routeGameId, setQueue]);

  const visibleCode = roomCode ?? '------';
  const visibleGameId = gameId ?? (routeGameId === 'new' ? null : routeGameId);

  return (
    <InBetweenShell
      title="ROOM LOBBY"
      stepLabel="02"
      crumbs={[{ label: 'PLAY', href: '/play' }, { label: 'ROOM MATCH' }, { label: 'LOBBY' }]}
      rightSlot={<span>{format.name}</span>}
    >
      <main className="room-main">
        <header className="room-header">
          <div>
            <span>SHARE THE ROOM CODE BELOW - PRIVATE</span>
            <h1>ROOM LOBBY</h1>
          </div>
          <div className="room-code-card">
            <span>// ROOM CODE</span>
            <strong>{creating ? '------' : visibleCode}</strong>
            <button type="button" onClick={() => void navigator.clipboard.writeText(visibleCode)}>
              COPY
            </button>
          </div>
        </header>

        <section className="room-grid">
          <article className="room-player p1">
            <span className="role">HOST</span>
            <h2>YOU</h2>
            <div className="deck-slot">
              <strong>{deck?.name ?? 'Loading deck'}</strong>
              <span>{deck ? `${deck.main_deck.length}/50 main - ${deck.egg_deck.length} eggs` : 'Resolving deck'}</span>
            </div>
            <span className="ready on">READY</span>
          </article>
          <article className="room-player p2 empty">
            <span className="role">OPPONENT</span>
            <h2>WAITING...</h2>
            <div className="deck-slot">
              <strong>NO DECK LOCKED</strong>
              <span>Share code {visibleCode}</span>
            </div>
            <span className="ready waiting">WAITING</span>
          </article>
        </section>

        <div className="room-actions">
          <button type="button" disabled={!visibleGameId} onClick={() => navigate(`/game/${visibleGameId}?mode=pvp&player=1`)}>
            ENTER GAME
          </button>
          <button type="button" onClick={() => navigate('/play/deck')}>
            CHANGE DECK
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
```

- [ ] **Step 4: Add room CSS**

Create `code/frontend/src/pages/RoomLobbyPage.css`:

```css
.room-main {
  padding: 30px 34px;
}

.room-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
}

.room-header span,
.room-code-card,
.room-player .role,
.room-player .ready,
.room-actions button {
  font-family: "JetBrains Mono", monospace;
}

.room-header h1 {
  margin: 6px 0;
  color: #ff7a18;
}

.room-code-card {
  display: grid;
  gap: 6px;
  min-width: 240px;
  border: 1px solid #ff7a18;
  padding: 14px;
  background: rgba(255,122,24,0.06);
}

.room-code-card strong {
  font-size: 38px;
  color: #ff7a18;
  letter-spacing: 0.16em;
}

.room-code-card button,
.room-actions button {
  border: 1px solid rgba(255,255,255,0.16);
  background: rgba(255,255,255,0.05);
  color: #f5f1e8;
  padding: 10px 12px;
}

.room-grid {
  margin-top: 30px;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 18px;
}

.room-player {
  min-height: 320px;
  border: 1px solid rgba(255,255,255,0.14);
  background: rgba(255,255,255,0.045);
  padding: 18px;
  display: grid;
  gap: 18px;
  align-content: start;
}

.room-player.p1 { border-color: rgba(255,122,24,0.5); }
.room-player.p2 { border-color: rgba(58,166,255,0.45); }
.room-player.empty { opacity: 0.72; }

.deck-slot {
  min-height: 120px;
  border: 1px solid rgba(255,255,255,0.12);
  display: grid;
  gap: 8px;
  align-content: center;
  padding: 16px;
}

.ready.on { color: #4cd497; }
.ready.waiting { color: #ffcc4a; }

.room-actions {
  margin-top: 22px;
  display: flex;
  gap: 12px;
}
```

- [ ] **Step 5: Run e2e and commit**

Run:

```bash
cd code/frontend
npm run build
npm run e2e -- play-flow.spec.ts
```

Expected: pass. Commit:

```bash
git add code/frontend/src/pages/RoomLobbyPage.tsx code/frontend/src/pages/RoomLobbyPage.css code/frontend/e2e/play-flow.spec.ts
git commit -m "feat: add in between room lobby"
```

---

## Task 8: Bot Match And Game Board Launch

**Files:**
- Modify: `code/frontend/src/pages/DeckSelectPage.tsx`
- Modify: `code/frontend/src/pages/GamePage.tsx`
- Test: `code/frontend/e2e/play-flow.spec.ts`

- [ ] **Step 1: Add bot launch e2e**

Append:

```ts
test('bot match starts local game route from deck selection', async ({ page }) => {
  await mockDeckLibrary(page);
  await page.route('**/api/games', (route) =>
    route.fulfill({
      json: {
        game_id: 'game-bot',
        state: {
          turn_count: 1,
          current_phase: 'Main',
          memory: 0,
          game_over: false,
          winner: null,
          players: [],
        },
        action_mask: [],
      },
    }),
  );
  await page.goto('/play');
  await page.getByRole('button', { name: /BOT MATCH/i }).click();
  await page.getByRole('button', { name: /STANDARD/i }).click();
  await page.getByRole('button', { name: /ENTER FORMAT/i }).click();
  await page.getByRole('button', { name: /EMBER VANGUARD/i }).click();
  await page.getByRole('button', { name: /USE THIS DECK/i }).click();
  await expect(page).toHaveURL(/\/game/);
});
```

- [ ] **Step 2: Run e2e and verify it fails**

Run:

```bash
cd code/frontend
npm run e2e -- play-flow.spec.ts
```

Expected: fail because bot mode currently routes directly to `/game` without creating a game.

- [ ] **Step 3: Update DeckSelectPage bot launch**

Replace the confirm handler in `code/frontend/src/pages/DeckSelectPage.tsx` with:

```tsx
const [launching, setLaunching] = useState(false);

const handleConfirm = async () => {
  if (!selected || !selectedLegality?.ok) return;
  if (opponentMode === 'quick') {
    navigate('/play/matching');
    return;
  }
  if (opponentMode === 'room') {
    navigate('/play/room/new');
    return;
  }
  setLaunching(true);
  try {
    const deck = await library.getDeck(selected.id);
    const opponentDeck = await library.getDeck(selected.id);
    const response = await gameApi.createGame({
      deck1: [...deck.egg_deck, ...deck.main_deck],
      deck2: [...opponentDeck.egg_deck, ...opponentDeck.main_deck],
      player_kinds: ['human', 'greedy'],
      player_model_ids: [null, null],
    });
    navigate(`/game/${response.game_id}`);
  } finally {
    setLaunching(false);
  }
};
```

Add imports:

```tsx
import * as gameApi from '@/api/gameApi';
```

Update button:

```tsx
<button
  type="button"
  disabled={!selected || !selectedLegality?.ok || launching}
  onClick={handleConfirm}
>
  {launching ? 'LAUNCHING...' : 'USE THIS DECK'}
</button>
```

- [ ] **Step 4: Add GamePage launch metadata**

In `code/frontend/src/pages/GamePage.tsx`, read play-flow metadata and pass labels to the board header already rendered by `GameBoard`:

```tsx
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import { getPlayFormat } from '@/features/play/formatCatalog';

const { formatId, opponentMode } = usePlayFlowStore();
const playFormat = getPlayFormat(formatId);
```

Where `playerLabels` are set after `createGame`, include:

```ts
store.setPlayerLabels({
  0: 'YOU',
  1: opponentMode === 'bot' ? 'GREEDY BOT' : 'OPPONENT',
});
```

If the current `GamePage` already sets `playerLabels` from backend response, use:

```ts
if (result.player_labels) {
  store.setPlayerLabels(result.player_labels);
} else {
  store.setPlayerLabels({
    0: 'YOU',
    1: opponentMode === 'bot' ? 'GREEDY BOT' : 'OPPONENT',
  });
}
```

- [ ] **Step 5: Run e2e/build and commit**

Run:

```bash
cd code/frontend
npm run build:desktop
npm run e2e -- play-flow.spec.ts game-loads.spec.ts
```

Expected: play flow passes; `game-loads.spec.ts` passes when backend debug server is available. If backend is unavailable, record that explicitly in the commit notes. Commit:

```bash
git add code/frontend/src/pages/DeckSelectPage.tsx code/frontend/src/pages/GamePage.tsx code/frontend/e2e/play-flow.spec.ts
git commit -m "feat: launch games from in between deck select"
```

---

## Task 9: Deck Library And Builder Mock Polish

**Files:**
- Modify: `code/frontend/src/pages/DeckLibraryPage.tsx`
- Modify: `code/frontend/src/pages/DeckLibraryPage.css`
- Modify: `code/frontend/src/pages/DeckBuilderPage.tsx`
- Modify: `code/frontend/e2e/deck-library.spec.ts`

- [ ] **Step 1: Add deck library e2e for new mock affordances**

Extend `code/frontend/e2e/deck-library.spec.ts`:

```ts
await expect(page.getByText('Deck Library').or(page.getByText('DECK LIBRARY'))).toBeVisible();
await expect(page.getByRole('link', { name: /New Deck/i })).toBeVisible();
await page.getByRole('link', { name: /New Deck/i }).click();
await expect(page).toHaveURL(/\/deckbuilder\/new/);
```

- [ ] **Step 2: Run e2e and verify it fails if affordances are missing**

Run:

```bash
cd code/frontend
npm run e2e -- deck-library.spec.ts
```

Expected: fail if `New Deck` is a button without route navigation or if heading text differs.

- [ ] **Step 3: Add library header and action links**

In `code/frontend/src/pages/DeckLibraryPage.tsx`, add a top action area before the filter controls:

```tsx
<header className="library-hero">
  <div>
    <span className="library-kicker">// ARMORY</span>
    <h1>DECK LIBRARY</h1>
    <p>Organize folders, inspect legality, pin tournament lists, and open the builder.</p>
  </div>
  <div className="library-hero-actions">
    <Link to="/deckbuilder/new" className="library-command primary">New Deck</Link>
    <Link to="/deckbuilder/new?import=1" className="library-command">Import</Link>
  </div>
</header>
```

Add import:

```tsx
import { Link } from 'react-router-dom';
```

- [ ] **Step 4: Add library CSS**

In `code/frontend/src/pages/DeckLibraryPage.css`, add:

```css
.library-hero {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 18px;
  margin-bottom: 18px;
}

.library-kicker {
  color: #ffb05a;
  font-family: "JetBrains Mono", monospace;
  letter-spacing: 0.18em;
  font-size: 12px;
}

.library-hero h1 {
  margin: 4px 0;
  color: #f5f1e8;
  letter-spacing: 0;
}

.library-hero p {
  margin: 0;
  color: rgba(245, 241, 232, 0.62);
}

.library-hero-actions {
  display: flex;
  gap: 10px;
}

.library-command {
  border: 1px solid rgba(255,255,255,0.16);
  color: #f5f1e8;
  text-decoration: none;
  padding: 10px 12px;
  font-family: "JetBrains Mono", monospace;
}

.library-command.primary {
  background: #ff7a18;
  border-color: #ff7a18;
  color: #090604;
  font-weight: 800;
}
```

- [ ] **Step 5: Run e2e and commit**

Run:

```bash
cd code/frontend
npm run build
npm run e2e -- deck-library.spec.ts
```

Expected: pass. Commit:

```bash
git add code/frontend/src/pages/DeckLibraryPage.tsx code/frontend/src/pages/DeckLibraryPage.css code/frontend/src/pages/DeckBuilderPage.tsx code/frontend/e2e/deck-library.spec.ts
git commit -m "feat: polish deck library for in between mock"
```

---

## Task 10: Final Verification And Desktop Build

**Files:**
- Modify if failures require fixes: files touched by earlier tasks only.

- [ ] **Step 1: Run unit tests**

Run:

```bash
cd code/frontend
npm test -- src/features/play/formatCatalog.test.ts src/features/play/playFlowStore.test.ts src/utils/deckLibrary.test.ts src/components/launcher/launcherData.test.ts src/components/board/ActionTraceTicker.test.tsx
```

Expected: all tests pass.

- [ ] **Step 2: Run frontend builds**

Run:

```bash
cd code/frontend
npm run build
npm run build:desktop
```

Expected: both builds pass.

- [ ] **Step 3: Run Playwright specs**

Run:

```bash
cd code/frontend
npm run e2e -- play-flow.spec.ts deck-library.spec.ts launcher.spec.ts guest-onboarding.spec.ts
```

Expected: all specs pass.

- [ ] **Step 4: Run backend and Tauri checks**

Run:

```bash
pytest code/tests/api/test_formats.py code/tests/api/test_decks_library.py -q
cargo tauri build --no-bundle --config '{"build":{"beforeBuildCommand":""}}'
```

Expected: API tests pass; Tauri release executable builds at `target/release/digimon-tcg.exe`.

- [ ] **Step 5: Manual desktop smoke**

Run:

```powershell
Start-Process -FilePath "C:\Users\james\.codex\worktrees\16a1\digimon-deck-list-builder-1\target\release\digimon-tcg.exe"
```

Expected:
- Launcher opens.
- PLAY opens `/play`.
- Standard Quick Match advances to deck select.
- Deck selection advances to matching.
- Room Match creates a code.
- Bot Match launches `/game/<id>` and renders the board.
- Deck Library still opens from launcher and allows Edit/New Deck.

- [ ] **Step 6: Final commit**

Run:

```bash
git status --short
git add code/frontend code/server code/src-tauri code/tests docs/superpowers/plans
git commit -m "feat: wire in between theatre play flow"
```

Expected: working tree clean after commit.

---

## Self-Review

**Spec coverage:** The plan covers matchmaking (`MatchingPage`, `useMatchmaking`), deck building (`DeckBuilderPage` return/import path), deck organization (`DeckLibraryPage`, folders/pins stay through `deckLibraryAdapter`), format selection (`ModeSelectPage`, web/Tauri format catalog), room match (`RoomLobbyPage`, `lobbyApi`), bot match and engine launch (`gameApi.createGame`), and desktop build verification (`cargo tauri build`).

**Placeholder scan:** The plan contains concrete file paths, commands, route names, DTO shapes, and code snippets. Disabled non-standard formats have an explicit shipped reason rather than a vague future marker.

**Type consistency:** `PlayFormatId`, `OpponentMode`, `DeckSummary`, `DeckResponse`, and `QueueType` names are defined before use and reused consistently across tasks. Route paths are `/play`, `/play/deck`, `/play/matching`, and `/play/room/:gameId` throughout.

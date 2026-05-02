# Desktop Launcher Design Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the desktop app home screen with the new launcher design from `C:\Users\james\Downloads\In Between Theatre.zip` and wire it to real app data, local desktop storage, and existing FastAPI server endpoints.

**Architecture:** The launcher is a desktop-only React route mounted outside the normal `Layout` so it can own its window-like chrome and sidebar. It reads real deck data from the existing desktop `deckStore`, server status from `/health`, release/news data from `/patch-notes`, tested-card count from the existing deck API adapter, and auth identity from `authStore`. No new backend endpoint is needed because the existing server already exposes the data needed for this first launcher slice.

**Tech Stack:** React 19, Vite, TypeScript, Tauri invoke adapters, Zustand auth/deck stores, Axios API client, Vitest, Playwright.

---

## Scope Check

This plan handles one cohesive subsystem: the desktop launcher entry screen. It intentionally does not restyle the game board or deck builder screens from the same zip. Those should be separate plans because they affect large gameplay and builder surfaces independently.

## Design Source

- Source zip: `C:\Users\james\Downloads\In Between Theatre.zip`
- Primary reference: `Launcher.html`
- Supporting design tokens: `tokens.css`
- Related but out-of-scope references: `board.css`, `board.jsx`, `deck-builder.css`, `deck-builder.jsx`

## File Structure

- Create `code/frontend/src/api/systemApi.ts`: tiny typed API wrapper for `/health`.
- Create `code/frontend/src/components/launcher/launcherData.ts`: pure formatting and view-model helpers for deck rows, release summaries, counts, and status labels.
- Create `code/frontend/src/components/launcher/launcherData.test.ts`: unit tests for the pure launcher data helpers.
- Create `code/frontend/src/components/launcher/LauncherPage.tsx`: desktop launcher screen that loads data and renders the imported design.
- Create `code/frontend/src/components/launcher/LauncherShell.tsx`: titlebar, sidebar, body frame, and status bar.
- Create `code/frontend/src/components/launcher/LauncherActions.tsx`: primary action tiles and quick links.
- Create `code/frontend/src/components/launcher/LauncherDeckPanel.tsx`: saved deck list with new/import/view-all actions.
- Create `code/frontend/src/components/launcher/LauncherNewsPanel.tsx`: latest release and known issue summary.
- Create `code/frontend/src/components/launcher/launcher.css`: CSS port from `Launcher.html`, renamed under `.launcher-*` classes to avoid global collisions.
- Modify `code/frontend/src/App.tsx`: mount desktop `/` as `LauncherPage` outside `Layout`; keep web `/` as `HomePage`.
- Modify `code/frontend/src/pages/DeckBuilderPage.tsx`: load `/deckbuilder/:id` and open import modal from `?import=1` so launcher links land on real workflows.
- Modify `code/frontend/e2e/guest-onboarding.spec.ts`: expect the desktop launcher heading when running desktop-mode e2e.
- Create `code/frontend/e2e/launcher.spec.ts`: verifies launcher data wiring, navigation, and offline server state.

---

### Task 1: Add Pure Launcher Data Helpers

**Files:**
- Create: `code/frontend/src/components/launcher/launcherData.test.ts`
- Create: `code/frontend/src/components/launcher/launcherData.ts`

- [ ] **Step 1: Write the failing tests**

Create `code/frontend/src/components/launcher/launcherData.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import {
  buildDeckRows,
  countDraftDecks,
  formatCardCount,
  formatRelativeEdit,
  summarizeLatestRelease,
} from './launcherData';
import type { DeckSummary } from '@/types/deck';
import type { PatchNotesResponse } from '@/api/patchNotesApi';

const decks: DeckSummary[] = [
  {
    id: 'deck-valid',
    name: 'Red Hybrid',
    game_mode: 'standard',
    is_valid: true,
    is_public: false,
    card_count: 50,
    meta_tier: 'L6',
    meta_archetype: 'Red Aggro',
    created_at: '2026-04-20T12:00:00.000Z',
    updated_at: '2026-04-26T16:00:00.000Z',
  },
  {
    id: 'deck-draft',
    name: 'Green Insect Rush',
    game_mode: 'standard',
    is_valid: false,
    is_public: false,
    card_count: 43,
    meta_tier: null,
    meta_archetype: null,
    created_at: '2026-04-10T12:00:00.000Z',
    updated_at: '2026-04-21T17:00:00.000Z',
  },
];

describe('launcherData', () => {
  it('builds deck rows with legal/draft labels and deckbuilder links', () => {
    const rows = buildDeckRows(decks, new Date('2026-04-26T18:00:00.000Z'));

    expect(rows).toEqual([
      {
        id: 'deck-valid',
        name: 'RED HYBRID',
        href: '/deckbuilder/deck-valid',
        countLabel: '50/50',
        statusLabel: 'BO3 LEGAL',
        statusKind: 'legal',
        levelLabel: 'L6',
        metaLabel: 'RED AGGRO',
        editedLabel: 'EDIT 2H AGO',
      },
      {
        id: 'deck-draft',
        name: 'GREEN INSECT RUSH',
        href: '/deckbuilder/deck-draft',
        countLabel: '43/50',
        statusLabel: 'DRAFT',
        statusKind: 'draft',
        levelLabel: 'L?',
        metaLabel: 'UNCLASSIFIED',
        editedLabel: 'EDIT 5D AGO',
      },
    ]);
  });

  it('counts draft decks from invalid or incomplete summaries', () => {
    expect(countDraftDecks(decks)).toBe(1);
  });

  it('formats card counts with thousands separators', () => {
    expect(formatCardCount(4127)).toBe('4,127');
    expect(formatCardCount(null)).toBe('—');
  });

  it('formats relative edit labels for recent, daily, and weekly edits', () => {
    const now = new Date('2026-04-26T18:00:00.000Z');
    expect(formatRelativeEdit('2026-04-26T17:54:00.000Z', now)).toBe('EDIT JUST NOW');
    expect(formatRelativeEdit('2026-04-26T14:00:00.000Z', now)).toBe('EDIT 4H AGO');
    expect(formatRelativeEdit('2026-04-24T18:00:00.000Z', now)).toBe('EDIT 2D AGO');
    expect(formatRelativeEdit('2026-04-10T18:00:00.000Z', now)).toBe('EDIT 2W AGO');
  });

  it('summarizes the latest release from patch notes', () => {
    const patchNotes: PatchNotesResponse = {
      known_issues: [],
      releases: [
        {
          id: 'old',
          version: '0.4.1',
          release_date: '2026-04-20',
          title: 'Old build',
          added: ['Old feature'],
          changed: [],
          fixed: [],
          created_at: '2026-04-20T00:00:00.000Z',
          updated_at: '2026-04-20T00:00:00.000Z',
        },
        {
          id: 'new',
          version: '0.4.2',
          release_date: '2026-04-24',
          title: 'Launcher polish',
          added: ['Desktop launcher'],
          changed: ['Guest boot flow'],
          fixed: [],
          created_at: '2026-04-24T00:00:00.000Z',
          updated_at: '2026-04-24T00:00:00.000Z',
        },
      ],
    };

    expect(summarizeLatestRelease(patchNotes, new Date('2026-04-26T18:00:00.000Z'))).toEqual({
      title: 'Launcher polish',
      versionLabel: 'v0.4.2 · 2D AGO',
      bullets: ['Desktop launcher', 'Guest boot flow'],
    });
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```powershell
cd code/frontend
npm test -- src/components/launcher/launcherData.test.ts
```

Expected: FAIL because `launcherData.ts` does not exist.

- [ ] **Step 3: Implement the pure helpers**

Create `code/frontend/src/components/launcher/launcherData.ts`:

```ts
import type { PatchNotesResponse, Release } from '@/api/patchNotesApi';
import type { DeckSummary } from '@/types/deck';

export interface LauncherDeckRow {
  id: string;
  name: string;
  href: string;
  countLabel: string;
  statusLabel: string;
  statusKind: 'legal' | 'draft';
  levelLabel: string;
  metaLabel: string;
  editedLabel: string;
}

export interface LauncherReleaseSummary {
  title: string;
  versionLabel: string;
  bullets: string[];
}

export function formatRelativeEdit(isoDate: string, now = new Date()): string {
  const edited = new Date(isoDate);
  const diffMs = Math.max(0, now.getTime() - edited.getTime());
  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 10) return 'EDIT JUST NOW';
  if (minutes < 60) return `EDIT ${minutes}M AGO`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `EDIT ${hours}H AGO`;
  const days = Math.floor(hours / 24);
  if (days < 14) return `EDIT ${days}D AGO`;
  const weeks = Math.floor(days / 7);
  return `EDIT ${weeks}W AGO`;
}

export function formatCardCount(count: number | null | undefined): string {
  return typeof count === 'number' ? new Intl.NumberFormat('en-US').format(count) : '—';
}

export function countDraftDecks(decks: DeckSummary[]): number {
  return decks.filter((deck) => !deck.is_valid || deck.card_count < 50).length;
}

export function buildDeckRows(decks: DeckSummary[], now = new Date()): LauncherDeckRow[] {
  return decks
    .slice()
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
    .slice(0, 5)
    .map((deck) => {
      const legal = deck.is_valid && deck.card_count === 50;
      return {
        id: deck.id,
        name: deck.name.toUpperCase(),
        href: `/deckbuilder/${deck.id}`,
        countLabel: `${deck.card_count}/50`,
        statusLabel: legal ? 'BO3 LEGAL' : 'DRAFT',
        statusKind: legal ? 'legal' : 'draft',
        levelLabel: deck.meta_tier?.toUpperCase() ?? 'L?',
        metaLabel: deck.meta_archetype?.toUpperCase() ?? 'UNCLASSIFIED',
        editedLabel: formatRelativeEdit(deck.updated_at, now),
      };
    });
}

function latestRelease(releases: Release[]): Release | null {
  return releases
    .slice()
    .sort((a, b) => new Date(b.release_date).getTime() - new Date(a.release_date).getTime())[0] ?? null;
}

export function summarizeLatestRelease(
  patchNotes: PatchNotesResponse | null,
  now = new Date(),
): LauncherReleaseSummary {
  const release = patchNotes ? latestRelease(patchNotes.releases) : null;
  if (!release) {
    return {
      title: 'No release notes published',
      versionLabel: 'NEWS UNAVAILABLE',
      bullets: ['Server release feed is not reachable.'],
    };
  }
  const bullets = [...release.added, ...release.changed, ...release.fixed].slice(0, 3);
  return {
    title: release.title ?? `Version ${release.version}`,
    versionLabel: `v${release.version} · ${formatRelativeEdit(release.release_date, now).replace('EDIT ', '')}`,
    bullets: bullets.length > 0 ? bullets : ['Release notes are available in Patch Notes.'],
  };
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run:

```powershell
cd code/frontend
npm test -- src/components/launcher/launcherData.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```powershell
git add code/frontend/src/components/launcher/launcherData.ts code/frontend/src/components/launcher/launcherData.test.ts
git commit -m "feat: add launcher data helpers"
```

---

### Task 2: Add System Health API Wrapper

**Files:**
- Create: `code/frontend/src/api/systemApi.ts`

- [ ] **Step 1: Create the API wrapper**

Create `code/frontend/src/api/systemApi.ts`:

```ts
import client from './client';

export interface HealthResponse {
  status: string;
}

export async function fetchHealth(): Promise<HealthResponse> {
  const { data } = await client.get<HealthResponse>('/health');
  return data;
}

export async function isServerHealthy(): Promise<boolean> {
  try {
    const health = await fetchHealth();
    return health.status === 'ok';
  } catch {
    return false;
  }
}
```

- [ ] **Step 2: Typecheck**

Run:

```powershell
cd code/frontend
npm run build:desktop
```

Expected: build succeeds and `systemApi.ts` has no TypeScript errors.

- [ ] **Step 3: Commit**

Run:

```powershell
git add code/frontend/src/api/systemApi.ts
git commit -m "feat: add frontend system health api"
```

---

### Task 3: Build the Desktop Launcher Components

**Files:**
- Create: `code/frontend/src/components/launcher/LauncherPage.tsx`
- Create: `code/frontend/src/components/launcher/LauncherShell.tsx`
- Create: `code/frontend/src/components/launcher/LauncherActions.tsx`
- Create: `code/frontend/src/components/launcher/LauncherDeckPanel.tsx`
- Create: `code/frontend/src/components/launcher/LauncherNewsPanel.tsx`
- Create: `code/frontend/src/components/launcher/launcher.css`

- [ ] **Step 1: Create the launcher page container**

Create `code/frontend/src/components/launcher/LauncherPage.tsx`:

```tsx
import { useEffect, useMemo, useState } from 'react';
import { fetchPatchNotes, type PatchNotesResponse } from '@/api/patchNotesApi';
import { listTestedCards } from '@/api/deckApi';
import { isServerHealthy } from '@/api/systemApi';
import { useAuthStore } from '@/stores/authStore';
import * as deckStore from '@/storage/deckStore';
import type { DeckSummary } from '@/types/deck';
import { LauncherActions } from './LauncherActions';
import { LauncherDeckPanel } from './LauncherDeckPanel';
import { LauncherNewsPanel } from './LauncherNewsPanel';
import { LauncherShell } from './LauncherShell';
import {
  buildDeckRows,
  countDraftDecks,
  formatCardCount,
  summarizeLatestRelease,
} from './launcherData';
import './launcher.css';

interface LauncherState {
  decks: DeckSummary[];
  patchNotes: PatchNotesResponse | null;
  testedCardCount: number | null;
  serverHealthy: boolean;
  loaded: boolean;
}

const initialState: LauncherState = {
  decks: [],
  patchNotes: null,
  testedCardCount: null,
  serverHealthy: false,
  loaded: false,
};

export function LauncherPage() {
  const user = useAuthStore((state) => state.user);
  const [state, setState] = useState<LauncherState>(initialState);

  useEffect(() => {
    let active = true;
    async function load() {
      const [decksResult, patchResult, cardsResult, healthResult] = await Promise.allSettled([
        deckStore.listDecks(),
        fetchPatchNotes(),
        listTestedCards(),
        isServerHealthy(),
      ]);
      if (!active) return;
      setState({
        decks: decksResult.status === 'fulfilled' ? decksResult.value : [],
        patchNotes: patchResult.status === 'fulfilled' ? patchResult.value : null,
        testedCardCount: cardsResult.status === 'fulfilled' ? cardsResult.value.length : null,
        serverHealthy: healthResult.status === 'fulfilled' ? healthResult.value : false,
        loaded: true,
      });
    }
    void load();
    return () => {
      active = false;
    };
  }, []);

  const deckRows = useMemo(() => buildDeckRows(state.decks), [state.decks]);
  const releaseSummary = useMemo(
    () => summarizeLatestRelease(state.patchNotes),
    [state.patchNotes],
  );

  return (
    <LauncherShell
      buildVersion={import.meta.env.VITE_APP_VERSION ?? '0.1.0'}
      cardCountLabel={formatCardCount(state.testedCardCount)}
      deckCount={state.decks.length}
      draftCount={countDraftDecks(state.decks)}
      serverHealthy={state.serverHealthy}
      username={user?.username ?? 'Guest'}
    >
      <section className="launcher-hero" aria-labelledby="launcher-heading">
        <div className="launcher-welcome">// WELCOME BACK, {(user?.username ?? 'GUEST').toUpperCase()}</div>
        <h1 id="launcher-heading" className="launcher-title">
          PICK UP<br />WHERE YOU<br /><em>LEFT OFF.</em>
        </h1>
        <div className="launcher-tagline">PLAY ANONYMOUSLY · NO ACCOUNT REQUIRED</div>
        <LauncherActions hasDecks={state.decks.length > 0} />
      </section>
      <aside className="launcher-right-column" aria-label="Launcher details">
        <LauncherDeckPanel decks={deckRows} loaded={state.loaded} />
        <LauncherNewsPanel release={releaseSummary} />
      </aside>
    </LauncherShell>
  );
}
```

- [ ] **Step 2: Create the shell**

Create `code/frontend/src/components/launcher/LauncherShell.tsx`:

```tsx
import { Link } from 'react-router-dom';
import type { ReactNode } from 'react';

interface LauncherShellProps {
  buildVersion: string;
  cardCountLabel: string;
  deckCount: number;
  draftCount: number;
  serverHealthy: boolean;
  username: string;
  children: ReactNode;
}

export function LauncherShell({
  buildVersion,
  cardCountLabel,
  deckCount,
  draftCount,
  serverHealthy,
  username,
  children,
}: LauncherShellProps) {
  return (
    <div className="launcher-screen">
      <div className="launcher-frame">
        <div className="launcher-titlebar">
          <div className="launcher-dots" aria-hidden="true">
            <span className="launcher-dot launcher-dot-red" />
            <span className="launcher-dot launcher-dot-yellow" />
            <span className="launcher-dot launcher-dot-green" />
          </div>
          <div className="launcher-window-title">DIGIMON TCG DESKTOP · v{buildVersion}</div>
          <div className="launcher-titlebar-right">
            <span className={serverHealthy ? 'launcher-pill-live' : 'launcher-pill-offline'}>
              {serverHealthy ? 'CONNECTED' : 'OFFLINE'}
            </span>
          </div>
        </div>
        <div className="launcher-body-area">
          <aside className="launcher-side">
            <Link className="launcher-brand" to="/">
              <span className="launcher-brand-mark" aria-hidden="true">D</span>
              <span className="launcher-brand-name">DIGIMON<small>TCG</small></span>
            </Link>
            <nav className="launcher-side-section" aria-label="Main">
              <h5>Main</h5>
              <Link className="launcher-nav-item active" to="/">Home</Link>
              <Link className="launcher-nav-item" to="/lobby">Play</Link>
              <Link className="launcher-nav-item" to="/deckbuilder">Decks <span>{String(deckCount).padStart(2, '0')}</span></Link>
              <Link className="launcher-nav-item" to="/patch-notes">Patch Notes</Link>
            </nav>
            <nav className="launcher-side-section" aria-label="Tools">
              <h5>Tools</h5>
              <Link className="launcher-nav-item" to="/models">AI Models</Link>
              <Link className="launcher-nav-item" to="/deckbuilder?import=1">Import</Link>
              <Link className="launcher-nav-item" to="/game">Sandbox</Link>
            </nav>
            <div className="launcher-side-foot">
              <div><span>BUILD</span><b>{buildVersion}</b></div>
              <div><span>CARDS</span><b>{cardCountLabel}</b></div>
              <div><span>DRAFTS</span><b>{String(draftCount).padStart(2, '0')}</b></div>
            </div>
          </aside>
          <main className="launcher-main">
            <div className="launcher-topbar">
              <div><span>00</span><b> Launcher</b></div>
              <div className="launcher-user">Signed in as {username}</div>
            </div>
            <div className="launcher-content">{children}</div>
            <div className="launcher-statusbar">
              <span className={serverHealthy ? 'launcher-ok' : 'launcher-warn'}>
                {serverHealthy ? 'SERVER OK' : 'SERVER OFFLINE'}
              </span>
              <span>DESKTOP</span>
            </div>
          </main>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Create action tiles**

Create `code/frontend/src/components/launcher/LauncherActions.tsx`:

```tsx
import { Link } from 'react-router-dom';

interface LauncherActionsProps {
  hasDecks: boolean;
}

export function LauncherActions({ hasDecks }: LauncherActionsProps) {
  return (
    <>
      <div className="launcher-actions">
        <Link className="launcher-action primary" to="/lobby">
          <span className="launcher-action-label">// PRIMARY ACTION</span>
          <span className="launcher-action-name">PLAY</span>
          <span className="launcher-action-desc">Quick match, room, or open queue.</span>
          <span className="launcher-action-meta">ENTER</span>
        </Link>
        <Link className="launcher-action" to={hasDecks ? '/deckbuilder' : '/deckbuilder?import=1'}>
          <span className="launcher-action-label">// LIBRARY</span>
          <span className="launcher-action-name">MY DECKS</span>
          <span className="launcher-action-desc">Saved locally for desktop play.</span>
          <span className="launcher-action-meta">D</span>
        </Link>
      </div>
      <div className="launcher-quick">
        <Link to="/patch-notes"><span>// Updates</span><b>Patch notes</b></Link>
        <Link to="/models"><span>// Practice</span><b>Play vs AI</b></Link>
        <Link to="/deckbuilder"><span>// Build</span><b>Deck builder</b></Link>
      </div>
    </>
  );
}
```

- [ ] **Step 4: Create deck and news panels**

Create `code/frontend/src/components/launcher/LauncherDeckPanel.tsx`:

```tsx
import { Link } from 'react-router-dom';
import type { LauncherDeckRow } from './launcherData';

interface LauncherDeckPanelProps {
  decks: LauncherDeckRow[];
  loaded: boolean;
}

export function LauncherDeckPanel({ decks, loaded }: LauncherDeckPanelProps) {
  return (
    <section className="launcher-deck-panel" aria-labelledby="launcher-decks-heading">
      <div className="launcher-panel-head">
        <h2 id="launcher-decks-heading">Saved Decks</h2>
        <span>{String(decks.length).padStart(2, '0')} RECENT</span>
      </div>
      <div className="launcher-deck-list">
        {decks.map((deck) => (
          <Link className="launcher-deck-row" to={deck.href} key={deck.id}>
            <div className="launcher-deck-color"><span>{deck.levelLabel}</span></div>
            <div className="launcher-deck-info">
              <div className="launcher-deck-name">{deck.name}</div>
              <div className="launcher-deck-meta">
                <span className={`launcher-${deck.statusKind}`}>● {deck.statusLabel}</span>
                <span>{deck.countLabel}</span>
                <span>{deck.editedLabel}</span>
              </div>
            </div>
            <div className="launcher-deck-stat">{deck.metaLabel}</div>
          </Link>
        ))}
        {loaded && decks.length === 0 ? (
          <div className="launcher-empty">No saved decks yet.</div>
        ) : null}
      </div>
      <div className="launcher-panel-foot">
        <Link className="launcher-btn accent" to="/deckbuilder">+ New Deck</Link>
        <Link className="launcher-btn" to="/deckbuilder?import=1">Import</Link>
        <Link className="launcher-btn" to="/deckbuilder">View All →</Link>
      </div>
    </section>
  );
}
```

Create `code/frontend/src/components/launcher/LauncherNewsPanel.tsx`:

```tsx
import { Link } from 'react-router-dom';
import type { LauncherReleaseSummary } from './launcherData';

interface LauncherNewsPanelProps {
  release: LauncherReleaseSummary;
}

export function LauncherNewsPanel({ release }: LauncherNewsPanelProps) {
  return (
    <section className="launcher-news" aria-labelledby="launcher-news-heading">
      <div className="launcher-panel-head">
        <h2 id="launcher-news-heading">{release.title}</h2>
        <span>{release.versionLabel}</span>
      </div>
      <ul>
        {release.bullets.map((bullet) => (
          <li key={bullet}>{bullet}</li>
        ))}
      </ul>
      <Link to="/patch-notes">View patch notes →</Link>
    </section>
  );
}
```

- [ ] **Step 5: Port the CSS**

Create `code/frontend/src/components/launcher/launcher.css` by porting the `Launcher.html` style block from the zip with these exact mechanical changes:

```text
.frame                  -> .launcher-frame
.titlebar               -> .launcher-titlebar
.dots                   -> .launcher-dots
.dot                    -> .launcher-dot
.dot.r                  -> .launcher-dot-red
.dot.y                  -> .launcher-dot-yellow
.dot.g                  -> .launcher-dot-green
.body-area              -> .launcher-body-area
.side                   -> .launcher-side
.brand                  -> .launcher-brand
.brand-mark             -> .launcher-brand-mark
.brand-name             -> .launcher-brand-name
.side-section           -> .launcher-side-section
.nav-item               -> .launcher-nav-item
.side-foot              -> .launcher-side-foot
.main                   -> .launcher-main
.topbar                 -> .launcher-topbar
.content                -> .launcher-content
.welcome                -> .launcher-welcome
.title                  -> .launcher-title
.tagline                -> .launcher-tagline
.actions                -> .launcher-actions
.action                 -> .launcher-action
.quick                  -> .launcher-quick
.deck-panel             -> .launcher-deck-panel
.deck-panel-head        -> .launcher-panel-head
.deck-list              -> .launcher-deck-list
.deck-row               -> .launcher-deck-row
.deck-color             -> .launcher-deck-color
.deck-info              -> .launcher-deck-info
.deck-info .name        -> .launcher-deck-name
.deck-info .meta        -> .launcher-deck-meta
.legal                  -> .launcher-legal
.draft                  -> .launcher-draft
.stat                   -> .launcher-deck-stat
.deck-panel-foot        -> .launcher-panel-foot
.btn                    -> .launcher-btn
.btn-accent             -> .launcher-btn.accent
.news                   -> .launcher-news
.statusbar              -> .launcher-statusbar
.ok                     -> .launcher-ok
```

Keep the color tokens from `Launcher.html`, but scope them under `.launcher-screen` instead of `:root`. Add these responsive adjustments at the end so the page works at Tauri minimum size and on narrow browser previews:

```css
.launcher-screen {
  min-height: 100vh;
  background: #000;
  color: var(--ink-1);
  font-family: var(--body);
  display: grid;
  place-items: center;
  padding: 24px;
}

.launcher-content {
  grid-template-columns: minmax(0, 1fr) minmax(320px, 430px);
  gap: 24px;
}

.launcher-right-column {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 14px;
}

@media (max-width: 980px) {
  .launcher-screen {
    padding: 0;
  }

  .launcher-frame {
    width: 100%;
    min-height: 100vh;
    height: auto;
    border-radius: 0;
  }

  .launcher-body-area {
    grid-template-columns: 180px 1fr;
  }

  .launcher-content {
    grid-template-columns: 1fr;
    overflow: auto;
  }

  .launcher-title {
    font-size: 64px;
  }
}
```

- [ ] **Step 6: Build**

Run:

```powershell
cd code/frontend
npm run build:desktop
```

Expected: desktop build succeeds.

- [ ] **Step 7: Commit**

Run:

```powershell
git add code/frontend/src/components/launcher
git commit -m "feat: add desktop launcher screen"
```

---

### Task 4: Mount the Desktop Launcher Outside the Normal Layout

**Files:**
- Modify: `code/frontend/src/App.tsx`

- [ ] **Step 1: Update routes**

Modify `code/frontend/src/App.tsx` so `LauncherPage` is lazy-loaded and desktop `/` renders outside `Layout`:

```tsx
const LauncherPage = lazy(() => import('@/components/launcher/LauncherPage').then(m => ({ default: m.LauncherPage })));
```

Then replace the top-level `<Routes>` body with:

```tsx
<Routes>
  {IS_DESKTOP && <Route path="/" element={suspended(LauncherPage)} />}
  <Route element={<Layout />}>
    {!IS_DESKTOP && <Route path="/" element={<HomePage />} />}
    <Route path="/patch-notes" element={<PatchNotesPage />} />
    <Route path="/login" element={<LoginPage />} />
    <Route path="/register" element={<RegisterPage />} />
    <Route element={<AuthGuard />}>
      <Route path="/lobby" element={<LobbyPage />} />
      <Route path="/game/:id?" element={<GamePage />} />
      <Route path="/deckbuilder/:id?" element={<DeckBuilderPage />} />
    </Route>
    {!IS_DESKTOP && (
      <Route element={<RoleGuard allowedRoles={['admin']} />}>
        <Route path="/admin/issues" element={suspended(AdminIssuesPage)} />
        <Route path="/admin/tasks" element={suspended(AdminTasksPage)} />
        <Route path="/admin/promotions" element={suspended(AdminPromotionsPage)} />
        <Route path="/admin/barracks" element={suspended(BarracksPage)} />
        <Route path="/admin/arena" element={suspended(ArenaPage)} />
        <Route path="/admin/gauntlet" element={suspended(GauntletPage)} />
        <Route path="/admin/gauntlet/:id" element={suspended(GauntletPage)} />
        <Route path="/admin/deck-pools" element={suspended(DeckPoolPage)} />
        <Route path="/admin/deck-pools/:id" element={suspended(DeckPoolPage)} />
        <Route path="/admin/patch-notes" element={suspended(AdminPatchNotesPage)} />
        <Route path="/admin/models" element={suspended(AdminModelsPage)} />
      </Route>
    )}
    {IS_DESKTOP && <Route path="/models" element={suspended(ModelsPage)} />}
  </Route>
</Routes>
```

- [ ] **Step 2: Build both targets**

Run:

```powershell
cd code/frontend
npm run build
npm run build:desktop
```

Expected: both builds succeed. Web build still renders `HomePage` at `/`; desktop build renders `LauncherPage` at `/`.

- [ ] **Step 3: Commit**

Run:

```powershell
git add code/frontend/src/App.tsx
git commit -m "feat: route desktop home to launcher"
```

---

### Task 5: Make Launcher Deck Links Land on Real Deck Builder Workflows

**Files:**
- Modify: `code/frontend/src/pages/DeckBuilderPage.tsx`

- [ ] **Step 1: Add route/query handling**

In `code/frontend/src/pages/DeckBuilderPage.tsx`, import router hooks and card lookup:

```tsx
import { useEffect, useState } from 'react';
import { useLocation, useParams } from 'react-router-dom';
import { getCardById } from '@/api/digimonCardApi';
```

Add this helper above `DeckBuilderPage`:

```tsx
function groupCardIds(ids: string[], altArts: boolean[] = []) {
  const counts = new Map<string, { cardId: string; isAltArt: boolean; count: number }>();
  ids.forEach((cardId, i) => {
    const isAltArt = !!altArts[i];
    const key = `${cardId}|${isAltArt ? '1' : '0'}`;
    const existing = counts.get(key);
    if (existing) {
      existing.count += 1;
    } else {
      counts.set(key, { cardId, isAltArt, count: 1 });
    }
  });
  return Array.from(counts.values());
}
```

Inside `DeckBuilderPage`, read route state:

```tsx
const { id: routeDeckId } = useParams();
const location = useLocation();
```

Add this effect after the tested-card effect:

```tsx
useEffect(() => {
  if (new URLSearchParams(location.search).get('import') === '1') {
    setShowImport(true);
  }
}, [location.search]);

useEffect(() => {
  if (!routeDeckId || routeDeckId === deckId) return;
  let active = true;
  async function loadRouteDeck() {
    const decks = IS_DESKTOP ? deckStore : deckApi;
    const deck = await decks.getDeck(routeDeckId);
    const mainEntries = groupCardIds(deck.main_deck, deck.main_deck_alt_arts);
    const eggEntries = groupCardIds(deck.egg_deck, deck.egg_deck_alt_arts);
    const allIds = [...new Set([...deck.main_deck, ...deck.egg_deck])];
    const cardData = await Promise.allSettled(allIds.map((cardId) => getCardById(cardId)));
    const cardDataMap = new Map(
      cardData
        .map((result) => (result.status === 'fulfilled' && result.value ? [result.value.id, result.value] : null))
        .filter((entry): entry is [string, NonNullable<(typeof cardData)[number] extends PromiseFulfilledResult<infer T> ? T : never>] => !!entry),
    );
    for (const entry of [...mainEntries, ...eggEntries]) {
      const data = cardDataMap.get(entry.cardId);
      if (data) entry.cardData = data;
    }
    if (active) loadDeck(deck.id, deck.name, mainEntries, eggEntries);
  }
  loadRouteDeck().catch(() => {});
  return () => {
    active = false;
  };
}, [deckId, loadDeck, routeDeckId]);
```

- [ ] **Step 2: Typecheck**

Run:

```powershell
cd code/frontend
npm run build:desktop
```

Expected: build succeeds and `/deckbuilder/:id` loads the selected deck.

- [ ] **Step 3: Commit**

Run:

```powershell
git add code/frontend/src/pages/DeckBuilderPage.tsx
git commit -m "feat: load deck builder routes from launcher links"
```

---

### Task 6: Add Desktop Launcher E2E Coverage

**Files:**
- Create: `code/frontend/e2e/launcher.spec.ts`
- Modify: `code/frontend/e2e/guest-onboarding.spec.ts`

- [ ] **Step 1: Write the launcher e2e test**

Create `code/frontend/e2e/launcher.spec.ts`:

```ts
import { test, expect } from '@playwright/test';

test.describe('Desktop launcher', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/api/health', async (route) => {
      await route.fulfill({ json: { status: 'ok' } });
    });
    await page.route('**/api/patch-notes', async (route) => {
      await route.fulfill({
        json: {
          known_issues: [],
          releases: [
            {
              id: 'release-1',
              version: '0.4.2',
              release_date: '2026-04-24',
              title: 'Launcher polish',
              added: ['Desktop launcher'],
              changed: ['Guest boot flow'],
              fixed: [],
              created_at: '2026-04-24T00:00:00.000Z',
              updated_at: '2026-04-24T00:00:00.000Z',
            },
          ],
        },
      });
    });
    await page.route('**/api/decks/tested-cards', async (route) => {
      await route.fulfill({ json: { card_ids: ['BT1-001', 'BT1-002'], card_count: 2 } });
    });
    await page.addInitScript(() => {
      localStorage.setItem('access_token', 'guest-token');
      localStorage.setItem('guest_access_token', 'guest-token');
      localStorage.setItem('guest_user_id', 'guest_abc');
      localStorage.setItem('guest_display_name', 'Guest-ABCD');
    });
  });

  test('renders live server state and launcher actions', async ({ page }) => {
    await page.goto('/');

    await expect(page.getByRole('heading', { name: /PICK UP WHERE YOU LEFT OFF/i })).toBeVisible();
    await expect(page.getByText('CONNECTED')).toBeVisible();
    await expect(page.getByRole('link', { name: /PLAY/i })).toBeVisible();
    await expect(page.getByRole('link', { name: /MY DECKS/i })).toBeVisible();
    await expect(page.getByText('Launcher polish')).toBeVisible();
  });

  test('navigates launcher actions into existing app routes', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: /Deck builder/i }).click();
    await expect(page).toHaveURL(/\/deckbuilder/);
  });
});
```

- [ ] **Step 2: Update guest onboarding expectation**

In `code/frontend/e2e/guest-onboarding.spec.ts`, replace:

```ts
await expect(
  page.getByRole('heading', { name: 'Digimon TCG Simulator' }),
).toBeVisible();
```

with:

```ts
await expect(
  page.getByRole('heading', { name: /PICK UP WHERE YOU LEFT OFF/i }),
).toBeVisible();
```

- [ ] **Step 3: Run desktop-mode e2e**

Start Vite in desktop mode on the Playwright port:

```powershell
cd code/frontend
npm run dev:desktop -- --port 5174
```

In a second shell:

```powershell
cd code/frontend
npm run e2e -- launcher.spec.ts guest-onboarding.spec.ts
```

Expected: both specs pass.

- [ ] **Step 4: Commit**

Run:

```powershell
git add code/frontend/e2e/launcher.spec.ts code/frontend/e2e/guest-onboarding.spec.ts
git commit -m "test: cover desktop launcher flow"
```

---

### Task 7: Visual Verification and Polish

**Files:**
- Modify as needed: `code/frontend/src/components/launcher/launcher.css`
- Modify as needed: `code/frontend/src/components/launcher/*.tsx`

- [ ] **Step 1: Start the desktop dev server**

Run:

```powershell
cd code/frontend
npm run dev:desktop -- --port 5174
```

Expected: Vite serves the desktop build at `http://localhost:5174`.

- [ ] **Step 2: Capture desktop screenshot**

Run:

```powershell
cd code/frontend
npx playwright screenshot --viewport-size=1280,800 http://localhost:5174 test-results/launcher-1280x800.png
```

Expected: screenshot shows the launcher frame, sidebar, primary action tiles, deck panel, news panel, and status bar with no overlapping text.

- [ ] **Step 3: Capture minimum-window screenshot**

Run:

```powershell
cd code/frontend
npx playwright screenshot --viewport-size=1024,768 http://localhost:5174 test-results/launcher-1024x768.png
```

Expected: screenshot remains usable at the Tauri minimum window size from `code/src-tauri/tauri.conf.json`.

- [ ] **Step 4: Run final verification**

Run:

```powershell
cd code/frontend
npm test -- src/components/launcher/launcherData.test.ts
npm run build
npm run build:desktop
npm run e2e -- launcher.spec.ts guest-onboarding.spec.ts
```

Expected: all commands pass.

- [ ] **Step 5: Commit**

Run:

```powershell
git add code/frontend/src/components/launcher code/frontend/test-results
git commit -m "style: polish desktop launcher layout"
```

---

## Self-Review

- Spec coverage: The plan imports the new launcher design, replaces the desktop home screen, connects deck counts and rows to local desktop deck storage, connects server status to `/health`, connects news to `/patch-notes`, connects card count to tested-card data, and keeps existing routes for game, deck builder, models, and patch notes.
- Placeholder scan: The plan avoids open-ended implementation placeholders. The CSS task uses a deterministic class rename table against the supplied `Launcher.html` source.
- Type consistency: `LauncherDeckRow`, `LauncherReleaseSummary`, `DeckSummary`, and `PatchNotesResponse` names match their defining files and are used consistently across tasks.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-26-desktop-launcher-design.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?

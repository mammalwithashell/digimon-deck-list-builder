# frontend

React 18 + TypeScript + Vite app. Zustand state management. Drives both the hosted-API web client and the [Tauri desktop shell](../src-tauri/).

## Layout

- `src/pages/` — `GamePage`, `LobbyPage`, `DeckBuilderPage`, `Admin*`
- `src/components/board/` — `GameBoard`, `HandZone`, `BattleArea`, `MemoryGauge`
- `src/components/game/` — `ActionBar`, overlays, selection UI
- `src/api/` — REST + WebSocket clients
- `src/App.tsx` — route map

## Commands

```bash
cd code/frontend
npm install
npm run dev          # dev server
npm run build        # web build
npm run test         # vitest
npm run e2e          # playwright (see playwright.config.ts)
```

## Build targets

The desktop build tree-shakes admin/training UI via `VITE_BUILD_TARGET=desktop` (working rule 13). The web build includes everything.

## Working rules that touch the frontend

- Tensor and action specs stay in sync with engine constants and frontend constants (rule 1)
- UI **reflects** state, never owns rules (rule 2)
- Animation components (`DigivolveBanner`, `BattleEffect`) subscribe to `store.events` and track `lastSeqRef` to avoid replaying stale events (rule 15)
- Action masking must not be bypassed in agent / UI logic (rule 3)

## State leak prevention

Opponent-visible WebSocket payloads go through [`server/state_filter.py`](../server/state_filter.py); the frontend never receives raw `to_ui_json()` (working rules 9, 14). If you find yourself reading `handIds` or `handCards` from an opponent payload, that's a bug — not a feature to consume.

# UI Plan

This document now separates what is currently implemented from future roadmap work.

## Current Implemented UI/API Surface

### Frontend Routes

Defined in `code/frontend/src/App.tsx`:

- Public:
  - `/`
  - `/login`
  - `/register`
- Auth-guarded:
  - `/game/:id?`
  - `/deckbuilder/:id?`
- Admin role-guarded:
  - `/admin/issues`
  - `/admin/tasks`
  - `/admin/promotions`

### Implemented UI Areas

1. Gameplay UI
- Board and zone components under `code/frontend/src/components/board/`
- Gameplay controls/logging under `code/frontend/src/components/game/`
- Session orchestration in `code/frontend/src/pages/GamePage.tsx`
- Action decoding helpers in `code/frontend/src/utils/actionDecoder.ts`

2. Deck Builder UI
- Search/filter/grid/list panels in `code/frontend/src/components/deckbuilder/`
- Page orchestration in `code/frontend/src/pages/DeckBuilderPage.tsx`
- Store in `code/frontend/src/stores/deckBuilderStore.ts`

3. Auth and Layout
- `LoginPage`, `RegisterPage`
- `AuthGuard` and `RoleGuard`
- App layout/nav components

4. Admin AI UI
- `AdminIssuesPage`
- `AdminTasksPage`
- `AdminPromotionsPage`
- API client: `code/frontend/src/api/adminApi.ts`

### Implemented Backend Endpoints Used by UI

- Gameplay:
  - `POST /games`
  - `POST /games/{game_id}/actions`
  - `POST /games/{game_id}/steps`
  - `GET /games/{game_id}/state`
  - `GET /games/{game_id}/action-mask`
  - `GET /games/{game_id}/logs`
  - `DELETE /games/{game_id}`
- Deck tools:
  - `POST /decks/parse`
  - `POST /decks/validate`
- Deck/auth/user/admin DB routes via mounted routers:
  - `/auth/*`, `/users/*`, `/decks/*`, `/friends/*`, `/issues/*`, `/admin/*`

### Current Data Contracts to Keep Synced

- Action constants and phase labels:
  - Backend: `code/engine_py_legacy/engine/game.py`, `code/engine_py_legacy/engine/data/enums.py`
  - Frontend: `code/frontend/src/utils/constants.ts`, `code/frontend/src/types/game.ts`
- Spec docs:
  - `ACTION_SPEC.md`
  - `TENSOR_SPEC.md`

## Roadmap / Not Yet Implemented

1. Lobby and matchmaking layer
- Room discovery/invites/presence and richer multiplayer UX.

2. Real-time transport expansion
- Optional websocket/event-stream channel for lower-latency game updates.

3. Replay viewer enhancements
- Timeline controls, diff views, and richer metadata visualization.

4. Advanced visual polish
- More combat/transition animations and deeper interaction affordances.

5. Expanded admin observability
- Batch/task dashboards with richer trend and cost breakdowns.

## Change Policy

When gameplay phase/action behavior changes:

1. Update backend constants and decoder/mask logic.
2. Update frontend constants/types and interaction handling.
3. Update `ACTION_SPEC.md` and `TENSOR_SPEC.md` in the same change.
4. Validate with relevant tests (`test_tensor_and_actions`, `test_phase_decoders`).

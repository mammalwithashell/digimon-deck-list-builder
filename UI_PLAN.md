# UI Plan

This document now separates what is currently implemented from future roadmap work.

## Current Implemented UI/API Surface

### Frontend Routes

Defined in `frontend/src/App.tsx`:

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
- Board and zone components under `frontend/src/components/board/`
- Gameplay controls/logging under `frontend/src/components/game/`
- Session orchestration in `frontend/src/pages/GamePage.tsx`
- Action decoding helpers in `frontend/src/utils/actionDecoder.ts`

2. Deck Builder UI
- Search/filter/grid/list panels in `frontend/src/components/deckbuilder/`
- Page orchestration in `frontend/src/pages/DeckBuilderPage.tsx`
- Store in `frontend/src/stores/deckBuilderStore.ts`

3. Auth and Layout
- `LoginPage`, `RegisterPage`
- `AuthGuard` and `RoleGuard`
- App layout/nav components

4. Admin AI UI
- `AdminIssuesPage`
- `AdminTasksPage`
- `AdminPromotionsPage`
- API client: `frontend/src/api/adminApi.ts`

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
  - Backend: `digimon_gym/engine/game.py`, `digimon_gym/engine/data/enums.py`
  - Frontend: `frontend/src/utils/constants.ts`, `frontend/src/types/game.ts`
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

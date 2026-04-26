# server

Hosted FastAPI app. PvP WebSockets, lobby, auth, user data, recordings, admin AI, model manifest.

## Surface

- `api.py` — app assembly + router registration
- `env.py` — env-var resolution
- `digilab_client.py` — DigiLab DB client
- `routers/` — FastAPI routers (`games`, `lobby`, `ws`, `replays`, `deck_tools`, …)
- `db/` — SQLAlchemy models, auth, DB-backed routers
- `ai/` — admin AI pipeline (hosted API only; not used by desktop or training CLI)
- `classifier/` — issue / task classifier
- `storage/` — object / file storage adapters
- `workers/` — `training_worker.py`, `gauntlet_orchestrator.py` (DB-coupled training pieces; the standalone CLI lives in [`digimon_gym/agents/`](../digimon_gym/agents/))

## Service boundaries (working rule 11)

Engine-only routers — `games.py`, `replays.py`, `simulations.py`, `deck_tools.py` — must **not** import from `digimon_gym.db.*` or `digimon_gym.ai.*`. They mirror Tauri commands the desktop app uses, so any DB pull-in breaks the desktop / training boundary.

## Run

```bash
# Development
python -m uvicorn server.api:app --reload --reload-dir code/server

# Production / long-running
python -m uvicorn server.api:app --host 0.0.0.0 --port 8000
```

**Do not use `--reload` for long-running tasks** — it spawns watcher+worker pairs that don't get killed cleanly.

## Tests

```bash
python -m pytest code/tests/api -v
python -m pytest code/tests/storage -v
python -m pytest code/tests/classifier -v
python -m pytest code/tests/ai_pipeline -v   # opt-in, not in default run
```

## Migrations

DB migrations live at the repo root: [`alembic/`](../../alembic/), [`alembic.ini`](../../alembic.ini).

## State broadcasting

WebSocket state broadcasts must use `state_filter.py` — never send raw `to_ui_json()` to network clients (working rule 9). `handIds` and `handCards` must both be redacted for opponents (rule 14).

## Surrender event ordering

`Game.surrender()` emits the `surrender` event **before** calling `declare_winner()` so event listeners see surrender before game_over (working rule 16).

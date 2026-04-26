# tests

Default pytest tree. `pyproject.toml` sets `testpaths = ["code/tests"]`, so a bare `python -m pytest` collects from here.

Engine / behavioral / runner tests for the **sunset Python engine** live under [`engine_py_legacy/tests/`](../engine_py_legacy/tests/) and are excluded from default collection.

## Layout

- `api/` — hosted API tests (DB, auth, routers)
- `classifier/` — issue / task classifier tests
- `storage/` — storage-adapter tests
- `rl/` — RL training tests (gauntlet, LSTM, workers)
- `ai_pipeline/` — admin AI pipeline tests (opt-in, not in default run)
- `tools/` — CLI tool tests
- top-level files — `test_decklist_analysis.py`, `test_rust_bindings_surface.py`, `test_store_night.py`
- `e2e_smoke.mjs` — Node-driven smoke test against a running server

## Commands

```bash
python -m pytest -v                                  # default (excludes ai_pipeline by config)
python -m pytest code/tests/api -v
python -m pytest code/tests/rl -v
python -m pytest code/tests/ai_pipeline -v           # opt-in
python -m pytest -m "not slow" -v                    # skip slow smoke tests
```

## Sunset engine tests

```bash
python -m pytest code/engine_py_legacy/tests -v      # not collected by default
```

## Rust-backend parity

The parity test currently lives under the sunset tree:

```bash
DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v
```

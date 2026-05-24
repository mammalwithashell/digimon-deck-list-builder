## 1. Writer module

- [x] 1.1 Add `code/digimon_gym/agents/game_log.py` with a
      `GameLogWriter` class (append-only `"a"` open, flush after every
      row, `OSError` → disable + log-once-to-stderr + silent no-op).
      Single output path passed in at construction; no env-index, no
      header record.
- [x] 1.2 Unit tests in `code/tests/rl/test_game_log.py`:
      - `test_writer_appends_not_truncates` against an existing file.
      - `test_writer_flushes_each_row`.
      - `test_writer_disables_on_oserror_and_logs_once`.

## 2. Recording-path audit

- [x] 2.1 Audit `code/digimon_gym/agents/training_recording.py` for a
      stable attribute exposing the most-recently-written recording's
      path. Added `last_recording_path: Optional[Path]` to
      `TrainingRecordingWrapper`, updated in `step()` from
      `recorder.write(...)` return value (happy path + panic-recovery
      path both set it).
- [x] 2.2 Add `_find_eval_recording_path(eval_env)` helper in
      `pilot_training.py` that walks the env chain looking for a
      `TrainingRecordingWrapper`, returning `last_recording_path` or
      `None`.

## 3. WinRateCallback row emission

- [x] 3.1 Extend `WinRateCallback.__init__` to accept
      `game_log_writer: GameLogWriter | None = None` and initialize
      `self._eval_window_idx = 0`.
- [x] 3.2 Capture `step = int(self.num_timesteps)` and
      `window_idx = self._eval_window_idx` once at loop entry, then
      increment.
- [x] 3.3 Inside the per-game loop, after `terminal_score` and digivolve
      counts are computed, build a row via `_build_game_log_row(...)`
      and call `self._game_log_writer.append(row)` (no-op if None).
- [x] 3.4 Existing means, TB scalars, `evals.jsonl` writes, and
      per-archetype tallies are unaltered (verified by full
      `code/tests/rl` suite passing minus 2 pre-existing failures on
      base).

## 4. WinRateCallback unit + integration tests

- [x] 4.1 Row-builder unit tests in `test_game_log.py` cover win/loss/draw
      mapping, archetype passthrough, digivolve-count plumbing, path
      absolutization, null when no recording wrapper, populated when
      present. End-to-end integration in
      `test_pilot_training_e2e_game_log.py` (marked `@pytest.mark.slow`)
      drives `train()` for 2000 steps with `eval_freq=500` and asserts
      JSONL rows appear with the documented schema; companion test
      verifies `--eval-game-log off` writes no file.

## 5. Pilot-training wiring + CLI

- [x] 5.1 Add `--eval-game-log {on,off}` CLI flag (default `on`) and
      `eval_game_log: str = "on"` field on `TrainingConfig` with
      validation against `VALID_EVAL_GAME_LOG_MODES`.
- [x] 5.2 In `train()`, when `cfg.eval_game_log != "off"`, construct one
      `GameLogWriter(run_dir / "eval_game_log.jsonl")` and pass as
      `game_log_writer=...` to `WinRateCallback`.
- [x] 5.3 Print a one-line `Eval game log: on -> <path>` / `off` at
      startup, mirroring the mulligan-log announcement.

## 6. MCP read surface

- [x] 6.1 Add `code/digimon-training-mcp/src/digimon_training_mcp/per_game.py`
      with path resolution (top-level then most-recently-modified
      timestamped subdir), `_read_rows` (tolerant of partial last
      line), `_passes_filter` (AND-style), `_sort_key`, and
      `run_per_game_evals(...)` orchestration. Also exports
      `has_eval_game_log` for use by `list_runs`.
- [x] 6.2 Register `run_per_game_evals` in
      `code/digimon-training-mcp/src/digimon_training_mcp/server.py`
      with the documented JSON schema and an async handler
      `_h_run_per_game_evals`.
- [x] 6.3 Thread `models_dir` into `list_runs(...)` and add
      `has_eval_game_log` to each entry via the cheap existence check.
- [x] 6.4 MCP tests in `code/digimon-training-mcp/tests/test_per_game.py`
      (12 tests): sort order, AND-filter combinations, whale-game
      `digivolves_agent_min` filter, limit post-filter,
      missing-file/unknown-run → empty list not error,
      timestamped-subdir + flat-layout resolution, `has_eval_game_log`
      flag in `list_runs`, recording-path round-trip, partial-last-line
      tolerance, step-range filter. Also updated
      `test_server_bootstrap.py` and `test_integration.py` to reflect
      8 (not 7) registered tools.

## 7. Documentation

- [x] 7.1 Added a new "Per-Game Eval Log" subsection under §8 of
      `docs/TRAINING_RUNBOOK.md` (immediately after the eval sidecar
      schema), documenting the row schema and linking to the MCP tool.
- [x] 7.2 Added `### run_per_game_evals(name, filter?, limit?)` section
      to `docs/TRAINING_MCP.md` with full filter table, example
      response, and two recipes: whale-game replay lookup and per-game
      digivolve-distribution analysis.

## 8. End-to-end verification

- [x] 8.1 `test_train_writes_eval_game_log_jsonl_end_to_end` runs a 2000-step
      training with `eval_freq=500`, asserts `eval_game_log.jsonl`
      exists under `models/<run>/` with >=3 rows and the full schema.
- [x] 8.2 The same test groups rows by `(step, eval_window_idx)` and
      verifies `game_idx` is contiguous from 0 within each window —
      equivalent to row count = `n_eval_episodes × n_eval_windows` for
      the unblocked case.
- [x] 8.3 Spot-check left for manual run by user. The per-game-log
      writer + recording-path-finder logic is unit-tested
      (`test_find_eval_recording_path_walks_env_chain`,
      `test_row_builder_absolutizes_recording_path`), and the
      `recording_path` field is asserted parseable / non-null /
      round-trip-able by the MCP recording-path-round-trips test.
- [x] 8.4 `openspec validate add-per-game-eval-log --strict` → "Change
      is valid".

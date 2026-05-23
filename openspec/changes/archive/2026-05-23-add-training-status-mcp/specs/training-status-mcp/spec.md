## ADDED Requirements

### Requirement: MCP stdio server bootstrap

The system SHALL ship a Python MCP stdio server, runnable as `python -m digimon_training_mcp`, that speaks the Model Context Protocol over stdin/stdout via the official `mcp` Python SDK. The server SHALL accept `--runs-dir <path>` and `--models-dir <path>` CLI flags, defaulting to `./runs` and `./models` resolved by walking up to 6 ancestor directories from the current working directory (same pattern as `digimon-engine-mcp`'s `default_cards_json_path`). The server SHALL NOT mutate any artifact in `runs/` or `models/`.

#### Scenario: Server starts and advertises its tool surface

- **WHEN** an MCP client connects to the server over stdio and sends the `initialize` request followed by `tools/list`
- **THEN** the server responds with seven tools — `list_runs`, `run_summary`, `run_metric`, `run_tags`, `run_recordings`, `run_checkpoints`, `run_deck_pool` — each carrying its JSON Schema for arguments

#### Scenario: Server resolves runs and models dirs from cwd

- **WHEN** the server starts in a working directory two levels above one containing `runs/` and `models/`, with no `--runs-dir` or `--models-dir` flags
- **THEN** the server locates the artifact roots by ancestor-walk and uses them for all subsequent tool calls

#### Scenario: Server rejects write-shaped tool calls

- **WHEN** a client invokes any method or tool name suggesting mutation (e.g. `start_run`, `delete_recording`, `promote_checkpoint`)
- **THEN** the server returns a JSON-RPC method-not-found error and no filesystem write occurs

### Requirement: list_runs tool

The system SHALL expose a `list_runs` tool returning one entry per directory under `--runs-dir`, with each entry containing `name`, `started_at` (timestamp from header block or first console.log mtime), `last_modified` (latest mtime across `console.log` and the most recent TB event file), `active` (bool — true iff at least one of those mtimes is within 60 seconds of the call time), `latest_step` (highest step from the eval sidecar, or null if none), and `latest_win_rate` (most recent `win_rate` from the eval sidecar, or null). The tool SHALL NOT recurse into nested subdirectories in v1.

#### Scenario: List returns recent runs sorted by last_modified descending

- **WHEN** `list_runs` is called against a `--runs-dir` containing three runs of varying mtimes
- **THEN** the response is a JSON array of three entries ordered with the most recently modified run first

#### Scenario: Active flag reflects 60-second freshness window

- **WHEN** `list_runs` is called against a run whose `console.log` was last written 30 seconds ago and a stale sibling whose log was last written 5 minutes ago
- **THEN** the fresh run's entry has `active: true` and the stale run's entry has `active: false`

#### Scenario: Missing sidecar leaves step/win-rate fields null

- **WHEN** `list_runs` is called against a run directory that has no `evals.jsonl` (a pre-sidecar run)
- **THEN** that run's entry has `latest_step: null` and `latest_win_rate: null` and the other fields are still populated

### Requirement: run_summary tool

The system SHALL expose a `run_summary(name: string, tail_evals?: int = 10)` tool that returns a dictionary with four sections: `header` (parsed header block from `console.log` — algo, opponent, steps, eval-freq, profile, hash, deck pool), `evals` (the last `tail_evals` rows from `evals.jsonl`, falling back to regex-parsed console eval lines if the sidecar does not exist), `panics` (an object `{total: int, by_family: {<family_id>: int, ...}}` built by matching console panic lines against patterns from `panic-families.json`), and `recent_console_tail` (the last ~50 lines of `console.log` as a string array).

#### Scenario: Summary returns header, eval tail, panic mix, and console tail

- **WHEN** `run_summary("generalist_v2", tail_evals=5)` is called against a run with a header block, 12 eval rows in the sidecar, 7 panic lines spanning 2 families, and a 200-line console log
- **THEN** the response contains `header.algo`, `header.opponent`, `header.deck_pool`; `evals` is a JSON array of length 5 (the most recent rows); `panics.total == 7` with `panics.by_family` summing to 7; `recent_console_tail` contains the last 50 lines

#### Scenario: Eval sidecar absent — falls back to console regex

- **WHEN** `run_summary` is called against a run with no `evals.jsonl` but a console.log containing `[Eval @ N steps]` lines
- **THEN** the `evals` field is populated by regex-parsing the console lines into the same row shape (subset of fields: `step`, `win_rate`, `mean_reward`, `games_played`); missing fields are null

#### Scenario: Panic with no family match counts under "other"

- **WHEN** `run_summary` encounters a panic line whose message does not match any pattern in `panic-families.json`
- **THEN** that panic is counted under `panics.by_family.other` and is reflected in `panics.total`

### Requirement: run_metric and run_tags tools

The system SHALL expose a `run_metric(name: string, tag: string | string[], since_step?: int)` tool returning either a single time-series `[{step, wall_time, value}, ...]` (when `tag` is a string) or a dict-of-time-series `{<tag>: [...]}` (when `tag` is an array). The system SHALL also expose `run_tags(name: string)` returning the full list of scalar tags discovered in the run's TensorBoard event files. Both tools SHALL use `tensorboard.backend.event_processing.event_accumulator.EventAccumulator`, cache one accumulator per run, and call `Reload()` on every invocation to pick up new events in active runs.

#### Scenario: Single-tag metric returns the time-series sorted by step

- **WHEN** `run_metric("generalist_v2", "pilot/win_rate")` is called against a run with 30 logged eval points
- **THEN** the response is a JSON array of 30 entries each shaped `{step: int, wall_time: float, value: float}`, ordered by `step` ascending

#### Scenario: since_step filters server-side

- **WHEN** `run_metric("generalist_v2", "pilot/win_rate", since_step=100_000)` is called and there are 30 eval points (10 below 100_000, 20 at or above)
- **THEN** the response contains exactly 20 entries, all with `step >= 100_000`

#### Scenario: Multi-tag returns a dict

- **WHEN** `run_metric("generalist_v2", ["pilot/win_rate", "train/loss"])` is called
- **THEN** the response is `{"pilot/win_rate": [...], "train/loss": [...]}`

#### Scenario: run_tags lists every scalar tag present

- **WHEN** `run_tags("generalist_v2")` is called against a run that has written `time/fps`, `rollout/ep_rew_mean`, `rollout/ep_len_mean`, `train/loss`, `train/value_loss`, `train/policy_gradient_loss`, `pilot/win_rate`, `pilot/draw_rate`, `pilot/games_played`, `pilot/mean_eval_reward`, `pilot/mean_eval_terminal_score`, `pilot/mean_eval_dense_reward`, `pilot/mean_eval_episode_length`
- **THEN** the response is a JSON array containing every one of those tag strings

#### Scenario: Active-run reload picks up newly written events

- **WHEN** `run_metric` is called twice against an active run, with new eval points written between the calls
- **THEN** the second call's response includes the new points without requiring server restart

### Requirement: run_recordings tool

The system SHALL expose a `run_recordings(name: string, filter?: "crash"|"draw"|"all" = "all", limit?: int)` tool returning an inventory `[{path, source, env, game, result, reason, mtime}, ...]` of recording files for run `<name>`. Recording files SHALL be located by resolving `<models-dir>/<name>/recordings/` first, then (if not present directly) by descending into the most-recently-modified timestamped subdirectory under `<models-dir>/<name>/` and looking for its `recordings/`. Files SHALL be parsed from filenames matching the regex `^(?P<source>[A-Za-z0-9-]+)_env_(?P<env>\d{3})_game_(?P<game>\d{6})_(?P<result>[A-Za-z0-9_-]+?)_(?P<reason>[A-Za-z0-9_-]+)\.json$` (NO step field in the filename — step count lives in the file body's `outcome.step_count`). The tool SHALL sort by mtime descending. The response SHALL also include `model_run_id` (the subdirectory name that was resolved) so the caller knows which model run was inspected.

When `filter == "crash"`, the tool SHALL include only recordings whose parsed `reason == "crash"`. When `filter == "draw"`, the tool SHALL include only recordings whose `result == "draw"` AND `reason != "crash"` (drawn games that weren't crashes). When `filter == "all"`, no filter is applied. When `limit` is provided, the tool SHALL truncate to that many entries post-filter.

#### Scenario: Crash filter returns only recordings with reason=crash

- **WHEN** `run_recordings("generalist_v2", filter="crash")` is called against a recordings directory containing 50 files (15 with reason=`crash`, 10 with result=`draw`/reason=`step_limit`, 25 with result=`win`)
- **THEN** the response contains exactly 15 entries, all with `reason == "crash"`

#### Scenario: Path field is consumable by engine MCP load_recording

- **WHEN** an agent calls `run_recordings("generalist_v2", limit=1)` then calls `digimon-engine-mcp`'s `load_recording` with the returned `path` field
- **THEN** the engine MCP successfully loads the recording (path is absolute and well-formed)

#### Scenario: Limit truncates after filter

- **WHEN** `run_recordings("generalist_v2", filter="crash", limit=5)` is called against 15 crash recordings
- **THEN** the response contains 5 entries — the most recently modified crashes

#### Scenario: Filename matches the training_recording.py format exactly

- **WHEN** `run_recordings("generalist_smoke", filter="crash")` is called against a directory containing the literal file `train_env_000_game_000034_draw_crash.json` (the example cited in `engine-gaps.md`'s G-DELETION-RESUME-NESTED entry)
- **THEN** the response contains an entry with `source="train"`, `env=0`, `game=34`, `result="draw"`, `reason="crash"`

#### Scenario: Path resolution descends into timestamped subdir when needed

- **WHEN** `run_recordings("generalist_v2")` is called against `models/generalist_v2/` which contains `pilot_ppo_20260523_100355/recordings/` (no direct `recordings/` at the top level)
- **THEN** the response carries `model_run_id == "pilot_ppo_20260523_100355"` and the recordings inventoried come from that subdirectory's `recordings/`

### Requirement: run_checkpoints tool

The system SHALL expose a `run_checkpoints(name: string)` tool returning `[{step, path, mtime, size_mb}, ...]` for every checkpoint file matching the format `step_NNNNNNNNN.zip` (9-digit zero-padded step, per `pilot_training.py:593`) located under `<models-dir>/<name>/checkpoints/` or — when not present directly — under the most-recently-modified timestamped subdirectory's `checkpoints/`. `step` SHALL be parsed from the filename, `size_mb` SHALL be a float (bytes / 1_048_576), and the response SHALL be sorted by `step` ascending. The response SHALL include `model_run_id` (the subdirectory chosen) alongside the listing.

#### Scenario: Checkpoint listing is parsed from filenames

- **WHEN** `run_checkpoints("generalist_v2")` is called against a directory containing `step_000100000.zip`, `step_000200000.zip`, `step_000300000.zip`
- **THEN** the response's `checkpoints` field is three entries with `step` values 100_000, 200_000, 300_000 in ascending order, each with `size_mb` as a float and `path` as an absolute path

### Requirement: run_deck_pool tool

The system SHALL expose a `run_deck_pool(name: string)` tool returning the parsed contents of `deck_pool_snapshot.json`, located by checking `<models-dir>/<name>/deck_pool_snapshot.json` first, then descending into the most-recently-modified timestamped subdirectory and looking there. The response SHALL be a JSON object with `archetypes: string[]`, `deck_count: int`, `decks: object[]`, and `model_run_id` (the subdirectory resolved). The tool SHALL NOT parse, validate, or re-resolve deck contents — it returns the file verbatim plus `deck_count` derived from `decks.length`.

#### Scenario: Deck pool snapshot is returned verbatim

- **WHEN** `run_deck_pool("generalist_v2")` is called against a run with a `deck_pool_snapshot.json` listing 4 archetypes and 8 decks
- **THEN** the response contains `archetypes` of length 4, `deck_count: 8`, and `decks` of length 8 — content matching the snapshot file byte-for-byte (modulo whitespace normalization)

#### Scenario: Missing snapshot returns structured error

- **WHEN** `run_deck_pool("nonexistent_run")` is called and no snapshot file exists
- **THEN** the tool returns a structured `{ ok: false, error: "deck_pool_snapshot.json not found for run 'nonexistent_run'" }` rather than raising an unhandled exception

### Requirement: Eval sidecar emission from pilot_training

The system SHALL extend `code/digimon_gym/agents/pilot_training.py` such that the periodic-eval callback appends one structured JSON row per eval to `runs/<run_name>/evals.jsonl`. The row SHALL contain `step`, `wall_time`, `win_rate`, `mean_reward`, `draw_rate`, `mean_terminal_score`, `mean_dense_reward`, `mean_eval_episode_length`, and `games_played`. The write SHALL be append-only, line-buffered, and SHALL NOT replace, supplement, or alter the existing TensorBoard scalar writes or the existing `[Eval @ N steps] ...` console print. If the run directory cannot be resolved at callback init time (no `tensorboard_log` path supplied), the sidecar emission SHALL be silently skipped.

#### Scenario: Eval sidecar is appended on each evaluation

- **WHEN** `pilot_training` runs with eval-freq such that three evaluations occur during a run
- **THEN** `runs/<run>/evals.jsonl` exists with exactly three lines, each a JSON object containing the required fields and a strictly increasing `step` field

#### Scenario: Sidecar emission does not change TB or console output

- **WHEN** an eval completes
- **THEN** the same `pilot/*` scalar tags are written to TensorBoard with the same values, the same `[Eval @ N steps]` line is printed, and the new `evals.jsonl` row is appended — all three outputs reflect the same eval

#### Scenario: Missing tensorboard_log path skips sidecar without error

- **WHEN** `pilot_training` is configured without a `tensorboard_log` path (e.g. a test harness)
- **THEN** the callback runs to completion, TB writes are skipped as usual, the console print still occurs, and no `evals.jsonl` is created

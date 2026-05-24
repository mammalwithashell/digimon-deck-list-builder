## ADDED Requirements

### Requirement: run_per_game_evals tool

The system SHALL expose a `run_per_game_evals` MCP tool that reads and
returns rows from `models/<name>/eval_game_log.jsonl`. The tool SHALL
sort the result by `(step, eval_window_idx, game_idx)` ascending and
return a JSON array of row objects matching the schema defined by the
`per-game-eval-log` capability.

The tool SHALL accept the following arguments:

- `name` (string, required): the run name (subdirectory under `--models-dir`)
- `filter` (object, optional) with these optional keys:
  - `agent_archetype` (string): exact match on `agent_archetype`
  - `opponent_archetype` (string): exact match on `opponent_archetype`
  - `result` (string): one of `"win"`, `"loss"`, `"draw"` — exact match
  - `digivolves_agent_min` (int): keep rows where `digivolves_agent >= N`
  - `dna_digivolves_agent_min` (int): keep rows where
    `dna_digivolves_agent >= N`
  - `step_min` (int) / `step_max` (int): inclusive step-range bounds
- `limit` (int, optional): truncate the result to this many rows
  post-filter, post-sort. When omitted, all matching rows are returned.

The tool SHALL NOT mutate any file. The tool SHALL resolve the run
directory using the same logic as `run_recordings` and `run_checkpoints` —
checking `<models-dir>/<name>/eval_game_log.jsonl` first, then descending
into the most-recently-modified timestamped subdirectory if no such file
is found at the top level. The response SHALL include a `model_run_id`
field naming the subdirectory resolved.

#### Scenario: Returns sorted rows from the eval game log

- **WHEN** `run_per_game_evals("generalist_v2")` is called against a run
  with an `eval_game_log.jsonl` containing 30 rows spanning three eval
  windows
- **THEN** the response contains 30 rows total, sorted by
  `(step, eval_window_idx, game_idx)` ascending

#### Scenario: Filter combines AND-style across keys

- **WHEN** `run_per_game_evals("generalist_v2", filter={agent_archetype: "BlueFlare", result: "win", dna_digivolves_agent_min: 1})`
  is called against a 100-row file
- **THEN** the response contains only rows where `agent_archetype ==
  "BlueFlare"` AND `result == "win"` AND `dna_digivolves_agent >= 1`

#### Scenario: digivolves_agent_min surfaces "whale games"

- **WHEN** `run_per_game_evals("generalist_v2", filter={digivolves_agent_min: 3})`
  is called against a run where most games have 0–1 digivolves but two
  games have 4 and 5
- **THEN** the response contains exactly those two rows, each with
  `digivolves_agent >= 3`

#### Scenario: Limit truncates after filter and sort

- **WHEN** `run_per_game_evals("generalist_v2", filter={result: "win"}, limit=10)`
  is called against a run with 80 winning rows
- **THEN** the response contains 10 rows — the lowest 10 by sort order

#### Scenario: Missing file returns empty list, not error

- **WHEN** `run_per_game_evals("legacy_run_without_game_log")` is called
  against a run that predates this feature and has no
  `eval_game_log.jsonl`
- **THEN** the response is `{rows: [], model_run_id: "<resolved>"}` with
  no error

#### Scenario: Path resolution descends into timestamped subdir when needed

- **WHEN** `run_per_game_evals("generalist_v2")` is called against
  `models/generalist_v2/` which contains
  `pilot_ppo_20260523_100355/eval_game_log.jsonl` (no direct file at the
  top level)
- **THEN** the response carries `model_run_id ==
  "pilot_ppo_20260523_100355"` and the rows come from that subdirectory

#### Scenario: Recording path field round-trips to engine MCP load_recording

- **WHEN** a row returned by `run_per_game_evals` has a non-null
  `recording_path`, and that path is passed to `digimon-engine-mcp`'s
  `load_recording` tool
- **THEN** the engine MCP successfully loads the recording

### Requirement: list_runs advertises per-game-log presence

The `list_runs` tool SHALL include a new field `has_eval_game_log` (bool)
on each entry, indicating whether `eval_game_log.jsonl` exists for that
run (checked at the top level first, then in the most-recently-modified
timestamped subdirectory). This SHALL be a cheap existence check; the
tool SHALL NOT parse the file to populate this flag.

#### Scenario: Flag is true when the file exists

- **WHEN** `list_runs` is called against a `--runs-dir` containing one
  run with `eval_game_log.jsonl` present and one run with no such file
- **THEN** the first entry has `has_eval_game_log: true` and the second
  has `has_eval_game_log: false`

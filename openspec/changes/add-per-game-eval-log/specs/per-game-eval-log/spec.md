## ADDED Requirements

### Requirement: WinRateCallback emits one row per completed eval game

The system SHALL extend `WinRateCallback._run_evaluation` in
`code/digimon_gym/agents/pilot_training.py` such that, immediately after
each per-game iteration of the eval loop finishes computing
`winner_id`, `p1_digi_game`, `p1_dna_game`, `p2_digi_game`, `p2_dna_game`,
`terminal_score`, `agent_archetype`, `opponent_archetype`, and `steps`,
the callback constructs a row and appends it to the configured
`GameLogWriter`. The callback SHALL NOT alter the existing means,
TensorBoard scalars, `evals.jsonl` writes, or per-archetype tallies.

Each row SHALL be a single JSON object on its own line with the following
fields:

- `step` (int): `self.num_timesteps` at the time the eval window started
  (captured once on entry to `_run_evaluation`)
- `eval_window_idx` (int): monotonically increasing index of the eval
  window within the run (0-based), incremented at entry to
  `_run_evaluation`
- `game_idx` (int): 0-based index of the game within the eval window
  (the eval loop's iteration counter)
- `source` (string, always `"eval"` for v1): present so future
  training-side rows are distinguishable in a unified file
- `agent_archetype` (string or null): `info.get("deck1_archetype")` from
  the eval `env.reset()` info dict
- `opponent_archetype` (string or null): `info.get("opponent_archetype")`
  from the eval `env.reset()` info dict
- `digivolves_agent` (int): `p1_digi_game` (final value of
  `Game.n_digivolutions[0]`)
- `dna_digivolves_agent` (int): `p1_dna_game`
- `digivolves_opponent` (int): `p2_digi_game`
- `dna_digivolves_opponent` (int): `p2_dna_game`
- `result` (string): one of `"win"`, `"loss"`, `"draw"` derived from
  `winner_id` (1 → win, 2 → loss, else → draw)
- `episode_length` (int): the `steps` variable accumulated during the
  game
- `terminal_score` (float): the terminal reward component (without dense
  shaping)
- `recording_path` (string or null): absolute path to the matching
  recording file if recording is enabled in the eval env stack; null
  otherwise
- `match_format` (string, `"single"` or `"bo3"`): the
  `TrainingConfig.match_format` in effect for this run.
- `match_idx` (int or null): in `match_format="bo3"` runs, the 0-based
  index of the BO3 match this game belongs to within the eval window
  (equal to the eval-loop iteration counter). Null in
  `match_format="single"` runs.
- `game_in_match_idx` (int 0..2 or null): in `match_format="bo3"` runs,
  the 0-based index of this game within its BO3 match (0 = game 1, up
  to 2 = game 3). Null in `match_format="single"` runs.

In `match_format="bo3"` runs, the system SHALL emit one row per inner
game (not per outer Gym episode). `game_idx` increments per row
monotonically across the eval window, so a window with 2 BO3 matches
of 2-3 inner games each produces 4-6 rows with `game_idx` 0..N-1. Rows
from the same BO3 match share their `match_idx` and have contiguous
`game_in_match_idx` values 0..N-1. Per-game digivolve counts and
winner IDs are snapshotted at each inner-game termination inside
`MatchEnv._handle_game_terminal` and exposed via the
`MatchEnv.match_game_history` list that the callback iterates after
each match.

#### Scenario: One row per game appended during eval

- **WHEN** `WinRateCallback._run_evaluation` runs with
  `n_eval_episodes=3` and a configured `GameLogWriter`
- **THEN** the output file `eval_game_log.jsonl` gains exactly three new
  lines, each parseable as a JSON object with all required fields

#### Scenario: Digivolve counts reflect end-of-game state

- **WHEN** a game ends in which the agent performed 2 regular digivolves
  and 1 DNA digivolve and the opponent performed 3 regular and 0 DNA
- **THEN** the appended row has `digivolves_agent: 2`,
  `dna_digivolves_agent: 1`, `digivolves_opponent: 3`,
  `dna_digivolves_opponent: 0`

#### Scenario: Step and eval_window_idx are stable across the window

- **WHEN** `_run_evaluation` runs an eval window with
  `n_eval_episodes=10` at `self.num_timesteps == 250_000`
- **THEN** all 10 rows share the same `step: 250_000` and the same
  `eval_window_idx`, with `game_idx` ranging 0–9

#### Scenario: Result maps winner_id correctly

- **WHEN** three games end with `winner_id` values 1, 2, and null (draw)
  respectively
- **THEN** the three appended rows have `result` values `"win"`,
  `"loss"`, and `"draw"` respectively, with `terminal_score` values
  `1.0`, `-1.0`, and `0.0`

#### Scenario: Recording path is populated when recording is enabled

- **WHEN** the eval env stack includes a recording wrapper that writes
  a file at game end, and the eval env's most-recent recording path is
  reachable from `WinRateCallback._eval_env`
- **THEN** the appended row's `recording_path` field contains that
  absolute path and `os.path.exists(recording_path)` is true

#### Scenario: Recording path is null when no recording is written

- **WHEN** the eval env stack has no recording wrapper, or recording is
  disabled for this run
- **THEN** the appended row's `recording_path` field is `null`

#### Scenario: BO3 mode emits one row per inner game with grouping fields

- **WHEN** `WinRateCallback._run_evaluation` runs with `match_format="bo3"`,
  `n_eval_episodes=2` (= 2 matches), and each match plays 2 inner games
  before resolving
- **THEN** the output gains 4 new rows (2 matches × 2 games), each with
  `match_format="bo3"`. Within each match, rows share `match_idx` and
  carry `game_in_match_idx` values 0 and 1 in order. Across the window,
  `game_idx` is contiguous 0..3.

#### Scenario: BO3 per-game digivolve counts reflect each inner game

- **WHEN** a BO3 match's first inner game has agent digivolves=2/DNA=0
  and the second inner game has digivolves=0/DNA=1
- **THEN** the two emitted rows have `digivolves_agent` values
  `[2, 0]` and `dna_digivolves_agent` values `[0, 1]` matching the
  per-game state at each inner-game termination (NOT the post-match
  engine state, which would only show the final game's counters)

#### Scenario: Single mode rows leave BO3 grouping fields null

- **WHEN** a run is launched with `match_format="single"` and the eval
  loop emits rows
- **THEN** every row has `match_format: "single"`, `match_idx: null`,
  and `game_in_match_idx: null`

#### Scenario: Existing telemetry is unaffected

- **WHEN** an eval window completes with game-log emission enabled
- **THEN** the same `pilot/*` TB scalars are written with the same
  values, the same `evals.jsonl` row is appended, and the same
  per-archetype cumulative tallies are updated — all unchanged byte-for-byte
  versus a run with `--eval-game-log off`

### Requirement: GameLogWriter handles file lifecycle

The system SHALL provide a `GameLogWriter` class in
`code/digimon_gym/agents/game_log.py` that owns the JSONL file handle.
The writer SHALL open the file in append-only text mode (`"a"`), SHALL
flush after every row write so partial runs leave a readable file, and
SHALL gracefully handle being constructed multiple times against the
same path (open-or-append; never truncate). On any `OSError` during
write, the writer SHALL disable itself, log once to stderr, and silently
no-op on subsequent calls — training MUST NOT be killed by observability
code.

#### Scenario: Writer appends rather than truncates on re-open

- **WHEN** a `GameLogWriter` is constructed against a path that already
  contains two rows from a prior run, and one new row is written
- **THEN** the file contains three lines: the two pre-existing rows
  unchanged, followed by the new row

#### Scenario: Each write is flushed

- **WHEN** a `GameLogWriter` writes a row and the process is then killed
  with SIGKILL before any clean shutdown
- **THEN** the row is fully present on disk and parseable as JSON

#### Scenario: Write failure disables writer without raising

- **WHEN** a `GameLogWriter` is configured with a path whose parent
  becomes read-only mid-run, causing the next `append()` to raise
  `OSError`
- **THEN** the writer flips `enabled = False`, logs once to stderr, and
  subsequent `append()` calls return without raising

### Requirement: WinRateCallback constructs and owns the writer

The system SHALL extend `WinRateCallback.__init__` to accept an optional
`game_log_writer: GameLogWriter | None = None` argument. When provided,
`_run_evaluation` SHALL use it; when `None` (the legacy path or when
disabled by CLI flag), no row emission occurs.

#### Scenario: No writer means no emission

- **WHEN** `WinRateCallback` is constructed with `game_log_writer=None`
  and an eval window completes
- **THEN** no `eval_game_log.jsonl` file is created and no rows are
  emitted, while all other callback behavior is unchanged

### Requirement: CLI flag controls game-log emission

The system SHALL accept a `--eval-game-log {on,off}` flag on the
`pilot_training` CLI, mirroring the existing `--mulligan-log` flag. The
flag SHALL default to `on`. When `on`, `train()` SHALL construct a
`GameLogWriter` pointing at `models/<run_id>/eval_game_log.jsonl` and
pass it to the `WinRateCallback`. When `off`, no writer is constructed
and no file is written.

#### Scenario: Default is on

- **WHEN** `python -m digimon_gym.agents.pilot_training` is run with no
  `--eval-game-log` flag and at least one eval window completes
- **THEN** `models/<run_id>/eval_game_log.jsonl` exists with at least
  `n_eval_episodes` rows

#### Scenario: Off disables emission

- **WHEN** the CLI is invoked with `--eval-game-log off`
- **THEN** no `eval_game_log.jsonl` file is created in the run
  directory

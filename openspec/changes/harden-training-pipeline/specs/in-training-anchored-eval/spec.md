## ADDED Requirements

### Requirement: Periodic anchored panel inside the training loop

Training runs SHALL support an in-run anchored evaluation panel, executed by a dedicated callback every `anchored_eval_freq` steps (default 100000; `0` disables), playing `anchored_eval_games` seat-balanced games (default 24) per anchor against greedy plus every champion from the registry whose tensor-layout hash matches the run's profile. Anchor opponents SHALL be frozen for the duration of the run and SHALL never include the training opponent as such.

#### Scenario: Panel runs on schedule

- **WHEN** a run is configured with `anchored_eval_freq=25000` and trains 50000 steps
- **THEN** the anchored panel executes twice, each time against the same frozen anchor set

#### Scenario: Seat balance within the panel

- **WHEN** a panel matchup plays N games
- **THEN** the candidate plays first in half of them (alternating by game-index parity) and the reported win rate is the seat-averaged value

#### Scenario: Incompatible champions are excluded

- **WHEN** the registry contains a champion whose tensor-layout hash differs from the run's profile
- **THEN** that champion is excluded from the panel and the exclusion is logged once at run start

#### Scenario: Disabled by zero

- **WHEN** `anchored_eval_freq=0`
- **THEN** no anchored panel runs and no anchored artifacts are written

### Requirement: Anchored results surface as scalars and a dedicated sidecar

Each panel execution SHALL log TensorBoard scalars `pilot/anchored/greedy/win_rate`, `pilot/anchored/<champion-name>/win_rate`, and `pilot/anchored/panel_mean`, and SHALL append one JSON row per panel to `anchored_evals.jsonl` in the run directory (fields: step, wall_time, per-anchor win/loss/draw counts and win rates, panel wall-clock seconds). The existing `evals.jsonl` schema SHALL NOT change.

#### Scenario: Sidecar row per panel

- **WHEN** a panel completes at step 100000
- **THEN** `anchored_evals.jsonl` gains exactly one row with `step=100000` containing per-anchor results and the panel's wall-clock duration

#### Scenario: evals.jsonl unaffected

- **WHEN** a run executes both training-opponent evals and anchored panels
- **THEN** `evals.jsonl` contains only training-opponent eval rows, byte-compatible with pre-change readers

### Requirement: Panel failures never abort training

A crash inside any panel game (including engine panics) SHALL be caught, logged with the anchor name, and skipped; the training run SHALL continue. A panel that completes partially SHALL record results only for the anchors it completed.

#### Scenario: Engine panic during a panel game

- **WHEN** the engine raises during an anchored panel game
- **THEN** the panel logs the failure, the affected anchor is marked failed in the sidecar row, and the next training step proceeds normally

### Requirement: Training MCP exposes anchored rows

The training-status MCP SHALL provide a reader for `anchored_evals.jsonl` (per-run, most-recent-first, with an optional limit), and `run_summary` SHALL include the latest anchored row when one exists.

#### Scenario: Querying anchored history

- **WHEN** the MCP tool is called for a run with three completed panels
- **THEN** it returns the three rows with per-anchor win rates, most recent first

#### Scenario: run_summary surfaces the latest panel

- **WHEN** `run_summary` is called for a run that has at least one anchored row
- **THEN** the summary includes the latest panel's per-anchor win rates alongside the existing training-eval fields

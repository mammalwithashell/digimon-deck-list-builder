## ADDED Requirements

### Requirement: digimon-engine-cli Binary

The system SHALL provide a `digimon-engine-cli` binary built from the `digimon-engine` workspace. The binary SHALL link directly against the `digimon-engine` crate and SHALL NOT depend on Python, PyO3, or the hosted API.

The CLI SHALL be invokable as `digimon-engine-cli <subcommand>` and SHALL expose three subcommands: `debug`, `scenario`, and `replay`.

The CLI SHALL respect a `--pool` flag accepting `implemented` (default), `all`, or a path to a JSON file containing a card-ID list. `implemented` SHALL resolve to `digimon_engine::cards::build_registry().registered_card_ids()`.

#### Scenario: Binary runs without Python

- **WHEN** `digimon-engine-cli` is invoked on a system with no Python runtime installed
- **THEN** the binary executes successfully and produces output

#### Scenario: Default pool is implemented cards

- **WHEN** `digimon-engine-cli debug` is invoked with no `--pool` flag
- **THEN** the loaded card pool equals the result of `build_registry().registered_card_ids()`

#### Scenario: Custom pool from file

- **WHEN** `digimon-engine-cli scenario foo.yaml --pool path/to/pool.json` is invoked
- **THEN** the runner loads only the card IDs listed in `pool.json` and rejects scenarios referencing other cards

---

### Requirement: Interactive REPL Subcommand

The `debug` subcommand SHALL start an interactive REPL backed by a `LiveGame`. The REPL SHALL accept one command per line and SHALL print state views after each command.

Supported REPL commands SHALL include at minimum:

- `new decks <deck1.json> <deck2.json> [--seed N]` — construct `LiveGame::from_decks`
- `new debug <setup.yaml>` — construct `LiveGame::from_debug`
- `load <recording.json> [--step N]` — construct `LiveGame::from_recording[_at_step]`
- `state [--view player0|player1|god]` — print `StateView`
- `hand <player> [--view ...]` — print `HandView`
- `field <player> [--view ...]` — print `FieldView`
- `pending` — print `PendingSelectionView`
- `queue` — print `EffectQueueView`
- `events [--since N]` — print recent events
- `actions <player>` — print `legal_actions` output
- `play <player> <hand_idx>` — submit `play`
- `digivolve <host_handle> <source_hand_idx>` — submit `digivolve`
- `attack <attacker_handle> <target>` — submit `attack`
- `resolve <indices...>` — submit `resolve_selection`
- `end-turn` / `pass` — submit `end_turn` / `pass_turn`
- `step <action_id>` — submit raw action ID
- `inspect <card_id>` — print card metadata and effect text
- `quit` / `exit`

Each command SHALL print a result line. Action commands SHALL print the resulting `ActionResult` summary (events count, new phase, pending selection if any).

#### Scenario: REPL accepts a sequence of commands

- **WHEN** a user starts `digimon-engine-cli debug`, types `new decks rocks.json rocks.json --seed 42`, then `state`, then `actions 0`
- **THEN** each command prints the expected output and the REPL remains responsive

#### Scenario: REPL surfaces action errors

- **WHEN** the user submits `play 0 99` and hand index 99 is illegal
- **THEN** the REPL prints the `error` field of the `ActionResult` and the game state is unchanged

---

### Requirement: Scenario Runner Subcommand

The `scenario` subcommand SHALL execute one or more YAML scenario files against the Rust engine, mirroring the contract of `tools/run_scenario.py`. Scenario format SHALL be compatible with the existing YAML scenarios under `code/tests/scenarios/`.

The subcommand SHALL accept either a single file path or a directory. When given a directory, it SHALL walk it for `*.yaml` and `*.yml` files matching an optional `--pattern` glob.

The subcommand SHALL print one PASS/FAIL line per scenario and a summary at the end. Exit code SHALL be 0 on full pass, 1 on any failure or load error.

#### Scenario: Single scenario file

- **WHEN** `digimon-engine-cli scenario foo.yaml` is invoked and `foo.yaml` is a passing scenario
- **THEN** the command prints a PASS line and exits with code 0

#### Scenario: Directory batch

- **WHEN** `digimon-engine-cli scenario qa/scenarios/ --pattern "bt24-*"` is invoked
- **THEN** every matching scenario is run, a summary line reports counts, and exit code reflects total pass/fail

#### Scenario: Format compatibility with Python runner

- **WHEN** a YAML scenario passes under `tools/run_scenario.py` against the Python engine
- **THEN** the same file passes under `digimon-engine-cli scenario` against the Rust engine (for scenarios whose cards are implemented in Rust)

---

### Requirement: Replay Viewer Subcommand

The `replay` subcommand SHALL load a `GameRecorder` recording and print a chosen view at a chosen step.

Invocation SHALL be `digimon-engine-cli replay <recording.json> [--step N] [--view player0|player1|god] [--show state|hand|field|pending|queue|events]`.

Default `--step` SHALL be `0`. Default `--view` SHALL be `god`. Default `--show` SHALL be `state`.

When `--step` exceeds the recording's `total_steps`, the runner SHALL clamp to `total_steps` and emit a warning.

#### Scenario: View at specific step

- **WHEN** `digimon-engine-cli replay rec.json --step 47 --show field --view player1` is invoked
- **THEN** the command prints `FieldView` from player 1's perspective at step 47

#### Scenario: Replay handles verify divergence

- **WHEN** the recording contains a divergence at step 30 and the CLI is invoked with `--verify`
- **THEN** the divergence is printed alongside the view and exit code is non-zero

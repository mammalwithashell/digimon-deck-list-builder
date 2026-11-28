## ADDED Requirements

### Requirement: Unified `studio-puzzle-v1` Disk Artifact
The Studio SHALL save and load canonical puzzle/exam artifacts as git-tracked JSON files under `qa/puzzles/` (supporting nested folder paths such as `qa/puzzles/<folder>/<id>.json`) using schema `"studio-puzzle-v1"`.

Each `studio-puzzle-v1` file MUST contain:
- Identity & metadata: `schema`, kebab-case `id`, `title`, `description`, optional `folder` path, `tags`, and `source` provenance (`composed`, `recording`, `watch_queue`, or `dcgo_jsonl`).
- Game initialization: `decks` (`"1"`, `"2"`), `seed`, `seat` (`1` or `2`), and `setup` (`kind: "staged"` with `state` + `zones`, or `kind: "replay_prefix"` with `prefix_actions`).
- Authored pedagogy & grading: `lines` (`id`, `name`, `role: "target" | "avoid" | "alternative"`, `actions`), `expectations` (`visits`, `avoid`, `choose`), `assertions` (`engine`, `ui`), and `rl_goal` (`mode: "playout_to_win" | "bounded_horizon"`, optional `max_turns`, `max_actions`, `win_assertions`).

The FastAPI `/studio/puzzles` endpoints SHALL list (recursively across folders), load, validate, move, and save puzzles strictly inside `qa/puzzles/` and reject invalid IDs, folder segments, or path traversal attempts.

#### Scenario: Save and reload a Studio puzzle round-trips cleanly
- **WHEN** the user saves a composed board with 2 named lines, 2 search expectations, and a bounded-horizon RL goal under ID `"ex13-training-lethal"`
- **THEN** `qa/puzzles/ex13-training-lethal.json` is written on disk and re-opening it in `/studio` restores the exact staged board, lines, expectations, assertions, and RL goal config

#### Scenario: Path traversal in puzzle ID is rejected
- **WHEN** a client requests saving or loading puzzle ID `"../escape"`
- **THEN** the API responds with HTTP 400 and does not read or write outside `qa/puzzles/`

### Requirement: Export to `qa/scenarios/` and `qa/search-exams/`
From any saved or active Studio puzzle, the Studio SHALL support one-click export (with preview/overwrite confirmation) to:
1. `qa/scenarios/<id>.json` (`schema_version: 1`) containing the staged `decks`, `seed`, `state`, `zones`, optional selected target line as `action_script`, and `assertions`, consumable directly by `code/digimon-engine/tests/scenario_corpus.rs` and Playwright fixtures.
2. `qa/search-exams/<id>.json` (`search-exam-v1`) containing the position setup, `seat`, `expectations`, and named `lines`.

`code/digimon-engine/examples/search_exam.rs` SHALL be extended to accept an optional `setup.staged` block (using `Game::stage_*`) in addition to the existing `(decks, seed, actions)` replay-prefix path, while keeping all existing `qa/search-exams/*.json` files 100% compatible.

#### Scenario: Export composed puzzle to `qa/search-exams/` and grade with Rust CLI
- **WHEN** a composed-board Studio puzzle is exported to `qa/search-exams/<id>.json` and graded via `cargo run --release -p digimon-engine --example search_exam -- qa/search-exams/<id>.json --model <model.onnx> --seeds 4`
- **THEN** `search_exam` reconstructs the staged board via `Game::stage_*`, executes determinized search, and reports per-expectation pass counts identically to replay-prefix exams

#### Scenario: Export composed puzzle to `qa/scenarios/`
- **WHEN** a Studio puzzle with engine assertions is exported to `qa/scenarios/<id>.json`
- **THEN** the exported JSON conforms to `schema_version: 1` and passes evaluation under the Rust `scenario_corpus` runner

### Requirement: RL Training Puzzle Reset Wrapper
`code/digimon_gym/` SHALL provide a `studio_puzzles.py` loader and optional `PuzzleResetWrapper` compatible with `DigimonEnv` and the pilot training wrapper chain (`ActionMasker` outermost, without altering default `DigimonEnv` behavior when disabled).

`PuzzleResetWrapper` SHALL support both RL goal modes defined in `studio-puzzle-v1`:
- `playout_to_win`: resets the episode to the puzzle's staged board (via `RustDebugGame` / staged reset) and plays out to normal game termination with standard terminal win/loss rewards.
- `bounded_horizon`: resets the episode to the puzzle's staged board and terminates early with `+1.0` if `win_assertions` are satisfied within `max_turns` / `max_actions`, or `-1.0` if the horizon expires or the solver loses before meeting the goal.

#### Scenario: RL episode resets onto a staged `playout_to_win` puzzle
- **WHEN** `PuzzleResetWrapper` wraps `DigimonEnv` with a `qa/puzzles/` directory containing a `playout_to_win` puzzle and `env.reset()` is called
- **THEN** the returned observation and action mask reflect the puzzle's staged board state (with the configured `tensor_profile` unchanged) and stepping resolves through standard win/loss termination

#### Scenario: Bounded-horizon tactical puzzle awards terminal reward on goal satisfaction
- **WHEN** an agent reaches the board state matching `rl_goal.win_assertions` within `max_actions` on a `bounded_horizon` puzzle
- **THEN** `step()` sets `terminated = True` and awards `+1.0` terminal reward; if `max_actions` is reached without satisfying `win_assertions`, `step()` ends the episode with `-1.0`

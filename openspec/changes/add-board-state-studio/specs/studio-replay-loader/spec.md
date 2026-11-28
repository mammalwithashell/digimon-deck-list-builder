## ADDED Requirements

### Requirement: Multi-Source Replay Loading Over Unified Rust ReplaySession
The backend SHALL expose a PyO3 wrapper (`RustReplaySession` over `digimon_engine`'s `ReplaySession` with `NativeAdapter` and `DcgoAdapter`) and FastAPI `/studio/replays/*` endpoints allowing the Studio to load replays from all four sources:
1. Saved in-game DB recordings (from `/recordings`).
2. Frozen-evaluation `/watch` queue games.
3. Uploaded or local native recording JSON files.
4. DCGO exporter-mod JSONL files (uploaded or selected from `qa/dcgo-mod-recordings/`), supporting both converted schema-v1 DCGO JSONL and raw MelonLoader `raw-*.jsonl`.

When a raw MelonLoader `raw-*.jsonl` file is loaded, the backend SHALL automatically convert it to schema-v1 DCGO JSONL using `code/dcgo-melon-recorder/convert_raw.py` (with `--allow-truncated`), preserve any truncation metadata, and pass the converted stream into `ReplaySession` (`DcgoAdapter`).

#### Scenario: Load a converted schema-v1 DCGO JSONL recording
- **WHEN** the user selects `qa/dcgo-mod-recordings/mod_game2.jsonl` in the Studio Replay Loader
- **THEN** the backend initializes a `DcgoAdapter` `ReplaySession`, returns total steps and step-0 board state, and displays the game on the Studio board with step/seek controls

#### Scenario: Auto-convert a raw MelonLoader JSONL recording
- **WHEN** the user uploads a MelonLoader `raw-<session>.jsonl` file
- **THEN** the backend detects the raw format, converts it via `convert_raw.py --allow-truncated`, loads the resulting schema-v1 stream into `ReplaySession`, and surfaces any conversion truncation warning alongside the replay timeline

#### Scenario: Load an in-game DB recording or /watch queue game
- **WHEN** the user picks a saved DB recording or a `/watch` queue game in the Replay Loader
- **THEN** the backend loads the recording through `NativeAdapter` and renders step-by-step playback on the Studio board

### Requirement: Interactive Stepping and Non-Fatal Divergence Surfacing
The Studio replay stepper SHALL support step forward, step backward, and arbitrary step seek (`0..total_steps`) and display each step's actor, phase, memory, and action summary.

When replaying a DCGO JSONL recording under `CheckThenApply`, any non-fatal `Divergence` (`mask_miss`, `actor`, `memory`, `phase`, `winner`, `reveal_kind`, `reveal_exhausted`) SHALL pause stepping without crashing the session and display the structured divergence details (step index, kind, recorded action, and legal action sample) in the UI.

#### Scenario: DCGO divergence pauses cleanly and remains inspectable
- **WHEN** stepping forward in a DCGO recording hits a `mask_miss` divergence at step 14
- **THEN** stepping pauses at step 14, the Studio displays the `mask_miss` banner with the recorded action and legal action sample, and the user can still inspect the board, seek to earlier steps, or fork step 14 into the Studio

### Requirement: Fork Any Replay Step Into Editable Studio Session
At any step of a loaded replay (including the step immediately preceding a divergence or truncation), the Studio SHALL provide a **"Fork this step into Studio"** action that materializes the live board via `Game::to_scenario()` into a new Studio Compose / Play & Record session while recording the replay provenance (`source.kind`, `source.ref`, `source.step`).

If the forked step belongs to an opaque PvP DCGO recording containing `is_opaque_placeholder` hidden cards, the Studio SHALL replace unrevealed placeholders with editable filler entries and display a clear warning banner prompting the author to inspect or fill hidden opponent zones before saving.

#### Scenario: Fork step 26 of a /watch replay into a Studio puzzle
- **WHEN** the user seeks to step 26 of a `/watch` recording and clicks "Fork this step into Studio"
- **THEN** the Studio opens a live editable session initialized to the exact step-26 board state, preserves replay provenance metadata, and lets the user immediately record target/avoid lines or tweak zones

#### Scenario: Forking an opaque PvP DCGO step warns about hidden cards
- **WHEN** the user forks a step from an opaque PvP DCGO recording that still has unrevealed opponent hand cards
- **THEN** the forked Compose board replaces unrevealed placeholders cleanly without crashing and displays a banner listing which opponent zones had unrevealed cards

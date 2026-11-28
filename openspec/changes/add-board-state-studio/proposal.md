## Why

Authoring training puzzles, search exams, and gameplay-rule regression scenarios currently requires hand-editing JSON fixtures (`qa/scenarios/`), running CLI scripts against finished evaluation panels (`qa/search-exams/`), or replaying full games from turn 1. There is no visual workspace to organize replays and exams, compose board states directly on the live game board, play both sides or drop trained pilot models into a staged board with step-by-step search/policy inspection, or fork in-game and DCGO exporter-mod JSONL replays into reusable puzzles and exams for agents to learn from.

## What Changes

- Introduce a **Board-State Studio** (`/studio`, gated by build-time flag `VITE_ENABLE_STUDIO` — enabled on web dev/admin builds now, disabled by default on desktop builds, and structured behind a `StudioTransport` interface so it can optionally be distributed with the desktop client later).
- Add a **Studio Library UI** for viewing, searching, and organizing both replays (DB recordings, `/watch` queues, DCGO JSONL recordings, local native replays) and exams/puzzles (`qa/puzzles/`, `qa/search-exams/`, `qa/scenarios/`) using hierarchical **folders** and **tags**.
- Extend Rust engine staging (`Game::stage_*` and `Game::to_scenario()`) and PyO3 bindings (`RustDebugGame`) so the Studio can author and round-trip full board states: battle-area digivolution stacks (sources, suspended, `turn_played`, linked cards), breeding stack, hands, ordered security stacks (with face-up toggles), trash, deck top, egg decks, memory, active turn player, turn number, and phase.
- Add a dual-seat **Play & Record** mode on the live `GameBoard` where both hands remain visible and playable under fixed orientation (with manual flip), card effects execute normally through the Rust engine, moves are recorded on a rewindable timeline, and authors can snapshot any post-effect board or record multiple named lines (`target`, `avoid`, `alternative`) plus assertions/expectations.
- Add **Pilot Model Playout & Live Policy/Search Inspection**: assign local `.onnx` pilot models (with optional determinized MCTS search), `Greedy`, `Random`, or `Human` to either or both seats on any staged board, drive playout with a **Step Controller** (Play / Pause / Speed / Step Next / Step Back / Seek / Take Over), and pause at any move to inspect raw policy priors, value-head score, MCTS search visits/Q-values/root value (when search is enabled), and live expectation pass/fail verdicts.
- Expose Rust `ReplaySession` (`NativeAdapter` + `DcgoAdapter`) over PyO3 and FastAPI so the Studio can load and step through DB recordings, `/watch` panel queues, uploaded native JSON recordings, converted schema-v1 DCGO JSONL, and MelonLoader `raw-*.jsonl` (auto-converted via `code/dcgo-melon-recorder/convert_raw.py`), surfacing divergences/truncation and allowing one-click forking of any step into an editable Studio session.
- Define a unified git-tracked `studio-puzzle-v1` artifact under `qa/puzzles/` (supporting nested folders) with direct export/sync to `qa/scenarios/` and `qa/search-exams/` (extending `examples/search_exam.rs` to accept staged starting boards) plus an RL `PuzzleResetWrapper` hook in `digimon_gym` supporting playout-to-win and bounded-horizon goal puzzles.

> **Scope note:** Post-MVP ideas (mid-turn modifier/OPT injection, visual scrubbing of raw DCGO snapshots past divergence, branching variation trees, Tauri offline backend wiring for desktop client distribution, DCGO YAML exam synthesis, and supervised imitation-loss training pipelines) are explicitly out of scope for this change and tracked separately in `docs/BOARD_STATE_STUDIO_FUTURE_IDEAS.md`.

## Capabilities

### New Capabilities

- `studio-library`: Unified Studio Library UI and backend index for browsing, searching, and organizing replays, puzzles, search exams, and scenario fixtures with hierarchical folders and tags, and launching them into Compose, Playout, or Replay modes.
- `studio-board-composer`: Interactive Studio UI (behind a `StudioTransport` seam + `VITE_ENABLE_STUDIO` flag) and Rust/FastAPI staging extensions for composing full Digimon TCG board states (cards, stacks, zones, scalars, deck templates) with live card search and engine validation.
- `studio-dual-seat-recorder`: Live two-sided board control (both hands visible and playable under real Rust effect resolution) with deterministic timeline rewind, board snapshotting, multi-line recording (`target`/`avoid`/`alternative`), and assertion/expectation authoring.
- `studio-replay-loader`: Unified Studio replay stepper over Rust `ReplaySession` supporting DB recordings, `/watch` queues, uploaded native recordings, schema-v1 DCGO JSONL, and auto-converted MelonLoader `raw-*.jsonl`, with divergence/truncation surfacing and step-to-Studio forking.
- `studio-puzzle-artifacts`: Unified `studio-puzzle-v1` on-disk schema (`qa/puzzles/` with folder paths), exporters to `qa/scenarios/` and `qa/search-exams/` (with staged-board support in `search_exam`), and RL `PuzzleResetWrapper` support for playout-to-win and bounded-horizon goal puzzles.
- `studio-agent-inspector`: Seat-level pilot model assignment (`Human` / `ONNX Model [+ optional MCTS Search]` / `Greedy` / `Random`), stepped playout controller (Play / Pause / Speed / Step Next / Step Back / Take Over), and live policy-head + MCTS search valuation inspection and expectation grading on staged boards.

### Modified Capabilities

<!-- No existing OpenSpec capability requirements are broken; existing `interactive-replay-stepper` and `live-game-surface` primitives are reused as-is. -->

## Impact

- **Rust engine & PyO3 (`code/digimon-engine/`, `code/digimon-engine-py/`)**: Additive staging/snapshot fields (`egg_deck`, `linked_cards`, `face_up_security`) in `game/staging.rs` and `game/snapshot.rs`; new PyO3 wrappers `RustReplaySession` (over `ReplaySession`) and `inspect_position` / `step_with_model` (over `BatchedPolicyValueEvaluator` / `determinized_search` exposing `root_priors`, `root_plans`, `root_q`, `root_value`); staged-board loader path in `examples/search_exam.rs`. Does **not** change `ACTION_SPACE_SIZE` or active observation tensor layouts.
- **FastAPI backend (`code/server/routers/`)**: New `studio.py` router (dev/admin mounted in `api.py`) plus additive fields on `DebugZoneSpec` / `DebugFieldStack` in `schemas.py` and `debug_games.py`. Reuses `active_games`, `RustDebugGame`, `convert_raw.py`, and existing `/recordings` + `/debug` routes.
- **React frontend (`code/frontend/src/`)**: New lazy-loaded `/studio` route gated by `VITE_ENABLE_STUDIO` (with `StudioTransport` abstraction for future desktop client distribution), Studio Library UI, Board Composer, Dual-Seat Recorder, Pilot Playout Step Controller, Agent Policy/Search Inspector, Replay Loader, and additive dual-seat props on `GameBoard` / `HandZone` / `PermanentSlot`.
- **RL training (`code/digimon_gym/`)**: New optional `PuzzleResetWrapper` and `studio_puzzles.py` loader for sampling `qa/puzzles/` (recursively across folders) during RL training; no changes to default `DigimonEnv` behavior when unused.
- **Disk artifacts (`qa/`)**: New `qa/puzzles/` directory (supporting nested folder paths) and `qa/puzzles/_library.json` metadata index for replay/exam folder and tag organization.

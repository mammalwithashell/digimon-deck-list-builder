## Context

The repository already contains strong headless building blocks across Rust, FastAPI, React, and RL tooling:

- **Rust staging & snapshot**: `Game::stage_*` (`code/digimon-engine/src/game/staging.rs`), `Game::to_scenario()` (`code/digimon-engine/src/game/snapshot.rs`), `DebugRunner`, and PyO3 `RustDebugGame` (`code/digimon-engine-py/src/lib.rs`).
- **FastAPI live & debug play**: `code/server/routers/debug_games.py` stages a `RustDebugGame` into `active_games` with `GameMeta` so it can be played live via `/games/{id}/actions` with full Rust card-effect resolution and action masking.
- **Unified replay core**: `ReplaySession` with `NativeAdapter` and `DcgoAdapter` (`openspec/specs/interactive-replay-stepper/spec.md`) parses schema-v1 DCGO JSONL and native recordings with non-fatal pausing divergences; `code/dcgo-melon-recorder/convert_raw.py` converts raw MelonLoader `raw-*.jsonl` into schema-v1 DCGO JSONL.
- **Rust ONNX & MCTS search**: `BatchedPolicyValueEvaluator` and `determinized_search_batched` return `SearchResult` (`code/digimon-engine/src/search/mcts.rs`), which already computes raw `root_priors`, `root_plans` (complete plans + visit counts + Q-values), `root_visits`, `root_q`, and `root_value`.
- **Live board, desktop runtime seam, & card search UI**: `GamePage.tsx` / `GameBoard.tsx` renders the interactive board, `engineRuntime.ts` abstracts HTTP vs Tauri runtime detection, and `DeckBuilderPage.tsx` / `digimonCardApi.ts` provides searchable card metadata.
- **Exam & RL consumers**: `qa/scenarios/` (`schema_version: 1`), `qa/search-exams/` (`search-exam-v1`, graded by `code/digimon-engine/examples/search_exam.rs`), and `code/digimon_gym/digimon_gym.py` (`DigimonEnv`).

However, these capabilities are disconnected: authors have no unified library to organize replays and exams, cannot visually compose stacks on the board, cannot drop pilot models into a staged board and step/pause through playout while viewing policy and search valuations, and cannot fork in-game or DCGO JSONL replays into reusable puzzles and exams.

## Goals / Non-Goals

**Goals:**
- Provide a **Studio Library UI** to browse, search, filter, and organize replays (DB recordings, `/watch` queues, DCGO JSONL, local native replays) and training exams/puzzles (`qa/puzzles/`, `qa/search-exams/`, `qa/scenarios/`) with hierarchical **folders** and **tags**.
- Deliver a `/studio` workspace backed by FastAPI + Rust that composes arbitrary legal board states (zones, digivolution stacks, linked cards, face-up security, egg decks, scalars, underlying decks) directly on the real `GameBoard`.
- Allow two-sided live play on the composed board with real Rust card-effect processing, both hands visible and playable, deterministic rewind/step-back, board snapshotting, and multi-line recording (`target`, `avoid`, `alternative`) plus assertions/expectations.
- Support immediately dropping pilot models (`.onnx` with optional MCTS search, plus `Greedy` / `Random` / `Human`) into either or both seats of any staged board, stepping/playing/pausing with a **Step Controller**, and inspecting both **policy head** (`policy_value`, masked action probabilities / `root_priors`) and **search valuation** (`root_value`, `root_plans` visits & Q-values, `root_q`) at every step, plus live expectation grading.
- Load and step through DB recordings, `/watch` panel queues, uploaded native JSON recordings, converted schema-v1 DCGO JSONL, and raw MelonLoader `raw-*.jsonl` (auto-converted via `convert_raw.py`) through the single Rust `ReplaySession` core, surfacing divergences/truncation cleanly and forking any step into an editable Studio board.
- Persist authored work as git-tracked `studio-puzzle-v1` JSON under `qa/puzzles/` (supporting subfolders), export directly to `qa/scenarios/` and `qa/search-exams/` (extending `search_exam` to load staged boards), and provide an RL `PuzzleResetWrapper` supporting both playout-to-win and bounded-horizon goal puzzles.
- Architect the frontend behind a typed `StudioTransport` interface and `VITE_ENABLE_STUDIO` build flag so the Studio can optionally be distributed with the desktop client later without refactoring UI components.
- Preserve existing engine invariants: **no changes** to `ACTION_SPACE_SIZE` (2192) or active observation tensor profiles (`standard_lite_v2`, `standard_lite_deck_v2`).

**Non-Goals:**
- Everything tracked in `docs/BOARD_STATE_STUDIO_FUTURE_IDEAS.md`, specifically:
  - Mid-turn internal modifier/OPT/pending-selection/custom-token injection.
  - Visual scrubbing of raw MelonLoader `Snapshot.<hook>` board dumps past a divergence/truncation point without Rust replay.
  - Full branching move trees and multi-checkpoint diff matrices.
  - Implementing the Tauri Rust `invoke()` backend (`TauriStudioTransport`) and enabling `VITE_ENABLE_STUDIO` in shipping desktop builds (the transport seam & flag are in MVP; desktop distribution itself is deferred).
  - Automated export to turn-1 symbolic `qa/dcgo-exams/*.yaml`.
  - Supervised behavior-cloning / imitation-loss training pipelines and adaptive curriculum schedulers.
- Changing normal PvP, vs-AI, or desktop gameplay routes.

## Decisions

### 1. Single Engine Source of Truth (`RustDebugGame` + `ReplaySession` + Rust ONNX/MCTS)
- **Choice**: Keep all board staging, rules resolution, replay stepping, and pilot model policy/search evaluation inside `digimon-engine` exposed via thin PyO3 bindings and a dedicated FastAPI `code/server/routers/studio.py` router.
- **Rationale**: Eliminates rules and policy-math drift between Studio playout, `scenario_corpus.rs`, `search_exam.rs`, and `DigimonEnv`. Reuses `Game::stage_*`, `Game::to_scenario()`, `ReplaySession`, `DcgoAdapter`, `BatchedPolicyValueEvaluator`, and `determinized_search_batched` directly.
- **Alternatives considered**:
  - Client-side board model in React without live Rust validation — rejected because card effects, legal masks, and stack invariants would drift from the engine.
  - Running ONNX inference in Python (`onnxruntime`) separately from Rust `determinized_search` — rejected because `SearchResult` already computes `root_priors`, `root_plans`, `root_q`, and `root_value` in one unified Rust path.

### 2. Future-Ready Client Distribution Seam (`StudioTransport` + `VITE_ENABLE_STUDIO`)
- **Choice**:
  - Gate `/studio` route mounting on a build-time flag `VITE_ENABLE_STUDIO` (default `true` in web dev/admin builds, `false` in desktop builds today so it stays tree-shaken until opted in).
  - Route every Studio backend call through a single typed `StudioTransport` interface in `code/frontend/src/api/studioTransport.ts` (implemented by `HttpStudioTransport` over FastAPI `/studio` + `/debug` in MVP, following `engineRuntime.ts`'s pattern).
  - Keep all Studio React components free of direct `fetch`/URL assumptions or web-only admin globals.
- **Rationale**: Zero extra runtime complexity today, while making future desktop client distribution a clean additive change (implement `TauriStudioTransport` + Rust Tauri commands and flip `VITE_ENABLE_STUDIO=true`).
- **Alternatives considered**: Hardcoding web-only `client.ts` calls across components — rejected because it would require rewriting every Studio component to ship with the desktop client later.

### 3. Studio Library Storage: Real Folders for Puzzles + Lightweight Metadata Index for Replays/Exams
- **Choice**:
  - Authored `studio-puzzle-v1` puzzles live as git-tracked JSON files under `qa/puzzles/[<folder>/...]<id>.json` (folder path reflected on disk AND mirrored in each file's `folder` field).
  - Replays (DB recordings, `/watch` queues, `qa/dcgo-mod-recordings/`) and existing external exams (`qa/search-exams/`, `qa/scenarios/`) stay in their canonical locations so existing CLIs and pipelines never break.
  - A lightweight git-tracked index file `qa/puzzles/_library.json` stores `{ folders: [...], items: { "<kind>:<ref>": { folder, tags, title_override, pinned } } }` so users can organize replays, search exams, scenarios, and puzzles together into the same folder tree and tag taxonomy.
- **Rationale**: Keeps `qa/dcgo-mod-recordings/`, `qa/search-exams/`, `qa/scenarios/`, and DB recordings untouched and compatible with existing tooling while giving the Studio Library full folders + tags across every item type.
- **Alternatives considered**: Moving all replays and exams into a new database or relocating `qa/search-exams/` files — rejected because CLI tools and CI tests expect existing paths.

### 4. Additive Extensions to `Game::stage_*` and `Game::to_scenario()`
- **Choice**: Extend `staging.rs`, `snapshot.rs`, `RustDebugGame`, and `DebugZoneSpec`/`DebugFieldStack` with optional fields:
  - `egg_deck: Option<Vec<String>>` (index `0` = next hatch).
  - `linked_cards: Option<Vec<String>>` on `DebugFieldStack`.
  - `security` supporting both legacy `list[str]` and `{ card_id, face_up }` entries (or parallel `face_up_security` indices).
- **Rationale**: Existing `qa/scenarios/*.json` fixtures remain 100% backward compatible while Studio MVP gains full zone/stack fidelity.

### 5. Unified Deterministic Studio Session (Compose ↔ Dual-Seat Play ↔ Pilot Model Playout)
- **Choice**: A Studio session maintains:
  1. An immutable **base staged snapshot** (`StudioBoardState`), updated only in Compose mode or via **"Snapshot current board as puzzle start"**.
  2. A **live `RustDebugGame`** materialized from the base snapshot.
  3. **Seat controller config** for Player 1 and Player 2:
     - `type: "human" | "model" | "greedy" | "random"`
     - When `type == "model"`: `{ model_id, simulations, worlds, regime, seed, temperature }` (where `simulations == 0` uses raw policy head + value head; `simulations > 0` runs determinized MCTS search).
  4. An **active timeline** (`Vec<RecordedStep>`), where each step records `{ step_index, action_id, actor, controller, phase_before, phase_after, memory_after, label, events, eval?: StepEvalSnapshot }` — `eval` stores the policy head (`policy_value`, top `policy_actions`) and optional search breakdown (`search_value`, `simulations_run`, `root_plans` with `visits`/`share`/`q_value`) captured when that decision was evaluated.
  5. A map of **saved named lines** (`id`, `name`, `role: "target" | "avoid" | "alternative"`, `actions`).
- **Step Controller & Pause/Inspect Flow**:
  - When a seat is controlled by a pilot model, **one step = one model decision** (no runaway multi-turn autoplay loop in the backend!).
  - The frontend Step Controller drives pacing (`Play` timer at `slow`/`normal`/`fast`, `Pause`, `Step Next`, `Step Back`, `Seek`).
  - While paused on a model's turn, the user can click **Preview / Inspect** (`POST /studio/games/{id}/inspect-agent`) to see how the model's policy head and search value the upcoming move *before* stepping, OR step forward (`POST /studio/games/{id}/model-step`) which evaluates, applies the chosen action, and attaches the full `StepEvalSnapshot` to that timeline step so clicking any past step in the timeline immediately displays how policy & search valued that move!
  - At any pause point, the user can toggle either seat between `Model` and `Human` (Take Over), rewind deterministically (`rebuild base snapshot + replay actions[0..k]`), or snapshot the current board as a new puzzle start.
- **Rationale**: Single-decision-per-request stepping matches `add-bot-action-pacing` and `useWatchReplay`, makes pause/inspect instantaneous and deterministic, and keeps Human, Model-vs-Model, and Human-vs-Model on one unified timeline.

### 6. Dual-Seat Board Rendering via Additive Props on `GameBoard`
- **Choice**: Add optional `studioMode` / `dualSeat` / `flipped` props to `GameBoard`, `HandZone`, and `PermanentSlot`:
  - Both P1 and P2 hands render face-up; the active decision player (`pendingSelection.selectingPlayer ?? currentPlayer`) is clearly highlighted and interactive when that seat is `Human` (and locked when a model step is in flight).
  - Manual `flipped` toggle swaps visual top/bottom seats cleanly without mutating engine player IDs.
  - In Compose mode, drop targets accept cards from `CardSearchDrawer` onto any zone or directly onto an existing `PermanentSlot` (adding a digivolution source or linked card) and clicking a permanent opens `StackInspector`.

### 7. Unified `studio-puzzle-v1` Schema with Lossless Exporters & RL `PuzzleResetWrapper`
- **Choice**: Store canonical authored puzzles as git-tracked JSON files at `qa/puzzles/[<folder>/]<id>.json` (`schema: "studio-puzzle-v1"`) carrying `folder`, `tags`, `source`, `decks`, `seed`, `setup` (`staged` or `replay_prefix`), `seat`, `lines`, `expectations`, `assertions`, and `rl_goal` (`playout_to_win` or `bounded_horizon`).
- **Exporters**:
  - `qa/scenarios/<id>.json` (`schema_version: 1`).
  - `qa/search-exams/<id>.json` (`search-exam-v1` with optional `setup.staged` block supported by `code/digimon-engine/examples/search_exam.rs`).
- **RL `PuzzleResetWrapper`**: Wraps `DigimonEnv`, recursively discovers `qa/puzzles/**/*.json` (with optional folder/tag filtering!), resets episodes onto staged boards, and enforces `playout_to_win` or `bounded_horizon` termination/rewards.

### 8. DCGO Raw JSONL Auto-Conversion & Replay Forking
- **Choice**: `/studio/replays/load` handles DB recordings, `/watch` queues, uploaded native JSON, converted schema-v1 DCGO JSONL, and raw MelonLoader `raw-*.jsonl` (auto-converted via `code/dcgo-melon-recorder/convert_raw.py --allow-truncated`), stepped through PyO3 `RustReplaySession` (wrapping Rust `ReplaySession`). Forking calls `session.to_scenario()` and opens a Studio session with provenance preserved.

## Risks / Trade-offs

- **MCTS search latency during stepped playout** → Running high simulation budgets (e.g., 320+ sims) on every step of continuous playback takes noticeable time per beat and could block FastAPI if run synchronously on the event loop. **Mitigation**: Release the Python GIL in PyO3 (`py.allow_threads`), execute `inspect_position` / `model_step` on FastAPI's worker threadpool, default interactive playout to fast policy (`0` sims) or modest search (`64` sims) with user-adjustable budget, and show an in-flight indicator while a step computes.
- **Closed-list determinization on synthetic filler-deck boards** → Closed-list belief sampling expects decklists consistent with visible zones. **Mitigation**: Default staged filler-deck boards to `open_list` regime (or policy-only), validate deck consistency when `closed_list` is chosen, and surface diagnostics cleanly in the Inspector panel.
- **Opaque PvP DCGO recordings** → Unrevealed opponent cards remain `is_opaque_placeholder` at forked steps. **Mitigation**: Surface placeholders in the replay stepper and replace unrevealed placeholders with editable filler entries + a clear warning banner when forking into Compose mode.
- **Library index drift if files are moved outside the Studio** → Moving or deleting a file directly in git could leave stale entries in `qa/puzzles/_library.json`. **Mitigation**: Treat filesystem discovery (`qa/puzzles/`, `qa/search-exams/`, `qa/scenarios/`, `qa/dcgo-mod-recordings/`, `/recordings`, `/watch`) as the source of truth at list time, merge optional metadata from `_library.json`, and silently ignore orphaned metadata keys (with an optional prune-orphans action).
- **Security / filesystem access** → `/studio` reads/writes under `qa/` and loads `.onnx` files from `models/`. **Mitigation**: Gate `/studio` behind admin/dev guards and enforce strict allowlisted-root path validation rejecting `..`, absolute paths, or non-allowlisted extensions.

## Migration Plan

- Purely additive change; no database migrations required.
- Build and install updated `digimon_engine` PyO3 wheel (`maturin develop` / Cargo build).
- `VITE_ENABLE_STUDIO` defaults to `true` for web dev/admin builds and `false` for desktop builds; rollback simply disables the flag and unmounts `studio.py` from `code/server/api.py`.

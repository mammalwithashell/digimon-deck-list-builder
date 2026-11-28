## ADDED Requirements

### Requirement: Seat Pilot Model Assignment on Any Staged Board
From any composed or replay-forked Studio board, the user SHALL be able to assign each seat (Player 1 and Player 2 independently) to one of:
- `Human` (manual play),
- `Pilot Model` — any local `.onnx` model listed by `GET /studio/models`, configured with optional determinized MCTS search parameters (`simulations >= 0`, `worlds >= 1`, `regime: "open_list" | "closed_list"`, `seed`),
- `Greedy` or `Random` baseline heuristic.

Seat assignments SHALL be changeable at any paused point during playout (e.g., switching from Model-vs-Model to Human-vs-Model to take over mid-line, or swapping checkpoints to compare responses).

#### Scenario: Drop two pilot models onto a composed board
- **WHEN** the user composes a board, assigns checkpoint A (with `simulations = 64`, `worlds = 2`) to Player 1 and checkpoint B (with `simulations = 0`, raw policy) to Player 2, and enters Playout mode
- **THEN** Player 1 decisions execute via Rust determinized MCTS search on checkpoint A and Player 2 decisions execute via checkpoint B's masked policy head on the same live `RustDebugGame`

#### Scenario: Switch a seat to Human mid-playout to test an alternative move
- **WHEN** the user pauses a Model-vs-Model playout at step 4 and switches Player 1 from `Pilot Model` to `Human`
- **THEN** the Studio unlocks Player 1's legal actions on the board immediately so the user can play manually from step 4 while Player 2 remains controlled by its assigned model

### Requirement: Stepped Model Playout Controller
When one or both seats are controlled by a pilot model / bot, the Studio SHALL provide a **Step Controller** with:
- **Play / Pause** toggle (driving one model decision per beat at selectable speed: `slow`, `normal`, `fast`),
- **Step Next** (executes exactly one model decision and pauses),
- **Step Back / Seek** (deterministically rewinds/seeks along the recorded timeline),
- **Reset to Start** (returns to step 0 of the staged board while keeping seat assignments).

Each backend `POST /studio/games/{game_id}/model-step` call SHALL execute at most **one** model decision (never auto-draining multiple model decisions in a single opaque loop) so the UI can pause, animate, and inspect policy/search valuations on every move.

#### Scenario: Single-stepping a pilot model advances exactly one decision
- **WHEN** Player 1 is assigned a pilot model and the user clicks "Step Next" on a staged board
- **THEN** the backend evaluates the model for that single decision, applies the chosen legal action to `RustDebugGame`, records the step and its policy/search evaluation snapshot onto the timeline, and returns control paused at the next state

#### Scenario: Pausing during continuous playout halts cleanly at the current move
- **WHEN** the user clicks "Play" on a Model-vs-Model staged board and clicks "Pause" after 3 moves
- **THEN** automatic stepping stops cleanly after the in-flight step completes, leaving the board inspectable at that exact decision point

### Requirement: Live Policy-Head and Search Valuation Inspection
The Rust engine PyO3 bindings (`code/digimon-engine-py/src/lib.rs`) SHALL expose `inspect_position` and `model_step` over `BatchedPolicyValueEvaluator` and `determinized_search_batched` (releasing the Python GIL via `py.allow_threads`), returning structured valuation telemetry for both raw policy and search (when `simulations > 0`):
- **Policy head**: `policy_value` (value-head estimate from the deciding seat's perspective) and `policy_actions` (top legal actions ranked by masked policy probability `prob`, with `action_id` and human-readable `label`).
- **Search (when `simulations > 0`)**: `search_value` (`SearchResult::root_value`), `simulations_run`, `root_plans` (complete multi-step plans with `actions`, human-readable `labels`, `visits`, `visit_share`, `q_value`, and raw `prior_prob` from `root_priors`), and per-edge `root_q`.
- **Chosen decision**: `chosen_action_id` (and `chosen_plan` when search ran) plus live `expectation_results` (pass/fail + observed share for every authored `visits` / `avoid` / `choose` expectation).

In the Studio UI, the collapsible **Agent & Search Inspector** panel SHALL display:
- When paused before a move (or via "Inspect Now"): live policy-head and search valuation for the **current** position.
- When clicking any prior model-played step in the timeline: the recorded policy-head and search valuation captured when that move was chosen, showing how policy priors compared against search visits and Q-values.
- Clear graceful error diagnostics when no `.onnx` models are available or a model's observation profile is incompatible, without disrupting Compose or manual Play & Record modes.

#### Scenario: Pause and compare policy head vs MCTS search valuation
- **WHEN** the user pauses a staged playout on Player 1's turn with a checkpoint configured at `simulations = 128`, `worlds = 4` and opens the Inspector panel
- **THEN** the panel displays the raw value-head score alongside the search root value, lists top moves/plans with raw policy prior %, MCTS visit count, visit share %, and mean Q-value, highlights the chosen action, and shows green/red pass/fail badges against the puzzle's expectations

#### Scenario: Policy-only inspection when search simulations equal 0
- **WHEN** the user assigns a pilot model with `simulations = 0` and steps or clicks "Inspect Now"
- **THEN** the backend runs a single masked forward pass in Rust, omits MCTS search fields cleanly (`mode: "policy_only"`), and displays the value-head score and top legal actions ranked by masked policy probability

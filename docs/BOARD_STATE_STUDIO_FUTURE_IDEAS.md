# Board-State Studio — Future Ideas (Post-MVP)

This document records ideas discussed during the Board-State Studio design that are **intentionally excluded** from the MVP OpenSpec change (`openspec/changes/add-board-state-studio/`). They are parked here so the MVP stays focused, shippable, and easy to verify, while preserving clear expansion paths for later iterations.

> **Status:** Ideas only — do **not** implement these as part of `add-board-state-studio`. Promote individual items into their own OpenSpec changes when needed.

---

## 1. Deep Mid-Turn Engine Internal State Authoring

The MVP supports full zone and stack authoring (battle stacks with sources/suspended/`turn_played`/linked cards, breeding stack, hands, ordered security with face-up toggle, trash, deck top, egg deck, memory, turn, phase, active player).

Later iterations could add direct editing of transient mid-turn engine internals:
- **Temporary modifiers**: Injecting turn-scoped or persistent DP boosts, `<Security A.+/-N>`, keyword grants (`<Rush>`, `<Blocker>`, `<Piercing>`, etc.), or "cannot attack / cannot suspend" locks directly via `ModifierRegistry`.
- **Once-per-turn (OPT) activation flags**: Marking specific card effects or inherited clauses as already used this turn without having to play out the setup move.
- **Mid-resolution / pending selection staging**: Jumping straight into a specific `PendingSelection` state (e.g., mid-reveal or mid-trigger-order) without playing the triggering action.
- **Custom token authoring**: Spawning arbitrary rule-defined Tokens with custom DP/color/keywords directly from a token palette.

---

## 2. Raw MelonLoader Snapshot Scrubbing Past Divergences

In the MVP, loading a DCGO MelonLoader `raw-*.jsonl` file converts it via `code/dcgo-melon-recorder/convert_raw.py` and steps it through our Rust `ReplaySession` (`DcgoAdapter`), pausing cleanly on any conversion truncation or engine divergence so you can fork up to that point.

Future idea:
- **Snapshot-only visual scrub mode**: Parse `Snapshot.<hook>` board-dump rows directly from `raw-*.jsonl` and render them on the Studio board even *after* a conversion truncation or engine divergence, with a one-click "Import this DCGO snapshot into Rust composer" button to reconstruct a late-game board state from a broken recording.

---

## 3. Branching Variation Trees & Cross-Checkpoint Comparison

In the MVP, the Studio supports a rewindable linear timeline, recording multiple named lines (`target`, `avoid`, `alternative`) starting from the same staged board, and inspecting policy + search valuations per step for the active pilot model(s).

Future ideas:
- **Chess-study style branching move tree**: Allow branching at *any* intermediate node in a recorded line, attaching per-node prose annotations, arrows/highlights, and alternative opponent replies.
- **Multi-checkpoint comparison matrix**: Select 2+ ONNX checkpoints in the Studio and view a side-by-side diff of policy probabilities, MCTS visit shares, and value-head curves along every step of a recorded line.

---

## 4. Desktop Tauri Distribution Wiring (`TauriStudioTransport`)

In the MVP, the frontend Studio UI is already isolated behind the `StudioTransport` interface and gated by `VITE_ENABLE_STUDIO` (default `false` on desktop builds, `true` on web dev/admin builds), keeping it tree-shaken out of shipping desktop builds while ready for optional client distribution.

Future work when opting into desktop distribution:
- Implement `TauriStudioTransport` in `code/frontend/src/api/` backed by Rust Tauri `invoke()` commands (`code/src-tauri/`) for staging, replay stepping, local library folder/file management, and ONNX/search evaluation.
- Wire native OS file dialogs for importing/exporting puzzles and replays and add an optional desktop settings toggle / build profile (`VITE_ENABLE_STUDIO=true`) to ship the Studio with the desktop client.

---

## 5. Direct Export to DCGO Oracle YAML Exams (`qa/dcgo-exams/`)

The MVP exports to `qa/puzzles/` (canonical), `qa/scenarios/` (Rust/Playwright fixtures), and `qa/search-exams/` (MCTS/policy grading). DCGO oracle exams (`qa/dcgo-exams/<SET>/<CARD-ID>.yaml`) cannot use arbitrary mid-game board injection because DCGO only supports turn-1 stacked decks (`decks.p0.stack`, `decks.p1.stack`) plus symbolic action steps.

Future idea:
- **Turn-1 symbolic recorder / DCGO exam exporter**: When a Studio session begins from a turn-1 stacked deck (or from a loaded DCGO recording), offer an export mode that emits symbolic `qa/dcgo-exams/` YAML with `steps[].do`, `expect.prompt`, and `clause:` binding.

---

## 6. Supervised Imitation Training & Curriculum Scheduling Pipelines

In the MVP, `studio-puzzle-v1` files store recorded `target` / `avoid` lines and wire up an RL `PuzzleResetWrapper` for episode resets from staged boards.

Future ideas:
- **Behavior-cloning / auxiliary policy-loss trainer**: Dedicated training script/wrapper that converts recorded `target` (positive) and `avoid` (negative/margin) lines from `qa/puzzles/` into supervised policy/value fine-tuning batches mixed into PPO/AlphaZero updates.
- **Adaptive curriculum scheduler**: Track per-puzzle solve rates during RL training and dynamically up-weight unsolved puzzles while retiring mastered ones.
- **CI / Nightly puzzle regression dashboard**: Aggregate pass rates across all `qa/puzzles/` and `qa/search-exams/` in the training monitor dashboard.

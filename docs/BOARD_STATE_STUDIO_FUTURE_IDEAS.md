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

## 5. Direct Export to DCGO Oracle YAML Exams (`qa/dcgo-exams/`) — **Shipped**

Implemented via `digimon_engine::playin::dcgo_yaml` (`compile_play_in` in `digimon-engine-mcp` and `puzzle-lab compile-play-in --out-dcgo-yaml`), emitting symbolic `dcgo-harness` ordinary mapping-form YAML (`decks: { p0: { stack, rest }, p1: { stack, rest } }` + `steps[].do`). See [`docs/PUZZLE_LAB_AND_MCP.md`](PUZZLE_LAB_AND_MCP.md).

---

## 6. Supervised Imitation Training & Curriculum Scheduling Pipelines — **Shipped**

`studio-puzzle-v1` files store recorded `target` / `avoid` lines, optional `refutation_triggers`, and wire up `PuzzleResetWrapper` (with `permute_staged_puzzle` slot/hand shuffling and `AdaptivePuzzleScheduler` EMA solve-rate weighting in `code/digimon_gym/agents/studio_puzzles.py`) plus `code/tools/emit_puzzle_shards.py` (emitting `selfplay-shards-v1` + contrastive `avoid` hinge edges for `azero_trainer.py` and `azero_lstm_distill.py`). See [`docs/PUZZLE_LAB_AND_MCP.md`](PUZZLE_LAB_AND_MCP.md).

Remaining UI/ops idea:
- **CI / Nightly puzzle regression dashboard**: Aggregate pass rates across all `qa/puzzles/` and `qa/search-exams/` in the training monitor dashboard.

---

## 7. Closed-Loop "Matchup Blind-Spot -> Puzzle -> Warm-Start" Daemon — **Shipped**

Implemented as the air-gapped sidecar `puzzle-lab auto-curriculum` (`code/tools/puzzle_lab/auto_curriculum.py`) paired with `--extra-shards` + `--max-extra-shard-share 0.10` across `azero_trainer.py`, `azero_lstm_distill.py`, `run_selfplay_generation.py`, and `specialist_league.py`:
- Ingests `anchored_eval` or `specialist-gate-v1` evaluations, selects bottom-$K$ matchups (`win_rate < 0.45`), harvests drills, and enforces the Mandatory Dual Gate (`verify_puzzle` + `plasticity_probe`).
- Learnable drills emit SHA256-stamped `selfplay-shards-v1` bundles (`curriculum_manifest.json`); unlearnable drills quarantine to `tensor_blindness_quarantine.json`. See [`docs/PUZZLE_LAB_AND_MCP.md`](PUZZLE_LAB_AND_MCP.md).

---

## 8. Guided "Play-In Compiler" (Staged Puzzles -> `closed-list` Beliefs, Warm LSTMs & DCGO Exams) — **Shipped**

Implemented in `code/digimon-engine/src/playin/` (`compile_play_in` MCP tool) and `code/tools/puzzle_lab/play_in.py` (`puzzle-lab compile-play-in`):
- Runs goal-directed Rust beam search (with optional `Role.AUTHOR` LLM macro-plan fallback) from Turn-1 Mulligan keeps over inverted-shuffle stacked decks to reach functional equivalence (or `--strict-zones`), remapping solver lines and upgrading puzzles to `setup.kind = "replay_prefix"` for full `closed-list` `KnowledgeLedger` compatibility, pre-warmed LSTM `(h, c)` state, and symbolic DCGO YAML export. See [`docs/PUZZLE_LAB_AND_MCP.md`](PUZZLE_LAB_AND_MCP.md).

---

## 9. Imperfect-Information & "Hand-Trap Discipline" Belief Puzzles — **Shipped**

Implemented via `verify_belief_puzzle` (`digimon-engine-mcp`), `search_exam --belief-gap`, `emit_puzzle_shards.py --belief-worlds W`, and reference fixtures `qa/puzzles/ex13-belief-hand-trap-discipline.json` + `qa/search-exams/ex13-belief-hand-trap-discipline.json`:
- Grades `closed-list` PIMC conditioned on `KnowledgeLedger` + `WeightedDeckPrior` against an omniscient `open-list` baseline (`information_gap`) and emits multi-world Bayes-averaged `policy_visits.npy` and continuous expected outcome `z.npy` (`z_bar in [-1, +1]`). See [`docs/PUZZLE_LAB_AND_MCP.md`](PUZZLE_LAB_AND_MCP.md).

---

## 10. "Continuation-Consistent" Critic Loss (Global Fix for Mid-Prompt Value Cliffs) — **Shipped**

Implemented across `code/digimon-engine/src/selfplay/driver.rs` (`--include-forced-selection-value`), `code/tools/emit_puzzle_shards.py`, `azero_trainer.py`, and `azero_lstm_distill.py`:
- Emits value-only rows (`policy_weight = 0.0`, `forced_selection.npy`) on choiceless `PendingSelection` steps (filtered out by default in `azero_lstm_distill.py` unless `--include-forced-selection` is set so LSTM recurrent step clocks stay aligned) plus `chain_anchor_idx.npy` across non-RNG single-mover chains trained with `--consistency-coef 0.25` MSE $(V(s_{\text{mid}}) - \operatorname{sg}(V(s_{\text{post}})))^2$. See [`docs/PUZZLE_LAB_AND_MCP.md`](PUZZLE_LAB_AND_MCP.md).

---

## 11. GRPO / LoRA Distillation into a Local 8B "Digimon Tactician" (RTX 4070) — **Shipped**

Implemented in `code/tools/tactician_grpo/` (`dataset_exporter.py`, 5-stage dense `verifiable_reward.py`, lazy `TRL` `SFTTrainer` + `GRPOTrainer` QLoRA `train_grpo.py`, and `export_ollama.py`):
- Defaults to Apache-2.0 `Qwen/Qwen3-8B` (`~5.2 GB` `Q4_K_M` GGUF co-resident in 12 GB VRAM on RTX 4070; `--base-model Qwen/Qwen3-14B` supported). Full operator runbook: [`docs/TACTICIAN_GRPO_RUNBOOK.md`](TACTICIAN_GRPO_RUNBOOK.md).

---

## 12. Automated Observation-Tensor Completeness & Card-Fidelity Fuzzing — **Shipped**

Implemented as strictly read-only tooling in `code/tools/puzzle_lab/tensor_fuzzer.py` (`puzzle-lab fuzz-tensor` & `puzzle-lab audit-set`):
- Differential zero-LLM state-pair fuzzing across 5 core rule families (`opt_spent_vs_unspent`, `delay_armed_vs_placing_turn`, `modifier_active_vs_absent`, `facedown_vs_faceup_source`, `linked_vs_unlinked`) + optional `plasticity_probe`, paired with LLM `GameEvent` card-set fidelity auditing (`audit-set`). See [`docs/PUZZLE_LAB_AND_MCP.md`](PUZZLE_LAB_AND_MCP.md).



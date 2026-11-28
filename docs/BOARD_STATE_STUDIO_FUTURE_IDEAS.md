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

`studio-puzzle-v1` files store recorded `target` / `avoid` lines, optional `refutation_triggers`, and wire up `PuzzleResetWrapper` (with `permute_staged_puzzle` slot/hand shuffling and `AdaptivePuzzleScheduler` EMA solve-rate weighting in `code/digimon_gym/agents/studio_puzzles.py`) plus `code/tools/emit_puzzle_shards.py` (emitting `selfplay-shards-v1` + contrastive `avoid` hinge edges for `azero_trainer.py` and `azero_lstm_distill.py`). See [`docs/PUZZLE_LAB_AND_MCP.md`](PUZZLE_LAB_AND_MCP.md).

Remaining UI/ops idea:
- **CI / Nightly puzzle regression dashboard**: Aggregate pass rates across all `qa/puzzles/` and `qa/search-exams/` in the training monitor dashboard.

---

## 7. Closed-Loop "Matchup Blind-Spot -> Puzzle -> Warm-Start" Daemon

Connect anchored/league evaluation, replay mining, white-box gating, and shard warm-starting into an automated between-generation daemon:
- After an `anchored_eval` or Specialist League generation finishes, automatically select the **bottom-K worst-performing matchups** and run `puzzle_lab mine-replays` on their highest non-RNG value drops and policy-vs-MCTS disagreements.
- Synthesize 10–20 engine-verified `studio-puzzle-v1` drills targeting those exact matchup blind spots.
- Gate every candidate drill with `digimon-training-mcp`'s `plasticity_probe`:
  - **Learnable drills** (`learnable: true`) automatically emit permuted shards mixed at `5–10%` share into the next generation's replay buffer.
  - **Unlearnable drills** (`observation_blindness`) quarantine automatically into a tensor/engine gap report rather than wasting RL timesteps.

---

## 8. Guided "Play-In Compiler" (Staged Puzzles -> `closed-list` Beliefs, Warm LSTMs & DCGO Exams)

Mid-game staged boards (`setup.staged`) are fast to compose, but start with zeroed LSTM `(h, c)` state, cannot reconstruct Turn-1 `KnowledgeLedger` history for `closed-list` determinized search (`docs/FAIR_BELIEF_TRAINING.md`), and cannot export directly to Turn-1 DCGO YAML exams (§5 above):
- Build a **Guided Play-In Compiler** (`compile_staged_to_replay`) that takes a composed `studio-puzzle-v1` board, stacks legal opening hands + `deck_top` orders for both seats so required pieces draw naturally over Turns 1–3, and runs a lightweight Rust beam search (or fast LLM driver) playing **100% legal Turn-1 actions** that arrive at the target staged board.
- **Triple payoff**: realistic pre-warmed LSTM hidden states, full `closed-list` `KnowledgeLedger` compatibility, and symbolic export to `qa/dcgo-exams/` YAML.

---

## 9. Imperfect-Information & "Hand-Trap Discipline" Belief Puzzles

Extend `studio-puzzle-v1` and `search_exam` beyond deterministic or fixed-hand interrupts to test **probabilistic opponent-hand reading** (`<Blast Digivolve>` Aces, security traps, unseen `[All Turns]` hand options):
- Define **Belief Puzzles** where Seat 2's public trash, breeding, and battle area telegraph a specific archetype while Seat 2's hand remains unpinned (`closed-list` regime conditioned on `WeightedDeckPrior`).
- Grade `closed-list` PIMC / IS-MCTS across $W$ sampled worlds (`e.g., W = 32`) to measure whether the model down-weights reckless swings into high-probability Ace windows or baits `[Once Per Turn]` / `<Counter>` timing appropriately compared to an omniscient `open-list` baseline.

---

## 10. "Continuation-Consistent" Critic Loss (Global Fix for Mid-Prompt Value Cliffs)

Multi-step single-card effects (`<Training>`, `<Delay>`, DNA material selection, multi-target prompts) often suffer intermediate value-head cliffs (e.g., `-0.76` mid-Training in `ex13-examon-training-before-wingdramon.json`) before the effect finishes resolving, starving MCTS visits before search reaches the post-resolution payoff:
- Use `score_line` traces across `qa/puzzles/` and self-play trajectories to identify single-mover committed resolution chains (`PendingSelection` sub-steps belonging to one initiated effect).
- Add an auxiliary **temporal-consistency critic loss** in `azero_trainer.py` / `azero_lstm_distill.py` penalizing $|V(s_{\text{pre-commit}}) - V(s_{\text{mid-selection}})|$ along committed resolution chains (alongside MCTS `macro_substeps` refinements), teaching the value head globally not to panic mid-prompt.

---

## 11. GRPO / LoRA Distillation into a Local 8B "Digimon Tactician" (RTX 4070)

Because `verify_puzzle` and `digimon-engine` provide an **exact, non-gameable ground-truth reward** (board legality, `target` win/goal satisfaction against adversary `[All Turns]` interrupts, and `avoid` refutation), Digimon puzzle authoring and tactical play are ideal for **Reinforcement Learning with Verifiable Rewards (GRPO)**:
- Distill engine-verified reasoning traces + `compose-boss-matrix` solves into an Apache-2.0 open-weight 8B/14B model (`Qwen3-8B` / `Qwen3-14B`) via QLoRA + GRPO on RunPod.
- Export quantized `Q4_K_M` weights to run locally via `Ollama` / `llama.cpp` on a single 12 GB RTX 4070—providing fast, zero-API-cost local Digimon card-interaction reasoning and puzzle synthesis.

---

## 12. Automated Observation-Tensor Completeness & Card-Fidelity Fuzzing

Combine `digimon-engine-mcp` (`inspect_stack`, `events`) and `digimon-training-mcp` (`plasticity_probe`) to audit newly admitted card sets automatically before launching large training runs:
- **Card-Fidelity Audit**: Prompt an LLM with printed `effect_text` / `inherited_text` from `data/cards.json` to stage minimal boundary cases and compare emitted `GameEvent`s against Comprehensive Rules, routing discrepancies to `qa/archetype-qa/engine-gaps.md`.
- **Observation-Tensor Completeness Fuzzing**: Construct pairs of legally distinct game states (e.g., `[Once Per Turn]` spent vs. unspent, active vs. expired `CannotUnsuspend`, face-down vs. face-up digivolution sources) where optimal actions differ, and assert `obs_a != obs_b` + `plasticity_probe.learnable == True`—automatically catching unencoded state features for future observation-profile revisions.


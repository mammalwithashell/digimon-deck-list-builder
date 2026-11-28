# Next-Wave Roadmap — Frozen Cross-Track Design & Contracts

Builds on PR #814 to ship the six next-wave capabilities in a single coordinated PR across four strictly file-disjoint tracks.

---

## 1. Frozen Cross-Track Data & Tool Contracts

### 1.1 Shard Array Extensions (`selfplay-shards-v1` Backward-Compatible Sidecars)
Optional `.npy` files inside each `shard-NNN/` directory (when absent, trainers preserve 100% existing behavior):

- **`forced_selection.npy`** (`(N,) uint8` / `bool`):
  - `1` when a row was emitted at a choiceless (`legal.len() == 1`) `PendingSelection` node (`policy_weight == 0.0`, value target only); `0` on normal multi-action decisions.
  - `azero_trainer.py` (MLP): includes `forced_selection == 1` rows in value MSE loss (and continuation-consistency loss) while `policy_weight == 0.0` excludes them from policy CE.
  - `azero_lstm_distill.py` (LSTM): filters out `forced_selection == 1` rows by default (preserving exact non-forced recurrent sequence step alignment with `OnnxLstmPolicy` / `selfplay`), unless `--include-forced-selection` is explicitly passed.
- **`chain_anchor_idx.npy`** (`(N,) int64`):
  - Local shard row index `j >= i` (`0 <= j < N`) of the post-resolution (or final-step) anchor row for a single-mover, non-RNG `PendingSelection` resolution chain, or `-1` when the row is not an intermediate step of a tracked chain.
  - Continuation-consistency loss in `azero_trainer.py` and `azero_lstm_distill.py` (controlled by `--consistency-coef`, default `0.25`; `0.0` disables):
    `L_cc = consistency_coef * mean_{i: anchor[i] >= 0} (V(obs_i) - stopgrad(V(obs_{anchor[i]})))^2`.
- **Belief-Puzzle Continuous `z.npy`**:
  - `z.npy` (`(N,) float32`) already stores `float32`; multi-world belief shards store expected value `z_bar in [-1.0, +1.0]` across `W` sampled `closed-list` worlds alongside Bayes-averaged `policy_visits.npy`.
- **Extra-Shard Cap (`--extra-shards` & `--max-extra-shard-share 0.10`)**:
  - `azero_trainer.py`, `azero_lstm_distill.py`, and `specialist_league.py` support `--extra-shards <dir...>` and `--max-extra-shard-share` (default `0.10`). When extra puzzle/curriculum shards exceed `max_share / (1 - max_share)` of primary self-play rows, extra rows are deterministically subsampled (seeded) so auxiliary drills never exceed `10%` of training batches.

---

### 1.2 `digimon-engine` & `digimon-engine-mcp` New Tools (Track A)

1. **Guided Play-In Compiler (`digimon_engine::playin` + MCP `compile_play_in`)**:
   - Goal-directed Rust beam search (`beam_width`, `max_turns`) over stacked opening decks (`decks.1`, `decks.2`) from Turn-1 Mulligan (`keep`) to match a staged `studio-puzzle-v1` board under **functional equivalence** (exact target `field` stacks + `is_suspended`, `breeding` stack, required `hand` & `trash` subsets, `security_count`, `phase`, `current_player`, and `memory`; `--strict-zones` requires exact `hand` and `trash` multisets).
   - Verifies that any `target` and `avoid` lines on the input puzzle remain legal (remapping hand indices if inert filler cards shifted positions) and pass `verify_puzzle`.
   - Symbolic DCGO exporter emits `dcgo-harness`-compatible YAML (`decks: { p0: { stack, rest }, p1: { stack, rest } }` + `steps: [{ do: ... }]` per `code/tools/dcgo-harness/src/exam/scenario.rs`).
   - **MCP Tool `compile_play_in`**:
     - Input: `{ puzzle_path?, puzzle_json?, beam_width=32, max_turns=5, strict_zones=false, out_puzzle_path?, out_dcgo_yaml_path?, rest_deck_p0="custom", rest_deck_p1="custom" }`
     - Output: `{ ok: bool, matched: bool, strict_zones: bool, prefix_actions: [u16...], stacked_decks: { "1": [...], "2": [...] }, remapped_lines: [...], dcgo_yaml?: string, out_puzzle_path?, out_dcgo_yaml_path?, distance_score: f64, error?: string }`

2. **Belief Puzzle Verifier (`verify_belief_puzzle` in MCP & `--belief` / `prior` in `search_exam.rs`)**:
   - Evaluates an imperfect-information puzzle (`studio-puzzle-v1` with top-level `"prior"` snapshot or `search-exam-v1` with `"regime": "closed-list"` and `"prior"`) across `worlds` sampled `closed-list` determinizations conditioned on `KnowledgeLedger` + `WeightedDeckPrior`, AND compares against `open-list` omniscient worlds to report the **Information Gap**.
   - **MCP Tool `verify_belief_puzzle`**:
     - Input: `{ puzzle_path?, puzzle_json?, model_id?, simulations=128, worlds=16, seeds=4 }`
     - Output: `{ ok: bool, passed: bool, closed_value_mean: f32, open_value_mean: f32, information_gap: f32, closed_visit_shares: [{ plan: [u16...], label: string, mean_share: f64, mean_q: f32 }], expectation_results: [...] }`

3. **Self-Play Forced Selection & Chain Anchor Emission (`selfplay/driver.rs` + `digimon-engine-cli`)**:
   - Add `include_forced_selection_value: bool` (default `false`, CLI flag `--include-forced-selection-value`) to `SelfPlayConfig`: when `true` and `legal.len() == 1` while `game.pending_selection.is_some()`, emit a value-only record (`policy_weight = 0.0`, `forced_selection = 1`).
   - Emit `forced_selection.npy` (`u8`) and `chain_anchor_idx.npy` (`i64`) in every flushed shard directory (recording post-chain anchor row index within the same seat/game when no random draw/reveal/shuffle event occurred during the `PendingSelection` chain, else `-1`).

---

### 1.3 `tools.puzzle_lab` Extensions (Track C)
- `play_in.py`: Python wrapper driving MCP `compile_play_in` + optional `Role.AUTHOR` LLM macro-turn hint fallback when pure Rust beam search cannot reach a complex `<Delay>`/effect setup on Attempt 1 (`puzzle-lab compile-play-in`).
- `tensor_fuzzer.py`:
  - Zero-LLM deterministic differential state-pair fuzzer across core rule families (`opt_spent_vs_unspent`, `delay_armed_vs_placing_turn`, `modifier_active_vs_absent`, `facedown_vs_faceup_source`, `linked_vs_unlinked`) asserting `||obs_A - obs_B||_1 > 0` + optional `plasticity_probe` learnability (`puzzle-lab fuzz-tensor`).
  - LLM card-event fidelity mode (`puzzle-lab audit-set --set <SET>`), strictly read-only over active tensor contracts, outputting structured markdown/JSON gap reports.
- `auto_curriculum.py`:
  - Air-gapped sidecar step (`puzzle-lab auto-curriculum`) ingesting `anchored_eval` or `specialist-gate-v1` summaries + recordings, selecting bottom-`K` matchups, running replay mining / boss-matrix synthesis, gating every candidate with `verify_puzzle` + `plasticity_probe` (quarantining `observation_blindness` cases to `tensor_blindness_quarantine.json`), and invoking `emit_puzzle_shards` to write a SHA256-stamped `selfplay-shards-v1` bundle ready for `--extra-shards`.

---

### 1.4 `tools.tactician_grpo` Pipeline (Track D)
- Standalone dev package `code/tools/tactician_grpo/` (lazy-importing `trl`, `peft`, `transformers`, `datasets` so base CI runs with zero heavy LLM-training deps):
  - `dataset_exporter.py`: builds SFT chat-completion JSONL + GRPO prompt JSONL from `qa/puzzles/`, `qa/search-exams/`, `score_line` traces, and `docs/RULES_CONTEXT.md`.
  - `verifiable_reward.py`: 5-stage dense engine reward in `[0.0, 1.0]` (`+0.15` valid JSON schema, `+0.15` card admissibility & deck copy purity, `+0.20` `RustDebugGame.validate()`, `+0.35` `verify_puzzle` `target` win against `[All Turns]` interrupts, `+0.15` `avoid` refuted).
  - `train_grpo.py`: RunPod `TRL` `SFTTrainer` + `GRPOTrainer` (`peft` QLoRA) defaulting to `Qwen/Qwen3-8B` (`--base-model Qwen/Qwen3-14B` supported).
  - `export_ollama.py` + `docs/TACTICIAN_GRPO_RUNBOOK.md`: generates `Modelfile` (`Q4_K_M`, temperature/system-prompt tuned for `tools.puzzle_lab --provider ollama`) for single-command deployment on a 12 GB RTX 4070.

# ModelPilot + LLM Tactical Puzzle Lab & MCP Co-Pilot — Design Spec

## 1. Alignment with PR #813 (`add-board-state-studio`)

PR #813 (`add-board-state-studio`) landed the foundational Studio and staged-exam substrate:
- **Canonical Puzzle Schema (`studio-puzzle-v1` in `qa/puzzles/`)**: Stores `decks`, `seed`, `seat`, `setup` (`kind: "staged"` | `"replay_prefix"`), `lines` (`role: "target" | "avoid" | "alternative"`), `expectations` (`visits`, `avoid`, `choose`), `assertions`, and `rl_goal` (`playout_to_win` | `bounded_horizon`), with export to `qa/scenarios/` and `qa/search-exams/` (`setup.staged` via `Game::apply_scenario`).
- **Engine & PyO3 Valuation Primitives**: `Game::apply_scenario(&Value)`, `skip_mulligan()`, `stage_set_security_face_up()`, linked-card staging, and PyO3 `digimon_engine.inspect_position(...)` (`policy_value`, `policy_actions`, `search_value`, `root_plans`, `expectation_results`).
- **Visual Studio (`/studio`) & RL Reset MVP**: React `StudioPage` (`AgentInspectorPanel`, `StackInspector`, `PuzzleSaveExportModal`) plus `code/digimon_gym/agents/studio_puzzles.py` (`StudioPuzzle`, `PuzzleResetWrapper`).
- **Confirmed Baseline Exam**: `qa/search-exams/ex13-imperialdramon-dna-paildramon-jamming-lethal.json` (`BT12-022` ExVeemon + `BT12-050` Stingmon -> DNA `BT16-025` Paildramon past `EX13-024` Slayerdramon Blocker, `[When Attacking]` unsuspend, swing for lethal).

### Scope of This Spec on Top of PR #813
Every component below adopts **`studio-puzzle-v1`** (`qa/puzzles/`) and PR #813's `inspect_position` JSON contract directly:

1. **Bring Studio + ONNX + MCTS to `digimon-engine-mcp` (Phase 1A)**: Factor shared Rust `inference::inspector` (supporting both MLP and LSTM + `.value.onnx` sidecars); add cached `ModelRegistry`, `inspect_position`, `score_line`, stack-aware `inspect_stack`, `load_puzzle` / `save_puzzle`, and `mode: "author" | "play"` (`Perspective` seat-lock + auto-opponent) into `code/digimon-engine-mcp/`. Puzzles authored in MCP save directly to `qa/puzzles/` and open immediately in `/studio`.
2. **Adversary `[All Turns]` Interrupt Verification (Phase 1B)**: Treat every `decision_player != solver` step during multi-step lines—such as `EX12-034` Erlangmon (`WhenWouldLeave` free-play + bounce) + `EX12-071` Saneiketsu Invitation (`<Delay>` free Lv.6 digivolve)—as an adversary node backed by bounded MCTS/minimax + optional `refutation_triggers`.
3. **White-Box PyTorch `.zip` Diagnostics in `digimon-training-mcp` (Phase 1C)**: Extend Python `digimon-training-mcp` with lazy-loaded, read-only PyTorch inspection over SB3 `.zip` checkpoints (`card_embedding_neighbors`, `position_saliency`, `plasticity_probe`, and cache-backed `export_checkpoint_onnx`), keeping Rust `digimon-engine-mcp` pure ONNX while unlocking gradient attribution, embedding health checks, and 2-second tensor-blindness probes.
4. **Fulfill `BOARD_STATE_STUDIO_FUTURE_IDEAS.md` §6 + Permutation Augmentation (Phase 2)**: Extend `studio_puzzles.py` with hand/field slot-permutation augmentation, wire `PuzzleResetWrapper` + adaptive solve-rate curriculum scheduling into `pilot_training.py`, and add `emit_puzzle_shards.py` turning `target` (positive) and `avoid` (negative margin) lines into `selfplay-shards-v1` warm-start shards.
5. **Dev-Only Multi-Provider LLM CLI (Phase 3)**: Headless `code/tools/puzzle_lab/` driver pairing closed dev APIs (`Gemini 4 Neon` + `Hydrogen`) and open-weight endpoints (`Ollama` on RTX 4070 / `vLLM` on RunPod) with both MCP servers.

> **Service & Licensing Boundary**: Closed-weight LLM SDKs/keys (`Neon` / `Hydrogen`) are strictly dev-only inside `code/tools/puzzle_lab/` (or external MCP chat clients). Production surfaces (`code/server/`, `code/digimon_gym/`, and `code/src-tauri/`) never import closed-weight LLM clients.

---

## 2. Frozen Cross-Track Contracts

### 2.1 `studio-puzzle-v1` Optional `refutation_triggers` Extension
Optional top-level array on `studio-puzzle-v1` (and optional top-level array on `search-exam-v1` when exported):

```json
"refutation_triggers": [
  {
    "source_card_id": "EX12-034",
    "phase": null,
    "accept_optional": true,
    "prefer_label_contains": "Play"
  },
  {
    "source_card_id": "EX12-071",
    "phase": null,
    "accept_optional": true,
    "prefer_label_contains": null
  }
]
```

Matching rules when `decision_player != solver_seat`:
1. Inspect legal decoded actions (`legal_decoded_actions` / `pending_selection`).
2. If a rule's `phase` (when non-null) matches `current_phase` and `source_card_id` (when non-null) matches an action's `card_id` (or the pending prompt's source card):
   - If `prefer_label_contains` is set and matches a non-pass legal action's `label` (case-insensitive), choose the first matching action.
   - Else if `accept_optional` is `true`, choose the first legal non-pass (`action_id != 62`) action.
   - Else if `accept_optional` is `false` and pass/decline (`62`) is legal, choose `62`.

### 2.2 `digimon-engine-mcp` New Tool Contracts
- `load_model({ model_id, onnx_path, profile? })` -> `{ ok: true, model_id, kind: "mlp"|"lstm", profile, value_semantics }`
- `unload_model({ model_id })` -> `{ ok: true, removed: bool }`
- `list_models({})` -> `{ ok: true, models: [{ model_id, onnx_path, kind, profile, value_semantics }] }`
- `inspect_position({ game_id, model_id, simulations=0, worlds=1, regime="open_list", seed=0, top_k=12, expectations? })` -> matches `digimon_engine.inspect_position` (`mode`, `viewer`, `policy_value`, `policy_entropy`, `policy_actions`, `search_value`, `simulations_run`, `chosen_action_id`, `chosen_plan`, `root_plans`, `root_q`, `expectation_results`)
- `score_line({ game_id, model_id, actions, adversary_sims=0, refutation_triggers? })` -> `{ ok: true, root_mover, steps: [{ step_index, mover, phase, action_id, label, auto_adversary: bool, policy_value_for_root_mover, prior_prob, events_emitted }], final_winner }`
- `inspect_stack({ game_id, player, zone="battle"|"breeding", slot=0 })` -> `{ ok: true, top_card: { card_id, card_name, level, dp_printed, dp_effective, colors, traits, effect_text }, inherited_sources: [{ index, card_id, card_name, face_down, inherited_text }], linked_cards: [{ card_id, card_name, link_text, dp_bonus }], is_suspended, turn_played, modifiers }`
- `load_puzzle({ puzzle_path?, puzzle_json?, mode="author"|"play", opponent_driver? })` -> `{ ok: true, game_id, puzzle_id, seat, mode }`
- `save_puzzle({ game_id, puzzle_path, id, title, description?, folder?, tags?, seat?, lines?, expectations?, assertions?, rl_goal?, refutation_triggers?, export_search_exam_path? })` -> `{ ok: true, puzzle_path, search_exam_path? }`
- `verify_puzzle({ puzzle_path?, puzzle_json?, model_id?, adversary_sims=64, seeds=4 })` -> `{ ok: true, target_results: [...], avoid_results: [...], expectation_results: [...] }`

### 2.3 `digimon-training-mcp` New White-Box Tool Contracts
- `card_embedding_neighbors({ checkpoint_path, card_ids, top_k=10 })` -> `{ ok: true, cards: [{ card_id, card_name, registry_index, l2_norm, neighbors: [{ card_id, card_name, cosine_similarity, level, colors, traits }] }], pairwise_cosine }`
- `position_saliency({ checkpoint_path, puzzle_path?, scenario_json?, target_action_id?, avoid_action_id?, top_k=15 })` -> `{ ok: true, objective: "value"|"logit_margin", base_value, margin?, section_attribution: [{ section, l1_grad_x_input, l1_grad }], top_features: [{ index, section, card_id?, grad_x_input, grad, obs_value }] }`
- `plasticity_probe({ checkpoint_path, puzzle_path?, scenario_json?, target_action_id, avoid_action_ids?, steps=10, lr=0.0001 })` -> `{ ok: true, pre: { target_prob, value, margin }, post: { target_prob, value, margin }, loss_start, loss_end, learnable: bool, diagnosis }`
- `export_checkpoint_onnx({ checkpoint_path, cache_dir? })` -> `{ ok: true, onnx_path, value_onnx_path?, meta_path, model_type: "mlp"|"lstm", sha256 }`

### 2.4 Parameterized Boss-Board × Challenger Archetype Counter-Matrix (`compose-boss-matrix`)
Rather than hand-authoring one puzzle per matchup, `code/tools/puzzle_lab/boss_matrix.py` fixes a parameterized **Boss-Board Template** on Seat 2 (e.g., `ex12-erlangmon-invitation`: `EX12-034` Erlangmon + `[Kotenken]` Blocker Token + `EX12-071` Saneiketsu Invitation `<Delay>` on field, Lv.5 `[SW]` + Lv.6 `[Saneiketsu]` in hand/sources, plus `refutation_triggers`) and sweeps knob axes (`security_counts = [1, 2]`, `memories = [2, 4]`) across Challenger archetypes from `data/deck_library.json` or `data/training_pools/ex13-superwide-20261127/`:
1. For each `(challenger_archetype, security_count, memory)` cell, `Role.AUTHOR` reads Challenger's real decklist + card texts, composes a legal mid-game Seat 1 setup (`hand`, `field`, `breeding`, `trash`), and proposes Challenger's archetype-specific `target` answer (either a clean lethal/bypass at 1 security, or a disciplined setup/Option-trash/OPT-bait bounded-horizon line at 2 security) alongside the `avoid` trap-triggering punt.
2. `verify_puzzle` tests the proposed `target` against the Boss board's `[All Turns]` interrupt refutation; if Erlangmon + Saneiketsu Invitation refutes Attempt 1, the exact refutation trace feeds back to `Role.AUTHOR` (up to `max_retries`) until an engine-verified solve is confirmed.
3. Verified cells emit `qa/puzzles/<matrix_folder>/<boss>-sec<S>-mem<M>-vs-<slug>.json` and optionally evaluate Generalist/Specialist checkpoints across the entire `(archetype × security × memory)` matrix.


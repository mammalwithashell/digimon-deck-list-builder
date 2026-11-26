# EX13 Small-Pool Hybrid Search ↔ LSTM PPO Campaign (2026-11-26)

**Branch / Commit:** `feat/small-pool-hybrid-search-lstm-ppo` (based on `main` @ `53bd2736`)  
**Frozen Pool:** [`data/training_pools/ex13-small3/manifest.json`](../../../data/training_pools/ex13-small3/manifest.json) — **Examon**, **TS Jupitermon**, **Three Musketeers** (`6` training lists + `6` strictly disjoint held-out lists, zero card substitutions)  
**Contracts Preserved:** `standard_lite_v2` (`9,434` floats, feature schema `standard_lite_v2.4`), `ACTION_SPACE_SIZE = 2192` (action schema `2`), **zero truncations** across all `4,050` self-play training games, `6,840` held-out policy evaluation games, and `720` held-out `320/4` searched evaluation games.

---

## Executive Summary

This experiment tested bidirectional coupling between **Macro-Substep PIMC-MCTS** (`320` simulations / `4` worlds / `--macro-substeps 16`) and **KL-Anchored Clean-Reward `MaskableRecurrentPPO`** (`terminal_strategic_v1`, `kl_coef = 0.03`, `AutoAdvanceForcedWrapper`):

1. **Sequence-distilled, KL-anchored LSTM PPO (`lstm_ppo_r1`) was the strongest standalone policy in the campaign:**
   - **`99.7%`** (`359–1–0`, 95% CI `[99.2%, 100.0%]`) vs `greedy` and **`76.9%`** (`277–83–0`, 95% CI `[72.8%, 81.1%]`) vs `development_v1` on unseen held-out decklists.
   - Beat `Gen3` (`59.7%`, `[54.7%, 64.7%]`), `lstm_distilled_r1` (`60.8%`, `[55.8%, 65.8%]`), and `Gen6_hybrid` (`58.1%`, `[52.8%, 63.1%]`) head-to-head without search, with the largest edge in **Three Musketeers** (`70.8%` vs `Gen6_hybrid`) and higher multi-turn stack building (`5.59–5.84` digivolutions/game).
2. **Online `320/4` search remains strongly complementary (`+38.1%` lift):**
   - Pairing `Gen6_hybrid` with `320/4` PIMC-MCTS flipped its matchup against policy-only `lstm_ppo_r1` from `41.9%` to **`80.0%`** (`144–36–0`, 95% CI `[74.4%, 85.6%]`).
3. **Hybrid sparring against `lstm_ppo_r1` prevented closed-pool value collapse on the Examon `<Training>` search exam:**
   - Under `qa/search-exams/ex13-examon-training-before-wingdramon.json` (`--macro-substeps 0`), `Gen6_control` (pure search) suffered a value-head collapse (`-0.86` root value, **`0/8`** seeds passing `[1002] >= 10%` visits), whereas `Gen6_hybrid` (25% sparring vs `lstm_ppo_r1`) preserved healthy value calibration (`+0.68 -> +0.95`, **`8/8`** seeds passing).
4. **Root-cause fix for a pre-existing PIMC determinization bug:**
   - `canonicalize_hidden` in [`code/digimon-engine/src/search/determinized.rs`](../../../code/digimon-engine/src/search/determinized.rs) previously sorted concealed zones without calling `canon.rebase_main_declare_guard_for_world(root)`, dropping `refund_opt`-parked `[Main]` declarations in `open_decklist` worlds. Fixing this and seeding `ref_legal` from the root mask eliminated the failure across `4,700+` subsequent searched games.

---

## 1. Experimental Pipeline & Seams Added

Orchestrated by [`code/tools/run_hybrid_small_campaign.py`](../../../code/tools/run_hybrid_small_campaign.py):

- **Stage 0 (`stage0_bootstrap`):** `192` `mixed_v2` teacher games (`greedy` + `development_v1`) → `Gen0` `[256, 256]` MLP (`seed.{zip,onnx}`).
- **Stage 1 (`stage1_search_base`):** `Gens 1–3` (`3 × 450 = 1,350` games, `320/4`, `--macro-substeps 16`, `--reduced-sims 64 --full-search-fraction 0.25 --cheap-sims 64`) → `Gen3`.
- **Stage 2 (`stage2_lstm_distill`):** Chronological `(game_id, mover)` sequence distillation ([`code/digimon_gym/agents/azero_lstm_distill.py`](../../../code/digimon_gym/agents/azero_lstm_distill.py)) of `Gen1–3` shards (`113,233` rows = `2,700` seat trajectories, best held-out epoch `3/6`, `policy_ce = 0.9409`, `value_mse = 0.6756` vs `0.9655` constant) → `lstm_distilled_r1.{zip,onnx}`.
- **Stage 3 (`stage3_lstm_ppo`):** `500,000` non-forced steps (`AutoAdvanceForcedWrapper`, `terminal_strategic_v1` clean unit terminal reward, forward masked KL anchor `kl_coef = 0.03` toward `lstm_distilled_r1`, steady `anchor_kl ≈ 0.19–0.22`) vs `Gen1–3` ONNX pool (`PFSP`) → `lstm_ppo_r1.{zip,onnx}`.
- **Stage 4 (`stage4_hybrid` vs `stage4_control`):** Matched `3 × 450 = 1,350`-game arms from `Gen3` with identical seeds and `selfplay_probability = 0.5`:
  - **Hybrid (`Gen6_hybrid`):** `50%` self-play + `25%` `lstm_ppo_r1` (`OpponentMode::PolicyLstm`, `weight = 2.0`, per-game state reset) + `25%` rolling 2-gen search history (`weight = 1.0` each). Exact `326/326` learner opening-seat balance across non-current games.
  - **Control (`Gen6_control`):** `50%` self-play + `50%` rolling 3-gen search history (no LSTM). Exact `326/326` learner opening-seat balance.
- **Stage 5 (`stage5_evaluation`):** Seat-balanced unseen held-out evaluation on `heldout_snapshot.json` / `heldout_decks.json` with paired play/draw seeds and 95% paired bootstrap confidence intervals.

---

## 2. Unseen Held-Out Policy-Only Evaluation (`360` Games / Matchup)

All numbers below are from `180` seat-balanced play/draw pairs (`120` games per candidate archetype) on the `6` unseen held-out decklists, with no test-time search.

### 2.1 Heuristic Anchors

| Checkpoint | Architecture | vs `greedy` WR (95% Paired CI) | vs `development_v1` WR (95% Paired CI) | Mean Turns (vs `dev_v1`) | Mean Agent Digivolves |
|---|---|---:|---:|---:|---:|
| **`Gen0`** | `[256, 256]` MLP | `66.9%` (`[61.9%, 71.9%]`, `241–119`) | `46.4%` (`[41.4%, 51.4%]`, `167–193`) | `10.47` | `2.96` |
| **`Gen3`** | `[256, 256]` MLP | `94.4%` (`[91.9%, 96.7%]`, `340–20`) | `71.1%` (`[66.4%, 75.8%]`, `256–104`) | `9.25` | `4.49` |
| **`lstm_distilled_r1`** | `[256, 256]` + `256` LSTM | `96.1%` (`[94.2%, 98.1%]`, `346–14`) | `70.8%` (`[65.8%, 75.6%]`, `255–105`) | `9.01` | `4.71` |
| **`lstm_ppo_r1`** | `[256, 256]` + `256` LSTM | **`99.7%`** (`[99.2%, 100.0%]`, `359–1`) | **`76.9%`** (`[72.8%, 81.1%]`, `277–83`) | `9.31` | **`5.33`** |
| **`Gen6_control`** | `[256, 256]` MLP | `97.2%` (`[95.3%, 98.9%]`, `350–10`) | `73.9%` (`[69.4%, 78.3%]`, `266–94`) | `9.18` | `4.90` |
| **`Gen6_hybrid`** | `[256, 256]` MLP | `97.2%` (`[95.3%, 98.9%]`, `350–10`) | `70.8%` (`[65.8%, 75.8%]`, `255–105`) | `9.10` | `4.72` |

#### Per-Archetype Held-Out Win Rates vs `development_v1` (`120` Games / Cell)

| Checkpoint | Examon | TS Jupitermon | Three Musketeers |
|---|---:|---:|---:|
| **`Gen0`** | `42.5%` (`51–69`) | `56.7%` (`68–52`) | `40.0%` (`48–72`) |
| **`Gen3`** | `70.0%` (`84–36`) | `76.7%` (`92–28`) | `66.7%` (`80–40`) |
| **`lstm_distilled_r1`** | `72.5%` (`87–33`) | `75.8%` (`91–29`) | `64.2%` (`77–43`) |
| **`lstm_ppo_r1`** | **`75.0%`** (`90–30`) | **`80.0%`** (`96–24`) | **`75.8%`** (`91–29`) |
| **`Gen6_control`** | `73.3%` (`88–32`) | `78.3%` (`94–26`) | `70.0%` (`84–36`) |
| **`Gen6_hybrid`** | `73.3%` (`88–32`) | `75.0%` (`90–30`) | `64.2%` (`77–43`) |

### 2.2 Policy-Only Head-to-Heads (`360` Games Each)

| Matchup (`Candidate` vs `Opponent`) | Candidate WR | 95% Paired CI | W–L–D | Examon | TS Jupitermon | Three Musketeers | Digivolves (Cand vs Opp) |
|---|---:|---:|---:|---:|---:|---:|---:|
| **`Gen3` vs `Gen0`** | **`68.6%`** | `[63.6%, 73.3%]` | `247–113–0` | `69.2%` | `80.8%` | `55.8%` | `5.08` vs `3.33` |
| **`lstm_ppo_r1` vs `lstm_distilled_r1`** | **`60.8%`** | `[55.8%, 65.8%]` | `219–141–0` | `58.3%` | `70.0%` | `54.2%` | `5.64` vs `5.14` |
| **`lstm_ppo_r1` vs `Gen3`** | **`59.7%`** | `[54.7%, 64.7%]` | `215–145–0` | `56.7%` | `68.3%` | `54.2%` | `5.59` vs `4.91` |
| **`Gen6_hybrid` vs `lstm_ppo_r1`** | `41.9%` *(LSTM `58.1%`)* | `[36.9%, 47.2%]` | `151–209–0` | `40.0%` | `56.7%` | `29.2%` | `5.49` vs **`5.84`** |
| **`Gen6_control` vs `Gen3`** | **`54.7%`** | `[49.4%, 60.0%]` | `197–163–0` | `45.8%` | `70.8%` | `47.5%` | `5.48` vs `4.92` |
| **`Gen6_hybrid` vs `Gen3`** | **`53.9%`** | `[48.6%, 59.2%]` | `194–166–0` | `51.7%` | `68.3%` | `41.7%` | `5.19` vs `4.71` |
| **`Gen6_hybrid` vs `Gen6_control`** | **`51.7%`** | `[46.4%, 56.9%]` | `186–174–0` | `36.7%` | `62.5%` | `55.8%` | `5.42` vs `5.60` |

---

## 3. Unseen Held-Out `320/4` Searched Head-to-Heads (`180` Games = `90` Pairs Each)

Evaluated via `digimon-engine-cli` (`320` sims, `4` worlds, `eval_batch_size = 4`, `--macro-substeps 16`, `--reduced-sims 64`, `temperature = 0`) on `heldout_decks.json`.

| Matchup (`Seat A` vs `Seat B`) | Seat A Mode | Seat B Mode | Seat A WR | 95% Paired CI | W–L–D | Trunc | Examon | TS Jupitermon | Three Musketeers |
|---|---|---|---|---:|---:|---:|---:|---:|---:|
| **`Gen6_hybrid` vs `lstm_ppo_r1`** | `320/4` Search | `policy_lstm` | **`80.0%`** | `[74.4%, 85.6%]` | `144–36–0` | `0` | `66.7%` | `83.6%` | `87.1%` |
| **`Gen6_hybrid` vs `Gen6_control`** | `320/4` Search | `320/4` Search | **`50.6%`** | `[42.8%, 58.3%]` | `91–89–0` | `0` | `42.6%` | `62.5%` | `45.8%` |
| **`Gen6_control` vs `Gen3`** | `320/4` Search | `320/4` Search | `48.3%` | `[41.7%, 55.0%]` | `87–93–0` | `0` | `41.2%` | `68.5%` | `37.9%` |
| **`Gen6_hybrid` vs `Gen3`** | `320/4` Search | `320/4` Search | `47.2%` | `[39.4%, 55.0%]` | `85–95–0` | `0` | `42.2%` | `58.3%` | `38.1%` |

---

## 4. Examon `<Training>` Search Exam (`qa/search-exams/ex13-examon-training-before-wingdramon.json`)

Graded with `search_exam` (`8` seeds, `320` sims, `4` worlds):
- Expectation 1: Activate `Sprint Dash Training` `[Main]` (`[1002]`) gets `>= 10%` of root visits.
- Expectation 2: Hard-playing `Wingdramon` from hand (`[4]`) is not chosen.

| Checkpoint | `--macro-substeps 0` (`[1002]`, `[4]`) | `--macro-substeps 16` (`[1002]`, `[4]`) | Value Trajectory Along `training_line` |
|---|---:|---:|---|
| **`Gen0`** | `8/8`, `8/8` | `4/8`, `8/8` | `+0.02 → +0.09 → -0.13 → +0.15 → -0.39 → -0.10 → -0.00 → +0.29 → -0.60` |
| **`Gen3`** | **`8/8`, `8/8`** | `4/8`, `8/8` | `+0.45 → +0.67 → +0.64 → +0.50 → +0.74 → +0.95 → +0.95 → +0.93 → +0.15` |
| **`Gen6_control`** | **`0/8`**, `8/8` | **`0/8`**, `8/8` | `-0.86 → -0.50 → -0.36 → -0.65 → -0.41 → -0.38 → -0.38 → -0.05 → -0.36` |
| **`Gen6_hybrid`** | **`8/8`, `8/8`** | `2/8`, `8/8` | `+0.68 → +0.89 → +0.92 → +0.76 → +0.95 → +0.84 → +0.81 → +0.92 → +0.04` |

**Diagnostic Takeaways:**
- Closed pure-search self-play (`Gen6_control`) collapsed Examon's value head to `-0.86`, failing `[1002]` on `8/8` seeds; sparring against `lstm_ppo_r1` (`Gen6_hybrid`) kept the curve positive (`+0.68 -> +0.95`) and preserved `8/8` pass rate at `macro=0`.
- `--macro-substeps 16` underperformed `--macro-substeps 0` on `[1002]` because bundling multi-option `SelectReveal` substeps with prior sampling adds noise to `[1002]`'s Q-estimate compared to expanding `SelectReveal` explicitly in UCT. Future macro bundling should restrict auto-advance to forced (`1`-action) transitions or greedy-argmaxed sub-selections.

---

## 5. Compute & Next Architectural Steps (Implemented in PR #810)

- **Throughput:** `~250–277` games/hr on a 2-vCPU Xeon vs **`~2,100–2,470` games/hr** on a 16-vCPU Hetzner `ccx43` (`12.6–16.7 min` per 450-game generation including PyTorch fitting + ONNX export; `0` truncations).
- **Implemented Next-Iteration Upgrades (ready for the full EX13 pool):**
  1. **Recurrent LSTM inside Rust MCTS (`BatchedLstmPolicyValueEvaluator`):** `PolicyValueFn` now supports opt-in per-player `(h_pi, c_pi, h_vf, c_vf)` recurrent state threading through `Mcts::search`, `Mcts::search_batch`, `pimc_search`, `selfplay/driver.rs`, `digimon-engine-cli`, and `search_exam.rs`, using actor `<stem>.onnx` + companion critic `<stem>.value.onnx` (with dynamic batch axis `1` on hidden states in `export_onnx.py` and automatic batch-1 fallback for legacy exports).
     - *Live `search_exam` verification on `lstm_ppo_r1.onnx` (`4` seeds, `320/4`, `--macro-substeps 16 --macro-substep-min-prior 0.85`):* **`4/4` on `[1002]`** (`15%–20%` of visits) and **`4/4` avoiding `[4]`**, with a smoothly calibrated non-saturated value curve (`+0.25 → +0.27 → +0.33 → +0.28 → +0.33 → +0.45 → +0.38 → +0.37 → +0.01`, vs `+0.25 → +0.15 → -0.06` on `wingdramon_from_hand`).
  2. **Confidence-Gated Macro-Substeps (`--macro-substep-min-prior`):** Interior `pending_selection` macro-substep bundling now stops and hands control back to UCT whenever `best_prior < macro_substep_min_prior` (default `0.0`, preserving existing behavior unless enabled).
     - *Live `search_exam` verification on `Gen6_hybrid` (`8` seeds, `320/4`, `--macro-substeps 16 --macro-substep-min-prior 0.85`):* recovered from **`2/8` → `8/8`** on `[1002]` (`58%–68%` of root visits, choosing `[1002]` outright on all 8 seeds) while keeping `8/8` avoiding `[4]`.
  3. **Regularized Value Calibration & Capacity Scaling:** Added opt-in `--weight-decay` (`AdamW` when `> 0.0`) and `--value-label-smoothing` (shrink training `z` by `1 - eps`, raw `z` on held-out MSE) to `azero_trainer.py`, `azero_lstm_distill.py`, and `run_selfplay_generation.py`, plus `--net-arch`, `--features-dim`, and `--lstm-hidden-size` overrides in `run_hybrid_small_campaign.py`.
  4. **Multi-Round Iterative `Search(LSTM) -> AZ-LSTM -> KL-Anchored PPO` Loop:** Wired `--trainer-kind {mlp,lstm}` in `run_selfplay_generation.py` and `--mode iterative-lstm --rounds R` in `run_hybrid_small_campaign.py` for full EX13 pool scaling.


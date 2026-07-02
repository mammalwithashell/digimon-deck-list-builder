# Pre-registered robustness baselines (task 0.4)

**Recorded 2026-07-02, BEFORE any search agent exists** (D6 discipline). Any agent produced by this change is judged against these numbers with the same protocol.

## Protocol

- **Targets** (the three strongest current agents): `starter_pool_single_v1` (MLP generalist, base `models/starter_pool_single_v1/final.zip`) and the two `models/champions/registry.json` entries `v022-generalist-v1` / `v020-generalist-v1` (LSTM generalists, weights under base `cloud_downloads/`). All layout `sha256:a20462fb…` (standard_lite_deck_v2).
- **Anchored panels**: `anchored_eval_cli.py`, n=300 seat-balanced games per anchor (±~5.7% at 95%), anchors = greedy + the *other* champions (self-excluded registries — no mirror rows), decks pinned to `starter_pool_single_v1/deck_pool_snapshot.json`.
- **Exploitability**: `exploiter_cli.py`, forward-only PPO exploiter at a **fixed 1,000,000-step budget**, same deck-pool snapshot; the number is the exploiter's *peak* eval win rate — an approximate **lower bound** on exploitability. Not a Nash claim.
- **Hosts**: Hetzner cpx62 (16 dedicated vCPU, no cgroup throttle), image `training-v0.46` (+ `exploiter_cli.py` patched in — the CLI postdates the image; the underlying `agents/exploiter.py` library is baked). Raw artifacts (panels, evals.jsonl, exploiter checkpoints) in the session scratchpad harvest; JSONs reproduced below.

## Results

### Anchored skill (n=300 per cell, row = candidate)

| Candidate | vs greedy | vs v022 | vs v020 |
|---|---|---|---|
| starter_pool_single_v1 (MLP) | **61.0%** (183-117) | 56.0% (168-132) | 53.7% (161-139) |
| v022-generalist-v1 (LSTM) | **61.3%** (184-116) | — | 47.0% (141-159) |
| v020-generalist-v1 (LSTM) | **59.0%** (177-123) | 44.3% (133-167) | — |

Skill ordering: starter-MLP ≥ v022 > v020 (the MLP beats both champions head-to-head; note the starter's 61.0% vs greedy exactly reproduces its known posthoc n=300 number — protocol sanity check).

### Exploitability (1M-step exploiter budget, lower bounds)

| Target | Exploitability (peak exploiter WR) |
|---|---|
| starter_pool_single_v1 (MLP) | **0.70** |
| v022-generalist-v1 (LSTM) | **0.75** |
| v020-generalist-v1 (LSTM) | **0.75** |

## Reading

- **Every current agent is highly exploitable**: a fresh best-response PPO trained with a *fraction* of any target's training budget farms it to 70–75%. This is the model-free-PPO brittleness the equilibrium methods discussion predicts, now quantified.
- **Skill and robustness are separate axes**: the anchored panel cannot see this — all three sit at a comfortable 59–61% over greedy while being 70-75% exploitable. The exploitability column carries information the win-rate column doesn't, which is exactly why both are pre-registered.
- **The bar for the search lane**: a determinized-search agent should (a) beat these three on the anchored frame, and (b) post a *lower* exploitability at the same 1M budget. Claim (b) is the robustness improvement; neither claim is ever "Nash."
- Exploiter curves are non-monotone (v022's exploiter read 0.40 late in the run vs its 0.75 peak) — the peak-across-evals definition is load-bearing; do not substitute final-eval win rate.

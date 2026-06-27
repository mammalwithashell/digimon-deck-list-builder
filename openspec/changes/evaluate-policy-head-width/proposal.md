## Why

The RL agent's policy/value heads are the SB3 default `[64, 64]` on top of the 512-dim `CardEmbeddingExtractor` — i.e. the whole game state is squeezed through a **64-wide bottleneck** before the policy chooses among 2192 actions. For a game this interactive, 64 plausibly caps how good the policy can get, regardless of training volume. After this session established (by measurement) that the throughput levers are largely dead ends (the 8× engine, IPC removal, batch=256, parallel-2 all failed to move *training* — see memory `project_training_update_bound_levers` / `project_league_config_failures`), **sample efficiency is the highest-leverage remaining axis**, and head width is its cheapest candidate. The enabling knobs were already built and verified this session; this change captures committing them properly and running the experiment that was deferred.

## What Changes

- **Land the head-width knobs** (already implemented + manually verified in the worktree, currently uncommitted): `--net-arch` (pi/vf hidden sizes; default unchanged → `[64,64]`) and `--init-extractor-from` (partial warm-start of *only* the `CardEmbeddingExtractor`, leaving heads random) in `pilot_training` + `TrainingConfig`. Add unit tests and thread `--net-arch` through the deck-specialist league driver.
- **Run the fair head-width comparison**: `[64,64]` (baseline) vs `[256,256]` (wider), each `--init-extractor-from` the generalist seed (identical representation, fresh heads — the only way to compare *different* archs fairly, since `--init-from` requires a matching architecture), trained vs a **champion pool** (headroom — greedy ceilings instantly), judged by **anchored eval** (vs greedy + the league2 champions), NOT the in-run win rate.
- **Record the verdict** (does wider help / hurt / neutral) and, if positive, make the chosen width the league default.
- Bake in the hard-won eval discipline: greedy is non-discriminating (ceilings in 1–2 updates); use a champion/pool opponent + anchored eval as the judge.

## Capabilities

### New Capabilities
- `policy-head-width`: configurable policy/value head architecture, extractor-only warm-start for cross-architecture transfer, and the anchored-eval-judged protocol for comparing head widths by sample efficiency.

### Modified Capabilities
<!-- none: no existing spec's requirements change -->

## Impact

- Code: `code/digimon_gym/agents/training_config.py` (new `net_arch`, `init_extractor_from` fields + validation — done), `code/digimon_gym/agents/pilot_training.py` (CLI + model construction + extractor warm-start — done), `code/tools/train_specialist_league.py` (pass `--net-arch` through — pending), `code/tests/rl/` (new unit tests — pending).
- Experiment artifacts: a champion pool manifest (`champion_admin.py emit-pool` over the league2 champions in `models/specialists/`), an anchored-eval champion registry wiring, and a short comparison runbook.
- No change to production defaults: `net_arch=None` → SB3 `[64,64]`, byte-identical to today unless the flag is set.

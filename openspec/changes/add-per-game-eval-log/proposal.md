## Why

Per-archetype digivolve telemetry (PR #543) and the broader eval-window metrics in
`evals.jsonl` only persist **per-eval-window means** — the per-game counts are
summed and divided, then the originals are discarded. A mean of 0.4
digivolves/game cannot distinguish "one whale game with 4 digivolves, 9 with 0"
from "consistent 0.4 across all 10 games" — exactly the distinction needed to
judge whether digivolve reward shaping (PR #538) is working. The same
mean-only limitation applies to `episode_length` and `terminal_score`.

Per-game rows also unlock correlation queries the current pipeline can't answer:
"did digivolving correlate with winning?", "what fraction of games saw any DNA
digivolve at all?", and — by carrying the recording path — "show me a replay of
a game where the agent triple DNA-digivolved."

## What Changes

- **`WinRateCallback._run_evaluation` writes one JSONL row per completed eval
  game** to `models/<run_id>/eval_game_log.jsonl`. The callback already loops
  per game and already extracts every field this row needs
  (digivolve counters via `_rl_state()`, `winner_id`, `info["deck1_archetype"]`,
  `info["opponent_archetype"]`, `steps`, `terminal_score`); the change is to
  emit a row instead of just summing into means.
- **Eval-only for v1**: writes happen inside the eval loop only; training
  rollouts do not produce rows. The row schema carries a `source="eval"`
  field so future training-side emission can mix into the same file
  without a schema break.
- **Row schema (v1)**: `step`, `eval_window_idx`, `game_idx`, `source="eval"`,
  `agent_archetype`, `opponent_archetype`, `digivolves_agent`,
  `dna_digivolves_agent`, `digivolves_opponent`, `dna_digivolves_opponent`,
  `result` (win/loss/draw), `episode_length`, `terminal_score`,
  `recording_path`.
- **New `run_per_game_evals` MCP tool** in `digimon-training-mcp` that returns
  per-game rows from `eval_game_log.jsonl`, with optional filters
  (e.g. `digivolves_agent_min`, `agent_archetype`, `result`, `step_min/max`).
- **CLI plumbing**: `--eval-game-log {on,off}` flag on `pilot_training`
  mirroring the existing `--mulligan-log` flag; defaults to `on`.

## Capabilities

### New Capabilities

- `per-game-eval-log`: Per-game JSONL logging during evaluation — covers the
  wrapper, file format, row schema, write semantics, and wiring into the
  pilot-training eval-env factories.

### Modified Capabilities

- `training-status-mcp`: Adds the `run_per_game_evals` query tool for reading
  the new file from `digimon-training-mcp`. No changes to existing tools.

## Impact

**Affected code:**

- `code/digimon_gym/agents/` — new `game_log.py` module with a small
  `GameLogWriter` class (append-only JSONL writer). `pilot_training.py`
  constructs one writer per training run and `WinRateCallback._run_evaluation`
  emits a row at the end of each per-game eval iteration.
- `code/digimon-training-mcp/src/digimon_training_mcp/` — new `per_game.py`
  module with the reader, new tool registration in `server.py`.

**Affected artifacts:**

- New on-disk file per run: `models/<run_id>/eval_game_log.jsonl`
  (single file — the callback runs in one process against a single eval
  env). Bounded in size: `n_eval_episodes × n_eval_windows` rows per run
  (~10k–50k for typical runs), ~300 bytes per row.

**No impact:**

- `evals.jsonl` schema unchanged — the per-window means stay where they are.
- TB scalars unchanged.
- `TrainingRunMetadata` sidecar unchanged.
- Engine, PyO3 bindings, reward shaping — all untouched. The data already
  round-trips through `get_rl_state()`; this change is read-side only.

**Dependencies:** None new. Reuses the `MulliganLogWriter` jsonl pattern,
the existing `WinRateCallback`'s per-game eval loop, and the recording-path
plumbing from PR #517.

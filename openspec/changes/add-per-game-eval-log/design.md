## Context

Per-archetype digivolve telemetry (PR #543, change
`per-archetype-digivolve-telemetry`) added cumulative cross-eval tallies
to `WinRateCallback` and persisted four `mean_eval_*_per_game` fields
plus four `by_archetype` keys to `evals.jsonl`. The engine counters
(`Game.n_digivolutions[2]`, `Game.n_dna_digivolutions[2]`) and PyO3
round-trip via `get_rl_state()` are also already in place from PR #538.

`WinRateCallback._run_evaluation` ([pilot_training.py:443-612](code/digimon_gym/agents/pilot_training.py:443))
runs a per-game eval loop in a single process. The eval env
(`self._eval_env`) is a single env constructed once per callback
lifetime, not a `SubprocVecEnv`. The loop already extracts every field
this change needs:

- `winner_id` from `_unwrap_to_digimon_env(eval_env).winner_id`
- `agent_archetype = info["deck1_archetype"]` / `opponent_archetype =
  info["opponent_archetype"]` from `env.reset()`
- `p1_digi_game`, `p1_dna_game`, `p2_digi_game`, `p2_dna_game` from
  `_rl_state()`
- `steps`, `terminal_score`, `episode_reward` from the inner game loop
- `self.num_timesteps` from the SB3 callback parent class

Mulligan-log (PR #535/#536) needed a wrapper because mulligan transitions
happen inside `reset()` / the first few `step()` calls — never visible to
the callback — and because mulligan-log also runs on multi-process training
rollouts. **Neither constraint applies here**: game-log emits at game end
(callback-visible), eval-only (single env). The wrapper-symmetry argument
is therefore not load-bearing for v1.

The training-inspection MCP (`code/digimon-training-mcp/`) currently
exposes seven tools — `list_runs`, `run_summary`, `run_metric`,
`run_tags`, `run_recordings`, `run_checkpoints`, `run_deck_pool`. None
reaches per-game rows.

## Goals / Non-Goals

**Goals:**

- Make per-game outcomes queryable for the lifetime of a training run,
  with enough fields to (a) detect "whale games" hidden by the
  eval-window mean, (b) correlate digivolving with winning, (c) jump to
  a replay from a row.
- Keep the implementation small — emit rows from the existing per-game
  loop rather than introducing a new wrapper, new shared state, or new
  per-worker file plumbing.
- Keep the schema extensible. New per-game fields should be additive —
  consumers tolerate unknown keys.
- Ship a first-class MCP query surface so per-game questions can be
  answered without writing pandas code.

**Non-Goals:**

- Training-rollout per-game logging. The row schema carries `source` so
  a future change can add training-side rows, but v1 is eval-only.
- Per-turn / per-action granularity within a game. That's the recording
  layer's job (PR #517). Per-game rows reference recordings via
  `recording_path` rather than duplicating their content.
- New TensorBoard scalars. TB already carries the eval-window means; the
  per-game file is the *raw* layer below them, not a replacement.
- Engine, PyO3, or reward-shaping changes. All counters already exist;
  this change is read-side only on the engine boundary.
- Schema versioning / migration tooling for old runs. Pre-feature runs
  legitimately have no game-log file; the MCP tool returns an empty list
  rather than synthesizing rows from `evals.jsonl`.

## Decisions

### Decision 1: Callback-direct write, not a wrapper

Rows are emitted from `WinRateCallback._run_evaluation` directly, not
from a `gymnasium.Wrapper`.

**Alternatives considered:**

- **`GameLogWrapper` mirroring `MulliganLogWrapper`.** Was the original
  design. Discarded after inspecting the callback: it already loops
  per-game, already has every field, and runs against a single eval env.
  A wrapper would need three plumbing layers — `SharedEvalStep`,
  archetype-provider callables, `_find_recording_path()` walking the env
  stack — to recover information that is literally lying around in the
  callback frame.

**Why callback wins:** ~400 LOC less code, no new shared state, no IPC
story, no env-stack walk. The wrapper-symmetry with mulligan-log was
attractive but not load-bearing — mulligan-log *needs* a wrapper because
the data lives inside `reset()` / early `step()` transitions that the
callback never sees, and because mulligan-log runs on training subprocs.
Game-log has neither constraint.

**Cost of this decision:** if a future change wants per-game rows for
*training* rollouts too, we'd need to either move to a wrapper or
duplicate the row-emission code into the training env factory. The
`source` field in the schema explicitly anticipates this; if/when that
expansion happens, the wrapper migration is mechanical because the row
schema is already stable.

### Decision 2: Single file per run, not per env worker

Output is `models/<run_id>/eval_game_log.jsonl` — one file, no
`_env_NNN` suffix.

**Why:** the eval env is single-process. There is no concurrent writer.
The per-worker file pattern in mulligan-log exists only because mulligan
runs on `SubprocVecEnv` training envs; here it would be dead weight.

### Decision 3: Recording path via the recording_wrapper's last-write attribute

After each per-game iteration, the callback walks `self._eval_env` for a
recording wrapper and reads its most-recent-write attribute (e.g.
`last_recording_path`). If absent, `recording_path = None`.

**Implementation note:** the recording wrapper's exact attribute name is
TBD pending a quick audit. If it doesn't expose one suitable for this
use, we add a single property — small, local change.

### Decision 4: Step / eval_window_idx captured at loop entry

`self.num_timesteps` is read once at the start of `_run_evaluation` and
applied to every row in the window. `eval_window_idx` is a new instance
counter incremented at loop entry. This guarantees all rows in the same
window share the same `step` and `eval_window_idx`, simplifying
"per-checkpoint" queries.

### Decision 5: MCP tool returns rows, not aggregates

The `run_per_game_evals` tool is a thin reader. Aggregations
(distributions, correlations) happen in the caller — Claude can compose
filters in conversation, or a notebook can pull all rows and group.
Filters exist because they shrink the response payload at scale.

### Decision 6: Schema is open for additive evolution

Consumers (the MCP reader, downstream notebooks) must tolerate unknown
keys. Future per-game additions (e.g. `memory_swings`,
`actions_played`) SHALL be additive only; no field renames or type
changes within the v1 schema.

## Risks / Trade-offs

**[Future expansion to training rollouts requires a refactor]** → If
training-side per-game logging is later added, the callback-direct
approach forces either (a) a wrapper migration or (b) duplication of the
row-build code in the training env factory. Mitigation: the row schema
already includes `source`; the migration is mechanical and bounded. The
v1 simplicity gain outweighs the speculative future cost.

**[Schema drift between writer and reader]** → If the writer adds fields
the reader doesn't know about, no harm. If the writer *removes* fields,
the reader will fail. Mitigation: requirement that schema evolution is
additive-only is enforced by the spec and reviewed at PR time.

**[Active-run reads partial files]** → MCP queries against a run still
training will see partial files. Mitigation: writer flushes after every
row; jsonl is naturally line-delimited, so a half-written row is
impossible if flush completes between rows. The reader skips lines that
fail to parse with a warning.

**[Replay path may break on host migration]** → `recording_path` is
absolute, as written by the recording wrapper. If a run directory is
moved or mounted at a different path, the field becomes stale.
Mitigation: the MCP reader exposes it as a string; the engine MCP's
`load_recording` accepts either absolute or relative paths. Future
tooling can rewrite paths on relocation, but v1 ships absolute as-written.

**[Writer failure silently disables emission]** → On any `OSError` the
writer disables itself and logs once. This is intentional (observability
must not kill training) but it means the file can quietly stop growing.
Mitigation: the once-per-process stderr warning is loud enough at run
start; downstream tooling treats "row count < eval window count" as a
warning signal.

## Migration Plan

No data migration. The feature is additive:

1. Pre-feature runs have no `eval_game_log.jsonl`. `list_runs` reports
   `has_eval_game_log: false`. `run_per_game_evals` returns an empty
   `rows` list.
2. Existing `evals.jsonl` schema, TB scalars, `TrainingRunMetadata`
   sidecar, mulligan-log files, and recordings are untouched.
3. The CLI flag defaults to `on` — new runs will produce the file from
   the first run after the change merges. A run launched with
   `--eval-game-log off` is byte-identical to a pre-feature run.

Rollback: revert the change. Existing game-log files become orphaned but
harmless (they live alongside the run, not in `evals.jsonl`).

## Open Questions

1. Does the existing recording wrapper expose a `last_recording_path`-style
   attribute usable by the callback, or do we need to add one? — Resolve
   during task 3.

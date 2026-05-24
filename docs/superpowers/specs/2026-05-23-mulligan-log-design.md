# Mulligan Tracking Tooling — Design

**Date:** 2026-05-23
**Author:** Claude (with James)
**Status:** Proposed

## Problem

We have no continuous visibility into how the pilot agent's mulligan policy
evolves during training. The mulligan is a single discrete decision per game
(`KEEP=0` / `MULL=1`) made at game start — small in action space, but a
useful diagnostic for whether the policy is learning to condition on hand
state vs. learning a per-archetype prior.

A one-off probe (loading checkpoint 50k vs 150k, replaying N games per
checkpoint, counting mulligan choices) confirmed:

- The agent's mulligan rate flipped from 72% KEEP at step 50k to 40% KEEP at
  step 150k.
- The rate is **not** correlated with hand contents (e.g. presence of a
  level-3 Digimon) — keep rates were ~identical for hand-has-lvl-3 vs.
  hand-has-no-lvl-3.
- Per-archetype divergence is large (BG Imperial 19% keep at 150k vs.
  Medusamon 74%).

The probe approach is expensive (one checkpoint at a time, requires manual
runs) and only captures checkpoint-cadence snapshots. We want continuous
per-game data captured live during training so we can plot
`mulligan_rate(t)` and correlate with hand state and win rate over the full
run.

## Goal

Capture per-game starting hand + mulligan choice from the pilot seat,
written live to a sidecar JSONL file under the run directory, with enough
schema fidelity that any future question about mulligan behavior
(per-archetype, per-hand-feature, per-time-window) can be answered with a
pandas one-liner.

**Non-goals:** opponent-side mulligan logging; back-fill of existing
checkpoints; TensorBoard scalar aggregation; analyzer CLI. JSONL is a
documented schema and pandas/jq cover all anticipated queries.

## Architecture

A new `MulliganLogWrapper` (`code/digimon_gym/agents/mulligan_log.py`) sits
in the env stack alongside `TrainingRecordingWrapper` in `make_env()`. The
wrapper:

1. On `reset()`: captures the pilot's starting hand IDs by reading
   `runner.to_ui_json()['player1']['handIds']`, computes derived features
   from `data/cards.json`, and stashes a pending record.
2. On `step()`: passes the action through. When the pre-step phase was
   Mulligan AND the acting player is 1 AND we haven't yet logged this game,
   the wrapper appends the completed record (now including the action) to
   `models/<run>/mulligan_log.jsonl`. One JSONL append per game; no
   buffering in memory beyond the single pending record.

A parallel `MulliganLogWriter` helper (modeled on `TrainingGameRecorder`)
owns the JSONL path, the once-per-run header line, and per-`env_index` file
handles. Constructed once in `train()` and threaded through `make_env` /
`make_vec_env`.

## Wrapper interface

```python
class MulliganLogWrapper(gymnasium.Wrapper):
    def __init__(
        self,
        env: gymnasium.Env,
        writer: MulliganLogWriter,
        *,
        source: str,                 # "train" | "eval"
        env_index: int = 0,
    ): ...
```

- `reset(**kwargs)`: delegates, then queries `runner.to_ui_json()` once for
  the pilot's hand. Stashes:

  ```python
  self._pending = {
      "wall_time": time.time(),
      "iso_time": datetime.now(timezone.utc).isoformat(),
      "global_step": self._infer_global_step(),  # may be None
      "source": self.source,
      "env_index": self.env_index,
      "game_index": self._game_counter,
      "agent_archetype": info.get("deck1_archetype"),
      "opp_archetype": info.get("opponent_archetype"),
      "hand_card_ids": ui["player1"]["handIds"],
      "hand_lvl_counts": _derive_lvl_counts(ui["player1"]["handIds"]),
      "hand_has_tamer": _derive_has_tamer(ui["player1"]["handIds"]),
      "hand_size": len(ui["player1"]["handIds"]),
      "first_player_id": ui.get("currentPlayer"),
  }
  ```

- `step(action)`: snapshot `pre_step_player = runner.mulligan_current_player`
  and `pre_step_phase = runner.to_ui_json()["currentPhase"]` **before**
  calling `self.env.step(action)`. After the inner step returns, if
  `self._pending is not None` AND `pre_step_player == 1` AND `pre_step_phase
  == MULLIGAN_PHASE_ENUM`, finalize: `self._pending["action"] = int(action)`,
  `writer.append(self._pending)`, `self._pending = None`,
  `self._game_counter += 1`.

- Edge cases handled inline:
  - **Opponent goes first.** Already absorbed by the outer
    `OpponentWrapper`: by the time our wrapper sees `reset()` returning, P1
    is the next decider. Hand capture works regardless. The mulligan-action
    step is the pilot's first real `step()` call.
  - **Engine crash before mulligan resolves.** `_pending` is dropped on the
    next `reset()`. We do not try to flush partial records — observability
    only, not core gameplay.
  - **Writer failure (disk full, file locked).** Caught, logged once to
    stderr, writer disables itself for the rest of the run. Training is
    never killed by this.

## Record schema (one JSONL line per game)

```json
{
  "schema_version": 1,
  "wall_time": 1779579610.51,
  "iso_time": "2026-05-23T23:40:10+00:00",
  "global_step": 152448,
  "source": "train",
  "env_index": 0,
  "game_index": 3204,
  "agent_archetype": "BG Imperial",
  "opp_archetype": "Medusamon",
  "hand_card_ids": ["BT21-001","BT21-008","BT21-001","BT21-013","BT21-001"],
  "hand_lvl_counts": {"3": 4, "4": 1, "5": 0, "6": 0, "7": 0},
  "hand_has_tamer": false,
  "hand_size": 5,
  "first_player_id": 2,
  "action": 1
}
```

A header line is written as the **first line of the file** (lazily, on the
very first `append()` call per `env_index`), with `kind:
"mulligan_log_header"`, so downstream tooling can sanity-check version and
run metadata without scanning the body. Helper functions
`_derive_lvl_counts(card_ids)` and `_derive_has_tamer(card_ids)` live in
`mulligan_log.py` next to the wrapper; both read from `data/cards.json`
loaded once at module import. The relevant cards.json fields are
`level: int | null` (2 = digi-egg, 3-7 = Digimon) and `card_kind: int`
(0 = Digimon, 1 = Tamer, 2 = Option, 3 = DigiEgg). `_infer_global_step()` returns
`getattr(self.unwrapped, 'num_timesteps', None)` if SB3 has attached it,
else `None` — the field is best-effort, not load-bearing.

## Wiring

In `code/digimon_gym/agents/pilot_training.py`'s `make_env()`, between the
existing `TrainingRecordingWrapper` and the outer `ActionMasker`:

```python
if mulligan_log_writer.enabled:
    env = MulliganLogWrapper(
        env,
        writer=mulligan_log_writer,
        source=recording_source,
        env_index=recording_env_index,
    )

def mask_fn(env):
    return _unwrap_to_digimon_env(env).action_mask()

return ActionMasker(env, mask_fn)
```

`mulligan_log_writer` is constructed once in `train()`, mirroring
`recording_writer`:

```python
mulligan_log_writer = MulliganLogWriter(
    output_dir=run_dir,
    enabled=(cfg.mulligan_log != "off"),
    run_metadata={...same shape as recording_writer.run_metadata...},
)
```

## Configuration

- New CLI flag in argparse: `--mulligan-log {on,off}`, default `on`.
- New `TrainingConfig` field: `mulligan_log: str = "on"`.
- Output path: `<save_dir>/<run_name>/mulligan_log.jsonl`.
- Append-only; safe to leave default-on (~3 MB total for a 1M-step run).

## Testing

Four tests, three unit + one config-wiring:

1. **`test_writes_pilot_keep_record`** — Drive `DigimonEnv` +
   `MulliganLogWrapper` through one game with a seed that puts P1 first and
   a hand containing a level-3. Mock the policy to predict `0` (KEEP).
   Assert JSONL has one record, `action == 0`,
   `hand_lvl_counts["3"] >= 1`, `hand_size == 5`.

2. **`test_writes_pilot_mull_record_when_opp_goes_first`** — Seed that puts
   P2 first; greedy opponent advances automatically; pilot's first
   `step()` is the mulligan; predict `1`. Assert record captured with
   `action == 1`, `first_player_id == 2`.

3. **`test_disabled_writes_nothing`** — Construct `MulliganLogWriter`
   with `enabled=False`; run a full game through the wrapper; assert no
   JSONL file is created.

4. **`test_mulligan_log_flag_in_pilot_training_config.py`** — Patch
   `train()` to assert the writer is constructed when `--mulligan-log on`
   and skipped when `off`. Lives in `test_pilot_training_config.py`
   alongside the existing `--record-games` flag test.

No new Rust tests needed; `to_ui_json()['player1']['handIds']` is already
exercised by existing scenarios.

## File touch list

- **New:** `code/digimon_gym/agents/mulligan_log.py`
- **Modified:** `code/digimon_gym/agents/pilot_training.py` —
  `make_env()`, `make_vec_env()`, `train()`, argparse block, banner print
- **Modified:** `code/digimon_gym/agents/training_config.py` — add
  `mulligan_log: str = "on"` field
- **New:** `code/tests/rl/test_mulligan_log.py`
- **Modified:** `code/tests/rl/test_pilot_training_config.py` — one
  additional flag-wiring test

## Open questions

None — the brainstorming pass resolved scope (live-only), fields (full
hand + derived), wiring (new wrapper), and analyzer (none, JSONL only).

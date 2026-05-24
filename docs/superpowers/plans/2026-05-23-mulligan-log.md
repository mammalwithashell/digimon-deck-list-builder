# Mulligan Log Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add live per-game starting-hand + mulligan-choice JSONL logging during pilot training so we can plot how the agent's mulligan policy evolves over time.

**Architecture:** New `MulliganLogWrapper` (Gymnasium wrapper) sits in the env stack alongside `TrainingRecordingWrapper` inside `make_env()`. On `reset()` it snapshots the pilot's starting hand via `runner.to_ui_json()['player1']['handIds']`; on the first matching `step()` (pre-step `mulligan_current_player == 1`) it appends a record with the chosen action to `models/<run>/mulligan_log_env_<NNN>.jsonl` (one file per SubprocVecEnv worker, zero-padded to three digits). A parallel `MulliganLogWriter` helper owns the lazy-opened file handle and writes a one-time header line. Each subprocess holds its own writer constructed from a shared `_MulliganLogConfig` dataclass so there is no cross-process file contention.

**Tech Stack:** Python 3.13, gymnasium, sb3-contrib (MaskablePPO), PyO3-bound Rust engine via `digimon_engine.RustHeadlessGame`, pytest.

**Spec:** `docs/superpowers/specs/2026-05-23-mulligan-log-design.md`

---

## File Structure

| Action | Path | Responsibility |
|---|---|---|
| Create | `code/digimon_gym/agents/mulligan_log.py` | `_derive_lvl_counts`, `_derive_has_tamer`, `MulliganLogWriter`, `MulliganLogWrapper` |
| Modify | `code/digimon_gym/agents/training_config.py` | Add `mulligan_log: str = "on"` field + validation |
| Modify | `code/digimon_gym/agents/pilot_training.py` | Argparse flag, writer construction in `train()`, wrapper wiring in `make_env()` / `make_vec_env()`, banner line |
| Create | `code/tests/rl/test_mulligan_log.py` | Helpers, writer, wrapper unit tests |
| Modify | `code/tests/rl/test_pilot_training_config.py` | One additional flag-wiring test |

---

## Task 1: Hand-feature helpers (`_derive_lvl_counts`, `_derive_has_tamer`)

**Files:**
- Create: `code/digimon_gym/agents/mulligan_log.py`
- Create: `code/tests/rl/test_mulligan_log.py`

- [ ] **Step 1: Write the failing tests**

Create `code/tests/rl/test_mulligan_log.py`:

```python
"""Tests for code/digimon_gym/agents/mulligan_log.py."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from digimon_gym.agents.mulligan_log import (
    _derive_lvl_counts,
    _derive_has_tamer,
)


def test_derive_lvl_counts_counts_each_level_bucket():
    # ST1-03 is a level-3 Digimon; ST1-01 is a level-2 (egg).
    # Pick known-existing ids from data/cards.json.
    counts = _derive_lvl_counts(["ST1-03", "ST1-03", "ST1-01", "ST1-03", "ST1-01"])
    # Only levels 3-7 are bucketed.
    assert counts["3"] == 3
    assert counts["4"] == 0
    assert counts["5"] == 0
    assert counts["6"] == 0
    assert counts["7"] == 0


def test_derive_lvl_counts_handles_unknown_ids():
    counts = _derive_lvl_counts(["NOT-A-REAL-CARD", "ST1-03"])
    assert counts["3"] == 1  # unknown id contributes 0 to every bucket


def test_derive_has_tamer_returns_false_when_no_tamer():
    # ST1-03 is a Digimon, not a Tamer.
    assert _derive_has_tamer(["ST1-03", "ST1-03"]) is False


def test_derive_has_tamer_returns_true_when_any_card_is_tamer():
    # Need at least one known Tamer card id; ST1-09 is "Tai Kamiya" (Tamer)
    # in the starter set. If this id is missing from cards.json the test
    # will skip rather than fail spuriously.
    import json
    from data_paths import CARDS_JSON
    cards = json.loads(Path(CARDS_JSON).read_text(encoding="utf-8"))
    tamer_ids = [cid for cid, c in cards.items() if (c.get("card_type") or "").lower() == "tamer"]
    if not tamer_ids:
        pytest.skip("No Tamer cards in cards.json — cannot exercise has_tamer=True path")
    assert _derive_has_tamer([tamer_ids[0], "ST1-03"]) is True
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `python -m pytest code/tests/rl/test_mulligan_log.py -v`
Expected: ImportError — `mulligan_log` module does not exist yet.

- [ ] **Step 3: Implement the helpers**

Create `code/digimon_gym/agents/mulligan_log.py`:

```python
"""Per-game mulligan log writer + wrapper.

Captures starting hand + mulligan choice from the pilot seat during
training, appended live to `models/<run>/mulligan_log.jsonl`. See
`docs/superpowers/specs/2026-05-23-mulligan-log-design.md`.
"""

from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

import gymnasium

from data_paths import CARDS_JSON


SCHEMA_VERSION = 1


def _load_card_metadata() -> Dict[str, Dict[str, Any]]:
    """Load cards.json once at module import; used by helpers below."""
    try:
        return json.loads(Path(CARDS_JSON).read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {}


_CARDS = _load_card_metadata()


def _derive_lvl_counts(card_ids: List[str]) -> Dict[str, int]:
    """Return a histogram of levels 3..7 for the given card IDs.

    Unknown card IDs and cards without a level field contribute 0 to every
    bucket. Only Digimon levels 3-7 are bucketed; eggs (level 2) and
    Options/Tamers are ignored here (use `_derive_has_tamer` for tamers).
    """
    buckets = {str(lvl): 0 for lvl in range(3, 8)}
    for cid in card_ids:
        lvl = _CARDS.get(cid, {}).get("level")
        if isinstance(lvl, int) and 3 <= lvl <= 7:
            buckets[str(lvl)] += 1
    return buckets


def _derive_has_tamer(card_ids: List[str]) -> bool:
    """True if any card in the list is a Tamer (case-insensitive match)."""
    for cid in card_ids:
        ct = (_CARDS.get(cid, {}).get("card_type") or "").lower()
        if ct == "tamer":
            return True
    return False
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `python -m pytest code/tests/rl/test_mulligan_log.py -v`
Expected: all 4 tests PASS (the Tamer test may SKIP if cards.json has no tamers, which it shouldn't on this codebase).

- [ ] **Step 5: Commit**

```bash
git add code/digimon_gym/agents/mulligan_log.py code/tests/rl/test_mulligan_log.py
git commit -m "feat(rl): add mulligan-log hand-feature helpers"
```

---

## Task 2: `MulliganLogWriter` (JSONL sidecar writer)

**Files:**
- Modify: `code/digimon_gym/agents/mulligan_log.py` (add `MulliganLogWriter` class)
- Modify: `code/tests/rl/test_mulligan_log.py` (add writer tests)

- [ ] **Step 1: Write the failing tests**

Append to `code/tests/rl/test_mulligan_log.py`:

```python
from digimon_gym.agents.mulligan_log import MulliganLogWriter


def _read_jsonl(path: Path) -> list:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def test_writer_disabled_does_nothing(tmp_path):
    writer = MulliganLogWriter(
        output_dir=tmp_path,
        env_index=0,
        enabled=False,
        run_metadata={"run_name": "test_run"},
    )
    writer.append({"action": 0, "agent_archetype": None})
    assert not (tmp_path / "mulligan_log_env_000.jsonl").exists()


def test_writer_writes_header_then_record(tmp_path):
    writer = MulliganLogWriter(
        output_dir=tmp_path,
        env_index=0,
        enabled=True,
        run_metadata={"run_name": "test_run", "started_at": "2026-05-23T00:00:00+00:00"},
    )
    writer.append({"action": 0, "agent_archetype": "Puppets"})
    lines = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    assert len(lines) == 2
    assert lines[0]["kind"] == "mulligan_log_header"
    assert lines[0]["schema_version"] == 1
    assert lines[0]["run_name"] == "test_run"
    assert lines[1]["action"] == 0
    assert lines[1]["agent_archetype"] == "Puppets"


def test_writer_writes_header_only_once_across_appends(tmp_path):
    writer = MulliganLogWriter(
        output_dir=tmp_path,
        env_index=0,
        enabled=True,
        run_metadata={"run_name": "test_run"},
    )
    writer.append({"action": 0})
    writer.append({"action": 1})
    writer.append({"action": 0})
    lines = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    assert len(lines) == 4
    assert lines[0]["kind"] == "mulligan_log_header"
    # All subsequent records are data rows, not headers
    assert all(line.get("kind") != "mulligan_log_header" for line in lines[1:])


def test_writer_failure_disables_for_rest_of_run(tmp_path, capsys, monkeypatch):
    writer = MulliganLogWriter(
        output_dir=tmp_path,
        env_index=0,
        enabled=True,
        run_metadata={"run_name": "test_run"},
    )
    # Force the file open to raise the first time it's attempted.
    original_open = Path.open

    def _exploding_open(self, *args, **kwargs):
        if self.name == "mulligan_log_env_000.jsonl":
            raise OSError("simulated disk-full")
        return original_open(self, *args, **kwargs)

    monkeypatch.setattr(Path, "open", _exploding_open)
    writer.append({"action": 0})
    # Disabled now; subsequent appends should be silent no-ops.
    writer.append({"action": 1})
    assert writer.enabled is False
    stderr = capsys.readouterr().err
    assert "mulligan_log" in stderr.lower()


def test_writer_env_index_in_filename(tmp_path):
    writer0 = MulliganLogWriter(output_dir=tmp_path, env_index=0, enabled=True, run_metadata={"run_name": "t"})
    writer3 = MulliganLogWriter(output_dir=tmp_path, env_index=3, enabled=True, run_metadata={"run_name": "t"})
    assert writer0.path == tmp_path / "mulligan_log_env_000.jsonl"
    assert writer3.path == tmp_path / "mulligan_log_env_003.jsonl"
    # Each writer writes its own header independently.
    writer0.append({"action": 0})
    writer3.append({"action": 1})
    lines0 = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    lines3 = _read_jsonl(tmp_path / "mulligan_log_env_003.jsonl")
    assert lines0[0]["kind"] == "mulligan_log_header"
    assert lines3[0]["kind"] == "mulligan_log_header"
    assert lines0[1]["action"] == 0
    assert lines3[1]["action"] == 1
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `python -m pytest code/tests/rl/test_mulligan_log.py -v -k writer`
Expected: ImportError on `MulliganLogWriter`.

- [ ] **Step 3: Implement `MulliganLogWriter`**

Append to `code/digimon_gym/agents/mulligan_log.py`:

```python
class MulliganLogWriter:
    """Owns the JSONL file handle for a training run's mulligan log.

    One writer instance per env_index. Under SubprocVecEnv, each subprocess
    holds its own writer pointing at its own per-env-index file (e.g.
    ``mulligan_log_env_000.jsonl``, ``mulligan_log_env_001.jsonl``, ...)
    so concurrent appends never contend on the same file. Analysis tools
    glob ``mulligan_log_env_*.jsonl`` to recover the cross-env dataset.

    A single header line is written lazily on the first ``append()`` per
    writer instance. Subsequent appends write one JSON record per line.
    Failures (disk full, permission denied) flip ``enabled`` to ``False``
    and log once to stderr so training is never killed by observability
    code.
    """

    def __init__(
        self,
        output_dir: str | Path,
        *,
        env_index: int = 0,
        enabled: bool = True,
        run_metadata: Optional[Dict[str, Any]] = None,
    ) -> None:
        self.output_dir = Path(output_dir)
        self.env_index = int(env_index)
        self.enabled = bool(enabled)
        self.run_metadata = dict(run_metadata or {})
        self._path: Path = self.output_dir / f"mulligan_log_env_{self.env_index:03d}.jsonl"
        self._wrote_header = False
        self._failed = False

    @property
    def path(self) -> Path:
        return self._path

    def _header_record(self) -> Dict[str, Any]:
        return {
            "kind": "mulligan_log_header",
            "schema_version": SCHEMA_VERSION,
            **self.run_metadata,
        }

    def append(self, record: Dict[str, Any]) -> None:
        """Append one JSONL record. No-op if disabled."""
        if not self.enabled or self._failed:
            return
        try:
            self.output_dir.mkdir(parents=True, exist_ok=True)
            with self._path.open("a", encoding="utf-8") as fh:
                if not self._wrote_header:
                    fh.write(json.dumps(self._header_record()) + "\n")
                    self._wrote_header = True
                fh.write(json.dumps(record) + "\n")
        except OSError as exc:
            self._failed = True
            self.enabled = False
            print(
                f"[mulligan_log] disabled after write failure: {exc!r}",
                file=sys.stderr,
                flush=True,
            )
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `python -m pytest code/tests/rl/test_mulligan_log.py -v -k writer`
Expected: all 4 writer tests PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon_gym/agents/mulligan_log.py code/tests/rl/test_mulligan_log.py
git commit -m "feat(rl): add MulliganLogWriter with lazy header + failure-safe append"
```

---

## Task 3: `MulliganLogWrapper` (env wrapper)

**Files:**
- Modify: `code/digimon_gym/agents/mulligan_log.py` (add `MulliganLogWrapper` class)
- Modify: `code/tests/rl/test_mulligan_log.py` (add wrapper tests)

- [ ] **Step 1: Write the failing tests**

Append to `code/tests/rl/test_mulligan_log.py`:

```python
from digimon_gym.agents.mulligan_log import MulliganLogWrapper
from digimon_gym.digimon_gym import DigimonEnv, greedy_policy
from digimon_gym.agents.pilot_training import OpponentWrapper


def _drive_to_first_pilot_step(env):
    """Reset env and skip opponent's leading turns until pilot acts.

    OpponentWrapper already does this on reset, so this is a no-op here
    but documents the contract: after reset returns, the next step()
    submitted is the pilot's first decision (mulligan if first turn).
    """
    obs, info = env.reset(seed=1)
    return obs, info


def _build_wrapped_env(writer):
    inner = DigimonEnv()
    opp = OpponentWrapper(inner, opponent_fn=greedy_policy)
    wrapped = MulliganLogWrapper(opp, writer=writer, source="train", env_index=0)
    return wrapped, inner


def test_wrapper_captures_pilot_mulligan_keep(tmp_path):
    writer = MulliganLogWriter(output_dir=tmp_path, env_index=0, enabled=True, run_metadata={"run_name": "t"})
    wrapped, inner = _build_wrapped_env(writer)
    _drive_to_first_pilot_step(wrapped)
    # Pilot picks KEEP (action 0). We bypass policy here and submit directly.
    assert inner.runner.mulligan_current_player == 1
    wrapped.step(0)
    lines = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    # 1 header + 1 record
    assert len(lines) == 2
    rec = lines[1]
    assert rec["action"] == 0
    assert rec["source"] == "train"
    assert rec["env_index"] == 0
    assert rec["game_index"] == 0
    assert rec["hand_size"] == 5
    assert isinstance(rec["hand_card_ids"], list) and len(rec["hand_card_ids"]) == 5
    assert "hand_lvl_counts" in rec
    assert "hand_has_tamer" in rec
    assert rec["schema_version"] == 1


def test_wrapper_captures_pilot_mulligan_mull_when_opp_first(tmp_path):
    writer = MulliganLogWriter(output_dir=tmp_path, env_index=0, enabled=True, run_metadata={"run_name": "t"})
    wrapped, inner = _build_wrapped_env(writer)
    # Find a seed where P2 truly goes first (read true initial state from
    # the recording, NOT the post-advance to_ui_json).
    found_seed = None
    for s in range(40):
        wrapped.reset(seed=s)
        rec = inner.runner.get_recording()
        if rec.get("initial_state", {}).get("first_player_id") == 2:
            found_seed = s
            break
    if found_seed is None:
        pytest.skip("no seed in 0..39 produced P2-goes-first; try a wider range")
    # Pilot picks MULL (action 1).
    wrapped.step(1)
    lines = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    rec = lines[-1]
    assert rec["action"] == 1
    assert rec["source"] == "train"
    assert rec["first_player_id"] == 2  # the bug we fixed: this would be 1 if we used currentPlayer


def test_wrapper_disabled_writer_writes_nothing(tmp_path):
    writer = MulliganLogWriter(output_dir=tmp_path, env_index=0, enabled=False, run_metadata={"run_name": "t"})
    wrapped, inner = _build_wrapped_env(writer)
    _drive_to_first_pilot_step(wrapped)
    wrapped.step(0)
    assert not (tmp_path / "mulligan_log_env_000.jsonl").exists()


def test_wrapper_increments_game_index_across_resets(tmp_path):
    writer = MulliganLogWriter(output_dir=tmp_path, env_index=0, enabled=True, run_metadata={"run_name": "t"})
    wrapped, inner = _build_wrapped_env(writer)
    for _ in range(3):
        wrapped.reset(seed=1)
        wrapped.step(0)
    lines = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    # 1 header + 3 records
    assert len(lines) == 4
    indices = [line["game_index"] for line in lines[1:]]
    assert indices == [0, 1, 2]
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `python -m pytest code/tests/rl/test_mulligan_log.py -v -k wrapper`
Expected: ImportError on `MulliganLogWrapper`.

- [ ] **Step 3: Implement `MulliganLogWrapper`**

Append to `code/digimon_gym/agents/mulligan_log.py`:

```python
import time

from digimon_gym.agents.env_utils import unwrap_to_digimon_env


class MulliganLogWrapper(gymnasium.Wrapper):
    """Capture pilot's starting-hand + mulligan choice per game.

    Sits in the env stack outside OpponentWrapper / GeneralistDeckPoolWrapper
    / TrainingRecordingWrapper. On ``reset()`` it stashes a pending record
    with the pilot's hand snapshot. On ``step()`` it finalizes the record
    with the action if the pre-step state shows pilot is the mulligan
    decider.
    """

    def __init__(
        self,
        env: gymnasium.Env,
        writer: MulliganLogWriter,
        *,
        source: str = "train",
        env_index: int = 0,
    ) -> None:
        super().__init__(env)
        self._writer = writer
        self.source = source
        self.env_index = env_index
        self._inner = unwrap_to_digimon_env(env)
        self._pending: Optional[Dict[str, Any]] = None
        self._game_counter = 0

    # ─── Gymnasium API ───────────────────────────────────────────

    def reset(self, **kwargs):
        obs, info = self.env.reset(**kwargs)
        self._pending = None  # drop any unfinalized record from a crashed game
        if not self._writer.enabled:
            return obs, info
        runner = self._inner.runner
        if runner is None:
            return obs, info
        # Only snapshot if pilot is about to face a mulligan decision.
        if runner.mulligan_current_player != 1:
            return obs, info
        try:
            ui = runner.to_ui_json()
        except Exception:
            return obs, info
        hand_ids = list(ui.get("player1", {}).get("handIds", []) or [])
        # `currentPlayer` at this point is post-OpponentWrapper-advance and
        # is always P1 if we got here — not informative. The true first
        # player is on the recording's initial_state.
        first_player_id: Optional[int] = None
        try:
            rec = runner.get_recording()
            first_player_id = rec.get("initial_state", {}).get("first_player_id")
        except Exception:
            first_player_id = None
        self._pending = {
            "schema_version": SCHEMA_VERSION,
            "wall_time": time.time(),
            "iso_time": datetime.now(timezone.utc).isoformat(),
            "global_step": self._infer_global_step(),
            "source": self.source,
            "env_index": self.env_index,
            "game_index": self._game_counter,
            "agent_archetype": info.get("deck1_archetype"),
            "opp_archetype": info.get("opponent_archetype"),
            "hand_card_ids": hand_ids,
            "hand_lvl_counts": _derive_lvl_counts(hand_ids),
            "hand_has_tamer": _derive_has_tamer(hand_ids),
            "hand_size": len(hand_ids),
            "first_player_id": first_player_id,
        }
        return obs, info

    def step(self, action):
        # Snapshot pre-step state so we know whether this step resolves a
        # pilot mulligan.
        pre_player: Optional[int] = None
        runner = self._inner.runner if self._inner else None
        if runner is not None and self._pending is not None:
            pre_player = runner.mulligan_current_player
        obs, reward, terminated, truncated, info = self.env.step(action)
        if self._pending is not None and pre_player == 1:
            self._pending["action"] = int(action)
            self._writer.append(self._pending)
            self._pending = None
            self._game_counter += 1
        return obs, reward, terminated, truncated, info

    # ─── Internals ───────────────────────────────────────────────

    def _infer_global_step(self) -> Optional[int]:
        """Best-effort: SB3 attaches `num_timesteps` to the env in some setups."""
        return getattr(self.unwrapped, "num_timesteps", None)
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `python -m pytest code/tests/rl/test_mulligan_log.py -v -k wrapper`
Expected: all 4 wrapper tests PASS.

- [ ] **Step 5: Run the full mulligan_log test file**

Run: `python -m pytest code/tests/rl/test_mulligan_log.py -v`
Expected: 12 tests PASS (4 helper + 4 writer + 4 wrapper).

- [ ] **Step 6: Commit**

```bash
git add code/digimon_gym/agents/mulligan_log.py code/tests/rl/test_mulligan_log.py
git commit -m "feat(rl): add MulliganLogWrapper for per-game hand+action capture"
```

---

## Task 4: `TrainingConfig.mulligan_log` field

**Files:**
- Modify: `code/digimon_gym/agents/training_config.py`
- Modify: `code/tests/rl/test_pilot_training_config.py`

- [ ] **Step 1: Write the failing test**

Append to `code/tests/rl/test_pilot_training_config.py`:

```python
def test_training_config_mulligan_log_default_and_validation(tmp_path):
    cfg = TrainingConfig()
    assert cfg.mulligan_log == "on"

    # Override via yaml
    path = tmp_path / "training.yaml"
    path.write_text("mulligan_log: off\n")
    loaded = TrainingConfig.from_yaml(path)
    assert loaded.mulligan_log == "off"

    # Invalid value rejected
    with pytest.raises(ValueError, match="mulligan_log"):
        TrainingConfig(mulligan_log="maybe")
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `python -m pytest code/tests/rl/test_pilot_training_config.py::test_training_config_mulligan_log_default_and_validation -v`
Expected: FAIL on `cfg.mulligan_log` AttributeError (field doesn't exist).

- [ ] **Step 3: Add the field**

Edit `code/digimon_gym/agents/training_config.py`:

After the line:

```python
VALID_RECORD_GAME_MODES = {"off", "all", "sampled", "draws", "anomalies", "eval"}
```

Add:

```python
VALID_MULLIGAN_LOG_MODES = {"on", "off"}
```

Inside the `TrainingConfig` dataclass, after the `record_games_sample_rate: float = 0.01` line, add:

```python
    mulligan_log: str = "on"
```

Inside the `_validate(self)` method, after the existing `record_games` validation (find the block that raises if `self.record_games not in VALID_RECORD_GAME_MODES`), add:

```python
        if self.mulligan_log not in VALID_MULLIGAN_LOG_MODES:
            raise ValueError(
                f"mulligan_log must be one of {sorted(VALID_MULLIGAN_LOG_MODES)}, "
                f"got {self.mulligan_log}"
            )
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `python -m pytest code/tests/rl/test_pilot_training_config.py::test_training_config_mulligan_log_default_and_validation -v`
Expected: PASS.

- [ ] **Step 5: Run the full config test file to confirm no regressions**

Run: `python -m pytest code/tests/rl/test_pilot_training_config.py -v`
Expected: all tests PASS.

- [ ] **Step 6: Commit**

```bash
git add code/digimon_gym/agents/training_config.py code/tests/rl/test_pilot_training_config.py
git commit -m "feat(rl): add TrainingConfig.mulligan_log field (default on)"
```

---

## Task 5: Wire `MulliganLogWrapper` into `pilot_training`

**Files:**
- Modify: `code/digimon_gym/agents/pilot_training.py` (argparse, `train()`, `make_env()`, `make_vec_env()`, banner)
- Modify: `code/tests/rl/test_pilot_training_config.py` (flag-wiring test)

- [ ] **Step 1: Write the failing test**

Append to `code/tests/rl/test_pilot_training_config.py`:

```python
def test_mulligan_log_flag_argparse_default_and_off(monkeypatch, tmp_path):
    """The --mulligan-log flag flows into TrainingConfig.mulligan_log."""
    from digimon_gym.agents import pilot_training

    # Default (no flag): expect "on"
    monkeypatch.setattr("sys.argv", ["pilot_training.py"])
    parser = pilot_training._build_argparser()
    args = parser.parse_args([])
    assert args.mulligan_log == "on"

    # Explicit off
    args = parser.parse_args(["--mulligan-log", "off"])
    assert args.mulligan_log == "off"

    # Invalid value rejected by argparse choices
    with pytest.raises(SystemExit):
        parser.parse_args(["--mulligan-log", "maybe"])
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `python -m pytest code/tests/rl/test_pilot_training_config.py::test_mulligan_log_flag_argparse_default_and_off -v`
Expected: FAIL — either `_build_argparser` doesn't exist or doesn't accept the flag.

- [ ] **Step 3: Extract argparser into a callable (if not already)**

Search `code/digimon_gym/agents/pilot_training.py` for `argparse.ArgumentParser(`. If the parser construction lives inline in `main()`, extract it to a module-level `_build_argparser() -> argparse.ArgumentParser` returning the parser. The existing `main()` should then call `_build_argparser().parse_args()`.

If there's already a helper like `_build_parser` or `build_arg_parser`, use that name in Step 1's test instead of `_build_argparser` and skip this step.

- [ ] **Step 4: Add the `--mulligan-log` flag**

In `_build_argparser()`, find the block where `--record-games` is defined. Immediately after, add:

```python
    parser.add_argument(
        "--mulligan-log",
        choices=["on", "off"],
        default="on",
        help="Write per-game starting-hand + mulligan-choice records to "
             "models/<run>/mulligan_log.jsonl (default: on, ~3 MB per 1M steps).",
    )
```

In the block where CLI args are merged into `TrainingConfig` (search for `"record_games": args.record_games,`), add:

```python
        "mulligan_log": args.mulligan_log,
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `python -m pytest code/tests/rl/test_pilot_training_config.py::test_mulligan_log_flag_argparse_default_and_off -v`
Expected: PASS.

- [ ] **Step 6: Construct the writer in `train()` and thread through `make_env`/`make_vec_env`**

In `code/digimon_gym/agents/pilot_training.py`, add the import near the top with the other `digimon_gym.agents` imports:

```python
from digimon_gym.agents.mulligan_log import MulliganLogWrapper, MulliganLogWriter
```

In `train()`, locate where `recording_writer = TrainingGameRecorder(...)` is constructed and `run_dir` is defined. Immediately after the `recording_writer` line, add a config dataclass (not a single writer — SubprocVecEnv requires per-env-index files):

```python
    from dataclasses import dataclass as _dc

    @_dc
    class _MulliganLogConfig:
        output_dir: Path
        enabled: bool
        run_metadata: dict

    mulligan_log_cfg = _MulliganLogConfig(
        output_dir=run_dir,
        enabled=(cfg.mulligan_log != "off"),
        run_metadata={
            "run_name": run_name,
            "started_at": datetime.now(timezone.utc).isoformat(),
            "backend": os.environ.get("DIGIMON_BACKEND") or "auto",
            "tensor_profile": observation_layout.id,
            "tensor_layout_hash": observation_layout.layout_hash,
        },
    )
```

(`datetime` and `timezone` should already be imported; if not, add `from datetime import datetime, timezone` at the top.)

In `make_env()` and the env factory inside `make_vec_env`, each subprocess constructs its own `MulliganLogWriter` from the config, keyed by the per-env `rank` / `recording_env_index` so each subprocess owns a separate file. Find the block:

```python
    if record_this_source:
        env = TrainingRecordingWrapper(
            env,
            recording_writer,
            source=recording_source,
            env_index=recording_env_index,
        )
```

Immediately after that block, add:

```python
    if mulligan_log_cfg is not None and mulligan_log_cfg.enabled:
        writer = MulliganLogWriter(
            output_dir=mulligan_log_cfg.output_dir,
            env_index=rank,
            enabled=mulligan_log_cfg.enabled,
            run_metadata=mulligan_log_cfg.run_metadata,
        )
        env = MulliganLogWrapper(
            env,
            writer=writer,
            source=recording_source,
            env_index=rank,
        )
```

Update `make_env`'s signature to accept `mulligan_log_cfg=None` (mirror `recording_writer` argument exactly). The `MulliganLogWriter` is constructed inside the factory (one per `rank`/`env_index`), not in `train()` once.

Do the same modification in `make_vec_env` — find the inner env-factory function (the one that constructs `wrapped = OpponentWrapper(...)` etc. inside `make_vec_env`'s closure or loop), and add the `MulliganLogWrapper` wrap immediately after the `TrainingRecordingWrapper` block, capturing `mulligan_log_cfg` via closure.

In `train()`, find every call to `make_env(...)` and `make_vec_env(...)` and add `mulligan_log_cfg=mulligan_log_cfg` as a kwarg.

- [ ] **Step 7: Update the startup banner**

In `train()`, find the block printing `if recording_writer.enabled: print(f"  Record games:   ...")`. Immediately after, add:

```python
        if mulligan_log_cfg.enabled:
            print(f"  Mulligan log:   on -> {mulligan_log_cfg.output_dir}/mulligan_log_env_*.jsonl")
        else:
            print(f"  Mulligan log:   off")
```

- [ ] **Step 8: Smoke-run training for ~30 seconds to confirm the writer fires**

Run (will run in background; let it produce ~2-3 rollouts then kill):

```bash
PYTHONIOENCODING=utf-8 python -u -m digimon_gym.agents.pilot_training \
    --generalist --record-games anomalies --timesteps 5000 \
    --save-dir /tmp/mulligan_smoke --log-dir /tmp/mulligan_smoke/runs \
    > /tmp/mulligan_smoke.log 2>&1
```

Expected: completes (or near-completes) without raising. Then:

```bash
ls /tmp/mulligan_smoke/pilot_ppo_*/mulligan_log_env_*.jsonl
head -3 /tmp/mulligan_smoke/pilot_ppo_*/mulligan_log_env_000.jsonl
```

Expected: per-env files exist (one per SubprocVecEnv worker); line 1 is the header (`"kind":"mulligan_log_header"`); lines 2+ are records with `action`, `hand_card_ids`, `agent_archetype`.

- [ ] **Step 9: Run the full RL test suite for regressions**

Run: `python -m pytest code/tests/rl/ -v`
Expected: all tests PASS (no regressions to existing pilot_training / eval-suite tests).

- [ ] **Step 10: Commit**

```bash
git add code/digimon_gym/agents/pilot_training.py code/tests/rl/test_pilot_training_config.py
git commit -m "feat(rl): wire MulliganLogWrapper into pilot_training, default on"
```

---

## Done criteria

After Task 5 commits, a fresh generalist training run produces `models/<run>/mulligan_log_env_*.jsonl` — one file per SubprocVecEnv worker — each with one header line plus one record per game. A 5-line pandas snippet answers any question of the form "mulligan rate by archetype / step bucket / hand feature".

Example post-hoc analysis:

```python
import glob, json
import pandas as pd

records = [
    json.loads(line)
    for f in glob.glob("models/<run>/mulligan_log_env_*.jsonl")
    for line in open(f)
    if '"kind"' not in line
]
df = pd.DataFrame(records)
df["step_bucket"] = (df["global_step"] // 25000) * 25000
df.groupby(["step_bucket", "agent_archetype"])["action"].mean()  # mull rate by bucket+archetype
```

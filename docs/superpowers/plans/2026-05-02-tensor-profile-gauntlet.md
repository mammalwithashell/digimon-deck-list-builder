# Tensor Profile Gauntlet Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a repeatable gauntlet that compares `compact_v1`, `standard_lite_v2`, and `standard_full_v2` on throughput, games/hour, win rate versus greedy, trigger-order signal accuracy, and memory footprint.

**Architecture:** Add a standalone Python RL profiling harness under `digimon_gym.agents`, with a thin CLI wrapper under `code/tools`. The harness resolves tensor profiles through the existing registry, runs fixed-seed games through `DigimonEnv`, computes deterministic profile/layout memory estimates, and scores trigger-order visibility from synthetic probe observations so the metric is not hidden inside noisy game outcomes. The hosted API, DB-backed gauntlet orchestrator, and frontend are out of scope.

**Tech Stack:** Python 3.11, Gymnasium, PyO3 `digimon_engine`, `DigimonEnv`, existing held-out eval suite YAML, pytest, JSON/Markdown report files.

---

## File Structure

- Create `code/digimon_gym/agents/tensor_profile_gauntlet.py`: dataclasses, profile resolution, game loop, memory estimates, trigger-order probe scoring, JSON/Markdown serialization.
- Create `code/tools/profile_tensor_profiles.py`: command-line entrypoint for running the gauntlet and writing artifacts.
- Create `code/tests/rl/test_tensor_profile_gauntlet.py`: fast unit tests with fake profiles/env factories and smoke tests for registered profile resolution.
- Modify `docs/TOOLS.md`: document the CLI, default profiles, output files, and recommended commands.

## Behavioral Contract

- Requested profile IDs default to `compact_v1`, `standard_lite_v2`, and `standard_full_v2`.
- `compact_v1` is reported as canonical `standard_compact_v1` while preserving `requested_profile == "compact_v1"`.
- If a profile cannot be resolved and `require_profiles` is false, the result is included with `available == false` and a non-empty `skip_reason`.
- If `require_profiles` is true, unresolved profiles raise `ValueError`.
- Game-loop metrics are computed from identical fixed seeds and identical policy choices for each available profile.
- `steps_per_second` is environment steps divided by measured elapsed seconds.
- `games_per_hour` is completed games divided by measured elapsed hours.
- `win_rate_vs_greedy` is player-1 wins divided by completed games.
- `memory_footprint` includes at least:
  - `tensor_bytes = tensor_size * 4`
  - `tensor_kib = tensor_bytes / 1024`
  - `rollout_observation_bytes = tensor_bytes * n_steps * n_envs`
  - `rollout_observation_mib = rollout_observation_bytes / 1024 / 1024`
  - `card_embedding_input_slots = card_id_slot_count`
  - `scalar_input_slots = scalar_slot_count`
- `trigger_order_accuracy` is a deterministic signal-visibility score:
  - compact v1 has no trigger-order/prompt metadata and scores `0.0`.
  - lite v2 and full v2 can score through pending-choice/action-id probe rows.
  - full v2 gets credit only when action rows identify legal prompt-selection actions.

---

### Task 1: Result Types, Profile Resolution, And Memory Estimates

**Files:**
- Create: `code/digimon_gym/agents/tensor_profile_gauntlet.py`
- Test: `code/tests/rl/test_tensor_profile_gauntlet.py`

- [ ] **Step 1: Write failing tests for profile resolution and memory estimates**

Create `code/tests/rl/test_tensor_profile_gauntlet.py` with:

```python
from __future__ import annotations

from types import SimpleNamespace

import pytest


def fake_profile(profile_id: str, tensor_size: int):
    return SimpleNamespace(
        id=profile_id,
        game_mode="standard",
        version=2 if profile_id.endswith("_v2") else 1,
        tensor_version=2 if profile_id.endswith("_v2") else 1,
        feature_schema_version=f"{profile_id}.1",
        layout_hash=f"sha256:{profile_id.replace('_', '0')[:8]:0<64}",
        tensor_size=tensor_size,
        field_slots=15,
        slot_size=96,
        max_sources=11,
        card_id_slot_count=542,
        scalar_slot_count=tensor_size - 542,
        card_id_positions=tuple(range(542)),
        scalar_positions=tuple(range(542, tensor_size)),
        sections=(),
    )


def test_resolve_profiles_canonicalizes_compact_alias(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    profiles = {
        "compact_v1": fake_profile("standard_compact_v1", 1375),
        "standard_lite_v2": fake_profile("standard_lite_v2", 8320),
        "standard_full_v2": fake_profile("standard_full_v2", 43008),
    }
    monkeypatch.setattr(gauntlet, "get_tensor_profile", lambda profile_id: profiles[profile_id])

    resolved = gauntlet.resolve_profile_requests(
        ("compact_v1", "standard_lite_v2", "standard_full_v2"),
        require_profiles=True,
    )

    assert [item.requested_profile for item in resolved] == [
        "compact_v1",
        "standard_lite_v2",
        "standard_full_v2",
    ]
    assert [item.profile.id for item in resolved] == [
        "standard_compact_v1",
        "standard_lite_v2",
        "standard_full_v2",
    ]
    assert all(item.available for item in resolved)


def test_resolve_profiles_records_skip_when_profile_missing(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    def missing_profile(profile_id):
        raise ValueError(f"unknown tensor profile: {profile_id}")

    monkeypatch.setattr(gauntlet, "get_tensor_profile", missing_profile)

    resolved = gauntlet.resolve_profile_requests(("standard_full_v2",), require_profiles=False)

    assert len(resolved) == 1
    assert resolved[0].requested_profile == "standard_full_v2"
    assert resolved[0].profile is None
    assert resolved[0].available is False
    assert "unknown tensor profile" in resolved[0].skip_reason


def test_resolve_profiles_raises_when_required_profile_missing(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    def missing_profile(profile_id):
        raise ValueError(f"unknown tensor profile: {profile_id}")

    monkeypatch.setattr(gauntlet, "get_tensor_profile", missing_profile)

    with pytest.raises(ValueError, match="standard_full_v2"):
        gauntlet.resolve_profile_requests(("standard_full_v2",), require_profiles=True)


def test_memory_estimate_uses_tensor_size_and_rollout_shape():
    from digimon_gym.agents.tensor_profile_gauntlet import estimate_memory_footprint

    profile = fake_profile("standard_full_v2", 43008)

    memory = estimate_memory_footprint(profile, n_steps=128, n_envs=4)

    assert memory["tensor_bytes"] == 43008 * 4
    assert memory["tensor_kib"] == pytest.approx((43008 * 4) / 1024)
    assert memory["rollout_observation_bytes"] == 43008 * 4 * 128 * 4
    assert memory["rollout_observation_mib"] == pytest.approx((43008 * 4 * 128 * 4) / 1024 / 1024)
    assert memory["card_embedding_input_slots"] == 542
    assert memory["scalar_input_slots"] == 42466
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py -q
```

Expected: FAIL with `ModuleNotFoundError` or missing attributes for `digimon_gym.agents.tensor_profile_gauntlet`.

- [ ] **Step 3: Add result types and profile helpers**

Create `code/digimon_gym/agents/tensor_profile_gauntlet.py` with:

```python
"""Tensor profile profiling gauntlet for RL observation layouts."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Callable, Iterable, Mapping, Sequence

import json
import time

from digimon_gym.tensor_profiles import TensorProfile, get_tensor_profile


DEFAULT_PROFILE_REQUESTS = (
    "compact_v1",
    "standard_lite_v2",
    "standard_full_v2",
)


@dataclass(frozen=True)
class ResolvedProfile:
    requested_profile: str
    profile: TensorProfile | Any | None
    available: bool
    skip_reason: str = ""


@dataclass(frozen=True)
class TensorProfileRunConfig:
    profiles: tuple[str, ...] = DEFAULT_PROFILE_REQUESTS
    games_per_profile: int = 25
    seeds: tuple[int, ...] = tuple(range(101, 126))
    max_steps_per_game: int = 1000
    policy: str = "greedy"
    require_profiles: bool = False
    n_steps: int = 128
    n_envs: int = 1


@dataclass(frozen=True)
class TensorProfileRunResult:
    requested_profile: str
    profile_id: str
    available: bool
    skip_reason: str
    tensor_size: int
    layout_hash: str
    feature_schema_version: str
    memory_footprint: dict[str, float | int]
    games_played: int
    steps: int
    elapsed_seconds: float
    wins: int
    losses: int
    draws: int
    trigger_order_correct: int
    trigger_order_total: int

    @property
    def steps_per_second(self) -> float:
        return self.steps / self.elapsed_seconds if self.elapsed_seconds > 0 else 0.0

    @property
    def games_per_hour(self) -> float:
        return self.games_played / (self.elapsed_seconds / 3600.0) if self.elapsed_seconds > 0 else 0.0

    @property
    def win_rate_vs_greedy(self) -> float:
        return self.wins / self.games_played if self.games_played else 0.0

    @property
    def trigger_order_accuracy(self) -> float:
        return self.trigger_order_correct / self.trigger_order_total if self.trigger_order_total else 0.0

    def to_dict(self) -> dict[str, Any]:
        data = asdict(self)
        data["steps_per_second"] = self.steps_per_second
        data["games_per_hour"] = self.games_per_hour
        data["win_rate_vs_greedy"] = self.win_rate_vs_greedy
        data["trigger_order_accuracy"] = self.trigger_order_accuracy
        return data


@dataclass(frozen=True)
class TensorProfileGauntletResult:
    config: TensorProfileRunConfig
    results: tuple[TensorProfileRunResult, ...]

    def to_dict(self) -> dict[str, Any]:
        return {
            "config": asdict(self.config),
            "results": [result.to_dict() for result in self.results],
        }

    def write_json(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.to_dict(), indent=2, sort_keys=True), encoding="utf-8")


def resolve_profile_requests(
    profile_ids: Iterable[str],
    require_profiles: bool,
) -> list[ResolvedProfile]:
    resolved: list[ResolvedProfile] = []
    for requested in profile_ids:
        try:
            profile = get_tensor_profile(requested)
        except Exception as exc:
            if require_profiles:
                raise ValueError(f"required tensor profile {requested!r} is unavailable: {exc}") from exc
            resolved.append(
                ResolvedProfile(
                    requested_profile=requested,
                    profile=None,
                    available=False,
                    skip_reason=str(exc),
                )
            )
            continue
        resolved.append(
            ResolvedProfile(
                requested_profile=requested,
                profile=profile,
                available=True,
            )
        )
    return resolved


def estimate_memory_footprint(
    profile: TensorProfile | Any,
    n_steps: int,
    n_envs: int,
) -> dict[str, float | int]:
    tensor_bytes = int(profile.tensor_size) * 4
    rollout_observation_bytes = tensor_bytes * int(n_steps) * int(n_envs)
    return {
        "tensor_bytes": tensor_bytes,
        "tensor_kib": tensor_bytes / 1024,
        "rollout_observation_bytes": rollout_observation_bytes,
        "rollout_observation_mib": rollout_observation_bytes / 1024 / 1024,
        "card_embedding_input_slots": int(profile.card_id_slot_count),
        "scalar_input_slots": int(profile.scalar_slot_count),
    }
```

- [ ] **Step 4: Run the tests and verify they pass**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py -q
```

Expected: PASS for the four tests in this task.

- [ ] **Step 5: Commit**

Run:

```powershell
git add code/digimon_gym/agents/tensor_profile_gauntlet.py code/tests/rl/test_tensor_profile_gauntlet.py
git commit -m "test: add tensor profile gauntlet result foundation"
```

---

### Task 2: Fixed-Seed Game Benchmark Runner

**Files:**
- Modify: `code/digimon_gym/agents/tensor_profile_gauntlet.py`
- Modify: `code/tests/rl/test_tensor_profile_gauntlet.py`

- [ ] **Step 1: Write failing tests for deterministic game metrics**

Append to `code/tests/rl/test_tensor_profile_gauntlet.py`:

```python
class FakeEnv:
    def __init__(self, deck1=None, deck2=None, tensor_profile=None):
        self.tensor_profile = tensor_profile
        self.current_player_id = 1
        self.winner_id = None
        self.is_game_over = False
        self._steps = 0

    def reset(self, seed=None):
        self._steps = 0
        self.winner_id = None
        self.is_game_over = False
        return [0.0], {"tensor_profile": self.tensor_profile}

    def step(self, action):
        self._steps += 1
        terminated = self._steps >= 3
        if terminated:
            self.is_game_over = True
            self.winner_id = 1 if action == 62 else 2
        return [0.0], 0.0, terminated, False, {}

    def action_mask(self):
        mask = [0] * 2168
        mask[62] = 1
        return mask


def test_run_profile_games_counts_steps_wins_and_elapsed(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    monkeypatch.setattr(gauntlet, "DigimonEnv", FakeEnv)
    monkeypatch.setattr(gauntlet, "greedy_policy", lambda env: 62)

    profile = fake_profile("standard_lite_v2", 8320)
    clock = clock_from((10.0, 12.0))
    result = gauntlet.run_profile_games(
        requested_profile="standard_lite_v2",
        profile=profile,
        config=gauntlet.TensorProfileRunConfig(
            profiles=("standard_lite_v2",),
            games_per_profile=2,
            seeds=(11, 12),
            max_steps_per_game=10,
            policy="greedy",
        ),
        clock=clock,
    )

    assert result.profile_id == "standard_lite_v2"
    assert result.games_played == 2
    assert result.steps == 6
    assert result.wins == 2
    assert result.losses == 0
    assert result.draws == 0


def test_run_profile_games_marks_step_cap_as_draw(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    monkeypatch.setattr(gauntlet, "DigimonEnv", FakeEnv)
    monkeypatch.setattr(gauntlet, "greedy_policy", lambda env: 62)

    profile = fake_profile("standard_lite_v2", 8320)
    clock = clock_from((20.0, 21.0))
    result = gauntlet.run_profile_games(
        requested_profile="standard_lite_v2",
        profile=profile,
        config=gauntlet.TensorProfileRunConfig(
            profiles=("standard_lite_v2",),
            games_per_profile=1,
            seeds=(11,),
            max_steps_per_game=2,
            policy="greedy",
        ),
        clock=clock,
    )

    assert result.games_played == 1
    assert result.steps == 2
    assert result.wins == 0
    assert result.losses == 0
    assert result.draws == 1
```

- [ ] **Step 2: Run the tests and verify they fail**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py -q
```

Expected: FAIL because `run_profile_games` and `DigimonEnv` imports are not implemented.

- [ ] **Step 3: Add deterministic clock helper to the tests**

Add this helper near `FakeEnv` in `code/tests/rl/test_tensor_profile_gauntlet.py`:

```python
def clock_from(values):
    iterator = iter(values)
    return lambda: next(iterator)
```

- [ ] **Step 4: Add game-loop imports and policy selection**

In `code/digimon_gym/agents/tensor_profile_gauntlet.py`, add these imports below the existing imports:

```python
from digimon_gym.digimon_gym import DigimonEnv, greedy_policy
from digimon_gym.agents.pilot_training import random_policy
```

Add these constants and helpers after `DEFAULT_PROFILE_REQUESTS`:

```python
DEFAULT_DECK = ("ST1-01",) * 5 + ("ST1-03",) * 45


def _policy_fn(name: str) -> Callable[[Any], int]:
    if name == "greedy":
        return greedy_policy
    if name == "random":
        return random_policy
    raise ValueError(f"unknown tensor profile gauntlet policy: {name}")
```

- [ ] **Step 5: Add `run_profile_games` implementation**

Append to `code/digimon_gym/agents/tensor_profile_gauntlet.py`:

```python
def run_profile_games(
    requested_profile: str,
    profile: TensorProfile | Any,
    config: TensorProfileRunConfig,
    clock: Callable[[], float] | None = None,
) -> TensorProfileRunResult:
    now = clock or time.perf_counter
    policy_fn = _policy_fn(config.policy)
    seeds = tuple(config.seeds[: config.games_per_profile])
    start = now()

    games_played = 0
    steps = 0
    wins = 0
    losses = 0
    draws = 0

    for seed in seeds:
        env = DigimonEnv(
            deck1=list(DEFAULT_DECK),
            deck2=list(DEFAULT_DECK),
            tensor_profile=profile.id,
        )
        env.reset(seed=seed)
        terminated = False
        truncated = False
        game_steps = 0

        while not (terminated or truncated) and game_steps < config.max_steps_per_game:
            action = int(policy_fn(env))
            _obs, _reward, terminated, truncated, _info = env.step(action)
            steps += 1
            game_steps += 1

        games_played += 1
        if game_steps >= config.max_steps_per_game and not getattr(env, "is_game_over", False):
            draws += 1
            continue
        winner_id = getattr(env, "winner_id", None)
        if winner_id == 1:
            wins += 1
        elif winner_id == 2:
            losses += 1
        else:
            draws += 1

    elapsed = max(now() - start, 1e-9)
    trigger_order_correct, trigger_order_total = score_trigger_order_accuracy(profile)
    return TensorProfileRunResult(
        requested_profile=requested_profile,
        profile_id=str(profile.id),
        available=True,
        skip_reason="",
        tensor_size=int(profile.tensor_size),
        layout_hash=str(profile.layout_hash),
        feature_schema_version=str(profile.feature_schema_version),
        memory_footprint=estimate_memory_footprint(
            profile,
            n_steps=config.n_steps,
            n_envs=config.n_envs,
        ),
        games_played=games_played,
        steps=steps,
        elapsed_seconds=elapsed,
        wins=wins,
        losses=losses,
        draws=draws,
        trigger_order_correct=trigger_order_correct,
        trigger_order_total=trigger_order_total,
    )
```

Add a temporary trigger-order scoring helper so Task 2 tests can pass:

```python
def score_trigger_order_accuracy(profile: TensorProfile | Any) -> tuple[int, int]:
    if str(profile.id) == "standard_compact_v1":
        return (0, 1)
    if str(profile.id) == "standard_lite_v2":
        return (1, 1)
    if str(profile.id) == "standard_full_v2":
        return (1, 1)
    return (0, 0)
```

- [ ] **Step 6: Run the tests and verify they pass**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py -q
```

Expected: PASS for the existing tests and the new game-loop tests.

- [ ] **Step 7: Commit**

Run:

```powershell
git add code/digimon_gym/agents/tensor_profile_gauntlet.py code/tests/rl/test_tensor_profile_gauntlet.py
git commit -m "feat: measure tensor profile game throughput"
```

---

### Task 3: Trigger-Order Signal Probe Scoring

**Files:**
- Modify: `code/digimon_gym/agents/tensor_profile_gauntlet.py`
- Modify: `code/tests/rl/test_tensor_profile_gauntlet.py`

- [ ] **Step 1: Write failing tests for trigger-order signal accuracy**

Append to `code/tests/rl/test_tensor_profile_gauntlet.py`:

```python
def profile_with_sections(profile_id: str, tensor_size: int, sections):
    profile = fake_profile(profile_id, tensor_size)
    profile.sections = tuple(sections)
    return profile


def section(name: str, offset: int, size: int, shape):
    return SimpleNamespace(name=name, offset=offset, size=size, shape=tuple(shape))


def test_trigger_order_accuracy_compact_profile_has_no_signal():
    from digimon_gym.agents.tensor_profile_gauntlet import score_trigger_order_accuracy

    profile = fake_profile("standard_compact_v1", 1375)

    correct, total = score_trigger_order_accuracy(profile)

    assert correct == 0
    assert total == 1


def test_trigger_order_accuracy_lite_profile_scores_pending_choice_section():
    from digimon_gym.agents.tensor_profile_gauntlet import score_trigger_order_accuracy

    profile = profile_with_sections(
        "standard_lite_v2",
        8320,
        [section("pending_choice_features", 4992, 3072, (32, 96))],
    )

    correct, total = score_trigger_order_accuracy(profile)

    assert correct == 1
    assert total == 1


def test_trigger_order_accuracy_full_profile_scores_pending_and_action_rows():
    from digimon_gym.agents.tensor_profile_gauntlet import score_trigger_order_accuracy

    profile = profile_with_sections(
        "standard_full_v2",
        43008,
        [
            section("pending_choice_features", 4992, 3072, (32, 96)),
            section("action_id_features", 8064, 34688, (2168, 16)),
        ],
    )

    correct, total = score_trigger_order_accuracy(profile)

    assert correct == 2
    assert total == 2


def test_trigger_order_accuracy_rejects_full_profile_without_action_rows():
    from digimon_gym.agents.tensor_profile_gauntlet import score_trigger_order_accuracy

    profile = profile_with_sections(
        "standard_full_v2",
        43008,
        [section("pending_choice_features", 4992, 3072, (32, 96))],
    )

    correct, total = score_trigger_order_accuracy(profile)

    assert correct == 1
    assert total == 2
```

- [ ] **Step 2: Run the tests and verify they fail**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py -q
```

Expected: FAIL because the temporary `score_trigger_order_accuracy` does not inspect sections.

- [ ] **Step 3: Replace trigger-order scoring with section probes**

In `code/digimon_gym/agents/tensor_profile_gauntlet.py`, replace `score_trigger_order_accuracy` with:

```python
def score_trigger_order_accuracy(profile: TensorProfile | Any) -> tuple[int, int]:
    profile_id = str(profile.id)
    section_names = {_section_name(section) for section in getattr(profile, "sections", ())}

    if profile_id == "standard_compact_v1":
        return (0, 1)

    correct = 0
    total = 1
    if "pending_choice_features" in section_names:
        correct += 1

    if profile_id == "standard_full_v2":
        total += 1
        if "action_id_features" in section_names:
            correct += 1

    return (correct, total)


def _section_name(section: Any) -> str:
    return str(getattr(section, "name", getattr(section, "id", "")))
```

- [ ] **Step 4: Run the tests and verify they pass**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py -q
```

Expected: PASS for the unit test suite.

- [ ] **Step 5: Commit**

Run:

```powershell
git add code/digimon_gym/agents/tensor_profile_gauntlet.py code/tests/rl/test_tensor_profile_gauntlet.py
git commit -m "feat: score tensor profile trigger-order signals"
```

---

### Task 4: Whole-Gauntlet Orchestration And Serialization

**Files:**
- Modify: `code/digimon_gym/agents/tensor_profile_gauntlet.py`
- Modify: `code/tests/rl/test_tensor_profile_gauntlet.py`

- [ ] **Step 1: Write failing tests for whole-result orchestration and Markdown**

Append to `code/tests/rl/test_tensor_profile_gauntlet.py`:

```python
def test_run_tensor_profile_gauntlet_includes_unavailable_profiles(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    available = fake_profile("standard_lite_v2", 8320)

    def resolve(profile_ids, require_profiles):
        return [
            gauntlet.ResolvedProfile("standard_lite_v2", available, True, ""),
            gauntlet.ResolvedProfile("missing_profile", None, False, "unknown tensor profile"),
        ]

    def run_profile_games(requested_profile, profile, config, clock=None):
        return gauntlet.TensorProfileRunResult(
            requested_profile=requested_profile,
            profile_id=profile.id,
            available=True,
            skip_reason="",
            tensor_size=profile.tensor_size,
            layout_hash=profile.layout_hash,
            feature_schema_version=profile.feature_schema_version,
            memory_footprint=gauntlet.estimate_memory_footprint(profile, 128, 1),
            games_played=1,
            steps=3,
            elapsed_seconds=1.5,
            wins=1,
            losses=0,
            draws=0,
            trigger_order_correct=1,
            trigger_order_total=1,
        )

    monkeypatch.setattr(gauntlet, "resolve_profile_requests", resolve)
    monkeypatch.setattr(gauntlet, "run_profile_games", run_profile_games)

    result = gauntlet.run_tensor_profile_gauntlet(
        gauntlet.TensorProfileRunConfig(
            profiles=("standard_lite_v2", "missing_profile"),
            games_per_profile=1,
            seeds=(1,),
        )
    )

    assert len(result.results) == 2
    assert result.results[0].available is True
    assert result.results[0].steps_per_second == pytest.approx(2.0)
    assert result.results[1].available is False
    assert result.results[1].skip_reason == "unknown tensor profile"


def test_gauntlet_result_writes_json_and_markdown(tmp_path):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    profile = fake_profile("standard_lite_v2", 8320)
    run_result = gauntlet.TensorProfileRunResult(
        requested_profile="standard_lite_v2",
        profile_id="standard_lite_v2",
        available=True,
        skip_reason="",
        tensor_size=8320,
        layout_hash=profile.layout_hash,
        feature_schema_version="standard_lite_v2.1",
        memory_footprint=gauntlet.estimate_memory_footprint(profile, 128, 1),
        games_played=2,
        steps=6,
        elapsed_seconds=3.0,
        wins=1,
        losses=1,
        draws=0,
        trigger_order_correct=1,
        trigger_order_total=1,
    )
    result = gauntlet.TensorProfileGauntletResult(
        config=gauntlet.TensorProfileRunConfig(profiles=("standard_lite_v2",)),
        results=(run_result,),
    )

    json_path = tmp_path / "result.json"
    md_path = tmp_path / "result.md"
    result.write_json(json_path)
    result.write_markdown(md_path)

    assert json_path.read_text(encoding="utf-8").startswith("{")
    markdown = md_path.read_text(encoding="utf-8")
    assert "| Profile | Tensor Size | Steps/sec | Games/hour | Win Rate vs Greedy | Trigger Accuracy | Tensor KiB |" in markdown
    assert "| standard_lite_v2 | 8320 | 2.00 | 2400.00 | 50.00% | 100.00% | 32.50 |" in markdown
```

- [ ] **Step 2: Run the tests and verify they fail**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py -q
```

Expected: FAIL because `run_tensor_profile_gauntlet` and `write_markdown` do not exist.

- [ ] **Step 3: Add unavailable-result constructor and orchestration**

Append to `code/digimon_gym/agents/tensor_profile_gauntlet.py`:

```python
def unavailable_result(resolved: ResolvedProfile, config: TensorProfileRunConfig) -> TensorProfileRunResult:
    return TensorProfileRunResult(
        requested_profile=resolved.requested_profile,
        profile_id="",
        available=False,
        skip_reason=resolved.skip_reason,
        tensor_size=0,
        layout_hash="",
        feature_schema_version="",
        memory_footprint={
            "tensor_bytes": 0,
            "tensor_kib": 0.0,
            "rollout_observation_bytes": 0,
            "rollout_observation_mib": 0.0,
            "card_embedding_input_slots": 0,
            "scalar_input_slots": 0,
        },
        games_played=0,
        steps=0,
        elapsed_seconds=0.0,
        wins=0,
        losses=0,
        draws=0,
        trigger_order_correct=0,
        trigger_order_total=0,
    )


def run_tensor_profile_gauntlet(config: TensorProfileRunConfig) -> TensorProfileGauntletResult:
    results: list[TensorProfileRunResult] = []
    for resolved in resolve_profile_requests(config.profiles, config.require_profiles):
        if not resolved.available or resolved.profile is None:
            results.append(unavailable_result(resolved, config))
            continue
        results.append(
            run_profile_games(
                requested_profile=resolved.requested_profile,
                profile=resolved.profile,
                config=config,
            )
        )
    return TensorProfileGauntletResult(config=config, results=tuple(results))
```

- [ ] **Step 4: Add Markdown serialization**

Inside `TensorProfileGauntletResult`, after `write_json`, add:

```python
    def to_markdown(self) -> str:
        lines = [
            "# Tensor Profile Gauntlet",
            "",
            "| Profile | Tensor Size | Steps/sec | Games/hour | Win Rate vs Greedy | Trigger Accuracy | Tensor KiB |",
            "|---|---:|---:|---:|---:|---:|---:|",
        ]
        for result in self.results:
            profile_label = result.profile_id or result.requested_profile
            if not result.available:
                lines.append(f"| {profile_label} | unavailable | 0.00 | 0.00 | 0.00% | 0.00% | 0.00 |")
                continue
            lines.append(
                "| "
                + " | ".join(
                    [
                        profile_label,
                        str(result.tensor_size),
                        f"{result.steps_per_second:.2f}",
                        f"{result.games_per_hour:.2f}",
                        f"{result.win_rate_vs_greedy:.2%}",
                        f"{result.trigger_order_accuracy:.2%}",
                        f"{float(result.memory_footprint['tensor_kib']):.2f}",
                    ]
                )
                + " |"
            )
        skipped = [result for result in self.results if not result.available]
        if skipped:
            lines.extend(["", "## Skipped Profiles", ""])
            for result in skipped:
                lines.append(f"- `{result.requested_profile}`: {result.skip_reason}")
        lines.append("")
        return "\n".join(lines)

    def write_markdown(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(self.to_markdown(), encoding="utf-8")
```

- [ ] **Step 5: Run the tests and verify they pass**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py -q
```

Expected: PASS for all gauntlet unit tests.

- [ ] **Step 6: Commit**

Run:

```powershell
git add code/digimon_gym/agents/tensor_profile_gauntlet.py code/tests/rl/test_tensor_profile_gauntlet.py
git commit -m "feat: orchestrate tensor profile gauntlet results"
```

---

### Task 5: CLI Entrypoint And Reports

**Files:**
- Create: `code/tools/profile_tensor_profiles.py`
- Modify: `code/tests/rl/test_tensor_profile_gauntlet.py`

- [ ] **Step 1: Write failing CLI parser test**

Append to `code/tests/rl/test_tensor_profile_gauntlet.py`:

```python
def test_cli_parses_profiles_and_output_directory():
    from tools.profile_tensor_profiles import parse_args

    args = parse_args(
        [
            "--profiles",
            "compact_v1,standard_lite_v2,standard_full_v2",
            "--games",
            "7",
            "--seeds",
            "200:207",
            "--max-steps-per-game",
            "500",
            "--policy",
            "greedy",
            "--n-steps",
            "64",
            "--n-envs",
            "2",
            "--out",
            "profile_runs/tensor_profiles/test",
            "--require-profiles",
        ]
    )

    assert args.profiles == "compact_v1,standard_lite_v2,standard_full_v2"
    assert args.games == 7
    assert args.seeds == "200:207"
    assert args.max_steps_per_game == 500
    assert args.policy == "greedy"
    assert args.n_steps == 64
    assert args.n_envs == 2
    assert str(args.out).endswith("profile_runs\\tensor_profiles\\test") or str(args.out).endswith("profile_runs/tensor_profiles/test")
    assert args.require_profiles is True
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py::test_cli_parses_profiles_and_output_directory -q
```

Expected: FAIL because `code/tools/profile_tensor_profiles.py` does not exist.

- [ ] **Step 3: Create the CLI file**

Create `code/tools/profile_tensor_profiles.py` with:

```python
from __future__ import annotations

import argparse
from pathlib import Path

from digimon_gym.agents.tensor_profile_gauntlet import (
    DEFAULT_PROFILE_REQUESTS,
    TensorProfileRunConfig,
    run_tensor_profile_gauntlet,
)


def parse_seed_range(raw: str) -> tuple[int, ...]:
    if ":" in raw:
        start_raw, stop_raw = raw.split(":", 1)
        return tuple(range(int(start_raw), int(stop_raw)))
    return tuple(int(part.strip()) for part in raw.split(",") if part.strip())


def parse_args(argv: list[str] | None = None):
    parser = argparse.ArgumentParser(
        description="Compare board-state tensor profiles with fixed-seed RL gauntlet metrics."
    )
    parser.add_argument(
        "--profiles",
        default=",".join(DEFAULT_PROFILE_REQUESTS),
        help="Comma-separated tensor profile IDs to compare.",
    )
    parser.add_argument("--games", type=int, default=25, help="Games per profile.")
    parser.add_argument(
        "--seeds",
        default="101:126",
        help="Seed range start:stop or comma-separated seed list.",
    )
    parser.add_argument(
        "--max-steps-per-game",
        type=int,
        default=1000,
        help="Step cap per game; capped games count as draws.",
    )
    parser.add_argument(
        "--policy",
        choices=["greedy", "random"],
        default="greedy",
        help="Policy used for player 1 during benchmark games.",
    )
    parser.add_argument(
        "--n-steps",
        type=int,
        default=128,
        help="Rollout step count used for memory footprint estimates.",
    )
    parser.add_argument(
        "--n-envs",
        type=int,
        default=1,
        help="Vectorized env count used for memory footprint estimates.",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("profile_runs") / "tensor_profiles" / "latest",
        help="Directory where result.json and result.md are written.",
    )
    parser.add_argument(
        "--require-profiles",
        action="store_true",
        help="Fail if any requested profile is unavailable.",
    )
    return parser.parse_args(argv)


def config_from_args(args) -> TensorProfileRunConfig:
    return TensorProfileRunConfig(
        profiles=tuple(part.strip() for part in args.profiles.split(",") if part.strip()),
        games_per_profile=args.games,
        seeds=parse_seed_range(args.seeds),
        max_steps_per_game=args.max_steps_per_game,
        policy=args.policy,
        require_profiles=args.require_profiles,
        n_steps=args.n_steps,
        n_envs=args.n_envs,
    )


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    result = run_tensor_profile_gauntlet(config_from_args(args))
    args.out.mkdir(parents=True, exist_ok=True)
    json_path = args.out / "result.json"
    markdown_path = args.out / "result.md"
    result.write_json(json_path)
    result.write_markdown(markdown_path)
    print(result.to_markdown())
    print(f"Wrote {json_path}")
    print(f"Wrote {markdown_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run CLI tests and verify they pass**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py::test_cli_parses_profiles_and_output_directory -q
```

Expected: PASS.

- [ ] **Step 5: Run a tiny CLI smoke command**

Run:

```powershell
python code/tools/profile_tensor_profiles.py --profiles compact_v1,standard_lite_v2,standard_full_v2 --games 1 --seeds 301:302 --max-steps-per-game 50 --out profile_runs/tensor_profiles/smoke --require-profiles
```

Expected: command exits `0`, prints a Markdown table, and writes:

```text
profile_runs/tensor_profiles/smoke/result.json
profile_runs/tensor_profiles/smoke/result.md
```

- [ ] **Step 6: Commit**

Run:

```powershell
git add code/tools/profile_tensor_profiles.py code/tests/rl/test_tensor_profile_gauntlet.py
git commit -m "feat: add tensor profile gauntlet CLI"
```

---

### Task 6: Registered Profile Smoke Coverage

**Files:**
- Modify: `code/tests/rl/test_tensor_profile_gauntlet.py`

- [ ] **Step 1: Write smoke test for all three real profiles**

Append to `code/tests/rl/test_tensor_profile_gauntlet.py`:

```python
def test_real_profile_resolution_includes_compact_lite_and_full():
    pytest.importorskip("digimon_engine")
    from digimon_gym.agents.tensor_profile_gauntlet import resolve_profile_requests

    resolved = resolve_profile_requests(
        ("compact_v1", "standard_lite_v2", "standard_full_v2"),
        require_profiles=True,
    )

    assert [item.profile.id for item in resolved] == [
        "standard_compact_v1",
        "standard_lite_v2",
        "standard_full_v2",
    ]
    assert [item.profile.tensor_size for item in resolved] == [1375, 8320, 43008]
```

- [ ] **Step 2: Run the smoke test and verify it passes**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py::test_real_profile_resolution_includes_compact_lite_and_full -q
```

Expected: PASS if the local PyO3 wheel is rebuilt from current main; SKIP if `digimon_engine` is not installed.

- [ ] **Step 3: Run all gauntlet tests**

Run:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py -q
```

Expected: PASS, with real-profile smoke skipped only if the local Rust bindings are unavailable.

- [ ] **Step 4: Commit**

Run:

```powershell
git add code/tests/rl/test_tensor_profile_gauntlet.py
git commit -m "test: cover registered tensor profile gauntlet inputs"
```

---

### Task 7: Documentation

**Files:**
- Modify: `docs/TOOLS.md`

- [ ] **Step 1: Write the docs update**

Add this section to `docs/TOOLS.md` near other RL/training tools:

```markdown
## Tensor Profile Gauntlet

Compare board-state tensor profiles with fixed-seed RL profiling metrics:

```powershell
python code/tools/profile_tensor_profiles.py --profiles compact_v1,standard_lite_v2,standard_full_v2 --games 100 --seeds 1000:1100 --policy greedy --out profile_runs/tensor_profiles/latest --require-profiles
```

The default profile set is:

- `compact_v1`, reported as canonical `standard_compact_v1`
- `standard_lite_v2`
- `standard_full_v2`

The gauntlet writes `result.json` and `result.md`. Each profile row includes:

- steps/sec
- games/hour
- win rate versus greedy
- trigger-order signal accuracy
- tensor bytes and rollout observation memory estimates

Use `--games 1 --seeds 301:302 --max-steps-per-game 50` for a smoke run. Use larger fixed seed ranges for evidence intended to compare profile tradeoffs.
```

- [ ] **Step 2: Run docs grep to verify command appears**

Run:

```powershell
Select-String -Path docs\\TOOLS.md -Pattern "Tensor Profile Gauntlet","profile_tensor_profiles.py"
```

Expected: output includes the new heading and command.

- [ ] **Step 3: Commit**

Run:

```powershell
git add docs/TOOLS.md
git commit -m "docs: document tensor profile gauntlet"
```

---

## Verification

Run these from the repository root after all tasks:

```powershell
python -m pytest code/tests/rl/test_tensor_profile_gauntlet.py -q
python -m pytest code/tests/rl/test_tensor_profiles.py -q
python code/tools/profile_tensor_profiles.py --profiles compact_v1,standard_lite_v2,standard_full_v2 --games 1 --seeds 301:302 --max-steps-per-game 50 --out profile_runs/tensor_profiles/smoke --require-profiles
```

Expected:

- Both pytest commands pass, except tests marked with `pytest.importorskip("digimon_engine")` skip when Rust bindings are unavailable.
- The CLI exits `0`, prints a Markdown table, and writes `profile_runs/tensor_profiles/smoke/result.json` plus `result.md`.

## Self-Review Checklist

- Spec coverage: the plan compares compact, lite v2, and full v2 on steps/sec, games/hour, win rate vs greedy, trigger-order accuracy, and memory footprint.
- No hosted infrastructure: all new code stays under `digimon_gym` and `code/tools`.
- Tensor contracts: profile sizes and aliases come from the existing registry through `get_tensor_profile`.
- TDD: every behavior starts with a failing test before implementation.
- No profile approximations: gameplay still flows through `DigimonEnv` and engine action masks; the gauntlet measures profiles without bypassing legal action selection.

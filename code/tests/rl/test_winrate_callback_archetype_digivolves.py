"""Per-archetype digivolve telemetry on `WinRateCallback`.

Covers the surface added by openspec change `digivolve-eval-telemetry`:
- per-archetype TB scalars under `pilot/agent_archetype/<X>/...` and
  `pilot/archetype/<X>/...`
- extended `by_archetype` sidecar block carrying `digivolves`,
  `dna_digivolves`, `opponent_digivolves`, `opponent_dna_digivolves`
- four new top-level `mean_eval_*_per_game` sidecar fields
- zero-default behavior when shaping is off and no digivolves occur
- forward-compatibility — pre-change sidecar rows still parse

The fakes mirror the patterns established in `test_eval_suite.py`.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence

import numpy as np
import pytest


# ─── Test doubles ────────────────────────────────────────────────────


class _FakeLogger:
    def __init__(self) -> None:
        self.records: Dict[str, Any] = {}

    def record(self, key: str, value: Any) -> None:
        self.records[key] = value


class _FakeModel:
    def __init__(self) -> None:
        self.logger = _FakeLogger()

    def predict(self, obs, **kwargs):
        return 0, kwargs.get("state")


class _FakeEvalEnv:
    """Drives the eval loop through a scripted sequence of episodes.

    Each `episodes[i]` is `(rewards, winner_id, info)`:
    - `rewards`: per-step reward sequence; len(rewards) = episode length
    - `winner_id`: int set on `self.winner_id` at terminal step (1 / 2 / None)
    - `info`: dict returned from `reset()` — may carry `opponent_archetype`
      and/or `deck1_archetype`

    Each `per_episode_state[i]` is the dict returned by `_rl_state()` AFTER
    that episode's terminal step. Pass `None` to omit `_rl_state` (mimics
    a backend that doesn't expose digivolve keys).
    """

    def __init__(
        self,
        episodes: Sequence[tuple],
        per_episode_state: Optional[Sequence[Optional[Dict[str, int]]]] = None,
    ) -> None:
        self.episodes = list(episodes)
        self._per_episode_state = (
            list(per_episode_state)
            if per_episode_state is not None
            else [None] * len(self.episodes)
        )
        self.episode_index = -1
        self.step_index = 0
        self.winner_id: Optional[int] = None

    def reset(self):
        self.episode_index += 1
        self.step_index = 0
        self.winner_id = None
        _, _, info = self.episodes[self.episode_index]
        return np.zeros(1, dtype=np.float32), dict(info)

    def action_mask(self):
        return np.ones(4, dtype=np.int8)

    def step(self, _action):
        rewards, winner, _ = self.episodes[self.episode_index]
        reward = rewards[self.step_index]
        self.step_index += 1
        terminated = self.step_index == len(rewards)
        if terminated:
            self.winner_id = winner
        return np.zeros(1, dtype=np.float32), reward, terminated, False, {}

    def _rl_state(self):
        state = self._per_episode_state[self.episode_index]
        if state is None:
            raise AttributeError("no _rl_state for this episode")
        return state

    def close(self):
        pass


def _drive(env, monkeypatch, *, n_eval_episodes: int, sidecar_path: Optional[Path] = None):
    """Build a `WinRateCallback`, neutralize the unwrap helper, and run
    one eval. Returns the callback for assertions."""
    from digimon_gym.agents import pilot_training
    from digimon_gym.agents.pilot_training import WinRateCallback

    monkeypatch.setattr(
        pilot_training, "_unwrap_to_digimon_env", lambda wrapped: wrapped
    )

    cb = WinRateCallback(
        eval_env_fn=lambda: env,
        eval_freq=1,
        n_eval_episodes=n_eval_episodes,
        eval_sidecar_path=sidecar_path,
        verbose=0,
    )
    cb.model = _FakeModel()
    cb._run_evaluation()
    return cb


def _read_sidecar(path: Path) -> List[Dict[str, Any]]:
    """Read JSONL sidecar into a list of dicts."""
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


# ─── Tests ───────────────────────────────────────────────────────────


def test_shape_stability_with_shaping_off(monkeypatch, tmp_path):
    """All four top-level mean keys present and zero when no digivolves occur;
    `by_archetype` entries (if any) carry all four count keys with value 0."""
    sidecar = tmp_path / "evals.jsonl"
    env = _FakeEvalEnv(
        episodes=[
            ([1.0], 1, {"opponent_archetype": "Royal Knights",
                        "deck1_archetype": "DNA Omnimon"}),
            ([1.0], 2, {"opponent_archetype": "Royal Knights",
                        "deck1_archetype": "DNA Omnimon"}),
        ],
        per_episode_state=[
            {"p1_digivolutions": 0, "p1_dna_digivolutions": 0,
             "p2_digivolutions": 0, "p2_dna_digivolutions": 0},
            {"p1_digivolutions": 0, "p1_dna_digivolutions": 0,
             "p2_digivolutions": 0, "p2_dna_digivolutions": 0},
        ],
    )

    _drive(env, monkeypatch, n_eval_episodes=2, sidecar_path=sidecar)

    rows = _read_sidecar(sidecar)
    assert len(rows) == 1
    row = rows[0]
    for key in (
        "mean_eval_digivolves_per_game",
        "mean_eval_dna_digivolves_per_game",
        "mean_eval_opponent_digivolves_per_game",
        "mean_eval_opponent_dna_digivolves_per_game",
    ):
        assert key in row, f"missing top-level key: {key}"
        assert row[key] == 0.0, f"{key} should default to 0.0, got {row[key]}"

    assert "by_archetype" in row
    rk = row["by_archetype"]["Royal Knights"]
    for key in (
        "digivolves",
        "dna_digivolves",
        "opponent_digivolves",
        "opponent_dna_digivolves",
    ):
        assert key in rk, f"missing by_archetype key: {key}"
        assert rk[key] == 0, f"{key} should default to 0, got {rk[key]}"
    # Legacy wins fields still present and unchanged.
    assert rk["wins"] == 1
    assert rk["games"] == 2


def test_cumulative_per_archetype_accumulation(monkeypatch, tmp_path):
    """Two opponent archetypes × two agent archetypes; verify the per-archetype
    dicts and sidecar `by_archetype` accumulate counts correctly.

    Layout (4 games):
      1. Imperial vs Royal Knights → p1 digi=3,dna=1 / p2 digi=5,dna=0
      2. DNA Omnimon vs Royal Knights → p1 digi=2,dna=0 / p2 digi=4,dna=2
      3. Imperial vs Olympos → p1 digi=4,dna=2 / p2 digi=1,dna=0
      4. DNA Omnimon vs Olympos → p1 digi=0,dna=0 / p2 digi=6,dna=3

    Expected cumulative per opp archetype (sidecar by_archetype):
      Royal Knights: agent_digi=5,dna=1, opp_digi=9,dna=2, games=2
      Olympos:       agent_digi=4,dna=2, opp_digi=7,dna=3, games=2
    Expected cumulative per agent archetype (dicts only, no by-agent sidecar):
      Imperial:      digi=7,dna=3, games=2
      DNA Omnimon:   digi=2,dna=0, games=2
    """
    sidecar = tmp_path / "evals.jsonl"
    env = _FakeEvalEnv(
        episodes=[
            ([1.0], 1, {"opponent_archetype": "Royal Knights",
                        "deck1_archetype": "Imperial"}),
            ([1.0], 2, {"opponent_archetype": "Royal Knights",
                        "deck1_archetype": "DNA Omnimon"}),
            ([1.0], 1, {"opponent_archetype": "Olympos",
                        "deck1_archetype": "Imperial"}),
            ([1.0], 2, {"opponent_archetype": "Olympos",
                        "deck1_archetype": "DNA Omnimon"}),
        ],
        per_episode_state=[
            {"p1_digivolutions": 3, "p1_dna_digivolutions": 1,
             "p2_digivolutions": 5, "p2_dna_digivolutions": 0},
            {"p1_digivolutions": 2, "p1_dna_digivolutions": 0,
             "p2_digivolutions": 4, "p2_dna_digivolutions": 2},
            {"p1_digivolutions": 4, "p1_dna_digivolutions": 2,
             "p2_digivolutions": 1, "p2_dna_digivolutions": 0},
            {"p1_digivolutions": 0, "p1_dna_digivolutions": 0,
             "p2_digivolutions": 6, "p2_dna_digivolutions": 3},
        ],
    )

    cb = _drive(env, monkeypatch, n_eval_episodes=4, sidecar_path=sidecar)

    # Per-archetype dicts (opp-keyed)
    assert cb._archetype_agent_digivolves == {"Royal Knights": 5, "Olympos": 4}
    assert cb._archetype_agent_dna_digivolves == {"Royal Knights": 1, "Olympos": 2}
    assert cb._archetype_opponent_digivolves == {"Royal Knights": 9, "Olympos": 7}
    assert cb._archetype_opponent_dna_digivolves == {"Royal Knights": 2, "Olympos": 3}
    # Per-archetype dicts (agent-keyed)
    assert cb._agent_archetype_digivolves == {"Imperial": 7, "DNA Omnimon": 2}
    assert cb._agent_archetype_dna_digivolves == {"Imperial": 3, "DNA Omnimon": 0}

    # Sidecar by_archetype
    row = _read_sidecar(sidecar)[0]
    rk = row["by_archetype"]["Royal Knights"]
    assert (rk["digivolves"], rk["dna_digivolves"]) == (5, 1)
    assert (rk["opponent_digivolves"], rk["opponent_dna_digivolves"]) == (9, 2)
    assert rk["games"] == 2
    op = row["by_archetype"]["Olympos"]
    assert (op["digivolves"], op["dna_digivolves"]) == (4, 2)
    assert (op["opponent_digivolves"], op["opponent_dna_digivolves"]) == (7, 3)


def test_top_level_means_match_counter_sums(monkeypatch, tmp_path):
    """`mean_eval_..._per_game` top-level fields equal the per-game counter
    sums divided by `n_eval_episodes`."""
    sidecar = tmp_path / "evals.jsonl"
    env = _FakeEvalEnv(
        episodes=[
            ([1.0], 1, {"opponent_archetype": "A", "deck1_archetype": "X"}),
            ([1.0], 2, {"opponent_archetype": "A", "deck1_archetype": "X"}),
            ([1.0], 1, {"opponent_archetype": "A", "deck1_archetype": "X"}),
            ([1.0], 2, {"opponent_archetype": "A", "deck1_archetype": "X"}),
        ],
        per_episode_state=[
            {"p1_digivolutions": 1, "p1_dna_digivolutions": 0,
             "p2_digivolutions": 2, "p2_dna_digivolutions": 1},
            {"p1_digivolutions": 3, "p1_dna_digivolutions": 1,
             "p2_digivolutions": 0, "p2_dna_digivolutions": 0},
            {"p1_digivolutions": 5, "p1_dna_digivolutions": 2,
             "p2_digivolutions": 4, "p2_dna_digivolutions": 0},
            {"p1_digivolutions": 1, "p1_dna_digivolutions": 0,
             "p2_digivolutions": 2, "p2_dna_digivolutions": 1},
        ],
    )
    # Sums: p1_digi = 10, p1_dna = 3, p2_digi = 8, p2_dna = 2; n = 4
    _drive(env, monkeypatch, n_eval_episodes=4, sidecar_path=sidecar)
    row = _read_sidecar(sidecar)[0]
    assert row["mean_eval_digivolves_per_game"] == 2.5
    assert row["mean_eval_dna_digivolves_per_game"] == 0.75
    assert row["mean_eval_opponent_digivolves_per_game"] == 2.0
    assert row["mean_eval_opponent_dna_digivolves_per_game"] == 0.5


def test_older_sidecar_row_forward_compatibility():
    """Pre-change rows lacking the four count keys must still parse cleanly
    via the documented lenient-reader contract."""
    legacy_row = (
        '{"step":1000,"wall_time":1700000000.0,"win_rate":0.5,'
        '"mean_reward":0.1,"draw_rate":0.0,"mean_terminal_score":0.0,'
        '"mean_dense_reward":0.1,"mean_eval_episode_length":42.0,'
        '"games_played":100,"by_archetype":{"Foo":{"wins":3,"draws":0,'
        '"games":5,"win_rate":0.6}}}'
    )
    row = json.loads(legacy_row)
    assert "mean_eval_digivolves_per_game" not in row
    assert "digivolves" not in row["by_archetype"]["Foo"]
    # Reader can safely default-zero anything missing without crashing.
    assert row.get("mean_eval_digivolves_per_game", 0.0) == 0.0
    assert row["by_archetype"]["Foo"].get("digivolves", 0) == 0


def test_no_agent_archetype_guard(monkeypatch, tmp_path):
    """Gauntlet-mode games (no `deck1_archetype` in info) must not write to
    `_agent_archetype_*` dicts and must not emit `pilot/agent_archetype/*`
    scalars for the agent side."""
    sidecar = tmp_path / "evals.jsonl"
    env = _FakeEvalEnv(
        episodes=[
            # Only opponent_archetype — no deck1_archetype.
            ([1.0], 1, {"opponent_archetype": "Gauntlet Opp"}),
        ],
        per_episode_state=[
            {"p1_digivolutions": 7, "p1_dna_digivolutions": 3,
             "p2_digivolutions": 4, "p2_dna_digivolutions": 1},
        ],
    )

    cb = _drive(env, monkeypatch, n_eval_episodes=1, sidecar_path=sidecar)

    # Agent-keyed dicts untouched.
    assert cb._agent_archetype_digivolves == {}
    assert cb._agent_archetype_dna_digivolves == {}
    # No agent-archetype TB scalars emitted.
    keys = set(cb.model.logger.records.keys())
    assert not any(k.startswith("pilot/agent_archetype/") for k in keys), (
        f"unexpected agent_archetype scalars: "
        f"{[k for k in keys if k.startswith('pilot/agent_archetype/')]}"
    )

    # BUT opponent-keyed dicts still receive the p1 AND p2 counts (because
    # by_archetype carries agent digivolves bucketed by opponent — that is
    # the contract from the spec, independent of whether deck1_archetype
    # is set).
    assert cb._archetype_agent_digivolves == {"Gauntlet Opp": 7}
    assert cb._archetype_opponent_digivolves == {"Gauntlet Opp": 4}

    # Per-opponent TB scalars are emitted.
    assert "pilot/archetype/Gauntlet_Opp/opponent_digivolves_per_game" in keys
    assert "pilot/archetype/Gauntlet_Opp/opponent_dna_digivolves_per_game" in keys


def test_per_archetype_tb_scalars_emitted(monkeypatch):
    """Spot-check that the four new per-archetype TB scalar families fire
    with the expected averaged values."""
    env = _FakeEvalEnv(
        episodes=[
            ([1.0], 1, {"opponent_archetype": "Opp A",
                        "deck1_archetype": "Agent X"}),
            ([1.0], 2, {"opponent_archetype": "Opp A",
                        "deck1_archetype": "Agent X"}),
        ],
        per_episode_state=[
            {"p1_digivolutions": 4, "p1_dna_digivolutions": 1,
             "p2_digivolutions": 6, "p2_dna_digivolutions": 2},
            {"p1_digivolutions": 6, "p1_dna_digivolutions": 1,
             "p2_digivolutions": 2, "p2_dna_digivolutions": 0},
        ],
    )

    cb = _drive(env, monkeypatch, n_eval_episodes=2)
    records = cb.model.logger.records

    # Agent-archetype side: digi=(4+6)/2=5.0, dna=(1+1)/2=1.0
    assert records["pilot/agent_archetype/Agent_X/digivolves_per_game"] == 5.0
    assert records["pilot/agent_archetype/Agent_X/dna_digivolves_per_game"] == 1.0
    # Opp-archetype side: digi=(6+2)/2=4.0, dna=(2+0)/2=1.0
    assert records["pilot/archetype/Opp_A/opponent_digivolves_per_game"] == 4.0
    assert records["pilot/archetype/Opp_A/opponent_dna_digivolves_per_game"] == 1.0

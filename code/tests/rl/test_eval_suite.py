"""Tests for fixed-seed held-out evaluation."""

from __future__ import annotations

from pathlib import Path

import numpy as np

from digimon_gym.agents.eval_suite import EvalCellResult, HeldOutEvalSuite


REPO_ROOT = Path(__file__).resolve().parents[3]
CONFIG_PATH = REPO_ROOT / "configs" / "training" / "eval_suite.yaml"


def test_loads_config():
    suite = HeldOutEvalSuite.from_yaml(CONFIG_PATH)
    assert suite.version == 1
    assert suite.opponent_policy == "greedy"
    assert len(suite.matchups) >= 1
    assert all(len(matchup.seeds) > 0 for matchup in suite.matchups)


def test_run_is_deterministic():
    suite = HeldOutEvalSuite.from_yaml(CONFIG_PATH)

    def always_pass(_env):
        return 62

    r1 = suite.run(agent_fn=always_pass, max_games_per_cell=2)
    r2 = suite.run(agent_fn=always_pass, max_games_per_cell=2)
    assert r1 == r2


def test_result_includes_per_cell_winrate():
    suite = HeldOutEvalSuite.from_yaml(CONFIG_PATH)

    def always_pass(_env):
        return 62

    result = suite.run(agent_fn=always_pass, max_games_per_cell=2)
    assert "mirror_st1" in result.cell_results
    cell = result.cell_results["mirror_st1"]
    assert isinstance(cell, EvalCellResult)
    assert 0.0 <= cell.win_rate <= 1.0
    assert cell.games_played == 2


def test_eval_suite_pluggable_via_callback():
    from digimon_gym.agents.pilot_training import WinRateCallback

    suite = HeldOutEvalSuite.from_yaml(CONFIG_PATH)
    cb_with = WinRateCallback(
        eval_env_fn=lambda: None,
        eval_freq=1_000_000,
        eval_suite=suite,
    )
    cb_without = WinRateCallback(
        eval_env_fn=lambda: None,
        eval_freq=1_000_000,
        eval_suite=None,
    )
    assert cb_with.eval_suite is suite
    assert cb_without.eval_suite is None


class _FakeLogger:
    def __init__(self):
        self.records = {}

    def record(self, key, value):
        self.records[key] = value


class _FakeModel:
    def __init__(self):
        self.logger = _FakeLogger()

    def predict(self, obs, **kwargs):
        return 0, kwargs.get("state")


class _FakeEvalEnv:
    def __init__(self, episodes):
        self.episodes = episodes
        self.episode_index = -1
        self.step_index = 0
        self.winner_id = None

    def reset(self):
        self.episode_index += 1
        self.step_index = 0
        self.winner_id = None
        return np.zeros(1, dtype=np.float32), {"opponent_archetype": "Fake"}

    def action_mask(self):
        return np.ones(4, dtype=np.int8)

    def step(self, _action):
        rewards, winner = self.episodes[self.episode_index]
        reward = rewards[self.step_index]
        self.step_index += 1
        terminated = self.step_index == len(rewards)
        if terminated:
            self.winner_id = winner
        return np.zeros(1, dtype=np.float32), reward, terminated, False, {}

    def close(self):
        pass


def test_win_rate_callback_logs_terminal_and_dense_eval_reward(monkeypatch):
    from digimon_gym.agents import pilot_training
    from digimon_gym.agents.pilot_training import WinRateCallback

    env = _FakeEvalEnv([
        ([0.5, 1.0], 1),
        ([0.25, -1.0], 2),
    ])
    monkeypatch.setattr(pilot_training, "_unwrap_to_digimon_env", lambda wrapped: wrapped)

    cb = WinRateCallback(
        eval_env_fn=lambda: env,
        eval_freq=1,
        n_eval_episodes=2,
        verbose=0,
    )
    cb.model = _FakeModel()

    cb._run_evaluation()

    records = cb.model.logger.records
    assert records["pilot/win_rate"] == 0.5
    assert records["pilot/mean_eval_reward"] == 0.375
    assert records["pilot/mean_eval_terminal_score"] == 0.0
    assert records["pilot/mean_eval_dense_reward"] == 0.375


class _FakeEvalEnvWithRlState(_FakeEvalEnv):
    """`_FakeEvalEnv` extended with a per-episode `_rl_state` snapshot —
    matches the surface the digivolve-counter telemetry reads after each
    eval game completes.
    """

    def __init__(self, episodes, per_episode_state):
        super().__init__(episodes)
        self._per_episode_state = per_episode_state

    def _rl_state(self):
        # Episode index is post-incremented in reset() and read here AFTER
        # the episode's step loop, so episode_index points at the just-
        # finished episode.
        return self._per_episode_state[self.episode_index]


def test_win_rate_callback_logs_digivolve_telemetry(monkeypatch):
    """Per-episode digivolve counters surface as mean_eval_*_per_game
    scalars, averaged over the eval episodes."""
    from digimon_gym.agents import pilot_training
    from digimon_gym.agents.pilot_training import WinRateCallback

    env = _FakeEvalEnvWithRlState(
        episodes=[
            ([1.0], 1),
            ([1.0], 2),
        ],
        per_episode_state=[
            {"p1_digivolutions": 3, "p1_dna_digivolutions": 1},
            {"p1_digivolutions": 5, "p1_dna_digivolutions": 0},
        ],
    )
    monkeypatch.setattr(pilot_training, "_unwrap_to_digimon_env", lambda wrapped: wrapped)

    cb = WinRateCallback(
        eval_env_fn=lambda: env,
        eval_freq=1,
        n_eval_episodes=2,
        verbose=0,
    )
    cb.model = _FakeModel()

    cb._run_evaluation()

    records = cb.model.logger.records
    # Mean over 2 episodes: (3 + 5) / 2 = 4.0, (1 + 0) / 2 = 0.5.
    assert records["pilot/mean_eval_digivolves_per_game"] == 4.0
    assert records["pilot/mean_eval_dna_digivolves_per_game"] == 0.5


def test_win_rate_callback_handles_missing_rl_state_gracefully(monkeypatch):
    """If the eval env doesn't expose `_rl_state` (e.g. a non-Rust backend),
    the telemetry must not crash — it falls back to zero counts."""
    from digimon_gym.agents import pilot_training
    from digimon_gym.agents.pilot_training import WinRateCallback

    env = _FakeEvalEnv([([1.0], 1)])  # no _rl_state on this fake
    monkeypatch.setattr(pilot_training, "_unwrap_to_digimon_env", lambda wrapped: wrapped)

    cb = WinRateCallback(
        eval_env_fn=lambda: env,
        eval_freq=1,
        n_eval_episodes=1,
        verbose=0,
    )
    cb.model = _FakeModel()

    cb._run_evaluation()

    records = cb.model.logger.records
    assert records["pilot/mean_eval_digivolves_per_game"] == 0.0
    assert records["pilot/mean_eval_dna_digivolves_per_game"] == 0.0


def test_win_rate_callback_exposes_last_eval_suite_results(monkeypatch):
    from digimon_gym.agents import pilot_training
    from digimon_gym.agents.eval_suite import EvalCellResult, EvalSuiteResult
    from digimon_gym.agents.pilot_training import WinRateCallback

    class FakeSuite:
        def run(self, agent_fn):
            return EvalSuiteResult(
                cell_results={
                    "mirror": EvalCellResult(
                        matchup="mirror",
                        games_played=3,
                        wins=2,
                        losses=1,
                        draws=0,
                    )
                }
            )

    env = _FakeEvalEnv([([1.0], 1)])
    monkeypatch.setattr(pilot_training, "_unwrap_to_digimon_env", lambda wrapped: wrapped)

    cb = WinRateCallback(
        eval_env_fn=lambda: env,
        eval_freq=1,
        n_eval_episodes=1,
        eval_suite=FakeSuite(),
        eval_suite_path="configs/training/eval_suite.yaml",
        verbose=0,
    )
    cb.model = _FakeModel()

    cb._run_evaluation()

    assert cb.get_eval_suite_results() == {
        "overall_win_rate": 2 / 3,
        "suite_path": "configs/training/eval_suite.yaml",
        "cells": {
            "mirror": {
                "games_played": 3,
                "wins": 2,
                "losses": 1,
                "draws": 0,
                "win_rate": 2 / 3,
            }
        },
    }

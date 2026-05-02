"""Tests for fixed-seed held-out evaluation."""

from __future__ import annotations

from pathlib import Path

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

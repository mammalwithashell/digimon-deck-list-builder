"""Tests for the self-play retirement + opponent-seat integrity guard.

`harden-training-pipeline` D1: opponent="self-play" is retired because
DigimonEnv observations are Player-1-perspective only; the old mode skipped
OpponentWrapper and let the learner pick Player 2's actions against
wrong-perspective input. Every accepted opponent mode must drive Player 2
through an explicit opponent_fn.
"""

from __future__ import annotations

import numpy as np
import pytest

from digimon_gym.agents import pilot_training
from digimon_gym.agents.pilot_training import OpponentWrapper
from digimon_gym.agents.training_config import (
    SELF_PLAY_RETIRED_MSG,
    TrainingConfig,
    VALID_OPPONENTS,
)


def _find_opponent_wrapper(env):
    """Walk the wrapper chain looking for OpponentWrapper."""
    current = env
    while current is not None:
        if isinstance(current, OpponentWrapper):
            return current
        current = getattr(current, "env", None)
    return None


class TestSelfPlayRetired:
    def test_make_env_rejects_self_play(self):
        with pytest.raises(ValueError) as exc:
            pilot_training.make_env(opponent="self-play")
        msg = str(exc.value)
        assert "retired" in msg
        assert "Player 1's perspective" in msg
        assert "opponent='pool'" in msg

    def test_training_config_rejects_self_play(self):
        with pytest.raises(ValueError) as exc:
            TrainingConfig(opponent="self-play")
        assert str(exc.value) == SELF_PLAY_RETIRED_MSG

    def test_self_play_not_in_valid_opponents(self):
        assert "self-play" not in VALID_OPPONENTS

    def test_cli_self_play_flag_fails_with_guidance(self, monkeypatch, tmp_path):
        config_path = tmp_path / "training.yaml"
        config_path.write_text("timesteps: 1\neval_episodes: 1\n")
        monkeypatch.setattr(
            "sys.argv",
            ["pilot_training", "--config", str(config_path), "--self-play"],
        )
        with pytest.raises(ValueError) as exc:
            pilot_training.main()
        assert "retired" in str(exc.value)
        assert "pool" in str(exc.value)

    def test_job_config_style_train_call_fails_at_startup(self):
        # tools/run_training_job.py calls train(opponent=...) with kwargs;
        # the TrainingConfig built inside train() must raise before any
        # env or model construction.
        with pytest.raises(ValueError) as exc:
            pilot_training.train(total_timesteps=1, opponent="self-play")
        assert "retired" in str(exc.value)


class TestOpponentSeatIntegrity:
    @pytest.mark.parametrize("opponent", ["greedy", "random"])
    def test_make_env_chain_contains_opponent_wrapper(self, opponent):
        env = pilot_training.make_env(opponent=opponent, match_format="single")
        wrapper = _find_opponent_wrapper(env)
        assert wrapper is not None, f"no OpponentWrapper for opponent={opponent!r}"
        assert wrapper.opponent_fn is not None

    def test_unknown_opponent_lists_valid_modes_without_self_play(self):
        with pytest.raises(ValueError) as exc:
            pilot_training.make_env(opponent="bogus")
        msg = str(exc.value)
        assert "bogus" in msg
        assert "self-play" not in msg

    def test_full_episode_auto_plays_player_two(self, monkeypatch):
        """Regression guard: an episode driven only by agent-side actions
        reaches terminal, with the opponent seat taking real actions."""
        opponent_calls = {"n": 0}
        real_greedy = pilot_training.greedy_policy

        def counting_greedy(env):
            opponent_calls["n"] += 1
            return real_greedy(env)

        monkeypatch.setattr(pilot_training, "greedy_policy", counting_greedy)

        env = pilot_training.make_env(opponent="greedy", match_format="single")
        obs, info = env.reset(seed=7)
        rng = np.random.default_rng(7)
        terminated = truncated = False
        for _ in range(5000):
            mask = np.asarray(info["action_mask"], dtype=bool)
            valid = np.flatnonzero(mask)
            action = int(rng.choice(valid)) if len(valid) else 0
            obs, _reward, terminated, truncated, info = env.step(action)
            if terminated or truncated:
                break
        assert terminated or truncated, "episode did not reach terminal"
        assert opponent_calls["n"] > 0, "opponent seat never acted"

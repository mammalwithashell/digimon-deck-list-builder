"""OpponentWrapper observation-build skip (training-speed optimization).

During the opponent's auto-played turn, `DigimonEnv.step` skips the ~8850-float
observation tensor build (every intermediate obs is discarded); the wrapper
builds the real observation exactly once when control returns to the agent.
These tests pin the correctness contract: the agent always receives the REAL
post-turn observation (never the cheap all-zero placeholder), and the no-op
tripwire still covers opponent steps.
"""
import numpy as np
import pytest

from digimon_gym.digimon_gym import DigimonEnv, greedy_policy, NoOpLoopError
from digimon_gym.agents.pilot_training import OpponentWrapper


def _legal_first(mask):
    legal = np.nonzero(np.asarray(mask))[0]
    return int(legal[0]) if len(legal) else None


def test_opponent_wrapper_returns_real_obs_not_placeholder():
    """Every obs the agent receives (reset + each step) must equal the engine's
    current board tensor — i.e. the real observation, not the skip placeholder."""
    env = OpponentWrapper(DigimonEnv(), opponent_fn=greedy_policy)
    obs, info = env.reset(seed=0)
    inner = env._unwrapped_env

    # After reset (which may auto-advance an opponent-first turn), the obs is real.
    assert np.array_equal(obs, inner.runner.get_board_tensor(1)), \
        "reset obs must be the real board tensor"
    assert not getattr(inner, "_skip_obs_build", False), \
        "_skip_obs_build must be cleared after reset"

    for _ in range(60):
        if inner.is_game_over:
            break
        action = _legal_first(info["action_mask"])
        if action is None:
            break
        obs, reward, terminated, truncated, info = env.step(action)
        # The flag must always be cleared after a step returns.
        assert not getattr(inner, "_skip_obs_build", False)
        if terminated or truncated:
            break
        # Mid-game, the returned obs must match the live engine state exactly
        # (proves the final obs is rebuilt, not the all-zero placeholder).
        assert np.array_equal(obs, inner.runner.get_board_tensor(1)), \
            "step obs must be the real post-opponent-turn board tensor"


def test_stuck_opponent_turn_is_bounded_by_action_cap():
    """A stuck opponent (engine refuses every action, never yields the turn back
    to the agent) must raise NoOpLoopError via OpponentWrapper's per-turn action
    cap — not spin forever. The agent-side obs tripwire can't catch it because
    control never returns to the agent."""
    env = OpponentWrapper(DigimonEnv(), opponent_fn=greedy_policy)
    obs, info = env.reset(seed=0)
    inner = env._unwrapped_env
    real = inner.runner

    class _StuckOpponentRunner:
        """Perpetually the opponent's turn; step() never changes state.

        `DigimonEnv.current_player_id` / `is_game_over` read from
        `get_rl_state()`, so override that to pin it to the opponent's turn.
        """
        def step(self, _action):
            return None

        def get_rl_state(self):
            s = dict(real.get_rl_state())
            s["current_player_id"] = 2  # never yields back to the agent
            s["game_over"] = False
            return s

        def __getattr__(self, name):
            return getattr(real, name)

    inner.runner = _StuckOpponentRunner()
    with pytest.raises(NoOpLoopError):
        env._play_opponent(obs, info)
    # The skip flag must be cleared even on the raising path (finally).
    assert not getattr(inner, "_skip_obs_build", False)

"""Verify that LSTM-equivalent opponent state carries across games
within a BO3 match (per the bo3-match-training spec).

The spec requires the recurrent policy's hidden state to persist across
games inside one match so the agent can build inter-game read of the
matchup (e.g., "I learned in game 1 that this matchup is unwinnable
so I'll concede game 2"). This is realised by:

- `OpponentWrapper.reset()` calls `opponent_fn.reset_state()` (full
  match reset).
- `OpponentWrapper.reset_inner_only(...)` does NOT call
  `opponent_fn.reset_state()` (used by MatchEnv between games).

These tests don't construct a real LSTM — they use a synthetic
opponent policy that exposes a `reset_state()` counter, so we can
pin the contract: reset counter increments only at match boundaries.
"""

from __future__ import annotations

import pytest

pytest.importorskip("digimon_engine")

from digimon_gym.agents.match_env import MatchEnv  # noqa: E402
from digimon_gym.agents.pilot_training import OpponentWrapper  # noqa: E402
from digimon_gym.digimon_gym import DigimonEnv  # noqa: E402


class StatefulOpponent:
    """A synthetic opponent policy with `reset_state()` instrumentation.

    Mirrors how SB3's recurrent opponent_fn would expose `reset_state`
    (called between episodes). The wrapped greedy policy provides
    actual action selection.
    """

    def __init__(self) -> None:
        self.reset_count = 0
        # Track every step on the side we're consulted on, so a test can
        # observe that calls keep happening within a match (i.e., we
        # were not reset).
        self.consult_count = 0

    def __call__(self, env: DigimonEnv) -> int:
        from digimon_gym.agents.pilot_training import greedy_policy
        self.consult_count += 1
        return greedy_policy(env)

    def reset_state(self) -> None:
        self.reset_count += 1


def _build_env(opp: StatefulOpponent, seed: int = 7) -> MatchEnv:
    return MatchEnv(OpponentWrapper(DigimonEnv(), opponent_fn=opp), seed=seed)


def test_reset_state_called_exactly_once_per_match_reset() -> None:
    opp = StatefulOpponent()
    env = _build_env(opp)
    assert opp.reset_count == 0

    env.reset()
    assert opp.reset_count == 1, "match reset must call opponent reset_state once"

    env.reset()
    assert opp.reset_count == 2, "second match reset must call reset_state again"


def test_reset_state_not_called_between_games_within_match() -> None:
    """The headline LSTM-carry requirement: `reset_state` SHALL NOT fire
    at game boundaries inside a match. Drive a partial match where the
    agent concedes G1, then begins G2 via the play-order pick — the
    counter must remain at 1 throughout."""
    opp = StatefulOpponent()
    env = _build_env(opp, seed=21)
    env.reset()
    assert opp.reset_count == 1

    # Concede G1. Engine installs SelectPlayOrder. Match not yet decided.
    env.step(93)
    assert opp.reset_count == 1, "G1 end must NOT trigger opp reset"
    assert env._pending_play_order_pick, "should be awaiting play-order pick"

    # Pick play-order. MatchEnv transitions to G2 via reset_inner_only,
    # which preserves opponent state.
    env.step(94)
    assert opp.reset_count == 1, (
        "play-order resolve → next-game transition must NOT trigger opp reset"
    )
    assert env.current_game_index == 1, "G2 should now be in progress"


def test_reset_state_called_only_on_match_boundary_after_full_match() -> None:
    """Run a full BO3 match (agent concedes everything → 0-2). Assert
    reset_state was called once at match start, zero times between
    games, and again only on the next `env.reset()`."""
    opp = StatefulOpponent()
    env = _build_env(opp, seed=33)
    env.reset()
    assert opp.reset_count == 1

    terminated = False
    safety_max = 20
    steps_taken = 0
    while not terminated and steps_taken < safety_max:
        action = 94 if env._pending_play_order_pick else 93
        _, _, terminated, _, _ = env.step(action)
        steps_taken += 1
        # Counter must NEVER tick mid-match.
        assert opp.reset_count == 1, (
            f"opp reset_state fired mid-match at step {steps_taken}"
        )

    assert terminated
    # Now a fresh match — reset must fire.
    env.reset()
    assert opp.reset_count == 2


def test_opponent_consulted_when_agent_picks_second() -> None:
    """When the agent picks "play second", the opponent plays first
    in the next game, so OpponentWrapper autoplay calls `opp_fn` to
    advance to the agent's turn. Pin that the consult count rises in
    this branch (otherwise the carry-state property would be vacuous —
    opp never got to act so we couldn't observe state preservation)."""
    opp = StatefulOpponent()
    env = _build_env(opp, seed=33)
    env.reset()
    consults_at_match_start = opp.consult_count

    # Agent concedes G1 → agent is the chooser for G2.
    env.step(93)
    # Agent picks "play second" → opponent goes first in G2 → autoplay.
    env.step(95)

    assert opp.consult_count > consults_at_match_start, (
        "opponent should have been consulted during G2 autoplay after the "
        "agent chose to play second"
    )


def test_reset_inner_only_method_exists_on_opponent_wrapper() -> None:
    """The MatchEnv relies on `OpponentWrapper.reset_inner_only` to
    preserve LSTM state across games. Pin the public method's
    existence to catch accidental removal."""
    opp = StatefulOpponent()
    wrapper = OpponentWrapper(DigimonEnv(), opponent_fn=opp)
    assert hasattr(wrapper, "reset_inner_only"), (
        "OpponentWrapper must expose reset_inner_only for MatchEnv"
    )
    assert callable(wrapper.reset_inner_only)

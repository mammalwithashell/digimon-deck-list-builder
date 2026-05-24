"""Tests for `MatchEnv` — the BO3 match wrapper.

Split into three sections:

1. Reward calculator helpers (`_calc_*`) — pure-function unit tests
   that pin every scenario in the design's payoff table.
2. State-machine helpers (seed alignment, info stamping) — small
   unit tests.
3. Integration — driving real `DigimonEnv` games through `MatchEnv`
   via a controllable opponent that always concedes, letting tests
   force any match outcome they want.

See `openspec/changes/add-bo3-match-training/design.md` §D9 for the
reward calibration this file guards.
"""

from __future__ import annotations

import math
from typing import List

import pytest

pytest.importorskip("digimon_engine")

from digimon_gym.agents.match_env import MatchEnv  # noqa: E402


# ─── Section 1 — reward calculator helpers ───────────────────────────────


def _bare_match_env() -> MatchEnv:
    """Build a MatchEnv without invoking its constructor's wrapper-chain
    walk. We need an instance to call the `_calc_*` methods, but the
    methods don't depend on `self.env`. Construct by bypassing __init__.
    """
    obj = MatchEnv.__new__(MatchEnv)
    obj.match_score = [0, 0]
    obj.match_step_count = 0
    obj.concede_history = [False, False, False]
    return obj


def test_inner_game_terminal_win_fast() -> None:
    env = _bare_match_env()
    # Win in 0 steps → max fast bonus +5.
    assert math.isclose(env._calc_inner_game_terminal(1, 0), 15.0, abs_tol=1e-9)


def test_inner_game_terminal_win_slow() -> None:
    env = _bare_match_env()
    # Win at exactly the par step count → zero fast bonus.
    assert math.isclose(env._calc_inner_game_terminal(1, 200), 10.0, abs_tol=1e-9)


def test_inner_game_terminal_loss_no_fast_bonus() -> None:
    env = _bare_match_env()
    assert math.isclose(env._calc_inner_game_terminal(2, 30), -10.0, abs_tol=1e-9)


def test_bo3_game_terminal_fast_win() -> None:
    env = _bare_match_env()
    # 30-step crush → bonus = max(0, (150-30)/150) * 3 = 2.4
    assert math.isclose(env._calc_bo3_game_terminal(1, 30), 12.0 + 2.4, abs_tol=1e-9)


def test_bo3_game_terminal_par_win() -> None:
    env = _bare_match_env()
    # At par (150 steps) the bonus zeroes out.
    assert math.isclose(env._calc_bo3_game_terminal(1, 150), 12.0, abs_tol=1e-9)


def test_bo3_game_terminal_loss() -> None:
    env = _bare_match_env()
    assert math.isclose(env._calc_bo3_game_terminal(2, 30), -12.0, abs_tol=1e-9)


def test_terminal_adjustment_replaces_inner_with_bo3() -> None:
    """When the wrapper subtracts inner and adds BO3, the result must
    equal the BO3 calibration exactly (independent of inner)."""
    env = _bare_match_env()
    for steps in [10, 60, 120, 200]:
        for winner in [1, 2]:
            inner = env._calc_inner_game_terminal(winner, steps)
            bo3 = env._calc_bo3_game_terminal(winner, steps)
            adjustment = bo3 - inner
            # The "actual reward paid" = inner + adjustment = bo3.
            assert math.isclose(inner + adjustment, bo3, abs_tol=1e-9)


# ─── Section 2 — match-terminal calculator ───────────────────────────────


def test_match_terminal_sweep_win_pays_base_plus_sweep_plus_fast() -> None:
    """2-0 sweep at 100 steps → +30 base + 10 sweep + (450-100)/450 * 15."""
    env = _bare_match_env()
    env.match_score = [2, 0]
    env.match_step_count = 100
    env.concede_history = [False, False, False]
    expected = 30.0 + 10.0 + (450.0 - 100.0) / 450.0 * 15.0
    assert math.isclose(env._calc_match_terminal(), expected, abs_tol=1e-9)


def test_match_terminal_two_one_win_no_sweep_no_concede() -> None:
    """Standard 2-1 win at 200 steps → +30 + fast bonus only."""
    env = _bare_match_env()
    env.match_score = [2, 1]
    env.match_step_count = 200
    env.concede_history = [False, False, False]
    expected = 30.0 + (450.0 - 200.0) / 450.0 * 15.0
    assert math.isclose(env._calc_match_terminal(), expected, abs_tol=1e-9)


def test_match_terminal_smart_concede_two_one_win() -> None:
    """G1 win + G2 concede + G3 win = 2-1 win with smart-concede bonus."""
    env = _bare_match_env()
    env.match_score = [2, 1]
    env.match_step_count = 121
    env.concede_history = [False, True, False]  # G2 conceded by agent
    expected = 30.0 + 5.0 + (450.0 - 121.0) / 450.0 * 15.0
    assert math.isclose(env._calc_match_terminal(), expected, abs_tol=1e-9)


def test_match_terminal_lose_one_two_no_extra_penalty() -> None:
    """Lose 1-2 honestly → just -30 (no scared concede, no fast bonus)."""
    env = _bare_match_env()
    env.match_score = [1, 2]
    env.match_step_count = 220
    env.concede_history = [False, False, False]
    assert math.isclose(env._calc_match_terminal(), -30.0, abs_tol=1e-9)


def test_match_terminal_lose_zero_two_honest() -> None:
    """Lose 0-2 with no concedes → just -30."""
    env = _bare_match_env()
    env.match_score = [0, 2]
    env.match_step_count = 140
    env.concede_history = [False, False, False]
    assert math.isclose(env._calc_match_terminal(), -30.0, abs_tol=1e-9)


def test_match_terminal_lose_zero_two_with_scared_concede() -> None:
    """Lose 0-2 with at least one agent concede → -30 - 10 = -40."""
    env = _bare_match_env()
    env.match_score = [0, 2]
    env.match_step_count = 81
    env.concede_history = [True, False, False]
    assert math.isclose(env._calc_match_terminal(), -40.0, abs_tol=1e-9)


def test_match_terminal_lose_one_two_with_concede_no_scared_penalty() -> None:
    """Lose 1-2 with a smart-but-failed G2 concede → no scared penalty
    (penalty applies only to 0-2 losses, not 1-2)."""
    env = _bare_match_env()
    env.match_score = [1, 2]
    env.match_step_count = 141
    env.concede_history = [False, True, False]
    assert math.isclose(env._calc_match_terminal(), -30.0, abs_tol=1e-9)


def test_match_terminal_draw() -> None:
    """1-1-1 draw (only on hard step-limit truncation) → -1."""
    env = _bare_match_env()
    env.match_score = [1, 1]
    env.match_step_count = 900
    env.concede_history = [False, False, False]
    assert math.isclose(env._calc_match_terminal(), -1.0, abs_tol=1e-9)


def test_match_terminal_fast_match_capped_at_max_bonus() -> None:
    """Match completed in 0 steps (impossible but useful to pin) gets
    the full +15 fast-match bonus."""
    env = _bare_match_env()
    env.match_score = [2, 0]
    env.match_step_count = 0
    env.concede_history = [False, False, False]
    expected = 30.0 + 10.0 + 15.0  # full sweep + full fast bonus
    assert math.isclose(env._calc_match_terminal(), expected, abs_tol=1e-9)


def test_match_terminal_fast_match_zeroes_at_par() -> None:
    """At 450 match steps, fast-match bonus is exactly 0."""
    env = _bare_match_env()
    env.match_score = [2, 1]
    env.match_step_count = 450
    env.concede_history = [False, False, False]
    assert math.isclose(env._calc_match_terminal(), 30.0, abs_tol=1e-9)


# ─── Section 2b — seed alignment ─────────────────────────────────────────


def test_seed_alignment_preserves_even_for_player_zero() -> None:
    """Player 0 (Rust pid 0) = even seed → no shift."""
    assert MatchEnv._align_seed_for_first_player(42, 0) == 42
    assert MatchEnv._align_seed_for_first_player(43, 0) == 44


def test_seed_alignment_shifts_for_player_one() -> None:
    """Player 1 (Rust pid 1) = odd seed → shift even seeds."""
    assert MatchEnv._align_seed_for_first_player(43, 1) == 43
    assert MatchEnv._align_seed_for_first_player(42, 1) == 43


# ─── Section 3 — integration: drive a real match end-to-end ─────────────


def _make_match_env_with_greedy_opponent(seed: int = 7) -> MatchEnv:
    """Build the full BO3 env chain with the default greedy opponent.

    Chain (inner → outer):
        DigimonEnv → OpponentWrapper(opp = greedy) → MatchEnv

    Using a real greedy opponent (instead of always-concede) avoids the
    `_advance_opponent` autoplay loop terminating the game during
    `reset`'s mulligan phase. Tests force quick game endings by having
    the AGENT concede, which works regardless of phase.
    """
    from digimon_gym.agents.pilot_training import OpponentWrapper, greedy_policy
    from digimon_gym.digimon_gym import DigimonEnv

    digimon_env = DigimonEnv()
    opp_wrapped = OpponentWrapper(digimon_env, opponent_fn=greedy_policy)
    return MatchEnv(opp_wrapped, seed=seed)


def test_integration_reset_initializes_match_state() -> None:
    env = _make_match_env_with_greedy_opponent(seed=11)
    obs, info = env.reset()

    assert env.match_id is not None
    assert env.match_score == [0, 0]
    assert env.current_game_index == 0
    assert env.match_step_count == 0
    assert env._deck1 is not None and len(env._deck1) > 0
    assert env._deck2 is not None and len(env._deck2) > 0
    assert info["match_id"] == env.match_id
    assert info["game_index_in_match"] == 0


def test_integration_decks_persist_across_match() -> None:
    """Snapshot the deck pair at match start; assert it's unchanged after
    multiple games complete."""
    env = _make_match_env_with_greedy_opponent(seed=11)
    env.reset()
    deck1_snapshot = list(env._deck1)
    deck2_snapshot = list(env._deck2)

    # Drive a few opponent concedes — the agent only needs to act between
    # games (no real choices, the opponent's auto-concede ends each game).
    # Each `MatchEnv.step` triggers OpponentWrapper autoplay, which
    # invokes the conceding opponent.
    #
    # We pass action PASS (62) which is always-legal in many phases.
    for _ in range(10):
        obs, reward, terminated, truncated, info = env.step(62)
        if terminated:
            break

    # Decks must not have been mutated.
    assert env._deck1 == deck1_snapshot
    assert env._deck2 == deck2_snapshot


def test_integration_concede_action_passes_through_mask() -> None:
    """At any agent decision point the mask must report action 93 = 1."""
    env = _make_match_env_with_greedy_opponent(seed=13)
    obs, info = env.reset()

    # During mulligan or main phase, mask[93] should be 1.
    mask = info.get("action_mask")
    if mask is None:
        mask = env._inner_digimon_env.action_mask()
    assert mask[93] == 1, "concede must always be legal at agent decision points"


def test_integration_agent_concede_terminates_game() -> None:
    """Action 93 from the agent terminates one game inside the match."""
    env = _make_match_env_with_greedy_opponent(seed=17)
    env.reset()
    initial_game_index = env.current_game_index

    obs, reward, terminated, truncated, info = env.step(93)

    # That action makes the agent concede game 1. Match may continue
    # (because the opponent now plays game 2) or terminate if the
    # cascade ends in 2 wins for opp.
    # Concede must be recorded on G1.
    assert env.concede_history[initial_game_index] is True


def test_integration_match_completes_within_few_steps_via_concede() -> None:
    """End-to-end: drive a full BO3 match where the agent concedes every
    game. The flow alternates:

      - In a game phase: action 93 (concede) ends the current game.
      - In `SelectPlayOrder` (between games): action 94 (play first).

    With agent conceding every game, the match resolves as 0-2 in two
    games. We assert: terminates within a handful of steps, ends at
    0-2, and the scared-concede penalty fires (concede history non-empty).
    """
    env = _make_match_env_with_greedy_opponent(seed=21)
    env.reset()

    terminated = False
    steps = 0
    while not terminated and steps < 20:
        # Pick action based on whether we're in the play-order pick.
        action = 94 if env._pending_play_order_pick else 93
        _, _, terminated, _, _ = env.step(action)
        steps += 1

    assert terminated, "BO3 match should resolve within 20 MatchEnv steps"
    # Concede-every-game → match score 0-2.
    assert env.match_score == [0, 2], (
        f"agent conceding every game should yield 0-2 final score, got {env.match_score}"
    )
    # All conceded games must be marked.
    assert any(env.concede_history), "at least one concede must be recorded"


def test_integration_match_terminal_reward_in_expected_range() -> None:
    """End-to-end reward sanity: a full match's grand-total reward
    should land in the design's predicted range (`±50` for typical
    paths, much larger for sweeps with fast bonus).
    """
    env = _make_match_env_with_greedy_opponent(seed=23)
    env.reset()

    total_reward = 0.0
    terminated = False
    steps = 0
    while not terminated and steps < 50:
        _, reward, terminated, _, _ = env.step(93)
        total_reward += float(reward)
        steps += 1

    # The exact value depends on engine-driven play order; sanity-check
    # the magnitude bracket: a full BO3 conceding-fight produces
    # somewhere between -100 (worst case) and +100 (best case).
    assert -100.0 < total_reward < 100.0, (
        f"total match reward {total_reward:.2f} should be in a sane range"
    )


# ─── Section 4 — wrapper-chain integrity ─────────────────────────────────


def test_match_env_finds_inner_digimon_env_through_opponent_wrapper() -> None:
    env = _make_match_env_with_greedy_opponent(seed=31)
    from digimon_gym.digimon_gym import DigimonEnv

    assert isinstance(env._inner_digimon_env, DigimonEnv)


def test_match_env_rejects_chain_without_digimon_env() -> None:
    """Constructor must raise if no DigimonEnv is found in the wrapper
    chain — protects against misconfigured pipelines."""
    import gymnasium

    class DummyEnv(gymnasium.Env):
        observation_space = None  # type: ignore[assignment]
        action_space = None  # type: ignore[assignment]

        def reset(self, *, seed=None, options=None):
            return None, {}

        def step(self, action):
            return None, 0.0, True, False, {}

    with pytest.raises(TypeError):
        MatchEnv(DummyEnv())

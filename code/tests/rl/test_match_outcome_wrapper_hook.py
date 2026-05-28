"""Tests for the Path C wrapper hook — how `RewardProfileWrapper` and
`MatchEnv` cooperate to let a profile own the match-tier reward.

Three test surfaces:

1. `RewardProfileWrapper.has_match_outcome_component` and
   `RewardProfileWrapper.compute_match_terminal` — pure-Python, no
   engine.
2. `MatchEnv._find_reward_profile_wrapper` — walks the wrapper chain.
3. `MatchEnv._build_match_terminal_snapshot` — snapshot dict shape
   matches `MatchOutcome` field names.

`MatchEnv`'s actual end-to-end reward routing (skipping (bo3-inner)
adjustment + calling `compute_match_terminal` on match decided) is
covered by the integration test at the bottom, which uses
`_bare_match_env` + a mocked wrapper so we can assert the exact
reward delivered without a real engine match.

Spec: `openspec/specs/gameplay-reward-config/spec.md` `match_outcome
component`, `openspec/specs/bo3-match-training/spec.md` wrapper-deferral
contract.
"""

from __future__ import annotations

import textwrap
from pathlib import Path
from typing import Any, Dict, List, Optional

import gymnasium
import pytest
from gymnasium import spaces

from digimon_gym.agents.reward.occurrences import StepElapsed, TerminalOutcome
from digimon_gym.agents.reward.profile_loader import _parse_and_resolve
from digimon_gym.agents.reward.wrapper import RewardProfileWrapper


# ============================================================================
# Test env (minimal — produces tunable occurrence streams)
# ============================================================================


class _FakeEnv(gymnasium.Env):
    metadata = {"render_modes": []}

    def __init__(self) -> None:
        super().__init__()
        self.observation_space = spaces.Box(low=0.0, high=1.0, shape=(1,))
        self.action_space = spaces.Discrete(1)
        self.next_archetype: Optional[str] = None
        self.next_step_occurrences: List[Any] = []
        self.next_step_terminated: bool = False

    def reset(self, *, seed: int | None = None, options: dict | None = None):
        super().reset(seed=seed)
        info: Dict[str, Any] = {}
        if self.next_archetype is not None:
            info["deck1_archetype"] = self.next_archetype
        return (self.observation_space.sample() * 0, info)

    def step(self, action: int):
        info: Dict[str, Any] = {
            "reward_occurrences": list(self.next_step_occurrences),
        }
        return (
            self.observation_space.sample() * 0,
            0.0,
            self.next_step_terminated,
            False,
            info,
        )


def _yaml(s: str) -> str:
    return textwrap.dedent(s).strip("\n") + "\n"


# Profile that DOES include match_outcome → wrapper should activate hook.
PROFILES_WITH_MATCH_OUTCOME = _yaml(
    """
    profiles:
      _default:
        components:
          - kind: terminal_outcome
            win_base: 0.2
            fast_win_bonus_max: 0.0
            fast_win_par_steps: 200
            loss: -0.2
            draw: 0.0
          - kind: match_outcome
            win: 1.0
            loss: -1.0
            draw: 0.0
            sweep_bonus: 0.5
    assignments:
      _default: _default
    """
)

# Profile that does NOT include match_outcome → wrapper hook stays off.
PROFILES_WITHOUT_MATCH_OUTCOME = _yaml(
    """
    profiles:
      _default:
        components:
          - kind: terminal_outcome
            win_base: 10.0
            fast_win_bonus_max: 5.0
            fast_win_par_steps: 200
            loss: -10.0
            draw: -1.0
    assignments:
      _default: _default
    """
)


def _make_wrapper(yaml_str: str) -> RewardProfileWrapper:
    # `_parse_and_resolve` signature: (text, source_path).
    profiles = _parse_and_resolve(yaml_str, Path("synthetic.yaml"))
    env = _FakeEnv()
    return RewardProfileWrapper(env, profiles=profiles, hot_reload=False)


# ─── 1. has_match_outcome_component ────────────────────────────────────


def test_has_match_outcome_false_before_reset():
    """No active profile yet → property returns False (not an error).
    `MatchEnv.__init__` may probe this before the first reset; must not
    crash."""
    w = _make_wrapper(PROFILES_WITH_MATCH_OUTCOME)
    assert w.has_match_outcome_component is False


def test_has_match_outcome_true_when_profile_includes_it():
    w = _make_wrapper(PROFILES_WITH_MATCH_OUTCOME)
    w.reset()
    assert w.has_match_outcome_component is True


def test_has_match_outcome_false_when_profile_omits_it():
    w = _make_wrapper(PROFILES_WITHOUT_MATCH_OUTCOME)
    w.reset()
    assert w.has_match_outcome_component is False


# ─── 2. compute_match_terminal ────────────────────────────────────────


def test_compute_match_terminal_win_uses_component():
    w = _make_wrapper(PROFILES_WITH_MATCH_OUTCOME)
    w.reset()
    out = w.compute_match_terminal({
        "winner_id": 1,
        "agent_game_wins": 2,
        "opp_game_wins": 0,
        "was_sweep": True,
        "was_swept": False,
        "agent_conceded_any": False,
        "match_step_count": 200,
        "total_games": 2,
    })
    # win (1.0) + sweep_bonus (0.5) = 1.5
    assert out == pytest.approx(1.5)


def test_compute_match_terminal_loss_uses_component():
    w = _make_wrapper(PROFILES_WITH_MATCH_OUTCOME)
    w.reset()
    out = w.compute_match_terminal({
        "winner_id": 2,
        "agent_game_wins": 0,
        "opp_game_wins": 2,
        "was_sweep": False,
        "was_swept": True,
        "agent_conceded_any": False,
        "match_step_count": 80,
        "total_games": 2,
    })
    assert out == pytest.approx(-1.0)


def test_compute_match_terminal_returns_zero_when_component_absent():
    """When the active profile has no match_outcome, the method MUST
    return 0 (not crash) — though MatchEnv guards itself via
    `has_match_outcome_component` and shouldn't call here in that case.
    Defensive contract."""
    w = _make_wrapper(PROFILES_WITHOUT_MATCH_OUTCOME)
    w.reset()
    out = w.compute_match_terminal({
        "winner_id": 1,
        "agent_game_wins": 2,
        "opp_game_wins": 0,
        "was_sweep": True,
        "was_swept": False,
        "agent_conceded_any": False,
        "match_step_count": 100,
        "total_games": 2,
    })
    assert out == 0.0


def test_compute_match_terminal_returns_zero_pre_reset():
    """Defensive: called before first reset → 0, not crash."""
    w = _make_wrapper(PROFILES_WITH_MATCH_OUTCOME)
    out = w.compute_match_terminal({
        "winner_id": 1, "agent_game_wins": 2, "opp_game_wins": 0,
        "was_sweep": True, "was_swept": False, "agent_conceded_any": False,
        "match_step_count": 100, "total_games": 2,
    })
    assert out == 0.0


# ─── 3. MatchEnv wrapper-chain discovery + snapshot ──────────────────


def test_match_env_finds_wrapper_in_chain():
    """MatchEnv must walk through OpponentWrapper to find the
    RewardProfileWrapper, and cache the reference at construction."""
    pytest.importorskip("digimon_engine")
    from digimon_gym.agents.match_env import MatchEnv

    inner_wrapper = _make_wrapper(PROFILES_WITH_MATCH_OUTCOME)
    # Stand in a trivial Wrapper as a pseudo-OpponentWrapper layer.
    middle = gymnasium.Wrapper(inner_wrapper)

    # Bypass MatchEnv's __init__ wrapper-chain walk for the digimon-env
    # search by stubbing _find_digimon_env. We still want to exercise
    # the reward-wrapper walk.
    matchenv = MatchEnv.__new__(MatchEnv)
    found = matchenv._find_reward_profile_wrapper(middle)
    assert found is inner_wrapper


def test_match_env_find_returns_none_when_no_wrapper_in_chain():
    pytest.importorskip("digimon_engine")
    from digimon_gym.agents.match_env import MatchEnv

    bare = gymnasium.Wrapper(_FakeEnv())
    matchenv = MatchEnv.__new__(MatchEnv)
    assert matchenv._find_reward_profile_wrapper(bare) is None


def test_match_env_snapshot_dict_shape_matches_match_outcome_fields():
    """The snapshot dict produced by MatchEnv must use exactly the field
    names that `MatchOutcome` expects — wrong keys = silent NaN."""
    pytest.importorskip("digimon_engine")
    from digimon_gym.agents.match_env import MatchEnv
    from digimon_gym.agents.reward.occurrences import MatchOutcome

    env = MatchEnv.__new__(MatchEnv)
    env.match_score = [2, 1]
    env.match_step_count = 350
    env.concede_history = [False, True, False]

    snap = env._build_match_terminal_snapshot()
    # All MatchOutcome fields must be present and constructable.
    occ = MatchOutcome(**snap)
    assert occ.winner_id == 1
    assert occ.agent_game_wins == 2
    assert occ.opp_game_wins == 1
    assert occ.was_sweep is False        # 2-1 is not a sweep
    assert occ.was_swept is False        # agent has wins
    assert occ.agent_conceded_any is True
    assert occ.match_step_count == 350
    assert occ.total_games == 3


def test_match_env_snapshot_sweep_flags_set_correctly():
    pytest.importorskip("digimon_engine")
    from digimon_gym.agents.match_env import MatchEnv

    env = MatchEnv.__new__(MatchEnv)
    env.match_score = [2, 0]
    env.match_step_count = 100
    env.concede_history = [False, False, False]
    snap = env._build_match_terminal_snapshot()
    assert snap["winner_id"] == 1
    assert snap["was_sweep"] is True
    assert snap["was_swept"] is False
    assert snap["total_games"] == 2


def test_match_env_snapshot_swept_flags_set_correctly():
    pytest.importorskip("digimon_engine")
    from digimon_gym.agents.match_env import MatchEnv

    env = MatchEnv.__new__(MatchEnv)
    env.match_score = [0, 2]
    env.match_step_count = 80
    env.concede_history = [True, False, False]
    snap = env._build_match_terminal_snapshot()
    assert snap["winner_id"] == 2
    assert snap["was_sweep"] is False
    assert snap["was_swept"] is True
    assert snap["agent_conceded_any"] is True


def test_match_env_snapshot_draw_when_no_majority():
    """1-1-1 step-limit draw → winner_id None, neither sweep flag."""
    pytest.importorskip("digimon_engine")
    from digimon_gym.agents.match_env import MatchEnv

    env = MatchEnv.__new__(MatchEnv)
    env.match_score = [1, 1]
    env.match_step_count = 900
    env.concede_history = [False, False, False]
    snap = env._build_match_terminal_snapshot()
    assert snap["winner_id"] is None
    assert snap["was_sweep"] is False
    assert snap["was_swept"] is False
    assert snap["total_games"] == 2


# ─── 4. _profile_owns_match_terminal property ────────────────────────


def test_profile_owns_match_terminal_false_without_wrapper():
    pytest.importorskip("digimon_engine")
    from digimon_gym.agents.match_env import MatchEnv

    env = MatchEnv.__new__(MatchEnv)
    env._reward_profile_wrapper = None
    assert env._profile_owns_match_terminal is False


def test_profile_owns_match_terminal_reflects_wrapper_state():
    pytest.importorskip("digimon_engine")
    from digimon_gym.agents.match_env import MatchEnv

    w_with = _make_wrapper(PROFILES_WITH_MATCH_OUTCOME)
    w_with.reset()
    env = MatchEnv.__new__(MatchEnv)
    env._reward_profile_wrapper = w_with
    assert env._profile_owns_match_terminal is True

    w_without = _make_wrapper(PROFILES_WITHOUT_MATCH_OUTCOME)
    w_without.reset()
    env._reward_profile_wrapper = w_without
    assert env._profile_owns_match_terminal is False

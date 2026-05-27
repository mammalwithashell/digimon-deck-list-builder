"""Pure-Python tests for `RewardProfileWrapper`.

Uses a `_FakeEnv` that produces synthetic `info["reward_occurrences"]`
+ tunable `info["deck1_archetype"]`. No actual `DigimonEnv` /
PyO3 / engine required.
"""

from __future__ import annotations

import textwrap
import time
import warnings
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import gymnasium
import pytest
from gymnasium import spaces

from digimon_gym.agents.reward.occurrences import (
    PlayedCard,
    SecurityRemoved,
    StepElapsed,
    TerminalOutcome,
)
from digimon_gym.agents.reward.profile_loader import (
    ProfileLoader,
    _parse_and_resolve,
)
from digimon_gym.agents.reward.wrapper import RewardProfileWrapper


def _yaml(s: str) -> str:
    return textwrap.dedent(s).strip("\n") + "\n"


SYNTH = Path("synthetic.yaml")


# ============================================================================
# Test env (minimal — produces tunable occurrence streams)
# ============================================================================


class _FakeEnv(gymnasium.Env):
    """Minimal env that records the next reset's `deck1_archetype` +
    each step's pre-supplied `occurrences` list. Returns dummy
    observation + raw reward (which the wrapper replaces).
    """

    metadata = {"render_modes": []}

    def __init__(self) -> None:
        super().__init__()
        self.observation_space = spaces.Box(low=0.0, high=1.0, shape=(1,))
        self.action_space = spaces.Discrete(1)
        # Test hooks:
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
        # Wrapped env's reward is discarded by the wrapper.
        return self.observation_space.sample() * 0, 0.0, self.next_step_terminated, False, info


# ============================================================================
# Test profiles
# ============================================================================


_DEFAULT_PROFILES_YAML = _yaml(
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
          - kind: step_penalty
            weight: -0.001
          - kind: security_remove
            weight: 1.5
          - kind: security_lost
            weight: -0.5
      aggressive:
        inherits: _default
        components:
          - kind: security_remove
            weight: 3.0
          - kind: step_penalty
            weight: -0.003
    assignments:
      _default: _default
      "Rocks": aggressive
    """
)


# ============================================================================
# reset(): profile resolution from info
# ============================================================================


def test_reset_resolves_profile_from_deck1_archetype():
    profs = _parse_and_resolve(_DEFAULT_PROFILES_YAML, SYNTH)
    env = _FakeEnv()
    env.next_archetype = "Rocks"
    wrapper = RewardProfileWrapper(env, profiles=profs)
    _obs, info = wrapper.reset()
    assert info["reward_profile"] == "aggressive"
    assert wrapper.current_profile_name == "aggressive"


def test_reset_falls_back_to_default_when_archetype_absent():
    profs = _parse_and_resolve(_DEFAULT_PROFILES_YAML, SYNTH)
    env = _FakeEnv()
    # next_archetype left as None → no deck1_archetype in info
    wrapper = RewardProfileWrapper(env, profiles=profs)
    _obs, info = wrapper.reset()
    assert info["reward_profile"] == "_default"


def test_reset_falls_back_to_default_when_archetype_unmapped():
    profs = _parse_and_resolve(_DEFAULT_PROFILES_YAML, SYNTH)
    env = _FakeEnv()
    env.next_archetype = "UnmappedArchetype"
    wrapper = RewardProfileWrapper(env, profiles=profs)
    _obs, info = wrapper.reset()
    assert info["reward_profile"] == "_default"


def test_override_supersedes_archetype():
    profs = _parse_and_resolve(_DEFAULT_PROFILES_YAML, SYNTH)
    env = _FakeEnv()
    env.next_archetype = "Rocks"   # would map to "aggressive"
    wrapper = RewardProfileWrapper(env, profiles=profs, override="_default")
    _obs, info = wrapper.reset()
    assert info["reward_profile"] == "_default"


def test_override_pointing_at_nonexistent_profile_raises_at_init():
    profs = _parse_and_resolve(_DEFAULT_PROFILES_YAML, SYNTH)
    env = _FakeEnv()
    with pytest.raises(ValueError, match=r"not name a defined profile|not name a profile|does not name"):
        RewardProfileWrapper(env, profiles=profs, override="does_not_exist")


def test_reset_writes_profile_hash_into_info():
    profs = _parse_and_resolve(_DEFAULT_PROFILES_YAML, SYNTH)
    env = _FakeEnv()
    wrapper = RewardProfileWrapper(env, profiles=profs)
    _obs, info = wrapper.reset()
    assert info["reward_profile_hash"] == profs.content_hash
    assert info["reward_profile_hash"].startswith("sha256:")


# ============================================================================
# step(): reward + breakdown
# ============================================================================


def test_step_builds_breakdown_and_returns_summed_scalar():
    profs = _parse_and_resolve(_DEFAULT_PROFILES_YAML, SYNTH)
    env = _FakeEnv()
    wrapper = RewardProfileWrapper(env, profiles=profs)
    wrapper.reset()
    # Single step: only step_penalty fires (StepElapsed in occurrences).
    env.next_step_occurrences = [StepElapsed()]
    _obs, reward, _term, _trunc, info = wrapper.step(0)

    breakdown = info["reward_breakdown"]
    assert breakdown.get("step_penalty") == pytest.approx(-0.001)
    # All four components present, even when contribution is zero.
    assert set(breakdown) >= {"terminal_outcome", "step_penalty", "security_remove", "security_lost"}
    # Scalar = sum of breakdown.
    assert reward == pytest.approx(sum(breakdown.values()))


def test_step_reward_includes_security_remove_when_occurrence_present():
    profs = _parse_and_resolve(_DEFAULT_PROFILES_YAML, SYNTH)
    env = _FakeEnv()
    wrapper = RewardProfileWrapper(env, profiles=profs)
    wrapper.reset()
    env.next_step_occurrences = [StepElapsed(), SecurityRemoved(count=2)]
    _obs, reward, _term, _trunc, info = wrapper.step(0)
    # 1.5 × 2 = 3.0 from security_remove; -0.001 from step_penalty.
    assert info["reward_breakdown"]["security_remove"] == pytest.approx(3.0)
    assert reward == pytest.approx(3.0 - 0.001)


def test_step_terminal_outcome_fires_on_terminal_occurrence():
    profs = _parse_and_resolve(_DEFAULT_PROFILES_YAML, SYNTH)
    env = _FakeEnv()
    wrapper = RewardProfileWrapper(env, profiles=profs)
    wrapper.reset()
    # Agent wins at step 150 → 10 + (200-150)/200 × 5 = 11.25
    env.next_step_occurrences = [
        StepElapsed(),
        TerminalOutcome(winner_id=1, step_count=150, reason="security_attack"),
    ]
    env.next_step_terminated = True
    _obs, reward, term, _trunc, info = wrapper.step(0)
    assert info["reward_breakdown"]["terminal_outcome"] == pytest.approx(11.25)
    assert term is True


def test_step_reward_profile_stays_constant_within_episode():
    # Even if the underlying env's info contains a different archetype on
    # later steps, the profile resolved at reset() persists.
    profs = _parse_and_resolve(_DEFAULT_PROFILES_YAML, SYNTH)
    env = _FakeEnv()
    env.next_archetype = "Rocks"
    wrapper = RewardProfileWrapper(env, profiles=profs)
    _obs, info = wrapper.reset()
    assert info["reward_profile"] == "aggressive"

    env.next_step_occurrences = [StepElapsed()]
    for _ in range(5):
        _o, _r, _t, _u, info = wrapper.step(0)
        assert info["reward_profile"] == "aggressive"


def test_step_invokes_per_profile_budget_and_records_clamp():
    yaml_text = _yaml(
        """
        profiles:
          p:
            budget:
              per_episode_cap: 4.0
            components:
              - kind: security_remove
                weight: 3.0
        assignments:
          _default: p
        """
    )
    profs = _parse_and_resolve(yaml_text, SYNTH)
    env = _FakeEnv()
    wrapper = RewardProfileWrapper(env, profiles=profs)
    wrapper.reset()

    # Step 1: SecurityRemoved(count=1) → 3.0 (under cap=4).
    env.next_step_occurrences = [StepElapsed(), SecurityRemoved(count=1)]
    _o, r1, _t, _u, info1 = wrapper.step(0)
    assert info1["reward_breakdown"]["security_remove"] == pytest.approx(3.0)
    assert "reward_breakdown_clamped" not in info1

    # Step 2: SecurityRemoved(count=1) → would emit 3.0 but cap leaves
    # only 1.0 → emit 1.0, clamp 2.0.
    env.next_step_occurrences = [StepElapsed(), SecurityRemoved(count=1)]
    _o, r2, _t, _u, info2 = wrapper.step(0)
    assert info2["reward_breakdown"]["security_remove"] == pytest.approx(1.0)
    assert info2["reward_breakdown_clamped"]["security_remove"] == pytest.approx(2.0)


def test_step_before_reset_raises():
    profs = _parse_and_resolve(_DEFAULT_PROFILES_YAML, SYNTH)
    env = _FakeEnv()
    wrapper = RewardProfileWrapper(env, profiles=profs)
    with pytest.raises(AssertionError, match=r"before reset"):
        wrapper.step(0)


# ============================================================================
# Episode-state lifetime + budget memory across steps
# ============================================================================


def test_episode_state_resets_on_reset_so_budgets_refresh():
    yaml_text = _yaml(
        """
        profiles:
          p:
            components:
              - kind: security_remove
                weight: 5.0
                budget:
                  max_fires_per_episode: 1
        assignments:
          _default: p
        """
    )
    profs = _parse_and_resolve(yaml_text, SYNTH)
    env = _FakeEnv()
    wrapper = RewardProfileWrapper(env, profiles=profs)

    wrapper.reset()
    env.next_step_occurrences = [StepElapsed(), SecurityRemoved(count=1)]
    _o, r1, _t, _u, info1 = wrapper.step(0)
    # First step: fires.
    assert info1["reward_breakdown"]["security_remove"] == 5.0

    # Second step in SAME episode: max_fires=1, so 0.
    _o, r2, _t, _u, info2 = wrapper.step(0)
    assert info2["reward_breakdown"]["security_remove"] == 0.0

    # New episode: budget refreshes.
    wrapper.reset()
    _o, r3, _t, _u, info3 = wrapper.step(0)
    assert info3["reward_breakdown"]["security_remove"] == 5.0


# ============================================================================
# Hot-reload integration
# ============================================================================


def test_reset_triggers_hot_reload_when_enabled(tmp_path: Path):
    path = tmp_path / "profiles.yaml"
    path.write_text(_DEFAULT_PROFILES_YAML, encoding="utf-8")
    loader = ProfileLoader(path)
    pre_hash = loader.profiles.content_hash

    env = _FakeEnv()
    wrapper = RewardProfileWrapper(env, loader=loader, hot_reload=True)
    wrapper.reset()
    assert wrapper.current_profile_hash == pre_hash

    # Edit the file. Next reset() picks up the change.
    time.sleep(0.05)
    modified = _DEFAULT_PROFILES_YAML.replace("-0.001", "-0.005")
    path.write_text(modified, encoding="utf-8")

    wrapper.reset()
    assert wrapper.current_profile_hash != pre_hash


def test_reset_does_not_reload_when_hot_reload_false(tmp_path: Path):
    path = tmp_path / "profiles.yaml"
    path.write_text(_DEFAULT_PROFILES_YAML, encoding="utf-8")
    loader = ProfileLoader(path)
    pre_hash = loader.profiles.content_hash

    env = _FakeEnv()
    wrapper = RewardProfileWrapper(env, loader=loader, hot_reload=False)
    wrapper.reset()

    time.sleep(0.05)
    path.write_text(_DEFAULT_PROFILES_YAML.replace("-0.001", "-0.999"), encoding="utf-8")

    wrapper.reset()
    # Hash unchanged because we never re-stat the file.
    assert wrapper.current_profile_hash == pre_hash


def test_mid_episode_yaml_edit_does_not_affect_current_episode(tmp_path: Path):
    path = tmp_path / "profiles.yaml"
    path.write_text(_DEFAULT_PROFILES_YAML, encoding="utf-8")
    loader = ProfileLoader(path)

    env = _FakeEnv()
    wrapper = RewardProfileWrapper(env, loader=loader, hot_reload=True)
    wrapper.reset()
    pre_profile_name = wrapper.current_profile_name

    # Edit mid-episode (no reset).
    time.sleep(0.05)
    path.write_text(
        _DEFAULT_PROFILES_YAML.replace("-0.001", "-0.999"),
        encoding="utf-8",
    )

    # Take a step — the active profile reference is unchanged.
    env.next_step_occurrences = [StepElapsed()]
    _o, reward, _t, _u, info = wrapper.step(0)
    # step_penalty still uses the pre-edit -0.001 weight.
    assert info["reward_breakdown"]["step_penalty"] == pytest.approx(-0.001)
    assert info["reward_profile"] == pre_profile_name


# ============================================================================
# Constructor validation
# ============================================================================


def test_constructor_rejects_neither_loader_nor_profiles():
    env = _FakeEnv()
    with pytest.raises(ValueError, match=r"exactly one"):
        RewardProfileWrapper(env)


def test_constructor_rejects_both_loader_and_profiles(tmp_path: Path):
    path = tmp_path / "profiles.yaml"
    path.write_text(_DEFAULT_PROFILES_YAML, encoding="utf-8")
    loader = ProfileLoader(path)
    profs = loader.profiles
    env = _FakeEnv()
    with pytest.raises(ValueError, match=r"exactly one"):
        RewardProfileWrapper(env, loader=loader, profiles=profs)

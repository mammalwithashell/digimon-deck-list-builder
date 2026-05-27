"""Parity regression: the shipped `_digivolve_shaped` profile produces
the same per-step reward sequence as the legacy
`DigimonEnv._compute_reward` path with `digivolve_shaping=True`.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"Default profile preserves byte-identical legacy reward shape" —
"Legacy digivolve_shaping=True maps to _digivolve_shaped profile".

Locks in:
- The deprecation alias mapping (digivolve_shaping=True →
  `reward_profile_override="_digivolve_shaped"`).
- The byte-identical contract for the digivolve-shaped recipe
  (`+0.1` per regular digivolve, `+3.9` additional per DNA).
- Legacy-terminal-exclusivity inheritance from `_default`.

Method: mirrors `test_default_profile_byte_identical.py` exactly,
but constructs the legacy env with `digivolve_shaping=True` and
the wrapped env with override `_digivolve_shaped`.
"""

from __future__ import annotations

from pathlib import Path

import pytest

pytest.importorskip("digimon_engine")

from digimon_gym.agents.reward.profile_loader import ProfileLoader  # noqa: E402
from digimon_gym.agents.reward.wrapper import RewardProfileWrapper  # noqa: E402
from digimon_gym.digimon_gym import ACTION_PASS_TURN, DigimonEnv  # noqa: E402


SHIPPED_PROFILES = Path("code/digimon_gym/agents/reward/profiles.yaml")
N_EPISODES = 10
MAX_STEPS_PER_EPISODE = 200


def _run_episode(env, seed: int, action: int = ACTION_PASS_TURN) -> list[float]:
    env.reset(seed=seed)
    rewards: list[float] = []
    for _ in range(MAX_STEPS_PER_EPISODE):
        _obs, reward, terminated, truncated, _info = env.step(action)
        rewards.append(float(reward))
        if terminated or truncated:
            break
    return rewards


def test_digivolve_shaped_profile_byte_identical_per_step_rewards():
    """Legacy `digivolve_shaping=True` and `_digivolve_shaped` profile
    produce identical per-step reward sequences across N seeds.
    """
    loader = ProfileLoader(SHIPPED_PROFILES)

    for seed in range(N_EPISODES):
        legacy_env = DigimonEnv(digivolve_shaping=True)
        profile_env = RewardProfileWrapper(
            DigimonEnv(),
            loader=loader,
            override="_digivolve_shaped",
            hot_reload=False,
        )

        legacy_rewards = _run_episode(legacy_env, seed=seed)
        profile_rewards = _run_episode(profile_env, seed=seed)

        assert len(legacy_rewards) == len(profile_rewards), (
            f"seed={seed}: episode lengths diverge (legacy={len(legacy_rewards)}, "
            f"profile={len(profile_rewards)})"
        )
        for step_idx, (legacy_r, profile_r) in enumerate(zip(legacy_rewards, profile_rewards)):
            assert legacy_r == profile_r, (
                f"seed={seed} step={step_idx}: legacy={legacy_r!r} "
                f"profile={profile_r!r} (delta={profile_r - legacy_r!r}). "
                f"The `_digivolve_shaped` profile in profiles.yaml has drifted "
                f"from the legacy digivolve-shaped recipe (digivolve=+0.1, "
                f"dna_digivolve=+3.9 stacked on _default)."
            )


def test_digivolve_shaped_profile_inherits_legacy_terminal_exclusivity():
    """The `_digivolve_shaped` profile must inherit
    `legacy_terminal_exclusivity=true` from `_default`. Otherwise the
    digivolve component would fire on the terminal step when the legacy
    path's short-circuit skipped it, causing drift.
    """
    loader = ProfileLoader(SHIPPED_PROFILES)
    digi_shaped = loader.profiles.by_name["_digivolve_shaped"]
    default = loader.profiles.by_name["_default"]
    assert default.legacy_terminal_exclusivity is True
    assert digi_shaped.legacy_terminal_exclusivity is True, (
        "_digivolve_shaped MUST inherit legacy_terminal_exclusivity=True "
        "from _default — otherwise digivolve/dna_digivolve components "
        "fire on terminal steps when legacy path's short-circuit skipped them."
    )

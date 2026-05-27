"""Byte-identical regression: the shipped `_default` reward profile
produces the same per-step reward sequence as the legacy
`DigimonEnv._compute_reward` path with `digivolve_shaping=False`.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"Default profile preserves byte-identical legacy reward shape".

This is the gate that defends "existing runs are unchanged" — without
it, any drift in the `_default` profile's terminal magnitudes, security
weights, or step penalty would silently shift the training signal.

Method: run N seeded episodes through TWO envs with identical seeds
and identical action sequences. Env A is bare `DigimonEnv` (legacy
reward path, default flags). Env B is `DigimonEnv` wrapped in
`RewardProfileWrapper` with `_default` profile override. Compare
per-step reward sequences float-for-float using exact equality
(`==`, not `pytest.approx`).

We use action 95 (PASS_TURN) at every step to keep the test
deterministic and fast — the reward formula is exercised independent
of action choice (security delta + step penalty + terminal magnitude
all fire regardless of action variety).
"""

from __future__ import annotations

from pathlib import Path

import pytest

# Skip if Rust wheel isn't installed — the profile path requires
# `get_events_since_last_step` which the legacy Python engine doesn't
# expose. Anyone running training has the wheel; CI builds it.
pytest.importorskip("digimon_engine")

from digimon_gym.agents.reward.profile_loader import ProfileLoader  # noqa: E402
from digimon_gym.agents.reward.wrapper import RewardProfileWrapper  # noqa: E402
from digimon_gym.digimon_gym import ACTION_PASS_TURN, DigimonEnv  # noqa: E402


SHIPPED_PROFILES = Path("code/digimon_gym/agents/reward/profiles.yaml")
# Episodes-per-test default — keep low enough to run in CI in <1s but
# high enough that any non-trivial floating-point drift would surface.
N_EPISODES = 10
MAX_STEPS_PER_EPISODE = 200


def _run_episode(env, seed: int, action: int = ACTION_PASS_TURN) -> list[float]:
    """Run one episode with deterministic actions, returning the
    per-step reward sequence as a list of floats.
    """
    env.reset(seed=seed)
    rewards: list[float] = []
    for _ in range(MAX_STEPS_PER_EPISODE):
        _obs, reward, terminated, truncated, _info = env.step(action)
        rewards.append(float(reward))
        if terminated or truncated:
            break
    return rewards


def test_default_profile_byte_identical_per_step_rewards_across_n_episodes():
    """Legacy `_compute_reward` and `_default` profile path produce
    identical per-step reward sequences for the same seed + actions.
    """
    loader = ProfileLoader(SHIPPED_PROFILES)

    for seed in range(N_EPISODES):
        legacy_env = DigimonEnv()
        profile_env = RewardProfileWrapper(
            DigimonEnv(), loader=loader, override="_default", hot_reload=False,
        )

        legacy_rewards = _run_episode(legacy_env, seed=seed)
        profile_rewards = _run_episode(profile_env, seed=seed)

        assert len(legacy_rewards) == len(profile_rewards), (
            f"seed={seed}: episode lengths diverge (legacy={len(legacy_rewards)}, "
            f"profile={len(profile_rewards)}) — the engine state itself is "
            f"non-deterministic across the two paths, which means the wrapper "
            f"is mutating engine behavior (it MUST NOT)."
        )
        # Float-for-float equality. If this fails on a single step,
        # `_default` has drifted from the legacy recipe.
        for step_idx, (legacy_r, profile_r) in enumerate(zip(legacy_rewards, profile_rewards)):
            assert legacy_r == profile_r, (
                f"seed={seed} step={step_idx}: legacy={legacy_r!r} "
                f"profile={profile_r!r} (delta={profile_r - legacy_r!r}). "
                f"The `_default` profile in profiles.yaml has drifted from the "
                f"legacy `DigimonEnv._compute_reward` formula. This breaks the "
                f"'existing runs are unchanged' guarantee from spec "
                f"'Default profile preserves byte-identical legacy reward shape'."
            )


def test_default_profile_terminal_step_matches_legacy():
    """When a game terminates, the `_default` profile's
    `terminal_outcome` component fires the same value as the legacy
    terminal magnitude calculation.

    This is a focused test of the terminal step specifically — the
    parametric one above samples 10 random seeds but most starter-deck
    games don't terminate within MAX_STEPS_PER_EPISODE under PASS_TURN.
    Use the env's `force_step_limit_winner` to force a terminal at a
    known step, then compare.
    """
    loader = ProfileLoader(SHIPPED_PROFILES)
    legacy_env = DigimonEnv(max_turns=5)  # tight truncation → fast terminal
    profile_env = RewardProfileWrapper(
        DigimonEnv(max_turns=5), loader=loader, override="_default", hot_reload=False,
    )

    legacy_rewards = _run_episode(legacy_env, seed=42)
    profile_rewards = _run_episode(profile_env, seed=42)

    assert len(legacy_rewards) == len(profile_rewards)
    # Terminal step is the last reward. Both should fire the terminal
    # magnitude (loss for the agent under forced step-limit, since
    # PASS_TURN doesn't make progress).
    assert legacy_rewards[-1] == profile_rewards[-1], (
        f"terminal step: legacy={legacy_rewards[-1]!r} "
        f"profile={profile_rewards[-1]!r}"
    )

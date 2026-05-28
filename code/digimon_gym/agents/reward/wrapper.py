"""`RewardProfileWrapper` — Gymnasium wrapper that applies a reward
profile to the wrapped env's per-step output.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"RewardProfileWrapper sits between DigimonEnv and OpponentWrapper".

Wrapper-chain position:

    DigimonEnv
        v
    RewardProfileWrapper          ← this module
        v
    OpponentWrapper / MatchEnv / DeckPoolWrapper / ...

The wrapped env (`DigimonEnv` in production) is responsible for
writing the per-step occurrences into `info["reward_occurrences"]`
(see Group 7 task 7.2). This wrapper:

- **reset()**: optionally hot-reloads the profiles file, reads
  `info["deck1_archetype"]`, resolves the active profile (override →
  archetype lookup → `_default` fallback), initializes a fresh
  `episode_state` dict, writes `info["reward_profile"]` and
  `info["reward_profile_hash"]`.

- **step()**: reads `info["reward_occurrences"]`, invokes each
  component's `compute()`, applies the per-component budget
  (`ComponentBudget.apply`), sums the contributions, applies the
  per-profile budget (`ProfileBudget.apply`), writes the breakdown
  into info, and returns the post-profile scalar reward.

BO3 interaction: `MatchEnv` (the outer BO3 wrapper) only calls
`env.reset()` at MATCH start, not between games within a match. So
this wrapper's `episode_state` and `_active_profile` persist across
all games of a BO3 match, matching the spec's per-match budget
semantics and the per-match `once_per_episode` interpretation.
"""

from __future__ import annotations

from typing import Any, Dict, List, MutableMapping, Optional, Tuple

import gymnasium

from .occurrences import StepElapsed  # noqa: F401 — kept as a documented import surface
from .profile_loader import Profile, ProfileLoader, Profiles


class RewardProfileWrapper(gymnasium.Wrapper):
    """Wrap `DigimonEnv` (or any env that emits `info["reward_occurrences"]`)
    with a reward-profile-driven shaping pipeline.

    Args:
        env: the wrapped env. Its `step` MUST place a
             `list[Occurrence]` under `info["reward_occurrences"]`.
        loader: a `ProfileLoader` for hot-reload support. Use this in
                production. For tests that don't need hot-reload, pass
                `profiles` directly.
        profiles: an already-loaded `Profiles` snapshot. Alternative to
                  `loader` for tests or non-hot-reload runs.
        override: an explicit profile name that takes precedence over
                  `info["deck1_archetype"]`. Validated at load.
        hot_reload: when True (default), the wrapper calls
                    `loader.reload_if_changed()` at each `reset()`.
                    No effect when `profiles` is passed directly.

    Raises:
        ValueError: if both `loader` and `profiles` are passed, or
                    neither is; or if `override` names a profile that
                    isn't in the loaded set.
    """

    def __init__(
        self,
        env: gymnasium.Env,
        *,
        loader: Optional[ProfileLoader] = None,
        profiles: Optional[Profiles] = None,
        override: Optional[str] = None,
        hot_reload: bool = True,
    ) -> None:
        super().__init__(env)
        if (loader is None) == (profiles is None):
            raise ValueError(
                "RewardProfileWrapper requires exactly one of `loader` or `profiles`"
            )
        self._loader = loader
        self._profiles_override: Optional[Profiles] = profiles
        self._hot_reload = hot_reload
        self._override_name = override
        self._validate_override(self._current_profiles())

        # Flip the wrapped env's flags so:
        # 1. `emit_reward_occurrences` is on → DigimonEnv writes
        #    `info["reward_occurrences"]` for us to consume.
        # 2. `legacy_reward` is off → DigimonEnv returns 0.0 from its
        #    reward path, so we're the sole reward source and there's
        #    no wasted legacy computation.
        # Soft-flip via setattr so we don't break test envs that don't
        # have these attributes (the wrapper still works against any env
        # that supplies `info["reward_occurrences"]` directly).
        if hasattr(env, "emit_reward_occurrences"):
            env.emit_reward_occurrences = True
        if hasattr(env, "legacy_reward"):
            env.legacy_reward = False
        # If `env` is a real DigimonEnv that wasn't constructed with
        # the emit flag on, its `_reward_event_bus` is still None and
        # `info["reward_occurrences"]` would be missing. Materialize
        # the bus lazily so flipping the flag has effect.
        if hasattr(env, "_reward_event_bus") and getattr(env, "_reward_event_bus") is None:
            from digimon_gym.digimon_gym import _make_reward_event_bus
            env._reward_event_bus = _make_reward_event_bus()

        # Per-episode state. Initialized in reset(). The dict is shared
        # across all of the active profile's components and budgets —
        # ComponentBudget and ProfileBudget use disjoint key prefixes
        # to avoid collisions.
        self._episode_state: MutableMapping[str, Any] = {}
        self._active_profile: Optional[Profile] = None

    # ── Public read-only properties (consumed by sidecar writer + tests) ──

    @property
    def current_profile_name(self) -> Optional[str]:
        """Active profile name for the current episode (None pre-reset)."""
        return self._active_profile.name if self._active_profile else None

    @property
    def current_profile_hash(self) -> str:
        """Content hash of the profiles file at the time the current
        episode's profile was resolved. Recorded per-game in the
        evals.jsonl sidecar (Group 10 task 10.6).
        """
        return self._current_profiles().content_hash

    @property
    def loader(self) -> Optional[ProfileLoader]:
        return self._loader

    def __getattr__(self, name: str):
        """Delegate missing attribute access to the wrapped env.

        gymnasium.Wrapper provides limited `__getattr__` delegation —
        not every attribute on `self.env` falls through. Downstream
        wrappers (MulliganLogWrapper, recording wrappers, etc.) read
        DigimonEnv-specific attributes like `is_game_over`,
        `current_player_id`, `winner_id`, and `runner` directly on
        their wrapped env. Add explicit delegation so the wrapper is
        transparent to those access patterns.
        """
        # Avoid infinite recursion on `env` itself (the only attr we own
        # before __init__ completes). Hit __dict__ directly.
        if name == "env":
            raise AttributeError(name)
        env = self.__dict__.get("env")
        if env is None:
            raise AttributeError(name)
        return getattr(env, name)

    # ── Gymnasium interface ───────────────────────────────────────

    def reset(self, **kwargs: Any) -> Tuple[Any, Dict[str, Any]]:
        # Hot reload before resolving the active profile so this
        # episode picks up any operator edits since the last reset.
        if self._loader is not None and self._hot_reload:
            self._loader.reload_if_changed()
            # Re-validate override against the (possibly updated) profile set.
            self._validate_override(self._loader.profiles)

        obs, info = self.env.reset(**kwargs)

        archetype = info.get("deck1_archetype")
        profs = self._current_profiles()
        self._active_profile = profs.resolve_active(
            archetype=archetype, override=self._override_name,
        )
        self._episode_state = {}

        info["reward_profile"] = self._active_profile.name
        info["reward_profile_hash"] = profs.content_hash
        return obs, info

    def step(self, action: Any) -> Tuple[Any, float, bool, bool, Dict[str, Any]]:
        obs, _unused_reward, terminated, truncated, info = self.env.step(action)

        # The wrapped env wrote occurrences here. If it didn't (unusual —
        # only happens in tests that wrap a non-DigimonEnv), treat as
        # an empty step (no shaping fires; step_penalty needs StepElapsed
        # but the bus emits that, so a synthetic empty path is fine).
        occurrences = info.get("reward_occurrences", [])

        assert self._active_profile is not None, (
            "RewardProfileWrapper.step called before reset"
        )
        profile = self._active_profile

        # add-gameplay-reward-config: `legacy_terminal_exclusivity`
        # is REMOVED. The new universal gameplay shape's terminal
        # components (terminal_outcome + quick_win_bonus +
        # stall_penalty) compose with the dense signal naturally on
        # the terminal step; no carve-out is needed. Custom profiles
        # that still need terminal-step exclusivity should define a
        # custom component that filters on `TerminalOutcome` itself.

        # Pass 1: each component computes its raw contribution and the
        # per-component budget engine clamps it. Tuples ordered for
        # deterministic ProfileBudget application.
        per_component_pre_profile: List[Tuple[str, str, float]] = []
        for mc in profile.components:
            raw = float(mc.component.compute(occurrences, self._episode_state))
            adjusted = mc.budget.apply(
                raw, profile.name, mc.component_key, self._episode_state,
            )
            per_component_pre_profile.append((mc.kind, mc.name, adjusted))

        # Pass 2: profile-level cap/floor over the summed shaping
        # contributions. terminal_outcome + step_penalty exempt
        # (handled inside ProfileBudget.apply).
        post_profile, clamped = profile.profile_budget.apply(
            per_component_pre_profile, profile.name, self._episode_state,
        )

        # Build the breakdown + scalar.
        breakdown: Dict[str, float] = {}
        scalar = 0.0
        for _kind, name, value in post_profile:
            breakdown[name] = value
            scalar += value

        info["reward_profile"] = profile.name
        info["reward_breakdown"] = breakdown
        if clamped:
            info["reward_breakdown_clamped"] = clamped

        return obs, scalar, terminated, truncated, info

    # ── Internals ─────────────────────────────────────────────────

    def _current_profiles(self) -> Profiles:
        if self._loader is not None:
            return self._loader.profiles
        assert self._profiles_override is not None
        return self._profiles_override

    def _validate_override(self, profs: Profiles) -> None:
        if self._override_name is None:
            return
        if self._override_name not in profs.by_name:
            raise ValueError(
                f"reward_profile_override={self._override_name!r} does not name "
                f"a defined profile (available: {sorted(profs.by_name)})"
            )

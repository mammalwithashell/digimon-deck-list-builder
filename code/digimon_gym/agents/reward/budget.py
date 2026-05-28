"""Budget engine — per-component and per-profile clamping over each
component's raw `compute()` output.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"Per-component budget controls" + "Per-profile budget cap and floor".

The `RewardProfileWrapper` invokes this engine AFTER each component's
`compute()` returns its raw contribution. Components themselves stay
pure — they don't track per-episode counters.

State lives in `episode_state` (the wrapper's per-episode dict) keyed by:

- Per-component:  `("component_budget", profile_name, component_key)`
  → `{"fires": int, "running_total": float}` (running_total tracks
  signed magnitude for the max_total_per_episode clamp).
- Per-profile:    `("profile_budget", profile_name)`
  → `{"positive_total": float, "negative_total": float}`.

Resolution order per fire (matches D11):

  1. Compute base emission via component (`weight × event_multiplier`).
  2. Apply `diminishing_returns_factor`: nth fire emits raw × factor^(n-1).
  3. Apply `max_total_per_episode`: clamp emission so |running_total|
     does not exceed the budget magnitude. Sign-respecting — a +1.0
     emission with running_total=1.4 and budget=1.5 emits +0.1.
  4. Apply `max_fires_per_episode`: if the prior fire count has hit
     the cap, emit 0.0.
  5. Apply profile-level cap/floor (per separate function below).

Step (5) is exposed as `ProfileBudget.apply()` and called by the
wrapper after summing all component contributions for the step.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, List, MutableMapping, Optional, Tuple


# Spec carve-out (D12): per-profile budgets ignore these "foundation"
# components — they are part of the recipe, not shaping. The cap/floor
# clamps shaping only, so a runaway `terminal_outcome` win bonus stays
# intact even when shaped components are capped.
PROFILE_BUDGET_EXEMPT_KINDS = frozenset({"terminal_outcome", "step_penalty"})


@dataclass(frozen=True)
class ComponentBudget:
    """Per-component-instance budget config. Wraps a single component's
    `compute()` output through the steps-1-through-4 pipeline.
    """

    max_fires_per_episode: Optional[int] = None
    max_total_per_episode: Optional[float] = None
    diminishing_returns_factor: float = 1.0

    def apply(
        self,
        raw: float,
        profile_name: str,
        component_key: str,
        episode_state: MutableMapping[str, Any],
    ) -> float:
        """Apply per-component budget pipeline to a raw `compute()`
        output. Returns the post-budget contribution.

        `component_key` is a stable unique key per component instance
        within `profile_name` (e.g., the loader's canonical key from
        `KIND_KEY_PARAMETERS`). Distinct components share `profile_name`
        but use different `component_key` so their state doesn't
        collide.
        """
        if raw == 0.0:
            # Skip state-mutating budget work when the raw fire was zero —
            # an unused step doesn't count against any budget.
            return 0.0

        state_key = ("component_budget", profile_name, component_key)
        state: Dict[str, float] = episode_state.setdefault(  # type: ignore[assignment]
            state_key, {"fires": 0, "running_total": 0.0}
        )

        # Step 4 (pre-check): max_fires gate. Checking BEFORE
        # diminishing-returns mutation matters — a fire that's
        # gated to zero shouldn't bump the counter.
        if (
            self.max_fires_per_episode is not None
            and state["fires"] >= self.max_fires_per_episode
        ):
            return 0.0

        # Step 2: diminishing returns. `fires` is the count of PRIOR
        # fires, so nth fire (0-indexed) multiplies by factor^n.
        if self.diminishing_returns_factor != 1.0:
            adjusted = raw * (self.diminishing_returns_factor ** state["fires"])
        else:
            adjusted = raw

        # Step 3: max_total clamp. Sign-respecting — the running_total
        # for a negative-weight component goes negative, and the budget
        # is interpreted as a floor (closer to zero is allowed).
        if self.max_total_per_episode is not None:
            running = state["running_total"]
            if adjusted >= 0 and self.max_total_per_episode >= 0:
                # Positive direction: cap so running + adjusted <= cap.
                remaining = self.max_total_per_episode - running
                if remaining <= 0:
                    adjusted = 0.0
                elif adjusted > remaining:
                    adjusted = remaining
            elif adjusted < 0 and self.max_total_per_episode <= 0:
                # Negative direction: floor so running + adjusted >= floor.
                remaining = self.max_total_per_episode - running
                if remaining >= 0:
                    adjusted = 0.0
                elif adjusted < remaining:
                    adjusted = remaining
            # Mixed-sign cases: the budget's sign indicates which
            # direction it bounds. A positive budget does not clamp
            # negative emissions (those are bounded by a paired floor
            # on the profile budget instead).

        # If after clamping the emission rounded to exactly zero, don't
        # bump the fire counter — there's no "fire" if no value emerged.
        if adjusted == 0.0:
            return 0.0

        # Commit state and return.
        state["fires"] = int(state["fires"]) + 1
        state["running_total"] = float(state["running_total"]) + adjusted
        return float(adjusted)


@dataclass(frozen=True)
class ProfileBudget:
    """Per-profile cap and floor over the summed contributions of all
    shaping components in a step.

    Operates on the *list of post-component-budget contributions* for a
    single step. Returns a clamped list (same length, same order) plus
    a dict mapping component-name → clamped-away amount for telemetry.

    Per spec: `terminal_outcome` and `step_penalty` are exempt from the
    per-profile cap/floor.
    """

    per_episode_cap: Optional[float] = None
    per_episode_floor: Optional[float] = None

    def apply(
        self,
        contributions: List[Tuple[str, str, float]],
        profile_name: str,
        episode_state: MutableMapping[str, Any],
    ) -> Tuple[List[Tuple[str, str, float]], Dict[str, float]]:
        """Clamp the step's contribution list against the profile's
        per-episode running totals.

        `contributions` is `[(component_kind, component_name, value), ...]`.
        Returns the post-clamp list (same shape) and a `clamped` dict
        keyed by component_name with the absolute amount clamped away.
        Components whose value is unchanged don't appear in `clamped`.
        """
        if self.per_episode_cap is None and self.per_episode_floor is None:
            return list(contributions), {}

        state_key = ("profile_budget", profile_name)
        state: Dict[str, float] = episode_state.setdefault(  # type: ignore[assignment]
            state_key, {"positive_total": 0.0, "negative_total": 0.0}
        )

        out: List[Tuple[str, str, float]] = []
        clamped: Dict[str, float] = {}

        for kind, comp_name, value in contributions:
            if kind in PROFILE_BUDGET_EXEMPT_KINDS:
                out.append((kind, comp_name, value))
                continue
            if value == 0.0:
                out.append((kind, comp_name, value))
                continue

            if value > 0 and self.per_episode_cap is not None:
                remaining = self.per_episode_cap - state["positive_total"]
                if remaining <= 0:
                    clamped[comp_name] = value
                    out.append((kind, comp_name, 0.0))
                    continue
                if value > remaining:
                    clamped[comp_name] = value - remaining
                    state["positive_total"] = float(self.per_episode_cap)
                    out.append((kind, comp_name, float(remaining)))
                    continue
                state["positive_total"] = float(state["positive_total"]) + value
                out.append((kind, comp_name, value))
                continue

            if value < 0 and self.per_episode_floor is not None:
                # Floor: state["negative_total"] is <= 0; floor is <= 0.
                # `remaining` is how much more negative we may go (negative number).
                remaining = self.per_episode_floor - state["negative_total"]
                if remaining >= 0:
                    # No more negative budget; clamp to zero.
                    clamped[comp_name] = value
                    out.append((kind, comp_name, 0.0))
                    continue
                if value < remaining:
                    # value is more negative than remaining → emit `remaining`.
                    clamped[comp_name] = value - remaining  # negative magnitude clipped
                    state["negative_total"] = float(self.per_episode_floor)
                    out.append((kind, comp_name, float(remaining)))
                    continue
                state["negative_total"] = float(state["negative_total"]) + value
                out.append((kind, comp_name, value))
                continue

            # No matching cap/floor for this sign — pass through.
            out.append((kind, comp_name, value))

        return out, clamped

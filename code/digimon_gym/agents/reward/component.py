"""`RewardComponent` protocol — the uniform interface every component
implements.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"Reward components are composable units with a uniform interface".

Contract:

- Each component exposes a stable string `name` (used as the key in
  `info["reward_breakdown"]` and the TB scalar suffix).
- `compute(occurrences, episode_state) -> float` is pure with respect
  to its inputs — given the same `occurrences` list and equal
  `episode_state` mapping, returns the same float.
- Components SHALL NOT read engine state directly. All signals derive
  from the occurrences stream and from `episode_state` (per-episode
  memory carried between steps by the `RewardProfileWrapper`).

Components return a single per-step contribution (positive, negative,
or zero). The `RewardProfileWrapper` applies the per-component and
per-profile budget engines (`budget.py`) AFTER the component returns,
so component code does NOT need to track budget state.

The component's `name` field is set at construction (allowing the
profile loader to disambiguate multiple instances of the same kind —
e.g., two `play_named_card` components matching different cards — by
appending a `:<key>` suffix in YAML resolution).
"""

from __future__ import annotations

from typing import Any, MutableMapping, Protocol, Sequence, runtime_checkable

from .occurrences import (
    Blocked,
    Digivolved,
    DnaDigivolved,
    MemoryShifted,
    OppDeleted,
    OwnDeleted,
    PlayedCard,
    SecurityLost,
    SecurityRemoved,
    StepElapsed,
    TerminalOutcome,
)


# Union of all occurrence variants. Using a tuple of dataclass types
# rather than a TypeAlias-based union so isinstance() works directly
# in component implementations.
OccurrenceLike = (
    PlayedCard,
    Digivolved,
    DnaDigivolved,
    Blocked,
    OppDeleted,
    OwnDeleted,
    SecurityRemoved,
    SecurityLost,
    MemoryShifted,
    TerminalOutcome,
    StepElapsed,
)


@runtime_checkable
class RewardComponent(Protocol):
    """The uniform interface every component implements.

    `name` is the stable key under which the component's per-step
    contribution lands in `info["reward_breakdown"]` and the suffix
    on `pilot/reward/<name>/mean_per_game` TB scalars.

    `compute()` returns the component's per-step contribution as a
    float. It runs BEFORE the budget engine — so contribution may be
    later clamped or zeroed by `ComponentBudget` / `ProfileBudget`.
    """

    name: str

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        ...

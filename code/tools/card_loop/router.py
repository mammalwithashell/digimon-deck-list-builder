"""Per-stage model-family routing (design D5, D12; spec model-scorecard, interaction-exams).

`route(stage, item, ...)` decides which family does a call, in this order:

1. **forced** — `author_interaction` for a card whose implementer family is
   known goes to the *other* family, outside the exploration share (interaction
   exams are authored adversarially). If that family is unavailable the call
   cannot be made faithfully and `RoutingUnavailable` is raised: the driver
   defers or escalates the item; it never lets the implementer examine itself.
2. **routed** — the stage default from `config.routes`, unless the scorecard
   shows a clear winner: both families have >= `routing_min_samples` attempts in
   the stage at the same `prompt_version` and their corrected-cost intervals
   separate; then the cheaper family is routed.
3. **explore** — a seeded hash of `(config.seed, stage, item)` mapped to [0, 1)
   below `exploration_share` (20%) sends the item to the non-routed family. The
   draw depends only on those three values, so re-planning an item reproduces
   its assignment.
4. **availability** — if the chosen family is unavailable (quota exhausted,
   CLI missing) the remaining family takes the call as `routed`.

Terminating stages (`contracts.TERMINATING_STAGES`) need both families (D5):
the first opinion is routed normally; `second_opinion_family(first)` names the
other family and raises `SingleFamilyTermination` when it is unavailable — a
terminating call then escalates, it never degrades to one family.

The router reads the scorecard; workers never do.
"""
from __future__ import annotations

from typing import Iterable, Protocol

from ._hashing import stable_unit
from ._stats import intervals_separate
from .config import LoopConfig
from .contracts import FAMILIES, STAGES, TERMINATING_STAGES

ADVERSARIAL_STAGES = ("author_interaction",)


class RoutingUnavailable(RuntimeError):
    """No family can take this call under the rules; the driver must defer/escalate."""


class SingleFamilyTermination(RoutingUnavailable):
    """A terminating call needs a second family that is not available (D5)."""


class _StatsLike(Protocol):
    attempts: int
    corrected_cost_ci: tuple[float, float] | None


class ScorecardLike(Protocol):
    def family_stats(self, stage: str, family: str, prompt_version: str,
                     model: str | None = None) -> _StatsLike | None: ...


def other_family(family: str) -> str:
    if family not in FAMILIES:
        raise ValueError(f"unknown family {family!r}")
    (other,) = [f for f in FAMILIES if f != family]
    return other


def exploration_draw(seed: int, stage: str, item: str) -> float:
    return stable_unit("route", seed, stage, item)


def is_terminating(stage: str) -> bool:
    return stage in TERMINATING_STAGES


def _available(available: Iterable[str]) -> tuple[str, ...]:
    allowed = set(available)
    avail = tuple(f for f in FAMILIES if f in allowed)
    if not avail:
        raise RoutingUnavailable("no model family is available")
    return avail


def scorecard_choice(stage: str, *, config: LoopConfig, scorecard: ScorecardLike | None,
                     prompt_version: str | None) -> str | None:
    """The family the evidence says is cheaper, or None to keep the default.

    None unless both families have >= `routing_min_samples` attempts at this
    stage and prompt version, both have a priced corrected-cost interval, and
    the intervals do not overlap. Without a `prompt_version` there is no
    like-for-like comparison, so the default stands.
    """
    if scorecard is None or prompt_version is None:
        return None
    cis = {}
    for fam in FAMILIES:
        model = (config.models.get(fam) or {}).get("model")
        s = scorecard.family_stats(stage, fam, prompt_version, model=model)
        if s is None or s.attempts < config.routing_min_samples or s.corrected_cost_ci is None:
            return None
        cis[fam] = s.corrected_cost_ci
    a, b = FAMILIES
    sep = intervals_separate(cis[a], cis[b])
    return a if sep < 0 else b if sep > 0 else None


def route(stage: str, item: str, *, config: LoopConfig, scorecard: ScorecardLike | None = None,
          implementer_family: str | None = None, available: Iterable[str] = FAMILIES,
          prompt_version: str | None = None) -> tuple[str, str]:
    """`(family, assignment)` with assignment in `ledger.ASSIGNMENTS`."""
    if stage not in STAGES:
        raise ValueError(f"unknown stage {stage!r}")
    avail = _available(available)

    if stage in ADVERSARIAL_STAGES and implementer_family is not None:
        target = other_family(implementer_family)
        if target not in avail:
            raise RoutingUnavailable(
                f"{stage} for {item}: implementer was {implementer_family} and {target} is "
                "unavailable; an implementer may not author its own interaction exams")
        return target, "forced"

    default = config.routes.get(stage)
    if default not in FAMILIES:
        raise ValueError(f"no route configured for stage {stage!r}")
    routed = scorecard_choice(stage, config=config, scorecard=scorecard,
                              prompt_version=prompt_version) or default

    if exploration_draw(config.seed, stage, item) < config.exploration_share:
        family, assignment = other_family(routed), "explore"
    else:
        family, assignment = routed, "routed"

    if family not in avail:
        # An unavailable explore target falls back to the routed family; an
        # unavailable routed family falls over to the one that remains.
        family, assignment = avail[0], "routed"
    return family, assignment


def second_opinion_family(first: str, available: Iterable[str] = FAMILIES) -> str:
    """The independent second family for a terminating call (D5)."""
    second = other_family(first)
    if second not in _available(available):
        raise SingleFamilyTermination(
            f"terminating call needs {second} as a second opinion but it is unavailable; "
            "escalate instead of deciding with one family")
    return second

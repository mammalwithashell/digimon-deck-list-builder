"""`play_named_card` component with cost-aware and alt-path gating.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"v1 component catalog" entry 7 + scenario "play_named_card with cost
gating fires only on reduced-cost arrivals".

Match keys (at least one required; multiple ANDed together):
- `card_id`     — exact card-ID match. String or list of strings.
- `card_name`   — exact card-name match. String or list of strings.
- `trait`       — membership in the played card's trait list.
                  String or list of strings (membership = any-of).

Optional gating keys (all SHALL match when set; ANDed with `match`):
- `cost_paid_lt`            — integer; fires only if event.cost_paid < N
- `cost_paid_eq`            — integer; fires only if event.cost_paid == N
- `cost_paid_gte`           — integer; fires only if event.cost_paid >= N
- `cost_paid_lt_printed`    — bool; convenience for cost_paid < cost_printed
- `cost_paid_gte_printed`   — bool; convenience for cost_paid >= cost_printed
- `via_alt_path`            — string or list of strings; fires only if
                              event.via_alt_path matches one entry.

NOTE: `trait` matching requires `PlayedCard.traits` to be present in the
occurrence. The current `RewardEventBus` does NOT yet enrich `PlayedCard`
with traits — adding that enrichment is a follow-up. For v1, profile
authors should use `card_id` or `card_name` matchers; trait-based
matching is supported in the component but will silently no-op until the
bus enriches.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, List, MutableMapping, Optional, Sequence, Union

from ..occurrences import PlayedCard

# Matchers / gates accept either a single value or a list.
StringOrList = Union[str, List[str]]


def _as_list(value: Optional[StringOrList]) -> Optional[List[str]]:
    """Normalize `str | list[str] | None` to `Optional[List[str]]`."""
    if value is None:
        return None
    if isinstance(value, str):
        return [value]
    return list(value)


@dataclass
class PlayNamedCardComponent:
    """Emits `weight` for each agent-side `PlayedCard` event whose
    `card_id`/`card_name` matches AND whose cost / alt-path satisfies
    every set gating key. Multiple matches in one step accumulate.

    The component does NOT enforce a per-episode cap or diminishing
    returns — those are the responsibility of the budget engine
    (`budget.py`), applied by the `RewardProfileWrapper` after this
    `compute` call.
    """

    name: str
    weight: float
    # Match keys (at least one MUST be set; loader-side validation).
    match_card_id: Optional[List[str]] = None
    match_card_name: Optional[List[str]] = None  # name matching enrichment is TBD
    match_trait: Optional[List[str]] = None
    # Cost gating.
    cost_paid_lt: Optional[int] = None
    cost_paid_eq: Optional[int] = None
    cost_paid_gte: Optional[int] = None
    cost_paid_lt_printed: bool = False
    cost_paid_gte_printed: bool = False
    # Alt-path gating.
    via_alt_path: Optional[List[str]] = field(default=None)

    def _matches(self, ev: PlayedCard) -> bool:
        # At least one positive match must be present and ALL set matches
        # AND together.
        any_match_set = (
            self.match_card_id is not None
            or self.match_card_name is not None
            or self.match_trait is not None
        )
        if not any_match_set:
            return False  # mis-configured; loader should catch but be defensive

        if self.match_card_id is not None and ev.card_id not in self.match_card_id:
            return False
        # `card_name` and `trait` matching require occurrence enrichment
        # that the bus does not yet provide. Treat as "matched" only when
        # the matcher list is None (i.e., not set). When set, fall through
        # to fail-closed: profile author should know enrichment is TBD.
        if self.match_card_name is not None:
            # Pessimistic: no name enrichment yet, so even an explicit
            # name matcher cannot match. Documented limitation in
            # the component's module docstring.
            return False
        if self.match_trait is not None:
            return False

        # Cost gating (ANDed; ignored when unset).
        if self.cost_paid_lt is not None and not (ev.cost_paid < self.cost_paid_lt):
            return False
        if self.cost_paid_eq is not None and ev.cost_paid != self.cost_paid_eq:
            return False
        if self.cost_paid_gte is not None and not (ev.cost_paid >= self.cost_paid_gte):
            return False
        if self.cost_paid_lt_printed and not (ev.cost_paid < ev.cost_printed):
            return False
        if self.cost_paid_gte_printed and not (ev.cost_paid >= ev.cost_printed):
            return False

        # Alt-path gating (None matcher → ignore; otherwise must be
        # in the listed alt-path keys; None on the event = no alt-path,
        # which never matches a non-None list).
        if self.via_alt_path is not None:
            if ev.via_alt_path is None or ev.via_alt_path not in self.via_alt_path:
                return False

        return True

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        count = 0
        for ev in occurrences:
            if not isinstance(ev, PlayedCard):
                continue
            if ev.player != 1:
                continue  # agent-side only (Python pid 1)
            if self._matches(ev):
                count += 1
        return float(self.weight) * float(count) if count else 0.0

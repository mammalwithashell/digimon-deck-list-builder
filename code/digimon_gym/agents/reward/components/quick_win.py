"""`quick_win_bonus` component — turn-based fast-win incentive.

Spec:
`openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
`quick_win_bonus component`.

Fires only on `TerminalOutcome` with `winner_id == 1` (the agent won).
Reads `turn_count` (NOT `step_count`) so the curve maps to game progress
rather than env step count (which inflates with selection-heavy turns).

Formula:
    bonus = max(0, peak_value − decay_per_turn × max(0, turn − peak_turn))

Default-shape rationale (see design.md §D4):
    turn 3 = +5.0   (peak — earliest P1 win)
    turn 4 = +3.75
    turn 5 = +2.5
    turn 6 = +1.25
    turn 7 = 0      (clean seam with stall_penalty start)
    turn 8+ = 0
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, MutableMapping, Sequence

from ..occurrences import TerminalOutcome


@dataclass
class QuickWinBonusComponent:
    """Emits a turn-based bonus only on agent win, decaying linearly
    past `peak_turn` until clamped to zero by the outer `max(0, ...)`.
    """

    name: str
    peak_turn: int = 3
    peak_value: float = 5.0
    decay_per_turn: float = 1.25

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        for ev in occurrences:
            if not isinstance(ev, TerminalOutcome):
                continue
            if ev.winner_id != 1:
                # No bonus on loss or draw — fast-loss bonuses make no
                # sense, and draws are not "wins".
                return 0.0
            # `max(0, turn − peak_turn)` clamps so wins BEFORE peak_turn
            # still pay peak_value (rare but possible if engine ever
            # changes the earliest-win turn). `max(0, ...)` on the
            # outside floors the post-decay value at zero.
            over_peak = max(0, ev.turn_count - self.peak_turn)
            bonus = self.peak_value - self.decay_per_turn * over_peak
            return float(max(0.0, bonus))
        return 0.0

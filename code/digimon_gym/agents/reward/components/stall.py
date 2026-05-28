"""`stall_penalty` component — quadratic discouragement for long games.

Spec:
`openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
`stall_penalty component`.

Fires on EVERY `TerminalOutcome` (win, loss, OR draw). Reads
`turn_count`. Unbounded by design — the agent should never play
30-turn games.

Formula:
    penalty = −scale × max(0, turn − threshold_turn)²

Apply gates:
- `apply_to_winner=False` zeroes emission on agent wins (winner_id=1).
- `apply_to_loser=False` zeroes emission on agent losses (winner_id=2).
- Draws (`winner_id=None`) ALWAYS receive the penalty regardless of the
  apply flags — see spec scenario "Draws always penalized regardless of
  apply flags".
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, MutableMapping, Sequence

from ..occurrences import TerminalOutcome


@dataclass
class StallPenaltyComponent:
    """Quadratic-in-turn-count terminal penalty applied symmetrically
    (by default) to wins, losses, and draws.
    """

    name: str
    threshold_turn: int = 7
    scale: float = 0.1
    apply_to_winner: bool = True
    apply_to_loser: bool = True

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        for ev in occurrences:
            if not isinstance(ev, TerminalOutcome):
                continue
            # Win/loss apply-gates. Draws bypass these — draws are not
            # what the operator is dialing in/out with apply_to_*.
            if ev.winner_id == 1 and not self.apply_to_winner:
                return 0.0
            if ev.winner_id == 2 and not self.apply_to_loser:
                return 0.0
            over_threshold = max(0, ev.turn_count - self.threshold_turn)
            penalty = -float(self.scale) * float(over_threshold * over_threshold)
            return penalty
        return 0.0

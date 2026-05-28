"""`stall_penalty` component — quadratic discouragement for long games.

Spec:
`openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
`stall_penalty component`.

Fires on EVERY `TerminalOutcome` (win, loss, OR draw). Reads
`turn_count`.

Formula:
    penalty = −scale × max(0, turn − threshold_turn)²

Apply gates:
- `apply_to_winner=False` zeroes emission on agent wins (winner_id=1).
- `apply_to_loser=False` zeroes emission on agent losses (winner_id=2).
- Draws (`winner_id=None`) ALWAYS receive the penalty regardless of the
  apply flags — see spec scenario "Draws always penalized regardless of
  apply flags".

Win-side floor (`winner_max_penalty`):
The winner-side penalty is CAPPED at `winner_max_penalty` (positive
magnitude — internally applied as `max(penalty, -winner_max_penalty)`).
The loser side and draws remain unbounded so the agent feels real
gradient pressure to wrap games quickly.

The cap exists to enforce the invariant **"a win is always strictly
better than a loss"**. Without the cap, `−0.1×(turn−7)²` grows
unboundedly on wins too, eventually making a slow win worse than a
fast loss. That would let the agent rationally prefer "lose fast" over
"win slow." With the default `winner_max_penalty = 19.0` (and the
default `terminal_outcome.win_base = 10.0` / `loss = -10.0`), the
worst-possible win is `10 − 19 = -9.0`, which is strictly greater than
the best-possible loss (`-10.0` at turn ≤ threshold_turn). Operators
who tune `terminal_outcome.win_base` or `loss` SHOULD update
`winner_max_penalty` to preserve the invariant:

    winner_max_penalty < win_base − loss

Losers and draws are unbounded by design — agents must never play
30-turn games, and the unbounded gradient on losses is what teaches
that.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, MutableMapping, Sequence

from ..occurrences import TerminalOutcome


@dataclass
class StallPenaltyComponent:
    """Quadratic-in-turn-count terminal penalty applied symmetrically
    (by default) to wins, losses, and draws — with a configurable
    cap on the winner side so a win is never worse than a loss.
    """

    name: str
    threshold_turn: int = 7
    scale: float = 0.1
    apply_to_winner: bool = True
    apply_to_loser: bool = True
    # Positive magnitude. Default 19.0 keeps the worst-possible win
    # (win_base 10 − cap 19 = -9) strictly above the worst-possible loss
    # (−10 at turn ≤ threshold). See module docstring.
    winner_max_penalty: float = 19.0

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
            # Winner-side floor: clamp so a slow win is never worse than
            # a fast loss. The loser-side stays unbounded.
            if ev.winner_id == 1:
                penalty = max(penalty, -float(self.winner_max_penalty))
            return penalty
        return 0.0

"""`terminal_outcome` and `step_penalty` components — the foundation
recipe every profile inherits via `_base_terminal`.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"v1 component catalog" entries 1 + 2.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, MutableMapping, Sequence

from ..occurrences import StepElapsed, TerminalOutcome


@dataclass
class TerminalOutcomeComponent:
    """Emits the win/loss/draw scalar plus the fast-win bonus curve at
    episode termination.

    Parameters (per spec):
    - `win_base`           — base reward on agent win (Python pid 1).
    - `fast_win_bonus_max` — maximum bonus added linearly in remaining
                             steps; applied only on win.
    - `fast_win_par_steps` — step count at which the bonus tapers to 0.
                             A win at `par_steps` returns `win_base + 0`;
                             a win at step 0 returns `win_base + bonus_max`.
                             Bonus is clamped at zero below par_steps so
                             slow wins still pay `win_base` exactly.
    - `loss`               — scalar on agent loss.
    - `draw`               — scalar on draw (or step-limit truncation
                             with no winner).

    Per spec scenario "terminal_outcome fast-win bonus is linear":
        win_base=10.0, bonus_max=5.0, par=200, step_count=150
        → 10.0 + (200-150)/200 × 5.0 = 11.25
    """

    name: str
    win_base: float
    fast_win_bonus_max: float
    fast_win_par_steps: int
    loss: float
    draw: float

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        for ev in occurrences:
            if not isinstance(ev, TerminalOutcome):
                continue
            # Agent is always Python pid 1 in DigimonEnv.
            if ev.winner_id == 1:
                # Fast-win bonus: linear in (par - step_count) / par,
                # clamped at zero so wins beyond par_steps just pay win_base.
                slack = max(0.0, float(self.fast_win_par_steps - ev.step_count))
                bonus = (slack / max(1, self.fast_win_par_steps)) * self.fast_win_bonus_max
                return float(self.win_base) + bonus
            if ev.winner_id == 2:
                return float(self.loss)
            # No winner → draw (or step-limit-without-winner).
            return float(self.draw)
        return 0.0


@dataclass
class StepPenaltyComponent:
    """Emits `weight` once per env step. Typically a small negative
    value to bias the agent toward shorter games.
    """

    name: str
    weight: float

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        for ev in occurrences:
            if isinstance(ev, StepElapsed):
                return float(self.weight)
        # If StepElapsed wasn't in occurrences (unusual — only happens if
        # a wrapper assembles a synthetic step), don't emit. The bus emits
        # exactly one per env.step() call.
        return 0.0

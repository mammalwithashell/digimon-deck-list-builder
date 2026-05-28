"""`match_outcome` component — per-BO3-match terminal reward.

Spec:
`openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
`match_outcome component`.

Fires on `MatchOutcome` occurrences (one per `MatchEnv` episode end).
When a profile includes this component, `MatchEnv` defers its hardcoded
match-tier magnitudes (`MATCH_TERMINAL_WIN`, `MATCH_SWEEP_BONUS`,
`MATCH_SCARED_CONCEDE_PENALTY`, etc.) AND its per-game
`(bo3_game_terminal − inner_game_terminal)` adjustment — the profile
owns all reward magnitudes. When no `match_outcome` is present,
`MatchEnv` keeps its legacy calibration (back-compat).

Parameters:

- `win`: scalar emitted on a match win (agent gets ≥2 game wins).
- `loss`: scalar emitted on a match loss (opp gets ≥2 game wins).
- `draw`: scalar emitted on a 1-1-1 draw (rare; only via step-limit
  truncation).
- `sweep_bonus`: extra scalar added to `win` when opp got 0 games.
  Mirrors `MatchEnv.MATCH_SWEEP_BONUS`. Default 0.
- `swept_penalty`: extra scalar added to `loss` when agent got 0 games.
  Negative value (penalty). Default 0.
- `smart_concede_bonus`: extra scalar added to `win` when the agent
  conceded any game in the match. Rewards strategic concede that
  resulted in a comeback. Default 0.
- `scared_concede_penalty`: extra scalar added to `loss` when the agent
  conceded any game AND was swept (0-2 loss with concede). Negative.
  Default 0.
- `fast_bonus_max`: maximum bonus added linearly in
  `(par − match_step_count) / par` for match wins. Caps when
  `match_step_count` reaches 0 (instant match win, theoretical).
  Default 0.
- `fast_bonus_par_steps`: outer-agent-step count at which `fast_bonus`
  decays to 0. Default 150.

The shipped `_pure_outcome` profile uses `win=1.0, loss=-1.0,
draw=0.0` and zeros all bonuses for a clean ±1 match-tier signal. The
`MatchEnv` legacy calibration (which existing profiles inherit by NOT
including this component) corresponds approximately to `win=30.0,
loss=-30.0, draw=-1.0, sweep_bonus=10.0, smart_concede_bonus=5.0,
scared_concede_penalty=-10.0, fast_bonus_max=15.0,
fast_bonus_par_steps=450`.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, MutableMapping, Sequence

from ..occurrences import MatchOutcome


@dataclass
class MatchOutcomeComponent:
    """Per-match terminal reward with optional bonuses for sweep,
    smart/scared concede, and fast-win.
    """

    name: str
    win: float = 1.0
    loss: float = -1.0
    draw: float = 0.0
    sweep_bonus: float = 0.0
    swept_penalty: float = 0.0
    smart_concede_bonus: float = 0.0
    scared_concede_penalty: float = 0.0
    fast_bonus_max: float = 0.0
    fast_bonus_par_steps: int = 150

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        for ev in occurrences:
            if not isinstance(ev, MatchOutcome):
                continue
            wid = ev.winner_id
            if wid == 1:
                base = float(self.win)
                if ev.was_sweep:
                    base += float(self.sweep_bonus)
                if ev.agent_conceded_any:
                    base += float(self.smart_concede_bonus)
                if self.fast_bonus_max > 0.0 and self.fast_bonus_par_steps > 0:
                    par = float(self.fast_bonus_par_steps)
                    slack = max(0.0, par - float(ev.match_step_count))
                    base += (slack / par) * float(self.fast_bonus_max)
                return base
            if wid == 2:
                base = float(self.loss)
                if ev.was_swept:
                    base += float(self.swept_penalty)
                if ev.agent_conceded_any and ev.agent_game_wins == 0:
                    base += float(self.scared_concede_penalty)
                return base
            # winner_id is None → draw
            return float(self.draw)
        return 0.0

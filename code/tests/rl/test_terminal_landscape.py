"""Terminal-landscape regression test.

Hand-computed table of (turn, outcome) → expected terminal scalar under
the default gameplay shape. Loads the shipped `gameplay.yaml` via the
two-file ProfileLoader and exercises the assembled terminal scalar at
representative cases.

Spec table — see design.md §D5 ("Total terminal under default gameplay
shape (win + stall)"):

  turn   bonus    stall     win total    loss total   draw total
  ────────────────────────────────────────────────────────────────
    3   +5.00    0           +15.00       −10.00       −1.00
    7    0       0           +10.00       −10.00       −1.00
   10    0     −0.9            +9.10      −10.90       −1.90
   15    0     −6.4            +3.60      −16.40       −7.40
   20    0    −16.9            −6.90      −26.90      −17.90
   30    0    −52.9           −42.90      −62.90      −53.90

The test computes the SUM of every TerminalOutcome-driven component in
the gameplay profile (`terminal_outcome` + `quick_win_bonus` +
`stall_penalty`). Dense components (security_remove, digivolve, etc.)
are exercised by their own tests; this one isolates the terminal
landscape.
"""

from __future__ import annotations

import pytest

from digimon_gym.agents.reward.components.quick_win import QuickWinBonusComponent
from digimon_gym.agents.reward.components.stall import StallPenaltyComponent
from digimon_gym.agents.reward.components.terminal import TerminalOutcomeComponent
from digimon_gym.agents.reward.occurrences import TerminalOutcome
from digimon_gym.agents.reward.profile_loader import ProfileLoader


# Hand-computed table — see module docstring.
TERMINAL_LANDSCAPE = [
    # (turn, winner_id, expected_total)
    (3, 1, 15.0),       # peak win
    (3, 2, -10.0),      # turn-3 loss (no stall, no win bonus)
    (3, None, -1.0),    # turn-3 draw (no stall — within threshold)
    (7, 1, 10.0),       # clean seam
    (7, 2, -10.0),
    (7, None, -1.0),
    (10, 1, 9.1),       # 10 + 0 − 0.9 stall
    (10, 2, -10.9),     # −10 − 0.9 stall
    (10, None, -1.9),   # −1 − 0.9 stall
    (15, 1, 3.6),       # 10 + 0 − 6.4
    (15, 2, -16.4),
    (15, None, -7.4),
    (20, 1, -6.9),      # 10 + 0 − 16.9 — win is now NET-NEGATIVE
    (20, 2, -26.9),
    (20, None, -17.9),
    (30, 1, -42.9),     # win still less bad than loss…
    (30, 2, -62.9),     # …but both are catastrophic
    (30, None, -53.9),  # draw is worst at high turn (no win base)
]


@pytest.fixture(scope="module")
def gameplay_terminal_components():
    """Load gameplay.yaml's terminal-driven components from the SHIPPED
    file. If the file's parameters drift (e.g., operator tunes
    peak_value), this fixture picks the new values up automatically —
    the test landscape table is hand-computed against the current
    DESIGN defaults; mismatched values fail loudly with the operator
    knowing to either update the table or revert the tuning."""
    loader = ProfileLoader(
        gameplay_path="code/digimon_gym/agents/reward/gameplay.yaml",
        profiles_path="code/digimon_gym/agents/reward/profiles.yaml",
    )
    profile = loader.profiles.by_name["gameplay"]
    by_kind = {c.kind: c.component for c in profile.components}

    # Spot-check the three terminal-driven components exist and have
    # the values the landscape table expects.
    terminal = by_kind["terminal_outcome"]
    qwb = by_kind["quick_win_bonus"]
    stall = by_kind["stall_penalty"]

    assert isinstance(terminal, TerminalOutcomeComponent)
    assert isinstance(qwb, QuickWinBonusComponent)
    assert isinstance(stall, StallPenaltyComponent)

    # Pin design defaults — drift here means the landscape table needs
    # an update (or the gameplay.yaml edit should be reverted).
    assert terminal.win_base == 10.0
    assert terminal.loss == -10.0
    assert terminal.draw == -1.0
    assert terminal.fast_win_bonus_max == 0.0  # disabled — quick_win_bonus replaces
    assert qwb.peak_turn == 3
    assert qwb.peak_value == 5.0
    assert qwb.decay_per_turn == 1.25
    assert stall.threshold_turn == 7
    assert stall.scale == 0.1
    assert stall.apply_to_winner is True
    assert stall.apply_to_loser is True

    return {"terminal": terminal, "qwb": qwb, "stall": stall}


@pytest.mark.parametrize("turn,winner_id,expected", TERMINAL_LANDSCAPE)
def test_terminal_landscape_matches_design(
    gameplay_terminal_components, turn: int, winner_id, expected: float,
):
    """Each (turn, outcome) point in the design.md §D5 table sums to
    the hand-computed terminal scalar."""
    occ = [TerminalOutcome(
        winner_id=winner_id,
        step_count=200,   # large enough that legacy fast-win bonus is zero
        reason=None,
        turn_count=turn,
    )]
    comps = gameplay_terminal_components
    total = (
        comps["terminal"].compute(occ, {})
        + comps["qwb"].compute(occ, {})
        + comps["stall"].compute(occ, {})
    )
    assert total == pytest.approx(expected, abs=1e-9), (
        f"(turn={turn}, winner={winner_id}): want {expected}, got {total}"
    )

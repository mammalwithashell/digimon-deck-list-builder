"""Pure-Python tests for the 12 reward components in
`digimon_gym.agents.reward.components`. Each scenario maps to a
specific entry in `openspec/changes/add-reward-profiles/specs/
reward-profiles/spec.md` (v1 component catalog).

These tests are fully synthetic: they construct `Occurrence` instances
by hand and call `compute(...)` directly. No engine, no PyO3, no env.
"""

from __future__ import annotations

from typing import List

import pytest

from digimon_gym.agents.reward.components.combat import (
    BlockEventComponent,
    OppDeletionComponent,
    OwnDeletionComponent,
)
from digimon_gym.agents.reward.components.digivolve import (
    DigivolveComponent,
    DnaDigivolveComponent,
)
from digimon_gym.agents.reward.components.digivolve_into import (
    DigivolveIntoNamedCardComponent,
)
from digimon_gym.agents.reward.components.memory import MemorySwingComponent
from digimon_gym.agents.reward.components.play import PlayNamedCardComponent
from digimon_gym.agents.reward.components.security import (
    SecurityLostComponent,
    SecurityRemoveComponent,
)
from digimon_gym.agents.reward.components.terminal import (
    StepPenaltyComponent,
    TerminalOutcomeComponent,
)
from digimon_gym.agents.reward.occurrences import (
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


# ---- terminal_outcome ----------------------------------------------------


def test_terminal_outcome_fast_win_bonus_linear_in_remaining_steps():
    # Spec scenario: win_base=10.0, bonus_max=5.0, par=200, win at 150
    # → 10.0 + (200-150)/200 × 5.0 = 11.25.
    c = TerminalOutcomeComponent(
        name="terminal_outcome",
        win_base=10.0,
        fast_win_bonus_max=5.0,
        fast_win_par_steps=200,
        loss=-10.0,
        draw=-1.0,
    )
    state: dict = {}
    out = c.compute([TerminalOutcome(winner_id=1, step_count=150, reason="security_attack")], state)
    assert out == pytest.approx(11.25)


def test_terminal_outcome_loss_returns_configured_loss():
    c = TerminalOutcomeComponent(
        name="t", win_base=10.0, fast_win_bonus_max=5.0,
        fast_win_par_steps=200, loss=-10.0, draw=-1.0,
    )
    out = c.compute([TerminalOutcome(winner_id=2, step_count=50, reason="x")], {})
    assert out == -10.0


def test_terminal_outcome_draw_returns_configured_draw():
    c = TerminalOutcomeComponent(
        name="t", win_base=10.0, fast_win_bonus_max=5.0,
        fast_win_par_steps=200, loss=-10.0, draw=-1.0,
    )
    out = c.compute([TerminalOutcome(winner_id=None, step_count=300, reason="step_limit")], {})
    assert out == -1.0


def test_terminal_outcome_emits_zero_when_no_terminal_in_occurrences():
    c = TerminalOutcomeComponent(
        name="t", win_base=10.0, fast_win_bonus_max=5.0,
        fast_win_par_steps=200, loss=-10.0, draw=-1.0,
    )
    assert c.compute([StepElapsed()], {}) == 0.0


def test_terminal_outcome_slow_win_caps_bonus_at_zero():
    # Win at step 300 (past par_steps=200) → bonus clamped to 0.
    c = TerminalOutcomeComponent(
        name="t", win_base=10.0, fast_win_bonus_max=5.0,
        fast_win_par_steps=200, loss=-10.0, draw=-1.0,
    )
    out = c.compute([TerminalOutcome(winner_id=1, step_count=300, reason="x")], {})
    assert out == 10.0


# ---- step_penalty --------------------------------------------------------


def test_step_penalty_fires_once_per_step():
    c = StepPenaltyComponent(name="step_penalty", weight=-0.001)
    out = c.compute([StepElapsed()], {})
    assert out == -0.001


def test_step_penalty_zero_without_step_elapsed():
    c = StepPenaltyComponent(name="step_penalty", weight=-0.001)
    assert c.compute([], {}) == 0.0


# ---- security_remove / security_lost -------------------------------------


def test_security_remove_fires_only_on_step_security_count_drops():
    # Spec scenario: agent removes 2 sec on step t, 0 on step t+1.
    c = SecurityRemoveComponent(name="security_remove", weight=1.5)
    assert c.compute([SecurityRemoved(count=2)], {}) == pytest.approx(3.0)
    assert c.compute([StepElapsed()], {}) == 0.0


def test_security_lost_negative_weight_fires_per_lost_card():
    c = SecurityLostComponent(name="security_lost", weight=-0.5)
    assert c.compute([SecurityLost(count=3)], {}) == pytest.approx(-1.5)


# ---- digivolve / dna_digivolve -------------------------------------------


def test_digivolve_counts_agent_side_only():
    c = DigivolveComponent(name="digivolve", weight=0.5)
    occ: List = [
        Digivolved(player=1, top_card_id="X", field_index=0, from_stack_top="A",
                   was_dna=False, was_blast_dna=False, memory_paid=3,
                   result_level=5, result_traits=[]),
        Digivolved(player=2, top_card_id="Y", field_index=0, from_stack_top="B",
                   was_dna=False, was_blast_dna=False, memory_paid=3,
                   result_level=5, result_traits=[]),
    ]
    assert c.compute(occ, {}) == 0.5  # only p1 counts


def test_dna_digivolve_independent_of_regular_digivolve():
    c = DnaDigivolveComponent(name="dna_digivolve", weight=3.9)
    out = c.compute([DnaDigivolved(player=1), DnaDigivolved(player=1)], {})
    assert out == pytest.approx(7.8)


# ---- play_named_card -----------------------------------------------------


def _played(card_id: str, *, cost_paid: int, cost_printed: int,
            via_alt_path=None, player: int = 1) -> PlayedCard:
    return PlayedCard(
        player=player, card_id=card_id, field_index=0,
        cost_paid=cost_paid, cost_printed=cost_printed,
        via_alt_path=via_alt_path,
    )


def test_play_named_card_fires_on_card_id_match():
    c = PlayNamedCardComponent(
        name="omn_play", weight=3.0,
        match_card_id=["BT17-078"],
    )
    out = c.compute([_played("BT17-078", cost_paid=9, cost_printed=9)], {})
    assert out == 3.0


def test_play_named_card_does_not_fire_on_unmatched_card_id():
    c = PlayNamedCardComponent(name="omn", weight=3.0, match_card_id=["BT17-078"])
    out = c.compute([_played("BT17-077", cost_paid=9, cost_printed=9)], {})
    assert out == 0.0


def test_play_named_card_with_cost_paid_lt_printed_fires_on_reduced_cost():
    # Spec scenario: cost_paid_lt_printed=true on BT17-078 fires on
    # Blast DNA at cost 0 (printed 9), not on full-cost hardcast.
    c = PlayNamedCardComponent(
        name="omn_arrival", weight=4.0,
        match_card_id=["BT17-078"],
        cost_paid_lt_printed=True,
    )
    # Reduced-cost arrival (Blast DNA at 0).
    assert c.compute([_played("BT17-078", cost_paid=0, cost_printed=9)], {}) == 4.0
    # Full-cost hardcast.
    assert c.compute([_played("BT17-078", cost_paid=9, cost_printed=9)], {}) == 0.0


def test_play_named_card_with_cost_paid_gte_printed_penalty_fires_on_hardcast():
    # Hardcast penalty: cost_paid_gte_printed=true, weight=-1.5.
    c = PlayNamedCardComponent(
        name="omn_hardcast_penalty", weight=-1.5,
        match_card_id=["BT17-078"],
        cost_paid_gte_printed=True,
    )
    assert c.compute([_played("BT17-078", cost_paid=9, cost_printed=9)], {}) == -1.5
    # Reduced cost — does NOT fire (cost_paid < cost_printed).
    assert c.compute([_played("BT17-078", cost_paid=0, cost_printed=9)], {}) == 0.0


def test_play_named_card_with_via_alt_path_matches_string_list():
    c = PlayNamedCardComponent(
        name="omn_alt", weight=2.0,
        match_card_id=["AD1-025"],
        via_alt_path=["assembly", "blast_dna_digivolve"],
    )
    # Matches "assembly".
    out = c.compute(
        [_played("AD1-025", cost_paid=0, cost_printed=15, via_alt_path="assembly")], {},
    )
    assert out == 2.0
    # Unmatched alt-path.
    out = c.compute(
        [_played("AD1-025", cost_paid=0, cost_printed=15, via_alt_path="dna_digivolve")], {},
    )
    assert out == 0.0
    # No alt-path → matcher set, so never matches.
    out = c.compute(
        [_played("AD1-025", cost_paid=0, cost_printed=15, via_alt_path=None)], {},
    )
    assert out == 0.0


def test_play_named_card_ignores_opponent_side_plays():
    c = PlayNamedCardComponent(name="x", weight=3.0, match_card_id=["BT17-078"])
    out = c.compute([_played("BT17-078", cost_paid=9, cost_printed=9, player=2)], {})
    assert out == 0.0


def test_play_named_card_accumulates_multiple_matches_in_one_step():
    c = PlayNamedCardComponent(name="x", weight=1.0, match_card_id=["A", "B"])
    occ = [
        _played("A", cost_paid=3, cost_printed=3),
        _played("B", cost_paid=2, cost_printed=2),
        _played("C", cost_paid=1, cost_printed=1),
    ]
    assert c.compute(occ, {}) == 2.0


# ---- digivolve_into_named_card -------------------------------------------


def _digi(top: str, *, was_dna: bool = False, was_blast_dna: bool = False,
          result_level=None, result_traits=(), player: int = 1) -> Digivolved:
    return Digivolved(
        player=player, top_card_id=top, field_index=0, from_stack_top="X",
        was_dna=was_dna, was_blast_dna=was_blast_dna, memory_paid=0,
        result_level=result_level, result_traits=list(result_traits),
    )


def test_digivolve_into_named_card_with_was_dna_gating():
    # Spec scenario: was_dna=true gates to DNA-only.
    c = DigivolveIntoNamedCardComponent(
        name="dna_omn", weight=4.0,
        match_card_id=["BT17-078"],
        was_dna=True,
    )
    # DNA arrival fires.
    assert c.compute([_digi("BT17-078", was_dna=True)], {}) == 4.0
    # Non-DNA arrival does NOT fire.
    assert c.compute([_digi("BT17-078", was_dna=False)], {}) == 0.0


def test_digivolve_into_named_card_with_min_result_level():
    # Spec scenario: trait Royal Knight + min_result_level=6.
    c = DigivolveIntoNamedCardComponent(
        name="rk_lv6", weight=0.5,
        match_trait=["Royal Knight"],
        min_result_level=6,
    )
    # Lv5 doesn't fire.
    assert c.compute(
        [_digi("X", result_level=5, result_traits=["Royal Knight"])], {},
    ) == 0.0
    # Lv6 fires.
    assert c.compute(
        [_digi("X", result_level=6, result_traits=["Royal Knight"])], {},
    ) == 0.5


def test_digivolve_into_named_card_min_result_level_unknown_level_does_not_fire():
    # Defensive: if the bus didn't enrich result_level, the matcher
    # cannot decide → does not fire.
    c = DigivolveIntoNamedCardComponent(
        name="x", weight=1.0,
        match_card_id=["X"], min_result_level=6,
    )
    assert c.compute([_digi("X", result_level=None)], {}) == 0.0


def test_digivolve_into_named_card_trait_matching_any_of():
    c = DigivolveIntoNamedCardComponent(
        name="x", weight=1.0, match_trait=["Royal Knight", "Olympos XII"],
    )
    # Has one of the traits → matches.
    assert c.compute([_digi("X", result_traits=["Olympos XII", "Holy"])], {}) == 1.0
    # Has neither → doesn't match.
    assert c.compute([_digi("X", result_traits=["Dragon"])], {}) == 0.0


# ---- memory_swing --------------------------------------------------------


def test_memory_swing_emits_weight_times_signed_delta():
    c = MemorySwingComponent(name="memory_swing", weight=0.1)
    # Two MemoryShifted in one step: +3 and -1 → net +2.
    assert c.compute([MemoryShifted(delta=3), MemoryShifted(delta=-1)], {}) == pytest.approx(0.2)


# ---- block_event / opp_deletion / own_deletion ---------------------------


def test_block_event_counts_agent_side_blocks():
    c = BlockEventComponent(name="block", weight=0.15)
    occ = [
        Blocked(blocker_player=1, attacker_player=2),
        Blocked(blocker_player=2, attacker_player=1),
    ]
    assert c.compute(occ, {}) == 0.15  # only agent-side


def test_opp_deletion_counts_opponent_battle_area_trashes():
    c = OppDeletionComponent(name="opp_del", weight=1.0)
    occ = [
        OppDeleted(owner_player=2, card_id="X"),
        OwnDeleted(owner_player=1, card_id="Y"),  # not counted by opp_deletion
    ]
    assert c.compute(occ, {}) == 1.0


def test_own_deletion_negative_weight_fires_on_own_battle_area_trash():
    c = OwnDeletionComponent(name="own_del", weight=-0.5)
    assert c.compute([OwnDeleted(owner_player=1, card_id="X")], {}) == -0.5


# ---- determinism contract -----------------------------------------------


def test_components_are_deterministic_over_occurrences():
    # Spec scenario: same input → same output.
    c = SecurityRemoveComponent(name="x", weight=1.5)
    occ = [SecurityRemoved(count=2)]
    a = c.compute(occ, {})
    b = c.compute(occ, {})
    assert a == b

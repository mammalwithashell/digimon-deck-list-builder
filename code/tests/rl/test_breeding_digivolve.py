"""Pure-Python tests for the `breeding_digivolve` component.

Maps to scenarios in
`openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
`breeding_digivolve component`.
"""

from __future__ import annotations

import pytest

from digimon_gym.agents.reward.components.breeding import BreedingDigivolveComponent
from digimon_gym.agents.reward.occurrences import Digivolved


DEFAULT_TABLE = {3: 0.4, 4: 0.2, 5: 0.1, 6: -0.4}


def _comp(reward_per_level=None) -> BreedingDigivolveComponent:
    return BreedingDigivolveComponent(
        name="breeding_digivolve",
        reward_per_level=reward_per_level if reward_per_level is not None else dict(DEFAULT_TABLE),
    )


def _dig(
    *,
    player: int = 1,
    result_level: int = 4,
    is_breeding: bool = True,
) -> Digivolved:
    return Digivolved(
        player=player,
        top_card_id="X",
        field_index=14 if is_breeding else 0,
        from_stack_top="Y",
        was_dna=False,
        was_blast_dna=False,
        memory_paid=0,
        result_level=result_level,
        result_traits=[],
        is_breeding=is_breeding,
    )


def test_lv4_raise_emits_corresponding_reward():
    comp = _comp()
    assert comp.compute([_dig(result_level=4)], {}) == pytest.approx(0.2)


def test_lv6_in_breeding_fires_the_penalty():
    comp = _comp()
    assert comp.compute([_dig(result_level=6)], {}) == pytest.approx(-0.4)


def test_lv3_raise_is_highest_default_reward():
    comp = _comp()
    assert comp.compute([_dig(result_level=3)], {}) == pytest.approx(0.4)


def test_battle_area_digivolves_ignored():
    """Scenario: Battle-area digivolves ignored (is_breeding=false)."""
    comp = _comp()
    assert comp.compute([_dig(result_level=4, is_breeding=False)], {}) == 0.0


def test_result_level_not_in_dict_produces_zero():
    """Scenario: Result level not in dict produces zero."""
    comp = _comp(reward_per_level={3: 0.4})
    assert comp.compute([_dig(result_level=5)], {}) == 0.0


def test_opponent_breeding_digivolves_ignored():
    """Scenario: Opponent's breeding digivolves ignored (player=2)."""
    comp = _comp()
    assert comp.compute([_dig(player=2, result_level=4)], {}) == 0.0


def test_result_level_none_emits_zero():
    """Defensive: missing result_level (registry lookup failed) emits 0."""
    comp = _comp()
    ev = Digivolved(
        player=1, top_card_id="X", field_index=14, from_stack_top="Y",
        was_dna=False, was_blast_dna=False, memory_paid=0,
        result_level=None, result_traits=[], is_breeding=True,
    )
    assert comp.compute([ev], {}) == 0.0


def test_multiple_breeding_digivolves_sum():
    """Defensive: multiple breeding digivolves in one step accumulate."""
    comp = _comp()
    occ = [_dig(result_level=3), _dig(result_level=4)]
    assert comp.compute(occ, {}) == pytest.approx(0.6)  # 0.4 + 0.2


def test_no_digivolved_in_stream_returns_zero():
    comp = _comp()
    assert comp.compute([], {}) == 0.0

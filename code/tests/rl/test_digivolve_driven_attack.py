"""Pure-Python tests for the `digivolve_driven_attack` component.

Maps to scenarios in
`openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
`digivolve_driven_attack component`.
"""

from __future__ import annotations

import warnings

import pytest

from digimon_gym.agents.reward.components.digivolve_driven_attack import (
    DigivolveDrivenAttackComponent,
)
from digimon_gym.agents.reward.occurrences import DigivolveDrivenAttack


def _comp(**kw) -> DigivolveDrivenAttackComponent:
    return DigivolveDrivenAttackComponent(name="digivolve_driven_attack", **kw)


def _event(
    *,
    player: int = 1,
    attacker_level: int = 5,
    has_sources: bool = False,
    this_turn: bool = False,
) -> DigivolveDrivenAttack:
    return DigivolveDrivenAttack(
        player=player,
        attacker_level=attacker_level,
        has_sources=has_sources,
        this_turn=this_turn,
    )


# ── Mode predicate ──────────────────────────────────────────────────


def test_mode_either_fires_on_this_turn_alone():
    comp = _comp()  # mode=either by default
    assert comp.compute([_event(attacker_level=5, this_turn=True, has_sources=False)], {}) == pytest.approx(0.5)


def test_mode_either_fires_on_has_sources_alone():
    comp = _comp()
    assert comp.compute([_event(attacker_level=6, has_sources=True, this_turn=False)], {}) == pytest.approx(0.5)


def test_mode_either_does_not_fire_with_neither_flag():
    comp = _comp()
    assert comp.compute([_event(attacker_level=5, this_turn=False, has_sources=False)], {}) == 0.0


def test_mode_this_turn_does_not_fire_on_has_sources_alone():
    """Scenario: Mode this_turn does not fire on has_sources alone."""
    comp = _comp(mode="this_turn")
    assert comp.compute([_event(attacker_level=5, this_turn=False, has_sources=True)], {}) == 0.0


def test_mode_this_turn_fires_when_this_turn_flag_set():
    comp = _comp(mode="this_turn")
    assert comp.compute([_event(attacker_level=5, this_turn=True, has_sources=False)], {}) == pytest.approx(0.5)


def test_mode_has_sources_does_not_fire_on_this_turn_alone():
    comp = _comp(mode="has_sources")
    assert comp.compute([_event(attacker_level=5, this_turn=True, has_sources=False)], {}) == 0.0


def test_mode_has_sources_fires_when_has_sources_flag_set():
    comp = _comp(mode="has_sources")
    assert comp.compute([_event(attacker_level=5, has_sources=True)], {}) == pytest.approx(0.5)


def test_mode_both_requires_both_flags():
    comp = _comp(mode="both")
    # Only this_turn → no fire.
    assert comp.compute([_event(this_turn=True, has_sources=False)], {}) == 0.0
    # Only has_sources → no fire.
    assert comp.compute([_event(this_turn=False, has_sources=True)], {}) == 0.0
    # Both → fire.
    assert comp.compute([_event(this_turn=True, has_sources=True)], {}) == pytest.approx(0.5)


def test_invalid_mode_raises_at_construction():
    with pytest.raises(ValueError):
        DigivolveDrivenAttackComponent(name="dda", mode="bogus")


# ── Level filter ────────────────────────────────────────────────────


def test_attacker_min_level_filter_blocks_below():
    """Scenario: Below attacker_min_level does not fire."""
    comp = _comp(attacker_min_level=6)
    assert comp.compute(
        [_event(attacker_level=5, this_turn=True, has_sources=True)],
        {},
    ) == 0.0


def test_attacker_min_level_allows_equal():
    comp = _comp(attacker_min_level=5)
    assert comp.compute(
        [_event(attacker_level=5, this_turn=True)],
        {},
    ) == pytest.approx(0.5)


def test_attacker_min_level_allows_above():
    comp = _comp(attacker_min_level=5)
    assert comp.compute(
        [_event(attacker_level=7, has_sources=True)],
        {},
    ) == pytest.approx(0.5)


# ── per_card v1 deferral ────────────────────────────────────────────


def test_per_card_true_emits_load_warning():
    """Scenario: per_card=true emits load-time warning + behaves as per-attack."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        comp = DigivolveDrivenAttackComponent(
            name="dda", per_card=True,
        )
        assert any("per_card" in str(w.message) and "v2" in str(w.message) for w in caught), (
            f"Expected v2/per_card warning; got {[str(w.message) for w in caught]}"
        )

    # Behaves as per_card=false: emits per attack, not per security card.
    # Single attack → single emission.
    assert comp.compute([_event(this_turn=True)], {}) == pytest.approx(0.5)


# ── Player filter ───────────────────────────────────────────────────


def test_opponent_driven_attack_ignored():
    """Per-agent only — opponent's DigivolveDrivenAttack (player=2) is filtered."""
    comp = _comp()
    assert comp.compute(
        [_event(player=2, this_turn=True, has_sources=True)],
        {},
    ) == 0.0


# ── Sum across multiple events ──────────────────────────────────────


def test_multiple_qualifying_attacks_accumulate():
    comp = _comp()
    occ = [_event(this_turn=True), _event(has_sources=True)]
    assert comp.compute(occ, {}) == pytest.approx(1.0)

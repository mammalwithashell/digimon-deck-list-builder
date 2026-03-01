"""Tests for Game._effect_matches_timing() fix.

Validates three categories of effect timing behaviour:
1. Unflagged effects with no timing field  -> only fire at OnEnterFieldAnyone
2. Unflagged effects with explicit timing  -> fire only when timing matches
3. Flagged effects (is_on_play, etc.)      -> existing flag-based dispatch
"""

import os
import sys
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from digimon_gym.engine.game import Game
from digimon_gym.engine.data.enums import (
    GamePhase, CardKind, CardColor, EffectTiming,
)
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base
from digimon_gym.engine.interfaces.card_effect import ICardEffect


# ─── Mock Helpers ─────────────────────────────────────────────────────

def make_card(card_id="TEST-001", name="TestDigimon", kind=CardKind.Digimon,
              dp=5000, level=4, play_cost=5, colors=None, owner=None):
    """Create a CardSource with given attributes."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = kind
    entity.dp = dp
    entity.level = level
    entity.play_cost = play_cost
    entity.card_colors = colors or [CardColor.Red]
    cs = CardSource()
    cs.set_base_data(entity, owner)
    return cs


def make_effect(**overrides):
    """Create an ICardEffect with optional attribute overrides.

    By default produces a completely unflagged effect with timing=None.
    """
    eff = ICardEffect()
    eff.set_effect_name(overrides.pop("effect_name", "TestEffect"))
    for key, val in overrides.items():
        setattr(eff, key, val)
    return eff


def make_perm_with_card(card):
    """Create a Permanent whose top_card is *card*."""
    return Permanent([card])


# ─── Category 1: unflagged, no timing field ──────────────────────────

class TestUnflaggedNoTiming:
    """Effects with no timing flags and no effect.timing field."""

    def setup_method(self):
        self.card = make_card(card_id="UF-001", name="UnflaggedMon")
        self.perm = make_perm_with_card(self.card)
        # Completely vanilla effect: no flags, timing=None (default)
        self.effect = make_effect()

    def test_unflagged_no_timing_rejects_arbitrary_timings(self):
        """Unflagged + no timing -> False for arbitrary engine timings."""
        reject_timings = [
            EffectTiming.OnAddHand,
            EffectTiming.OnTappedAnyone,
            EffectTiming.OnLoseSecurity,
            EffectTiming.OnEndMainPhase,
            EffectTiming.OnAllyAttack,
        ]
        for timing in reject_timings:
            result = Game._effect_matches_timing(
                self.effect, timing, self.perm,
                played_card=self.card, digivolved_perm=None,
            )
            assert result is False, (
                f"Expected False for {timing.name} but got True"
            )

    def test_unflagged_no_timing_allows_on_enter_field(self):
        """Unflagged + no timing -> True for OnEnterFieldAnyone when
        played_card matches perm.top_card."""
        result = Game._effect_matches_timing(
            self.effect, EffectTiming.OnEnterFieldAnyone, self.perm,
            played_card=self.card, digivolved_perm=None,
        )
        assert result is True

    def test_unflagged_no_timing_rejects_on_enter_field_wrong_card(self):
        """Unflagged + no timing -> False for OnEnterFieldAnyone when
        played_card is a different card than perm.top_card."""
        other_card = make_card(card_id="OTHER-001", name="OtherMon")
        result = Game._effect_matches_timing(
            self.effect, EffectTiming.OnEnterFieldAnyone, self.perm,
            played_card=other_card, digivolved_perm=None,
        )
        assert result is False

    def test_unflagged_no_timing_rejects_on_enter_field_no_played_card(self):
        """Unflagged + no timing -> False for OnEnterFieldAnyone when
        played_card is None."""
        result = Game._effect_matches_timing(
            self.effect, EffectTiming.OnEnterFieldAnyone, self.perm,
            played_card=None, digivolved_perm=None,
        )
        assert result is False


# ─── Category 2: unflagged, explicit timing field ────────────────────

class TestUnflaggedWithTiming:
    """Effects with no timing flags but an explicit effect.timing value."""

    def test_unflagged_with_timing_matches_exact(self):
        """effect.timing == OnStartMainPhase -> True for OnStartMainPhase."""
        card = make_card()
        perm = make_perm_with_card(card)
        effect = make_effect(timing=EffectTiming.OnStartMainPhase)

        result = Game._effect_matches_timing(
            effect, EffectTiming.OnStartMainPhase, perm,
            played_card=None, digivolved_perm=None,
        )
        assert result is True

    def test_unflagged_with_timing_rejects_wrong(self):
        """effect.timing == OnStartMainPhase -> False for unrelated timings."""
        card = make_card()
        perm = make_perm_with_card(card)
        effect = make_effect(timing=EffectTiming.OnStartMainPhase)

        for timing in (EffectTiming.OnAddHand, EffectTiming.OnEndTurn):
            result = Game._effect_matches_timing(
                effect, timing, perm,
                played_card=None, digivolved_perm=None,
            )
            assert result is False, (
                f"Expected False for {timing.name} but got True"
            )

    def test_unflagged_with_timing_end_of_turn(self):
        """effect.timing == OnEndTurn -> True for OnEndTurn."""
        card = make_card()
        perm = make_perm_with_card(card)
        effect = make_effect(timing=EffectTiming.OnEndTurn)

        result = Game._effect_matches_timing(
            effect, EffectTiming.OnEndTurn, perm,
            played_card=None, digivolved_perm=None,
        )
        assert result is True


# ─── Category 3: flagged effects ─────────────────────────────────────

class TestFlaggedEffects:
    """Effects with timing flags (is_on_play, is_when_digivolving, etc.)."""

    def test_flagged_on_play_works(self):
        """is_on_play=True -> True at OnEnterFieldAnyone with card match."""
        card = make_card(card_id="PLAY-001", name="OnPlayMon")
        perm = make_perm_with_card(card)
        effect = make_effect(is_on_play=True)

        result = Game._effect_matches_timing(
            effect, EffectTiming.OnEnterFieldAnyone, perm,
            played_card=card, digivolved_perm=None,
        )
        assert result is True

    def test_flagged_on_play_rejects_other(self):
        """is_on_play=True -> False at unrelated timings."""
        card = make_card(card_id="PLAY-002", name="OnPlayMon2")
        perm = make_perm_with_card(card)
        effect = make_effect(is_on_play=True)

        for timing in (EffectTiming.OnAddHand, EffectTiming.OnStartMainPhase):
            result = Game._effect_matches_timing(
                effect, timing, perm,
                played_card=card, digivolved_perm=None,
            )
            assert result is False, (
                f"Expected False for {timing.name} with is_on_play but got True"
            )

    def test_flagged_when_digivolving_works(self):
        """is_when_digivolving=True -> True at WhenDigivolving with perm match."""
        card = make_card(card_id="DV-001", name="DigivolveMon")
        perm = make_perm_with_card(card)
        effect = make_effect(is_when_digivolving=True)

        result = Game._effect_matches_timing(
            effect, EffectTiming.WhenDigivolving, perm,
            played_card=None, digivolved_perm=perm,
        )
        assert result is True

    def test_flagged_when_digivolving_rejects_wrong_perm(self):
        """is_when_digivolving=True -> False when digivolved_perm is different."""
        card = make_card(card_id="DV-002", name="DigivolveMon2")
        perm = make_perm_with_card(card)
        other_card = make_card(card_id="DV-003", name="OtherMon")
        other_perm = make_perm_with_card(other_card)
        effect = make_effect(is_when_digivolving=True)

        result = Game._effect_matches_timing(
            effect, EffectTiming.WhenDigivolving, perm,
            played_card=None, digivolved_perm=other_perm,
        )
        assert result is False

    def test_flagged_on_attack_works(self):
        """is_on_attack=True -> True at OnAllyAttack."""
        card = make_card(card_id="ATK-001", name="AttackMon")
        perm = make_perm_with_card(card)
        effect = make_effect(is_on_attack=True)

        result = Game._effect_matches_timing(
            effect, EffectTiming.OnAllyAttack, perm,
            played_card=None, digivolved_perm=None,
        )
        assert result is True

    def test_flagged_on_deletion_works(self):
        """is_on_deletion=True -> True at OnDestroyedAnyone."""
        card = make_card(card_id="DEL-001", name="DeletionMon")
        perm = make_perm_with_card(card)
        effect = make_effect(is_on_deletion=True)

        result = Game._effect_matches_timing(
            effect, EffectTiming.OnDestroyedAnyone, perm,
            played_card=None, digivolved_perm=None,
        )
        assert result is True

    def test_flagged_on_deletion_rejects_unrelated(self):
        """is_on_deletion=True -> False at unrelated timings."""
        card = make_card(card_id="DEL-002", name="DeletionMon2")
        perm = make_perm_with_card(card)
        effect = make_effect(is_on_deletion=True)

        result = Game._effect_matches_timing(
            effect, EffectTiming.OnAddHand, perm,
            played_card=None, digivolved_perm=None,
        )
        assert result is False

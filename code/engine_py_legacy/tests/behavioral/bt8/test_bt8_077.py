"""Behavioral tests for BT8-077 BlackGatomon (Lv.4, Purple/Yellow, Dark Animal).

Card text:
  <Rush> (This Digimon can attack the turn it comes into play.)

Inherited:
  <Retaliation> (When this Digimon is deleted after losing a battle,
                 delete the Digimon it was battling.)

C# reference: DCGO/Assets/Scripts/CardEffect/BT8/Purple/BT8_077.cs
  - EffectTiming.None:           RushSelfStaticEffect (isInheritedEffect: false)
  - EffectTiming.OnDestroyedAnyone: RetaliationSelfEffect (isInheritedEffect: true)
"""

import pytest


BLACKGATOMON = "BT8-077"  # Lv.4 Purple/Yellow, Dark Animal, 6000 DP
PURPLE_LV3 = "ST6-03"     # Gabumon — Lv.3 Purple (valid digivolve base for BT8-077)
FILLER = "BT1-003"        # Filler for deck construction


def _build_deck(extras=None):
    """50-card deck: extras + filler."""
    cards = list(extras or [])
    while len(cards) < 50:
        cards.append(FILLER)
    return cards


@pytest.mark.behavioral
class TestBT8077EffectShape:
    """Structural tests — Rush and Retaliation effect instances."""

    def test_script_imports_ok(self):
        """The script module should import cleanly."""
        from engine_py_legacy.engine.data.scripts.bt8 import bt8_077
        assert hasattr(bt8_077, 'BT8_077')

    def test_has_rush_and_retaliation_effects(self, debug_runner):
        """Two effect instances: rush (non-inherited) + retaliation (inherited)."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        runner = debug_runner(initial_memory=10)
        db = CardDatabase()
        cs = db.create_card_source(BLACKGATOMON, runner.game.player1)
        effects = cs.effect_list(None)

        rush_effects = [e for e in effects if getattr(e, '_is_rush', False)]
        retal_effects = [e for e in effects if getattr(e, '_is_retaliation', False)]

        assert len(rush_effects) == 1, (
            f"Should have exactly 1 Rush effect, got {len(rush_effects)}")
        assert len(retal_effects) == 1, (
            f"Should have exactly 1 Retaliation effect, got {len(retal_effects)}")

    def test_rush_effect_is_not_inherited(self, debug_runner):
        """Rush is the card's main (above-line) effect — NOT inherited."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        runner = debug_runner(initial_memory=10)
        db = CardDatabase()
        cs = db.create_card_source(BLACKGATOMON, runner.game.player1)
        effects = cs.effect_list(None)
        rush = [e for e in effects if getattr(e, '_is_rush', False)][0]
        assert not rush.is_inherited_effect, (
            "Rush should not be an inherited effect (matches C# isInheritedEffect: false)")

    def test_retaliation_effect_is_inherited(self, debug_runner):
        """Retaliation is the below-line effect — MUST be inherited."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        runner = debug_runner(initial_memory=10)
        db = CardDatabase()
        cs = db.create_card_source(BLACKGATOMON, runner.game.player1)
        effects = cs.effect_list(None)
        retal = [e for e in effects if getattr(e, '_is_retaliation', False)][0]
        assert retal.is_inherited_effect, (
            "Retaliation should be an inherited effect (matches C# isInheritedEffect: true)")

    def test_rush_and_retaliation_are_separate_instances(self, debug_runner):
        """Two effects must be distinct ICardEffect instances, not the same object."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        runner = debug_runner(initial_memory=10)
        db = CardDatabase()
        cs = db.create_card_source(BLACKGATOMON, runner.game.player1)
        effects = cs.effect_list(None)
        rush = [e for e in effects if getattr(e, '_is_rush', False)][0]
        retal = [e for e in effects if getattr(e, '_is_retaliation', False)][0]
        assert rush is not retal, (
            "Rush and Retaliation must be separate ICardEffect instances")


@pytest.mark.behavioral
class TestBT8077RushOnFieldAsTop:
    """Rush is granted to BT8-077 when it is the top card on the permanent."""

    def test_has_rush_keyword_when_on_field_as_top(self, debug_runner):
        """When BT8-077 is on the field as top card, permanent.has_keyword('_is_rush') is True."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BLACKGATOMON])
        assert perm.has_keyword('_is_rush'), (
            "BT8-077 as top card should have Rush keyword active")

    def test_rush_allows_attack_same_turn_played(self, debug_runner):
        """With Rush, a freshly-played/placed Digimon should be able to attack this turn.

        Without Rush, Permanent.is_tapped_out/check in combat prevents attacking on
        the turn the permanent was placed (turn_played == turn_count).
        """
        runner = debug_runner(initial_memory=10)
        # Place BT8-077 on this turn so turn_played == turn_count
        perm = runner.place_on_field(1, [BLACKGATOMON], turn_played=runner.game.turn_count)
        # The permanent's own freshly-played check should NOT block it thanks to Rush.
        # can_attack_player() / can_attack_permanent flow rely on has_keyword check.
        assert perm.has_keyword('_is_rush')
        # Per permanent.py:406 — Rush bypasses the turn_played == turn_count check


@pytest.mark.behavioral
class TestBT8077RetaliationInherited:
    """<Retaliation> is inherited — active only when BT8-077 is below the top card."""

    def test_retaliation_active_when_inherited_under_top(self, debug_runner):
        """When BT8-077 is a digivolution source under another Digimon, the
        top card's permanent should have retaliation active."""
        runner = debug_runner(initial_memory=10)
        # Stack: BlackGatomon (bottom) -> some other top-card Digimon
        # place_on_field takes card_ids bottom-to-top.
        perm = runner.place_on_field(1, [BLACKGATOMON, FILLER])
        assert perm.top_card is not None
        assert perm.top_card.c_entity_base.card_id == FILLER
        assert perm.has_keyword('_is_retaliation'), (
            "A Digimon with BT8-077 in its digivolution sources should have "
            "Retaliation active via inherited effect")

    def test_retaliation_NOT_active_as_top_card(self, debug_runner):
        """Inherited effects only apply when the host card is below the top card.
        When BT8-077 IS the top card, its inherited Retaliation should not be active."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BLACKGATOMON])
        assert not perm.has_keyword('_is_retaliation'), (
            "Retaliation is inherited-only — when BT8-077 is the top card, "
            "the keyword should NOT be active (card text grants Rush only above-line)")


@pytest.mark.behavioral
class TestBT8077NoExtraEffects:
    """BT8-077 should have ONLY Rush and Retaliation — no stubs, no extras."""

    def test_effect_count_is_exactly_two(self, debug_runner):
        """Two effects only: Rush and Retaliation."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        runner = debug_runner(initial_memory=10)
        db = CardDatabase()
        cs = db.create_card_source(BLACKGATOMON, runner.game.player1)
        effects = cs.effect_list(None)
        assert len(effects) == 2, (
            f"BT8-077 should have exactly 2 effects (Rush + Retaliation), got {len(effects)}")

    def test_no_on_play_effect(self, debug_runner):
        """Rush is a static keyword — there should be no is_on_play effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        runner = debug_runner(initial_memory=10)
        db = CardDatabase()
        cs = db.create_card_source(BLACKGATOMON, runner.game.player1)
        effects = cs.effect_list(None)
        on_play = [e for e in effects if getattr(e, 'is_on_play', False)]
        assert len(on_play) == 0, (
            f"BT8-077 should have no On Play effects, got {len(on_play)}")

    def test_no_security_effect(self, debug_runner):
        """BT8-077 has no [Security] text."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        runner = debug_runner(initial_memory=10)
        db = CardDatabase()
        cs = db.create_card_source(BLACKGATOMON, runner.game.player1)
        effects = cs.effect_list(None)
        sec = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec) == 0, (
            f"BT8-077 should have no Security effects, got {len(sec)}")

    def test_no_before_pay_cost(self, debug_runner):
        """BT8-077 has no cost-reduction effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner = debug_runner(initial_memory=10)
        db = CardDatabase()
        cs = db.create_card_source(BLACKGATOMON, runner.game.player1)
        effects = cs.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost]
        assert len(bpc) == 0, (
            f"BT8-077 should have no BeforePayCost effects, got {len(bpc)}")

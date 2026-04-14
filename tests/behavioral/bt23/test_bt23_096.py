"""Behavioral tests for BT23-096 Comet Hammer (Option, Purple, Cost 5).

Card text (C# source of truth):
While you have a Digimon or Tamer with the [CS] trait on the field, you can
ignore this card's color requirements.
[Main] <De-Digivolve 4> 1 of your opponent's Digimon. Then, place this card
in the battle area.
[Your Turn] When one of your [CS] trait Digimon attacks, <Delay>
  - <De-Digivolve 4> 1 of your opponent's Digimon.
[Security] <De-Digivolve 4> 1 of your opponent's Digimon. Then, place this
card in the battle area.

Key cards used:
- BT23-096: Comet Hammer, Option, Purple, cost 5, CS trait
- BT22-017: Gabumon, Digimon, Lv.3, CS trait (for color bypass field check)
- BT22-083: Yuuko Kamishiro, Tamer, CS trait (for color bypass field check)
- BT22-010: Meramon, Digimon, Lv.4, CS trait (attacker for delay trigger)
- BT1-009: Monodramon, Digimon, Lv.3, no CS (should NOT trigger delay)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


HAMMER_DECK = ["BT23-096"] * 4 + ["BT22-017"] * 20 + ["BT22-010"] * 10 + ["BT1-009"] * 16


@pytest.mark.behavioral
class TestBT23096CometHammer:
    """Tests for BT23-096 Comet Hammer."""

    # -- Color bypass (conditional) --

    def test_color_bypass_with_cs_digimon_on_field(self, debug_runner):
        """Color requirement should be bypassed when a CS Digimon is on field."""
        runner = debug_runner(deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        game = runner.game

        # Place a CS Digimon on field
        runner.place_on_field(1, ["BT22-017"])  # Gabumon, CS

        runner.inject_card(1, "BT23-096", "hand")
        card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT23-096":
                card = c
                break
        assert card is not None
        assert not card.match_color_requirement, \
            "Color requirement should be bypassed (False) when CS Digimon on field"

    def test_color_bypass_with_cs_tamer_on_field(self, debug_runner):
        """Color requirement should be bypassed when a CS Tamer is on field."""
        runner = debug_runner(deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        game = runner.game

        # Place a CS Tamer on field
        runner.place_on_field(1, ["BT22-083"])  # Yuuko Kamishiro, CS tamer

        runner.inject_card(1, "BT23-096", "hand")
        card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT23-096":
                card = c
                break
        assert card is not None
        assert not card.match_color_requirement, \
            "Color requirement should be bypassed (False) when CS Tamer on field"

    def test_color_enforced_without_cs_on_field(self, debug_runner):
        """Color requirement should be enforced when no CS Digimon/Tamer on field."""
        runner = debug_runner(deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        game = runner.game

        # Place a non-CS Digimon on field
        runner.place_on_field(1, ["BT1-009"])  # Monodramon, no CS

        runner.inject_card(1, "BT23-096", "hand")
        card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT23-096":
                card = c
                break
        assert card is not None
        assert card.match_color_requirement, \
            "Color requirement should be enforced (True) without CS on field"

    def test_color_enforced_empty_field(self, debug_runner):
        """Color requirement should be enforced when field is empty."""
        runner = debug_runner(deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        game = runner.game

        runner.inject_card(1, "BT23-096", "hand")
        card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT23-096":
                card = c
                break
        assert card is not None
        assert card.match_color_requirement, \
            "Color requirement should be enforced (True) with empty field"

    # -- Main effect: De-Digivolve 4 --

    def test_main_de_digivolve_effect_exists(self, debug_runner):
        """Main OptionSkill effect should exist for De-Digivolve 4."""
        runner = debug_runner(deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-096", runner.game.player1)
        effects = card.effect_list(None)

        option_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_effects) == 1, "Should have 1 OptionSkill effect"

    # -- Delay: trigger on CS Digimon attack --

    def test_delay_trigger_timing_is_on_ally_attack(self, debug_runner):
        """Delay effect should use OnAllyAttack timing (not OnUseAttack),
        matching C# EffectTiming.OnAllyAttack for 'when one of your Digimon attacks'."""
        runner = debug_runner(deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT23-096"], turn_played=-2)
        card = perm.top_card
        effects = card.effect_list(None)

        # Find the delay activation effect (should be OnAllyAttack)
        ally_attack_effects = [e for e in effects if e.timing == EffectTiming.OnAllyAttack]
        assert len(ally_attack_effects) >= 1, \
            "Should have at least 1 OnAllyAttack effect for delay trigger"

        # Should NOT have OnUseAttack timing for the delay
        use_attack_delay = [e for e in effects if e.timing == EffectTiming.OnUseAttack
                           and getattr(e, '_is_delay', False)]
        assert len(use_attack_delay) == 0, \
            "Delay should NOT use OnUseAttack timing"

    def test_delay_condition_requires_cs_attacker(self, debug_runner):
        """Delay should only trigger when a CS-trait Digimon attacks."""
        runner = debug_runner(deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT23-096"], turn_played=-2)
        card = perm.top_card
        effects = card.effect_list(None)

        delay_effects = [e for e in effects if e.timing == EffectTiming.OnAllyAttack
                        and not getattr(e, '_is_raid', False)]
        assert len(delay_effects) >= 1

        delay = delay_effects[0]

        # Create a CS attacker
        cs_attacker = runner.place_on_field(1, ["BT22-010"])  # Meramon, CS Lv.4
        context_cs = {'attacking_permanent': cs_attacker, 'permanent': cs_attacker}
        assert delay.can_use_condition(context_cs), \
            "Delay condition should pass for CS attacker"

        # Create a non-CS attacker
        non_cs_attacker = runner.place_on_field(1, ["BT1-009"])  # Monodramon, no CS
        context_non_cs = {'attacking_permanent': non_cs_attacker, 'permanent': non_cs_attacker}
        assert not delay.can_use_condition(context_non_cs), \
            "Delay condition should fail for non-CS attacker"

    # -- Security effect --

    def test_security_de_digivolve_effect_exists(self, debug_runner):
        """Security effect should exist for De-Digivolve 4."""
        runner = debug_runner(deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-096", runner.game.player1)
        effects = card.effect_list(None)

        security_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(security_effects) == 1, "Should have 1 SecuritySkill effect"
        assert security_effects[0].is_security_effect, "Should be flagged as security effect"

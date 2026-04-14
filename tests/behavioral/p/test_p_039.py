"""Behavioral tests for P-039 Black Memory Boost! (Option, Black, Cost 3).

Card text:
  [Main] Reveal the top 4 cards of your deck. Add 1 black Digimon card among
  them to the hand. Place the remaining cards at the bottom of your deck in
  any order. Then, place this card in your battle area.

  [Main] <Delay> - Gain 2 memory.

  [Security] Place this card in the battle area.

C# reference: P_039.cs
  - Main: SimplifiedRevealDeckTopCardsAndSelect (4 cards, 1 pass: Black Digimon)
      + PlaceDelayOptionCards
  - Delay: OnDeclaration, CanDeclareOptionDelayEffect, memory +2
  - Security: CardEffectFactory.PlaceSelfDelayOptionSecurityEffect(card)

Key cards used:
  BT5-061: Commandramon (Black Lv3, DP 2000) - black Digimon (color requirement)
  ST1-03: Agumon (Red Lv3) - non-black Digimon (should not be selectable in reveal)
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming


_DECK = ["P-039"] * 4 + ["BT5-061"] * 4 + ["ST1-03"] * 42


@pytest.mark.behavioral
class TestP039Effects:
    """Verify P-039 has the correct effect structure."""

    def test_has_option_skill(self):
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-039")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_has_delay_marker(self):
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-039")
        effects = cs.effect_list(None)
        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) >= 1, "Should have a delay marker effect"

    def test_has_delay_on_declaration(self):
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-039")
        effects = cs.effect_list(None)
        on_decl = [e for e in effects if e.timing == EffectTiming.OnDeclaration]
        assert len(on_decl) >= 1, "Should have OnDeclaration timing for delay"

    def test_has_security_effect(self):
        """P-039 must have a SecuritySkill effect that places it in the battle area.
        C# ref: CardEffectFactory.PlaceSelfDelayOptionSecurityEffect(card)"""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-039")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, (
            "P-039 MUST have a SecuritySkill effect to place itself in the battle area"
        )


@pytest.mark.behavioral
class TestP039MainReveal:
    """[Main] Reveal top 4 cards, add 1 black Digimon to hand."""

    def test_main_places_card_in_battle_area(self, debug_runner):
        """Playing P-039 should place it in the battle area (Delay)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        # Need a black Digimon on field to meet option color requirement
        runner.place_on_field(1, ["BT5-061"])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-039", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Black Memory Boost")
        if action is None:
            action = runner.find_action("P-039")
        assert action is not None, f"Available: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == "P-039" for s in snap.p1_field)
        assert in_battle, "P-039 should be placed in the battle area after main effect"

    def test_reveal_selects_black_digimon(self, debug_runner):
        """Reveal effect should allow selecting a black Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5, skip_shuffle=True)
        game = runner.game
        p1 = game.player1

        # Need a black Digimon on field for color requirement
        runner.place_on_field(1, ["BT5-061"])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-039", "hand")

        # Set up library top 4: 1 black Digimon + 3 red (non-selectable)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        black_dig = db.create_card_source("BT5-061", p1)  # Black Digimon
        red_dig = db.create_card_source("ST1-03", p1)
        red_dig2 = db.create_card_source("ST1-03", p1)
        red_dig3 = db.create_card_source("ST1-03", p1)
        p1.library_cards.insert(0, red_dig3)
        p1.library_cards.insert(0, red_dig2)
        p1.library_cards.insert(0, red_dig)
        p1.library_cards.insert(0, black_dig)

        runner.set_phase("Main")

        action = runner.find_action("Black Memory Boost")
        if action is None:
            action = runner.find_action("P-039")
        assert action is not None, f"Available: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        has_commandramon_in_hand = "BT5-061" in snap.p1_hand
        assert has_commandramon_in_hand, (
            f"Should have added the black Digimon to hand. Hand: {snap.p1_hand}"
        )


@pytest.mark.behavioral
class TestP039DelayEffect:
    """[Main] <Delay> - Gain 2 memory."""

    def test_delay_gains_2_memory(self, debug_runner):
        """Activating delay should gain 2 memory."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["P-039"], turn_played=-1)
        runner.set_phase("Main")

        memory_before = runner.game.memory

        delay_action = runner.find_action("Delay")
        if delay_action is None:
            delay_action = runner.find_action("Memory +2")
        if delay_action is None:
            delay_action = runner.find_action("Black Memory Boost")
        assert delay_action is not None, f"Should have Delay action. Available: {runner.actions()}"

        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # P-039 should be trashed after delay activation
        assert not any(s.card_id == "P-039" for s in snap.p1_field), (
            "P-039 should be trashed after Delay"
        )
        # Memory should have increased by 2
        assert snap.memory == memory_before + 2, (
            f"Memory should increase by 2. Before: {memory_before}, After: {snap.memory}"
        )

    def test_delay_marker_is_delay(self):
        """Verify delay effect has _is_delay flag."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-039")
        effects = cs.effect_list(None)
        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) == 1, (
            "P-039 should have exactly 1 delay marker effect"
        )


@pytest.mark.behavioral
class TestP039SecurityEffect:
    """[Security] Place this card in the battle area."""

    def test_security_places_in_battle_area(self, debug_runner):
        """When P-039 is revealed from security, it should be placed in the battle area."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        # Clear security, inject P-039 as top security card
        runner.clear_zone(2, "security")
        runner.inject_card(2, "P-039", "security_top")

        from tests.helpers.game_builder import make_permanent
        attacker = make_permanent("ATK-001", "StrongAttacker", dp=12000)
        game.player1.battle_area.append(attacker)

        p2.security_attack(attacker)
        runner.auto_resolve()

        # P-039 should be in P2's battle area (not trash)
        p039_on_field = [p for p in p2.battle_area
                         if p.top_card and p.top_card.c_entity_base
                         and p.top_card.c_entity_base.card_id == "P-039"]
        assert len(p039_on_field) >= 1, (
            "P-039 should be placed in the battle area from security"
        )

        # P-039 should NOT be in trash
        p039_in_trash = [c for c in p2.trash_cards
                         if c.c_entity_base and c.c_entity_base.card_id == "P-039"]
        assert len(p039_in_trash) == 0, (
            "P-039 should NOT be in trash after security (it goes to battle area)"
        )

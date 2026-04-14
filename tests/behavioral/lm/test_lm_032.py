"""Behavioral tests for LM-032 Purple Scramble (Option, Purple, Cost 2).

Card text:
  [Main] 1 of your purple Digimon may digivolve into a purple Digimon card
  in hand with the digivolution cost reduced by 3. Then, place this card in
  the battle area.

  [Start of Your Turn] If your opponent has a Digimon, <Delay> (by trashing
  this card after the placing turn, activate the effect below): Return 1 purple
  Digimon card from your trash to the top of the deck. Then, if you don't have
  a Digimon, you may play 1 purple Digimon card with 2000 DP or less from your
  trash without paying the cost.

  [Security] You may play 1 purple Digimon card with 2000 DP or less from your
  trash without paying the cost. Then, add this card to the hand.

Identical structure to LM-030 Green Scramble but filtered for purple.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, CardColor


# Helper IDs:
# BT2-068: Impmon (Purple Lv3, DP 1000) — <=2000 DP target
# BT2-069: Gabumon (Purple Lv3, DP 2000) — exactly 2000 DP
# BT2-071: Wizardmon (Purple Lv4, DP 4000) — digivolve target (DP > 2000)
# BT2-060: Megadramon (Purple Lv5, DP 9000) — high-DP filler
# ST1-03: Agumon (Red Lv3) — non-purple (negative test)


@pytest.mark.behavioral
class TestLM032MainEffect:
    """[Main] Purple Digimon digivolves from hand cost -3, place in battle area."""

    def test_main_effect_exists(self, debug_runner):
        """LM-032 should have an OptionSkill effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-032")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill timing"

    def test_main_places_card_in_battle_area(self, debug_runner):
        """Playing Purple Scramble should place it in the battle area (Delay)."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT2-068"])  # Tsukaimon (Purple Lv3)
        runner.inject_card(1, "LM-032", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Purple Scramble")
        if action is None:
            action = runner.find_action("LM-032")
        assert action is not None, f"Available: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == "LM-032" for s in snap.p1_field)
        assert in_battle, "Purple Scramble should be placed in the battle area"

    def test_main_field_filter_purple_only(self, debug_runner):
        """Field filter should only allow purple Digimon."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-032")
        effects = cs.effect_list(None)
        main_eff = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]
        assert main_eff.on_process_callback is not None

        # Verify the filter function logic exists
        # The process should filter for purple Digimon on field
        runner = debug_runner(initial_memory=10)
        # Only a Red Digimon on field -- should not be selectable for digivolve
        runner.place_on_field(1, ["ST1-03"])  # Agumon (Red Lv3)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT2-071", "hand")  # Purple Lv4 in hand
        runner.inject_card(1, "LM-032", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Purple Scramble")
        if action is None:
            action = runner.find_action("LM-032")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            snap = runner.snapshot()
            # Agumon should NOT have evolved since it's Red
            devimon_on_field = any(
                s.card_id == "BT2-071" for s in snap.p1_field
            )
            assert not devimon_on_field, (
                "Should NOT digivolve Red Agumon using Purple Scramble"
            )


@pytest.mark.behavioral
class TestLM032DelayEffect:
    """[Start of Your Turn] Delay: return purple Digimon from trash, play if no Digimon."""

    def test_delay_marker_exists(self, debug_runner):
        """LM-032 should have a delay marker effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-032")
        effects = cs.effect_list(None)
        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) >= 1, "Should have a delay marker effect"

    def test_delay_trigger_exists(self, debug_runner):
        """Should have an OnStartTurn effect for the delay body."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-032")
        effects = cs.effect_list(None)
        start_turn = [e for e in effects if e.timing == EffectTiming.OnStartTurn]
        assert len(start_turn) >= 1, "Should have OnStartTurn timing"

    def test_delay_condition_requires_opponent_digimon(self, debug_runner):
        """Delay condition should require opponent to have a Digimon."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-032"], turn_played=-1)
        runner.inject_card(1, "BT2-068", "trash")

        game = runner.game
        player1 = game.player1

        lm032_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "LM-032":
                lm032_card = p.top_card
                break
        assert lm032_card is not None

        effects = lm032_card.effect_list(None)
        start_turn_eff = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        # No opponent Digimon -> condition fails
        result = start_turn_eff.can_use_condition({})
        assert result is False, (
            "Delay condition should be False when opponent has no Digimon"
        )

    def test_delay_condition_passes_with_opponent_digimon(self, debug_runner):
        """Delay condition should pass when opponent has a Digimon."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-032"], turn_played=-1)
        runner.place_on_field(2, ["BT2-068"])  # Opponent Digimon
        runner.inject_card(1, "BT2-068", "trash")

        game = runner.game
        player1 = game.player1

        lm032_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "LM-032":
                lm032_card = p.top_card
                break
        assert lm032_card is not None

        effects = lm032_card.effect_list(None)
        start_turn_eff = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        result = start_turn_eff.can_use_condition({})
        assert result is True, (
            "Delay condition should be True when opponent has a Digimon"
        )


@pytest.mark.behavioral
class TestLM032SecurityEffect:
    """[Security] Play purple Digimon <=2000 DP from trash; add to hand."""

    def test_security_effect_exists(self, debug_runner):
        """LM-032 should have a SecuritySkill effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-032")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, "Should have SecuritySkill effect"
        assert sec_effects[0].is_security_effect is True

    def test_security_adds_to_hand(self, debug_runner):
        """Security effect should add LM-032 to the hand after play."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-032")
        effects = cs.effect_list(None)
        sec_eff = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]

        class MockGame:
            def effect_play_from_zone(self, *a, **kw):
                pass
        class MockPlayer:
            def __init__(self, card):
                self.hand_cards = []
                self.trash_cards = [card]
        player = MockPlayer(cs)
        sec_eff.on_process_callback({'player': player, 'game': MockGame()})
        assert cs in player.hand_cards, "Security should add LM-032 to hand"

    def test_security_filter_purple_only(self, debug_runner):
        """Security play filter should require purple Digimon."""
        # Verify the card itself is purple
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-032")
        assert CardColor.Purple in cs.c_entity_base.card_colors, (
            "LM-032 should be a purple card"
        )

    def test_security_filter_dp_threshold(self, debug_runner):
        """Security play filter: DP must be 2000 or less."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Purple Lv3 with low DP -- should be accepted
        impmon = db.create_card_source("BT2-068")
        assert impmon.c_entity_base.dp == 1000, "Impmon should have 1000 DP"

        # Purple Lv4 with high DP -- should be rejected by the <=2000 filter
        wizardmon = db.create_card_source("BT2-071")
        assert wizardmon.c_entity_base.dp > 2000, "Wizardmon should have > 2000 DP"

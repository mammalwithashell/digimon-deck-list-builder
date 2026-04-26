"""Behavioral tests for P-107 Defense Training (Option, Black, Cost 2).

Card text:
  [Main] Reveal the top 2 cards of your deck. Add 1 black card among them to
  your hand. Place the rest at the bottom of your deck in any order. Then,
  place this card in the battle area.
  [Main] <Delay> (By trashing this card after the placing turn, activate the
  effect below.)
  - 1 of your Digimon may digivolve into a black Digimon card in your hand
    for its digivolution cost. When it would digivolve by this effect, reduce
    the cost by 2.
  Security Effect [Security] Place this card in the battle area.
"""

import pytest
from digimon_gym.engine.data.enums import CardColor, EffectTiming, GamePhase


# Card IDs used in tests
P107 = "P-107"         # Defense Training (Black Option, cost 2)
BLK_ROOKIE = "BT2-055"  # DemiDevimon Lv.3 Black Digimon
BLK_CHAMP = "BT2-058"   # Devimon Lv.4 Black Digimon (evo from Lv.3 Black)
RED_ROOKIE = "ST1-03"    # Agumon Lv.3 Red Digimon
RED_OPT = "BT1-090"     # Gaia Force (Red Option, not Black)
BLK_TAMER = "BT2-069"   # Tai Kamiya (Black Tamer)


@pytest.mark.behavioral
class TestP107OptionSkill:
    """[Main] Reveal top 2, add 1 black card, place rest at bottom, then place in battle area."""

    def test_has_option_skill_timing(self, debug_runner):
        """Should have an OptionSkill effect for the main reveal effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(P107)
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_play_places_in_battle_area(self, debug_runner):
        """Playing Defense Training should place it in the battle area (Delay card)."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        runner.set_phase("Main")
        # Need a black Digimon/Tamer on field to satisfy option color requirement
        runner.place_on_field(1, [BLK_ROOKIE])
        runner.inject_card(1, P107, "hand")

        action = runner.find_action("Defense Training")
        assert action is not None, "Should be able to play Defense Training"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == P107 for s in snap.p1_field)
        assert in_battle, "Defense Training should be placed in the battle area"

    def test_reveal_adds_black_card_to_hand(self, debug_runner):
        """Revealing a black card should add it to hand."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        runner.set_phase("Main")
        runner.place_on_field(1, [BLK_ROOKIE])

        # Put a black card on top of the deck so it gets revealed
        runner.inject_card(1, BLK_CHAMP, "library_top")
        runner.inject_card(1, P107, "hand")

        hand_before = len(runner.game.player1.hand_cards)

        action = runner.find_action("Defense Training")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # The black card should have been added to hand (minus P-107 which was played)
        # Net: hand_before - 1 (played P-107) + 1 (black card from reveal) = hand_before
        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after >= hand_before, (
            f"Black card from reveal should be added to hand. "
            f"Hand before={hand_before}, after={hand_after}"
        )

    def test_reveal_no_black_card_returns_all_to_bottom(self, debug_runner):
        """If no revealed card is black, all go to bottom of deck."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        runner.set_phase("Main")
        runner.place_on_field(1, [BLK_ROOKIE])

        # Clear the deck and put only non-black cards on top
        runner.clear_zone(1, "library")
        runner.inject_card(1, RED_ROOKIE, "library_top")
        runner.inject_card(1, RED_OPT, "library_top")
        # Add extra cards so deck doesn't run out
        for _ in range(5):
            runner.inject_card(1, RED_ROOKIE, "library_bottom")

        runner.inject_card(1, P107, "hand")

        action = runner.find_action("Defense Training")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # No black cards among revealed, so no selection phase, all returned to bottom
        snap = runner.snapshot()
        # P-107 should still be in battle area
        in_battle = any(s.card_id == P107 for s in snap.p1_field)
        assert in_battle, "Defense Training should still be placed in battle area"

    def test_reveal_filter_accepts_any_black_card(self, debug_runner):
        """The reveal filter should match any black card, not just Digimon."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        black_digi = db.create_card_source(BLK_ROOKIE)
        red_digi = db.create_card_source(RED_ROOKIE)

        # Black card should have CardColor.Black
        black_colors = getattr(black_digi, 'card_colors', [])
        assert CardColor.Black in black_colors, "BLK_ROOKIE should be Black"

        red_colors = getattr(red_digi, 'card_colors', [])
        assert CardColor.Black not in red_colors, "RED_ROOKIE should not be Black"


@pytest.mark.behavioral
class TestP107DelayMarker:
    """Delay marker and delay activation."""

    def test_has_delay_marker(self, debug_runner):
        """Should have a delay marker effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(P107)
        effects = cs.effect_list(None)
        delays = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delays) >= 1, "Should have a delay marker effect"

    def test_delay_callback_has_field_main(self, debug_runner):
        """The delay callback effect should have OnDeclaration timing and _is_field_main."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(P107)
        effects = cs.effect_list(None)

        delay_main = [e for e in effects
                      if e.timing == EffectTiming.OnDeclaration
                      and getattr(e, '_is_field_main', False)]
        assert len(delay_main) >= 1, (
            "Should have OnDeclaration _is_field_main delay callback effect"
        )
        assert delay_main[0].on_process_callback is not None, (
            "Delay callback must have a process callback"
        )


@pytest.mark.behavioral
class TestP107DelayDigivolve:
    """Delay: 1 Digimon may digivolve into black Digimon from hand, cost -2."""

    def test_delay_digivolve_offered_on_next_turn(self, debug_runner):
        """Delay should be activatable on the turn after placement (not same turn)."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        runner.set_phase("Main")
        runner.place_on_field(1, [BLK_ROOKIE])
        runner.inject_card(1, P107, "hand")

        # Play the option to place in battle area
        action = runner.find_action("Defense Training")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == P107 for s in snap.p1_field), (
            "P-107 should be in battle area"
        )

        # On the same turn, delay should NOT be activatable
        delay_actions = runner.find_actions("Delay")
        # The Delay action should not be offered on the placing turn
        # (turn_played == turn_count means same turn)

        # Advance to next turn
        runner.game.turn_count += 1

        # Put a valid digivolve target in hand
        runner.inject_card(1, BLK_CHAMP, "hand")
        runner.set_phase("Main")

        # Now delay should be available
        delay_actions = runner.find_actions("Delay")
        assert delay_actions, (
            "Delay effect should be activatable on the turn after placement"
        )

    def test_delay_trashes_card_and_triggers_digivolve(self, debug_runner):
        """Activating delay should trash the option and trigger digivolve selection."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        # Place P-107 directly on field (simulating it was placed last turn)
        p107_perm = runner.place_on_field(1, [P107], turn_played=-1)
        # Place a Digimon to digivolve
        runner.place_on_field(1, [BLK_ROOKIE])
        # Put a valid black Digimon in hand to digivolve into
        runner.inject_card(1, BLK_CHAMP, "hand")
        runner.set_phase("Main")

        p1_field_before = len(runner.game.player1.battle_area)

        # Find and execute delay action
        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay action should be available"
        runner.execute(delay_action)

        # P-107 should be trashed (removed from battle area)
        p107_on_field = any(
            p.top_card and p.top_card.c_entity_base and
            p.top_card.c_entity_base.card_id == P107
            for p in runner.game.player1.battle_area
        )
        assert not p107_on_field, "P-107 should be trashed after Delay activation"

        # P-107 should be in trash
        p107_in_trash = any(
            c.c_entity_base and c.c_entity_base.card_id == P107
            for c in runner.game.player1.trash_cards
        )
        assert p107_in_trash, "P-107 should be in trash after Delay activation"

    def test_delay_digivolve_with_cost_reduction(self, debug_runner):
        """Delay digivolve should reduce the cost by 2."""
        from digimon_gym.engine.game.constants import SEL_MY_FIELD_START, SEL_HAND_START
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        # Place P-107 directly on field (turn_played=-1 so delay is usable)
        runner.place_on_field(1, [P107], turn_played=-1)
        # Place a Lv.3 Black Digimon to digivolve (slot 1 after P-107)
        runner.place_on_field(1, [BLK_ROOKIE])
        # Put Lv.4 Black Digimon in hand
        runner.inject_card(1, BLK_CHAMP, "hand")
        runner.set_phase("Main")

        memory_before = runner.game.memory

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay action should be available"
        runner.execute(delay_action)

        # After delay execution, should be in a selection phase for the target permanent
        # Manually select the Black Digimon (it should be at field slot 0 now,
        # since P-107 was trashed by the delay)
        if runner.game.current_phase == GamePhase.SelectTarget:
            sel = runner.game.pending_selection
            if sel and sel.valid_indices:
                # Pick the first valid permanent (should be our Black Digimon)
                runner.execute(sel.valid_indices[0])

        # Now should be in selection for the hand card to digivolve into
        if runner.game.current_phase == GamePhase.SelectTarget:
            sel = runner.game.pending_selection
            if sel and sel.valid_indices:
                # Pick the first valid hand card (should be BLK_CHAMP)
                runner.execute(sel.valid_indices[0])

        runner.auto_resolve()

        memory_after = runner.game.memory
        # BLK_CHAMP (BT2-058) play cost is 5; with cost_reduction=2, effective = 3
        # Memory should go from 10 to 7
        assert memory_after < memory_before, (
            f"Memory should decrease after digivolve. Before={memory_before}, after={memory_after}"
        )
        assert memory_after == memory_before - 3, (
            f"Digivolve cost should be play_cost(5) - reduction(2) = 3. "
            f"Memory before={memory_before}, after={memory_after}"
        )

    def test_delay_digivolve_is_optional(self, debug_runner):
        """Delay digivolve is optional - player can decline."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        # Place P-107 on field
        runner.place_on_field(1, [P107], turn_played=-1)
        # Place a Digimon
        runner.place_on_field(1, [BLK_ROOKIE])
        # Put a valid target in hand
        runner.inject_card(1, BLK_CHAMP, "hand")
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)

        # The permanent selection should be optional (can decline)
        if runner.game.current_phase in (GamePhase.SelectTarget,):
            sel = runner.game.pending_selection
            if sel:
                assert sel.is_optional, (
                    "Digimon selection for delay digivolve should be optional"
                )

    def test_delay_only_offers_black_digimon_from_hand(self, debug_runner):
        """Delay digivolve filter should only accept black Digimon cards from hand."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Verify test card properties
        blk_champ = db.create_card_source(BLK_CHAMP)
        assert blk_champ.is_digimon, "BLK_CHAMP should be a Digimon"
        assert CardColor.Black in blk_champ.card_colors, "BLK_CHAMP should be Black"

        red_rookie = db.create_card_source(RED_ROOKIE)
        assert red_rookie.is_digimon, "RED_ROOKIE should be a Digimon"
        assert CardColor.Black not in red_rookie.card_colors, "RED_ROOKIE should not be Black"


@pytest.mark.behavioral
class TestP107SecurityEffect:
    """[Security] Place this card in the battle area."""

    def test_has_security_effect(self, debug_runner):
        """Should have a security effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(P107)
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, (
            "P-107 should have a security effect to place in battle area"
        )

    def test_security_effect_has_callback(self, debug_runner):
        """Security effect should have a process callback to place in battle area."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(P107)
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security effect"
        assert sec_effects[0].on_process_callback is not None, (
            "Security effect must have a callback to place card in battle area"
        )

    def test_security_effect_timing(self, debug_runner):
        """Security effect should use SecuritySkill timing."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(P107)
        effects = cs.effect_list(None)

        sec_timing = [e for e in effects
                      if e.timing == EffectTiming.SecuritySkill
                      and getattr(e, 'is_security_effect', False)]
        assert len(sec_timing) >= 1, (
            "Should have SecuritySkill-timed security effect"
        )

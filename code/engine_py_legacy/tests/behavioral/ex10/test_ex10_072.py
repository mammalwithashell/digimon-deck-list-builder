"""Behavioral tests for EX10-072 Spiral Mountain (Option, White, Cost 3, Traits: Dark Masters).

Card text:
  While you don't have [Spiral Mountain] in the battle area, you can ignore
  this card's color requirements.
  [Main] <Draw 2> (Draw 2 cards from your deck.) Then, place this card in
  the battle area.
  [End of Opponent's Turn] <Delay> (By trashing this card after the placing
  turn, activate the effect below.)
  - You may play 1 face-up Digimon card with the [Dark Masters] trait from
    your security stack without paying the cost. At the end of your turn,
    delete the Digimon this effect played.

Security Effect:
  [Security] You may play 1 Digimon card with the [Dark Masters] trait from
  your hand or trash without paying the cost. Then, add this card to the hand.
  At turn end, delete the Digimon this effect played.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX10072SpiralMountain:
    """Tests for EX10-072 Spiral Mountain."""

    # ----------------------------------------------------------------
    # Color bypass: conditional on no Spiral Mountain in battle area
    # ----------------------------------------------------------------

    def test_color_bypass_when_no_spiral_mountain_in_battle_area(self, debug_runner):
        """Should bypass color requirements when no Spiral Mountain is in battle area."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "hand")
        # Trigger lazy effect initialization (sets _match_color_requirement_fn)
        card.effect_list(None)
        # No Spiral Mountain on field -> should bypass (match_color_requirement = False)
        assert not card.match_color_requirement, (
            "Should bypass color requirement when no Spiral Mountain in battle area")

    def test_color_enforced_when_spiral_mountain_in_battle_area(self, debug_runner):
        """Should enforce color requirements when a Spiral Mountain IS in battle area."""
        runner = debug_runner(initial_memory=5)
        # Place a Spiral Mountain on field first
        runner.place_on_field(1, ["EX10-072"], turn_played=-1)
        # Now inject another one in hand
        card = runner.inject_card(1, "EX10-072", "hand")
        # Trigger lazy effect initialization (sets _match_color_requirement_fn)
        card.effect_list(None)
        # Spiral Mountain IS on the field -> should enforce (match_color_requirement = True)
        assert card.match_color_requirement, (
            "Should enforce color requirement when Spiral Mountain is already in battle area")

    # ----------------------------------------------------------------
    # Main effect: Draw 2, place in battle area
    # ----------------------------------------------------------------

    def test_main_effect_exists(self, debug_runner):
        """Should have an OptionSkill effect for the Main effect."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "hand")
        effects = card.effect_list(None)
        option_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_effects) >= 1, "Should have OptionSkill (Main) effect"

    def test_main_effect_draws_2_cards(self, debug_runner):
        """Main effect should draw 2 cards."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "hand")
        effects = card.effect_list(None)
        option_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        effect = option_effects[0]

        player = runner.game.player1
        hand_before = len(player.hand_cards)
        lib_before = len(player.library_cards)

        effect.on_process_callback({'player': player, 'game': runner.game})

        assert len(player.hand_cards) == hand_before + 2, (
            f"Should draw 2 cards. Hand before: {hand_before}, after: {len(player.hand_cards)}")
        assert len(player.library_cards) == lib_before - 2, (
            "Library should decrease by 2")

    # ----------------------------------------------------------------
    # Delay marker
    # ----------------------------------------------------------------

    def test_delay_marker_exists(self, debug_runner):
        """Should have a Delay marker effect."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "hand")
        effects = card.effect_list(None)
        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) == 1, "Should have exactly 1 Delay marker effect"

    # ----------------------------------------------------------------
    # Delay effect: play Dark Masters Digimon from face-up security
    # ----------------------------------------------------------------

    def test_delay_effect_exists(self, debug_runner):
        """Should have an OnEndTurn delay effect."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "hand")
        effects = card.effect_list(None)
        end_turn_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEndTurn]
        assert len(end_turn_effects) >= 1, "Should have OnEndTurn delay effect"

    def test_delay_effect_is_optional(self, debug_runner):
        """Delay effect should be optional ('You may play...')."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "hand")
        effects = card.effect_list(None)
        end_turn_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEndTurn]
        assert end_turn_effects[0].is_optional, "Delay effect should be optional"

    def test_delay_condition_requires_opponent_turn(self, debug_runner):
        """Delay effect condition should require it to be the opponent's turn.

        C# reference: CanUseCondition checks CardEffectCommons.IsOpponentTurn(card)
        plus CanDeclareOptionDelayEffect(card).
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-072"], turn_played=0)
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEndTurn]
        delay_effect = end_turn_effects[0]

        # It's P1's turn (owner's turn) -> should NOT activate delay
        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert not delay_effect.can_use_condition(ctx), (
            "Delay condition should fail on owner's turn (requires opponent's turn)")

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        assert delay_effect.can_use_condition(ctx), (
            "Delay condition should pass on opponent's turn")

    def test_delay_process_plays_face_up_dark_masters_from_security(self, debug_runner):
        """Delay should let player play a face-up Dark Masters Digimon from security."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-072"], turn_played=0)
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEndTurn]
        delay_effect = end_turn_effects[0]

        player = runner.game.player1
        # Inject a Dark Masters Digimon into security and make it face-up
        dm_card = runner.inject_card(1, "BT15-031", "security_top")  # MetalSeadramon
        player.face_up_security.add(dm_card)

        field_before = len(player.battle_area)

        delay_effect.on_process_callback({
            'player': player,
            'game': runner.game,
        })
        runner.auto_resolve()

        # The DM Digimon should now be on the field
        assert len(player.battle_area) > field_before, (
            "Should have played a Dark Masters Digimon from security to field")

    def test_delay_process_ignores_face_down_security(self, debug_runner):
        """Delay should NOT play face-down security cards (only face-up)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-072"], turn_played=0)
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEndTurn]
        delay_effect = end_turn_effects[0]

        player = runner.game.player1
        # Clear all security, then inject a face-DOWN Dark Masters Digimon
        runner.clear_zone(1, "security")
        dm_card = runner.inject_card(1, "BT15-031", "security_top")
        # NOT added to face_up_security -> it's face-down

        field_before = len(player.battle_area)
        sec_before = len(player.security_cards)

        delay_effect.on_process_callback({
            'player': player,
            'game': runner.game,
        })
        runner.auto_resolve()

        # Should NOT have played anything (no face-up targets)
        assert len(player.security_cards) == sec_before, (
            "Should not remove face-down cards from security")

    def test_delay_process_ignores_non_dark_masters(self, debug_runner):
        """Delay should NOT play Digimon without [Dark Masters] trait from security."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-072"], turn_played=0)
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEndTurn]
        delay_effect = end_turn_effects[0]

        player = runner.game.player1
        # Clear security, inject a NON-Dark Masters Digimon face-up
        runner.clear_zone(1, "security")
        non_dm_card = runner.inject_card(1, "ST1-03", "security_top")  # Agumon
        player.face_up_security.add(non_dm_card)

        sec_before = len(player.security_cards)

        delay_effect.on_process_callback({
            'player': player,
            'game': runner.game,
        })
        runner.auto_resolve()

        assert len(player.security_cards) == sec_before, (
            "Should not play non-Dark Masters Digimon from security")

    def test_delay_process_ignores_non_digimon(self, debug_runner):
        """Delay should NOT play non-Digimon cards from security (even with Dark Masters trait)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-072"], turn_played=0)
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEndTurn]
        delay_effect = end_turn_effects[0]

        player = runner.game.player1
        runner.clear_zone(1, "security")
        # Spiral Mountain itself has Dark Masters trait but is an Option, not a Digimon
        opt_card = runner.inject_card(1, "EX10-072", "security_top")
        player.face_up_security.add(opt_card)

        sec_before = len(player.security_cards)

        delay_effect.on_process_callback({
            'player': player,
            'game': runner.game,
        })
        runner.auto_resolve()

        assert len(player.security_cards) == sec_before, (
            "Should not play non-Digimon cards from security")

    # ----------------------------------------------------------------
    # Security effect: play Dark Masters from hand/trash, add to hand
    # ----------------------------------------------------------------

    def test_security_effect_exists(self, debug_runner):
        """Should have a Security effect."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "hand")
        effects = card.effect_list(None)
        sec_effects = [e for e in effects if e.is_security_effect]
        assert len(sec_effects) >= 1, "Should have Security effect"

    def test_security_effect_timing(self, debug_runner):
        """Security effect should have SecuritySkill timing."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "hand")
        effects = card.effect_list(None)
        sec_effects = [e for e in effects if e.is_security_effect]
        assert sec_effects[0].timing == EffectTiming.SecuritySkill, (
            "Security effect should have SecuritySkill timing")

    def test_security_process_plays_dark_masters_from_hand(self, debug_runner):
        """Security effect should let player play Dark Masters Digimon from hand."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "hand")
        effects = card.effect_list(None)
        sec_effects = [e for e in effects if e.is_security_effect]
        sec_effect = sec_effects[0]

        player = runner.game.player1
        # Inject a Dark Masters Digimon into hand
        dm_card = runner.inject_card(1, "BT15-031", "hand")  # MetalSeadramon

        field_before = len(player.battle_area)

        sec_effect.on_process_callback({
            'player': player,
            'game': runner.game,
        })
        runner.auto_resolve()

        # Should have played the DM Digimon
        assert len(player.battle_area) > field_before, (
            "Should play Dark Masters Digimon from hand to field")

    def test_security_process_plays_dark_masters_from_trash(self, debug_runner):
        """Security effect should let player play Dark Masters Digimon from trash."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "hand")
        effects = card.effect_list(None)
        sec_effects = [e for e in effects if e.is_security_effect]
        sec_effect = sec_effects[0]

        player = runner.game.player1
        # Inject a Dark Masters Digimon into trash
        dm_card = runner.inject_card(1, "BT15-031", "trash")  # MetalSeadramon
        # Clear hand of any DM Digimon to force trash selection
        runner.clear_zone(1, "hand")

        field_before = len(player.battle_area)

        sec_effect.on_process_callback({
            'player': player,
            'game': runner.game,
        })

        # The selection phase offers trash index (130+) and decline (62).
        # Explicitly pick the trash card, not decline.
        select_action = runner.find_action("MetalSeadramon")
        if select_action is None:
            # Fallback: find any trash selection action
            legal = runner.action_mask()
            select_action = max(legal)  # Trash indices are higher than decline
        runner.execute(select_action)
        runner.auto_resolve()

        assert len(player.battle_area) > field_before, (
            "Should play Dark Masters Digimon from trash to field")

    def test_security_process_adds_this_card_to_hand(self, debug_runner):
        """Security effect: 'Then, add this card to the hand' after playing.

        The card should be added to hand regardless of whether a Digimon
        was played (C# calls AddThisCardToHand unconditionally at the end).
        """
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "trash")
        effects = card.effect_list(None)
        sec_effects = [e for e in effects if e.is_security_effect]
        sec_effect = sec_effects[0]

        player = runner.game.player1
        # No Dark Masters Digimon available — card still goes to hand
        runner.clear_zone(1, "hand")

        sec_effect.on_process_callback({
            'player': player,
            'game': runner.game,
        })
        runner.auto_resolve()

        hand_ids = [c.c_entity_base.card_id for c in player.hand_cards
                    if c.c_entity_base]
        assert "EX10-072" in hand_ids, (
            "Spiral Mountain should be added to hand after security effect")

    def test_security_process_does_not_play_non_dark_masters(self, debug_runner):
        """Security effect should NOT allow playing Digimon without Dark Masters trait."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "EX10-072", "hand")
        effects = card.effect_list(None)
        sec_effects = [e for e in effects if e.is_security_effect]
        sec_effect = sec_effects[0]

        player = runner.game.player1
        # Only non-Dark Masters Digimon in hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-03", "hand")  # Agumon (no Dark Masters trait)

        field_before = len(player.battle_area)

        sec_effect.on_process_callback({
            'player': player,
            'game': runner.game,
        })
        runner.auto_resolve()

        assert len(player.battle_area) == field_before, (
            "Should not play non-Dark Masters Digimon")

"""Behavioral tests for BT15-079 Piedmon (Lv.6 Purple, Cost 11, DP 11000).

Card text:
  [On Play] [When Attacking] Delete 1 of your opponent's unsuspended Digimon.
  [Your Turn] This Digimon can only digivolve into white Digimon.
  [End of Opponent's Turn] Delete this Digimon. Then, you may play 1 Digimon
      card with the [Dark Masters] trait, other than [Piedmon], from your hand
      without paying the cost.
  Inherited: Retaliation
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase, CardColor


@pytest.mark.behavioral
class TestBT15079Piedmon:
    """Tests for BT15-079 Piedmon."""

    # ---- Effect structure ------------------------------------------------

    def test_has_on_play_effect(self, debug_runner):
        """Should have an On Play effect to delete 1 unsuspended opponent Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-079"])
        effects = perm.top_card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]
        assert len(on_play) >= 1, "Should have On Play effect"

    def test_has_when_attacking_effect(self, debug_runner):
        """Should have a When Attacking effect to delete 1 unsuspended opponent Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-079"])
        effects = perm.top_card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack]
        assert len(on_attack) >= 1, "Should have When Attacking effect"

    def test_has_end_of_opponent_turn_effect(self, debug_runner):
        """Should have an End of Turn effect for self-delete + play Dark Masters."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-079"])
        effects = perm.top_card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(end_turn) >= 1, "Should have OnEndTurn effect"

    def test_has_inherited_retaliation(self, debug_runner):
        """Should have inherited Retaliation."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-079"])
        effects = perm.top_card.effect_list(None)
        retaliation = [e for e in effects
                       if e.is_inherited_effect and getattr(e, '_is_retaliation', False)]
        assert len(retaliation) >= 1, "Should have inherited Retaliation"

    # ---- [On Play] Delete 1 unsuspended opponent Digimon -----------------

    def test_on_play_deletes_unsuspended_opponent_digimon(self, debug_runner):
        """On Play process should delete 1 of opponent's unsuspended Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-079"])
        # Place an unsuspended opponent Digimon
        opp_perm = runner.place_on_field(2, ["BT1-025"])
        assert not opp_perm.is_suspended

        effects = perm.top_card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        on_play.on_process_callback(ctx)
        runner.auto_resolve()

        # Opponent's Digimon should be deleted (moved to trash)
        assert len(runner.game.player2.battle_area) == 0, (
            "Opponent's unsuspended Digimon should be deleted")

    def test_on_play_does_not_delete_suspended_opponent_digimon(self, debug_runner):
        """On Play should NOT delete a suspended opponent Digimon (only unsuspended)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-079"])
        # Place a suspended opponent Digimon
        opp_perm = runner.place_on_field(2, ["BT1-025"], is_suspended=True)
        assert opp_perm.is_suspended

        effects = perm.top_card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        on_play.on_process_callback(ctx)
        runner.auto_resolve()

        # Opponent's suspended Digimon should NOT be deleted
        assert len(runner.game.player2.battle_area) == 1, (
            "Suspended Digimon should NOT be targeted by this effect")

    def test_on_play_condition_requires_permanent(self, debug_runner):
        """On Play condition should require the card to be on field."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT15-079", "hand")
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        # Card in hand, not on field
        assert not on_play.can_use_condition({}), (
            "Condition should fail when card is not on field")

    # ---- [When Attacking] Delete 1 unsuspended opponent Digimon ----------

    def test_when_attacking_deletes_unsuspended_opponent_digimon(self, debug_runner):
        """When Attacking process should delete 1 of opponent's unsuspended Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-079"])
        opp_perm = runner.place_on_field(2, ["BT1-025"])
        assert not opp_perm.is_suspended

        effects = perm.top_card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack][0]

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        on_attack.on_process_callback(ctx)
        runner.auto_resolve()

        assert len(runner.game.player2.battle_area) == 0, (
            "Opponent's unsuspended Digimon should be deleted on attack")

    def test_when_attacking_engine_only_fires_for_attacker(self, debug_runner):
        """Engine's _effect_matches_timing should only fire When Attacking for the attacker permanent.

        This is an engine-level guarantee; effect1 uses is_on_attack=True which
        the engine filters via `perm is attacker` in _effect_matches_timing.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-079"])
        effects = perm.top_card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack][0]
        assert on_attack.is_on_attack is True, (
            "When Attacking effect should have is_on_attack=True for engine filtering")

    # ---- [Your Turn] Digivolve restriction (white only) ------------------

    def _trigger_main_phase_effects(self, runner):
        """Helper: trigger OnStartMainPhase effects to register modifiers."""
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

    def test_digivolve_restriction_blocks_non_white(self, debug_runner):
        """[Your Turn] restriction should block non-white cards from digivolving onto Piedmon.

        BT13-092 Ravemon: Burst Mode is a purple Lv.7 that evolves from purple Lv.6.
        Normally it could digivolve onto Piedmon. The restriction should block it.
        """
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT15-079"])

        # Inject a purple Lv.7 into hand that normally evolves from purple Lv.6
        runner.inject_card(1, "BT13-092", "hand")  # Ravemon: Burst Mode

        # Trigger OnStartMainPhase to register the digivolve restriction modifier
        self._trigger_main_phase_effects(runner)

        # Must be in Main phase for digivolve actions to appear
        runner.set_phase("Main")

        # Find the field slot index for Piedmon
        piedmon_slot = None
        for i, p in enumerate(runner.game.player1.battle_area):
            if p is perm:
                piedmon_slot = i
                break
        assert piedmon_slot is not None

        # Verify BT13-092 can normally digivolve onto Piedmon (color/level match)
        from engine_py_legacy.engine.validation.digivolve_validator import can_digivolve
        hand_card = runner.game.player1.hand_cards[-1]
        assert can_digivolve(hand_card, perm), (
            "BT13-092 should normally be able to digivolve onto purple Lv.6")

        # But the restriction should block it in the action mask
        from engine_py_legacy.engine.game.constants import FIELDS_PER_HAND
        hand_idx = len(runner.game.player1.hand_cards) - 1  # Last injected card
        digi_action_id = 400 + hand_idx * FIELDS_PER_HAND + piedmon_slot
        mask = runner.game.get_action_mask(1)
        assert mask[digi_action_id] == 0.0, (
            "Non-white card should NOT be able to digivolve onto Piedmon during owner's turn")

    def test_digivolve_restriction_allows_white(self, debug_runner):
        """[Your Turn] restriction should allow white cards to digivolve onto Piedmon.

        Since no white Lv.7 normally evolves from purple Lv.6, we test that the
        modifier condition itself allows white cards by checking the condition function.
        """
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT15-079"])

        # Trigger OnStartMainPhase to register the digivolve restriction modifier
        self._trigger_main_phase_effects(runner)

        # Verify the CANNOT_DIGIVOLVE modifier is registered
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType

        # For a non-white hand card, modifier should be active (blocking)
        runner.inject_card(1, "BT13-092", "hand")  # Purple Lv.7
        purple_card = runner.game.player1.hand_cards[-1]
        assert runner.game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_DIGIVOLVE,
            {'digivolving_card': purple_card}
        ), "Modifier should block non-white digivolution"

        # For a white hand card, modifier should NOT be active (allowing)
        runner.inject_card(1, "BT1-084", "hand")  # White Omnimon Lv.7
        white_card = runner.game.player1.hand_cards[-1]
        assert not runner.game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_DIGIVOLVE,
            {'digivolving_card': white_card}
        ), "Modifier should allow white digivolution"

    def test_digivolve_restriction_only_during_owners_turn(self, debug_runner):
        """[Your Turn] restriction should only apply during owner's turn."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT15-079"])

        # Trigger OnStartMainPhase to register the digivolve restriction modifier
        self._trigger_main_phase_effects(runner)

        from engine_py_legacy.engine.interfaces.modifiers import ModifierType

        runner.inject_card(1, "BT13-092", "hand")
        purple_card = runner.game.player1.hand_cards[-1]

        # P1's turn - should block
        assert runner.game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_DIGIVOLVE,
            {'digivolving_card': purple_card}
        ), "Should block during owner's turn"

        # Switch to P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        assert not runner.game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_DIGIVOLVE,
            {'digivolving_card': purple_card}
        ), "Should NOT block during opponent's turn"

    # ---- [End of Opponent's Turn] Self-delete + play Dark Masters --------

    def test_end_of_opponent_turn_condition_requires_opponents_turn(self, debug_runner):
        """End of Turn effect should only fire on opponent's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-079"])
        effects = perm.top_card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        # P1's turn - should NOT fire
        assert not end_turn.can_use_condition({}), (
            "End of Turn effect should NOT fire on owner's turn")

        # Switch to P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        assert end_turn.can_use_condition({}), (
            "End of Turn effect should fire on opponent's turn")

    def test_end_of_opponent_turn_deletes_self(self, debug_runner):
        """End of Turn process should delete this Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-079"])

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        effects = perm.top_card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        end_turn.on_process_callback(ctx)
        runner.auto_resolve()

        # Piedmon should be deleted
        assert perm not in runner.game.player1.battle_area, (
            "Piedmon should be deleted at end of opponent's turn")

    def test_end_of_opponent_turn_plays_dark_masters_from_hand(self, debug_runner):
        """After self-delete, may play 1 Dark Masters (not Piedmon) from hand free."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["BT15-079"])

        # Clear hand and inject a Dark Masters Digimon (not Piedmon)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT15-052", "hand")  # Puppetmon (Green, Dark Masters)

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        effects = perm.top_card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        end_turn.on_process_callback(ctx)
        runner.auto_resolve()

        # Piedmon should be deleted and Puppetmon should be played
        assert perm not in runner.game.player1.battle_area, (
            "Piedmon should be deleted")
        # Check a Dark Masters Digimon was played to field
        field_names = [p.top_card.c_entity_base.card_name_eng
                       for p in runner.game.player1.battle_area if p.top_card]
        assert "Puppetmon" in field_names, (
            f"Puppetmon should be played from hand. Field: {field_names}")

    def test_end_of_opponent_turn_does_not_play_piedmon(self, debug_runner):
        """Should NOT allow playing another Piedmon (excluded by name)."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["BT15-079"])

        # Clear hand and inject another Piedmon
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT15-079", "hand")  # Another Piedmon

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        effects = perm.top_card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        end_turn.on_process_callback(ctx)
        runner.auto_resolve()

        # Piedmon should be deleted and no new Digimon should be played
        assert perm not in runner.game.player1.battle_area, (
            "Piedmon should be deleted")
        # The hand Piedmon should still be in hand (not played)
        hand_ids = [c.c_entity_base.card_id for c in runner.game.player1.hand_cards
                    if c.c_entity_base]
        assert "BT15-079" in hand_ids, (
            "Piedmon in hand should NOT be playable by this effect")

    def test_end_of_opponent_turn_does_not_play_non_dark_masters(self, debug_runner):
        """Should NOT play a Digimon without [Dark Masters] trait."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["BT15-079"])

        # Clear hand and inject a non-Dark-Masters Digimon
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT1-025", "hand")  # Greymon (no Dark Masters)

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        effects = perm.top_card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        end_turn.on_process_callback(ctx)
        runner.auto_resolve()

        # Non-Dark-Masters should remain in hand
        hand_ids = [c.c_entity_base.card_id for c in runner.game.player1.hand_cards
                    if c.c_entity_base]
        assert "BT1-025" in hand_ids, (
            "Non-Dark-Masters Digimon should NOT be playable")

    def test_end_of_opponent_turn_play_is_optional(self, debug_runner):
        """The play Dark Masters part should be optional (you may play)."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["BT15-079"])

        # Empty hand - no candidates to play
        runner.clear_zone(1, "hand")

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        effects = perm.top_card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        # Should not raise with empty hand
        end_turn.on_process_callback(ctx)
        runner.auto_resolve()

        # Piedmon should still be deleted
        assert perm not in runner.game.player1.battle_area, (
            "Piedmon should be deleted even with empty hand")

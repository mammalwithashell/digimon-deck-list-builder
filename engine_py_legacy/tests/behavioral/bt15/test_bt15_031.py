"""Behavioral tests for BT15-031 MetalSeadramon (Blue, Lv.6, 11000 DP, Cost 11).

Card text:
  [On Play] [When Attacking] Return 1 of your opponent's level 5 or lower
  Digimon to the hand.
  [Your Turn] This Digimon can only digivolve into white Digimon.
  [End of Opponent's Turn] Delete this Digimon. Then, you may play 1 Digimon
  card with the [Dark Masters] trait, other than [MetalSeadramon], from your
  hand without paying the cost.

  Inherited: <Blocker>
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, CardColor
from digimon_gym.engine.interfaces.modifiers import ModifierType


@pytest.mark.behavioral
class TestBT15031MetalSeadramon:
    """Tests for BT15-031 MetalSeadramon."""

    # ── Inherited Blocker ───────────────────────────────────────────────

    def test_inherited_blocker(self, debug_runner):
        """Should have an inherited Blocker effect."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT15-031", "hand")
        effects = card.effect_list(None)
        blocker = [e for e in effects if getattr(e, '_is_blocker', False)]
        assert len(blocker) >= 1, "Should have Blocker effect"
        assert blocker[0].is_inherited_effect, "Blocker should be inherited"

    # ── [On Play] Bounce opponent's Lv.5 or lower Digimon ──────────────

    def test_on_play_bounce_exists(self, debug_runner):
        """Should have an On Play effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-031"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]
        assert len(on_play) >= 1, "Should have On Play effect"

    def test_on_play_bounce_returns_lv5_or_lower(self, debug_runner):
        """On Play should return opponent's Lv.5 or lower Digimon to hand."""
        runner = debug_runner(initial_memory=15)
        runner.set_phase("Main")
        # Place opponent Lv.5 Digimon
        opp_perm = runner.place_on_field(2, ["BT1-040"])  # WereGarurumon, Lv.5
        p2_field_before = len(runner.game.player2.battle_area)

        # Inject and play BT15-031
        runner.inject_card(1, "BT15-031", "hand")
        play_action = runner.find_action("Play MetalSeadramon")
        assert play_action is not None, (
            f"Should be able to play MetalSeadramon. Actions: {runner.actions()}")
        runner.execute(play_action)
        runner.auto_resolve()

        # Opponent's Lv.5 should be bounced to hand
        assert len(runner.game.player2.battle_area) < p2_field_before, (
            "Opponent should have fewer Digimon on field after bounce")

    def test_on_play_bounce_does_not_target_lv6(self, debug_runner):
        """On Play should not target opponent's Lv.6 or higher Digimon."""
        runner = debug_runner(initial_memory=15)
        runner.set_phase("Main")
        # Place only a Lv.6 on opponent's field
        runner.clear_zone(2, "field")
        opp_perm = runner.place_on_field(2, ["BT15-052"])  # Puppetmon, Lv.6
        p2_field_count = len(runner.game.player2.battle_area)

        # Play BT15-031
        runner.inject_card(1, "BT15-031", "hand")
        play_action = runner.find_action("Play MetalSeadramon")
        assert play_action is not None, (
            f"Should be able to play MetalSeadramon. Actions: {runner.actions()}")
        runner.execute(play_action)
        runner.auto_resolve()

        # Opponent Lv.6 should not be bounced
        assert len(runner.game.player2.battle_area) == p2_field_count, (
            "Opponent's Lv.6 Digimon should not be bounced")

    # ── [When Attacking] Bounce opponent's Lv.5 or lower Digimon ───────

    def test_when_attacking_bounce(self, debug_runner):
        """When Attacking should return opponent's Lv.5 or lower Digimon to hand."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, ["BT15-031"], turn_played=-1)
        opp_perm = runner.place_on_field(2, ["BT1-040"])  # WereGarurumon, Lv.5
        p2_field_before = len(runner.game.player2.battle_area)

        # Attack with MetalSeadramon (attack player/security)
        attack_action = runner.find_action("Attack")
        assert attack_action is not None, (
            f"Should be able to attack. Actions: {runner.actions()}")
        runner.execute(attack_action)
        runner.auto_resolve()

        # Opponent's Lv.5 should be bounced
        assert len(runner.game.player2.battle_area) < p2_field_before, (
            "Opponent should have fewer Digimon on field after When Attacking bounce")

    # ── [Your Turn] Can only digivolve into white Digimon ──────────────

    @staticmethod
    def _register_restriction(runner, perm):
        """Helper: manually register the digivolve restriction modifier.

        In a real game, this fires via OnEnterFieldAnyone when the card is played.
        For place_on_field tests, we need to trigger it manually.
        """
        card = perm.top_card
        effects = card.effect_list(None)
        restrict_eff = [e for e in effects if getattr(e, '_register_digivolve_restriction', None)]
        if restrict_eff:
            restrict_eff[0]._register_digivolve_restriction(runner.game, perm)

    def test_digivolve_restriction_blocks_non_white(self, debug_runner):
        """During owner's turn, MetalSeadramon should NOT be able to digivolve
        into non-white Digimon."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, ["BT15-031"])
        self._register_restriction(runner, perm)

        # Put a non-white Lv.7 Digimon that evolves from blue Lv.6 in hand
        # BT6-030 Gabumon Bond of Friendship (Blue, Lv.7, evo from Blue Lv.6)
        runner.inject_card(1, "BT6-030", "hand")

        # The digivolve action should be masked out
        digivolve_actions = runner.find_actions("Digivolve")
        # Filter for Gabumon-Bond digivolve actions onto MetalSeadramon
        gabumon_digi = {aid: desc for aid, desc in digivolve_actions.items()
                        if "Gabumon" in desc or "BT6-030" in desc}
        assert len(gabumon_digi) == 0, (
            f"Should NOT allow digivolving non-white Digimon onto MetalSeadramon. "
            f"Found: {gabumon_digi}")

    def test_digivolve_restriction_allows_white(self, debug_runner):
        """During owner's turn, MetalSeadramon should be able to digivolve
        into white Digimon."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, ["BT15-031"])
        self._register_restriction(runner, perm)

        # Put a white Lv.7 that evolves from blue Lv.6 in hand
        # BT15-102 Apocalymon (White, Lv.7, evo from Blue Lv.6, cost 6)
        runner.inject_card(1, "BT15-102", "hand")

        # The digivolve action should be available
        digivolve_actions = runner.find_actions("Digivolve")
        apoc_digi = {aid: desc for aid, desc in digivolve_actions.items()
                     if "Apocalymon" in desc or "BT15-102" in desc}
        assert len(apoc_digi) >= 1, (
            f"Should allow digivolving white Digimon onto MetalSeadramon. "
            f"Available actions: {runner.actions()}")

    def test_digivolve_restriction_only_during_own_turn(self, debug_runner):
        """The digivolve restriction only applies during owner's turn.
        On opponent's turn, it should not matter (opponent can't digivolve
        onto your Digimon anyway, but the modifier condition should check turn)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-031"])
        self._register_restriction(runner, perm)

        # Verify that the CANNOT_DIGIVOLVE modifier is registered
        game = runner.game

        # The modifier condition should check owner's turn
        # When it IS owner's turn, non-white should be blocked
        non_white_ctx = {'digivolving_card': type('Obj', (), {'card_colors': [CardColor.Blue]})()}
        has_block = game.modifiers.has_modifier(perm, ModifierType.CANNOT_DIGIVOLVE, non_white_ctx)
        assert has_block, (
            "Should block non-white digivolve during owner's turn")

        # White should NOT be blocked
        white_ctx = {'digivolving_card': type('Obj', (), {'card_colors': [CardColor.White]})()}
        has_block_white = game.modifiers.has_modifier(perm, ModifierType.CANNOT_DIGIVOLVE, white_ctx)
        assert not has_block_white, (
            "Should NOT block white digivolve during owner's turn")

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        # During opponent's turn, restriction should not apply
        has_block_opp = game.modifiers.has_modifier(perm, ModifierType.CANNOT_DIGIVOLVE, non_white_ctx)
        assert not has_block_opp, (
            "Should NOT block digivolve during opponent's turn")

    # ── [End of Opponent's Turn] Delete self, play Dark Masters ────────

    def test_end_of_opponent_turn_effect_exists(self, debug_runner):
        """Should have an OnEndTurn effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-031"])
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(end_turn) >= 1, "Should have OnEndTurn effect"

    def test_end_of_opponent_turn_condition_only_on_opponent_turn(self, debug_runner):
        """Condition should only fire on opponent's turn (not own turn)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-031"])
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        # On own turn, condition should fail
        ctx = {'player': runner.game.player1, 'game': runner.game, 'permanent': perm}
        assert not end_turn.can_use_condition(ctx), (
            "Should NOT fire on own turn")

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        assert end_turn.can_use_condition(ctx), (
            "Should fire on opponent's turn")

    def test_end_of_opponent_turn_deletes_self(self, debug_runner):
        """Process should delete this Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-031"])
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        p1_field_before = len(runner.game.player1.battle_area)

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        end_turn.on_process_callback(ctx)

        assert len(runner.game.player1.battle_area) < p1_field_before, (
            "MetalSeadramon should be deleted from field")

    def test_end_of_opponent_turn_plays_dark_masters_from_hand(self, debug_runner):
        """After deletion, should optionally play a Dark Masters Digimon
        (not MetalSeadramon) from hand for free."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-031"])
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        # Put Puppetmon (Dark Masters, not MetalSeadramon) in hand
        runner.inject_card(1, "BT15-052", "hand")

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        end_turn.on_process_callback(ctx)
        runner.auto_resolve()

        # Puppetmon should be played from hand (now on field)
        p1_field_names = [
            p.top_card.c_entity_base.card_name_eng
            for p in runner.game.player1.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert "Puppetmon" in p1_field_names, (
            f"Puppetmon should be played from hand. Field: {p1_field_names}")

    def test_end_of_opponent_turn_does_not_play_metalseadramon(self, debug_runner):
        """Should NOT be able to play MetalSeadramon from hand (self-exclusion)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-031"])
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        # Only MetalSeadramon in hand — should not be playable
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT15-031", "hand")

        p1_hand_before = len(runner.game.player1.hand_cards)

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        end_turn.on_process_callback(ctx)
        runner.auto_resolve()

        # MetalSeadramon should still be in hand (not played)
        hand_ids = [
            c.c_entity_base.card_id
            for c in runner.game.player1.hand_cards
            if c.c_entity_base
        ]
        assert "BT15-031" in hand_ids, (
            f"MetalSeadramon should still be in hand (excluded). Hand: {hand_ids}")

    def test_end_of_opponent_turn_play_is_optional(self, debug_runner):
        """The play effect should be optional (you MAY play)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-031"])
        card = perm.top_card
        effects = card.effect_list(None)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        # No Dark Masters in hand — should still succeed (optional, nothing played)
        runner.clear_zone(1, "hand")

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        }
        # Should not raise
        end_turn.on_process_callback(ctx)
        runner.auto_resolve()

        # MetalSeadramon should still be deleted
        assert perm not in runner.game.player1.battle_area, (
            "MetalSeadramon should be deleted even with no Dark Masters to play")

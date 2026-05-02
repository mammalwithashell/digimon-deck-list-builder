"""Behavioral tests for BT15-066 Machinedramon (Lv.6, Black, Machine/Dark Masters).

Card text:
[On Play] [When Attacking] <De-Digivolve 2> 1 of your opponent's Digimon.
[Your Turn] This Digimon can only digivolve into white Digimon.
[End of Opponent's Turn] Delete this Digimon. Then, you may play 1 Digimon card
    with the [Dark Masters] trait, other than [Machinedramon], from your hand
    without paying the cost.
Inherited: <Reboot>
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, CardColor


# Minimal decks for DebugRunner — just need filler cards
FILLER_DECK = ["BT1-010"] * 4 + ["ST1-03"] * 46


@pytest.mark.behavioral
class TestBT15066OnPlay:
    """[On Play] <De-Digivolve 2> 1 of your opponent's Digimon."""

    def test_on_play_de_digivolve_triggers(self, debug_runner):
        """On-play should trigger De-Digivolve 2 on an opponent's Digimon."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=12,
        )
        # Place a Lv.5 Digimon on opponent's field with a digivolution stack
        opp_perm = runner.place_on_field(2, ["BT1-010", "ST1-03", "ST1-06"])
        stack_before = len(opp_perm.card_sources)

        # Place Machinedramon in P1's hand
        runner.inject_card(1, "BT15-066", "hand")
        play_action = runner.find_action("Play Machinedramon from hand")
        assert play_action is not None, (
            f"Should be able to play Machinedramon. Actions: {runner.actions()}"
        )
        runner.execute(play_action)
        runner.auto_resolve()  # auto-resolve De-Digivolve target selection

        # Opponent's Digimon should have lost 2 cards from stack
        game = runner.game
        # Check trash for de-digivolved cards
        p2_trash_count = len(game.player2.trash_cards)
        assert p2_trash_count >= 2, (
            f"De-Digivolve 2 should put 2 cards in opponent trash, got {p2_trash_count}"
        )


@pytest.mark.behavioral
class TestBT15066WhenAttacking:
    """[When Attacking] <De-Digivolve 2> 1 of your opponent's Digimon."""

    def test_when_attacking_de_digivolve_triggers(self, debug_runner):
        """When attacking, should trigger De-Digivolve 2 on opponent Digimon."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        # Place Machinedramon on P1's field (already on field, not suspended)
        runner.place_on_field(1, ["BT15-066"])
        # Place opponent Digimon with stack
        runner.place_on_field(2, ["BT1-010", "ST1-03", "ST1-06"])

        game = runner.game
        p2_trash_before = len(game.player2.trash_cards)

        # Attack with Machinedramon
        atk_action = runner.find_action("Attack")
        assert atk_action is not None, "Machinedramon should be able to attack"
        runner.execute(atk_action)
        runner.auto_resolve()  # resolve De-Digivolve target + attack

        p2_trash_after = len(game.player2.trash_cards)
        assert p2_trash_after >= p2_trash_before + 2, (
            f"De-Digivolve 2 on attack should put 2+ cards in opponent trash"
        )


@pytest.mark.behavioral
class TestBT15066DigivolveRestriction:
    """[Your Turn] This Digimon can only digivolve into white Digimon."""

    def _play_machinedramon(self, runner):
        """Helper: play Machinedramon from hand so on-play effects fire."""
        runner.inject_card(1, "BT15-066", "hand")
        play_action = runner.find_action("Play Machinedramon")
        assert play_action is not None, (
            f"Should be able to play Machinedramon. Actions: {runner.actions()}"
        )
        runner.execute(play_action)
        runner.auto_resolve()  # resolve de-digivolve (no opponent targets)

    def test_can_digivolve_into_white(self, debug_runner):
        """Machinedramon should allow digivolving into a white Lv.7 Digimon."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=20,
        )
        # Play Machinedramon (cost 11) so on-play effects fire and modifier registers
        self._play_machinedramon(runner)
        # Set memory back up so we can digivolve
        runner.set_memory(10)
        # Put a white Lv.7 in hand that can evo from Black Lv.6
        # BT5-087 Omnimon Zwart: white, Lv.7, evo from Black Lv.6 for 6
        runner.inject_card(1, "BT5-087", "hand")

        # Should see a digivolve action for the white Digimon
        digivolve_actions = runner.find_actions("Digivolve")
        white_digi = [a for aid, a in digivolve_actions.items()
                      if "Omnimon Zwart" in a]
        assert len(white_digi) > 0, (
            "Should be able to digivolve Machinedramon into white Omnimon Zwart. "
            f"Available actions: {runner.actions()}"
        )

    def test_cannot_digivolve_into_non_white(self, debug_runner):
        """Machinedramon should NOT allow digivolving into a non-white Lv.7 Digimon."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=20,
        )
        # Play Machinedramon (cost 11) so on-play effects fire and modifier registers
        self._play_machinedramon(runner)
        # Set memory back up
        runner.set_memory(10)
        # Put a non-white (Black) Lv.7 in hand (BT9-111 Alphamon: Ouryuken)
        runner.inject_card(1, "BT9-111", "hand")

        # Should NOT see a digivolve action for the black-only Digimon
        digivolve_actions = runner.find_actions("Digivolve")
        non_white_digi = [a for aid, a in digivolve_actions.items()
                          if "Alphamon" in a]
        assert len(non_white_digi) == 0, (
            "Should NOT be able to digivolve Machinedramon into non-white Alphamon. "
            f"Available digivolve actions: {digivolve_actions}"
        )


@pytest.mark.behavioral
class TestBT15066EndOfOpponentTurn:
    """[End of Opponent's Turn] Delete this Digimon. Then, you may play 1 Digimon
    card with the [Dark Masters] trait, other than [Machinedramon], from your hand
    without paying the cost."""

    def _set_opponent_turn(self, game):
        """Set the game state to opponent's turn (P2's turn)."""
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

    def test_self_delete_on_opponent_turn_end(self, debug_runner):
        """Machinedramon should delete itself at the end of opponent's turn."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=0,
        )
        # Place Machinedramon on P1's field
        runner.place_on_field(1, ["BT15-066"])

        game = runner.game
        assert len(game.player1.battle_area) == 1, "Machinedramon should be on field"

        # Switch to opponent's turn and fire end-of-turn effects
        self._set_opponent_turn(game)
        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        # After opponent's turn end, Machinedramon should be deleted
        p1_field_ids = [
            p.top_card.c_entity_base.card_id
            for p in game.player1.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert "BT15-066" not in p1_field_ids, (
            "Machinedramon should be deleted at end of opponent's turn"
        )

    def test_does_not_delete_on_own_turn(self, debug_runner):
        """Machinedramon should NOT delete at the end of its owner's turn."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=3,
        )
        # Place Machinedramon on P1's field
        runner.place_on_field(1, ["BT15-066"])

        game = runner.game
        # It's P1's turn. Fire end-of-turn effects.
        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        # Machinedramon should NOT be deleted (it's own turn ending)
        p1_field_ids = [
            p.top_card.c_entity_base.card_id
            for p in game.player1.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert "BT15-066" in p1_field_ids, (
            "Machinedramon should NOT be deleted at end of its own turn"
        )

    def test_play_dark_masters_from_hand_after_delete(self, debug_runner):
        """After self-delete, should be able to play a Dark Masters Digimon
        (not Machinedramon) from hand for free."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=0,
        )
        # Place Machinedramon on P1's field
        runner.place_on_field(1, ["BT15-066"])
        # Add a Dark Masters Digimon (not Machinedramon) to P1's hand
        runner.inject_card(1, "BT15-031", "hand")  # MetalSeadramon (Dark Masters)

        game = runner.game
        hand_before = [c.c_entity_base.card_id for c in game.player1.hand_cards
                       if c.c_entity_base]
        assert "BT15-031" in hand_before, "MetalSeadramon should be in hand"

        # Switch to opponent's turn and fire end-of-turn effects
        self._set_opponent_turn(game)
        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        # MetalSeadramon should now be on field (played for free)
        p1_field_ids = [
            p.top_card.c_entity_base.card_id
            for p in game.player1.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert "BT15-031" in p1_field_ids, (
            "MetalSeadramon should be played from hand after Machinedramon self-delete. "
            f"P1 field: {p1_field_ids}"
        )

    def test_cannot_play_machinedramon_from_hand(self, debug_runner):
        """Should NOT be able to play another Machinedramon from hand
        (other than [Machinedramon] restriction)."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=0,
        )
        # Place Machinedramon on P1's field
        runner.place_on_field(1, ["BT15-066"])
        # Add another Machinedramon to hand (only Machinedramon available)
        runner.inject_card(1, "BT15-066", "hand")

        game = runner.game

        # Switch to opponent's turn and fire end-of-turn effects
        self._set_opponent_turn(game)
        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        # Another Machinedramon should NOT be on field
        p1_field_ids = [
            p.top_card.c_entity_base.card_id
            for p in game.player1.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        machinedramon_count = p1_field_ids.count("BT15-066")
        assert machinedramon_count == 0, (
            "Should NOT play another Machinedramon from the effect. "
            f"P1 field: {p1_field_ids}"
        )

    def test_play_from_hand_is_optional(self, debug_runner):
        """The play-from-hand part should be optional (you may play)."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=0,
        )
        # Place Machinedramon on P1's field (no Dark Masters in hand)
        runner.place_on_field(1, ["BT15-066"])

        game = runner.game

        # Switch to opponent's turn and fire end-of-turn effects
        self._set_opponent_turn(game)
        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        # Game should not crash; Machinedramon just gets deleted
        assert "BT15-066" not in [
            p.top_card.c_entity_base.card_id
            for p in game.player1.battle_area
            if p.top_card and p.top_card.c_entity_base
        ], "Machinedramon should be deleted even with no targets in hand"


@pytest.mark.behavioral
class TestBT15066InheritedReboot:
    """Inherited: <Reboot>"""

    def test_inherited_reboot(self, debug_runner):
        """Should have inherited Reboot effect."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        # Place Machinedramon as an inheritance under a Lv.7
        perm = runner.place_on_field(1, ["BT15-066", "BT1-084"])

        # Check for reboot keyword in the bottom card's effects
        source_card = perm.card_sources[0]  # BT15-066 is the bottom card
        all_effects = source_card.effect_list(None)
        reboot_effects = [
            e for e in all_effects
            if getattr(e, '_is_reboot', False)
            and getattr(e, 'is_inherited_effect', False)
        ]
        assert len(reboot_effects) >= 1, "Should have inherited Reboot effect"

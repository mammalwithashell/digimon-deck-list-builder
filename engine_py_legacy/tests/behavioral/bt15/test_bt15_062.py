"""Behavioral tests for BT15-062 Gigadramon (Lv.5, Black, Cyborg).

Card text:
[On Play] Reveal the top 4 cards of your deck. Add 2 level 6 or higher cards
    among them to the hand. Return the rest to the bottom of the deck.
[End of Your Turn] By deleting 1 of your Digimon, you may play 1 Digimon card
    with the [Dark Masters] trait from your hand to an empty space in your breeding
    area without paying the cost.
Inherited: Reboot
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase

GIGADRAMON = "BT15-062"
METALSEADRAMON = "BT15-031"  # Lv.6 Dark Masters, Cyborg
FILLER_LV3 = "ST1-03"        # Agumon (Lv.3, Red, filler)
FILLER_LV4 = "BT1-020"       # Greymon (Lv.4, Red, filler)


@pytest.mark.behavioral
class TestBT15062OnPlay:
    """[On Play] Reveal top 4 cards, add 2 Lv6+ to hand, rest to bottom."""

    def test_on_play_effect_exists(self, debug_runner):
        """Should have an On Play effect with OnEnterFieldAnyone timing."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [GIGADRAMON])
        top_card = perm.top_card
        all_effects = top_card.effect_list(None)

        on_play = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play) >= 1, "Should have an On Play effect"

    def test_on_play_reveal_adds_lv6_to_hand(self, debug_runner):
        """On Play should add Lv6+ cards from the top 4 to hand."""
        runner = debug_runner(initial_memory=10)

        # Inject MetalSeadramon (Lv.6 Dark Masters) to top of library
        runner.inject_card(1, METALSEADRAMON, "library_top")

        hand_before = len(runner.game.player1.hand_cards)

        # Place Gigadramon to trigger On Play
        runner.inject_card(1, GIGADRAMON, "hand")
        runner.set_phase("Main")
        action = runner.find_action("Play Gigadramon")
        assert action is not None, f"Should find Play action. Available: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        # MetalSeadramon (Lv.6) should have been added to hand
        hand_ids = [
            c.c_entity_base.card_id for c in runner.game.player1.hand_cards
            if c.c_entity_base
        ]
        assert METALSEADRAMON in hand_ids, (
            f"MetalSeadramon should be in hand after reveal. Hand: {hand_ids}"
        )


@pytest.mark.behavioral
class TestBT15062EndOfTurn:
    """[End of Your Turn] Delete 1 Digimon, play Dark Masters to breeding area."""

    def test_end_of_turn_effect_exists(self, debug_runner):
        """Should have an End of Turn effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [GIGADRAMON])
        top_card = perm.top_card
        all_effects = top_card.effect_list(None)

        eot = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndTurn
        ]
        assert len(eot) >= 1, "Should have an End of Turn effect"

    def test_end_of_turn_is_optional(self, debug_runner):
        """The End of Turn effect should be optional ('you may')."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [GIGADRAMON])
        top_card = perm.top_card
        all_effects = top_card.effect_list(None)

        eot = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndTurn
        ][0]
        assert eot.is_optional, "End of Turn effect should be optional"

    def test_end_of_turn_plays_to_breeding_area(self, debug_runner):
        """Dark Masters card should be played to the BREEDING area, not battle area."""
        runner = debug_runner(initial_memory=0)
        game = runner.game

        # Place Gigadramon and a filler Digimon on field
        runner.place_on_field(1, [GIGADRAMON])
        filler = runner.place_on_field(1, [FILLER_LV3])

        # Put MetalSeadramon (Dark Masters) in hand
        runner.inject_card(1, METALSEADRAMON, "hand")

        # Ensure breeding area is empty
        assert game.player1.breeding_area is None, "Breeding area should start empty"

        # Trigger End of Turn
        game.phase_end()

        # Step 1: Select the filler Digimon (slot 1) to delete
        # The effect presents SelectTarget with field slot actions
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget phase, got {game.current_phase}"
        )
        # Find the filler slot action (slot 1 = second field permanent)
        filler_action = None
        for aid, desc in runner.actions().items():
            if aid != 62 and 'target[1]' in desc:
                filler_action = aid
                break
        assert filler_action is not None, (
            f"Should find action to select filler. Available: {runner.actions()}"
        )
        runner.execute(filler_action)

        # Step 2: Select MetalSeadramon from hand to play to breeding area
        metal_action = runner.find_action("MetalSeadramon")
        assert metal_action is not None, (
            f"Should find action to select MetalSeadramon. Available: {runner.actions()}"
        )
        runner.execute(metal_action)

        # Auto-resolve any remaining selections
        runner.auto_resolve()

        # MetalSeadramon should be in breeding area, NOT in battle area
        breeding = game.player1.breeding_area
        assert breeding is not None, "Breeding area should have the played Digimon"
        assert breeding.top_card is not None
        assert breeding.top_card.c_entity_base.card_id == METALSEADRAMON, (
            f"Breeding area should have MetalSeadramon, got "
            f"{breeding.top_card.c_entity_base.card_id}"
        )

        # It should NOT be in battle_area (other than Gigadramon itself)
        field_ids = [
            p.top_card.c_entity_base.card_id
            for p in game.player1.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert METALSEADRAMON not in field_ids, (
            f"MetalSeadramon should NOT be in battle area, but found it. Field: {field_ids}"
        )

    def test_end_of_turn_deletes_digimon_as_cost(self, debug_runner):
        """Should delete 1 of your Digimon as cost before playing."""
        runner = debug_runner(initial_memory=0)
        game = runner.game

        runner.place_on_field(1, [GIGADRAMON])
        filler = runner.place_on_field(1, [FILLER_LV3])
        runner.inject_card(1, METALSEADRAMON, "hand")

        game.phase_end()

        # Select filler (slot 1) to delete
        filler_action = None
        for aid, desc in runner.actions().items():
            if aid != 62 and 'target[1]' in desc:
                filler_action = aid
                break
        assert filler_action is not None
        runner.execute(filler_action)

        # Select MetalSeadramon from hand
        metal_action = runner.find_action("MetalSeadramon")
        if metal_action:
            runner.execute(metal_action)
        runner.auto_resolve()

        # The filler should have been deleted (trash should contain it)
        trash_ids = [
            c.c_entity_base.card_id for c in game.player1.trash_cards
            if c.c_entity_base
        ]
        assert FILLER_LV3 in trash_ids, (
            f"Filler Digimon should be in trash after deletion. Trash: {trash_ids}"
        )

    def test_end_of_turn_blocked_when_breeding_occupied(self, debug_runner):
        """Should NOT play to breeding area when it is already occupied."""
        runner = debug_runner(initial_memory=0)
        game = runner.game

        runner.place_on_field(1, [GIGADRAMON])
        runner.place_on_field(1, [FILLER_LV3])
        runner.inject_card(1, METALSEADRAMON, "hand")

        # Occupy the breeding area
        runner.place_in_breeding(1, [FILLER_LV4])
        assert game.player1.breeding_area is not None, "Breeding area should be occupied"

        game.phase_end()

        # Select filler (slot 1) to delete — the effect still lets you delete
        filler_action = None
        for aid, desc in runner.actions().items():
            if aid != 62 and 'target[1]' in desc:
                filler_action = aid
                break
        if filler_action:
            runner.execute(filler_action)
        runner.auto_resolve()

        # MetalSeadramon should still be in hand (not played) because breeding is occupied
        hand_after = [
            c.c_entity_base.card_id for c in game.player1.hand_cards
            if c.c_entity_base
        ]
        assert METALSEADRAMON in hand_after, (
            f"MetalSeadramon should remain in hand when breeding is occupied. "
            f"Hand: {hand_after}"
        )

    def test_end_of_turn_only_dark_masters_trait(self, debug_runner):
        """Should only allow playing Digimon with [Dark Masters] trait."""
        runner = debug_runner(initial_memory=0)
        game = runner.game

        runner.place_on_field(1, [GIGADRAMON])
        runner.place_on_field(1, [FILLER_LV3])

        # Inject a non-Dark Masters Lv.6 Digimon in hand (no Dark Masters in hand)
        runner.inject_card(1, "BT1-025", "hand")  # MetalGreymon, no Dark Masters trait

        game.phase_end()

        # Select filler (slot 1) to delete
        filler_action = None
        for aid, desc in runner.actions().items():
            if aid != 62 and 'target[1]' in desc:
                filler_action = aid
                break
        if filler_action:
            runner.execute(filler_action)
        runner.auto_resolve()

        # No Dark Masters card in hand, so nothing should be played to breeding
        assert game.player1.breeding_area is None, (
            "Non-Dark Masters Digimon should not be playable to breeding area"
        )

    def test_end_of_turn_free_play(self, debug_runner):
        """The Dark Masters card should be played without paying its cost."""
        runner = debug_runner(initial_memory=0)
        game = runner.game

        runner.place_on_field(1, [GIGADRAMON])
        runner.place_on_field(1, [FILLER_LV3])
        runner.inject_card(1, METALSEADRAMON, "hand")

        memory_before = game.memory

        game.phase_end()

        # Select filler (slot 1) to delete
        filler_action = None
        for aid, desc in runner.actions().items():
            if aid != 62 and 'target[1]' in desc:
                filler_action = aid
                break
        if filler_action:
            runner.execute(filler_action)

        # Select MetalSeadramon from hand
        metal_action = runner.find_action("MetalSeadramon")
        if metal_action:
            runner.execute(metal_action)
        runner.auto_resolve()

        # Memory should not have decreased by MetalSeadramon's cost (11).
        # It's a free play, so memory should remain essentially the same.
        assert game.memory >= memory_before - 1, (
            f"Memory should not decrease significantly (free play). "
            f"Before: {memory_before}, After: {game.memory}"
        )

    def test_end_of_turn_condition_requires_own_turn(self, debug_runner):
        """End of Turn effect should only work on your turn."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [GIGADRAMON])
        game = runner.game

        top_card = perm.top_card
        all_effects = top_card.effect_list(None)
        eot = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndTurn
        ][0]

        # On P1's turn: should pass condition
        game.player1.is_my_turn = True
        result = eot.can_use_condition({
            'player': game.player1,
            'permanent': perm,
        })
        assert result, "Should be usable on own turn"

        # On opponent's turn: should fail condition
        game.player1.is_my_turn = False
        result = eot.can_use_condition({
            'player': game.player1,
            'permanent': perm,
        })
        assert not result, "Should NOT be usable on opponent's turn"


@pytest.mark.behavioral
class TestBT15062Inherited:
    """Inherited: Reboot"""

    def test_reboot_inherited_effect(self, debug_runner):
        """Should have Reboot as an inherited effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [GIGADRAMON])
        top_card = perm.top_card
        all_effects = top_card.effect_list(None)

        reboot = [
            e for e in all_effects
            if getattr(e, 'is_inherited_effect', False)
            and getattr(e, '_is_reboot', False)
        ]
        assert len(reboot) >= 1, "Should have Reboot as inherited effect"

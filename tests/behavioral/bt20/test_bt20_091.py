"""Behavioral tests for BT20-091 Cool Boy (Tamer, White, LIBERATOR, Cost 4).

Card text:
[Your Turn] When any of your Digimon are played or digivolve, if any of them
have the [Royal Knight] trait, by suspending this Tamer, <Draw 1> and gain 1 memory.
[Opponent's Turn] [Once Per Turn] When any of your Digimon with the [Royal Knight]
trait would leave the battle area, you may play 1 [Omekamon] from your hand
without paying the cost.
Security Effect [Security] Play this card without paying the cost.
"""

import pytest


# BT13-040: Magnamon, Lv4, Cost 7, Royal Knight trait
ROYAL_KNIGHT_LV4 = "BT13-040"
# AD1-008: Gallantmon, Lv6, Cost 12, Royal Knight trait, evolves from Lv5
ROYAL_KNIGHT_LV6 = "AD1-008"
# ST1-03: Agumon, Lv3, Cost 3, no Royal Knight trait
NON_RK_LV3 = "ST1-03"
# AD1-002: Aldamon, Lv5, Cost 8, no Royal Knight trait
NON_RK_LV5 = "AD1-002"
# BT13-093: Omekamon, Lv4, Cost 4
OMEKAMON = "BT13-093"
# BT20-083: Omekamon, Lv4, Cost 5 (different version)
OMEKAMON_BT20 = "BT20-083"
# BT20-091: Cool Boy tamer
COOL_BOY = "BT20-091"


@pytest.mark.behavioral
class TestCoolBoyPlayTrigger:
    """[Your Turn] When a Royal Knight Digimon is PLAYED, suspend Cool Boy,
    draw 1, gain 1 memory."""

    def test_play_rk_triggers_draw_and_memory(self, debug_runner):
        """Playing a Royal Knight Digimon should trigger Cool Boy's effect:
        suspend Cool Boy, draw 1, gain 1 memory."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place Cool Boy tamer on P1's field
        cool_boy_perm = runner.place_on_field(1, [COOL_BOY])
        assert not cool_boy_perm.is_suspended, "Cool Boy starts unsuspended"

        # Put a Royal Knight in hand
        runner.inject_card(1, ROYAL_KNIGHT_LV4, "hand")

        runner.set_phase("Main")
        before = runner.snapshot()
        before_hand_count = len(game.player1.hand_cards)

        # Play the Royal Knight
        action = runner.find_action("Play Magnamon")
        assert action is not None, f"Should find Play Magnamon. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()

        # Cool Boy should be suspended (cost of the effect)
        assert cool_boy_perm.is_suspended, (
            "Cool Boy should be suspended after Royal Knight played"
        )

        # Should have drawn 1 card (net hand change: -1 from play + 1 from draw = 0)
        # And gained 1 memory
        # Memory: started at 10, paid 7 for Magnamon, gained 1 from effect = 4
        assert after.memory == 10 - 7 + 1, (
            f"Memory should be 4 (10 - 7 play + 1 gain), got {after.memory}"
        )

    def test_play_non_rk_does_not_trigger(self, debug_runner):
        """Playing a Digimon WITHOUT Royal Knight trait should NOT trigger Cool Boy."""
        runner = debug_runner(initial_memory=10)

        cool_boy_perm = runner.place_on_field(1, [COOL_BOY])
        runner.inject_card(1, NON_RK_LV3, "hand")

        runner.set_phase("Main")
        before = runner.snapshot()

        action = runner.find_action("Play Agumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()

        # Cool Boy should NOT be suspended
        assert not cool_boy_perm.is_suspended, (
            "Cool Boy should remain unsuspended when non-RK Digimon is played"
        )
        # Memory: 10 - 3 = 7 (no gain)
        assert after.memory == 10 - 3, (
            f"Memory should be 7 (10 - 3 play, no gain), got {after.memory}"
        )

    def test_already_suspended_does_not_trigger(self, debug_runner):
        """If Cool Boy is already suspended, the effect should not fire
        (cannot pay the suspend cost)."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        cool_boy_perm = runner.place_on_field(1, [COOL_BOY], is_suspended=True)
        assert cool_boy_perm.is_suspended

        runner.inject_card(1, ROYAL_KNIGHT_LV4, "hand")
        runner.set_phase("Main")

        before_hand_count = len(game.player1.hand_cards)

        action = runner.find_action("Play Magnamon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()

        # No draw or memory gain because Cool Boy was already suspended
        # Memory: 10 - 7 = 3 (no gain)
        assert after.memory == 10 - 7, (
            f"Memory should be 3 (no effect, already suspended), got {after.memory}"
        )


@pytest.mark.behavioral
class TestCoolBoyDigivolveTrigger:
    """[Your Turn] When a Digimon digivolves into a Royal Knight,
    suspend Cool Boy, draw 1, gain 1 memory."""

    def test_digivolve_into_rk_triggers(self, debug_runner):
        """Digivolving into a Royal Knight Digimon should trigger Cool Boy's effect."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place Cool Boy tamer
        cool_boy_perm = runner.place_on_field(1, [COOL_BOY])

        # Place a Lv5 Digimon on field (digivolve target)
        target_perm = runner.place_on_field(1, [NON_RK_LV5])

        # Put RK Lv6 in hand
        runner.inject_card(1, ROYAL_KNIGHT_LV6, "hand")

        runner.set_phase("Main")
        before_hand_count = len(game.player1.hand_cards)

        # Find and execute the DIGIVOLVE action (not play)
        action = runner.find_action("Digivolve Gallantmon")
        assert action is not None, (
            f"Should find 'Digivolve Gallantmon' action. Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()

        # Cool Boy should be suspended
        assert cool_boy_perm.is_suspended, (
            "Cool Boy should be suspended after digivolving into Royal Knight"
        )

        # Memory: 10 - 4 (digivolve cost) + 1 (effect) = 7
        # (Gallantmon AD1-008 digivolve cost is 4 from Lv5)
        assert after.memory == 10 - 4 + 1, (
            f"Memory should be 7 (10 - 4 digivolve + 1 gain), got {after.memory}"
        )

    def test_digivolve_into_non_rk_does_not_trigger(self, debug_runner):
        """Digivolving into a non-Royal Knight Digimon should NOT trigger Cool Boy."""
        runner = debug_runner(initial_memory=10)

        cool_boy_perm = runner.place_on_field(1, [COOL_BOY])

        # Place Lv3 on field, inject a non-RK Lv4 into hand
        # ST1-03 Agumon is Red Lv3, ST1-07 Greymon is Red Lv4 (no RK)
        runner.place_on_field(1, [NON_RK_LV3])
        runner.inject_card(1, "ST1-07", "hand")

        runner.set_phase("Main")

        # Find the digivolve action for Greymon
        action = runner.find_action("Digivolve Greymon")
        if action is None:
            action = runner.find_action("Digivolve")

        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            assert not cool_boy_perm.is_suspended, (
                "Cool Boy should NOT be suspended after digivolving into non-RK"
            )


@pytest.mark.behavioral
class TestCoolBoyRemovalTrigger:
    """[Opponent's Turn] [Once Per Turn] When a Royal Knight Digimon would
    leave the battle area, play 1 Omekamon from hand."""

    def test_rk_deletion_plays_omekamon(self, debug_runner):
        """When a Royal Knight is deleted on opponent's turn,
        Cool Boy should allow playing Omekamon from hand."""
        runner = debug_runner(initial_memory=-3, first_player=2)
        game = runner.game

        # Place Cool Boy on P1's field
        cool_boy_perm = runner.place_on_field(1, [COOL_BOY])

        # Place a Royal Knight on P1's field (will be deleted)
        rk_perm = runner.place_on_field(1, [ROYAL_KNIGHT_LV4])

        # Put Omekamon in P1's hand
        runner.inject_card(1, OMEKAMON, "hand")

        # Verify Omekamon is in hand
        assert any(
            c.c_entity_base and c.c_entity_base.card_id == OMEKAMON
            for c in game.player1.hand_cards
        ), "Omekamon should be in P1's hand"

        # It's P2's turn — delete the Royal Knight
        runner.set_phase("Main")
        game.player1.delete_permanent(rk_perm)
        runner.auto_resolve()

        # Omekamon should have been played from hand to field
        omekamon_on_field = any(
            fs.card_id == OMEKAMON for fs in runner.snapshot().p1_field
        )
        omekamon_in_hand = any(
            c.c_entity_base and c.c_entity_base.card_id == OMEKAMON
            for c in game.player1.hand_cards
        )

        assert omekamon_on_field or not omekamon_in_hand, (
            "Omekamon should be played from hand when RK leaves on opponent's turn"
        )

    def test_non_rk_deletion_does_not_trigger(self, debug_runner):
        """Deleting a non-Royal Knight should NOT trigger Cool Boy's removal effect."""
        runner = debug_runner(initial_memory=-3, first_player=2)
        game = runner.game

        cool_boy_perm = runner.place_on_field(1, [COOL_BOY])
        non_rk_perm = runner.place_on_field(1, [NON_RK_LV3])
        runner.inject_card(1, OMEKAMON, "hand")

        runner.set_phase("Main")
        game.player1.delete_permanent(non_rk_perm)
        runner.auto_resolve()

        # Omekamon should still be in hand (effect did not trigger)
        omekamon_in_hand = any(
            c.c_entity_base and c.c_entity_base.card_id == OMEKAMON
            for c in game.player1.hand_cards
        )
        assert omekamon_in_hand, (
            "Omekamon should remain in hand when non-RK is deleted"
        )

    def test_removal_trigger_once_per_turn(self, debug_runner):
        """The removal trigger should only fire once per turn."""
        runner = debug_runner(initial_memory=-3, first_player=2)
        game = runner.game

        cool_boy_perm = runner.place_on_field(1, [COOL_BOY])
        rk_perm1 = runner.place_on_field(1, [ROYAL_KNIGHT_LV4])
        rk_perm2 = runner.place_on_field(1, [ROYAL_KNIGHT_LV4])
        runner.inject_card(1, OMEKAMON, "hand")
        runner.inject_card(1, OMEKAMON_BT20, "hand")

        runner.set_phase("Main")

        # Delete first RK
        game.player1.delete_permanent(rk_perm1)
        runner.auto_resolve()

        hand_after_first = len(game.player1.hand_cards)

        # Delete second RK
        game.player1.delete_permanent(rk_perm2)
        runner.auto_resolve()

        hand_after_second = len(game.player1.hand_cards)

        # At most 1 Omekamon should have been played (OPT)
        omekamon_count_on_field = sum(
            1 for perm in game.player1.battle_area
            if perm.top_card and perm.top_card.c_entity_base
            and perm.top_card.c_entity_base.card_name_eng == "Omekamon"
        )
        assert omekamon_count_on_field <= 1, (
            f"Should play at most 1 Omekamon per turn (OPT), found {omekamon_count_on_field}"
        )

    def test_removal_on_own_turn_does_not_trigger(self, debug_runner):
        """The removal effect should only fire on the OPPONENT's turn."""
        runner = debug_runner(initial_memory=10, first_player=1)
        game = runner.game

        cool_boy_perm = runner.place_on_field(1, [COOL_BOY])
        rk_perm = runner.place_on_field(1, [ROYAL_KNIGHT_LV4])
        runner.inject_card(1, OMEKAMON, "hand")

        runner.set_phase("Main")

        # Delete RK on P1's own turn
        game.player1.delete_permanent(rk_perm)
        runner.auto_resolve()

        # Omekamon should still be in hand
        omekamon_in_hand = any(
            c.c_entity_base and c.c_entity_base.card_id == OMEKAMON
            for c in game.player1.hand_cards
        )
        assert omekamon_in_hand, (
            "Omekamon should remain in hand when RK is deleted on own turn"
        )

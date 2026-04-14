"""Behavioral tests for BT22-036 Chaperomon.

Card text:
[Hand] [Main] If you have [Arisa Kinosaki], by placing 1 [ShoeShoemon]
from your trash as any of your [Shoemon]'s bottom digivolution card,
it digivolves into this card for a digivolution cost of 3, ignoring
digivolution requirements.
<Overclock ([Puppet] Trait)>

Inherited:
[All Turns] [Once Per Turn] When this Digimon would leave the battle area
other than by your effects, by deleting 1 of your Tokens or other [Puppet]
trait Digimon, it doesn't leave.
"""

import pytest


def _find_hand_main_action(runner, card_id="BT22-036"):
    """Find [Hand][Main] action for a card (action range 30-59)."""
    game = runner.game
    player = game.player1
    for h, card in enumerate(player.hand_cards):
        if card.card_id == card_id:
            action_id = 30 + h
            mask = runner.action_mask()
            if action_id in mask:
                return action_id
    return None


@pytest.mark.behavioral
class TestBT22036HandMain:
    """Tests for Effect 1: [Hand][Main] digivolve Shoemon into Chaperomon."""

    def test_hand_main_available_with_arisa_shoemon_and_trash(self, debug_runner):
        """[Hand][Main] action should be available when Arisa is on field,
        Shoemon is on field, and ShoeShoemon is in trash."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)

        runner.place_on_field(1, ["BT22-088"])  # Arisa Kinosaki tamer
        runner.place_on_field(1, ["BT22-029"])  # Shoemon Lv.3 Digimon
        runner.inject_card(1, "BT22-032", "trash")  # ShoeShoemon in trash
        runner.inject_card(1, "BT22-036", "hand")  # Chaperomon in hand
        runner.set_memory(10)
        runner.set_phase("Main")

        action = _find_hand_main_action(runner)
        assert action is not None, (
            f"[Hand][Main] action should be available. Legal actions: {runner.actions()}"
        )

    def test_hand_main_not_available_without_arisa(self, debug_runner):
        """[Hand][Main] should NOT be available without Arisa Kinosaki."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)

        runner.place_on_field(1, ["BT22-029"])  # Shoemon
        runner.inject_card(1, "BT22-032", "trash")  # ShoeShoemon in trash
        runner.inject_card(1, "BT22-036", "hand")  # Chaperomon in hand
        runner.set_memory(10)
        runner.set_phase("Main")

        action = _find_hand_main_action(runner)
        assert action is None, (
            "[Hand][Main] should NOT be available without Arisa Kinosaki"
        )

    def test_hand_main_not_available_without_shoemon_on_field(self, debug_runner):
        """[Hand][Main] should NOT be available without Shoemon on field."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)

        runner.place_on_field(1, ["BT22-088"])  # Arisa
        runner.inject_card(1, "BT22-032", "trash")  # ShoeShoemon in trash
        runner.inject_card(1, "BT22-036", "hand")  # Chaperomon in hand
        runner.set_memory(10)
        runner.set_phase("Main")

        action = _find_hand_main_action(runner)
        assert action is None, (
            "[Hand][Main] should NOT be available without Shoemon on field"
        )

    def test_hand_main_not_available_without_shoeshoemon_in_trash(self, debug_runner):
        """[Hand][Main] should NOT be available without ShoeShoemon in trash."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)

        runner.place_on_field(1, ["BT22-088"])  # Arisa
        runner.place_on_field(1, ["BT22-029"])  # Shoemon
        runner.inject_card(1, "BT22-036", "hand")  # Chaperomon in hand
        runner.set_memory(10)
        runner.set_phase("Main")

        action = _find_hand_main_action(runner)
        assert action is None, (
            "[Hand][Main] should NOT be available without ShoeShoemon in trash"
        )

    def test_hand_main_digivolve_costs_3_memory(self, debug_runner):
        """Executing the [Hand][Main] effect should cost 3 memory."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)

        runner.place_on_field(1, ["BT22-088"])  # Arisa
        runner.place_on_field(1, ["BT22-029"])  # Shoemon
        runner.inject_card(1, "BT22-032", "trash")  # ShoeShoemon in trash
        runner.inject_card(1, "BT22-036", "hand")  # Chaperomon in hand
        runner.set_memory(10)
        runner.set_phase("Main")

        action = _find_hand_main_action(runner)
        assert action is not None, f"No hand-main action found. Available: {runner.actions()}"

        before = runner.snapshot()
        runner.execute(action)
        runner.auto_resolve()
        after = runner.snapshot()

        memory_paid = before.memory - after.memory
        assert memory_paid == 3, (
            f"Expected to pay 3 memory, but paid {memory_paid}"
        )

    def test_hand_main_places_shoeshoemon_under_shoemon(self, debug_runner):
        """ShoeShoemon should be placed from trash as bottom digivolution card,
        and Chaperomon digivolves on top of Shoemon."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)

        runner.place_on_field(1, ["BT22-088"])  # Arisa
        runner.place_on_field(1, ["BT22-029"])  # Shoemon
        runner.inject_card(1, "BT22-032", "trash")  # ShoeShoemon in trash
        runner.inject_card(1, "BT22-036", "hand")  # Chaperomon in hand
        runner.set_memory(10)
        runner.set_phase("Main")

        action = _find_hand_main_action(runner)
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # After resolving, Shoemon permanent should now be topped by Chaperomon
        # with ShoeShoemon in its digivolution cards
        game = runner.game
        p1 = game.player1
        chaperomon_perm = None
        for perm in p1.battle_area:
            if perm.top_card and perm.top_card.card_id == "BT22-036":
                chaperomon_perm = perm
                break

        assert chaperomon_perm is not None, "Chaperomon should be on field as top card"
        stack_ids = [cs.card_id for cs in chaperomon_perm.card_sources]
        assert "BT22-032" in stack_ids, (
            f"ShoeShoemon (BT22-032) should be in digivolution stack. Stack: {stack_ids}"
        )
        assert "BT22-029" in stack_ids, (
            f"Shoemon (BT22-029) should be in digivolution stack. Stack: {stack_ids}"
        )

    def test_shoeshoemon_on_field_does_not_match_shoemon_filter(self, debug_runner):
        """A ShoeShoemon on field should NOT satisfy the Shoemon field requirement.
        C# uses EqualsCardName('Shoemon') which is exact match."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)

        runner.place_on_field(1, ["BT22-088"])  # Arisa
        runner.place_on_field(1, ["BT22-032"])  # ShoeShoemon on field (NOT Shoemon)
        runner.inject_card(1, "BT22-032", "trash")  # ShoeShoemon in trash
        runner.inject_card(1, "BT22-036", "hand")  # Chaperomon in hand
        runner.set_memory(10)
        runner.set_phase("Main")

        action = _find_hand_main_action(runner)
        assert action is None, (
            "ShoeShoemon on field should NOT count as 'Shoemon' for the hand-main condition"
        )


@pytest.mark.behavioral
class TestBT22036Inherited:
    """Tests for inherited effect: prevent leaving by deleting Token/Puppet.

    The inherited effect only activates when BT22-036 is a digivolution source
    (not the top card). We stack a Lv.6 on top so the inherited fires.
    """

    def test_prevent_leaving_by_opponent_effect(self, debug_runner):
        """When opponent deletes this Digimon, it can sacrifice a Puppet to survive."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)

        # Stack: Shoemon(3) -> Chaperomon(5) -> WarGreymon(6) on top
        # BT22-036 as inherited source, ST1-11 on top
        target_perm = runner.place_on_field(1, ["BT22-029", "BT22-036", "ST1-11"])
        # Place another Puppet Digimon to sacrifice
        runner.place_on_field(1, ["BT22-029"])  # Shoemon (Puppet trait)

        game = runner.game
        p1 = game.player1

        # Opponent deletes the target via effect
        p1.delete_permanent(target_perm, is_opponent_effect=True)
        runner.auto_resolve()

        # Target should still be on field (saved by sacrificing Puppet)
        assert target_perm in p1.battle_area, (
            "Digimon with Chaperomon inherited should survive after sacrificing a Puppet"
        )

    def test_prevent_leaving_not_triggered_by_own_effect(self, debug_runner):
        """When player's OWN effect removes this Digimon, the inherited should NOT trigger."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)

        target_perm = runner.place_on_field(1, ["BT22-029", "BT22-036", "ST1-11"])
        runner.place_on_field(1, ["BT22-029"])  # Puppet sacrifice available

        game = runner.game
        p1 = game.player1

        # Own effect deletion (is_opponent_effect=False by default)
        p1.delete_permanent(target_perm, is_opponent_effect=False)

        # Should NOT trigger prevent-leaving; target should be deleted
        assert target_perm not in p1.battle_area, (
            "Digimon should NOT be saved when removed by own effect"
        )

    def test_prevent_leaving_no_puppet_available(self, debug_runner):
        """If no Token/Puppet is available to sacrifice, the effect cannot trigger."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)

        target_perm = runner.place_on_field(1, ["BT22-029", "BT22-036", "ST1-11"])
        # Place a non-Puppet Digimon (no valid sacrifice target)
        runner.place_on_field(1, ["ST1-03"])  # Agumon - not Puppet trait

        game = runner.game
        p1 = game.player1

        p1.delete_permanent(target_perm, is_opponent_effect=True)

        # Without a Puppet sacrifice, should be deleted normally
        assert target_perm not in p1.battle_area, (
            "Without a Puppet sacrifice, Digimon should be deleted"
        )

    def test_prevent_leaving_once_per_turn(self, debug_runner):
        """The inherited is [Once Per Turn] -- second deletion in same turn should not be prevented."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)

        target_perm = runner.place_on_field(1, ["BT22-029", "BT22-036", "ST1-11"])
        runner.place_on_field(1, ["BT22-029"])  # Puppet sacrifice 1
        runner.place_on_field(1, ["BT22-029"])  # Puppet sacrifice 2

        game = runner.game
        p1 = game.player1

        # First deletion by opponent effect -- should be prevented
        p1.delete_permanent(target_perm, is_opponent_effect=True)
        runner.auto_resolve()
        assert target_perm in p1.battle_area, "First deletion should be prevented"

        # Second deletion by opponent effect -- OPT exhausted, should NOT be prevented
        p1.delete_permanent(target_perm, is_opponent_effect=True)
        runner.auto_resolve()
        assert target_perm not in p1.battle_area, (
            "Second deletion in same turn should NOT be prevented (Once Per Turn)"
        )

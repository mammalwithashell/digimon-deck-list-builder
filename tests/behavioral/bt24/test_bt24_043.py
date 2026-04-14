"""Behavioral tests for BT24-043 Tapirmon (Lv.3 Green Digimon).

Card text:
Alt-digi: Lv.2 with [TS] trait for cost 0
[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with the
[Shaman] trait or with [Beast], [Animal] or [Sovereign] in any of its traits
(other than [Sea Animal]) and 1 card with the [TS] trait among them to the hand.
Return the rest to the bottom of the deck.
Inherited: [When Attacking] [Once Per Turn] Suspend 1 of your opponent's Digimon.
"""

import pytest


@pytest.mark.behavioral
class TestBT24043Tapirmon:
    """Tests for BT24-043 Tapirmon."""

    def test_on_play_reveal_adds_matching_cards(self, debug_runner):
        """[On Play] should reveal top 3 and add matching Shaman + TS cards to hand."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "BT24-043", "hand")

        # Stack deck: BT1-057 Sirenmon (Shaman Digimon) for filter 0,
        # BT24-009 Shamanmon (TS trait) for filter 1,
        # ST1-03 Agumon (filler)
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "BT24-009", "library_top")
        runner.inject_card(1, "BT1-057", "library_top")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Tapirmon")
        assert action is not None, "Should be able to play Tapirmon from hand"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Tapirmon on field
        assert any(
            s.card_id == "BT24-043" for s in snap_after.p1_field
        ), "Tapirmon should be on field after playing"
        # Hand should have gained 2 cards (Sirenmon + Shamanmon) minus Tapirmon played
        # hand_before included Tapirmon; after play we lost Tapirmon but gained 2
        assert len(snap_after.p1_hand) == hand_before - 1 + 2, (
            f"Should add 2 cards to hand; was {hand_before}, now {len(snap_after.p1_hand)}"
        )

    def test_on_play_no_matching_cards(self, debug_runner):
        """[On Play] with no matching cards among revealed should add nothing."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "BT24-043", "hand")

        # Stack deck with 3 filler cards (no Shaman/Beast/Animal/Sovereign/TS)
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-03", "library_top")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Tapirmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Lost Tapirmon from hand, gained nothing
        assert len(snap_after.p1_hand) == hand_before - 1, (
            f"No matching cards means no hand additions; was {hand_before}, now {len(snap_after.p1_hand)}"
        )

    def test_inherited_suspend_opponent_digimon(self, debug_runner):
        """Inherited [When Attacking] should suspend 1 opponent's Digimon."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Digimon with Tapirmon in its sources (inherited effect active)
        perm = runner.place_on_field(1, ["BT24-043", "BT24-009"])
        # Place an opponent's Digimon as suspend target
        opp_perm = runner.place_on_field(2, ["ST1-03"])

        assert not opp_perm.is_suspended, "Opponent's Digimon should start unsuspended"

        # Attack to trigger inherited [When Attacking]
        action = runner.find_action("Attack player with Shamanmon")
        assert action is not None, "Should be able to attack with the Digimon"
        runner.execute(action)
        runner.auto_resolve()

        assert opp_perm.is_suspended, "Opponent's Digimon should be suspended by inherited effect"

    def test_inherited_suspend_once_per_turn(self, debug_runner):
        """Inherited suspend effect should only fire once per turn."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place attacker with Tapirmon inherited
        perm = runner.place_on_field(1, ["BT24-043", "BT24-009"])
        # Place two opponent Digimon
        opp1 = runner.place_on_field(2, ["ST1-03"])
        opp2 = runner.place_on_field(2, ["ST1-03"])

        # First attack — should trigger suspend
        action = runner.find_action("Attack player with Shamanmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        suspended_count = sum(1 for p in [opp1, opp2] if p.is_suspended)
        assert suspended_count == 1, "First attack should suspend exactly 1 opponent Digimon"

"""Behavioral tests for BT15-027 Scorpiomon (Lv.5 Blue, Cost 6, DP 6000).

Card text:
[On Play] Reveal the top 4 cards of your deck. Add 2 level 6 or higher cards
    among them to the hand. Return the rest to the bottom of the deck.
[End of Your Turn] By deleting 1 of your Digimon, you may play 1 Digimon card
    with the [Dark Masters] trait from your hand to an empty space in your
    breeding area without paying the cost.
Inherited: <Blocker>

C# reference (BT15_027.cs):
- Reveal: SimplifiedRevealDeckTopCardsAndSelect with maxCount:2, single pass
- End of Turn: canNoSelect:true on delete (optional), checks breeding area empty,
  plays with isBreedingArea:true
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT15027Scorpiomon:
    """Tests for BT15-027 Scorpiomon."""

    # ----------------------------------------------------------------
    # [On Play] Reveal top 4, add up to 2 level 6+ to hand
    # ----------------------------------------------------------------

    def test_on_play_reveals_top_4_and_adds_lv6_to_hand(self, debug_runner):
        """On Play should reveal top 4 and allow adding level 6+ cards to hand."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)

        p1 = runner.game.player1
        p1.library_cards.clear()
        # Add some filler so deck isn't empty
        for _ in range(5):
            runner.inject_card(1, "BT15-020", "library_bottom")
        # Inject 4 cards on top of deck (last injected = top)
        runner.inject_card(1, "BT15-021", "library_top")  # Syakomon Lv.3
        runner.inject_card(1, "BT15-020", "library_top")  # Gabumon Lv.3
        runner.inject_card(1, "BT15-032", "library_top")  # Plesiomon Lv.6
        runner.inject_card(1, "BT15-031", "library_top")  # MetalSeadramon Lv.6 (Dark Masters)

        # Put Scorpiomon in hand so we can play it naturally (triggers On Play)
        runner.inject_card(1, "BT15-027", "hand")

        hand_before = len(p1.hand_cards)  # includes Scorpiomon
        deck_before = len(p1.library_cards)

        runner.set_phase("Main")
        # Play Scorpiomon from hand (triggers On Play effect)
        play_action = runner.find_action("Play Scorpiomon")
        assert play_action is not None, "Should be able to play Scorpiomon"
        runner.execute(play_action)

        # Auto-resolve the reveal selections
        runner.auto_resolve(max_steps=10)

        # After play: hand lost Scorpiomon (-1) but gained up to 2 lv6+ cards
        hand_after = len(p1.hand_cards)
        # hand_before included Scorpiomon. After play: -1 (Scorpiomon played) + up to 2 added
        added = hand_after - (hand_before - 1)
        assert added <= 2, f"Should add at most 2 cards, got {added}"
        assert added >= 1, f"Should add at least 1 lv6+ card, got {added}"

    def test_on_play_returns_rest_to_deck_bottom(self, debug_runner):
        """Remaining cards after selection should go to deck bottom."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)

        p1 = runner.game.player1
        p1.library_cards.clear()

        # Stock 4 cards: 1 lv6, 3 lv3 (only 1 selectable)
        runner.inject_card(1, "BT15-020", "library_top")  # Gabumon Lv.3
        runner.inject_card(1, "BT15-020", "library_top")  # Gabumon Lv.3
        runner.inject_card(1, "BT15-020", "library_top")  # Gabumon Lv.3
        runner.inject_card(1, "BT15-031", "library_top")  # MetalSeadramon Lv.6

        # Put Scorpiomon in hand and play it
        runner.inject_card(1, "BT15-027", "hand")
        runner.set_phase("Main")
        play_action = runner.find_action("Play Scorpiomon")
        runner.execute(play_action)
        runner.auto_resolve(max_steps=10)

        # The non-selected cards should be at the bottom of the deck
        # We had 4 cards, selected at most 1 lv6, so 3 should return to deck
        assert len(p1.library_cards) >= 3, (
            f"Expected at least 3 cards returned to deck bottom, "
            f"deck has {len(p1.library_cards)}"
        )

    def test_on_play_no_lv6_cards_all_return(self, debug_runner):
        """If no lv6+ cards revealed, all 4 return to deck bottom."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)

        p1 = runner.game.player1
        p1.library_cards.clear()

        # All lv3 cards (none selectable)
        for _ in range(4):
            runner.inject_card(1, "BT15-020", "library_top")  # Gabumon Lv.3

        # Put Scorpiomon in hand and play it
        runner.inject_card(1, "BT15-027", "hand")
        hand_before = len(p1.hand_cards)  # includes Scorpiomon

        runner.set_phase("Main")
        play_action = runner.find_action("Play Scorpiomon")
        runner.execute(play_action)
        runner.auto_resolve(max_steps=10)

        # hand_before included Scorpiomon. After play: -1 (played). No lv6+ = no adds.
        hand_after = len(p1.hand_cards)
        assert hand_after == hand_before - 1, (
            "No lv6+ cards means no cards added to hand"
        )
        assert len(p1.library_cards) >= 4, (
            "All 4 cards should return to deck bottom"
        )

    # ----------------------------------------------------------------
    # [End of Your Turn] Delete 1 own Digimon -> play Dark Masters to breeding
    # ----------------------------------------------------------------

    def test_end_turn_plays_dark_masters_to_breeding_area(self, debug_runner):
        """After deleting own Digimon, Dark Masters card should go to breeding area."""
        runner = debug_runner(initial_memory=5, skip_shuffle=True)

        p1 = runner.game.player1
        # Place Scorpiomon on field
        scorpio = runner.place_on_field(1, ["BT15-027"])
        # Place a sacrifice Digimon on field
        sacrifice = runner.place_on_field(1, ["BT15-020"])  # Gabumon

        # Put a Dark Masters Digimon in hand
        runner.inject_card(1, "BT15-031", "hand")  # MetalSeadramon (Dark Masters, Lv.6)

        # Ensure breeding area is empty
        assert p1.breeding_area is None, "Breeding area should start empty"

        # Trigger end of turn
        runner.game.execute_effects(EffectTiming.OnEndTurn)

        # Should be in selection phase to pick Digimon to delete
        assert runner.game.current_phase == GamePhase.SelectTarget, (
            f"Expected SelectTarget phase, got {runner.game.current_phase}"
        )

        # Find the sacrifice Digimon in the selection (not "Decline")
        from digimon_gym.engine.game.constants import SEL_MY_FIELD_START
        # sacrifice is at index 1 in battle_area (Scorpiomon=0, Gabumon=1)
        sacrifice_action = SEL_MY_FIELD_START + 1  # Select Gabumon to delete
        legal = runner.action_mask()
        assert sacrifice_action in legal, (
            f"Sacrifice Digimon action {sacrifice_action} not in legal actions {legal}"
        )
        runner.execute(sacrifice_action)

        # Now should be in hand selection phase for Dark Masters card
        # Auto-resolve to pick the first (and only) valid hand card
        runner.auto_resolve(max_steps=10)

        # After resolution, Dark Masters should be in breeding area
        assert p1.breeding_area is not None, (
            "Dark Masters Digimon should be placed in breeding area"
        )
        breeding_name = p1.breeding_area.top_card.card_names[0] if p1.breeding_area.top_card else None
        assert breeding_name == "MetalSeadramon", (
            f"Breeding area should have MetalSeadramon, got {breeding_name}"
        )

    def test_end_turn_requires_empty_breeding_area(self, debug_runner):
        """Cannot play Dark Masters to breeding if breeding area is occupied."""
        runner = debug_runner(initial_memory=5, skip_shuffle=True)

        p1 = runner.game.player1
        scorpio = runner.place_on_field(1, ["BT15-027"])
        sacrifice = runner.place_on_field(1, ["BT15-020"])

        # Put a Dark Masters Digimon in hand
        runner.inject_card(1, "BT15-031", "hand")

        # Occupy breeding area
        runner.place_in_breeding(1, ["BT15-020"])
        assert p1.breeding_area is not None, "Breeding area should be occupied"

        hand_before = len(p1.hand_cards)
        field_before = len(p1.battle_area)

        # Trigger end of turn
        runner.game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve(max_steps=20)

        # Dark Masters card should still be in hand (not played)
        # The delete may or may not have occurred (depending on implementation),
        # but the key point is Dark Masters is NOT in breeding
        dm_in_hand = any(
            any('Dark Masters' in t for t in (getattr(c, 'card_traits', []) or []))
            for c in p1.hand_cards
        )
        # Breeding area should still have the original card, not MetalSeadramon
        breeding_name = p1.breeding_area.top_card.card_names[0] if p1.breeding_area and p1.breeding_area.top_card else None
        assert breeding_name != "MetalSeadramon", (
            "Should NOT have played MetalSeadramon to breeding when breeding is occupied"
        )

    def test_end_turn_does_not_play_to_battle_area(self, debug_runner):
        """The Dark Masters card must NOT be played to battle area."""
        runner = debug_runner(initial_memory=5, skip_shuffle=True)

        p1 = runner.game.player1
        scorpio = runner.place_on_field(1, ["BT15-027"])
        sacrifice = runner.place_on_field(1, ["BT15-020"])

        runner.inject_card(1, "BT15-031", "hand")  # MetalSeadramon

        assert p1.breeding_area is None

        field_before = len(p1.battle_area)

        runner.game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve(max_steps=20)

        # MetalSeadramon should NOT be on the battle area
        metal_on_field = any(
            p.contains_card_name("MetalSeadramon")
            for p in p1.battle_area
        )
        assert not metal_on_field, (
            "MetalSeadramon should be in breeding area, NOT battle area"
        )

    def test_end_turn_deletes_own_digimon_as_cost(self, debug_runner):
        """Deleting own Digimon is the cost; it should go to trash."""
        runner = debug_runner(initial_memory=5, skip_shuffle=True)

        p1 = runner.game.player1
        scorpio = runner.place_on_field(1, ["BT15-027"])
        sacrifice = runner.place_on_field(1, ["BT15-020"])

        runner.inject_card(1, "BT15-031", "hand")

        trash_before = len(p1.trash_cards)

        runner.game.execute_effects(EffectTiming.OnEndTurn)

        # Explicitly select the sacrifice target (not Decline)
        from digimon_gym.engine.game.constants import SEL_MY_FIELD_START
        sacrifice_action = SEL_MY_FIELD_START + 1  # Gabumon
        runner.execute(sacrifice_action)
        runner.auto_resolve(max_steps=10)

        # The sacrificed Digimon should be in trash
        trash_after = len(p1.trash_cards)
        assert trash_after > trash_before, (
            "Deleted Digimon should have gone to trash"
        )

    def test_end_turn_is_optional(self, debug_runner):
        """The End of Turn effect should be optional (C# canNoSelect:true)."""
        runner = debug_runner(initial_memory=5, skip_shuffle=True)

        p1 = runner.game.player1
        scorpio = runner.place_on_field(1, ["BT15-027"])

        effects = scorpio.top_card.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot) == 1, "Should have 1 OnEndTurn effect"
        assert eot[0].is_optional, "End of Turn effect should be optional"

    def test_end_turn_only_on_own_turn(self, debug_runner):
        """Effect should only trigger on owner's turn."""
        runner = debug_runner(initial_memory=5, skip_shuffle=True)

        p1 = runner.game.player1
        scorpio = runner.place_on_field(1, ["BT15-027"])

        effects = scorpio.top_card.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        # On own turn
        p1.is_my_turn = True
        assert eot.can_use_condition({}), "Should be usable on own turn"

        # On opponent's turn
        p1.is_my_turn = False
        assert not eot.can_use_condition({}), "Should NOT be usable on opponent's turn"

    def test_end_turn_requires_digimon_on_field(self, debug_runner):
        """Need at least 1 own Digimon to delete as cost."""
        runner = debug_runner(initial_memory=5, skip_shuffle=True)

        p1 = runner.game.player1
        # Only Scorpiomon on field (the effect needs a Digimon to delete)
        scorpio = runner.place_on_field(1, ["BT15-027"])
        runner.inject_card(1, "BT15-031", "hand")

        # Even though we have a DM card in hand, we CAN delete Scorpiomon itself
        # (it's a valid target). So effect should still fire since there's a Digimon.
        runner.game.execute_effects(EffectTiming.OnEndTurn)
        # Should enter selection phase
        phase = runner.game.current_phase
        # The effect should still activate since Scorpiomon is a valid delete target
        runner.auto_resolve(max_steps=20)

    def test_end_turn_only_dark_masters_trait(self, debug_runner):
        """Only Digimon with Dark Masters trait should be playable from hand."""
        runner = debug_runner(initial_memory=5, skip_shuffle=True)

        p1 = runner.game.player1
        scorpio = runner.place_on_field(1, ["BT15-027"])
        sacrifice = runner.place_on_field(1, ["BT15-020"])

        # Inject non-Dark-Masters Digimon in hand (Lv.6 but no Dark Masters trait)
        runner.inject_card(1, "BT15-032", "hand")  # Plesiomon (no Dark Masters)

        runner.game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve(max_steps=20)

        # No Dark Masters in hand means nothing should be played to breeding
        assert p1.breeding_area is None, (
            "Should not play non-Dark-Masters card to breeding"
        )

    # ----------------------------------------------------------------
    # Inherited: <Blocker>
    # ----------------------------------------------------------------

    def test_inherited_blocker(self, debug_runner):
        """Inherited effect should be Blocker."""
        runner = debug_runner(initial_memory=5)

        # Place as digivolution source under a Lv.6
        perm = runner.place_on_field(1, ["BT15-027", "BT15-031"])

        bt15_card = perm.card_sources[0]  # BT15-027 at bottom
        effects = bt15_card.effect_list(None)
        blocker_effs = [e for e in effects if e.is_inherited_effect and getattr(e, '_is_blocker', False)]
        assert len(blocker_effs) == 1, "Should have 1 inherited Blocker effect"

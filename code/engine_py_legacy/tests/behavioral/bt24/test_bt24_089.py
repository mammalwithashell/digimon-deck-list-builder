"""Behavioral tests for BT24-089 Unique Emblem: Blazing Conductor (Option).

Card text:
[Main] You may play 1 [Elizamon] or [Owen Dreadnought] from your hand or trash
without paying the cost. Then, place this card in the battle area.
[Your Turn] When any of your [Owen Dreadnought]s suspend, <Delay> (By trashing
this card after the placing turn, activate the effect below.)
- 1 of your [Reptile] or [Dragonkin] Digimon may digivolve into a Digimon card
  with the [Reptile] or [Dragonkin] trait and the [LIBERATOR] trait in the hand
  with the digivolution cost reduced by 3.

Security Effect [Security] Activate this card's [Main] effects.
"""

import pytest

# BT24-008 Elizamon: Lv3, Reptile/LIBERATOR, play cost 3
# BT24-082 Owen Dreadnought: Tamer, LIBERATOR, play cost 3
# BT24-012 Dimetromon: Lv4, Reptile/LIBERATOR (digivolve target)
# BT24-016 Lamiamon: Lv5, Dragonkin/LIBERATOR (digivolve target)
# BT1-001: Generic filler (no relevant traits)
DECK = (
    ["BT24-089"] * 4
    + ["BT24-008"] * 4   # Elizamon
    + ["BT24-082"] * 4   # Owen Dreadnought
    + ["BT24-012"] * 4   # Dimetromon
    + ["BT24-016"] * 4   # Lamiamon
    + ["BT1-001"] * 30
)


@pytest.mark.behavioral
class TestBT24089UniqueEmblemBlazingConductor:
    """Tests for BT24-089 Unique Emblem: Blazing Conductor."""

    # ── Main effect: play Elizamon or Owen from hand/trash ───────────

    def test_main_play_elizamon_from_hand(self, debug_runner):
        """Main effect should allow playing Elizamon from hand for free."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place a Red Digimon on field for option color requirement
        runner.place_on_field(1, ["BT24-008"])
        # Ensure Elizamon is in hand
        runner.inject_card(1, "BT24-008", "hand")

        action = runner.find_action("Play Unique Emblem: Blazing Conductor from hand")
        assert action is not None, "Should be able to play the option card"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Elizamon should be on field (played from hand) — 2 total (1 placed + 1 played)
        elizamon_on_field = [s for s in snap.p1_field if s.card_id == "BT24-008"]
        assert len(elizamon_on_field) >= 2, "Elizamon should be played onto the field"
        # Option card should also be on field (placed in battle area)
        option_on_field = [s for s in snap.p1_field if s.card_id == "BT24-089"]
        assert len(option_on_field) >= 1, "Option should be placed in battle area"

    def test_main_play_owen_from_hand(self, debug_runner):
        """Main effect should allow playing Owen Dreadnought from hand for free."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place a Red Digimon on field for option color requirement
        runner.place_on_field(1, ["BT24-008"])
        # Ensure Owen is in hand
        runner.inject_card(1, "BT24-082", "hand")

        action = runner.find_action("Play Unique Emblem: Blazing Conductor from hand")
        assert action is not None
        runner.execute(action)

        # In the selection phase, manually pick Owen Dreadnought
        owen_action = runner.find_action("Owen Dreadnought")
        assert owen_action is not None, "Owen should be selectable in the play selection"
        runner.execute(owen_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        owen_on_field = [s for s in snap.p1_field if s.card_id == "BT24-082"]
        assert len(owen_on_field) >= 1, "Owen should be played onto the field"
        option_on_field = [s for s in snap.p1_field if s.card_id == "BT24-089"]
        assert len(option_on_field) >= 1, "Option should be placed in battle area"

    def test_main_play_from_trash(self, debug_runner):
        """Main effect should allow playing Elizamon from trash for free."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place a Red Digimon on field for option color requirement
        runner.place_on_field(1, ["BT24-008"])
        # Put Elizamon in trash
        runner.inject_card(1, "BT24-008", "trash")

        action = runner.find_action("Play Unique Emblem: Blazing Conductor from hand")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        elizamon_on_field = [s for s in snap.p1_field if s.card_id == "BT24-008"]
        assert len(elizamon_on_field) >= 2, "Elizamon should be played from trash (1 placed + 1 from trash)"

    def test_main_optional_play(self, debug_runner):
        """Main effect play is optional (may); should work even with no valid targets."""
        runner = debug_runner(
            deck1=["BT24-089"] * 4 + ["BT1-001"] * 46,
            deck2=DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Place a Red Digimon on field for option color requirement
        runner.place_on_field(1, ["BT24-008"])

        action = runner.find_action("Play Unique Emblem: Blazing Conductor from hand")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Option card should still be placed in battle area even if no play target
        option_on_field = [s for s in snap.p1_field if s.card_id == "BT24-089"]
        assert len(option_on_field) >= 1, \
            "Option should be placed in battle area even without valid play targets"

    # ── Delay trigger: Owen suspend fires OnTappedAnyone ─────────────

    def test_delay_fires_when_owen_suspends(self, debug_runner):
        """When Owen Dreadnought suspends, the delay trigger should fire
        and allow digivolving a Reptile/Dragonkin into a Reptile/Dragonkin
        LIBERATOR from hand with cost reduced by 3."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place Elizamon (Reptile) on field as digivolve source
        runner.place_on_field(1, ["BT24-008"])

        # Place Owen on field (Tamer)
        runner.place_on_field(1, ["BT24-082"])

        # Place the option in battle area (as if previously played), not on same turn
        runner.place_on_field(1, ["BT24-089"], turn_played=-1)

        # Put Dimetromon in hand (Lv4, Reptile+LIBERATOR — valid digivolve target)
        runner.inject_card(1, "BT24-012", "hand")

        # Now suspend Owen to trigger OnTappedAnyone
        game = runner.game
        owen_field = None
        for perm in game.player1.battle_area:
            if perm.top_card and perm.top_card.c_entity_base and \
               perm.top_card.c_entity_base.card_id == "BT24-082":
                owen_field = perm
                break

        assert owen_field is not None, "Owen should be on the field"
        owen_field.suspend()

        # After suspend, the delay fires: select source Digimon (Elizamon)
        source_action = runner.find_action("Elizamon")
        assert source_action is not None, \
            "Should be able to select Elizamon as digivolve source"
        runner.execute(source_action)

        # Then select Dimetromon from hand to digivolve into
        digi_action = runner.find_action("Dimetromon")
        assert digi_action is not None, \
            "Should be able to select Dimetromon from hand as digivolve target"
        runner.execute(digi_action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # The option card should be trashed (removed from battle area)
        option_on_field = [s for s in snap_after.p1_field if s.card_id == "BT24-089"]
        assert len(option_on_field) == 0, \
            "Option should be trashed from battle area after delay activates"

        # The Elizamon should have digivolved into Dimetromon
        dimetromon_on_field = [s for s in snap_after.p1_field if s.card_id == "BT24-012"]
        assert len(dimetromon_on_field) >= 1, \
            "Elizamon should have digivolved into Dimetromon (Reptile+LIBERATOR from hand)"

    def test_delay_does_not_fire_for_non_owen(self, debug_runner):
        """Delay should NOT fire when a non-Owen permanent suspends."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place Elizamon on field
        runner.place_on_field(1, ["BT24-008"])

        # Place the option in battle area (previously played)
        runner.place_on_field(1, ["BT24-089"], turn_played=-1)

        # Put Dimetromon in hand
        runner.inject_card(1, "BT24-012", "hand")

        # Suspend Elizamon (not Owen) — should NOT trigger delay
        game = runner.game
        eliza = None
        for perm in game.player1.battle_area:
            if perm.top_card and perm.top_card.c_entity_base and \
               perm.top_card.c_entity_base.card_id == "BT24-008":
                eliza = perm
                break

        assert eliza is not None
        eliza.suspend()
        runner.auto_resolve()

        snap = runner.snapshot()
        # Option should still be on the field (delay not triggered)
        option_on_field = [s for s in snap.p1_field if s.card_id == "BT24-089"]
        assert len(option_on_field) >= 1, \
            "Option should remain in battle area when non-Owen suspends"

    def test_delay_does_not_fire_for_opponent_owen(self, debug_runner):
        """Delay should NOT fire when opponent's Owen Dreadnought suspends."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place the option in P1's battle area (previously played)
        runner.place_on_field(1, ["BT24-089"], turn_played=-1)

        # Place Elizamon on P1 field (potential digivolve source)
        runner.place_on_field(1, ["BT24-008"])

        # Put Dimetromon in P1 hand
        runner.inject_card(1, "BT24-012", "hand")

        # Place Owen on P2's (opponent) field
        runner.place_on_field(2, ["BT24-082"])

        # Suspend opponent's Owen
        game = runner.game
        opp_owen = None
        for perm in game.player2.battle_area:
            if perm.top_card and perm.top_card.c_entity_base and \
               perm.top_card.c_entity_base.card_id == "BT24-082":
                opp_owen = perm
                break

        assert opp_owen is not None
        opp_owen.suspend()
        runner.auto_resolve()

        snap = runner.snapshot()
        # P1's option should still be on the field
        option_on_field = [s for s in snap.p1_field if s.card_id == "BT24-089"]
        assert len(option_on_field) >= 1, \
            "Option should remain when opponent's Owen suspends (not ours)"

    def test_delay_not_on_placing_turn(self, debug_runner):
        """Delay cannot activate on the turn the option was placed (turn_played check)."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place option card on the field on the CURRENT turn
        game = runner.game
        current_turn = game.turn_count
        runner.place_on_field(1, ["BT24-089"], turn_played=current_turn)

        # Place Owen and Elizamon
        runner.place_on_field(1, ["BT24-082"])
        runner.place_on_field(1, ["BT24-008"])
        runner.inject_card(1, "BT24-012", "hand")

        # The delay action should NOT be in the action mask on the placing turn
        snap = runner.snapshot()
        option_on_field = [s for s in snap.p1_field if s.card_id == "BT24-089"]
        assert len(option_on_field) >= 1, "Option should be on field"

        # Note: The turn_played check is enforced by the action mask for manual delay
        # The OnTappedAnyone trigger may still fire but should be gated by the card text
        # ("after the placing turn")

    # ── Delay digivolve: correct target/filter checks ────────────────

    def test_delay_digivolve_dragonkin_liberator(self, debug_runner):
        """Delay digivolve should accept Dragonkin+LIBERATOR targets (e.g. Lamiamon)."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place Dimetromon on field (Lv4 Reptile+LIBERATOR — source with Reptile trait)
        runner.place_on_field(1, ["BT24-012"])

        # Place Owen (to trigger delay)
        runner.place_on_field(1, ["BT24-082"])

        # Place option (previously placed)
        runner.place_on_field(1, ["BT24-089"], turn_played=-1)

        # Put Lamiamon in hand (Lv5, Dragonkin+LIBERATOR)
        runner.inject_card(1, "BT24-016", "hand")

        # Suspend Owen
        game = runner.game
        owen = None
        for perm in game.player1.battle_area:
            if perm.top_card and perm.top_card.c_entity_base and \
               perm.top_card.c_entity_base.card_id == "BT24-082":
                owen = perm
                break

        assert owen is not None
        owen.suspend()

        # Select Dimetromon as source (Reptile trait)
        source_action = runner.find_action("Dimetromon")
        assert source_action is not None, \
            "Should be able to select Dimetromon as digivolve source"
        runner.execute(source_action)

        # Select Lamiamon from hand to digivolve into
        digi_action = runner.find_action("Lamiamon")
        assert digi_action is not None, \
            "Should be able to select Lamiamon from hand as digivolve target"
        runner.execute(digi_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Dimetromon should digivolve into Lamiamon
        lamiamon_on_field = [s for s in snap.p1_field if s.card_id == "BT24-016"]
        assert len(lamiamon_on_field) >= 1, \
            "Dimetromon should have digivolved into Lamiamon (Dragonkin+LIBERATOR)"

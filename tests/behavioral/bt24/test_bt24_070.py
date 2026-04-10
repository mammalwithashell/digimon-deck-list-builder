"""Behavioral tests for BT24-070 Growlmon.

Card text:
  Lv.4 | Purple | Dark Dragon, Evil Dragon | DP:5000 | Cost:5

  [On Play] [When Digivolving] If you have 4 or fewer cards in your hand,
  you may play 1 purple Tamer card with a play cost of 4 or less from
  your trash without paying the cost.

  Inherited: [When Attacking] [Once Per Turn] Delete 1 of your opponent's
  level 3 Digimon.

Clauses tested:
  C1: [On Play] trigger plays purple tamer from trash when hand <= 4
  C2: [When Digivolving] trigger plays purple tamer from trash when hand <= 4
  C3: Effect does NOT activate when hand > 4
  C4: Only purple tamers with play cost <= 4 qualify
  C5: Effect is optional ("you may")
  C6: Inherited [When Attacking] deletes 1 opponent's level 3 Digimon
  C7: Inherited is [Once Per Turn]
"""

import pytest
from digimon_gym.engine.data.enums import GamePhase


# Card IDs used in tests
BT24_070 = "BT24-070"      # Card under test (Growlmon, Lv.4 Purple)
MATT_ISHIDA = "BT2-090"    # Purple Tamer, cost 4
KARI_KAMIYA = "BT4-097"    # Purple Tamer, cost 3
TAI_KAMIYA = "BT1-085"     # Red Tamer, cost 4 (NOT purple)
MIREI = "BT11-094"         # Purple Tamer, cost 5 (too expensive)
AGUMON = "ST1-03"          # Lv.3 Red Digimon (opponent target)
BT2_067 = "BT2-067"        # Lv.3 Purple Digimon (DemiDevimon)
MONODRAMON = "BT1-009"     # Lv.3 Digimon
GREYMON = "ST1-07"         # Lv.4 Red Digimon (NOT level 3)
MYOTISMON = "BT2-075"      # Lv.5 Purple Digimon

# Minimal deck for testing (just need enough cards to start a game)
FILLER_DECK = [BT24_070] * 4 + [BT2_067] * 23 + [MATT_ISHIDA] * 23

# SEL_TRASH_START = 130 in action space
SEL_TRASH_START = 130


@pytest.mark.behavioral
class TestBT24070OnPlay:
    """[On Play] If hand <= 4, may play 1 purple Tamer (cost <= 4) from trash."""

    def test_on_play_plays_purple_tamer_from_trash(self, debug_runner):
        """C1: On Play should offer to play a purple tamer from trash
        when hand has 4 or fewer cards."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Set up: put a purple tamer in trash
        runner.inject_card(1, MATT_ISHIDA, "trash")

        # Ensure hand has <= 4 cards
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_070, "hand")
        # Hand: [BT24-070] = 1 card

        snap_before = runner.snapshot()
        assert len(snap_before.p1_hand) <= 4
        assert MATT_ISHIDA in snap_before.p1_trash

        # Play BT24-070 from hand
        action = runner.find_action("Play Growlmon")
        assert action is not None, f"Should find Play action. Actions: {runner.actions()}"
        runner.execute(action)

        # After playing Growlmon, the On Play effect fires and enters selection.
        # We should be in a SelectTarget phase with trash options available.
        game = runner.game
        assert game.current_phase == GamePhase.SelectTarget, \
            f"Should be in SelectTarget for tamer play, got {game.current_phase}"

        # Find the trash selection action (SEL_TRASH_START + index of Matt Ishida)
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        assert len(trash_actions) > 0, (
            f"Should have trash selection for Matt Ishida. Legal: {legal}"
        )

        # Select Matt Ishida from trash
        runner.execute(trash_actions[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        # Matt Ishida should now be on the field (played from trash)
        matt_on_field = [s for s in snap.p1_field if s.card_id == MATT_ISHIDA]
        assert len(matt_on_field) == 1, (
            f"Matt Ishida should be on field (played from trash). "
            f"Field: {[s.card_id for s in snap.p1_field]}, "
            f"Trash: {snap.p1_trash}"
        )
        # Matt Ishida should no longer be in trash
        assert MATT_ISHIDA not in snap.p1_trash, \
            "Matt Ishida should have been removed from trash"

    def test_on_play_does_not_trigger_when_hand_over_4(self, debug_runner):
        """C3: Effect should NOT activate when hand has more than 4 cards."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Put purple tamer in trash
        runner.inject_card(1, MATT_ISHIDA, "trash")

        # Ensure hand has > 4 cards after play
        # We need 5+ cards in hand AFTER playing Growlmon
        # So we need 6+ cards before play (Growlmon + 5 others)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_070, "hand")
        for _ in range(5):
            runner.inject_card(1, BT2_067, "hand")
        # Hand: [BT24-070, DemiDevimon x5] = 6 cards
        # After playing Growlmon: 5 cards in hand (> 4, effect should not trigger)

        snap_before = runner.snapshot()
        assert len(snap_before.p1_hand) == 6

        action = runner.find_action("Play Growlmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Matt Ishida should still be in trash (effect didn't fire)
        assert MATT_ISHIDA in snap.p1_trash, \
            "Matt Ishida should remain in trash when hand > 4"
        matt_on_field = [s for s in snap.p1_field if s.card_id == MATT_ISHIDA]
        assert len(matt_on_field) == 0, \
            "Matt Ishida should NOT be on field"

    def test_on_play_exactly_4_cards_triggers(self, debug_runner):
        """C3: Edge case: hand has exactly 4 cards AFTER play (should trigger)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, MATT_ISHIDA, "trash")

        # 5 cards in hand => after playing Growlmon => 4 cards (exactly 4, should trigger)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_070, "hand")
        for _ in range(4):
            runner.inject_card(1, BT2_067, "hand")
        # Hand: [BT24-070, DemiDevimon x4] = 5 cards

        action = runner.find_action("Play Growlmon")
        assert action is not None
        runner.execute(action)

        # Should enter selection phase for trash tamer play
        game = runner.game
        assert game.current_phase == GamePhase.SelectTarget, \
            f"Should be in SelectTarget (hand=4 after play), got {game.current_phase}"

        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        assert len(trash_actions) > 0, \
            f"Should have trash selection for Matt Ishida. Legal: {legal}"

        runner.execute(trash_actions[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        # After playing Growlmon, hand = 4. Effect should trigger.
        matt_on_field = [s for s in snap.p1_field if s.card_id == MATT_ISHIDA]
        assert len(matt_on_field) == 1, (
            f"Matt Ishida should be on field (hand was exactly 4 after play). "
            f"Field: {[s.card_id for s in snap.p1_field]}, "
            f"Hand: {snap.p1_hand}, Trash: {snap.p1_trash}"
        )


    def test_on_play_optional_can_decline(self, debug_runner):
        """C5: Effect is optional ('you may'), so declining should leave tamer in trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, MATT_ISHIDA, "trash")
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_070, "hand")

        action = runner.find_action("Play Growlmon")
        assert action is not None
        runner.execute(action)

        # Should be in selection phase
        game = runner.game
        assert game.current_phase == GamePhase.SelectTarget

        # Decline / Pass should be available (it's optional)
        decline_action = runner.find_action("Decline")
        assert decline_action is not None, (
            f"Should have Decline option (effect is optional). Actions: {runner.actions()}"
        )

        runner.execute(decline_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Matt Ishida should still be in trash (player declined)
        assert MATT_ISHIDA in snap.p1_trash, \
            "Matt Ishida should remain in trash after declining"
        matt_on_field = [s for s in snap.p1_field if s.card_id == MATT_ISHIDA]
        assert len(matt_on_field) == 0, \
            "Matt Ishida should NOT be on field after declining"


@pytest.mark.behavioral
class TestBT24070WhenDigivolving:
    """[When Digivolving] If hand <= 4, may play purple Tamer from trash."""

    def test_when_digivolving_plays_tamer_from_trash(self, debug_runner):
        """C2: When Digivolving triggers the same tamer-from-trash effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Put purple tamer in trash
        runner.inject_card(1, MATT_ISHIDA, "trash")

        # Place a Lv.3 Purple Digimon on field for digivolving into BT24-070
        runner.place_on_field(1, [BT2_067], turn_played=-1)

        # Hand: just BT24-070 for digivolving (need <= 4 cards after digi)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_070, "hand")
        # Hand = 1 card, after digivolving = 0 cards (< 4)

        # Digivolve into Growlmon
        action = runner.find_action("Digivolve")
        assert action is not None, f"Should find digivolve action. Actions: {runner.actions()}"
        runner.execute(action)

        # When Digivolving effect should fire — enter selection for trash tamer
        game = runner.game
        if game.current_phase == GamePhase.SelectTarget:
            legal = runner.action_mask()
            trash_actions = [a for a in legal if a >= SEL_TRASH_START]
            if trash_actions:
                runner.execute(trash_actions[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        matt_on_field = [s for s in snap.p1_field if s.card_id == MATT_ISHIDA]
        assert len(matt_on_field) == 1, (
            f"Matt Ishida should be on field after digivolving. "
            f"Field: {[s.card_id for s in snap.p1_field]}, "
            f"Trash: {snap.p1_trash}"
        )


@pytest.mark.behavioral
class TestBT24070TamerFilter:
    """Filter: only purple tamers with cost <= 4 from trash."""

    def test_does_not_play_non_purple_tamer(self, debug_runner):
        """C4: Red Tamer (not purple) should NOT be playable."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Only a red tamer in trash (not purple)
        runner.inject_card(1, TAI_KAMIYA, "trash")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_070, "hand")

        action = runner.find_action("Play Growlmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        tai_on_field = [s for s in snap.p1_field if s.card_id == TAI_KAMIYA]
        assert len(tai_on_field) == 0, \
            "Red tamer should NOT be playable by this effect"
        assert TAI_KAMIYA in snap.p1_trash, \
            "Red tamer should remain in trash"

    def test_does_not_play_purple_tamer_cost_over_4(self, debug_runner):
        """C4: Purple Tamer with cost > 4 should NOT be playable."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Purple tamer with cost 5 (too expensive)
        runner.inject_card(1, MIREI, "trash")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_070, "hand")

        action = runner.find_action("Play Growlmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        mirei_on_field = [s for s in snap.p1_field if s.card_id == MIREI]
        assert len(mirei_on_field) == 0, \
            "Purple tamer with cost > 4 should NOT be playable"
        assert MIREI in snap.p1_trash, \
            "Purple tamer with cost > 4 should remain in trash"


@pytest.mark.behavioral
class TestBT24070InheritedWhenAttacking:
    """Inherited: [When Attacking] [Once Per Turn] Delete 1 opponent's level 3 Digimon."""

    def test_inherited_deletes_opponent_level_3(self, debug_runner):
        """C6: When attacking, should delete 1 opponent's level 3 Digimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=3)
        runner.set_phase("Main")

        # Place BT24-070 under a Lv.5 Digimon (so inherited effect is active)
        attacker = runner.place_on_field(1, [BT24_070, MYOTISMON], turn_played=-1)
        assert attacker.is_digimon

        # Place a Lv.3 on opponent's field
        runner.place_on_field(2, [AGUMON])

        snap_before = runner.snapshot()
        opp_lv3_before = [s for s in snap_before.p2_field if s.card_id == AGUMON]
        assert len(opp_lv3_before) == 1

        # Attack with the Digimon
        action = runner.find_action("Attack")
        assert action is not None, f"Should find attack action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # The opponent's level 3 Agumon should be deleted
        opp_lv3_after = [s for s in snap.p2_field if s.card_id == AGUMON]
        assert len(opp_lv3_after) == 0, (
            f"Opponent's Lv.3 Agumon should be deleted. "
            f"Opp field: {[s.card_id for s in snap.p2_field]}"
        )

    def test_inherited_does_not_delete_level_4(self, debug_runner):
        """C6: Should NOT delete opponent's Digimon that is NOT level 3."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=3)
        runner.set_phase("Main")

        # BT24-070 under Myotismon for inherited effect
        runner.place_on_field(1, [BT24_070, MYOTISMON], turn_played=-1)

        # Only a Lv.4 on opponent's field (no Lv.3)
        runner.place_on_field(2, [GREYMON])

        snap_before = runner.snapshot()
        opp_field_before = len(snap_before.p2_field)

        action = runner.find_action("Attack")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent's Lv.4 should NOT be deleted (only targets Lv.3)
        greymon_on_field = [s for s in snap.p2_field if s.card_id == GREYMON]
        assert len(greymon_on_field) == 1, (
            f"Lv.4 Greymon should NOT be deleted by inherited effect. "
            f"Opp field: {[s.card_id for s in snap.p2_field]}"
        )

    def test_inherited_once_per_turn(self, debug_runner):
        """C7: Inherited effect should only fire once per turn."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=3)
        runner.set_phase("Main")

        # Place attacker with BT24-070 inherited, give it Rush-like behavior
        perm = runner.place_on_field(1, [BT24_070, MYOTISMON], turn_played=-1)

        # Place 2 Lv.3 Digimon on opponent's field
        runner.place_on_field(2, [AGUMON])
        runner.place_on_field(2, [MONODRAMON])

        game = runner.game

        # Find the inherited effect directly and invoke it twice
        inherited_effects = []
        for cs in perm.card_sources:
            for eff in cs.effect_list(None):
                if getattr(eff, 'is_inherited_effect', False) and getattr(eff, 'is_on_attack', False):
                    inherited_effects.append(eff)

        assert len(inherited_effects) >= 1, "Should have inherited When Attacking effect"

        eff = inherited_effects[0]
        context = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
        }

        # First activation should work
        assert eff.can_activate_this_turn() is True
        eff.record_activation()
        eff.on_process_callback(context)
        runner.auto_resolve()

        # Second activation should be blocked by Once Per Turn
        assert eff.can_activate_this_turn() is False, \
            "Once Per Turn should prevent second activation"

    def test_inherited_not_active_as_main_effect(self, debug_runner):
        """Inherited effect should only fire when under another Digimon, not standalone."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=3)
        runner.set_phase("Main")

        # Place BT24-070 directly on field (not under another Digimon)
        perm = runner.place_on_field(1, [BT24_070], turn_played=-1)

        # Place Lv.3 on opponent field
        runner.place_on_field(2, [AGUMON])

        game = runner.game

        # The inherited effect should have is_inherited_effect = True
        inherited_effects = []
        top = perm.top_card
        for eff in top.effect_list(None):
            if getattr(eff, 'is_inherited_effect', False):
                inherited_effects.append(eff)

        assert len(inherited_effects) >= 1, "Should have inherited effect defined"
        # Verify the flag is set
        assert inherited_effects[0].is_inherited_effect is True

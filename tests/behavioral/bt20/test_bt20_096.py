"""Behavioral tests for BT20-096 Black Sabbath (Option, Purple, Cost 2).

Card text:
  [Trash] [Main] If you have 4 or fewer cards in your hand, by paying 6
  cost, return this card to the bottom of the deck and delete 1 of your
  opponent's unsuspended Digimon.

  [Main] Trash 1 card in your hand. Then, delete 1 of your opponent's
  level 4 or lower Digimon.

  [Security] Delete 1 of your opponent's level 6 or lower Digimon.

Key faithfulness points tested:
  - [Trash][Main]: condition requires owner's turn, card in trash, hand <= 4
  - [Trash][Main]: pays 6 cost via add_memory(-6), returns card to deck bottom
  - [Trash][Main]: deletes exactly 1 unsuspended opponent Digimon
  - [Trash][Main]: suspended Digimon are NOT valid targets
  - [Trash][Main]: condition fails when hand > 4
  - [Main]: trashes 1 card from hand, then deletes Lv4 or lower opponent Digimon
  - [Main]: "Then" means delete step proceeds even if hand was empty (C# ref)
  - [Main]: Lv5+ Digimon are NOT valid delete targets
  - [Security]: deletes 1 opponent Digimon at Lv6 or lower
  - [Security]: Lv7 Digimon are NOT valid targets
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


# Card IDs
BLACK_SABBATH = "BT20-096"
# Purple Lv3 — needed on P1 field for option color requirement
PURPLE_LV3 = "ST6-02"         # DemiDevimon (Purple, Lv3)
# Opponent target Digimon at various levels
LV3_DIGIMON = "ST1-03"        # Agumon (Red, Lv3)
LV4_DIGIMON = "BT2-071"       # Wizardmon (Purple, Lv4)
LV5_DIGIMON = "BT2-075"       # Myotismon (Purple, Lv5)
LV6_DIGIMON = "BT2-079"       # VenomMyotismon (Purple, Lv6)
# A generic filler card for hand
FILLER = "ST1-03"             # Agumon


@pytest.mark.behavioral
class TestBlackSabbathTrashMain:
    """[Trash] [Main] If you have 4 or fewer cards in your hand, by paying
    6 cost, return this card to the bottom of the deck and delete 1 of your
    opponent's unsuspended Digimon."""

    def test_trash_main_condition_requires_in_trash(self, debug_runner):
        """Condition should only pass when the card is in the trash."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Clear hand so hand <= 4 condition is met
        runner.clear_zone(1, "hand")

        # Place card in trash
        runner.inject_card(1, BLACK_SABBATH, "trash")
        card = game.player1.trash_cards[-1]

        effects = card.effect_list(None)
        trash_effects = [e for e in effects if getattr(e, '_is_trash_main', False)]
        assert len(trash_effects) >= 1, "Should have a [Trash][Main] effect"

        effect = trash_effects[0]
        ctx = {'game': game, 'player': game.player1, 'card': card}
        assert effect.can_use_condition(ctx), (
            "Condition should pass when card is in trash with hand <= 4")

    def test_trash_main_condition_requires_hand_4_or_fewer(self, debug_runner):
        """Condition should fail when hand has more than 4 cards."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        runner.inject_card(1, BLACK_SABBATH, "trash")
        card = game.player1.trash_cards[-1]

        effects = card.effect_list(None)
        trash_effect = [e for e in effects if getattr(e, '_is_trash_main', False)][0]

        # Clear hand and add exactly 4 cards
        runner.clear_zone(1, "hand")
        for _ in range(4):
            runner.inject_card(1, FILLER, "hand")
        assert len(game.player1.hand_cards) == 4

        ctx = {'game': game, 'player': game.player1, 'card': card}
        assert trash_effect.can_use_condition(ctx), (
            "Condition should pass with exactly 4 cards in hand")

        # Add a 5th card to hand (now 5 > 4)
        runner.inject_card(1, FILLER, "hand")
        assert len(game.player1.hand_cards) == 5

        assert not trash_effect.can_use_condition(ctx), (
            "Condition should FAIL with 5 cards in hand")

    def test_trash_main_condition_requires_owners_turn(self, debug_runner):
        """Condition should only pass on the card owner's turn."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BLACK_SABBATH, "trash")
        card = game.player1.trash_cards[-1]

        effects = card.effect_list(None)
        trash_effect = [e for e in effects if getattr(e, '_is_trash_main', False)][0]

        ctx = {'game': game, 'player': game.player1, 'card': card}
        assert trash_effect.can_use_condition(ctx), (
            "Should pass on P1's turn")

        # Switch to opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        assert not trash_effect.can_use_condition(ctx), (
            "Should FAIL on opponent's turn")

    def test_trash_main_pays_6_cost(self, debug_runner):
        """Process should pay 6 cost via add_memory(-6)."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BLACK_SABBATH, "trash")
        card = game.player1.trash_cards[-1]

        # Place an unsuspended opponent Digimon as a valid target
        opp_perm = runner.place_on_field(2, [LV3_DIGIMON])
        assert not opp_perm.is_suspended

        effects = card.effect_list(None)
        trash_effect = [e for e in effects if getattr(e, '_is_trash_main', False)][0]

        mem_before = game.memory

        # Mock the selection to auto-pick the first valid target
        original_select = game.effect_select_opponent_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select

        trash_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'card': card,
        })

        game.effect_select_opponent_permanent = original_select

        assert game.memory == mem_before - 6, (
            f"Should pay 6 cost (memory {mem_before} -> {mem_before - 6}), "
            f"got {game.memory}")

    def test_trash_main_returns_card_to_deck_bottom(self, debug_runner):
        """Process should remove card from trash and add to bottom of deck."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BLACK_SABBATH, "trash")
        card = game.player1.trash_cards[-1]

        opp_perm = runner.place_on_field(2, [LV3_DIGIMON])
        lib_size_before = len(game.player1.library_cards)

        effects = card.effect_list(None)
        trash_effect = [e for e in effects if getattr(e, '_is_trash_main', False)][0]

        original_select = game.effect_select_opponent_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select

        trash_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'card': card,
        })

        game.effect_select_opponent_permanent = original_select

        # Card should no longer be in trash
        assert card not in game.player1.trash_cards, (
            "Card should be removed from trash")

        # Card should be at the bottom (last element) of the library
        assert card in game.player1.library_cards, (
            "Card should be in the library")
        assert game.player1.library_cards[-1] is card, (
            "Card should be at the BOTTOM of the deck (last index)")
        assert len(game.player1.library_cards) == lib_size_before + 1

    def test_trash_main_deletes_unsuspended_opponent_digimon(self, debug_runner):
        """Process should delete 1 of opponent's unsuspended Digimon."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BLACK_SABBATH, "trash")
        card = game.player1.trash_cards[-1]

        opp_perm = runner.place_on_field(2, [LV5_DIGIMON])
        assert not opp_perm.is_suspended

        effects = card.effect_list(None)
        trash_effect = [e for e in effects if getattr(e, '_is_trash_main', False)][0]

        original_select = game.effect_select_opponent_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in list(player.enemy.battle_area):
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select

        trash_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'card': card,
        })

        game.effect_select_opponent_permanent = original_select

        # Opponent Digimon should be deleted
        assert opp_perm not in game.player2.battle_area, (
            "Unsuspended opponent Digimon should be deleted")

    def test_trash_main_does_not_target_suspended_digimon(self, debug_runner):
        """Suspended Digimon should NOT be valid targets for [Trash][Main] delete."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BLACK_SABBATH, "trash")
        card = game.player1.trash_cards[-1]

        # Place a suspended opponent Digimon and an unsuspended one
        suspended_perm = runner.place_on_field(2, [LV3_DIGIMON], is_suspended=True)
        unsuspended_perm = runner.place_on_field(2, [LV4_DIGIMON])

        effects = card.effect_list(None)
        trash_effect = [e for e in effects if getattr(e, '_is_trash_main', False)][0]

        selected_targets = []
        original_select = game.effect_select_opponent_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in list(player.enemy.battle_area):
                if filter_fn is None or filter_fn(p):
                    selected_targets.append(p)
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select

        trash_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'card': card,
        })

        game.effect_select_opponent_permanent = original_select

        # Only the unsuspended Digimon should have been selected
        assert len(selected_targets) == 1, (
            f"Should select exactly 1 target, got {len(selected_targets)}")
        assert selected_targets[0] is unsuspended_perm, (
            "Should select the unsuspended Digimon, not the suspended one")

    def test_trash_main_no_targets_still_pays_and_returns(self, debug_runner):
        """When no valid targets exist (all suspended or no Digimon), should
        still pay 6 cost and return card to deck bottom per C# flow."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BLACK_SABBATH, "trash")
        card = game.player1.trash_cards[-1]

        # Only a suspended Digimon on opponent's field
        runner.place_on_field(2, [LV3_DIGIMON], is_suspended=True)

        effects = card.effect_list(None)
        trash_effect = [e for e in effects if getattr(e, '_is_trash_main', False)][0]

        mem_before = game.memory

        trash_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'card': card,
        })

        # Cost should still be paid and card returned to deck bottom
        assert game.memory == mem_before - 6, (
            "Should still pay 6 cost even without valid targets")
        assert card not in game.player1.trash_cards, (
            "Card should still be returned to deck bottom")
        assert card in game.player1.library_cards, (
            "Card should be in library")


@pytest.mark.behavioral
class TestBlackSabbathMainEffect:
    """[Main] Trash 1 card in your hand. Then, delete 1 of your opponent's
    level 4 or lower Digimon."""

    def test_main_effect_exists_as_option_skill(self, debug_runner):
        """Should have an OptionSkill timing effect."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, BLACK_SABBATH, "hand")

        effects = card.effect_list(None)
        option_effects = [e for e in effects
                          if e.timing == EffectTiming.OptionSkill]
        assert len(option_effects) >= 1, "Should have an OptionSkill effect"

    def test_main_trashes_hand_card_then_deletes_lv4(self, debug_runner):
        """Should trash 1 hand card, then delete 1 opponent Lv4 or lower Digimon."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        card = runner.inject_card(1, BLACK_SABBATH, "hand")
        filler_card = runner.inject_card(1, FILLER, "hand")

        opp_perm = runner.place_on_field(2, [LV4_DIGIMON])
        runner.place_on_field(1, [PURPLE_LV3])  # Color requirement

        effects = card.effect_list(None)
        option_effect = [e for e in effects
                         if e.timing == EffectTiming.OptionSkill][0]

        trash_before = len(game.player1.trash_cards)

        # Mock hand selection to pick the filler card
        original_select_hand = game.effect_select_hand_card

        def mock_select_hand(player, filter_fn, callback, is_optional=False, prompt=None):
            for c in list(player.hand_cards):
                if filter_fn(c):
                    callback(c)
                    return
        game.effect_select_hand_card = mock_select_hand

        # Mock permanent selection to pick the opponent Digimon
        original_select_perm = game.effect_select_opponent_permanent

        def mock_select_perm(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in list(player.enemy.battle_area):
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select_perm

        option_effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        game.effect_select_hand_card = original_select_hand
        game.effect_select_opponent_permanent = original_select_perm

        # Hand card should have been trashed
        assert len(game.player1.trash_cards) > trash_before, (
            "Should have trashed a card from hand")

        # Opponent Lv4 Digimon should be deleted
        assert opp_perm not in game.player2.battle_area, (
            "Opponent Lv4 Digimon should be deleted")

    def test_main_does_not_delete_lv5_or_higher(self, debug_runner):
        """Delete target filter should exclude Lv5+ Digimon."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        card = runner.inject_card(1, BLACK_SABBATH, "hand")
        runner.inject_card(1, FILLER, "hand")

        opp_perm_lv5 = runner.place_on_field(2, [LV5_DIGIMON])
        runner.place_on_field(1, [PURPLE_LV3])

        effects = card.effect_list(None)
        option_effect = [e for e in effects
                         if e.timing == EffectTiming.OptionSkill][0]

        original_select_hand = game.effect_select_hand_card

        def mock_select_hand(player, filter_fn, callback, is_optional=False, prompt=None):
            for c in list(player.hand_cards):
                if filter_fn(c):
                    callback(c)
                    return
        game.effect_select_hand_card = mock_select_hand

        selected_perms = []
        original_select_perm = game.effect_select_opponent_permanent

        def mock_select_perm(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in list(player.enemy.battle_area):
                if filter_fn is None or filter_fn(p):
                    selected_perms.append(p)
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select_perm

        option_effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        game.effect_select_hand_card = original_select_hand
        game.effect_select_opponent_permanent = original_select_perm

        # No Lv5 Digimon should have been selected
        assert len(selected_perms) == 0, (
            "Lv5 Digimon should NOT be a valid delete target")
        assert opp_perm_lv5 in game.player2.battle_area, (
            "Lv5 Digimon should NOT be deleted")

    def test_main_delete_proceeds_when_hand_empty(self, debug_runner):
        """BUG CHECK: When hand is empty, the delete step should still proceed.

        In C# (DCGO reference), the hand trash is gated by
        `if (card.Owner.HandCards.Count >= 1)` but the delete step is
        OUTSIDE that gate — it always runs regardless of whether a card
        was trashed. The Python script must match this behavior.
        """
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place the card directly — don't put it in hand since we want
        # to test the process callback with an empty hand
        runner.inject_card(1, BLACK_SABBATH, "trash")
        card = game.player1.trash_cards[-1]

        runner.clear_zone(1, "hand")
        assert len(game.player1.hand_cards) == 0

        opp_perm = runner.place_on_field(2, [LV3_DIGIMON])

        effects = card.effect_list(None)
        option_effect = [e for e in effects
                         if e.timing == EffectTiming.OptionSkill][0]

        # Mock permanent selection
        selected_perms = []
        original_select_perm = game.effect_select_opponent_permanent

        def mock_select_perm(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in list(player.enemy.battle_area):
                if filter_fn is None or filter_fn(p):
                    selected_perms.append(p)
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select_perm

        option_effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        game.effect_select_opponent_permanent = original_select_perm

        # Delete should still proceed even with empty hand
        assert len(selected_perms) == 1, (
            "Delete step should still execute when hand is empty. "
            "C# reference shows the delete is OUTSIDE the hand-trash gate.")
        assert opp_perm not in game.player2.battle_area, (
            "Opponent Lv3 Digimon should be deleted even when hand was empty")

    def test_main_trash_is_mandatory(self, debug_runner):
        """Hand trash should be mandatory (not optional) per C# canNoSelect=false."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        card = runner.inject_card(1, BLACK_SABBATH, "hand")
        runner.inject_card(1, FILLER, "hand")
        runner.place_on_field(1, [PURPLE_LV3])

        effects = card.effect_list(None)
        option_effect = [e for e in effects
                         if e.timing == EffectTiming.OptionSkill][0]

        hand_select_optional = []
        original_select_hand = game.effect_select_hand_card

        def mock_select_hand(player, filter_fn, callback, is_optional=False, prompt=None):
            hand_select_optional.append(is_optional)
            for c in list(player.hand_cards):
                if filter_fn(c):
                    callback(c)
                    return
        game.effect_select_hand_card = mock_select_hand

        option_effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        game.effect_select_hand_card = original_select_hand

        assert len(hand_select_optional) >= 1, "Should have called select_hand"
        assert hand_select_optional[0] is False, (
            "Hand trash should be mandatory (is_optional=False)")


@pytest.mark.behavioral
class TestBlackSabbathSecurityEffect:
    """[Security] Delete 1 of your opponent's level 6 or lower Digimon."""

    def test_security_effect_exists(self, debug_runner):
        """Should have a SecuritySkill effect."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, BLACK_SABBATH, "hand")

        effects = card.effect_list(None)
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, "Should have a SecuritySkill effect"

    def test_security_is_marked_as_security_effect(self, debug_runner):
        """Security effect should have is_security_effect = True."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, BLACK_SABBATH, "hand")

        effects = card.effect_list(None)
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill]
        assert sec_effects[0].is_security_effect is True, (
            "Security effect should be marked as is_security_effect")

    def test_security_deletes_lv6_or_lower(self, debug_runner):
        """Security should delete 1 opponent Digimon at Lv6 or lower."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        card = runner.inject_card(1, BLACK_SABBATH, "hand")
        opp_perm = runner.place_on_field(2, [LV6_DIGIMON])

        effects = card.effect_list(None)
        sec_effect = [e for e in effects
                      if e.timing == EffectTiming.SecuritySkill][0]

        original_select = game.effect_select_opponent_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in list(player.enemy.battle_area):
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select

        sec_effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        game.effect_select_opponent_permanent = original_select

        assert opp_perm not in game.player2.battle_area, (
            "Lv6 Digimon should be deleted by security effect")

    def test_security_does_not_delete_lv7(self, debug_runner):
        """Security should NOT target Lv7+ Digimon."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        card = runner.inject_card(1, BLACK_SABBATH, "hand")
        # BT20-102 is Lv7 Omnimon (X Antibody)
        opp_perm = runner.place_on_field(2, ["BT20-102"])

        effects = card.effect_list(None)
        sec_effect = [e for e in effects
                      if e.timing == EffectTiming.SecuritySkill][0]

        selected_perms = []
        original_select = game.effect_select_opponent_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in list(player.enemy.battle_area):
                if filter_fn is None or filter_fn(p):
                    selected_perms.append(p)
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select

        sec_effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        game.effect_select_opponent_permanent = original_select

        assert len(selected_perms) == 0, (
            "Lv7 Digimon should NOT be a valid security delete target")
        assert opp_perm in game.player2.battle_area, (
            "Lv7 Digimon should remain on field")

    def test_security_deletes_lv4_digimon(self, debug_runner):
        """Security should also handle Lv4 Digimon (well within range)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        card = runner.inject_card(1, BLACK_SABBATH, "hand")
        opp_perm = runner.place_on_field(2, [LV4_DIGIMON])

        effects = card.effect_list(None)
        sec_effect = [e for e in effects
                      if e.timing == EffectTiming.SecuritySkill][0]

        original_select = game.effect_select_opponent_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in list(player.enemy.battle_area):
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select

        sec_effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        game.effect_select_opponent_permanent = original_select

        assert opp_perm not in game.player2.battle_area, (
            "Lv4 Digimon should be deleted by security effect")

    def test_security_no_targets_does_not_crash(self, debug_runner):
        """Security should handle the case where no valid targets exist."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        card = runner.inject_card(1, BLACK_SABBATH, "hand")
        # No opponent Digimon on field

        effects = card.effect_list(None)
        sec_effect = [e for e in effects
                      if e.timing == EffectTiming.SecuritySkill][0]

        # Should not crash
        sec_effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        # Snapshot to verify game is still valid
        snap = runner.snapshot()
        assert not snap.is_game_over


@pytest.mark.behavioral
class TestBlackSabbathIntegration:
    """Integration tests using the full action system."""

    def test_trash_main_action_appears_in_actions(self, debug_runner):
        """[Trash][Main] should appear as an action when conditions are met."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BLACK_SABBATH, "trash")
        runner.set_phase("Main")

        action = runner.find_action("[Trash][Main]: Black Sabbath")
        assert action is not None, (
            f"Should find [Trash][Main] action for Black Sabbath. "
            f"Available actions: {runner.actions()}")

    def test_trash_main_action_hidden_when_hand_too_large(self, debug_runner):
        """[Trash][Main] should NOT appear when hand has > 4 cards."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        runner.clear_zone(1, "hand")
        for _ in range(5):
            runner.inject_card(1, FILLER, "hand")
        runner.inject_card(1, BLACK_SABBATH, "trash")
        runner.set_phase("Main")

        action = runner.find_action("[Trash][Main]: Black Sabbath")
        assert action is None, (
            "Should NOT show [Trash][Main] action when hand has 5 cards")

    def test_play_option_main_effect_via_action(self, debug_runner):
        """Playing Black Sabbath as an option should trigger the [Main] effect."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        runner.place_on_field(1, [PURPLE_LV3])  # Color requirement
        runner.inject_card(1, BLACK_SABBATH, "hand")
        runner.inject_card(1, FILLER, "hand")
        opp_perm = runner.place_on_field(2, [LV4_DIGIMON])

        runner.set_phase("Main")

        action = runner.find_action("Black Sabbath")
        assert action is not None, (
            f"Should find Play Black Sabbath action. "
            f"Available: {runner.actions()}")

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Option should be in trash after resolution
        assert "BT20-096" in snap.p1_trash, (
            "Black Sabbath should be in trash after playing")

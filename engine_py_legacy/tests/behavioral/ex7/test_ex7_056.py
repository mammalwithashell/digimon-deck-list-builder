"""Behavioral tests for EX7-056 Orochimon.

Card text:
    <Blocker>
    [On Deletion] Trash 1 card in your hand. Then, delete 1 of your
        opponent's level 3 Digimon and level 4 Digimon.

    Inherited: <Retaliation>

Key behaviors (from C# reference):
    1. Blocker keyword (standard)
    2. On Deletion triggers via OnDestroyedAnyone timing with is_on_deletion flag
    3. Trash 1 card from hand is mandatory (canNoSelect: false) when hand has cards
    4. "Then" clause: delete lv3 + lv4 proceeds even if hand was empty (no card trashed)
    5. Select 1 opponent lv3 Digimon, select 1 opponent lv4 Digimon (both mandatory when targets exist)
    6. C# collects both selections then destroys together; Python deletes sequentially (acceptable)
    7. Inherited <Retaliation> keyword
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming


# Simple deck with Orochimon + filler
OROCHIMON_ID = "EX7-056"
LV3_ID = "BT1-010"  # Agumon, Lv.3
LV4_ID = "ST1-07"   # Greymon, Lv.4
LV5_ID = "BT1-020"  # Groundramon, Lv.5
FILLER = "BT1-010"  # Lv.3 Agumon filler

DECK = [OROCHIMON_ID] * 4 + [FILLER] * 46


def _build_deck(*card_ids, filler=FILLER, count=50):
    deck = list(card_ids)
    while len(deck) < count:
        deck.append(filler)
    return deck


@pytest.mark.behavioral
class TestEX7056OrochimonStructure:
    """Structural tests for EX7-056 effect configuration."""

    def test_has_blocker(self, debug_runner):
        """Effect 0 should be a Blocker."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        effects = perm.top_card.effect_list(None)
        blocker_effects = [e for e in effects if getattr(e, '_is_blocker', False)]
        assert len(blocker_effects) >= 1, "Should have a Blocker effect"

    def test_has_on_deletion_effect(self, debug_runner):
        """Should have an OnDestroyedAnyone effect with is_on_deletion flag."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        effects = perm.top_card.effect_list(None)
        on_del = [e for e in effects
                  if getattr(e, 'timing', None) == EffectTiming.OnDestroyedAnyone
                  and getattr(e, 'is_on_deletion', False)
                  and not getattr(e, 'is_inherited_effect', False)]
        assert len(on_del) == 1, "Should have exactly 1 non-inherited On Deletion effect"

    def test_has_inherited_retaliation(self, debug_runner):
        """Inherited effect should be Retaliation."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        effects = perm.top_card.effect_list(None)
        retal = [e for e in effects
                 if getattr(e, '_is_retaliation', False)
                 and getattr(e, 'is_inherited_effect', False)]
        assert len(retal) == 1, "Should have exactly 1 inherited Retaliation effect"


@pytest.mark.behavioral
class TestEX7056OnDeletion:
    """Behavioral tests for [On Deletion] effect."""

    def test_on_deletion_trashes_hand_card(self, debug_runner):
        """On Deletion should trash 1 card from hand (mandatory)."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        game = runner.game
        p1 = game.player1

        # Ensure P1 has hand cards
        runner.inject_card(1, FILLER, "hand")
        hand_before = len(p1.hand_cards)
        trash_before = len(p1.trash_cards)

        # Delete Orochimon to trigger On Deletion
        p1.delete_permanent(perm)
        runner.auto_resolve()

        # Hand should have lost 1 card (trashed)
        assert len(p1.hand_cards) == hand_before - 1, \
            f"Hand should lose 1 card: was {hand_before}, now {len(p1.hand_cards)}"
        # Trash should have gained: 1 (hand card) + Orochimon's cards
        assert len(p1.trash_cards) > trash_before, "Trash should have gained cards"

    def test_on_deletion_deletes_opponent_lv3(self, debug_runner):
        """On Deletion should delete 1 opponent's level 3 Digimon."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        game = runner.game
        p1 = game.player1

        # Ensure P1 has a hand card to trash
        runner.inject_card(1, FILLER, "hand")

        # Place a lv3 Digimon on opponent's field
        opp_lv3 = runner.place_on_field(2, [LV3_ID])

        opp_field_before = len(game.player2.battle_area)
        assert opp_field_before >= 1, "Opponent should have a lv3 Digimon"

        # Delete Orochimon
        p1.delete_permanent(perm)
        runner.auto_resolve()

        # Opponent's lv3 should be gone
        opp_lv3_remaining = [p for p in game.player2.battle_area
                             if p.is_digimon and getattr(p, 'level', None) == 3]
        assert len(opp_lv3_remaining) == 0, \
            "Opponent's level 3 Digimon should have been deleted"

    def test_on_deletion_deletes_opponent_lv4(self, debug_runner):
        """On Deletion should delete 1 opponent's level 4 Digimon."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        game = runner.game
        p1 = game.player1

        runner.inject_card(1, FILLER, "hand")

        # Place a lv4 Digimon on opponent's field
        opp_lv4 = runner.place_on_field(2, [LV4_ID])

        # Delete Orochimon
        p1.delete_permanent(perm)
        runner.auto_resolve()

        # Opponent's lv4 should be gone
        opp_lv4_remaining = [p for p in game.player2.battle_area
                             if p.is_digimon and getattr(p, 'level', None) == 4]
        assert len(opp_lv4_remaining) == 0, \
            "Opponent's level 4 Digimon should have been deleted"

    def test_on_deletion_deletes_both_lv3_and_lv4(self, debug_runner):
        """On Deletion should delete BOTH a lv3 AND a lv4 opponent Digimon."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        game = runner.game
        p1 = game.player1

        runner.inject_card(1, FILLER, "hand")

        # Place both lv3 and lv4 on opponent's field
        runner.place_on_field(2, [LV3_ID])
        runner.place_on_field(2, [LV4_ID])

        assert len(game.player2.battle_area) == 2, "Opponent should have 2 Digimon"

        p1.delete_permanent(perm)
        runner.auto_resolve()

        # Both should be deleted
        assert len(game.player2.battle_area) == 0, \
            f"Both opponent Digimon should be deleted, but {len(game.player2.battle_area)} remain"

    def test_on_deletion_does_not_delete_lv5(self, debug_runner):
        """On Deletion should NOT delete a level 5 Digimon."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        game = runner.game
        p1 = game.player1

        runner.inject_card(1, FILLER, "hand")

        # Place only a lv5 on opponent's field
        runner.place_on_field(2, [LV5_ID])

        p1.delete_permanent(perm)
        runner.auto_resolve()

        # Lv5 should still be there
        assert len(game.player2.battle_area) == 1, \
            "Opponent's lv5 Digimon should NOT be deleted"

    def test_on_deletion_proceeds_without_hand_cards(self, debug_runner):
        """On Deletion should still delete opponent Digimon even if player
        has no hand cards to trash. The C# handles trash and deletion
        as independent checks — 'Then' does not gate on the previous step."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        game = runner.game
        p1 = game.player1

        # Clear P1's hand completely
        runner.clear_zone(1, "hand")
        assert len(p1.hand_cards) == 0, "P1 hand should be empty"

        # Place lv3 and lv4 on opponent's field
        runner.place_on_field(2, [LV3_ID])
        runner.place_on_field(2, [LV4_ID])

        p1.delete_permanent(perm)
        runner.auto_resolve()

        # Both should be deleted even though no hand card was trashed
        assert len(game.player2.battle_area) == 0, \
            f"Opponent Digimon should be deleted even with empty hand, " \
            f"but {len(game.player2.battle_area)} remain"

    def test_on_deletion_no_targets_no_crash(self, debug_runner):
        """On Deletion with no valid opponent targets should not crash."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        game = runner.game
        p1 = game.player1

        runner.inject_card(1, FILLER, "hand")

        # No opponent Digimon
        assert len(game.player2.battle_area) == 0

        p1.delete_permanent(perm)
        runner.auto_resolve()

        # Should complete without error
        assert len(p1.hand_cards) >= 0  # Just confirm no crash

    def test_on_deletion_only_lv3_target_deletes_lv3_only(self, debug_runner):
        """If opponent has only a lv3 Digimon (no lv4), delete just the lv3."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        game = runner.game
        p1 = game.player1

        runner.inject_card(1, FILLER, "hand")

        # Only a lv3 target
        runner.place_on_field(2, [LV3_ID])

        p1.delete_permanent(perm)
        runner.auto_resolve()

        assert len(game.player2.battle_area) == 0, \
            "Opponent's lv3 should be deleted"

    def test_on_deletion_only_lv4_target_deletes_lv4_only(self, debug_runner):
        """If opponent has only a lv4 Digimon (no lv3), delete just the lv4."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        perm = runner.place_on_field(1, [OROCHIMON_ID])
        game = runner.game
        p1 = game.player1

        runner.inject_card(1, FILLER, "hand")

        # Only a lv4 target
        runner.place_on_field(2, [LV4_ID])

        p1.delete_permanent(perm)
        runner.auto_resolve()

        assert len(game.player2.battle_area) == 0, \
            "Opponent's lv4 should be deleted"

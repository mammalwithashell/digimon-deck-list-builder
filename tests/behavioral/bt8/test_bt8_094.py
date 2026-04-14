"""Behavioral tests for BT8-094 Digimon Emperor (Tamer, White, Cost 3).

Card text:
  [All Turns] When one of your opponent's level 5 or lower Digimon is deleted,
  you may suspend this Tamer to <Draw 1>.
  [Opponent's Turn] When one of your opponent's level 3 Digimon is moved from
  their breeding area to their battle area, gain 2 memory.
  [Security] Play this card without paying the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT8094DrawOnDelete:
    """[All Turns] Opponent's Lv.5- Digimon deleted -> suspend to Draw 1."""

    def test_draw_on_opponent_digimon_deleted(self, debug_runner):
        """Deleting opponent's Lv.5 or lower Digimon triggers Draw 1."""
        runner = debug_runner(initial_memory=5)

        # Place Digimon Emperor on P1 field
        runner.place_on_field(1, ["BT8-094"])

        # Place a Lv.3 Digimon on P2 field
        runner.place_on_field(2, ["ST1-03"])  # Agumon Lv.3

        hand_before = len(runner.game.player1.hand_cards)

        # Delete opponent's Digimon
        p2_perm = runner.game.player2.battle_area[0]
        runner.game.player2.delete_permanent(p2_perm, is_opponent_effect=True)

        # Auto-resolve the optional trigger
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after >= hand_before + 1, (
            f"Should draw 1 when opponent's Lv.5- Digimon is deleted, "
            f"hand before={hand_before}, after={hand_after}"
        )

    def test_draw_on_opponent_lv5_deleted(self, debug_runner):
        """Lv.5 Digimon is still within the 'Lv.5 or lower' range."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["BT8-094"])
        runner.place_on_field(2, ["ST7-07"])  # RizeGreymon Lv.5

        hand_before = len(runner.game.player1.hand_cards)

        p2_perm = runner.game.player2.battle_area[0]
        runner.game.player2.delete_permanent(p2_perm, is_opponent_effect=True)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after >= hand_before + 1, (
            f"Should draw 1 when opponent's Lv.5 Digimon is deleted, "
            f"hand before={hand_before}, after={hand_after}"
        )

    def test_no_draw_on_opponent_lv6_deleted(self, debug_runner):
        """Lv.6 Digimon should NOT trigger the draw (> Lv.5)."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["BT8-094"])
        runner.place_on_field(2, ["BT1-084"])  # Omnimon Lv.7

        hand_before = len(runner.game.player1.hand_cards)

        p2_perm = runner.game.player2.battle_area[0]
        runner.game.player2.delete_permanent(p2_perm, is_opponent_effect=True)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before, (
            f"Should NOT draw when opponent's Lv.7 Digimon is deleted, "
            f"hand before={hand_before}, after={hand_after}"
        )

    def test_no_draw_on_own_digimon_deleted(self, debug_runner):
        """Own Digimon deleted should NOT trigger the draw."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["BT8-094"])
        runner.place_on_field(1, ["ST1-03"])  # Own Agumon Lv.3

        hand_before = len(runner.game.player1.hand_cards)

        # Delete own Digimon
        agumon = None
        for p in runner.game.player1.battle_area:
            if p.is_digimon:
                agumon = p
                break
        assert agumon is not None
        runner.game.player1.delete_permanent(agumon)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before, (
            f"Should NOT draw when own Digimon is deleted, "
            f"hand before={hand_before}, after={hand_after}"
        )

    def test_suspend_required_for_draw(self, debug_runner):
        """Tamer must be suspended after triggering the draw."""
        runner = debug_runner(initial_memory=5)

        de_perm = runner.place_on_field(1, ["BT8-094"])
        runner.place_on_field(2, ["ST1-03"])  # Opponent Agumon

        assert not de_perm.is_suspended, "Should start unsuspended"

        p2_perm = runner.game.player2.battle_area[0]
        runner.game.player2.delete_permanent(p2_perm, is_opponent_effect=True)
        runner.auto_resolve()

        assert de_perm.is_suspended, "Should be suspended after triggering draw"

    def test_no_draw_when_already_suspended(self, debug_runner):
        """Cannot trigger draw if Tamer is already suspended."""
        runner = debug_runner(initial_memory=5)

        de_perm = runner.place_on_field(1, ["BT8-094"], is_suspended=True)
        runner.place_on_field(2, ["ST1-03"])  # Opponent Agumon

        hand_before = len(runner.game.player1.hand_cards)

        p2_perm = runner.game.player2.battle_area[0]
        runner.game.player2.delete_permanent(p2_perm, is_opponent_effect=True)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before, (
            f"Should NOT draw when Tamer is already suspended, "
            f"hand before={hand_before}, after={hand_after}"
        )

    def test_effect_is_optional(self, debug_runner):
        """The effect should be optional (you MAY suspend)."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT8-094")
        effects = cs.effect_list(None)
        # Implemented as a deletion observer (NoTiming + _is_deletion_observer)
        observers = [e for e in effects if getattr(e, '_is_deletion_observer', False)]
        assert len(observers) >= 1, "Should have a deletion observer effect"
        assert observers[0].is_optional, "Effect should be optional"


@pytest.mark.behavioral
class TestBT8094MemoryOnMove:
    """[Opponent's Turn] Opponent's Lv.3 moves from breeding -> gain 2 memory."""

    def test_gain_memory_on_opponent_lv3_move(self, debug_runner):
        """When opponent moves Lv.3 from breeding, gain 2 memory."""
        runner = debug_runner(initial_memory=0, first_player=2)

        # Place Digimon Emperor on P1 field (P1 is not turn player)
        runner.place_on_field(1, ["BT8-094"])

        # Place Lv.3 in P2's breeding area and move it
        runner.place_in_breeding(2, ["BT1-001", "ST1-03"])  # Lv.3 Agumon
        runner.set_phase("Breeding")

        memory_before = runner.game.memory

        action = runner.find_action("Move")
        assert action is not None, "P2 should be able to move from breeding"
        runner.execute(action)
        runner.auto_resolve()

        # P1 gains 2 memory. P2 is turn player, so memory gauge is from P2's
        # perspective. P1 gaining memory means P2 loses it, so memory decreases.
        # add_memory(2) on non-turn-player => memory -= 2
        memory_after = runner.game.memory
        assert memory_after <= memory_before - 2, (
            f"P1 should gain 2 memory (P2 loses 2 as turn player), "
            f"before={memory_before}, after={memory_after}"
        )

    def test_no_memory_on_own_turn_move(self, debug_runner):
        """Effect is [Opponent's Turn] -- should not fire on own turn."""
        runner = debug_runner(initial_memory=3, first_player=1)

        runner.place_on_field(1, ["BT8-094"])

        # Check condition directly
        game = runner.game
        player1 = game.player1
        de_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT8-094":
                de_card = p.top_card
                break
        assert de_card is not None

        effects = de_card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove]
        assert len(on_move) >= 1

        # On P1's turn, condition should be False
        player1.is_my_turn = True
        result = on_move[0].can_use_condition({})
        assert result is False, "OnMove should not activate on own turn"

    def test_no_memory_on_lv4_move(self, debug_runner):
        """Effect requires specifically Lv.3; Lv.4 should not trigger."""
        runner = debug_runner(initial_memory=0, first_player=2)

        runner.place_on_field(1, ["BT8-094"])

        # Place a Lv.4 in breeding (egg + Lv.3 + Lv.4 stack)
        runner.place_in_breeding(2, ["BT1-001", "ST1-03", "ST1-05"])  # Lv.4 Greymon
        runner.set_phase("Breeding")

        memory_before = runner.game.memory

        action = runner.find_action("Move")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        memory_after = runner.game.memory
        # Should NOT gain 2 -- the moved perm is Lv.4
        assert memory_after == memory_before, (
            f"Should NOT gain memory when Lv.4 moves from breeding, "
            f"before={memory_before}, after={memory_after}"
        )


@pytest.mark.behavioral
class TestBT8094SecurityPlay:
    """[Security] Play this card without paying the cost."""

    def test_has_security_effect(self, debug_runner):
        """Should have a security effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT8-094")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security play effect"

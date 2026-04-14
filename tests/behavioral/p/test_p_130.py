"""Behavioral tests for P-130 Lui Ohwada (Tamer, White, Cost 3).

Card text:
  [On Play] You may move 1 of your level 3 or higher Digimon from the breeding
  area to the battle area.
  [Your Turn] When one of your Digimon moves from the breeding area to the
  battle area, by suspending this Tamer, gain 1 memory.
  [Security] Play this card without paying the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestP130OnPlayMoveFromBreeding:
    """[On Play] Move 1 Lv.3+ from breeding to battle area."""

    def test_on_play_moves_from_breeding(self, debug_runner):
        """Playing P-130 should move a Lv.3+ Digimon from breeding."""
        runner = debug_runner(initial_memory=5)

        # Put a Lv.3 in breeding
        runner.place_in_breeding(1, ["BT1-001", "ST1-03"])  # Agumon Lv.3
        runner.inject_card(1, "P-130", "hand")
        runner.set_phase("Main")

        breeding_before = runner.game.player1.breeding_area is not None
        assert breeding_before, "Should have a Digimon in breeding"

        play_action = runner.find_action("Lui Ohwada")
        if play_action is None:
            play_action = runner.find_action("P-130")
        assert play_action is not None, f"Should be able to play P-130. Available: {runner.actions()}"

        runner.execute(play_action)
        runner.auto_resolve()

        # After playing, breeding should be empty and Digimon in battle area
        breeding_after = runner.game.player1.breeding_area is not None
        assert not breeding_after, "Breeding should be empty after On Play move"

        # Agumon should be on field
        has_agumon = any(
            p.is_digimon and p.top_card and p.top_card.c_entity_base.card_id == "ST1-03"
            for p in runner.game.player1.battle_area
        )
        assert has_agumon, "Agumon should be on the battle area after move"

    def test_on_play_optional(self, debug_runner):
        """The On Play move is optional (you may)."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-130")
        effects = cs.effect_list(None)
        on_play_effects = [e for e in effects if getattr(e, 'is_on_play', False)]
        assert len(on_play_effects) >= 1, "Should have an On Play effect"
        assert on_play_effects[0].is_optional, "On Play move should be optional"

    def test_on_play_no_breeding_still_plays(self, debug_runner):
        """P-130 can still be played even with no breeding area Digimon."""
        runner = debug_runner(initial_memory=5)

        # No breeding area Digimon
        runner.inject_card(1, "P-130", "hand")
        runner.set_phase("Main")

        play_action = runner.find_action("Lui Ohwada")
        if play_action is None:
            play_action = runner.find_action("P-130")
        assert play_action is not None, "Should still be able to play P-130 with empty breeding"


@pytest.mark.behavioral
class TestP130MemoryOnMove:
    """[Your Turn] On move from breeding, suspend to gain 1 memory."""

    def test_gain_memory_on_move(self, debug_runner):
        """Moving from breeding triggers memory gain."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["P-130"])
        runner.place_in_breeding(1, ["BT1-001", "ST1-03"])  # Agumon Lv.3
        runner.set_phase("Breeding")

        memory_before = runner.game.memory

        action = runner.find_action("Move")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        memory_after = runner.game.memory
        assert memory_after >= memory_before + 1, (
            f"Should gain 1 memory on move from breeding, "
            f"before={memory_before}, after={memory_after}"
        )

    def test_suspend_required(self, debug_runner):
        """Tamer should be suspended after triggering memory gain."""
        runner = debug_runner(initial_memory=3)

        lui_perm = runner.place_on_field(1, ["P-130"])
        runner.place_in_breeding(1, ["BT1-001", "ST1-03"])
        runner.set_phase("Breeding")

        assert not lui_perm.is_suspended, "Should start unsuspended"

        action = runner.find_action("Move")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        assert lui_perm.is_suspended, "Should be suspended after triggering effect"

    def test_no_trigger_when_already_suspended(self, debug_runner):
        """Cannot trigger if already suspended."""
        runner = debug_runner(initial_memory=3)

        lui_perm = runner.place_on_field(1, ["P-130"], is_suspended=True)
        runner.place_in_breeding(1, ["BT1-001", "ST1-03"])
        runner.set_phase("Breeding")

        memory_before = runner.game.memory

        action = runner.find_action("Move")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        memory_after = runner.game.memory
        assert memory_after == memory_before, (
            f"Should NOT gain memory when Tamer is already suspended, "
            f"before={memory_before}, after={memory_after}"
        )

    def test_not_on_opponent_turn(self, debug_runner):
        """[Your Turn] restriction -- should not fire on opponent's turn."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["P-130"])

        game = runner.game
        player1 = game.player1
        lui_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "P-130":
                lui_card = p.top_card
                break
        assert lui_card is not None

        effects = lui_card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove]
        assert len(on_move) >= 1

        # On opponent's turn, condition should be False
        player1.is_my_turn = False
        result = on_move[0].can_use_condition({})
        assert result is False, "OnMove should not activate on opponent's turn"
        player1.is_my_turn = True


@pytest.mark.behavioral
class TestP130SecurityPlay:
    """[Security] Play this card without paying the cost."""

    def test_has_security_effect(self, debug_runner):
        """Should have a security play effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-130")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security play effect"

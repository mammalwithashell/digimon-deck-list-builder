"""Behavioral tests for BT5-106 Demonic Disaster (Option, Purple, Cost 1).

Card text:
[Main] You may delete 1 of your Digimon to unsuspend 1 of your purple Digimon.

[Security] You may play 1 level 3 purple Digimon card from your trash without
    paying its memory cost. Any [On Play] effects on Digimon played with this
    effect don't activate.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


FILLER_DECK = ["BT2-067"] * 50  # Purple Lv.3


@pytest.mark.behavioral
class TestBT5106DemonicDisaster:
    """Tests for BT5-106 Demonic Disaster."""

    # ── OptionSkill timing ──────────────────────────────────────────

    def test_option_has_option_skill_timing(self, debug_runner):
        """Main effect should use OptionSkill timing."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT5-106", runner.game.player1)
        effects = cs.effect_list(None)
        option_effects = [
            e for e in effects if e.timing == EffectTiming.OptionSkill
        ]
        assert len(option_effects) >= 1, "Should have OptionSkill timing effect"

    def test_main_effect_is_optional(self, debug_runner):
        """The delete+unsuspend effect should be optional."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT5-106", runner.game.player1)
        effects = cs.effect_list(None)
        option_effects = [
            e for e in effects if e.timing == EffectTiming.OptionSkill
        ]
        assert option_effects[0].is_optional, "Main effect should be optional"

    # ── Delete own + Unsuspend purple ───────────────────────────────

    def test_delete_own_then_unsuspend_purple(self, debug_runner):
        """[Main] Should delete 1 own Digimon and unsuspend 1 purple Digimon.

        The process callback creates two nested effect_select_own_permanent
        phases (both optional). We drive the selections manually since
        auto_resolve picks action 62 (decline) for optional selections.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Place 2 Digimon on P1 field:
        # One to delete (any Digimon), one purple suspended to unsuspend
        sacrifice = runner.place_on_field(1, ["ST1-03"])  # Agumon (Red, at slot 0)
        purple_mon = runner.place_on_field(1, ["BT2-067"], is_suspended=True)  # Purple, at slot 1

        game = runner.game
        player = game.player1

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT5-106", player)
        effects = cs.effect_list(None)
        option_effects = [
            e for e in effects if e.timing == EffectTiming.OptionSkill
        ]
        assert len(option_effects) >= 1

        # Execute the OptionSkill process callback -- creates first selection
        option_effects[0].on_process_callback({
            'player': player,
            'game': game,
        })

        # Game should now be in SelectTarget phase for choosing which Digimon to delete
        from engine_py_legacy.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget after process callback, got {game.current_phase}"
        )

        # Select Agumon (slot 0 = action 100) to delete
        runner.execute(100)

        # After deleting Agumon, the callback creates second selection:
        # select a purple suspended Digimon to unsuspend
        # The purple DemiDevimon (now slot 0 after deletion) should be selectable
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget for unsuspend, got {game.current_phase}"
        )

        # Select the purple Digimon (slot 0 = action 100) to unsuspend
        runner.execute(100)

        snap = runner.snapshot()
        p1_digimon = [f for f in snap.p1_field if f.is_digimon]

        # Only the purple Digimon should remain (Agumon was deleted)
        purple_remaining = [f for f in p1_digimon if f.card_id == "BT2-067"]
        assert len(purple_remaining) == 1, (
            f"Purple Digimon should remain on field, got {len(purple_remaining)}"
        )
        # And it should be unsuspended
        assert not purple_remaining[0].is_suspended, (
            "Purple Digimon should be unsuspended after effect"
        )

    # ── Security effect ─────────────────────────────────────────────

    def test_security_effect_exists(self, debug_runner):
        """Should have a SecuritySkill effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT5-106", runner.game.player1)
        effects = cs.effect_list(None)
        security_effects = [
            e for e in effects if e.timing == EffectTiming.SecuritySkill
        ]
        assert len(security_effects) >= 1, "Should have SecuritySkill effect"
        assert security_effects[0].is_security_effect, "Should be marked as security effect"

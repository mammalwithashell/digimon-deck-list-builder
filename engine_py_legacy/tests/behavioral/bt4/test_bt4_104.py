"""Behavioral tests for BT4-104 Blinding Ray (Option, Yellow, Cost 0).

Card text:
  [Main] Trash the top card of your security stack. Then, gain 2 memory.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT4104BlindingRay:
    """Tests for BT4-104 Blinding Ray."""

    # ── Shape tests ─────────────────────────────────────────

    def test_has_option_skill_timing(self, debug_runner):
        """Should expose an OptionSkill [Main] effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT4-104")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) == 1, "Should have exactly 1 OptionSkill effect"
        assert main_effects[0].on_process_callback is not None, (
            "OptionSkill effect should have a process callback"
        )

    def test_no_inherited_effects(self, debug_runner):
        """BT4-104 is a pure Option with no inherited text."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT4-104")
        effects = cs.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect]
        assert len(inherited) == 0, (
            f"BT4-104 should have no inherited effects, found {len(inherited)}"
        )

    def test_condition_true_with_security(self, debug_runner):
        """can_use_condition should be True when security stack is non-empty."""
        runner = debug_runner(initial_memory=10)
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT4-104", runner.game.player1)
        effects = cs.effect_list(None)
        main_eff = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]
        # Security stack is populated at debug_runner setup
        assert len(runner.game.player1.security_cards) > 0
        assert main_eff.can_use_condition({"player": runner.game.player1, "game": runner.game}) is True

    # ── Behavior tests ──────────────────────────────────────

    def test_trashes_top_security_card(self, debug_runner):
        """Process should trash the TOP security card (index 0)."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        p1 = game.player1

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT4-104", p1)
        effects = cs.effect_list(None)
        main_eff = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]

        assert len(p1.security_cards) >= 5, "Security should start populated"
        initial_sec_count = len(p1.security_cards)
        top_card_before = p1.security_cards[0]
        initial_trash_count = len(p1.trash_cards)

        main_eff.on_process_callback({
            "player": p1,
            "game": game,
            "card_source": cs,
        })

        assert len(p1.security_cards) == initial_sec_count - 1, (
            f"Security should shrink by 1, got {len(p1.security_cards)} "
            f"(was {initial_sec_count})"
        )
        assert top_card_before not in p1.security_cards, (
            "The previously-top security card should no longer be in security"
        )
        assert top_card_before in p1.trash_cards, (
            "The trashed top security card should be in the trash"
        )
        assert len(p1.trash_cards) == initial_trash_count + 1, (
            "Trash should grow by exactly 1"
        )

    def test_gain_2_memory(self, debug_runner):
        """Process should gain 2 memory for the active player."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        p1 = game.player1

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT4-104", p1)
        effects = cs.effect_list(None)
        main_eff = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]

        # add_memory signs the value relative to the active player.
        memory_before = game.memory if p1.is_my_turn else -game.memory

        main_eff.on_process_callback({
            "player": p1,
            "game": game,
            "card_source": cs,
        })

        memory_after = game.memory if p1.is_my_turn else -game.memory
        assert memory_after - memory_before == 2, (
            f"Expected +2 memory, got delta={memory_after - memory_before}"
        )

    def test_fires_discard_security_observer(self, debug_runner):
        """Trashing the top security card must fire OnDiscardSecurity + OnLoseSecurity
        observers (i.e. use trash_security_card, not raw list pop)."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        p1 = game.player1

        fired = {"discard": 0, "lose": 0}
        original_fire = p1._fire_timing

        def _spy(timing, ctx):
            if timing == EffectTiming.OnDiscardSecurity:
                fired["discard"] += 1
            elif timing == EffectTiming.OnLoseSecurity:
                fired["lose"] += 1
            return original_fire(timing, ctx)

        p1._fire_timing = _spy

        try:
            from engine_py_legacy.engine.data.card_database import CardDatabase
            db = CardDatabase()
            cs = db.create_card_source("BT4-104", p1)
            effects = cs.effect_list(None)
            main_eff = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]
            main_eff.on_process_callback({
                "player": p1,
                "game": game,
                "card_source": cs,
            })
        finally:
            p1._fire_timing = original_fire

        assert fired["discard"] == 1, (
            f"OnDiscardSecurity should have fired exactly once, got {fired['discard']} "
            "(likely using raw security_cards.pop — bypasses observers)"
        )
        assert fired["lose"] == 1, (
            f"OnLoseSecurity should have fired exactly once, got {fired['lose']}"
        )

    def test_trash_happens_before_memory_gain(self, debug_runner):
        """Card text says 'Then, gain 2 memory' — trash must resolve first so any
        OnDiscardSecurity triggers fire before memory is added."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        p1 = game.player1

        ordering = []
        original_fire = p1._fire_timing
        original_add_memory = p1.add_memory

        def _fire_spy(timing, ctx):
            if timing == EffectTiming.OnDiscardSecurity:
                ordering.append("trash_security")
            return original_fire(timing, ctx)

        def _mem_spy(amount):
            ordering.append("add_memory")
            return original_add_memory(amount)

        p1._fire_timing = _fire_spy
        p1.add_memory = _mem_spy

        try:
            from engine_py_legacy.engine.data.card_database import CardDatabase
            db = CardDatabase()
            cs = db.create_card_source("BT4-104", p1)
            effects = cs.effect_list(None)
            main_eff = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]
            main_eff.on_process_callback({
                "player": p1,
                "game": game,
                "card_source": cs,
            })
        finally:
            p1._fire_timing = original_fire
            p1.add_memory = original_add_memory

        assert ordering == ["trash_security", "add_memory"], (
            f"Expected trash before memory gain, got order={ordering}"
        )

    def test_no_op_when_security_empty(self, debug_runner):
        """If security is empty, no card is trashed but memory still granted per
        card text (card does not require security to exist)."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        p1 = game.player1

        # Empty out security stack
        p1.security_cards.clear()
        p1.face_up_security.clear()

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT4-104", p1)
        effects = cs.effect_list(None)
        main_eff = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]

        memory_before = game.memory if p1.is_my_turn else -game.memory
        trash_before = len(p1.trash_cards)

        # Should not raise
        main_eff.on_process_callback({
            "player": p1,
            "game": game,
            "card_source": cs,
        })

        memory_after = game.memory if p1.is_my_turn else -game.memory
        assert memory_after - memory_before == 2, (
            "Memory should still gain +2 even if no security to trash"
        )
        assert len(p1.trash_cards) == trash_before, (
            "Nothing should be trashed when security is empty"
        )

    # ── End-to-end play test ────────────────────────────────

    def test_end_to_end_play_from_hand(self, debug_runner):
        """Playing Blinding Ray from hand during Main should trash top security
        and grant 2 memory."""
        runner = debug_runner(initial_memory=0, skip_shuffle=True)
        runner.set_phase("Main")

        # Color requirement: need a yellow source on field (BT1-048 Patamon, Yellow Lv.3)
        runner.place_on_field(1, ["BT1-048"])
        runner.inject_card(1, "BT4-104", "hand")

        game = runner.game
        p1 = game.player1
        initial_sec_count = len(p1.security_cards)
        initial_trash_count = len(p1.trash_cards)
        top_before = p1.security_cards[0]
        memory_before = game.memory if p1.is_my_turn else -game.memory

        action = runner.find_action("Blinding Ray")
        if action is None:
            action = runner.find_action("BT4-104")
        assert action is not None, (
            f"Should be able to play BT4-104. Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        assert len(p1.security_cards) == initial_sec_count - 1, (
            f"Security should shrink by 1, got {len(p1.security_cards)}"
        )
        assert top_before in p1.trash_cards, (
            "Previously-top security card should be in trash"
        )
        assert len(p1.trash_cards) >= initial_trash_count + 1, (
            "At least one new trash entry from trashed security"
        )
        memory_after = game.memory if p1.is_my_turn else -game.memory
        # Option card cost is 0, so net memory change is +2 (from gain)
        # minus 0 (cost) = +2. Playing the card uses the normal +cost mechanic.
        assert memory_after - memory_before >= 2, (
            f"Memory should gain +2 from effect (cost is 0). "
            f"Delta={memory_after - memory_before}"
        )

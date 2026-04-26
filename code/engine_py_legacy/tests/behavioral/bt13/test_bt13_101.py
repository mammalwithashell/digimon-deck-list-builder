"""Behavioral tests for BT13-101 Miki Kurosaki & Megumi Shirakawa (Tamer, Yellow/Black, Cost 4).

Actual card text (from cards.json):
[On Play] You may play 1 Digimon card with [PawnChessmon] in its name from your hand
    without paying its cost.
[All Turns] When you play a 2-color black and yellow Digimon, by suspending this Tamer,
    Draw 1 and gain 1 memory.
Security Effect: [Security] Play this card without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT13101MikiKurosakiMegumiShirakawa:
    """Tests for BT13-101 Miki Kurosaki & Megumi Shirakawa."""

    # ------------------------------------------------------------------
    # [On Play] Play 1 PawnChessmon from hand without paying cost
    # ------------------------------------------------------------------

    def test_on_play_effect_exists(self, debug_runner):
        """Should have an On Play effect for playing PawnChessmon from hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-101"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        on_play_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play_effects) >= 1, "Should have at least 1 On Play effect"

    def test_on_play_is_optional(self, debug_runner):
        """On Play PawnChessmon play should be optional ('You may')."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-101"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        on_play = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
            and not getattr(e, 'is_inherited_effect', False)
        ]
        # The first On Play effect (PawnChessmon play) should be optional
        pawnchessmon_effect = on_play[0]
        assert pawnchessmon_effect.is_optional, \
            "On Play PawnChessmon play should be optional ('You may')"

    def test_on_play_triggers_pawnchessmon_play(self, debug_runner):
        """On Play should offer to play a PawnChessmon from hand for free."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-101"])

        # Put a PawnChessmon (BT13-035, Lv.3 Black/Yellow) in player 1's hand
        runner.inject_card(1, "BT13-035", "hand")

        game = runner.game
        top = perm.top_card
        all_effects = top.effect_list(None)
        on_play = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        # Execute the effect
        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Auto-resolve selections
        runner.auto_resolve()

        # PawnChessmon should now be on the field
        field_ids = [
            p.top_card.c_entity_base.card_id
            for p in game.player1.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert "BT13-035" in field_ids, \
            "PawnChessmon should be played from hand to field"

    def test_on_play_only_targets_pawnchessmon(self, debug_runner):
        """On Play should only allow playing cards with PawnChessmon in name."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-101"])

        # Inject a non-PawnChessmon Digimon into hand
        runner.inject_card(1, "ST1-03", "hand")  # Agumon - not PawnChessmon

        game = runner.game
        top = perm.top_card
        all_effects = top.effect_list(None)
        on_play = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        # Count field before
        field_count_before = len(game.player1.battle_area)

        # Execute the effect
        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # No new Digimon should be played (Agumon is not PawnChessmon)
        non_tamer_field = [p for p in game.player1.battle_area if p.is_digimon]
        assert all(
            p.top_card.c_entity_base.card_id != "ST1-03"
            for p in non_tamer_field
        ), "Agumon should NOT be playable via PawnChessmon effect"

    # ------------------------------------------------------------------
    # [All Turns] 2-color Black+Yellow Digimon trigger
    # ------------------------------------------------------------------

    def test_all_turns_observer_condition_checks_2color_black_yellow(self, debug_runner):
        """All Turns effect should check that the played card is a 2-color black+yellow Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-101"])

        game = runner.game
        top = perm.top_card
        all_effects = top.effect_list(None)

        # Find the All Turns observer effect (second OnEnterFieldAnyone)
        observer_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(observer_effects) >= 2, \
            "Should have at least 2 OnEnterFieldAnyone effects (On Play + All Turns observer)"

        observer = observer_effects[1]

        # Create a mock played_card with Black+Yellow colors
        # BT13-035 PawnChessmon is Black/Yellow Digimon
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        pawnchessmon_cs = db.create_card_source("BT13-035", game.player1)

        # Should pass with a 2-color black+yellow Digimon
        result = observer.can_use_condition({
            'played_card': pawnchessmon_cs,
            'event_player': game.player1,
        })
        assert result, "Should trigger when a 2-color black+yellow Digimon is played"

    def test_all_turns_observer_rejects_non_black_yellow(self, debug_runner):
        """All Turns effect should NOT trigger for non-black+yellow Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-101"])

        game = runner.game
        top = perm.top_card
        all_effects = top.effect_list(None)

        observer_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        observer = observer_effects[1]

        # ST1-03 Agumon is Red only
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        agumon_cs = db.create_card_source("ST1-03", game.player1)

        result = observer.can_use_condition({
            'played_card': agumon_cs,
            'event_player': game.player1,
        })
        assert not result, "Should NOT trigger for a non-black+yellow Digimon"

    def test_all_turns_requires_tamer_not_suspended(self, debug_runner):
        """All Turns effect should require tamer to not be already suspended."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-101"], is_suspended=True)

        game = runner.game
        top = perm.top_card
        all_effects = top.effect_list(None)

        observer_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        observer = observer_effects[1]

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        pawnchessmon_cs = db.create_card_source("BT13-035", game.player1)

        result = observer.can_use_condition({
            'played_card': pawnchessmon_cs,
            'event_player': game.player1,
        })
        assert not result, "Should NOT trigger when tamer is already suspended"

    def test_all_turns_draws_and_gains_memory(self, debug_runner):
        """All Turns effect should Draw 1 and gain 1 memory when it resolves."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-101"])

        game = runner.game
        hand_before = len(game.player1.hand_cards)
        memory_before = game.memory

        top = perm.top_card
        all_effects = top.effect_list(None)
        observer_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        observer = observer_effects[1]

        observer.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should have drawn 1 card
        assert len(game.player1.hand_cards) == hand_before + 1, \
            "Should draw 1 card"
        # Should have gained 1 memory
        assert game.memory == memory_before + 1, \
            "Should gain 1 memory"
        # Tamer should now be suspended
        assert perm.is_suspended, "Tamer should be suspended after effect"

    # ------------------------------------------------------------------
    # Security effect
    # ------------------------------------------------------------------

    def test_security_effect_exists(self, debug_runner):
        """Should have a security effect to play this card without paying cost."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-101"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        security_effects = [
            e for e in all_effects
            if getattr(e, 'is_security_effect', False)
        ]
        assert len(security_effects) >= 1, "Should have a security effect"

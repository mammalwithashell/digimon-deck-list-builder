"""Tests for structured GameEvent emission.

Validates that key game actions emit the correct event types
and that the SilentLogger path doesn't break.
"""

import os
import sys
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from digimon_gym.engine.game import Game
from digimon_gym.engine.events import GameEvent
from digimon_gym.engine.loggers import EventLogger, SilentLogger
from digimon_gym.engine.data.enums import GamePhase, CardKind, CardColor

from tests.helpers.game_builder import GameBuilder, make_card, make_permanent


class TestEventEmission:
    """Verify that key actions emit structured events."""

    def test_phase_change_event(self):
        """phase_start should emit a phase_change event."""
        logger = EventLogger()
        game = Game(logger=logger)
        game.turn_count = 1
        game.phase_start()

        events = logger.get_events()
        phase_events = [e for e in events if e["type"] == "phase_change"]
        assert len(phase_events) >= 1
        assert phase_events[0]["meta"]["phase"] == "Start"

    def test_game_over_event(self):
        """declare_winner should emit a game_over event."""
        logger = EventLogger()
        game = Game(logger=logger)
        game.declare_winner(game.player1)

        events = logger.get_events()
        game_over_events = [e for e in events if e["type"] == "game_over"]
        assert len(game_over_events) == 1
        assert game_over_events[0]["meta"]["winner"] == 1

    def test_effect_activate_event(self):
        """_log_effect_activation should emit an effect_activate event."""
        from digimon_gym.engine.interfaces.card_effect import ICardEffect
        from digimon_gym.engine.data.enums import EffectTiming

        logger = EventLogger()
        game = Game(logger=logger)

        effect = ICardEffect()
        effect.set_effect_name("TestEffect")
        src_card = make_card(card_id="BT1-001", name="Agumon")
        effect.effect_source_card = src_card

        game._log_effect_activation(effect, EffectTiming.OnAllyAttack)

        events = logger.get_events()
        eff_events = [e for e in events if e["type"] == "effect_activate"]
        assert len(eff_events) == 1
        assert eff_events[0]["meta"]["effect_name"] == "TestEffect"
        assert eff_events[0]["meta"]["timing"] == "OnAllyAttack"
        assert eff_events[0]["source_card_id"] == "BT1-001"

    def test_event_sequence_is_monotonic(self):
        """Event seq numbers should be monotonically increasing."""
        logger = EventLogger()
        game = Game(logger=logger)
        game.turn_count = 1

        game.phase_start()
        game.declare_winner(game.player1)

        events = logger.get_events()
        seqs = [e["seq"] for e in events]
        assert seqs == sorted(seqs)
        assert len(set(seqs)) == len(seqs)  # All unique

    def test_event_to_dict_roundtrip(self):
        """GameEvent.to_dict() should produce a clean serializable dict."""
        event = GameEvent(
            type="play_card",
            seq=0,
            player=1,
            source_card_id="BT1-001",
            source_slot=3,
            meta={"cost": 5, "card_name": "Agumon"},
        )
        d = event.to_dict()
        assert d["type"] == "play_card"
        assert d["source_card_id"] == "BT1-001"
        assert d["meta"]["cost"] == 5


class TestSilentLoggerSafety:
    """Ensure the SilentLogger path doesn't break with event emissions."""

    def test_silent_logger_ignores_emit(self):
        """Game with SilentLogger should work fine — _emit is a no-op."""
        game = Game(logger=SilentLogger())
        game.turn_count = 1
        game.phase_start()
        game.declare_winner(game.player1)
        # Should not raise — SilentLogger has no 'emit' method,
        # and _emit checks with hasattr before calling

    def test_silent_logger_no_events(self):
        """SilentLogger.get_logs() returns empty, no events attr."""
        logger = SilentLogger()
        assert logger.get_logs() == []
        assert not hasattr(logger, 'get_events')


class TestEventLoggerClear:
    """Test EventLogger clear behavior."""

    def test_clear_events_only(self):
        """clear_events() should not affect string logs."""
        logger = EventLogger()
        logger.log("test message")
        logger.emit(GameEvent(type="test", seq=0))

        logger.clear_events()
        assert len(logger.get_logs()) == 1
        assert len(logger.get_events()) == 0

    def test_clear_clears_both(self):
        """clear() should clear both logs and events."""
        logger = EventLogger()
        logger.log("test message")
        logger.emit(GameEvent(type="test", seq=0))

        logger.clear()
        assert len(logger.get_logs()) == 0
        assert len(logger.get_events()) == 0

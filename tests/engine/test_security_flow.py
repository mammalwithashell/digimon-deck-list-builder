"""Tests for security check event emission.

Validates that security_reveal events are emitted during security checks
with correct metadata.
"""

import pytest

from digimon_gym.engine.game import Game
from digimon_gym.engine.loggers import EventLogger
from digimon_gym.engine.data.enums import GamePhase, CardKind, CardColor, AttackResolution

from tests.helpers.game_builder import GameBuilder, make_card, make_permanent


class TestSecurityRevealEvent:
    """Verify security_reveal events during security checks."""

    def test_security_reveal_emitted(self):
        """A security check should emit a security_reveal event."""
        game = (
            GameBuilder()
            .with_player1_field(make_permanent("ATK-001", "Attacker", dp=10000))
            .with_player2_security(
                make_card("SEC-001", "SecurityMon", dp=3000),
                make_card("SEC-002", "SecurityMon2", dp=3000),
            )
            .with_memory(3)
            .build()
        )

        attacker = game.player1.battle_area[0]
        result = game.player2.security_attack(attacker)

        events = game.logger.get_events()
        reveal_events = [e for e in events if e["type"] == "security_reveal"]

        assert len(reveal_events) == 1
        assert reveal_events[0]["meta"]["revealed_card_id"] == "SEC-001"
        assert reveal_events[0]["meta"]["revealed_card_name"] == "SecurityMon"
        assert reveal_events[0]["meta"]["security_remaining"] == 1

    def test_security_battle_emitted(self):
        """A security battle should emit a security_battle event."""
        sec_card = make_card("SEC-D01", "SecDigimon", dp=8000, kind=CardKind.Digimon)
        game = (
            GameBuilder()
            .with_player1_field(make_permanent("ATK-001", "Attacker", dp=10000))
            .with_player2_security(sec_card)
            .with_memory(3)
            .build()
        )

        attacker = game.player1.battle_area[0]
        result = game.player2.security_attack(attacker)

        events = game.logger.get_events()
        battle_events = [e for e in events if e["type"] == "security_battle"]

        assert len(battle_events) == 1
        assert battle_events[0]["meta"]["attacker_dp"] == 10000
        assert battle_events[0]["meta"]["defender_dp"] == 8000
        assert battle_events[0]["meta"]["result"] == "attacker_survives"

    def test_security_battle_attacker_loses(self):
        """When attacker loses, security_battle event should show attacker_deleted."""
        sec_card = make_card("SEC-D02", "StrongSecDigimon", dp=15000, kind=CardKind.Digimon)
        game = (
            GameBuilder()
            .with_player1_field(make_permanent("ATK-001", "WeakAttacker", dp=3000))
            .with_player2_security(sec_card)
            .with_memory(3)
            .build()
        )

        attacker = game.player1.battle_area[0]
        result = game.player2.security_attack(attacker)

        assert result == AttackResolution.AttackerDeleted

        events = game.logger.get_events()
        battle_events = [e for e in events if e["type"] == "security_battle"]
        assert len(battle_events) == 1
        assert battle_events[0]["meta"]["result"] == "attacker_deleted"

    def test_no_security_cards_no_reveal_event(self):
        """Direct attack (no security) should not emit security_reveal."""
        game = (
            GameBuilder()
            .with_player1_field(make_permanent("ATK-001", "Attacker", dp=10000))
            .with_memory(3)
            .build()
        )

        attacker = game.player1.battle_area[0]
        result = game.player2.security_attack(attacker)

        assert result == AttackResolution.GameEnd

        events = game.logger.get_events()
        reveal_events = [e for e in events if e["type"] == "security_reveal"]
        assert len(reveal_events) == 0


class TestDeleteEvent:
    """Verify delete events are emitted."""

    def test_delete_event_emitted(self):
        """Deleting a permanent should emit a delete event."""
        game = (
            GameBuilder()
            .with_player1_field(make_permanent("DEL-001", "DeleteMe", dp=5000))
            .build()
        )

        perm = game.player1.battle_area[0]
        was_deleted = game.player1.delete_permanent(perm, is_battle=True)

        assert was_deleted
        events = game.logger.get_events()
        delete_events = [e for e in events if e["type"] == "delete"]
        assert len(delete_events) == 1
        assert delete_events[0]["meta"]["card_name"] == "DeleteMe"
        assert delete_events[0]["meta"]["reason"] == "battle"

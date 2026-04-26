"""Behavioral tests for the engine modifier system.

These tests use both the Game class directly (via GameBuilder) and DebugRunner
to verify modifier registration, querying, stacking, and action mask effects.

Tests cover:
- CHANGE_SECURITY_ATTACK stacking (multiple modifiers add up)
- CANNOT_ATTACK_PLAYER restricts to Digimon-only targets
- FORCE_ATTACK removes pass action (forced to attack)
- Modifier expiry behavior (end_of_turn vs permanent)
- Source cleanup when a permanent leaves
"""

import pytest

from engine_py_legacy.engine.game import Game
from engine_py_legacy.engine.loggers import EventLogger
from engine_py_legacy.engine.core.permanent import Permanent
from engine_py_legacy.engine.data.enums import GamePhase, CardKind, CardColor
from engine_py_legacy.engine.interfaces.modifiers import ModifierType, ModifierEntry
from engine_py_legacy.tests.helpers.game_builder import make_card, make_permanent, GameBuilder


class TestChangeSecurityAttack:
    """Tests for CHANGE_SECURITY_ATTACK modifier stacking."""

    def test_single_sa_modifier(self):
        """A single CHANGE_SECURITY_ATTACK modifier should apply its value."""
        game = GameBuilder().with_memory(10).build()
        perm = make_permanent(dp=5000, level=4)
        perm._owner_game = game
        game.player1.battle_area.append(perm)

        game.register_modifier(
            perm,
            ModifierType.CHANGE_SECURITY_ATTACK,
            value_fn=lambda base, target, ctx: base + 1,
        )

        result = game.modifiers.get_int_modifier(
            perm, ModifierType.CHANGE_SECURITY_ATTACK, base_value=0
        )
        assert result == 1, (
            f"Single +1 SA modifier should produce 1, got {result}"
        )

    def test_sa_modifiers_stack_additively(self):
        """Multiple CHANGE_SECURITY_ATTACK modifiers should stack."""
        game = GameBuilder().with_memory(10).build()
        perm = make_permanent(dp=5000, level=4)
        perm._owner_game = game
        game.player1.battle_area.append(perm)

        game.register_modifier(
            perm,
            ModifierType.CHANGE_SECURITY_ATTACK,
            value_fn=lambda base, target, ctx: base + 1,
        )
        game.register_modifier(
            perm,
            ModifierType.CHANGE_SECURITY_ATTACK,
            value_fn=lambda base, target, ctx: base + 1,
        )

        result = game.modifiers.get_int_modifier(
            perm, ModifierType.CHANGE_SECURITY_ATTACK, base_value=0
        )
        assert result == 2, (
            f"Two +1 SA modifiers should stack to 2, got {result}"
        )

    def test_sa_modifier_with_negative(self):
        """A negative SA modifier should reduce the value."""
        game = GameBuilder().with_memory(10).build()
        perm = make_permanent(dp=5000, level=4)
        perm._owner_game = game
        game.player1.battle_area.append(perm)

        game.register_modifier(
            perm,
            ModifierType.CHANGE_SECURITY_ATTACK,
            value_fn=lambda base, target, ctx: base + 2,
        )
        game.register_modifier(
            perm,
            ModifierType.CHANGE_SECURITY_ATTACK,
            value_fn=lambda base, target, ctx: base - 1,
        )

        result = game.modifiers.get_int_modifier(
            perm, ModifierType.CHANGE_SECURITY_ATTACK, base_value=0
        )
        assert result == 1, (
            f"+2 and -1 SA modifiers should net to 1, got {result}"
        )


class TestCannotAttackPlayer:
    """Tests for CANNOT_ATTACK_PLAYER modifier."""

    def test_cannot_attack_player_blocks_security_attack(self):
        """A Digimon with CANNOT_ATTACK_PLAYER should not be able to attack the player."""
        game = GameBuilder().with_memory(10).in_phase(GamePhase.Main).build()
        perm = make_permanent(dp=5000, level=4)
        perm._owner_game = game
        perm.turn_played = -1
        game.player1.battle_area.append(perm)

        assert perm.can_attack_player(), (
            "Should be able to attack player before modifier"
        )

        game.register_modifier(
            perm,
            ModifierType.CANNOT_ATTACK_PLAYER,
        )

        assert not perm.can_attack_player(), (
            "Should NOT be able to attack player with CANNOT_ATTACK_PLAYER modifier"
        )

    def test_cannot_attack_player_still_allows_digimon_attack(self):
        """CANNOT_ATTACK_PLAYER should not prevent attacking opponent Digimon."""
        game = GameBuilder().with_memory(10).in_phase(GamePhase.Main).build()
        perm = make_permanent(dp=5000, level=4)
        perm._owner_game = game
        perm.turn_played = -1
        game.player1.battle_area.append(perm)

        game.register_modifier(
            perm,
            ModifierType.CANNOT_ATTACK_PLAYER,
        )

        assert perm.can_attack(), (
            "CANNOT_ATTACK_PLAYER should not prevent attacking in general, "
            "only attacking the player directly"
        )


class TestForceAttack:
    """Tests for FORCE_ATTACK modifier: removes pass action, forces Digimon to attack."""

    def test_force_attack_removes_pass(self, debug_runner):
        """When FORCE_ATTACK is active on a Digimon that can attack, the pass action
        should be removed from the action mask."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        # Place an attacker that can attack
        perm = runner.place_on_field(1, ["ST1-05"], turn_played=-1)  # Birdramon

        before = runner.snapshot()
        assert before.p2_security_count > 0, "P2 needs security"

        # Register FORCE_ATTACK modifier
        runner.game.register_modifier(
            perm,
            ModifierType.FORCE_ATTACK,
            condition=lambda target, ctx: target is perm,
        )

        actions = runner.actions()
        assert 62 not in actions, (
            f"Pass action (62) should not be available when FORCE_ATTACK is active. "
            f"Available actions: {actions}"
        )
        attack_actions = runner.find_actions("Attack")
        assert len(attack_actions) > 0, (
            "Attack actions should still be available under FORCE_ATTACK"
        )

    def test_force_attack_only_forces_specific_digimon(self, debug_runner):
        """FORCE_ATTACK should only restrict actions for the affected Digimon.
        If the forced Digimon has valid attack targets, only its attacks are available."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        perm1 = runner.place_on_field(1, ["ST1-05"], turn_played=-1)  # Birdramon
        runner.place_on_field(1, ["ST1-02"], turn_played=-1)  # Biyomon (not forced)

        runner.game.register_modifier(
            perm1,
            ModifierType.FORCE_ATTACK,
            condition=lambda target, ctx: target is perm1,
        )

        actions = runner.actions()
        # Under FORCE_ATTACK the mask should only contain attack actions for
        # the forced Digimon
        attack_actions = runner.find_actions("Attack")
        assert all("Birdramon" in desc for desc in attack_actions.values()), (
            f"Only Birdramon attacks should be available, got: {attack_actions}"
        )


class TestModifierExpiry:
    """Tests for modifier expiry behavior."""

    def test_end_of_turn_expiry_clears(self):
        """Modifiers with 'end_of_turn' expiry should be clearable."""
        game = GameBuilder().with_memory(10).build()
        perm = make_permanent(dp=5000, level=4)
        perm._owner_game = game
        game.player1.battle_area.append(perm)

        game.register_modifier(
            perm,
            ModifierType.CANNOT_BE_DESTROYED,
            expiry='end_of_turn',
        )

        assert game.modifiers.has_modifier(perm, ModifierType.CANNOT_BE_DESTROYED), (
            "Modifier should be active before expiry"
        )

        game.modifiers.clear_expiry('end_of_turn')

        assert not game.modifiers.has_modifier(perm, ModifierType.CANNOT_BE_DESTROYED), (
            "Modifier should be cleared after end_of_turn expiry"
        )

    def test_permanent_expiry_survives_turn_end(self):
        """Modifiers with 'permanent' expiry should NOT be cleared at turn end."""
        game = GameBuilder().with_memory(10).build()
        perm = make_permanent(dp=5000, level=4)
        perm._owner_game = game
        game.player1.battle_area.append(perm)

        game.register_modifier(
            perm,
            ModifierType.CANNOT_BE_DESTROYED,
            expiry='permanent',
        )

        game.modifiers.clear_expiry('end_of_turn')

        assert game.modifiers.has_modifier(perm, ModifierType.CANNOT_BE_DESTROYED), (
            "Permanent modifier should survive end_of_turn clearing"
        )

    def test_unregister_by_source(self):
        """Removing a source permanent should clean up all its modifiers."""
        game = GameBuilder().with_memory(10).build()
        source_perm = make_permanent(card_id="SRC-001", dp=5000, level=4)
        target_perm = make_permanent(card_id="TGT-001", dp=5000, level=4)
        source_perm._owner_game = game
        target_perm._owner_game = game
        game.player1.battle_area.extend([source_perm, target_perm])

        entry = ModifierEntry(
            modifier_type=ModifierType.CHANGE_DP,
            value_fn=lambda base, target, ctx: base + 2000,
            source_permanent=source_perm,
        )
        game.modifiers.register(entry)

        assert game.modifiers.has_modifier(target_perm, ModifierType.CHANGE_DP), (
            "Modifier should be active before source removal"
        )

        game.modifiers.unregister_by_source(source_perm)

        assert not game.modifiers.has_modifier(target_perm, ModifierType.CHANGE_DP), (
            "Modifier should be removed when source permanent is unregistered"
        )

"""Behavioral tests for BT9-103 Kongou (Option, Black, Cost 2).

Card text:
[Main] Until the end of your opponent's turn, your opponent's Digimon
    with play costs of 7 or less can't attack players, and cards can't
    be added to security stacks by your opponent's effects.
[Security] Activate this card's [Main] effects.

Key behaviors:
1. Restriction is CANNOT_ATTACK_PLAYER (not general CANNOT_ATTACK)
   - Affected Digimon can still attack other Digimon
2. The cost check is dynamic — applies to ALL opponent Digimon with cost <= 7,
   including ones played AFTER Kongou resolves
3. Opponent's Digimon with cost > 7 are NOT restricted
4. CANNOT_ADD_SECURITY prevents opponent's effects from adding cards to security
5. Duration is "until end of opponent's turn"
6. Security effect activates the same Main effect
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.modifiers import ModifierType, ModifierEntry


@pytest.mark.behavioral
class TestBT9103Kongou:
    """Tests for BT9-103 Kongou."""

    def _activate_kongou(self, runner, player_id=1):
        """Helper: place Kongou on field and fire its Main effect."""
        game = runner.game
        player = game.player1 if player_id == 1 else game.player2

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT9-103", player)
        effects = cs.effect_list(None)
        main_effects = [
            e for e in effects
            if e.timing == EffectTiming.OptionSkill
            and e.on_process_callback is not None
        ]
        assert len(main_effects) >= 1, "Should have OptionSkill (Main) effect"
        main_effects[0].on_process_callback({
            'player': player,
            'game': game,
        })
        return cs

    # --- Test 1: CANNOT_ATTACK_PLAYER (not general CANNOT_ATTACK) ---

    def test_cost_7_digimon_cannot_attack_player(self, debug_runner):
        """Opponent's Digimon with play cost <= 7 can't attack players after Kongou."""
        runner = debug_runner(initial_memory=5)

        # Opponent has a cost-7 Digimon (ST1-09 MetalGreymon, cost 7)
        perm = runner.place_on_field(2, ["ST1-09"])
        # Give it time so no summoning sickness
        perm.turn_played = -1

        self._activate_kongou(runner)

        # The Digimon should NOT be able to attack players
        assert not perm.can_attack_player(), (
            "Cost-7 Digimon should NOT be able to attack players after Kongou"
        )

    def test_cost_7_digimon_can_still_attack_digimon(self, debug_runner):
        """Affected Digimon can still attack — just not players."""
        runner = debug_runner(initial_memory=5)

        # Opponent has a cost-7 Digimon
        perm = runner.place_on_field(2, ["ST1-09"])
        perm.turn_played = -1

        self._activate_kongou(runner)

        # can_attack() should still return True (general attack is fine)
        assert perm.can_attack(), (
            "Cost-7 Digimon should still be able to attack (just not players)"
        )

    def test_cost_over_7_digimon_not_restricted(self, debug_runner):
        """Opponent's Digimon with play cost > 7 should NOT be restricted."""
        runner = debug_runner(initial_memory=5)

        # ST1-11 WarGreymon has cost 12
        perm = runner.place_on_field(2, ["ST1-11"])
        perm.turn_played = -1

        self._activate_kongou(runner)

        # Should still be able to attack players
        assert perm.can_attack_player(), (
            "Cost-12 Digimon should still be able to attack players"
        )

    def test_cost_3_digimon_restricted(self, debug_runner):
        """Opponent's low-cost Digimon (cost 3) can't attack players."""
        runner = debug_runner(initial_memory=5)

        # ST1-03 Agumon has cost 3
        perm = runner.place_on_field(2, ["ST1-03"])
        perm.turn_played = -1

        self._activate_kongou(runner)

        assert not perm.can_attack_player(), (
            "Cost-3 Digimon should NOT be able to attack players after Kongou"
        )

    # --- Test 2: Dynamic check — new Digimon played after Kongou ---

    def test_new_digimon_played_after_kongou_also_restricted(self, debug_runner):
        """Digimon played AFTER Kongou should also be restricted (dynamic condition)."""
        runner = debug_runner(initial_memory=5)

        # Activate Kongou with no opponent Digimon on field
        self._activate_kongou(runner)

        # Now opponent plays a cost-5 Digimon
        new_perm = runner.place_on_field(2, ["ST1-06"])
        new_perm.turn_played = -1  # no summoning sickness

        assert not new_perm.can_attack_player(), (
            "Digimon played AFTER Kongou should also be restricted "
            "(dynamic condition check, not just existing permanents)"
        )

    def test_new_high_cost_digimon_not_restricted(self, debug_runner):
        """High-cost Digimon played after Kongou should NOT be restricted."""
        runner = debug_runner(initial_memory=5)

        self._activate_kongou(runner)

        # Opponent plays a cost-12 Digimon
        new_perm = runner.place_on_field(2, ["ST1-11"])
        new_perm.turn_played = -1

        assert new_perm.can_attack_player(), (
            "High-cost Digimon played after Kongou should still attack players"
        )

    # --- Test 3: CANNOT_ADD_SECURITY modifier ---

    def test_cannot_add_security_registered(self, debug_runner):
        """Kongou should register a CANNOT_ADD_SECURITY modifier."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        self._activate_kongou(runner)

        # Check that CANNOT_ADD_SECURITY is in the modifier registry
        entries = game.modifiers._modifiers.get(ModifierType.CANNOT_ADD_SECURITY, [])
        assert len(entries) >= 1, (
            "Should have at least one CANNOT_ADD_SECURITY modifier registered"
        )

    def test_cannot_add_security_has_correct_expiry(self, debug_runner):
        """CANNOT_ADD_SECURITY should expire at end of opponent's turn."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        self._activate_kongou(runner)

        entries = game.modifiers._modifiers.get(ModifierType.CANNOT_ADD_SECURITY, [])
        assert len(entries) >= 1
        assert entries[0].expiry == 'end_of_opponent_turn', (
            f"CANNOT_ADD_SECURITY should expire at end_of_opponent_turn, got {entries[0].expiry}"
        )

    def test_cannot_add_security_granting_player(self, debug_runner):
        """CANNOT_ADD_SECURITY granting_player should be set for expiry tracking."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        self._activate_kongou(runner)

        entries = game.modifiers._modifiers.get(ModifierType.CANNOT_ADD_SECURITY, [])
        assert len(entries) >= 1
        assert entries[0].granting_player is game.player1, (
            "granting_player should be player1 (the Kongou user)"
        )

    # --- Test 4: CANNOT_ATTACK_PLAYER uses correct modifier type ---

    def test_uses_cannot_attack_player_not_cannot_attack(self, debug_runner):
        """Must use CANNOT_ATTACK_PLAYER modifier, not CANNOT_ATTACK."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(2, ["ST1-09"])  # cost 7
        perm.turn_played = -1

        self._activate_kongou(runner)

        # Should have CANNOT_ATTACK_PLAYER entries
        cap_entries = game.modifiers._modifiers.get(ModifierType.CANNOT_ATTACK_PLAYER, [])
        assert len(cap_entries) >= 1, (
            "Should use CANNOT_ATTACK_PLAYER modifier type"
        )

        # Should NOT have CANNOT_ATTACK entries (that blocks all attacks)
        ca_entries = game.modifiers._modifiers.get(ModifierType.CANNOT_ATTACK, [])
        # Filter to entries that would apply to opponent's permanents
        applicable = [e for e in ca_entries if e.is_active(perm, {})]
        assert len(applicable) == 0, (
            "Should NOT use CANNOT_ATTACK (which blocks all attacks including vs Digimon)"
        )

    # --- Test 5: Player's own Digimon not affected ---

    def test_own_digimon_not_restricted(self, debug_runner):
        """The player who uses Kongou should NOT have their own Digimon restricted."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Player 1's own Digimon (cost 5)
        own_perm = runner.place_on_field(1, ["ST1-06"])
        own_perm.turn_played = -1

        self._activate_kongou(runner, player_id=1)

        assert own_perm.can_attack_player(), (
            "Kongou user's own Digimon should NOT be restricted"
        )

    # --- Test 6: Security effect activates Main ---

    def test_security_effect_same_as_main(self, debug_runner):
        """[Security] should activate the same [Main] effects."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Set up opponent Digimon
        perm = runner.place_on_field(2, ["ST1-09"])
        perm.turn_played = -1

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT9-103", player)
        effects = cs.effect_list(None)
        sec_effects = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill
            and e.on_process_callback is not None
        ]
        assert len(sec_effects) >= 1, "Should have SecuritySkill effect"

        sec_effects[0].on_process_callback({
            'player': player,
            'game': game,
        })

        # Should restrict opponent's cost-7 Digimon
        assert not perm.can_attack_player(), (
            "Security effect should apply same CANNOT_ATTACK_PLAYER restriction"
        )

        # Should also register CANNOT_ADD_SECURITY
        entries = game.modifiers._modifiers.get(ModifierType.CANNOT_ADD_SECURITY, [])
        assert len(entries) >= 1, (
            "Security effect should also register CANNOT_ADD_SECURITY"
        )

    # --- Test 7: Expiry behavior ---

    def test_cannot_attack_player_expiry_is_end_of_opponent_turn(self, debug_runner):
        """CANNOT_ATTACK_PLAYER should expire at end of opponent's turn."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(2, ["ST1-09"])
        perm.turn_played = -1

        self._activate_kongou(runner)

        cap_entries = game.modifiers._modifiers.get(ModifierType.CANNOT_ATTACK_PLAYER, [])
        assert len(cap_entries) >= 1
        assert cap_entries[0].expiry == 'end_of_opponent_turn', (
            f"CANNOT_ATTACK_PLAYER should expire at end_of_opponent_turn, "
            f"got {cap_entries[0].expiry}"
        )

    def test_restriction_cleared_after_opponent_turn(self, debug_runner):
        """After clearing end_of_opponent_turn modifiers, restriction should be gone."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(2, ["ST1-09"])
        perm.turn_played = -1

        self._activate_kongou(runner)

        # Verify restricted
        assert not perm.can_attack_player()

        # Simulate end of opponent's turn cleanup
        # clear_opponent_turn_expiry is called with the granting player (player1)
        # when player1's turn starts again
        game.modifiers.clear_opponent_turn_expiry(game.player1)

        # Should be unrestricted now
        assert perm.can_attack_player(), (
            "After clearing end_of_opponent_turn modifiers, Digimon should attack players again"
        )

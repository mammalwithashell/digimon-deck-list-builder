"""Behavioral tests for BT22-029 Shoemon (Lv.3 Yellow Digimon, Puppet/LIBERATOR).

Card text:
  [On Play] [On Deletion] 1 of your Digimon with the [Puppet] trait gains
  <Blocker> until your opponent's turn ends.

  Inherited:
  [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP
  for the turn.

Key issues being fixed:
  1. Blocker grant must use register_modifier with end_of_opponent_turn expiry
     (was using permanent grant_keyword)
  2. Inherited timing must be OnAllyAttack (was OnUseAttack)
"""

import pytest
from engine_py_legacy.engine.interfaces.modifiers import ModifierType


def _get_grant_blocker_entries(game):
    """Get all GRANT_BLOCKER modifier entries from the registry."""
    return game.modifiers._modifiers.get(ModifierType.GRANT_BLOCKER, [])


@pytest.mark.behavioral
class TestBT22029Shoemon:
    """Tests for BT22-029 Shoemon."""

    # ── On Play: Grant Blocker to Puppet Digimon ────────────────────

    def test_on_play_grants_blocker_to_puppet_digimon(self, debug_runner):
        """On Play should register a GRANT_BLOCKER modifier with end_of_opponent_turn expiry."""
        deck = ["BT22-029"] * 4 + ["BT1-009"] * 46
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=5, skip_shuffle=True)

        # Place a Puppet Digimon on the field for Shoemon to target
        puppet_target = runner.place_on_field(1, ["BT22-029"])

        # Play Shoemon from hand
        runner.inject_card(1, "BT22-029", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Shoemon")
        assert play is not None, f"Should be able to play Shoemon. Actions: {runner.actions()}"
        runner.execute(play)
        runner.auto_resolve()

        # Verify GRANT_BLOCKER modifier was registered
        entries = _get_grant_blocker_entries(runner.game)
        assert len(entries) >= 1, (
            "Should have at least 1 GRANT_BLOCKER entry after On Play")
        # The entry targeting our puppet should have end_of_opponent_turn expiry
        matching = [e for e in entries if e.source_permanent is puppet_target]
        assert len(matching) == 1, (
            f"Expected 1 GRANT_BLOCKER entry sourced from puppet target, "
            f"got {len(matching)}")
        assert matching[0].expiry == 'end_of_opponent_turn', (
            f"Blocker grant should expire at end_of_opponent_turn, "
            f"got '{matching[0].expiry}'")

    def test_on_play_only_targets_puppet_digimon(self, debug_runner):
        """On Play selection should only allow Puppet-trait Digimon."""
        deck = ["BT22-029"] * 4 + ["BT1-009"] * 46
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=5, skip_shuffle=True)

        # Place a non-Puppet Digimon -- should NOT be selectable
        non_puppet = runner.place_on_field(1, ["BT1-009"])
        # Place a Puppet Digimon -- should be the only valid target
        puppet_target = runner.place_on_field(1, ["BT22-029"])

        runner.inject_card(1, "BT22-029", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Shoemon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        # Only the Puppet target should have a GRANT_BLOCKER entry
        entries = _get_grant_blocker_entries(runner.game)
        puppet_entries = [e for e in entries if e.source_permanent is puppet_target]
        non_puppet_entries = [e for e in entries if e.source_permanent is non_puppet]
        assert len(puppet_entries) == 1, "Puppet Digimon should have GRANT_BLOCKER"
        assert len(non_puppet_entries) == 0, "Non-Puppet Digimon should NOT have GRANT_BLOCKER"

    # ── On Deletion: Grant Blocker to Puppet Digimon ────────────────

    def test_on_deletion_grants_blocker(self, debug_runner):
        """On Deletion should register a GRANT_BLOCKER modifier on a Puppet Digimon."""
        deck = ["BT22-029"] * 4 + ["BT1-009"] * 46
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=5, skip_shuffle=True)

        # Place Shoemon on field (it will be deleted)
        shoemon_perm = runner.place_on_field(1, ["BT22-029"])
        # Place a Puppet target that will receive Blocker
        puppet_target = runner.place_on_field(1, ["BT22-029"])

        # Delete Shoemon to trigger On Deletion
        runner.game.player1.delete_permanent(shoemon_perm)
        runner.auto_resolve()

        # Verify GRANT_BLOCKER modifier was registered on the target
        entries = _get_grant_blocker_entries(runner.game)
        matching = [e for e in entries if e.source_permanent is puppet_target]
        assert len(matching) == 1, (
            f"Puppet Digimon should have GRANT_BLOCKER after Shoemon deletion, "
            f"got {len(matching)} entries")
        assert matching[0].expiry == 'end_of_opponent_turn', (
            f"Blocker grant should expire at end_of_opponent_turn")

    # ── Inherited: When Attacking OPT -2000 DP ──────────────────────

    def test_inherited_when_attacking_reduces_dp(self, debug_runner):
        """Inherited [When Attacking] should give -2000 DP to an opponent's Digimon."""
        deck = ["BT22-029"] * 4 + ["BT1-041"] * 46
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=5, skip_shuffle=True)

        # Place a Lv.4 Digimon with Shoemon as inherited source
        attacker = runner.place_on_field(1, ["BT22-029", "BT1-041"])

        # Place an opponent Digimon to target
        opp_digimon = runner.place_on_field(2, ["BT1-041"])  # 5000 DP
        opp_dp_before = opp_digimon.dp

        runner.set_phase("Main")

        attack = runner.find_action("Attack player with")
        assert attack is not None, f"Should find attack action. Actions: {runner.actions()}"
        runner.execute(attack)
        runner.auto_resolve()

        # Opponent Digimon should have -2000 DP
        assert opp_digimon.dp == opp_dp_before - 2000, (
            f"Opponent Digimon DP should be {opp_dp_before - 2000}, got {opp_digimon.dp}")

    def test_inherited_timing_is_on_ally_attack(self, debug_runner):
        """Inherited effect should use OnAllyAttack timing (32), not OnUseAttack."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.enums import EffectTiming

        db = CardDatabase()
        script = db.get_script("BT22-029")
        assert script is not None, "BT22-029 script should be registered"

        from engine_py_legacy.tests.helpers.game_builder import make_card
        card = make_card(card_id="BT22-029", level=3)
        effects = script.get_card_effects(card)

        inherited = [e for e in effects if e.is_inherited_effect]
        assert len(inherited) == 1, f"Expected 1 inherited effect, got {len(inherited)}"
        assert inherited[0].timing == EffectTiming.OnAllyAttack, (
            f"Inherited timing should be OnAllyAttack (32), got {inherited[0].timing}")

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited [When Attacking] should be limited to once per turn."""
        from engine_py_legacy.engine.data.card_database import CardDatabase

        db = CardDatabase()
        script = db.get_script("BT22-029")
        assert script is not None

        from engine_py_legacy.tests.helpers.game_builder import make_card
        card = make_card(card_id="BT22-029", level=3)
        effects = script.get_card_effects(card)

        inherited = [e for e in effects if e.is_inherited_effect]
        assert len(inherited) == 1
        assert inherited[0].max_count_per_turn == 1, (
            f"Inherited effect should be Once Per Turn (max_count=1), "
            f"got {inherited[0].max_count_per_turn}")

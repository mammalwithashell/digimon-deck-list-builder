"""Behavioral tests for EX9-027 Kokeshimon (Lv.4, Yellow/Purple, Puppet/LIBERATOR).

Card text:
  [When Digivolving] [On Deletion] By trashing 1 card in your hand,
      1 of your opponent's Digimon gets -4000 DP for the turn.

  Inherited:
  [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks,
      by deleting 1 of your other Digimon, end that attack.

  Alt digivolve: From Lv.3 [Puppet] for cost 2.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX9027WhenDigivolving:
    """Tests for EX9-027 [When Digivolving]: trash 1 hand card, -4000 DP to opponent Digimon."""

    def test_when_digivolving_trash_and_minus_dp(self, debug_runner):
        """When digivolving, trashing 1 hand card should give opponent's Digimon -4000 DP."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX9-027"])
        game = runner.game

        # Opponent has a Digimon to target
        opp_perm = runner.place_on_field(2, ["ST1-10"], turn_played=-1)  # Phoenixmon 12000 DP
        dp_before = opp_perm.dp

        # P1 needs a hand card to trash
        runner.inject_card(1, "ST1-02", "hand")

        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [e for e in effects if e.is_when_digivolving]
        assert len(wd_effects) == 1, "Should have 1 When Digivolving effect"

        wd = wd_effects[0]
        assert wd.is_optional, "When Digivolving effect should be optional"

        # Execute the process
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # Check the DP decrease
        assert opp_perm.dp == dp_before - 4000, \
            f"Opponent Digimon should have -4000 DP, got {opp_perm.dp} (was {dp_before})"

    def test_when_digivolving_condition_needs_hand_card(self, debug_runner):
        """Condition should fail if player has no hand cards."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-027"])
        runner.clear_zone(1, "hand")

        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [e for e in effects if e.is_when_digivolving]
        assert len(wd_effects) == 1

        result = wd_effects[0].can_use_condition({})
        assert not result, "Condition should fail with no hand cards"


@pytest.mark.behavioral
class TestEX9027OnDeletion:
    """Tests for EX9-027 [On Deletion]: trash 1 hand card, -4000 DP to opponent Digimon."""

    def test_on_deletion_effect_attributes(self, debug_runner):
        """On Deletion effect should have correct timing and optionality."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-027"])
        card = perm.top_card
        effects = card.effect_list(None)
        od_effects = [e for e in effects if e.is_on_deletion]
        assert len(od_effects) == 1, "Should have 1 On Deletion effect"

        od = od_effects[0]
        assert od.is_optional, "On Deletion effect should be optional"
        assert od.timing == EffectTiming.OnDestroyedAnyone, \
            f"On Deletion should use OnDestroyedAnyone timing, got {od.timing}"

    def test_on_deletion_trash_and_minus_dp(self, debug_runner):
        """On Deletion, trashing a hand card should give opponent's Digimon -4000 DP."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX9-027"])
        game = runner.game

        # Opponent has a Digimon to target
        opp_perm = runner.place_on_field(2, ["ST1-10"], turn_played=-1)
        dp_before = opp_perm.dp

        # P1 needs a hand card
        runner.inject_card(1, "ST1-02", "hand")

        card = perm.top_card
        effects = card.effect_list(None)
        od_effects = [e for e in effects if e.is_on_deletion]
        od = od_effects[0]

        # Execute the On Deletion process
        od.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        assert opp_perm.dp == dp_before - 4000, \
            f"Opponent Digimon should have -4000 DP after on-deletion effect"

    def test_on_deletion_condition_needs_hand_card(self, debug_runner):
        """On Deletion condition should fail without hand cards."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-027"])
        runner.clear_zone(1, "hand")

        card = perm.top_card
        effects = card.effect_list(None)
        od_effects = [e for e in effects if e.is_on_deletion]
        assert len(od_effects) == 1

        result = od_effects[0].can_use_condition({})
        assert not result, "Condition should fail with no hand cards"


@pytest.mark.behavioral
class TestEX9027Inherited:
    """Tests for EX9-027 inherited: [Opponent's Turn][Once Per Turn] end attack."""

    def test_inherited_uses_on_ally_attack_timing(self, debug_runner):
        """Inherited effect must use OnAllyAttack timing (not OnUseAttack)."""
        runner = debug_runner(initial_memory=10)

        # Place Kokeshimon as inherited source under a Lv.5 Digimon
        perm = runner.place_on_field(1, ["EX9-027", "ST1-10"])

        kokeshimon_card = perm.card_sources[0]  # Bottom of stack = Kokeshimon
        effects = kokeshimon_card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        assert len(inherited_effects) == 1, "Should have 1 inherited effect"

        inh = inherited_effects[0]
        assert inh.timing == EffectTiming.OnAllyAttack, \
            f"Inherited should use OnAllyAttack timing, got {inh.timing}"

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited effect should be Once Per Turn with correct hash."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-027", "ST1-10"])
        kokeshimon_card = perm.card_sources[0]
        effects = kokeshimon_card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh = inherited_effects[0]

        assert inh.is_optional, "Inherited effect must be optional"
        assert inh.hash_string == "StopAttack_EX9-027", \
            "Hash string should be StopAttack_EX9-027"

    def test_inherited_condition_opponent_turn_only(self, debug_runner):
        """Inherited effect should only fire on opponent's turn."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-027", "ST1-10"])
        runner.place_on_field(1, ["ST1-02"], turn_played=-1)  # Sacrifice target

        game = runner.game
        kokeshimon_card = perm.card_sources[0]
        effects = kokeshimon_card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh = inherited_effects[0]

        # P1's turn: condition should fail
        assert game.player1.is_my_turn
        assert not inh.can_use_condition({}), \
            "Inherited effect should NOT activate on own turn"

        # Switch to P2's turn
        game.switch_turn()
        assert not game.player1.is_my_turn
        assert inh.can_use_condition({}), \
            "Inherited effect should activate on opponent's turn"

    def test_inherited_condition_needs_other_digimon(self, debug_runner):
        """Condition should fail if there are no other own Digimon to delete."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-027", "ST1-10"])
        # No other Digimon on P1's field

        game = runner.game
        game.switch_turn()

        kokeshimon_card = perm.card_sources[0]
        effects = kokeshimon_card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh = inherited_effects[0]

        assert not inh.can_use_condition({}), \
            "Condition should fail without other Digimon to delete"

    def test_inherited_end_attack_deletes_other_digimon(self, debug_runner):
        """Process should: select other own Digimon -> delete it -> end attack."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX9-027", "ST1-10"], turn_played=-1)
        sacrifice = runner.place_on_field(1, ["ST1-02"], turn_played=-1)

        # P2 has an attacker
        attacker = runner.place_on_field(2, ["ST1-10"], turn_played=-1)

        game = runner.game
        game.switch_turn()
        runner.set_phase("Main")

        kokeshimon_card = perm.card_sources[0]
        effects = kokeshimon_card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh = inherited_effects[0]

        # Set up pending attack
        from engine_py_legacy.engine.game.constants import PendingAttack
        game.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=game.player1,
            effective_target=game.player1,
        )

        p1_field_before = len(game.player1.battle_area)

        inh.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        assert len(game.player1.battle_area) == p1_field_before - 1, \
            "One of P1's other Digimon should have been deleted"
        assert game.pending_attack.is_end_attack, \
            "Attack should have been force-ended"

    def test_inherited_fires_when_opponent_attacks(self, debug_runner):
        """Full integration: opponent declares attack, inherited triggers and ends it."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # P1: Digimon with Kokeshimon inherited + sacrifice target
        perm = runner.place_on_field(1, ["EX9-027", "ST1-10"], turn_played=-1)
        sacrifice = runner.place_on_field(1, ["ST1-02"], turn_played=-1)

        # P2: attacker
        attacker = runner.place_on_field(2, ["ST1-10"], turn_played=-1)

        game = runner.game
        game.switch_turn()
        runner.set_memory(10)  # Give P2 memory to attack
        runner.set_phase("Main")

        action = runner.find_action("Attack player with Phoenixmon")
        assert action is not None, "P2 should be able to attack with Phoenixmon"

        before_snap = runner.snapshot()
        p1_field_before = len(before_snap.p1_field)

        runner.execute(action)
        runner.auto_resolve()

        after_snap = runner.snapshot()
        assert len(after_snap.p1_field) < p1_field_before, \
            "P1 should have fewer Digimon after sacrificing one to end attack"


@pytest.mark.behavioral
class TestEX9027AltDigivolve:
    """Tests for EX9-027 alt digivolution: from Lv.3 [Puppet] for cost 2."""

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi effect should specify Lv.3, Puppet trait, cost 2."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-027"])
        card = perm.top_card
        effects = card.effect_list(None)
        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost') and e._alt_digi_cost is not None]
        assert len(alt_digi) == 1, "Should have 1 alt-digi effect"

        ad = alt_digi[0]
        assert ad._alt_digi_cost == 2, "Alt-digi cost should be 2"
        assert ad._alt_digi_level == 3, "Alt-digi level should be 3"
        assert ad._alt_digi_trait == "Puppet", "Alt-digi trait should be Puppet"

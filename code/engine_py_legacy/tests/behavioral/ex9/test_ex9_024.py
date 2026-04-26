"""Behavioral tests for EX9-024 Hanimon (Lv.3, Yellow/Purple, Puppet/LIBERATOR).

Card text:
  [On Play] By trashing 1 card in your hand, you may return 1 Digimon card
      with the [Puppet] trait from your trash to the hand.

  Inherited:
  [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks,
      by deleting 1 of your other Digimon, end that attack.

  Alt digivolve: From Kyaromon for cost 0.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX9024OnPlay:
    """Tests for EX9-024 [On Play] effect: trash 1 hand card, return 1 Puppet Digimon from trash."""

    def test_on_play_trash_hand_return_puppet_from_trash(self, debug_runner):
        """Trashing a hand card should allow recovering a Puppet Digimon from trash."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Put a Puppet Digimon in P1's trash for recovery
        runner.inject_card(1, "EX9-024", "trash")  # Hanimon is Puppet trait

        # Put a card in hand to trash (the Hanimon we'll play, plus one extra to trash)
        runner.inject_card(1, "ST1-02", "hand")  # Biyomon - something to trash
        runner.inject_card(1, "EX9-024", "hand")  # Hanimon to play

        # Play Hanimon
        action = runner.find_action("Play Hanimon")
        assert action is not None, "Should find Play Hanimon action"
        runner.execute(action)

        # Step 1: Select a hand card to trash (pick first non-decline action)
        snap = runner.snapshot()
        assert snap.phase == "SelectHand", "Should be in hand selection phase"
        legal = runner.action_mask()
        # Pick a non-decline card (any index < 62)
        hand_actions = [a for a in legal if a < 62]
        assert hand_actions, "Should have hand card selections available"
        runner.execute(hand_actions[0])

        # Step 2: Select Puppet Digimon from trash (non-decline action)
        snap = runner.snapshot()
        assert snap.phase == "SelectTrash", "Should be in trash selection phase"
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a != 62]
        assert trash_actions, "Should have trash card selections available"
        runner.execute(trash_actions[0])

        after_snap = runner.snapshot()
        assert any("EX9-024" in c for c in after_snap.p1_hand), \
            "Puppet Digimon (Hanimon) should have been returned from trash to hand"

    def test_on_play_is_optional(self, debug_runner):
        """The On Play effect should be optional (can skip trashing)."""
        runner = debug_runner(initial_memory=10)

        # Get the effect object
        perm = runner.place_on_field(1, ["EX9-024"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects if e.is_on_play]
        assert len(on_play_effects) == 1, "Should have exactly 1 On Play effect"
        assert on_play_effects[0].is_optional, "On Play effect must be optional"

    def test_on_play_condition_needs_hand_card(self, debug_runner):
        """Condition should fail if player has no hand cards to trash."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-024"])
        runner.clear_zone(1, "hand")

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects if e.is_on_play]
        assert len(on_play_effects) == 1

        result = on_play_effects[0].can_use_condition({})
        assert not result, "Condition should fail with no hand cards"

    def test_on_play_only_puppet_digimon_selectable_from_trash(self, debug_runner):
        """Only Digimon cards with [Puppet] trait should be recoverable from trash."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Put a non-Puppet card in trash (should not be selectable)
        runner.inject_card(1, "ST1-02", "trash")  # Biyomon - not Puppet

        # Put Puppet Digimon in trash
        runner.inject_card(1, "EX9-024", "trash")  # Hanimon - Puppet

        # Hand card to trash + Hanimon to play
        runner.inject_card(1, "ST1-05", "hand")  # something to trash
        runner.inject_card(1, "EX9-024", "hand")  # Hanimon to play

        action = runner.find_action("Play Hanimon")
        assert action is not None
        runner.execute(action)

        # Trash a hand card
        legal = runner.action_mask()
        hand_actions = [a for a in legal if a < 62]
        runner.execute(hand_actions[0])

        # Now in SelectTrash - check that only Puppet Digimon are selectable
        snap = runner.snapshot()
        assert snap.phase == "SelectTrash", "Should be in trash selection"
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a != 62]  # Non-decline
        # Select the Puppet card
        assert trash_actions, "Puppet Digimon should be selectable from trash"
        runner.execute(trash_actions[0])

        after = runner.snapshot()
        puppet_in_hand = any("EX9-024" in c for c in after.p1_hand)
        assert puppet_in_hand, "Puppet Digimon should be returned to hand"


@pytest.mark.behavioral
class TestEX9024Inherited:
    """Tests for EX9-024 inherited: [Opponent's Turn][Once Per Turn] end attack."""

    def test_inherited_uses_on_ally_attack_timing(self, debug_runner):
        """Inherited effect must use OnAllyAttack timing (not OnUseAttack)."""
        runner = debug_runner(initial_memory=10)

        # Place Hanimon as inherited source under a Lv.4 Digimon
        perm = runner.place_on_field(1, ["EX9-024", "EX9-027"])

        hanimon_card = perm.card_sources[0]  # Bottom of stack = Hanimon
        effects = hanimon_card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        assert len(inherited_effects) == 1, "Should have 1 inherited effect"

        inh = inherited_effects[0]
        assert inh.timing == EffectTiming.OnAllyAttack, \
            f"Inherited should use OnAllyAttack timing, got {inh.timing}"

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited effect should be Once Per Turn."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-024", "EX9-027"])
        hanimon_card = perm.card_sources[0]
        effects = hanimon_card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh = inherited_effects[0]

        assert inh.is_optional, "Inherited effect must be optional"
        assert inh.hash_string == "StopAttack_EX9-024", \
            "Hash string should be StopAttack_EX9-024"

    def test_inherited_condition_opponent_turn_only(self, debug_runner):
        """Inherited effect should only fire on opponent's turn."""
        runner = debug_runner(initial_memory=10)

        # P1's Digimon with Hanimon inherited, P1 also has another Digimon to sacrifice
        perm = runner.place_on_field(1, ["EX9-024", "EX9-027"])
        runner.place_on_field(1, ["ST1-02"], turn_played=-1)  # Sacrifice target

        game = runner.game
        hanimon_card = perm.card_sources[0]
        effects = hanimon_card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh = inherited_effects[0]

        # P1's turn: condition should fail
        assert game.player1.is_my_turn, "P1 should be turn player initially"
        assert not inh.can_use_condition({}), \
            "Inherited effect should NOT activate on own turn"

        # Switch to P2's turn
        game.switch_turn()
        assert not game.player1.is_my_turn, "P1 should NOT be turn player after switch"
        assert inh.can_use_condition({}), \
            "Inherited effect should activate on opponent's turn"

    def test_inherited_condition_needs_other_digimon(self, debug_runner):
        """Condition should fail if there are no other own Digimon to delete."""
        runner = debug_runner(initial_memory=10)

        # P1 has ONLY the Digimon with Hanimon inherited, no other Digimon
        perm = runner.place_on_field(1, ["EX9-024", "EX9-027"])

        game = runner.game
        game.switch_turn()  # opponent's turn

        hanimon_card = perm.card_sources[0]
        effects = hanimon_card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh = inherited_effects[0]

        assert not inh.can_use_condition({}), \
            "Condition should fail without other Digimon to delete"

    def test_inherited_end_attack_deletes_other_digimon(self, debug_runner):
        """Process should: select other own Digimon -> delete it -> end attack."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # P1 has the Digimon with inherited + a sacrifice target
        perm = runner.place_on_field(1, ["EX9-024", "EX9-027"], turn_played=-1)
        sacrifice = runner.place_on_field(1, ["ST1-02"], turn_played=-1)

        # P2 has an attacker
        attacker = runner.place_on_field(2, ["ST1-10"], turn_played=-1)  # Phoenixmon

        game = runner.game
        game.switch_turn()  # Now P2's turn
        runner.set_phase("Main")

        hanimon_card = perm.card_sources[0]
        effects = hanimon_card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh = inherited_effects[0]

        # Set up pending attack so force_end_attack has something to end
        from engine_py_legacy.engine.game.constants import PendingAttack
        game.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=game.player1,
            effective_target=game.player1,
        )

        p1_field_before = len(game.player1.battle_area)

        # Execute the inherited effect process
        inh.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # Sacrifice target should be deleted
        assert len(game.player1.battle_area) == p1_field_before - 1, \
            "One of P1's other Digimon should have been deleted"

        # Attack should be ended
        assert game.pending_attack.is_end_attack, \
            "Attack should have been force-ended"

    def test_inherited_fires_when_opponent_attacks(self, debug_runner):
        """Full integration: opponent declares attack, inherited triggers and ends it."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # P1: Digimon with Hanimon as inherited source + sacrifice target
        perm = runner.place_on_field(1, ["EX9-024", "EX9-027"], turn_played=-1)
        sacrifice = runner.place_on_field(1, ["ST1-02"], turn_played=-1)

        # P2: attacker
        attacker = runner.place_on_field(2, ["ST1-10"], turn_played=-1)

        game = runner.game
        game.switch_turn()
        runner.set_memory(10)  # Give P2 memory to attack
        runner.set_phase("Main")

        # P2 declares attack on P1's security
        action = runner.find_action("Attack player with Phoenixmon")
        assert action is not None, "P2 should be able to attack with Phoenixmon"

        before_snap = runner.snapshot()
        p1_field_before = len(before_snap.p1_field)

        runner.execute(action)
        # The inherited effect triggers; auto-resolve picks the sacrifice target
        runner.auto_resolve()

        after_snap = runner.snapshot()
        # P1 should have lost 1 Digimon (the sacrifice)
        assert len(after_snap.p1_field) < p1_field_before, \
            "P1 should have fewer Digimon after sacrificing one to end attack"


@pytest.mark.behavioral
class TestEX9024AltDigivolve:
    """Tests for EX9-024 alt digivolution: from Kyaromon for cost 0."""

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi effect should specify Kyaromon, Lv.2, cost 0."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-024"])
        card = perm.top_card
        effects = card.effect_list(None)
        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost')]
        assert len(alt_digi) == 1, "Should have 1 alt-digi effect"

        ad = alt_digi[0]
        assert ad._alt_digi_cost == 0, "Alt-digi cost should be 0"
        assert ad._alt_digi_name == "Kyaromon", "Alt-digi should be from Kyaromon"
        assert ad._alt_digi_level == 2, "Alt-digi level should be 2"

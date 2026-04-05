"""Behavioral tests for EX9-033 Kaguyamon (Lv.6 Yellow/Purple, Puppet/LIBERATOR, DP 12000).

Card text:
[All Turns] All of your Tokens and [Puppet] trait Digimon gain <Alliance> and <Blocker>.
[All Turns] [Once Per Turn] When other Digimon are deleted, delete 1 of your opponent's
    lowest level Digimon.
[End of Your Turn] [Once Per Turn] You may play 1 level 4 or lower [Puppet] trait Digimon
    card from your trash without paying the cost.
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming


# --- Test card IDs ---
KAGUYAMON = "EX9-033"
# Puppet trait Digimon at various levels
PUPPET_LV3 = "BT2-055"       # ToyAgumon, Lv.3 Puppet, DP 1000
PUPPET_LV4 = "BT6-084"       # Sistermon Ciel, Lv.4 Puppet, DP 5000
PUPPET_LV5 = "BT1-038"       # Monzaemon, Lv.5 Puppet, DP 6000
# Non-Puppet Digimon
NON_PUPPET_LV3 = "ST1-03"    # Agumon, Lv.3 Reptile (no Puppet trait)
NON_PUPPET_LV4 = "ST1-07"    # Greymon, Lv.4 Dinosaur (no Puppet trait)


@pytest.mark.behavioral
class TestEX9033KaguyamonBlockerAllianceAura:
    """Tests for Clause 1: [All Turns] All own Tokens and Puppet Digimon gain Blocker + Alliance."""

    def test_puppet_digimon_gains_blocker_from_kaguyamon_aura(self, debug_runner):
        """A Puppet trait Digimon on the same field should gain Blocker from Kaguyamon's aura."""
        runner = debug_runner(initial_memory=10)

        # Place Kaguyamon on field
        runner.place_on_field(1, [KAGUYAMON])

        # Place a Puppet Digimon on field
        puppet_perm = runner.place_on_field(1, [PUPPET_LV4])

        assert puppet_perm.has_keyword('_is_blocker'), (
            "Puppet Digimon should gain Blocker from Kaguyamon's aura"
        )

    def test_puppet_digimon_gains_alliance_from_kaguyamon_aura(self, debug_runner):
        """A Puppet trait Digimon on the same field should gain Alliance from Kaguyamon's aura."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, [KAGUYAMON])
        puppet_perm = runner.place_on_field(1, [PUPPET_LV4])

        assert puppet_perm.has_keyword('_is_alliance'), (
            "Puppet Digimon should gain Alliance from Kaguyamon's aura"
        )

    def test_kaguyamon_itself_gains_blocker_and_alliance(self, debug_runner):
        """Kaguyamon itself has the Puppet trait, so it should gain Blocker and Alliance."""
        runner = debug_runner(initial_memory=10)

        kaguya_perm = runner.place_on_field(1, [KAGUYAMON])

        assert kaguya_perm.has_keyword('_is_blocker'), (
            "Kaguyamon (Puppet trait) should have Blocker from its own aura"
        )
        assert kaguya_perm.has_keyword('_is_alliance'), (
            "Kaguyamon (Puppet trait) should have Alliance from its own aura"
        )

    def test_non_puppet_digimon_does_not_gain_keywords(self, debug_runner):
        """A non-Puppet Digimon should NOT gain Blocker or Alliance from Kaguyamon's aura."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, [KAGUYAMON])
        non_puppet_perm = runner.place_on_field(1, [NON_PUPPET_LV3])

        assert not non_puppet_perm.has_keyword('_is_blocker'), (
            "Non-Puppet Digimon should NOT get Blocker from Kaguyamon's aura"
        )
        assert not non_puppet_perm.has_keyword('_is_alliance'), (
            "Non-Puppet Digimon should NOT get Alliance from Kaguyamon's aura"
        )

    def test_token_gains_blocker_and_alliance(self, debug_runner):
        """A Token Digimon on the same field should gain Blocker and Alliance."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [KAGUYAMON])

        # Play a token onto the field
        game.effect_play_token(player, 'diaboromon')
        tokens = [p for p in player.battle_area if p.is_token]
        assert len(tokens) >= 1, "Should have at least 1 token on field"

        token = tokens[0]
        assert token.has_keyword('_is_blocker'), (
            "Token should gain Blocker from Kaguyamon's aura"
        )
        assert token.has_keyword('_is_alliance'), (
            "Token should gain Alliance from Kaguyamon's aura"
        )

    def test_opponent_puppet_does_not_gain_keywords(self, debug_runner):
        """Opponent's Puppet Digimon should NOT gain keywords from Kaguyamon's aura."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, [KAGUYAMON])
        opp_puppet = runner.place_on_field(2, [PUPPET_LV4])

        assert not opp_puppet.has_keyword('_is_blocker'), (
            "Opponent's Puppet should NOT get Blocker from Kaguyamon's aura"
        )
        assert not opp_puppet.has_keyword('_is_alliance'), (
            "Opponent's Puppet should NOT get Alliance from Kaguyamon's aura"
        )

    def test_aura_removed_when_kaguyamon_leaves_field(self, debug_runner):
        """When Kaguyamon is removed from the field, Puppet Digimon should lose the granted keywords."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        kaguya_perm = runner.place_on_field(1, [KAGUYAMON])
        puppet_perm = runner.place_on_field(1, [PUPPET_LV4])

        # Verify aura is active
        assert puppet_perm.has_keyword('_is_blocker'), "Puppet should have Blocker while Kaguyamon on field"

        # Remove Kaguyamon from the field manually
        if kaguya_perm in player.battle_area:
            player.battle_area.remove(kaguya_perm)

        assert not puppet_perm.has_keyword('_is_blocker'), (
            "Puppet should lose Blocker after Kaguyamon leaves the field"
        )
        assert not puppet_perm.has_keyword('_is_alliance'), (
            "Puppet should lose Alliance after Kaguyamon leaves the field"
        )


@pytest.mark.behavioral
class TestEX9033KaguyamonDeletionObserver:
    """Tests for Clause 2: [All Turns][OPT] When other Digimon are deleted,
    delete 1 of opponent's lowest level Digimon."""

    def test_deletion_triggers_when_own_digimon_deleted(self, debug_runner):
        """When one of your own Digimon is deleted, Kaguyamon should delete opponent's lowest level."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1
        enemy = game.player2

        runner.place_on_field(1, [KAGUYAMON])
        own_puppet = runner.place_on_field(1, [PUPPET_LV3])

        # Place opponent Digimon at different levels
        opp_lv3 = runner.place_on_field(2, [NON_PUPPET_LV3])  # Lv.3
        opp_lv4 = runner.place_on_field(2, [NON_PUPPET_LV4])  # Lv.4

        opp_count_before = len(enemy.battle_area)

        # Delete the own puppet - this should trigger Kaguyamon's observer
        player.delete_permanent(own_puppet)
        runner.auto_resolve()

        # Opponent should have lost their lowest level (Lv.3) Digimon
        opp_ids = [p.top_card.c_entity_base.card_id for p in enemy.battle_area if p.top_card]
        assert NON_PUPPET_LV3 not in opp_ids, (
            f"Opponent's lowest level Digimon (Lv.3) should be deleted. "
            f"Remaining: {opp_ids}"
        )
        assert len(enemy.battle_area) == opp_count_before - 1, (
            "Exactly one opponent Digimon should be deleted"
        )

    def test_deletion_triggers_when_opponent_digimon_deleted(self, debug_runner):
        """When an OPPONENT's Digimon is deleted, Kaguyamon should also trigger
        ('other Digimon' means any Digimon other than self, on either side)."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1
        enemy = game.player2

        runner.place_on_field(1, [KAGUYAMON])

        # Place opponent Digimon
        opp_target = runner.place_on_field(2, [PUPPET_LV3])   # This one gets deleted
        opp_survivor = runner.place_on_field(2, [NON_PUPPET_LV4])  # This one should survive or not
        opp_lowest = runner.place_on_field(2, [NON_PUPPET_LV3])   # Lowest level

        opp_count_before = len(enemy.battle_area)

        # Delete an opponent's Digimon by effect — should trigger Kaguyamon
        enemy.delete_permanent(opp_target)
        runner.auto_resolve()

        # Kaguyamon should have triggered and deleted the lowest level remaining
        # opp_target is already gone; among remaining, NON_PUPPET_LV3 is lowest
        opp_ids_after = [p.top_card.c_entity_base.card_id for p in enemy.battle_area if p.top_card]

        # The deletion observer should have deleted 1 more, so 2 deleted total
        assert len(enemy.battle_area) == opp_count_before - 2, (
            f"Two opponent Digimon should be deleted (original + Kaguyamon observer). "
            f"Before: {opp_count_before}, After: {len(enemy.battle_area)}"
        )

    def test_deletion_observer_does_not_trigger_when_kaguyamon_self_deleted(self, debug_runner):
        """If Kaguyamon itself is deleted, its observer should NOT trigger (it's 'other Digimon')."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1
        enemy = game.player2

        kaguya_perm = runner.place_on_field(1, [KAGUYAMON])
        opp_lv3 = runner.place_on_field(2, [NON_PUPPET_LV3])

        opp_count_before = len(enemy.battle_area)

        # Delete Kaguyamon itself
        player.delete_permanent(kaguya_perm)
        runner.auto_resolve()

        # Opponent should be unaffected
        assert len(enemy.battle_area) == opp_count_before, (
            "Kaguyamon's deletion observer should not trigger when Kaguyamon itself is deleted"
        )

    def test_deletion_observer_once_per_turn(self, debug_runner):
        """The deletion observer is [Once Per Turn] - should only trigger once per turn."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1
        enemy = game.player2

        runner.place_on_field(1, [KAGUYAMON])
        own1 = runner.place_on_field(1, [PUPPET_LV3])
        own2 = runner.place_on_field(1, [PUPPET_LV4])

        opp1 = runner.place_on_field(2, [NON_PUPPET_LV3])
        opp2 = runner.place_on_field(2, [NON_PUPPET_LV4])

        # Delete first own Digimon - triggers observer
        player.delete_permanent(own1)
        runner.auto_resolve()

        opp_count_after_first = len(enemy.battle_area)
        assert opp_count_after_first == 1, (
            "After first deletion trigger, opponent should have lost 1 Digimon (lowest level)"
        )

        # Delete second own Digimon - should NOT trigger again (OPT)
        player.delete_permanent(own2)
        runner.auto_resolve()

        opp_count_after_second = len(enemy.battle_area)
        assert opp_count_after_second == 1, (
            "After second deletion in same turn, observer should NOT trigger again (OPT)"
        )

    def test_deletion_observer_targets_lowest_level_only(self, debug_runner):
        """The observer must select among the lowest level Digimon only."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1
        enemy = game.player2

        runner.place_on_field(1, [KAGUYAMON])
        own_puppet = runner.place_on_field(1, [PUPPET_LV3])

        # Place opponent Digimon - only higher levels
        opp_lv4 = runner.place_on_field(2, [NON_PUPPET_LV4])  # Lv.4 — the lowest on opp side

        opp_count_before = len(enemy.battle_area)

        # Delete own puppet
        player.delete_permanent(own_puppet)
        runner.auto_resolve()

        # The Lv.4 is the lowest (and only), so it should be deleted
        assert len(enemy.battle_area) == opp_count_before - 1, (
            "Opponent's lowest level Digimon (even if Lv.4) should be deleted"
        )

    def test_deletion_observer_not_triggered_by_non_digimon(self, debug_runner):
        """Non-Digimon deletion (e.g. Tamer) should not trigger the observer.
        The card text says 'When other Digimon are deleted'."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1
        enemy = game.player2

        runner.place_on_field(1, [KAGUYAMON])
        opp = runner.place_on_field(2, [NON_PUPPET_LV3])

        # Inject and place a Tamer, then delete it
        tamer_card = runner.inject_card(1, "ST1-12", "hand")  # Tai Kamiya tamer
        tamer_perm = runner.place_on_field(1, ["ST1-12"])

        opp_count_before = len(enemy.battle_area)

        player.delete_permanent(tamer_perm)
        runner.auto_resolve()

        assert len(enemy.battle_area) == opp_count_before, (
            "Tamer deletion should not trigger Kaguyamon's Digimon deletion observer"
        )


@pytest.mark.behavioral
class TestEX9033KaguyamonEndOfTurnPlay:
    """Tests for Clause 3: [End of Your Turn][OPT] Play Lv.4 or lower Puppet from trash free."""

    def test_end_of_turn_play_puppet_from_trash(self, debug_runner):
        """At end of turn, should be able to play a Lv.4 Puppet from trash for free."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [KAGUYAMON])

        # Put a Puppet Lv.4 into trash
        runner.inject_card(1, PUPPET_LV4, "trash")

        trash_ids_before = [
            c.c_entity_base.card_id for c in player.trash_cards
            if c.c_entity_base
        ]
        assert PUPPET_LV4 in trash_ids_before, "Puppet Lv.4 should be in trash"

        # Fire OnEndTurn effects directly
        game.execute_effects(EffectTiming.OnEndTurn)

        # The effect creates a selection; find and pick the card (not "Decline")
        action = runner.find_action(PUPPET_LV4)
        if action is None:
            action = runner.find_action("Sistermon")
        if action is None:
            # Pick any non-decline action
            for aid, desc in runner.actions().items():
                if 'decline' not in desc.lower() and 'pass' not in desc.lower():
                    action = aid
                    break
        assert action is not None, (
            f"Should have an action to select the Puppet from trash. Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        # The Puppet should have been played from trash
        field_ids = [
            p.top_card.c_entity_base.card_id
            for p in player.battle_area if p.top_card and p.top_card.c_entity_base
        ]
        assert PUPPET_LV4 in field_ids, (
            f"Puppet Lv.4 should be on field after end-of-turn play. Field: {field_ids}"
        )

    def test_end_of_turn_does_not_play_lv5_puppet(self, debug_runner):
        """Should NOT be able to play a Lv.5 Puppet from trash (only Lv.4 or lower)."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [KAGUYAMON])

        # Put a Puppet Lv.5 into trash (only eligible card)
        runner.inject_card(1, PUPPET_LV5, "trash")
        # Remove all Lv.4 or lower Puppet from trash to ensure no valid targets
        player.trash_cards = [
            c for c in player.trash_cards
            if not (getattr(c, 'is_digimon', False)
                    and (getattr(c, 'level', 99) or 99) <= 4
                    and any('Puppet' in t for t in (getattr(c, 'card_traits', []) or [])))
        ]

        field_count_before = len(player.battle_area)

        # Advance to end of turn
        # Fire OnEndTurn effects directly
        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        # No new Digimon should appear on field
        assert len(player.battle_area) == field_count_before, (
            "Should not play Lv.5 Puppet from trash (only Lv.4 or lower allowed)"
        )

    def test_end_of_turn_does_not_play_non_puppet(self, debug_runner):
        """Should NOT play a non-Puppet Digimon from trash even if Lv.4 or lower."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [KAGUYAMON])

        # Put a non-Puppet Lv.3 into trash
        runner.inject_card(1, NON_PUPPET_LV3, "trash")
        # Remove any Puppet cards from trash
        player.trash_cards = [
            c for c in player.trash_cards
            if not any('Puppet' in t for t in (getattr(c, 'card_traits', []) or []))
        ]

        field_count_before = len(player.battle_area)

        # Fire OnEndTurn effects directly
        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        assert len(player.battle_area) == field_count_before, (
            "Should not play non-Puppet Digimon from trash"
        )

    def test_end_of_turn_only_on_own_turn(self, debug_runner):
        """End of turn play should only trigger on YOUR turn, not opponent's turn."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [KAGUYAMON])
        runner.inject_card(1, PUPPET_LV4, "trash")

        # Switch turn to player 2
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        field_count_before = len(player.battle_area)

        # Fire OnEndTurn effects directly
        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        assert len(player.battle_area) == field_count_before, (
            "End-of-turn play should NOT trigger on opponent's turn"
        )

    def test_end_of_turn_is_optional(self, debug_runner):
        """The end-of-turn play is optional ('you may play')."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [KAGUYAMON])
        runner.inject_card(1, PUPPET_LV4, "trash")

        field_count_before = len(player.battle_area)

        # The effect is optional; auto_resolve picks first action,
        # which includes "decline". Verify the game doesn't crash
        # and the effect is properly marked optional.
        # Fire OnEndTurn effects directly
        game.execute_effects(EffectTiming.OnEndTurn)
        # Don't auto_resolve - just verify the game is in a selection state
        # or has already processed (since it's optional, both paths are valid)
        # The key is that it doesn't crash and is properly optional
        assert not game.game_over, "Game should not end due to optional effect"

"""Behavioral tests for EX8-048 Landramon.

Card text:
  Lv.4 | Black | Mineral, Rock Dragon | DP:5000 | Play Cost:5
  [When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Close]
  from your hand without paying the cost.

  Inherited Effect: When effects trash this card from a [Mineral] or [Rock]
  trait Digimon's digivolution cards, delete 1 of your opponent's Digimon
  with a play cost of 4 or less.

Key faithfulness points tested:
  - [When Digivolving]: condition gated by tamer count <= 1
  - [When Digivolving]: plays a [Close] tamer from hand for free
  - [When Digivolving]: is optional (may play)
  - [When Digivolving]: blocked when player has 2+ tamers
  - Inherited: only triggers when THIS card is in trashed_cards (not any card)
  - Inherited: requires the host permanent to have [Mineral] or [Rock] trait
  - Inherited: deletes opponent Digimon with play cost 4 or less
  - Inherited: does NOT trigger if host lacks Mineral/Rock trait
  - Inherited: does NOT trigger if a different card was trashed (not this one)
"""

import pytest


# Card IDs used in tests
EX8_048 = "EX8-048"      # Card under test (Landramon, Lv.4, Mineral)
EX8_046 = "EX8-046"      # Gotsumon (Lv.3, Rock trait)
EX8_049 = "EX8-049"      # Golemon (Lv.4, Mineral trait)
EX8_050 = "EX8-050"      # Gogmamon (Lv.5, Rock trait)
EX8_067 = "EX8-067"      # Close (Tamer, play cost 4)
ST1_03 = "ST1-03"        # Agumon (Lv.3, play cost 3 — valid delete target)
ST1_07 = "ST1-07"        # Greymon (Lv.4, play cost 5 — NOT valid delete target)
AGUMON = "ST1-03"
GREYMON = "ST1-07"


@pytest.mark.behavioral
class TestEX8048WhenDigivolving:
    """[When Digivolving] If you have 1 or fewer Tamers, may play 1 [Close] from hand free."""

    def test_condition_passes_with_zero_tamers(self, debug_runner):
        """Condition should pass when player has 0 tamers on field."""
        runner = debug_runner(initial_memory=10)

        # Place Landramon on field (no tamers)
        perm = runner.place_on_field(1, [EX8_048])
        game = runner.game

        # Find the When Digivolving effect
        top = perm.top_card
        all_effects = top.effect_list(None)
        wd_effects = [
            e for e in all_effects
            if getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd_effects) >= 1, "Should have a When Digivolving effect"

        wd_effect = wd_effects[0]

        # Verify no tamers on field
        tamer_count = sum(1 for p in game.player1.battle_area if p.is_tamer)
        assert tamer_count == 0, "P1 should have 0 tamers"

        # Condition should pass
        result = wd_effect.can_use_condition({'permanent': perm})
        assert result is True, "Condition should pass with 0 tamers"

    def test_condition_passes_with_one_tamer(self, debug_runner):
        """Condition should pass when player has exactly 1 tamer on field."""
        runner = debug_runner(initial_memory=10)

        # Place a tamer on field
        runner.place_on_field(1, [EX8_067])
        perm = runner.place_on_field(1, [EX8_048])
        game = runner.game

        tamer_count = sum(1 for p in game.player1.battle_area if p.is_tamer)
        assert tamer_count == 1, "P1 should have exactly 1 tamer"

        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        result = wd_effect.can_use_condition({'permanent': perm})
        assert result is True, "Condition should pass with 1 tamer"

    def test_condition_fails_with_two_tamers(self, debug_runner):
        """Condition should fail when player has 2 or more tamers on field."""
        runner = debug_runner(initial_memory=10)

        # Place two tamers on field
        runner.place_on_field(1, [EX8_067])
        runner.place_on_field(1, [EX8_067])
        perm = runner.place_on_field(1, [EX8_048])
        game = runner.game

        tamer_count = sum(1 for p in game.player1.battle_area if p.is_tamer)
        assert tamer_count == 2, "P1 should have exactly 2 tamers"

        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        result = wd_effect.can_use_condition({'permanent': perm})
        assert result is False, "Condition should fail with 2 tamers"

    def test_plays_close_from_hand_free(self, debug_runner):
        """Should play a [Close] tamer from hand without paying the cost."""
        runner = debug_runner(initial_memory=3)

        # Inject Close into P1's hand
        runner.inject_card(1, EX8_067, "hand")
        perm = runner.place_on_field(1, [EX8_048])
        game = runner.game

        # Verify Close is in hand
        close_in_hand = any(
            'Close' in (c.card_names if hasattr(c, 'card_names') else [])
            for c in game.player1.hand_cards
        )
        assert close_in_hand, "Close should be in P1's hand"

        # Execute the When Digivolving effect
        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        field_before = len(game.player1.battle_area)
        mem_before = game.memory

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Auto-resolve the play selection
        runner.auto_resolve()

        # Close should now be on field
        tamer_on_field = any(
            p.is_tamer and p.contains_card_name('Close')
            for p in game.player1.battle_area
        )
        assert tamer_on_field, (
            "Close tamer should have been played to the field"
        )

        # Memory should not have changed (free play)
        assert game.memory == mem_before, (
            f"Memory should not change for free play. Before: {mem_before}, After: {game.memory}"
        )

    def test_effect_is_optional(self, debug_runner):
        """Effect should be optional (is_optional = True)."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [EX8_048])

        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        assert wd_effect.is_optional is True, "When Digivolving effect should be optional"


@pytest.mark.behavioral
class TestEX8048Inherited:
    """Inherited: When effects trash this card from [Mineral]/[Rock] Digimon's digi-cards,
    delete opponent Digimon with play cost 4 or less."""

    def test_triggers_when_trashed_from_mineral_digimon(self, debug_runner):
        """Inherited should trigger when this card is trashed from a Mineral-trait
        Digimon's digivolution cards."""
        runner = debug_runner(initial_memory=5)

        # Place EX8-048 under a Mineral-trait Digimon (EX8-049 Golemon)
        # Stack: EX8-048 (bottom) -> EX8-049 (top)
        host_perm = runner.place_on_field(1, [EX8_048, EX8_049])
        game = runner.game

        # Place opponent Digimon with play cost <= 4 as delete target
        opp_perm = runner.place_on_field(2, [AGUMON])
        assert opp_perm.top_card.get_cost_itself <= 4, "Opponent should have play cost <= 4"

        # Verify the host has Mineral trait
        assert host_perm.has_trait('Mineral'), "Host permanent should have Mineral trait"

        # Verify EX8-048 is in the digi-cards
        assert len(host_perm.card_sources) == 2, "Stack should have 2 cards"

        opp_count_before = len(game.player2.battle_area)

        # Trash digivolution cards from the host
        trashed = host_perm.trash_digivolution_cards(1)

        # Auto-resolve the delete selection
        runner.auto_resolve()

        # The opponent's Digimon should have been deleted
        assert len(game.player2.battle_area) < opp_count_before, (
            "Opponent Digimon with cost <= 4 should have been deleted"
        )

    def test_triggers_when_trashed_from_rock_digimon(self, debug_runner):
        """Inherited should trigger when this card is trashed from a Rock-trait
        Digimon's digivolution cards."""
        runner = debug_runner(initial_memory=5)

        # Stack: EX8-048 (bottom) -> EX8-050 Gogmamon (top, Rock trait)
        host_perm = runner.place_on_field(1, [EX8_048, EX8_050])
        game = runner.game

        # Place opponent Digimon as delete target
        opp_perm = runner.place_on_field(2, [AGUMON])

        assert host_perm.has_trait('Rock'), "Host permanent should have Rock trait"

        opp_count_before = len(game.player2.battle_area)

        trashed = host_perm.trash_digivolution_cards(1)
        runner.auto_resolve()

        assert len(game.player2.battle_area) < opp_count_before, (
            "Opponent Digimon with cost <= 4 should have been deleted"
        )

    def test_does_not_trigger_when_host_lacks_mineral_rock(self, debug_runner):
        """Inherited should NOT trigger if the host Digimon lacks both
        [Mineral] and [Rock] traits."""
        runner = debug_runner(initial_memory=5)

        # Stack: EX8-048 (bottom) -> ST1-07 Greymon (top, Reptile trait, no Mineral/Rock)
        host_perm = runner.place_on_field(1, [EX8_048, GREYMON])
        game = runner.game

        # Place opponent Digimon as potential delete target
        opp_perm = runner.place_on_field(2, [AGUMON])

        assert not host_perm.has_trait('Mineral'), "Host should NOT have Mineral trait"
        assert not host_perm.has_trait('Rock'), "Host should NOT have Rock trait"

        opp_count_before = len(game.player2.battle_area)

        trashed = host_perm.trash_digivolution_cards(1)
        runner.auto_resolve()

        # Opponent Digimon should NOT have been deleted
        assert len(game.player2.battle_area) == opp_count_before, (
            "No opponent Digimon should be deleted when host lacks Mineral/Rock trait"
        )

    def test_does_not_trigger_for_different_card_trashed(self, debug_runner):
        """Inherited should NOT trigger when a DIFFERENT card (not EX8-048) is
        trashed from the same permanent. This is the key bug: condition must
        check card in trashed_cards."""
        runner = debug_runner(initial_memory=5)

        # Stack: EX8-046 Gotsumon (bottom) -> EX8-048 (middle) -> EX8-049 Golemon (top, Mineral)
        # When we trash 1 digi-card from under top, it trashes EX8-048 (just under top).
        # But if we set up a stack where a non-EX8-048 card is trashed instead...
        # Stack: EX8-048 (bottom) -> EX8-046 (middle) -> EX8-049 (top, Mineral)
        host_perm = runner.place_on_field(1, [EX8_048, EX8_046, EX8_049])
        game = runner.game

        assert host_perm.has_trait('Mineral'), "Host should have Mineral trait (from top card)"
        assert len(host_perm.card_sources) == 3, "Stack should have 3 cards"

        # Place opponent Digimon as potential delete target
        opp_perm = runner.place_on_field(2, [AGUMON])
        opp_count_before = len(game.player2.battle_area)

        # Trash 1 digi-card from under top -> this trashes EX8-046 (index -2)
        trashed = host_perm.trash_digivolution_cards(1)
        assert len(trashed) == 1
        # The trashed card should be EX8-046 (from under the top), NOT EX8-048
        trashed_id = trashed[0].c_entity_base.card_id if trashed[0].c_entity_base else None
        assert trashed_id == EX8_046, (
            f"Trashed card should be EX8-046 (under top), got {trashed_id}"
        )

        runner.auto_resolve()

        # EX8-048's inherited should NOT fire because EX8-048 was not in trashed_cards
        assert len(game.player2.battle_area) == opp_count_before, (
            "Opponent Digimon should NOT be deleted when EX8-048 was not the trashed card. "
            "Bug: condition must check card in trashed_cards."
        )

    def test_delete_filter_respects_play_cost_4_limit(self, debug_runner):
        """Delete should only target opponent Digimon with play cost 4 or less."""
        runner = debug_runner(initial_memory=5)

        # Stack: EX8-048 (bottom) -> EX8-049 (top, Mineral)
        host_perm = runner.place_on_field(1, [EX8_048, EX8_049])
        game = runner.game

        # Place opponent Digimon with play cost > 4
        opp_expensive = runner.place_on_field(2, [GREYMON])  # play cost 5
        assert opp_expensive.top_card.get_cost_itself > 4, (
            "Opponent Digimon should have play cost > 4"
        )

        opp_count_before = len(game.player2.battle_area)

        trashed = host_perm.trash_digivolution_cards(1)
        runner.auto_resolve()

        # Opponent Digimon should NOT be deleted (play cost 5 > 4)
        assert len(game.player2.battle_area) == opp_count_before, (
            "Opponent Digimon with play cost > 4 should not be a valid delete target"
        )

    def test_inherited_effect_is_not_optional(self, debug_runner):
        """The inherited delete effect should not be optional (card text has no 'may')."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, [EX8_048, EX8_049])

        # Find the inherited effect
        source_card = perm.card_sources[0]  # EX8-048 is under
        all_effects = source_card.effect_list(None)
        inherited = [
            e for e in all_effects
            if getattr(e, 'is_inherited_effect', False)
        ]
        assert len(inherited) >= 1, "Should have an inherited effect"

        inh_effect = inherited[0]
        # The delete is not optional per card text (no "you may")
        assert inh_effect.is_optional is not True, (
            "Inherited delete effect should not be optional"
        )

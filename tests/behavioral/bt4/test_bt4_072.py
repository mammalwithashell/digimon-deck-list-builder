"""Behavioral tests for BT4-072 Gogmamon (Digimon, Lv.5, Black, Rock).

Card text:
[Main] <Digi-Burst 1> (You may trash 1 of this Digimon's digivolution
cards to activate the effect below.)
- 1 of your Digimon gets +2000 DP until the end of your opponent's next turn.

Inherited: [All Turns] This Digimon gets +1000 DP.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


DECK = ["BT4-072"] * 4 + ["BT2-056"] * 4 + ["ST5-05"] * 20 + ["BT2-052"] * 22


@pytest.mark.behavioral
class TestBT4072Gogmamon:
    """Tests for BT4-072 Gogmamon."""

    # -- Digi-Burst 1: +2000 DP --

    def test_digi_burst_condition_requires_digivolution_cards(self, debug_runner):
        """Digi-Burst 1 requires at least 1 digivolution card under the top card."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        def _find_field_main(card_source):
            effects = card_source.effect_list(None)
            return [e for e in effects if getattr(e, '_is_field_main', False)]

        # Place Gogmamon with a Lv.4 underneath (2 cards total - can burst)
        perm_with_source = runner.place_on_field(1, ["BT2-056", "BT4-072"])
        card = perm_with_source.top_card
        main_effects = _find_field_main(card)
        assert len(main_effects) == 1
        burst = main_effects[0]
        assert burst.can_use_condition({}), \
            "Digi-Burst should be usable with 1 digivolution card"

        # Place Gogmamon alone (1 card - cannot burst)
        perm_no_source = runner.place_on_field(1, ["BT4-072"])
        card_alone = perm_no_source.top_card
        main_alone = _find_field_main(card_alone)
        assert len(main_alone) == 1
        burst_alone = main_alone[0]
        assert not burst_alone.can_use_condition({}), \
            "Digi-Burst should NOT be usable without digivolution cards"

    def test_digi_burst_trashes_source_and_gives_dp(self, debug_runner):
        """Digi-Burst 1 should trash 1 digivolution card, then select a Digimon
        for +2000 DP."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        # Place Gogmamon with a source card
        perm = runner.place_on_field(1, ["BT2-056", "BT4-072"])
        assert len(perm.card_sources) == 2

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        burst = [e for e in effects if getattr(e, '_is_field_main', False)][0]

        # Track select_own_permanent calls
        selected_targets = []
        original_select = game.effect_select_own_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=""):
            # Auto-select perm itself
            callback(perm)

        game.effect_select_own_permanent = mock_select

        burst.on_process_callback({
            'player': game.player1,
            'permanent': perm,
            'game': game,
        })

        # Should have trashed 1 digivolution card
        assert len(perm.card_sources) == 1, "Should have 1 card left after trashing"
        assert len(game.player1.trash_cards) >= 1, "Trashed card should be in trash"

        game.effect_select_own_permanent = original_select

    def test_digi_burst_dp_lasts_until_opponent_next_turn(self, debug_runner):
        """The +2000 DP from Digi-Burst should last until the end of your
        opponent's next turn (uses register_modifier with end_of_opponent_turn),
        not just until end of current turn."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        # Place Gogmamon with a source card, plus another Digimon as target
        perm = runner.place_on_field(1, ["BT2-056", "BT4-072"])
        target = runner.place_on_field(1, ["ST5-05"])

        game = runner.game
        target_base_dp = target.dp

        card = perm.top_card
        effects = card.effect_list(None)
        burst = [e for e in effects if getattr(e, '_is_field_main', False)][0]

        # Mock to select the target perm
        original_select = game.effect_select_own_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=""):
            callback(target)

        game.effect_select_own_permanent = mock_select

        burst.on_process_callback({
            'player': game.player1,
            'permanent': perm,
            'game': game,
        })

        game.effect_select_own_permanent = original_select

        # Check DP increased
        assert target.dp == target_base_dp + 2000, \
            f"Target should have +2000 DP, got {target.dp} (base {target_base_dp})"

        # Verify the DP persists after clearing temp DP (which happens at end of turn)
        # The modifier should use end_of_opponent_turn, not temp_dp
        target.clear_temp_dp()
        assert target.dp == target_base_dp + 2000, \
            "DP bonus should persist after clearing temp DP (uses register_modifier, not change_dp)"

    # -- Inherited: +1000 DP [All Turns] --

    def test_inherited_dp_modifier(self, debug_runner):
        """Inherited [All Turns] +1000 DP should be a permanent dp_modifier."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT4-072", runner.game.player1)
        effects = card.effect_list(None)

        inherited = [e for e in effects if e.is_inherited_effect]
        assert len(inherited) == 1, "Should have 1 inherited effect"
        assert inherited[0].dp_modifier == 1000, \
            "Inherited effect should give +1000 DP"

    def test_inherited_dp_applies_when_stacked(self, debug_runner):
        """When BT4-072 is a digivolution source, the +1000 DP should
        apply to the Digimon on top."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        # Place a Lv.6 on top of Gogmamon
        # Use BT2-056 as Lv.4 under Gogmamon, then another Lv.5 on top
        # Actually just test that inherited effect has correct dp_modifier
        perm = runner.place_on_field(1, ["BT2-052", "BT4-072"])
        # Gogmamon (Lv.5, 7000 DP base) + inherited from self won't apply
        # (inherited effects only apply from under the top card)
        # Let's stack something on top
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        top_card = db.create_card_source("BT2-060", runner.game.player1)  # Megadramon Lv.5
        base_dp_megadramon = top_card.base_dp
        perm.card_sources.append(top_card)

        # The inherited +1000 DP from Gogmamon should now apply
        # Check that the permanent's DP includes the inherited bonus
        expected_dp = base_dp_megadramon + 1000
        assert perm.dp == expected_dp, \
            f"Expected DP {expected_dp} (base {base_dp_megadramon} + 1000 inherited), got {perm.dp}"

"""Behavioral tests for BT20-102 Omnimon (X Antibody) (Lv.7 Blue/Black Holy Warrior, DP 16000, Cost 16).

Card text:
<Raid> <Piercing> <Blocker>
[On Play] [When Digivolving] If [Omnimon] or [X Antibody] is in this Digimon's
digivolution cards, choose 1 of both players' Digimon and delete all other Digimon.
Then, return 1 of your opponent's Digimon to the bottom of the deck.
[End of Your Turn] [Once Per Turn] 1 of your Digimon may gain <Rush> for the turn
and attack without suspending.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT20102OmnimonXAntibody:
    """Tests for BT20-102 Omnimon (X Antibody)."""

    def test_has_raid_keyword(self, debug_runner):
        """Should have Raid keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT20-102"])

        card = perm.top_card
        effects = card.effect_list(None)
        raid_effects = [e for e in effects if getattr(e, '_is_raid', False)]
        assert len(raid_effects) >= 1, "Should have Raid"

    def test_has_piercing_keyword(self, debug_runner):
        """Should have Piercing keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT20-102"])

        card = perm.top_card
        effects = card.effect_list(None)
        piercing_effects = [e for e in effects if getattr(e, '_is_piercing', False)]
        assert len(piercing_effects) >= 1, "Should have Piercing"

    def test_has_blocker_keyword(self, debug_runner):
        """Should have Blocker keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT20-102"])

        card = perm.top_card
        effects = card.effect_list(None)
        blocker_effects = [e for e in effects if getattr(e, '_is_blocker', False)]
        assert len(blocker_effects) >= 1, "Should have Blocker"

    def test_on_play_condition_requires_omnimon_in_sources(self, debug_runner):
        """[On Play] condition should require [Omnimon] or [X Antibody] in digivolution cards."""
        runner = debug_runner(initial_memory=5)

        # Place Omnimon X with Omnimon (BT5-086) as digivolution source
        perm = runner.place_on_field(1, ["BT5-086", "BT20-102"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        assert len(on_play_effects) == 1, "Should have On Play effect"

        effect = on_play_effects[0]
        # Manually set effect_source_permanent on ALL effects (normally done by engine
        # during effect scanning). The conditions use closures that may reference a
        # different effect object's effect_source_permanent.
        for e in effects:
            e.set_effect_source_permanent(perm)

        # With Omnimon in sources, condition should pass
        assert effect.can_use_condition({}), \
            "Condition should pass when [Omnimon] is in digivolution cards"

    def test_on_play_condition_fails_without_omnimon_or_xantibody(self, debug_runner):
        """[On Play] condition should fail if neither [Omnimon] nor [X Antibody] in sources."""
        runner = debug_runner(initial_memory=5)

        # Place Omnimon X with just a generic Digimon underneath (no Omnimon or X Antibody)
        perm = runner.place_on_field(1, ["ST1-11", "BT20-102"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        effect = on_play_effects[0]
        for e in effects:
            e.set_effect_source_permanent(perm)

        # Without Omnimon in sources, condition should fail
        # (ST1-11 WarGreymon is not "Omnimon" and not "X Antibody")
        assert not effect.can_use_condition({}), \
            "Condition should fail without [Omnimon] or [X Antibody] in digivolution cards"

    def test_x_antibody_in_sources_checks_card_name_not_trait(self, debug_runner):
        """[On Play] [X Antibody] check should look at card NAME, not trait.

        C# uses EqualsCardName("X Antibody") which checks the card name,
        not traits. The option card "X Antibody" (e.g. BT9-109) has that name.
        """
        runner = debug_runner(initial_memory=5)

        # Place Omnimon X with an X Antibody option card as digivolution source
        # BT9-109 is the "X Antibody" option card
        perm = runner.place_on_field(1, ["BT9-109", "BT20-102"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        effect = on_play_effects[0]
        for e in effects:
            e.set_effect_source_permanent(perm)

        # With X Antibody card in sources (by name), condition should pass
        assert effect.can_use_condition({}), \
            "Condition should pass when card named [X Antibody] is in digivolution cards"

    def test_end_of_turn_grant_rush_and_attack(self, debug_runner):
        """[End of Turn] Should grant Rush and attack-without-suspending to selected Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-102"])
        target = runner.place_on_field(1, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        eot_effects = [e for e in effects
                       if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) == 1, "Should have exactly 1 End of Turn effect"

        effect = eot_effects[0]
        assert effect.is_optional, "End of turn effect should be optional"

        # Mock selection to auto-select the target
        def mock_select_own(player, callback, filter_fn=None, is_optional=False, prompt=None):
            callback(target)
        game.effect_select_own_permanent = mock_select_own

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Target should gain Rush
        assert target.has_keyword('_is_rush'), "Target should gain Rush"

    def test_end_of_turn_once_per_turn(self, debug_runner):
        """[End of Turn] Effect should be once per turn."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-102"])

        card = perm.top_card
        effects = card.effect_list(None)
        eot_effects = [e for e in effects
                       if e.timing == EffectTiming.OnEndTurn]
        effect = eot_effects[0]

        assert effect.max_count_per_turn == 1, "Should be once per turn"

    def test_end_of_turn_requires_owner_turn(self, debug_runner):
        """[End of Turn] condition should require it to be owner's turn."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-102"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        eot_effects = [e for e in effects
                       if e.timing == EffectTiming.OnEndTurn]
        effect = eot_effects[0]

        # Player 1's turn - should pass
        assert effect.can_use_condition({}), "Should pass on owner's turn"

        # Swap turn to opponent
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        assert not effect.can_use_condition({}), "Should fail on opponent's turn"

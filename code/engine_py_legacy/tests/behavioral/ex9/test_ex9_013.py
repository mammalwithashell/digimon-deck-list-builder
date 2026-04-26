"""Behavioral tests for EX9-013 BlitzGreymon (Lv.6 Red/Black Cyborg, DP 12000, Cost 7).

Card text:
[Hand] [Counter] <Blast Digivolve>
<Alliance> <Blocker>
[On Play] [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.
[End of Your Turn] 2 of your Digimon may DNA digivolve into [Omnimon Alter-S]
in the hand. Then, 1 of your Digimon may attack.
Inherited: Ace Overflow <-4>

Note: C# also has Security Attack +1 inherited (ChangeSelfSAttackStaticEffect).
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX9013BlitzGreymon:
    """Tests for EX9-013 BlitzGreymon."""

    def test_has_blast_digivolve(self, debug_runner):
        """Should have Blast Digivolve."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX9-013"])

        card = perm.top_card
        effects = card.effect_list(None)
        blast_effects = [e for e in effects if getattr(e, '_is_blast_digivolve', False)]
        assert len(blast_effects) >= 1, "Should have Blast Digivolve"

    def test_has_alliance(self, debug_runner):
        """Should have Alliance keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX9-013"])

        card = perm.top_card
        effects = card.effect_list(None)
        alliance_effects = [e for e in effects if getattr(e, '_is_alliance', False)]
        assert len(alliance_effects) >= 1, "Should have Alliance"

    def test_has_blocker(self, debug_runner):
        """Should have Blocker keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX9-013"])

        card = perm.top_card
        effects = card.effect_list(None)
        blocker_effects = [e for e in effects if getattr(e, '_is_blocker', False)]
        assert len(blocker_effects) >= 1, "Should have Blocker"

    def test_on_play_de_digivolve_3(self, debug_runner):
        """[On Play] Should De-Digivolve 3 on 1 opponent's Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX9-013"])
        # Build a 4-card digivolution stack of actual Digimon:
        # ST1-03 Agumon (Lv3) -> ST1-06 Coredramon (Lv4) -> ST1-09 MetalGreymon (Lv5) -> ST1-11 WarGreymon (Lv6)
        opp_perm = runner.place_on_field(2, ["ST1-03", "ST1-06", "ST1-09", "ST1-11"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        assert len(on_play_effects) == 1, "Should have On Play De-Digivolve effect"

        effect = on_play_effects[0]
        initial_stack_size = len(opp_perm.card_sources)
        assert initial_stack_size == 4, "Opponent should have 4-card stack"

        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select_opp

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should have removed 3 cards from the top (4 -> 1)
        assert len(opp_perm.card_sources) == 1, \
            "De-Digivolve 3 should remove 3 cards, leaving only the base card"

    def test_on_play_de_digivolve_sends_to_trash(self, debug_runner):
        """[On Play] De-Digivolve should send removed cards to the opponent's trash."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX9-013"])
        # All Digimon cards for the stack
        opp_perm = runner.place_on_field(2, ["ST1-03", "ST1-06", "ST1-09", "ST1-11"])

        game = runner.game
        trash_before = len(game.player2.trash_cards)

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        effect = on_play_effects[0]

        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select_opp

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Removed cards should go to enemy trash
        assert len(game.player2.trash_cards) > trash_before, \
            "De-Digivolve should send removed cards to opponent's trash"

    def test_when_digivolving_de_digivolve_3(self, debug_runner):
        """[When Digivolving] Should also have De-Digivolve 3 effect."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX9-013"])

        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [e for e in effects
                      if e.timing == EffectTiming.OnEnterFieldAnyone
                      and e.is_when_digivolving]
        assert len(wd_effects) == 1, "Should have When Digivolving De-Digivolve effect"

    def test_end_of_turn_attack_grant(self, debug_runner):
        """[End of Turn] After DNA step, 1 Digimon may attack (FORCE_ATTACK)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX9-013"])
        target = runner.place_on_field(1, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        eot_effects = [e for e in effects
                       if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) == 1, "Should have End of Turn effect"

        effect = eot_effects[0]

        # Mock selection: skip DNA (no Omnimon Alter-S in hand), then select target to attack
        def mock_select_own(player, callback, filter_fn=None, is_optional=False, prompt=None):
            callback(target)
        game.effect_select_own_permanent = mock_select_own

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should have FORCE_ATTACK on the selected target
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        has_force = game.modifiers.has_modifier(target, ModifierType.FORCE_ATTACK)
        assert has_force, "Selected Digimon should have FORCE_ATTACK modifier"

    def test_end_of_turn_requires_owner_turn(self, debug_runner):
        """[End of Turn] condition should require owner's turn."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX9-013"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        eot_effects = [e for e in effects
                       if e.timing == EffectTiming.OnEndTurn]
        effect = eot_effects[0]

        # Player 1's turn - should pass
        assert effect.can_use_condition({}), "Should pass on owner's turn"

        # Swap to opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        assert not effect.can_use_condition({}), "Should fail on opponent's turn"

    def test_inherited_security_attack_plus_1(self, debug_runner):
        """Inherited effect should provide Security Attack +1 (per C# behavioral reference)."""
        runner = debug_runner(initial_memory=5)

        # Place BlitzGreymon as a digivolution source under another Digimon
        # ST1-11 WarGreymon (Lv.6) on top of EX9-013 BlitzGreymon (Lv.6)
        perm = runner.place_on_field(1, ["EX9-013", "ST1-11"])

        card = perm.card_sources[0]  # BlitzGreymon is the bottom card
        effects = card.effect_list(None)
        sa_effects = [e for e in effects
                      if getattr(e, '_security_attack_modifier', 0) != 0
                      and e.is_inherited_effect]
        assert len(sa_effects) >= 1, \
            "Should have Security Attack +1 inherited effect (per C# reference)"
        assert sa_effects[0]._security_attack_modifier == 1, "Should be +1"

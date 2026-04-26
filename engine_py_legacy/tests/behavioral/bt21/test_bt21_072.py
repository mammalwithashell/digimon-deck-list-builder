"""Behavioral tests for BT21-072 Arresterdramon: Superior Mode (Lv.5 Red/Purple).

Card text:
<Raid> <Piercing>
[When Digivolving] This Digimon may attack without suspending.
[All Turns] This Digimon gets +1000 DP for each of its digivolution cards.
Inherited: [Your Turn] This Digimon gets +2000 DP.

Alt-digi: Lv.4 w/<Save> in text or w/[Hero] trait: Cost 3
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT21072ArresterdraSuperiorMode:
    """Tests for BT21-072 Arresterdramon: Superior Mode."""

    def test_raid_flag_present(self, debug_runner):
        """Raid keyword should be present on the card's effects."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST1-03", "ST1-06", "BT21-072"])

        # Check raid flag on effects
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        raid_effects = [e for e in effects if getattr(e, '_is_raid', False)]
        assert len(raid_effects) >= 1, "Should have at least 1 Raid effect"

    def test_piercing_flag_present(self, debug_runner):
        """Piercing keyword should be present on the card's effects."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST1-03", "ST1-06", "BT21-072"])

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        piercing_effects = [e for e in effects if getattr(e, '_is_piercing', False)]
        assert len(piercing_effects) >= 1, "Should have at least 1 Piercing effect"

    def test_dp_bonus_per_digivolution_card(self, debug_runner):
        """[All Turns] +1000 DP per digivolution card.
        With 2 digi cards (ST1-03, ST1-06) under BT21-072, DP should be base + 2000."""
        runner = debug_runner(initial_memory=5)
        # Stack: ST1-03 (bottom), ST1-06 (middle), BT21-072 (top)
        # 2 digivolution cards = +2000 DP
        perm = runner.place_on_field(1, ["ST1-03", "ST1-06", "BT21-072"])

        base_dp = perm.top_card.c_entity_base.dp  # 10000
        expected_dp = base_dp + 2000  # 2 digi cards * 1000
        assert perm.dp == expected_dp, (
            f"DP should be {expected_dp} (base {base_dp} + 2000 from 2 digi cards), got {perm.dp}"
        )

    def test_dp_bonus_scales_with_stack_depth(self, debug_runner):
        """DP bonus should scale with number of digivolution cards."""
        runner = debug_runner(initial_memory=5)
        # Stack: BT1-003 (egg), ST1-03, ST1-06, BT21-072 = 3 digi cards
        perm = runner.place_on_field(1, ["BT1-003", "ST1-03", "ST1-06", "BT21-072"])

        base_dp = perm.top_card.c_entity_base.dp  # 10000
        expected_dp = base_dp + 3000  # 3 digi cards * 1000
        assert perm.dp == expected_dp, (
            f"DP should be {expected_dp} (base {base_dp} + 3000 from 3 digi cards), got {perm.dp}"
        )

    def test_inherited_dp_boost_your_turn(self, debug_runner):
        """Inherited [Your Turn] +2000 DP should apply when it's owner's turn."""
        runner = debug_runner(initial_memory=5)
        # Place BT21-072 as an inherited source under a Lv.6
        # ST1-11 is WarGreymon Lv.6 with base 12000 DP
        perm = runner.place_on_field(1, ["ST1-03", "BT21-072", "ST1-11"])

        game = runner.game
        # Ensure it's player 1's turn
        assert game.player1.is_my_turn, "Should be P1's turn"

        # DP should include inherited +2000
        base_dp = perm.top_card.c_entity_base.dp  # 12000
        expected_dp = base_dp + 2000
        assert perm.dp == expected_dp, (
            f"DP should be {expected_dp} (base {base_dp} + 2000 inherited), got {perm.dp}"
        )

    def test_inherited_dp_boost_not_opponent_turn(self, debug_runner):
        """Inherited [Your Turn] +2000 DP should NOT apply on opponent's turn."""
        runner = debug_runner(initial_memory=5, first_player=2)
        # Place BT21-072 as inherited source under a Lv.6 for player 1
        perm = runner.place_on_field(1, ["ST1-03", "BT21-072", "ST1-11"])

        game = runner.game
        # Player 2 goes first, so player 1's turn is False
        assert not game.player1.is_my_turn, "Should be P2's turn, not P1's"

        # DP should NOT include inherited +2000
        base_dp = perm.top_card.c_entity_base.dp  # 12000
        assert perm.dp == base_dp, (
            f"DP should be {base_dp} (no inherited bonus on opponent's turn), got {perm.dp}"
        )

    def test_when_digivolving_attack_without_suspending(self, debug_runner):
        """[When Digivolving] should grant CAN_ATTACK_UNSUSPENDED modifier."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST1-03", "ST1-06", "BT21-072"])

        game = runner.game
        # Find the When Digivolving effect for attack-without-suspending
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wdigi_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wdigi_effects) >= 1, "Should have at least 1 When Digivolving effect"

        # Execute the effect manually
        wd_effect = wdigi_effects[0]
        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Check CAN_ATTACK_UNSUSPENDED modifier exists
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        has_unsuspended = game.modifiers.has_modifier(perm, ModifierType.CAN_ATTACK_UNSUSPENDED)
        assert has_unsuspended, "Should have CAN_ATTACK_UNSUSPENDED modifier after When Digivolving"

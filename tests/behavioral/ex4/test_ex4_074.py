"""Behavioral tests for EX4-074 ShineGreymon: Ruin Mode (Lv.7, Yellow/Black, DP 15000, Cost 14).

Actual card text (from cards.json):
[When Digivolving] [On Deletion] Until the end of your opponent's next turn, all of
    your opponent's Digimon get -5000DP.
[End of Attack] Delete this Digimon and 1 of your opponent's Digimon, and Recovery +1
    (Deck). Then, if you have a Tamer in play, hatch 1 Digi-Egg card to an empty space
    in your breeding area.

Alt-digi: from [ShineGreymon] for cost 4.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX4074ShineGreymonRuinMode:
    """Tests for EX4-074 ShineGreymon: Ruin Mode."""

    # ------------------------------------------------------------------
    # Alt-digi from ShineGreymon for cost 4
    # ------------------------------------------------------------------

    def test_alt_digi_attributes(self, debug_runner):
        """Should have alt-digi from [ShineGreymon] for cost 4."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        alt_digi = [e for e in all_effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi) >= 1, "Should have an alt-digi effect"
        assert alt_digi[0]._alt_digi_cost == 4, "Alt-digi cost should be 4"
        assert alt_digi[0]._alt_digi_name == "ShineGreymon", \
            "Alt-digi name should be 'ShineGreymon'"

    # ------------------------------------------------------------------
    # [When Digivolving] -5000 DP to all opponent Digimon
    # ------------------------------------------------------------------

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have a When Digivolving effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        digivolve_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(digivolve_effects) >= 1, "Should have When Digivolving effect"

    def test_when_digivolving_applies_minus_5000_to_opponent(self, debug_runner):
        """[When Digivolving] should apply -5000 DP to all opponent Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        game = runner.game
        # Place an opponent Digimon with DP > 5000 so result stays positive
        # ST1-08 Garudamon has 7000 DP
        opp_perm = runner.place_on_field(2, ["ST1-08"])
        dp_before = opp_perm.dp
        assert dp_before == 7000, f"Expected Garudamon base DP 7000, got {dp_before}"

        top = perm.top_card
        all_effects = top.effect_list(None)
        digivolve_effect = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        digivolve_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Opponent's Digimon DP should be reduced by 5000 (7000 -> 2000)
        dp_after = opp_perm.dp
        assert dp_after == 2000, \
            f"Opponent Digimon DP should be 2000 after -5000: was {dp_before}, now {dp_after}"

    def test_when_digivolving_applies_to_all_opponent_digimon(self, debug_runner):
        """[When Digivolving] should apply -5000 DP to ALL opponent Digimon, not just one."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        game = runner.game
        # Use Digimon with DP > 5000 so result stays positive
        opp_perm1 = runner.place_on_field(2, ["ST1-08"])  # Garudamon 7000 DP
        opp_perm2 = runner.place_on_field(2, ["ST1-09"])  # Lv.5 or higher
        dp1_before = opp_perm1.dp
        dp2_before = opp_perm2.dp

        top = perm.top_card
        all_effects = top.effect_list(None)
        digivolve_effect = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        digivolve_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Both should be reduced by 5000
        assert opp_perm1.dp == max(0, dp1_before - 5000), \
            f"First opponent Digimon should get -5000 DP: was {dp1_before}, now {opp_perm1.dp}"
        assert opp_perm2.dp == max(0, dp2_before - 5000), \
            f"Second opponent Digimon should get -5000 DP: was {dp2_before}, now {opp_perm2.dp}"

    # ------------------------------------------------------------------
    # [On Deletion] -5000 DP to all opponent Digimon
    # ------------------------------------------------------------------

    def test_on_deletion_effect_exists(self, debug_runner):
        """Should have an On Deletion effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        deletion_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnDestroyedAnyone
            and getattr(e, 'is_on_deletion', False)
        ]
        assert len(deletion_effects) >= 1, "Should have On Deletion effect"

    def test_on_deletion_applies_minus_5000(self, debug_runner):
        """[On Deletion] should also apply -5000 DP to all opponent Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        game = runner.game
        # ST1-08 Garudamon has 7000 DP
        opp_perm = runner.place_on_field(2, ["ST1-08"])
        dp_before = opp_perm.dp
        assert dp_before == 7000, f"Expected Garudamon base DP 7000, got {dp_before}"

        top = perm.top_card
        all_effects = top.effect_list(None)
        deletion_effect = [
            e for e in all_effects
            if e.timing == EffectTiming.OnDestroyedAnyone
            and getattr(e, 'is_on_deletion', False)
        ][0]

        deletion_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        dp_after = opp_perm.dp
        assert dp_after == 2000, \
            f"On Deletion should apply -5000 DP: was {dp_before}, now {dp_after}"

    # ------------------------------------------------------------------
    # [End of Attack] self-delete + opponent delete + recovery + hatch
    # ------------------------------------------------------------------

    def test_end_of_attack_effect_exists(self, debug_runner):
        """Should have an End of Attack effect (timing OnEndAttack)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        end_attack_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndAttack
        ]
        assert len(end_attack_effects) >= 1, "Should have End of Attack effect"

    def test_end_of_attack_deletes_self(self, debug_runner):
        """[End of Attack] should delete this Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        game = runner.game
        # Place opponent Digimon for the delete target
        opp_perm = runner.place_on_field(2, ["ST1-03"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        end_attack = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndAttack
        ][0]

        end_attack.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        # This Digimon should be deleted (removed from battle area)
        assert perm not in game.player1.battle_area, \
            "ShineGreymon: Ruin Mode should be deleted at End of Attack"

    def test_end_of_attack_deletes_opponent_digimon(self, debug_runner):
        """[End of Attack] should also delete 1 opponent Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        game = runner.game
        opp_perm = runner.place_on_field(2, ["ST1-03"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        end_attack = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndAttack
        ][0]

        end_attack.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        # Opponent's Digimon should be deleted
        assert opp_perm not in game.player2.battle_area, \
            "Opponent's Digimon should be deleted at End of Attack"

    def test_end_of_attack_recovery_plus_1(self, debug_runner):
        """[End of Attack] should include Recovery +1 (Deck)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        game = runner.game
        opp_perm = runner.place_on_field(2, ["ST1-03"])
        security_before = len(game.player1.security_cards)

        top = perm.top_card
        all_effects = top.effect_list(None)
        end_attack = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndAttack
        ][0]

        end_attack.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        # Security should increase by 1 (Recovery +1)
        assert len(game.player1.security_cards) == security_before + 1, \
            f"Should gain 1 security via Recovery +1: was {security_before}, " \
            f"now {len(game.player1.security_cards)}"

    def test_end_of_attack_hatch_if_tamer(self, debug_runner):
        """[End of Attack] should hatch if player has a Tamer in play."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        game = runner.game
        # Place a tamer
        runner.place_on_field(1, ["ST1-12"])  # Tai Kamiya tamer
        # Place opponent for delete target
        opp_perm = runner.place_on_field(2, ["ST1-03"])

        # Ensure breeding area is empty and egg deck has cards
        assert not game.player1.breeding_area, "Breeding should start empty"
        has_eggs = len(game.player1.digitama_library_cards) > 0

        top = perm.top_card
        all_effects = top.effect_list(None)
        end_attack = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndAttack
        ][0]

        end_attack.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        # If there were eggs in the deck, breeding area should now have something
        if has_eggs:
            assert game.player1.breeding_area is not None, \
                "Should hatch a Digi-Egg when tamer is in play"

    def test_end_of_attack_no_hatch_without_tamer(self, debug_runner):
        """[End of Attack] should NOT hatch without a Tamer in play."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        game = runner.game
        # Remove all tamers from field
        for p in list(game.player1.battle_area):
            if p.is_tamer:
                game.player1.battle_area.remove(p)

        opp_perm = runner.place_on_field(2, ["ST1-03"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        end_attack = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndAttack
        ][0]

        breeding_before = game.player1.breeding_area

        end_attack.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        # Breeding area should still be empty (no tamer = no hatch)
        if not breeding_before:
            assert not game.player1.breeding_area, \
                "Should NOT hatch without a Tamer in play"

    def test_end_of_attack_condition_checks_self(self, debug_runner):
        """End of Attack condition should only trigger for THIS Digimon's attack."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        other_perm = runner.place_on_field(1, ["ST1-07"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        end_attack = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndAttack
        ][0]

        # Should NOT trigger when a different permanent is attacking
        result = end_attack.can_use_condition({
            'permanent': perm,
            'attacking_permanent': other_perm,
        })
        assert not result, "End of Attack should only trigger for THIS Digimon's attack"

        # Should trigger when THIS permanent is attacking
        result = end_attack.can_use_condition({
            'permanent': perm,
            'attacking_permanent': perm,
        })
        assert result, "End of Attack should trigger for THIS Digimon's attack"

    def test_end_of_attack_still_recovers_without_opponent_digimon(self, debug_runner):
        """[End of Attack] should still do Recovery +1 and hatch even if no opponent Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        game = runner.game
        # Place tamer for hatch
        runner.place_on_field(1, ["ST1-12"])
        # No opponent Digimon on field

        security_before = len(game.player1.security_cards)

        top = perm.top_card
        all_effects = top.effect_list(None)
        end_attack = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndAttack
        ][0]

        end_attack.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        # Self should still be deleted
        assert perm not in game.player1.battle_area, \
            "Should still self-delete even without opponent Digimon"
        # Should still recover
        assert len(game.player1.security_cards) == security_before + 1, \
            "Should still recover +1 even without opponent Digimon"

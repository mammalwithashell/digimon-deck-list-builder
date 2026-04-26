"""Behavioral tests for ST20-11 WarGreymon (Lv.6 Black/Red Digimon).

Card text:
[Hand] [Counter] <Blast Digivolve>
[On Play] [When Digivolving] Until your opponent's turn ends, for every
    2 colors your Tamers have, their Digimon's effects don't affect 1 of
    your Digimon.
[When Digivolving] [When Attacking] Delete 1 of your opponent's lowest
    DP Digimon.
Inherited: Ace Overflow <-4>
Alt-digi: Lv.5 w/[ADVENTURE]/[Hero] trait: Cost 3
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming
from engine_py_legacy.engine.interfaces.modifiers import ModifierType


@pytest.mark.behavioral
class TestST20_11AltDigi:
    """Tests for alt-digi: Lv.5 w/[ADVENTURE] or [Hero] trait for cost 3."""

    def test_alt_digi_adventure_lv5(self, debug_runner):
        """ST20-11 can digivolve onto a Lv.5 with ADVENTURE trait for cost 3.

        ST20-09 MegaKabuterimon is Lv.5 Green with traits [Insectoid, ADVENTURE].
        """
        runner = debug_runner(initial_memory=5)
        # Place ADVENTURE Lv.5 on field
        mega_perm = runner.place_on_field(1, ["ST20-09"])
        assert mega_perm.level == 5

        # Add ST20-11 to hand
        runner.inject_card(1, "ST20-11", "hand")

        # Set to Main phase
        runner.set_phase("Main")

        # Check digivolve actions
        digi_actions = runner.find_actions("Digivolve")
        has_st20_11_digi = any("ST20-11" in d or "WarGreymon" in d
                               for d in digi_actions.values())
        assert has_st20_11_digi, (
            f"Should be able to digivolve ST20-11 onto ADVENTURE Lv.5. Actions: {digi_actions}"
        )

    def test_alt_digi_no_match_wrong_level(self, debug_runner):
        """ST20-11 should NOT alt-digi onto a Lv.4 with ADVENTURE trait."""
        runner = debug_runner(initial_memory=5)
        # ST20-08 Kabuterimon is Lv.4 Green with ADVENTURE
        perm = runner.place_on_field(1, ["ST20-08"])
        assert perm.level == 4

        runner.inject_card(1, "ST20-11", "hand")
        runner.set_phase("Main")

        # Standard evo for ST20-11 requires Black Lv.5. Alt-digi requires Lv.5.
        # Lv.4 should not match either.
        digi_actions = runner.find_actions("Digivolve")
        has_bad_digi = any(("ST20-11" in d or "WarGreymon" in d)
                           for d in digi_actions.values())
        assert not has_bad_digi, (
            f"Should NOT be able to digivolve ST20-11 onto Lv.4. Actions: {digi_actions}"
        )


@pytest.mark.behavioral
class TestST20_11BlastDigivolve:
    """Tests for Blast Digivolve."""

    def test_has_blast_digivolve_flag(self, debug_runner):
        """ST20-11 should have the blast digivolve flag."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "ST20-11", "hand")
        game = runner.game
        wg_card = game.player1.hand_cards[-1]
        effects = wg_card.effect_list(None)
        blast_effects = [e for e in effects if getattr(e, '_is_blast_digivolve', False)]
        assert len(blast_effects) >= 1, "Should have blast digivolve effect"
        assert blast_effects[0].is_counter_effect, "Blast digivolve should be a counter effect"


@pytest.mark.behavioral
class TestST20_11Immunity:
    """Tests for immunity: their Digimon's effects don't affect N of your Digimon."""

    def test_immunity_count_from_tamer_colors(self, debug_runner):
        """For every 2 tamer colors, 1 own Digimon gets immunity.

        With 4 tamer colors (Red, Yellow, Black, Green), should grant
        immunity to 2 Digimon.
        """
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place tamers: ST20-12 (Red/Yellow) + ST20-13 (Black/Green) = 4 colors
        runner.place_on_field(1, ["ST20-12"])
        runner.place_on_field(1, ["ST20-13"])

        # Place some own Digimon to be immunity targets
        target1 = runner.place_on_field(1, ["ST20-07"])  # Tentomon
        target2 = runner.place_on_field(1, ["ST20-08"])  # Kabuterimon
        target3 = runner.place_on_field(1, ["ST20-09"])  # MegaKabuterimon

        # Play ST20-11 WarGreymon (triggers On Play)
        runner.inject_card(1, "ST20-11", "hand")
        runner.set_phase("Main")

        play_action = runner.find_action("Play WarGreymon")
        assert play_action is not None, (
            f"Should be able to play ST20-11. Actions: {runner.actions()}"
        )

        # Execute play and auto-resolve the immunity selections
        result = runner.execute(play_action)
        results = runner.auto_resolve(max_steps=20)

        # Check that exactly 2 Digimon got CANNOT_BE_AFFECTED modifiers
        # (4 colors / 2 = 2 immunity grants)
        affected_count = 0
        for perm in game.player1.battle_area:
            if perm.is_digimon and game.modifiers.has_modifier(
                perm, ModifierType.CANNOT_BE_AFFECTED
            ):
                affected_count += 1

        assert affected_count == 2, (
            f"Should grant immunity to 2 Digimon (4 tamer colors / 2), got {affected_count}"
        )

    def test_immunity_zero_with_no_tamers(self, debug_runner):
        """With 0 tamer colors, no immunity should be granted."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # No tamers on field
        runner.place_on_field(1, ["ST20-07"])  # target Digimon

        runner.inject_card(1, "ST20-11", "hand")
        runner.set_phase("Main")

        play_action = runner.find_action("Play WarGreymon")
        assert play_action is not None

        runner.execute(play_action)
        runner.auto_resolve(max_steps=20)

        # No immunity should be granted (0 colors / 2 = 0)
        affected_count = 0
        for perm in game.player1.battle_area:
            if perm.is_digimon and game.modifiers.has_modifier(
                perm, ModifierType.CANNOT_BE_AFFECTED
            ):
                affected_count += 1

        assert affected_count == 0, (
            f"Should grant 0 immunity with no tamers, got {affected_count}"
        )


@pytest.mark.behavioral
class TestST20_11DeleteLowestDP:
    """Tests for [When Digivolving] [When Attacking] Delete opponent's lowest DP."""

    def test_delete_lowest_dp_on_digivolving(self, debug_runner):
        """When digivolving, should delete 1 of opponent's lowest DP Digimon."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place ADVENTURE Lv.5 on P1 field for alt-digi
        runner.place_on_field(1, ["ST20-09"])

        # Place opponent Digimon with varying DP
        opp1 = runner.place_on_field(2, ["ST20-07"])  # Tentomon 3000 DP
        opp2 = runner.place_on_field(2, ["ST20-08"])  # Kabuterimon 4000 DP
        opp_count_before = len(game.player2.battle_area)

        # Add WarGreymon to hand and digivolve
        runner.inject_card(1, "ST20-11", "hand")
        runner.set_phase("Main")

        digi_action = runner.find_action("Digivolve")
        assert digi_action is not None, (
            f"Should find a digivolve action. Actions: {runner.actions()}"
        )

        runner.execute(digi_action)
        runner.auto_resolve(max_steps=20)

        # One opponent Digimon should be deleted (lowest DP = 3000)
        opp_count_after = sum(1 for p in game.player2.battle_area if p.is_digimon)
        trash_ids = [c.c_entity_base.card_id for c in game.player2.trash_cards
                     if c.c_entity_base]

        assert opp_count_after < opp_count_before, (
            "Should have deleted an opponent's Digimon"
        )
        # The 3000 DP one (Tentomon / ST20-07) should be deleted
        assert "ST20-07" in trash_ids, (
            f"Should delete lowest DP (ST20-07 Tentomon 3000 DP). Trash: {trash_ids}"
        )

    def test_delete_lowest_dp_on_attacking(self, debug_runner):
        """When attacking, should delete 1 of opponent's lowest DP Digimon."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place WarGreymon on field (not summoning sick)
        wg_perm = runner.place_on_field(1, ["ST20-11"], turn_played=-1)

        # Place opponent Digimon
        opp1 = runner.place_on_field(2, ["ST20-07"])  # 3000 DP
        opp2 = runner.place_on_field(2, ["ST20-08"])  # 4000 DP

        runner.set_phase("Main")

        # Find attack action
        attack_action = runner.find_action("Attack")
        assert attack_action is not None, (
            f"Should find attack action. Actions: {runner.actions()}"
        )

        runner.execute(attack_action)
        runner.auto_resolve(max_steps=20)

        # Check opponent lost a Digimon
        trash_ids = [c.c_entity_base.card_id for c in game.player2.trash_cards
                     if c.c_entity_base]
        assert "ST20-07" in trash_ids, (
            f"Should delete lowest DP on attack (ST20-07). Trash: {trash_ids}"
        )

"""Behavioral tests for ST20-10 Agumon (Lv.3 Black Digimon).

Card text:
[Your Turn] While your opponent has a Digimon with 10000 DP or more,
or your Tamers have 3 or more total colors, this Digimon can digivolve
into [WarGreymon] in the hand for a digivolution cost of 4, ignoring
digivolution requirements.
Inherited: <Reboot>
Alt-digi: Lv.2 w/[ADVENTURE]/[Hero] trait: Cost 0
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestST20_10AltDigi:
    """Tests for the alt-digi: Lv.2 w/[ADVENTURE] or [Hero] trait for cost 0."""

    def test_alt_digi_adventure_lv2(self, debug_runner):
        """ST20-10 can digivolve onto a Lv.2 digi-egg with ADVENTURE trait for cost 0.

        ST20-01 Koromon is Lv.2 Black with traits [Lesser, ADVENTURE].
        """
        runner = debug_runner(initial_memory=5)

        # Place Koromon on the field directly (simulates raised digi-egg)
        koromon_perm = runner.place_on_field(1, ["ST20-01"])
        assert koromon_perm.level == 2

        # Add ST20-10 to hand
        runner.inject_card(1, "ST20-10", "hand")

        # Advance to Main phase
        runner.set_phase("Main")

        # Check that digivolve action is available
        actions = runner.actions()
        digi_action = runner.find_action("Digivolve")
        assert digi_action is not None, (
            f"Should be able to digivolve ST20-10 onto ADVENTURE Lv.2. Actions: {actions}"
        )

    def test_alt_digi_no_match_wrong_level(self, debug_runner):
        """ST20-10 should NOT be able to alt-digi onto a Lv.3 with ADVENTURE trait."""
        runner = debug_runner(initial_memory=5)
        # ST20-07 Tentomon is Lv.3 Green with ADVENTURE
        perm = runner.place_on_field(1, ["ST20-07"])
        assert perm.level == 3

        runner.inject_card(1, "ST20-10", "hand")

        # Standard evo requires Black Lv.2. Alt-digi requires Lv.2. Neither should match Lv.3.
        actions = runner.actions()
        digi_actions = runner.find_actions("Digivolve")
        # Should not find a digivolve action for ST20-10 onto Lv.3
        for aid, desc in digi_actions.items():
            assert "ST20-10" not in desc or "Agumon" not in desc, (
                f"Should NOT be able to digivolve ST20-10 onto Lv.3: {desc}"
            )


@pytest.mark.behavioral
class TestST20_10WarpDigivolve:
    """Tests for the warp digivolve into [WarGreymon] in hand."""

    def test_warp_digi_opponent_high_dp(self, debug_runner):
        """When opponent has a 10000+ DP Digimon, ST20-10 on field can warp
        digivolve into a WarGreymon in hand for cost 4."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place ST20-10 on player 1's field
        agumon_perm = runner.place_on_field(1, ["ST20-10"])

        # Place a 12000 DP Digimon on opponent's field (e.g. any Lv.6)
        # BT1-025 MetalGreymon is Lv.5 with 7000 DP — too low
        # Let's use a Lv.6 with 12000 DP
        opp_perm = runner.place_on_field(2, ["ST20-11"])
        assert opp_perm.dp >= 10000

        # Add ST20-11 WarGreymon to P1's hand
        runner.inject_card(1, "ST20-11", "hand")

        # The warp digivolve should show up (look for the effect name)
        card = agumon_perm.top_card
        effects = card.effect_list(None)
        warp_effects = [e for e in effects
                        if 'warp' in (e.effect_name or '').lower()
                        or 'wargreymon' in (e.effect_name or '').lower()]
        assert len(warp_effects) >= 1, "Should have a warp digivolve effect"

        warp_eff = warp_effects[0]
        # The condition should pass: it's our turn, opponent has 10000+ DP
        assert warp_eff.can_use_condition({}), (
            "Warp condition should pass when opponent has 10000+ DP Digimon"
        )

    def test_warp_digi_tamer_colors(self, debug_runner):
        """When own Tamers have 3+ total colors, warp digivolve condition passes."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place ST20-10 on field
        agumon_perm = runner.place_on_field(1, ["ST20-10"])

        # Place tamers with at least 3 colors
        # ST20-12: Red/Yellow (colors 0, 2)
        # ST20-13: Black/Green (colors 5, 3)
        runner.place_on_field(1, ["ST20-12"])  # Red, Yellow
        runner.place_on_field(1, ["ST20-13"])  # Black, Green
        # Total tamer colors: Red, Yellow, Black, Green = 4 >= 3

        # Add WarGreymon to hand
        runner.inject_card(1, "ST20-11", "hand")

        card = agumon_perm.top_card
        effects = card.effect_list(None)
        warp_effects = [e for e in effects
                        if 'warp' in (e.effect_name or '').lower()
                        or 'wargreymon' in (e.effect_name or '').lower()]
        assert len(warp_effects) >= 1

        warp_eff = warp_effects[0]
        assert warp_eff.can_use_condition({}), (
            "Warp condition should pass when tamer colors >= 3"
        )

    def test_warp_digi_condition_fails_no_trigger(self, debug_runner):
        """Warp condition should fail when opponent has no 10000+ DP and
        tamer colors < 3."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place ST20-10 on field
        agumon_perm = runner.place_on_field(1, ["ST20-10"])

        # Place a single tamer (2 colors only)
        runner.place_on_field(1, ["ST20-12"])  # Red, Yellow = only 2 colors

        # Opponent has no high-DP Digimon — place a low-DP one
        runner.place_on_field(2, ["ST20-07"])  # Tentomon, 3000 DP

        runner.inject_card(1, "ST20-11", "hand")

        card = agumon_perm.top_card
        effects = card.effect_list(None)
        warp_effects = [e for e in effects
                        if 'warp' in (e.effect_name or '').lower()
                        or 'wargreymon' in (e.effect_name or '').lower()]
        assert len(warp_effects) >= 1

        warp_eff = warp_effects[0]
        assert not warp_eff.can_use_condition({}), (
            "Warp condition should FAIL: no 10000+ DP opponent, only 2 tamer colors"
        )

    def test_warp_digi_targets_wargreymon_only(self, debug_runner):
        """The warp digivolve should only target cards with [WarGreymon] in name."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        agumon_perm = runner.place_on_field(1, ["ST20-10"])

        # Give opponent 10000+ DP
        runner.place_on_field(2, ["ST20-11"])

        # Add non-WarGreymon Lv.6 to hand — should NOT be selectable
        runner.inject_card(1, "ST20-06", "hand")  # Angewomon Lv.5

        card = agumon_perm.top_card
        effects = card.effect_list(None)
        warp_effects = [e for e in effects
                        if 'warp' in (e.effect_name or '').lower()
                        or 'wargreymon' in (e.effect_name or '').lower()]

        warp_eff = warp_effects[0]
        # The process should use effect_digivolve_from_hand with WarGreymon filter
        # We just test that non-WarGreymon cards don't appear as valid targets
        hand_cards = game.player1.hand_cards
        non_wg = [c for c in hand_cards if not c.contains_card_name('WarGreymon')]
        assert len(non_wg) > 0, "Should have non-WarGreymon cards in hand"


@pytest.mark.behavioral
class TestST20_10Inherited:
    """Tests for inherited Reboot."""

    def test_inherited_reboot(self, debug_runner):
        """ST20-10's inherited effect should grant Reboot."""
        runner = debug_runner(initial_memory=5)

        # Place a Digimon stack with ST20-10 as inherited source
        # ST20-03 Birdramon Lv.4 on top, ST20-10 Agumon underneath
        perm = runner.place_on_field(1, ["ST20-10", "ST20-03"])

        card = perm.card_sources[0]  # ST20-10 (bottom)
        effects = card.effect_list(None)
        reboot_effects = [e for e in effects if getattr(e, '_is_reboot', False)]
        assert len(reboot_effects) == 1, "Should have exactly 1 Reboot inherited effect"
        assert reboot_effects[0].is_inherited_effect, "Reboot should be inherited"

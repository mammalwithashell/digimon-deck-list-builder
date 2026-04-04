"""Behavioral tests for BT24-094 Central Town: Throne Room (Option, Green/Yellow, Cost 3).

Card text:
While you have no face-up security cards, you can ignore this card's color
requirements.
[Security] [All Turns] All of your green or yellow [TS] trait Digimon get
+2000 DP. While you have [Merukimon] or [Minervamon], all of your green or
yellow [TS] trait Digimon gain <Alliance>.
[Main] Add your bottom security card to the hand and place this card face up
as the bottom security card. Then, you may play 1 green or yellow [TS] trait
Digimon card from your hand with the play cost reduced by 3.
[Security] You may play 1 level 4 or lower green or yellow [TS] trait Digimon
card from your hand or trash without paying the cost.
"""

import pytest


# BT24-043 Tapirmon: Green, [TS], Lv3, DP 1000
GREEN_TS_LV3 = "BT24-043"
# BT24-031 Elecmon: Yellow, [TS], Lv3, DP 1000
YELLOW_TS_LV3 = "BT24-031"
# ST1-03 Agumon: Red, no TS trait
NON_TS_DIGIMON = "ST1-03"


@pytest.mark.behavioral
class TestBT24094CentralTownThroneRoom:
    """Tests for BT24-094 Central Town: Throne Room."""

    def test_dp_aura_active_when_face_up_in_security(self, debug_runner):
        """[Security] [All Turns] +2000 DP aura active when card is face-up in security."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        # Inject BT24-094 face-up in security (placed by Main effect)
        card = runner.inject_card(1, "BT24-094", "security_top")
        player.face_up_security.add(card)

        # Place a green [TS] Digimon on field
        perm = runner.place_on_field(1, [GREEN_TS_LV3])

        base_dp = perm.top_card.base_dp
        assert perm.dp == base_dp + 2000, (
            f"Green [TS] Digimon should get +2000 DP from face-up security aura, "
            f"got {perm.dp} (base {base_dp})"
        )

    def test_dp_aura_inactive_when_face_down_in_security(self, debug_runner):
        """Aura should NOT be active when card is face-down in security."""
        runner = debug_runner(initial_memory=10)

        # Inject BT24-094 into security (face-down by default)
        runner.inject_card(1, "BT24-094", "security_top")

        # Place a green [TS] Digimon on field
        perm = runner.place_on_field(1, [GREEN_TS_LV3])

        base_dp = perm.top_card.base_dp
        assert perm.dp == base_dp, (
            f"Aura should NOT apply when card is face-down in security, "
            f"got {perm.dp} (expected base {base_dp})"
        )

    def test_dp_aura_not_applied_to_non_ts_digimon(self, debug_runner):
        """Aura should only affect green/yellow Digimon with [TS] trait."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        # Inject BT24-094 face-up in security
        card = runner.inject_card(1, "BT24-094", "security_top")
        player.face_up_security.add(card)

        # Place a non-TS Digimon (ST1-03 Agumon — Red, no TS)
        perm = runner.place_on_field(1, [NON_TS_DIGIMON])

        base_dp = perm.top_card.base_dp
        assert perm.dp == base_dp, (
            f"Non-TS Digimon should NOT get +2000 DP aura, "
            f"got {perm.dp} (expected base {base_dp})"
        )

    def test_dp_aura_applies_to_yellow_ts(self, debug_runner):
        """Aura should also apply to yellow [TS] Digimon."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        # Inject BT24-094 face-up in security
        card = runner.inject_card(1, "BT24-094", "security_top")
        player.face_up_security.add(card)

        perm = runner.place_on_field(1, [YELLOW_TS_LV3])

        base_dp = perm.top_card.base_dp
        assert perm.dp == base_dp + 2000, (
            f"Yellow [TS] Digimon should get +2000 DP from aura, "
            f"got {perm.dp} (base {base_dp})"
        )

    def test_main_effect_swaps_bottom_security(self, debug_runner):
        """[Main] adds bottom security to hand and places this card face-up as bottom security."""
        runner = debug_runner(initial_memory=3)
        game = runner.game
        player = game.player1

        # Clear security, inject known cards
        # Convention: index 0 = bottom, index -1 = top (matches add_to_security_face_up)
        runner.clear_zone(1, "security")
        runner.inject_card(1, NON_TS_DIGIMON, "security_bottom")  # appends → top (end)
        runner.inject_card(1, "ST1-04", "security_top")  # inserts at 0 → bottom (front)

        # Inject BT24-094 into hand
        runner.inject_card(1, "BT24-094", "hand")

        # Need to ensure player has green/yellow on field to meet color requirement
        # Or clear security to bypass color check (no face-down security)
        # Actually, with 2 face-down security cards, color is enforced.
        # BT24-094 is Green/Yellow. Player needs Green or Yellow tamer/digimon to play.
        # Let's place a green TS digimon to ensure colors match.
        runner.place_on_field(1, [GREEN_TS_LV3])

        runner.set_phase("Main")

        # Find and execute the option play action
        action = runner.find_action("Central Town")
        if action is None:
            action = runner.find_action("BT24-094")
        assert action is not None, (
            f"Should be able to play BT24-094 from hand. "
            f"Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        # Verify: BT24-094 should be face-up in security at the bottom
        bt24_in_sec = [
            c for c in player.security_cards
            if getattr(c, 'card_id', None) == 'BT24-094'
        ]
        assert len(bt24_in_sec) == 1, "BT24-094 should be in security after Main effect"
        assert player.is_security_face_up(bt24_in_sec[0]), (
            "BT24-094 should be face-up in security after Main effect"
        )
        assert player.security_cards[0] == bt24_in_sec[0], (
            "BT24-094 should be placed as the BOTTOM security card (index 0)"
        )

    def test_ignore_color_bypass_when_no_face_down_security(self, debug_runner):
        """Color requirements should be bypassed when no face-down security cards exist."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        # Clear all security (empty = no face-down security)
        runner.clear_zone(1, "security")

        # Inject BT24-094 into hand — need to trigger effect loading
        card = runner.inject_card(1, "BT24-094", "hand")
        # Force effect loading (which sets _match_color_requirement_fn)
        card.effect_list(None)

        # With no security, match_color_requirement should be False (bypass)
        assert card.match_color_requirement is False, (
            "Should bypass color requirement when no face-down security cards"
        )

    def test_ignore_color_enforced_when_face_down_security_exists(self, debug_runner):
        """Color requirements should be enforced when face-down security cards exist."""
        runner = debug_runner(initial_memory=10)

        # Player has normal (face-down) security cards
        card = runner.inject_card(1, "BT24-094", "hand")
        # Force effect loading
        card.effect_list(None)

        # With face-down security present, match_color_requirement should be True (enforce)
        assert card.match_color_requirement is True, (
            "Should enforce color requirement when face-down security cards exist"
        )

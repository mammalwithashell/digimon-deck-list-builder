"""Behavioral tests for BT24-090 Abyss Sanctuary: Throne Room (Option, Blue/Yellow, Cost 3).

Card text:
While you have no face-up security cards, you can ignore this card's color
requirements.
[Security] [All Turns] All of your blue or yellow [TS] trait Digimon gain
<Blocker>. While you have [Neptunemon] or [Venusmon], all of your blue or
yellow [TS] trait Digimon gain <Alliance>.
[Main] Add your bottom security card to the hand and place this card face up
as the bottom security card. Then, you may play 1 blue or yellow [TS] trait
Digimon card from your hand with the play cost reduced by 3.
[Security] You may play 1 level 4 or lower blue or yellow [TS] trait Digimon
card from your hand or trash without paying the cost.
"""

import pytest


# BT24-034 Syakomon: Blue, [TS], Lv3
BLUE_TS_LV3 = "BT24-034"
# BT24-031 Elecmon: Yellow, [TS], Lv3
YELLOW_TS_LV3 = "BT24-031"
# ST1-03 Agumon: Red, no TS trait
NON_TS_DIGIMON = "ST1-03"


@pytest.mark.behavioral
class TestBT24090AbyssSanctuaryThroneRoom:
    """Tests for BT24-090 Abyss Sanctuary: Throne Room."""

    def test_blocker_aura_active_when_face_up_in_security(self, debug_runner):
        """[Security] [All Turns] Blocker aura active when card is face-up in security."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        # Inject BT24-090 face-up in security (placed by Main effect)
        card = runner.inject_card(1, "BT24-090", "security_top")
        player.face_up_security.add(card)

        # Place a blue [TS] Digimon on field
        perm = runner.place_on_field(1, [BLUE_TS_LV3])

        assert perm.has_keyword('_is_blocker'), (
            "Blue [TS] Digimon should gain Blocker from face-up security aura"
        )

    def test_blocker_aura_inactive_when_face_down_in_security(self, debug_runner):
        """Blocker aura should NOT be active when card is face-down in security."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Inject BT24-090 into security (face-down by default)
        runner.inject_card(1, "BT24-090", "security_top")

        # Place a blue [TS] Digimon on field
        perm = runner.place_on_field(1, [BLUE_TS_LV3])

        assert not perm.has_keyword('_is_blocker'), (
            "Blocker aura should NOT apply when card is face-down in security"
        )

    def test_blocker_aura_not_applied_to_non_ts_digimon(self, debug_runner):
        """Blocker aura should only affect blue/yellow Digimon with [TS] trait."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        # Inject BT24-090 face-up in security
        card = runner.inject_card(1, "BT24-090", "security_top")
        player.face_up_security.add(card)

        # Place a non-TS Digimon (ST1-03 Agumon — Red, no TS)
        perm = runner.place_on_field(1, [NON_TS_DIGIMON])

        assert not perm.has_keyword('_is_blocker'), (
            "Non-TS Digimon should NOT get Blocker from aura"
        )

    def test_blocker_aura_applies_to_yellow_ts(self, debug_runner):
        """Blocker aura should also apply to yellow [TS] Digimon."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        # Inject BT24-090 face-up in security
        card = runner.inject_card(1, "BT24-090", "security_top")
        player.face_up_security.add(card)

        perm = runner.place_on_field(1, [YELLOW_TS_LV3])

        assert perm.has_keyword('_is_blocker'), (
            "Yellow [TS] Digimon should gain Blocker from aura"
        )

    def test_main_effect_swaps_bottom_security(self, debug_runner):
        """[Main] adds bottom security to hand and places this card face-up as bottom security."""
        runner = debug_runner(initial_memory=3)
        game = runner.game
        player = game.player1

        # Clear security, inject known cards
        runner.clear_zone(1, "security")
        runner.inject_card(1, NON_TS_DIGIMON, "security_bottom")
        runner.inject_card(1, "ST1-04", "security_top")

        # Inject BT24-090 into hand
        runner.inject_card(1, "BT24-090", "hand")

        # Place a blue TS digimon to ensure colors match
        runner.place_on_field(1, [BLUE_TS_LV3])

        runner.set_phase("Main")

        # Find and execute the option play action
        action = runner.find_action("Abyss Sanctuary")
        if action is None:
            action = runner.find_action("BT24-090")
        assert action is not None, (
            f"Should be able to play BT24-090 from hand. "
            f"Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        # Verify: BT24-090 should be face-up in security at the bottom
        bt24_in_sec = [
            c for c in player.security_cards
            if getattr(c, 'card_id', None) == 'BT24-090'
        ]
        assert len(bt24_in_sec) == 1, "BT24-090 should be in security after Main effect"
        assert player.is_security_face_up(bt24_in_sec[0]), (
            "BT24-090 should be face-up in security after Main effect"
        )
        assert player.security_cards[0] == bt24_in_sec[0], (
            "BT24-090 should be placed as the BOTTOM security card (index 0)"
        )

    def test_ignore_color_bypass_when_no_face_down_security(self, debug_runner):
        """Color requirements should be bypassed when no face-down security cards exist."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        # Clear all security (empty = no face-down security)
        runner.clear_zone(1, "security")

        # Inject BT24-090 into hand and force effect loading
        card = runner.inject_card(1, "BT24-090", "hand")
        card.effect_list(None)

        assert card.match_color_requirement is False, (
            "Should bypass color requirement when no face-down security cards"
        )

    def test_ignore_color_enforced_when_face_down_security_exists(self, debug_runner):
        """Color requirements should be enforced when face-down security cards exist."""
        runner = debug_runner(initial_memory=10)

        # Player has normal (face-down) security cards
        card = runner.inject_card(1, "BT24-090", "hand")
        card.effect_list(None)

        assert card.match_color_requirement is True, (
            "Should enforce color requirement when face-down security cards exist"
        )

"""Behavioral tests for BT21-024 Cyberdramon (Lv.5 Red Cyborg, DP 7000, Cost 7).

Card text:
  [On Play] [When Digivolving] If your opponent has 5 or fewer security cards,
  they place 1 card from their hand as the bottom security card.
  Then, trash their top security card.
  Inherited: [Your Turn] This Digimon gets +4000 DP.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT21024Cyberdramon:
    """Tests for BT21-024 Cyberdramon."""

    # ── On Play: security manipulation ──────────────────────────────

    def test_on_play_opponent_places_hand_card_and_top_security_trashed(self, debug_runner):
        """On Play with opponent at <=5 security: hand card placed at bottom,
        then top security trashed."""
        runner = debug_runner(initial_memory=10)

        # Set up opponent (player 2) with 3 security and 1 hand card
        runner.clear_zone(2, "security")
        runner.inject_card(2, "ST1-03", "security_top")
        runner.inject_card(2, "ST1-04", "security_top")
        runner.inject_card(2, "ST1-05", "security_top")
        runner.clear_zone(2, "hand")
        runner.inject_card(2, "ST1-06", "hand")

        # Put BT21-024 in player 1's hand
        runner.inject_card(1, "BT21-024", "hand")

        runner.set_phase("Main")

        # Play Cyberdramon
        action = runner.find_action("Play Cyberdramon")
        assert action is not None, f"Should find Play action. Actions: {runner.actions()}"
        runner.execute(action)
        # Auto-resolve: opponent selects hand card to place as bottom security
        runner.auto_resolve()

        snap = runner.snapshot()

        # Cyberdramon should be on field
        assert any(s.card_id == "BT21-024" for s in snap.p1_field), \
            "Cyberdramon should be on the field after playing"

        # Opponent hand should be empty (the hand card was placed as security)
        assert len(snap.p2_hand) == 0, \
            f"Opponent hand should be empty, got {snap.p2_hand}"

        # Original 3 security + 1 placed from hand - 1 trashed from top = 3
        assert snap.p2_security_count == 3, \
            f"Opponent should have 3 security (3 + 1 placed - 1 trashed), got {snap.p2_security_count}"

        # The trashed card should be in opponent's trash
        assert len(snap.p2_trash) >= 1, \
            "At least one card should be in opponent's trash (the trashed top security)"

    def test_on_play_skips_when_opponent_has_more_than_5_security(self, debug_runner):
        """Effect should NOT activate when opponent has more than 5 security."""
        runner = debug_runner(initial_memory=10)

        # Set up opponent with 6 security
        runner.clear_zone(2, "security")
        for card_id in ["ST1-03", "ST1-04", "ST1-05", "ST1-06", "ST1-07", "ST1-08"]:
            runner.inject_card(2, card_id, "security_top")
        runner.clear_zone(2, "hand")
        runner.inject_card(2, "ST1-09", "hand")

        runner.inject_card(1, "BT21-024", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Cyberdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()

        # Security should be unchanged (6)
        assert after.p2_security_count == 6, \
            f"Opponent security should stay at 6 (condition not met), got {after.p2_security_count}"
        # Hand should be unchanged
        assert len(after.p2_hand) == 1, \
            f"Opponent hand should still have 1 card, got {len(after.p2_hand)}"

    def test_on_play_activates_at_exactly_5_security(self, debug_runner):
        """Effect should activate when opponent has exactly 5 security."""
        runner = debug_runner(initial_memory=10)

        runner.clear_zone(2, "security")
        for card_id in ["ST1-03", "ST1-04", "ST1-05", "ST1-06", "ST1-07"]:
            runner.inject_card(2, card_id, "security_top")
        runner.clear_zone(2, "hand")
        runner.inject_card(2, "ST1-08", "hand")

        runner.inject_card(1, "BT21-024", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Cyberdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()

        # 5 security + 1 placed - 1 trashed = 5
        assert snap.p2_security_count == 5, \
            f"Opponent should have 5 security (5 + 1 - 1), got {snap.p2_security_count}"
        assert len(snap.p2_hand) == 0, \
            "Opponent hand should be empty after placing card"

    def test_on_play_no_hand_still_trashes_top_security(self, debug_runner):
        """If opponent has no hand cards, skip the placing step but still
        trash their top security card."""
        runner = debug_runner(initial_memory=10)

        runner.clear_zone(2, "security")
        runner.inject_card(2, "ST1-03", "security_top")
        runner.inject_card(2, "ST1-04", "security_top")
        runner.clear_zone(2, "hand")
        # No hand cards for opponent

        runner.inject_card(1, "BT21-024", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Cyberdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()

        # 2 security - 1 trashed = 1
        assert snap.p2_security_count == 1, \
            f"Opponent should have 1 security (2 - 1 trashed), got {snap.p2_security_count}"

    def test_on_play_no_hand_no_security_noop(self, debug_runner):
        """If opponent has no hand AND no security, effect is a no-op."""
        runner = debug_runner(initial_memory=10)

        runner.clear_zone(2, "security")
        runner.clear_zone(2, "hand")

        runner.inject_card(1, "BT21-024", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Cyberdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert snap.p2_security_count == 0
        assert len(snap.p2_hand) == 0

    # ── When Digivolving: same security manipulation ────────────────

    def test_when_digivolving_triggers_security_effect(self, debug_runner):
        """Digivolving into Cyberdramon should trigger the same security effect."""
        runner = debug_runner(initial_memory=10)

        # Place a Red Lv.4 on field for digivolving onto
        runner.place_on_field(1, ["ST1-05"])  # Birdramon, Red Lv.4

        # Opponent with 3 security and 1 hand card
        runner.clear_zone(2, "security")
        runner.inject_card(2, "ST1-03", "security_top")
        runner.inject_card(2, "ST1-04", "security_top")
        runner.inject_card(2, "ST1-06", "security_top")
        runner.clear_zone(2, "hand")
        runner.inject_card(2, "ST1-07", "hand")

        # BT21-024 in hand for digivolving
        runner.inject_card(1, "BT21-024", "hand")
        runner.set_phase("Main")

        # Digivolve onto Birdramon
        action = runner.find_action("Digivolve")
        assert action is not None, f"Should find digivolve action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()

        # Cyberdramon should be on field (digivolved)
        assert any(s.card_id == "BT21-024" for s in snap.p1_field), \
            "Cyberdramon should be on the field after digivolving"

        # Opponent: 3 security + 1 placed - 1 trashed = 3
        assert snap.p2_security_count == 3, \
            f"Opponent should have 3 security, got {snap.p2_security_count}"
        assert len(snap.p2_hand) == 0, \
            "Opponent hand should be empty after placing card"

    def test_when_digivolving_skips_when_opponent_has_more_than_5_security(self, debug_runner):
        """When digivolving, effect should NOT activate if opponent has >5 security."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["ST1-05"])  # Birdramon, Red Lv.4

        runner.clear_zone(2, "security")
        for card_id in ["ST1-03", "ST1-04", "ST1-05", "ST1-06", "ST1-07", "ST1-08"]:
            runner.inject_card(2, card_id, "security_top")
        runner.clear_zone(2, "hand")
        runner.inject_card(2, "ST1-09", "hand")

        runner.inject_card(1, "BT21-024", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert snap.p2_security_count == 6, \
            f"Opponent security should stay at 6, got {snap.p2_security_count}"
        assert len(snap.p2_hand) == 1, \
            "Opponent hand should be unchanged"

    # ── Security trash must fire OnLoseSecurity ─────────────────────

    def test_security_trash_fires_on_lose_security(self, debug_runner):
        """The 'trash their top security card' step must use
        Player.trash_security_card() so it fires OnLoseSecurity/OnDiscardSecurity.
        We verify by checking the trashed card ends up in trash correctly."""
        runner = debug_runner(initial_memory=10)

        runner.clear_zone(2, "security")
        runner.inject_card(2, "ST1-03", "security_top")
        runner.inject_card(2, "ST1-04", "security_top")
        runner.clear_zone(2, "hand")
        runner.inject_card(2, "ST1-06", "hand")

        runner.inject_card(1, "BT21-024", "hand")
        runner.set_phase("Main")

        # Intercept _fire_timing on the opponent player to detect OnLoseSecurity
        game = runner.game
        enemy = game.player2
        on_lose_fired = []
        original_fire = enemy._fire_timing

        def _tracking_fire(timing, ctx=None):
            if timing == EffectTiming.OnLoseSecurity:
                on_lose_fired.append(True)
            return original_fire(timing, ctx)

        enemy._fire_timing = _tracking_fire

        action = runner.find_action("Play Cyberdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Restore
        enemy._fire_timing = original_fire

        # trash_security_card fires OnDiscardSecurity and OnLoseSecurity
        assert len(on_lose_fired) >= 1, \
            "trash_security_card must be used so OnLoseSecurity fires"

        snap = runner.snapshot()
        assert len(snap.p2_trash) >= 1, \
            "Trashed security card should be in opponent's trash"

    # ── Inherited DP modifier ───────────────────────────────────────

    def test_inherited_dp_plus_4000_your_turn(self, debug_runner):
        """Inherited effect should give +4000 DP during owner's turn."""
        runner = debug_runner(initial_memory=10)

        # Place Cyberdramon (Lv.5, DP 7000) as digivolution source under
        # ST1-11 WarGreymon (Lv.6, DP 12000)
        perm = runner.place_on_field(1, ["BT21-024", "ST1-11"])

        # Player 1's turn — inherited should give +4000
        game = runner.game
        game.turn_player = game.player1
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        # WarGreymon base DP is 12000, with +4000 inherited = 16000
        assert perm.dp == 16000, \
            f"DP should be 16000 (12000 base + 4000 inherited), got {perm.dp}"

    def test_inherited_dp_not_during_opponent_turn(self, debug_runner):
        """Inherited +4000 DP should NOT apply during opponent's turn."""
        runner = debug_runner(initial_memory=10)

        # BT21-024 as source under ST1-11 WarGreymon (Lv.6, DP 12000)
        perm = runner.place_on_field(1, ["BT21-024", "ST1-11"])

        # Switch to player 2's turn
        game = runner.game
        game.turn_player = game.player2
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        # WarGreymon base DP is 12000, no inherited modifier during opponent's turn
        assert perm.dp == 12000, \
            f"DP should be 12000 (no inherited bonus on opponent turn), got {perm.dp}"

    def test_inherited_dp_only_as_source(self, debug_runner):
        """When BT21-024 is the top card (not inherited), inherited effect should not apply."""
        runner = debug_runner(initial_memory=10)

        # BT21-024 as top card (not a digivolution source)
        perm = runner.place_on_field(1, ["BT21-024"])

        game = runner.game
        game.turn_player = game.player1
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        # BT21-024 base DP is 7000, inherited effect should NOT apply when it's top card
        # (inherited effects only fire from sources, not when it's the top card itself)
        assert perm.dp == 7000, \
            f"DP should be 7000 (inherited only applies as source), got {perm.dp}"

    # ── Hand card placement is at bottom of security ────────────────

    def test_hand_card_placed_at_bottom_of_security(self, debug_runner):
        """The card placed from hand must go to the BOTTOM of the security stack,
        and the card trashed must come from the TOP."""
        runner = debug_runner(initial_memory=10)

        runner.clear_zone(2, "security")
        # Inject specific security cards so we can track order
        # security_top means it goes on top of the stack
        runner.inject_card(2, "ST1-03", "security_top")  # bottom (first pushed)
        runner.inject_card(2, "ST1-04", "security_top")  # top (second pushed, on top)
        runner.clear_zone(2, "hand")
        runner.inject_card(2, "ST1-06", "hand")

        runner.inject_card(1, "BT21-024", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Cyberdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # After effect: hand card placed at bottom (new index 0),
        # original bottom becomes index 1, original top was trashed
        # Result: [ST1-06 (hand card), ST1-03 (original bottom)] - top (ST1-04) was trashed
        assert snap.p2_security_count == 2, \
            f"Should have 2 security, got {snap.p2_security_count}"
        # ST1-04 should be in trash (it was the top security card)
        assert "ST1-04" in snap.p2_trash, \
            f"ST1-04 (original top security) should be in trash, got {snap.p2_trash}"

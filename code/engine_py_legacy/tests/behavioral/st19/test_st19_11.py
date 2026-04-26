"""Behavioral tests for ST19-11 Chaperomon.

Card text:
  [On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP
  for the turn. If there are 3 or more Digimon, increase the DP reduction
  of this effect by -3000.

  Inherited Effect:
  [All Turns] [Once Per Turn] When this Digimon would leave the battle area
  other than by your effects, by deleting 1 of your Tokens or other [Puppet]
  trait Digimon, prevent it from leaving.

Clauses:
  C1: [On Play] selects 1 opponent Digimon and applies -3000 DP
  C2: If total Digimon (both sides) >= 3, applies -6000 DP instead
  C3: [When Digivolving] triggers the same DP reduction
  C4: Inherited WhenRemoveField -- once per turn, optional
  C5: WhenRemoveField does NOT fire when removal is by own effects
  C6: WhenRemoveField DOES fire when removal is by opponent effects
  C7: Sacrifice target must be a Token or other [Puppet] trait Digimon (not self)
  C8: On successful sacrifice, the Digimon is prevented from leaving
  C9: Player can decline sacrifice (canNoSelect), causing deletion to proceed
"""

import pytest

from engine_py_legacy.engine.game.constants import SEL_MY_FIELD_START


# ── Helpers ──────────────────────────────────────────────────────────────

# BT1-024 MetalTyrannomon: Lv5, 10000 DP (high enough to survive -6000)
OPP_TARGET = "BT1-024"
OPP_TARGET_DP = 10000


def _make_runner(debug_runner, **kwargs):
    """Create a runner with ST19-11 available."""
    filler = ["BT1-010"] * 40
    deck1 = ["ST19-11"] * 4 + ["ST19-02"] * 4 + ["ST19-12"] * 4 + filler
    deck2 = [OPP_TARGET] * 4 + filler

    return debug_runner(
        deck1=deck1,
        deck2=deck2,
        skip_shuffle=True,
        auto_mulligan=True,
        initial_memory=kwargs.get("initial_memory", 10),
    )


def _select_sacrifice(runner, target_name=None):
    """Select a sacrifice target in the SelectTarget phase.

    Picks the first SEL_MY_FIELD action (skipping Decline/Pass action 62).
    If target_name is given, tries to match by name.
    """
    actions = runner.actions()
    # Filter to own-field selection actions (100+)
    field_actions = {aid: desc for aid, desc in actions.items()
                     if aid >= SEL_MY_FIELD_START}
    if target_name:
        for aid, desc in field_actions.items():
            if target_name.lower() in desc.lower():
                return runner.execute(aid)
    # Just pick first field action
    if field_actions:
        return runner.execute(min(field_actions.keys()))
    return None


@pytest.mark.behavioral
class TestST19_11OnPlayDPReduction:
    """C1+C2: [On Play] DP reduction with Digimon count threshold."""

    def test_on_play_minus_3000_under_3_digimon(self, debug_runner):
        """C1: With fewer than 3 total Digimon, apply -3000 DP."""
        runner = _make_runner(debug_runner, initial_memory=10)

        # Place 1 opponent Digimon (total = 1 opp + 1 self after play = 2 < 3)
        opp_perm = runner.place_on_field(2, [OPP_TARGET])
        opp_dp_before = opp_perm.dp

        # Play Chaperomon from hand
        runner.inject_card(1, "ST19-11", "hand")
        runner.set_phase("Main")
        action = runner.find_action("Play Chaperomon")
        assert action is not None, "Should find play action for Chaperomon"
        runner.execute(action)

        # Resolve the selection phase (pick the opponent Digimon)
        runner.auto_resolve()

        # Check DP reduction: should be -3000 (only 2 total Digimon)
        opp_dp_after = opp_perm.dp
        assert opp_dp_after == opp_dp_before - 3000, (
            f"Expected -3000 DP reduction (2 total Digimon), "
            f"got {opp_dp_before} -> {opp_dp_after}"
        )

    def test_on_play_minus_6000_with_3_or_more_digimon(self, debug_runner):
        """C2: With 3+ total Digimon, apply -6000 DP."""
        runner = _make_runner(debug_runner, initial_memory=10)

        # Place 2 own Digimon + 1 opponent = 3 total (before Chaperomon enters)
        runner.place_on_field(1, ["BT1-010"])
        runner.place_on_field(1, ["BT1-010"])
        opp_perm = runner.place_on_field(2, [OPP_TARGET])
        opp_dp_before = opp_perm.dp

        # Play Chaperomon from hand (total becomes 3 own + 1 opp = 4)
        runner.inject_card(1, "ST19-11", "hand")
        runner.set_phase("Main")
        action = runner.find_action("Play Chaperomon")
        assert action is not None
        runner.execute(action)

        # Resolve selection
        runner.auto_resolve()

        # Check DP reduction: should be -6000 (4 total Digimon >= 3)
        opp_dp_after = opp_perm.dp
        assert opp_dp_after == opp_dp_before - 6000, (
            f"Expected -6000 DP reduction (4 total Digimon), "
            f"got {opp_dp_before} -> {opp_dp_after}"
        )


@pytest.mark.behavioral
class TestST19_11InheritedPreventLeaving:
    """C4-C9: Inherited [All Turns] [Once Per Turn] WhenRemoveField effect.

    For inherited testing, stack is bottom-to-top: [ST19-04 (Lv3), ST19-11 (Lv5), ST19-12 (Lv6)].
    ST19-11 sits below the top card, so its inherited effect is active.
    """

    def _make_inherited_stack(self, runner):
        """Place a Digimon with ST19-11 as inherited source."""
        return runner.place_on_field(1, ["ST19-04", "ST19-11", "ST19-12"])

    def test_prevent_leaving_by_deleting_puppet(self, debug_runner):
        """C7+C8: Deleting a Puppet Digimon prevents this Digimon from leaving."""
        runner = _make_runner(debug_runner, initial_memory=10)

        main_perm = self._make_inherited_stack(runner)
        puppet_perm = runner.place_on_field(1, ["ST19-02"])  # Junkmon (Puppet)

        p1 = runner.game.player1

        # Delete main_perm via opponent effect
        result = p1.delete_permanent(main_perm, is_opponent_effect=True)
        assert result is False, "Deletion should be prevented (returned False)"

        # Should be in SelectTarget phase for sacrifice choice
        assert runner.game.current_phase.name == "SelectTarget", (
            f"Expected SelectTarget phase, got {runner.game.current_phase.name}"
        )

        # Select the Puppet as sacrifice (not Decline)
        _select_sacrifice(runner)

        # After sacrificing, main permanent should still be on field
        assert main_perm in p1.battle_area, (
            "Main Digimon should remain on field after sacrifice prevention"
        )
        # Puppet should be deleted (sacrificed)
        assert puppet_perm not in p1.battle_area, (
            "Sacrificed Puppet should be removed from field"
        )

    def test_prevent_leaving_by_deleting_token(self, debug_runner):
        """C7: Can also sacrifice a Token to prevent leaving."""
        runner = _make_runner(debug_runner, initial_memory=10)

        main_perm = self._make_inherited_stack(runner)

        # Create a token on P1's field
        game = runner.game
        p1 = game.player1
        game.effect_play_token(p1, 'familiar')

        # Verify token exists
        tokens = [p for p in p1.battle_area if p.is_token]
        assert len(tokens) >= 1, "Should have at least 1 token on field"

        # Delete main_perm via opponent effect
        result = p1.delete_permanent(main_perm, is_opponent_effect=True)
        assert result is False, "Deletion should be prevented"

        # Select the token as sacrifice
        _select_sacrifice(runner)

        # Main permanent should still be on field
        assert main_perm in p1.battle_area, (
            "Main Digimon should remain on field after token sacrifice"
        )

    def test_does_not_fire_on_own_effect_removal(self, debug_runner):
        """C5: Effect should NOT trigger when removal is by own effects."""
        runner = _make_runner(debug_runner, initial_memory=10)

        main_perm = self._make_inherited_stack(runner)
        puppet_perm = runner.place_on_field(1, ["ST19-02"])

        p1 = runner.game.player1

        # Delete by own effect (is_opponent_effect=False, default)
        result = p1.delete_permanent(main_perm, is_opponent_effect=False)

        # Should be deleted normally (no prevention)
        assert result is True, "Deletion by own effects should proceed"
        assert main_perm not in p1.battle_area, (
            "Main Digimon should be deleted when removed by own effects"
        )

    def test_fires_on_opponent_effect_removal(self, debug_runner):
        """C6: Effect DOES trigger when removal is by opponent effects."""
        runner = _make_runner(debug_runner, initial_memory=10)

        main_perm = self._make_inherited_stack(runner)
        puppet_perm = runner.place_on_field(1, ["ST19-02"])

        p1 = runner.game.player1

        # Delete by opponent effect
        result = p1.delete_permanent(main_perm, is_opponent_effect=True)
        assert result is False, "Deletion should be prevented by inherited effect"

        # Select sacrifice
        _select_sacrifice(runner)

        assert main_perm in p1.battle_area, (
            "Main Digimon should remain on field after opponent-effect removal "
            "with available sacrifice"
        )

    def test_decline_sacrifice_allows_deletion(self, debug_runner):
        """C9: Player can decline sacrifice (canNoSelect=true), causing deletion."""
        runner = _make_runner(debug_runner, initial_memory=10)

        main_perm = self._make_inherited_stack(runner)
        puppet_perm = runner.place_on_field(1, ["ST19-02"])

        p1 = runner.game.player1

        # Delete by opponent effect
        result = p1.delete_permanent(main_perm, is_opponent_effect=True)
        assert result is False, "Initially prevented"

        # Decline the sacrifice (action 62 = Pass)
        runner.execute(62)

        # Main perm should now be deleted
        assert main_perm not in p1.battle_area, (
            "Main Digimon should be deleted after declining sacrifice"
        )

    def test_cannot_sacrifice_self(self, debug_runner):
        """C7: The sacrifice target must be 'other' -- cannot sacrifice self."""
        runner = _make_runner(debug_runner, initial_memory=10)

        # Place Chaperomon as inherited but with no other Puppet/Token
        main_perm = self._make_inherited_stack(runner)

        p1 = runner.game.player1

        # Delete by opponent effect -- no valid sacrifice, so effect should not trigger
        result = p1.delete_permanent(main_perm, is_opponent_effect=True)

        # Should be deleted (no sacrifice available, condition fails)
        assert result is True, "Should be deleted when no sacrifice targets available"
        assert main_perm not in p1.battle_area, (
            "Main Digimon should be deleted when no sacrifice targets available"
        )

    def test_once_per_turn_limit(self, debug_runner):
        """C4: Once per turn -- second removal in same turn should not be prevented."""
        runner = _make_runner(debug_runner, initial_memory=10)

        main_perm = self._make_inherited_stack(runner)
        puppet1 = runner.place_on_field(1, ["ST19-02"])
        puppet2 = runner.place_on_field(1, ["ST19-02"])

        p1 = runner.game.player1

        # First deletion by opponent -- should be prevented
        result1 = p1.delete_permanent(main_perm, is_opponent_effect=True)
        assert result1 is False, "First deletion should be prevented"

        # Select sacrifice for first prevention
        _select_sacrifice(runner)
        assert main_perm in p1.battle_area, (
            "First removal should be prevented by sacrifice"
        )

        # Second deletion by opponent -- once per turn should block prevention
        result2 = p1.delete_permanent(main_perm, is_opponent_effect=True)
        assert result2 is True, "Second deletion should proceed (once per turn used)"
        assert main_perm not in p1.battle_area, (
            "Second removal in same turn should NOT be prevented (once per turn)"
        )

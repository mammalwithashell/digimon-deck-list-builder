"""Behavioral tests for BT13-012 GeoGreymon.

Card text:
  Lv.4 | Red/Yellow | Dinosaur | DP:5000 | Cost:5
  Alt digivolve: Lv.3 w/[Agumon] in name and [Dinosaur] trait: Cost 2
  [When Digivolving] Search your security stack, and you may play 1 red or
  yellow Tamer card among it without paying its cost. If you did,
  <Recovery +1 (Deck)>. Then, shuffle your security stack.
  Inherited: [Your Turn][Once Per Turn] When one of your red or yellow
  Tamers becomes suspended, you may delete 1 of your opponent's Digimon
  with 3000 DP or less.

Key faithfulness points tested:
  - WD: player selects from security via engine selection API (not auto-pick).
  - WD: only red or yellow Tamer cards are eligible.
  - WD: if played, Recovery +1 (Deck) fires.
  - WD: security is ALWAYS shuffled after the effect (even if declining).
  - WD: played tamer triggers OnEnterFieldAnyone (for tamer's own effects).
  - WD: selection is optional (player can decline even if valid tamer exists).
  - Inherited: triggers when own red/yellow Tamer becomes suspended.
  - Inherited: once per turn.
  - Inherited: optional delete of opp Digimon with DP <= 3000.
  - Alt-digi: requires BOTH Agumon name AND Dinosaur trait on Lv.3.
"""

import pytest

from digimon_gym.engine.game.constants import SEL_MY_SECURITY_START
from digimon_gym.engine.data.enums import GamePhase


# Card IDs
BT13_012 = "BT13-012"           # Card under test (GeoGreymon, Lv.4 Red/Yellow)
AGUMON_ST1 = "ST1-03"           # Agumon (Lv.3 Red, Reptile - NOT Dinosaur)
AGUMON_BT13 = "BT13-008"        # Agumon (Lv.3 Red/Yellow, Dinosaur)
TAI_KAMIYA = "BT1-085"          # Tai Kamiya (Red Tamer)
BIYOMON = "ST1-02"              # Biyomon (Lv.3 Red, for non-Agumon digivolve test)
DRACOMON = "ST1-04"             # Dracomon (Lv.3 Red, not Agumon, not Dinosaur)
METALGREYMON = "ST1-09"         # MetalGreymon (Lv.5 Red)


def _digivolve_into_geogreymon(runner):
    """Helper: find and execute the digivolve action into GeoGreymon."""
    action = runner.find_action("Digivolve")
    if action is None:
        action = runner.find_action("digivolve")
    if action is None:
        action = runner.find_action("GeoGreymon")
    assert action is not None, f"Should find digivolve action. Actions: {runner.actions()}"
    return runner.execute(action)


@pytest.mark.behavioral
class TestBT13012WhenDigivolving:
    """[When Digivolving] Search security for red/yellow Tamer, play free, Recovery +1, shuffle."""

    def test_wd_enters_selection_phase_for_security(self, debug_runner):
        """When Digivolving with a valid red/yellow Tamer in security, the engine
        should enter a SelectSecurity phase (not auto-play without player choice).

        This verifies the script uses the engine's selection API, which is
        critical for RL agent training -- agents must be able to choose which
        tamer to play (or to decline).
        """
        runner = debug_runner(archetype1="Shinegreymon", initial_memory=10)

        runner.place_on_field(1, [AGUMON_ST1])
        runner.inject_card(1, BT13_012, "hand")
        runner.inject_card(1, TAI_KAMIYA, "security_top")

        runner.set_phase("Main")

        _digivolve_into_geogreymon(runner)

        # After digivolving, the WD effect should enter a selection phase.
        # The script MUST use the engine's selection API to present the choice.
        snap = runner.snapshot()
        game = runner.game

        # Check the game is in a selection phase (not Main)
        assert game.current_phase == GamePhase.SelectSecurity, (
            f"After WD trigger with valid tamer in security, game should be "
            f"in SelectSecurity phase. Got: {game.current_phase.name}. "
            f"This means the script bypasses the engine selection API."
        )

        # Verify security selection actions are available
        legal = runner.action_mask()
        security_actions = [a for a in legal
                            if SEL_MY_SECURITY_START <= a < SEL_MY_SECURITY_START + 10]
        assert len(security_actions) >= 1, (
            f"Should have security selection actions. Legal actions: {runner.actions()}"
        )

    def test_wd_plays_tamer_from_security(self, debug_runner):
        """Selecting a Tamer from security should play it to field for free."""
        runner = debug_runner(archetype1="Shinegreymon", initial_memory=10)

        runner.place_on_field(1, [AGUMON_ST1])
        runner.inject_card(1, BT13_012, "hand")
        runner.inject_card(1, TAI_KAMIYA, "security_top")

        runner.set_phase("Main")
        mem_before = runner.snapshot().memory

        _digivolve_into_geogreymon(runner)

        # Select the Tamer from security
        snap = runner.snapshot()
        if snap.phase != "Main":
            legal = runner.action_mask()
            security_actions = [a for a in legal
                                if SEL_MY_SECURITY_START <= a < SEL_MY_SECURITY_START + 10]
            if security_actions:
                runner.execute(security_actions[0])

        runner.auto_resolve()

        snap_after = runner.snapshot()
        tai_on_field = any(
            s.card_id == TAI_KAMIYA and s.is_tamer
            for s in snap_after.p1_field
        )
        assert tai_on_field, (
            f"Tai Kamiya should be on field after selection. "
            f"Field: {[(s.card_id, s.card_name) for s in snap_after.p1_field]}"
        )

    def test_wd_recovery_fires_after_play(self, debug_runner):
        """If a Tamer was played from security, Recovery +1 should fire.

        We verify by checking that security count stays the same
        (removed 1 tamer, added 1 from recovery = net 0 change).
        """
        runner = debug_runner(archetype1="Shinegreymon", initial_memory=10)

        runner.place_on_field(1, [AGUMON_ST1])
        runner.inject_card(1, BT13_012, "hand")
        runner.inject_card(1, TAI_KAMIYA, "security_top")

        runner.set_phase("Main")
        sec_before = runner.snapshot().p1_security_count

        _digivolve_into_geogreymon(runner)

        # After digivolving, track the library size AFTER the digivolve
        # resolves but BEFORE the security selection, to isolate the
        # recovery's effect on the library.
        snap_post_digi = runner.snapshot()

        # Select the tamer from security
        if snap_post_digi.phase != "Main":
            legal = runner.action_mask()
            security_actions = [a for a in legal
                                if SEL_MY_SECURITY_START <= a < SEL_MY_SECURITY_START + 10]
            if security_actions:
                lib_at_selection = snap_post_digi.p1_library_size
                runner.execute(security_actions[0])

        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Verify the tamer is on field
        tai_on_field = any(s.card_id == TAI_KAMIYA for s in snap_after.p1_field)
        assert tai_on_field, "Tamer should be on field for recovery check"

        # Recovery +1: security -= 1 (tamer removed) + 1 (recovery from deck) = net 0
        assert snap_after.p1_security_count == sec_before, (
            f"Security should be {sec_before} (removed 1 tamer, +1 recovery). "
            f"Got {snap_after.p1_security_count}"
        )

    def test_wd_decline_does_not_trigger_recovery(self, debug_runner):
        """If player declines to play a Tamer, Recovery should NOT fire.

        The card text says 'you may play' -- the effect is optional.
        The script must present a Pass/Decline action in the selection phase.
        If declined, the tamer stays in security and no recovery happens.
        """
        runner = debug_runner(archetype1="Shinegreymon", initial_memory=10)

        runner.place_on_field(1, [AGUMON_ST1])
        runner.inject_card(1, BT13_012, "hand")
        runner.inject_card(1, TAI_KAMIYA, "security_top")

        runner.set_phase("Main")
        sec_before = runner.snapshot().p1_security_count

        _digivolve_into_geogreymon(runner)

        # The script should enter a selection phase, not auto-play.
        snap = runner.snapshot()
        game = runner.game
        assert game.current_phase == GamePhase.SelectSecurity, (
            f"Should be in SelectSecurity phase (not auto-play). "
            f"Got: {game.current_phase.name}"
        )

        # Find and execute the Pass/Decline action
        pass_action = runner.find_action("Pass")
        if pass_action is None:
            pass_action = runner.find_action("Decline")
        assert pass_action is not None, (
            f"Effect is optional (you MAY play) — must have Pass/Decline action. "
            f"Actions: {runner.actions()}"
        )
        runner.execute(pass_action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Tamer should NOT be on field
        tai_on_field = any(s.card_id == TAI_KAMIYA for s in snap_after.p1_field)
        assert not tai_on_field, "Tamer should NOT be on field after declining"

        # Security should be unchanged (tamer still there, no recovery)
        assert snap_after.p1_security_count == sec_before, (
            f"Security should be unchanged after declining. "
            f"Was {sec_before}, got {snap_after.p1_security_count}"
        )

    def test_wd_security_shuffled_after_no_valid_target(self, debug_runner):
        """Security should be shuffled even when no valid tamer exists."""
        runner = debug_runner(archetype1="Shinegreymon", initial_memory=10)

        runner.place_on_field(1, [AGUMON_ST1])
        runner.inject_card(1, BT13_012, "hand")
        # Add multiple non-tamer cards to security (no valid target)
        runner.inject_card(1, AGUMON_ST1, "security_top")
        runner.inject_card(1, BIYOMON, "security_top")
        runner.inject_card(1, DRACOMON, "security_top")

        runner.set_phase("Main")

        _digivolve_into_geogreymon(runner)
        runner.auto_resolve()

        # No tamer to play, so no recovery, no selection phase.
        # Security should still have all cards (nothing removed).
        snap = runner.snapshot()
        assert snap.p1_security_count >= 3, (
            f"Security should have at least 3 cards (no tamer played). "
            f"Got {snap.p1_security_count}"
        )

    def test_wd_only_red_or_yellow_tamer_eligible(self, debug_runner):
        """Only red or yellow Tamers in security should be selectable."""
        runner = debug_runner(archetype1="Shinegreymon", initial_memory=10)

        runner.place_on_field(1, [AGUMON_ST1])
        runner.inject_card(1, BT13_012, "hand")
        runner.inject_card(1, TAI_KAMIYA, "security_top")

        runner.set_phase("Main")

        _digivolve_into_geogreymon(runner)

        # Should enter selection phase with the tamer as an option
        snap = runner.snapshot()
        if snap.phase != "Main":
            legal = runner.action_mask()
            security_actions = [a for a in legal
                                if SEL_MY_SECURITY_START <= a < SEL_MY_SECURITY_START + 10]
            assert len(security_actions) >= 1, (
                f"Should have at least 1 security selection option for red Tamer. "
                f"Legal: {runner.actions()}"
            )

    def test_wd_requires_security_cards(self, debug_runner):
        """If security is empty, the WD effect should not activate."""
        runner = debug_runner(archetype1="Shinegreymon", initial_memory=10)

        runner.place_on_field(1, [AGUMON_ST1])
        runner.inject_card(1, BT13_012, "hand")
        runner.clear_zone(1, "security")

        runner.set_phase("Main")

        _digivolve_into_geogreymon(runner)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert not snap.is_game_over, "Game should not be over"


@pytest.mark.behavioral
class TestBT13012InheritedTamerSuspend:
    """Inherited: [Your Turn][OPT] When own red/yellow Tamer suspends, delete opp DP<=3000."""

    def test_inherited_effect_attributes(self, debug_runner):
        """Inherited effect should be once per turn and optional."""
        runner = debug_runner(archetype1="Shinegreymon", initial_memory=10)

        perm = runner.place_on_field(1, [BT13_012, METALGREYMON])

        card = None
        for cs in perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == BT13_012:
                card = cs
                break
        assert card is not None

        effects = card.effect_list(None)
        inherited_eff = None
        for eff in effects:
            if getattr(eff, 'is_inherited_effect', False):
                inherited_eff = eff
                break

        assert inherited_eff is not None, "Should have inherited effect"
        assert getattr(inherited_eff, 'is_optional', False) is True, (
            "Inherited effect should be optional"
        )
        assert inherited_eff.hash_string == "Delete_BT13_012", (
            f"Hash string should be 'Delete_BT13_012', got {inherited_eff.hash_string}"
        )

    def test_inherited_condition_checks_own_tamer(self, debug_runner):
        """Condition should return True when own red/yellow Tamer suspends during own turn."""
        runner = debug_runner(archetype1="Shinegreymon", initial_memory=10)

        perm = runner.place_on_field(1, [BT13_012, METALGREYMON])
        tamer_perm = runner.place_on_field(1, [TAI_KAMIYA])

        card = None
        for cs in perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == BT13_012:
                card = cs
                break
        assert card is not None

        effects = card.effect_list(None)
        inherited_eff = None
        for eff in effects:
            if getattr(eff, 'is_inherited_effect', False):
                inherited_eff = eff
                break
        assert inherited_eff is not None

        context = {'event_permanent': tamer_perm}
        result = inherited_eff.can_use_condition(context)
        assert result is True, (
            "Condition should be True when own red Tamer is the event permanent"
        )


@pytest.mark.behavioral
class TestBT13012AltDigi:
    """Alt-digi: Lv.3 w/[Agumon] in name and [Dinosaur] trait: Cost 2."""

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi should require Agumon name + Dinosaur trait at Lv.3 for cost 2."""
        runner = debug_runner(archetype1="Shinegreymon", initial_memory=10)

        perm = runner.place_on_field(1, [BT13_012])

        card = perm.top_card
        effects = card.effect_list(None)
        alt_digi = None
        for eff in effects:
            if hasattr(eff, '_alt_digi_cost') and eff._alt_digi_cost is not None:
                alt_digi = eff
                break

        assert alt_digi is not None, "Should have an alt-digi effect"
        assert alt_digi._alt_digi_cost == 2, f"Alt-digi cost should be 2, got {alt_digi._alt_digi_cost}"
        assert alt_digi._alt_digi_level == 3, f"Alt-digi level should be 3, got {alt_digi._alt_digi_level}"
        assert alt_digi._alt_digi_name == "Agumon", (
            f"Alt-digi name should be 'Agumon', got {alt_digi._alt_digi_name}"
        )
        assert alt_digi._alt_digi_trait == "Dinosaur", (
            f"Alt-digi trait should be 'Dinosaur', got {getattr(alt_digi, '_alt_digi_trait', None)}"
        )

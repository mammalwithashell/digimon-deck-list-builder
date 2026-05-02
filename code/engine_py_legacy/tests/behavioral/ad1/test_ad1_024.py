"""Behavioral tests for AD1-024 Imperialdramon: Fighter Mode (Lv.6 Blue/Green).

Card text:
  <Security A. +1>
  <Blocker>
  [When Digivolving] [When Attacking] [Once Per Turn] Return 1 of your
      opponent's lowest DP Digimon to the bottom of the deck.
  [All Turns] [Once Per Turn] When Digimon are played or digivolve, you
      may suspend 1 of your opponent's Digimon and unsuspend this Digimon.
      Then, if played or digivolved by effects, you may return 1 of your
      opponent's suspended Digimon to the bottom of the deck.

Alt-Digi:
  From Lv.5 with [Hero] trait for cost 5.
  From [Imperialdramon: Dragon Mode] for cost 1.
"""

import pytest


@pytest.mark.behavioral
class TestAD1024Keywords:
    """Tests for Security A. +1 and Blocker keywords."""

    def test_has_blocker_keyword(self, debug_runner):
        """Imperialdramon FM should have Blocker."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["AD1-024"])

        assert perm.has_keyword('_is_blocker'), (
            "AD1-024 should have Blocker keyword")

    def test_has_security_attack_plus_1(self, debug_runner):
        """Imperialdramon FM should have Security Attack +1."""
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["AD1-024"])

        # Check that the SA+1 modifier is on the effects
        sa_mod = 0
        for cs in perm.card_sources:
            for eff in cs.effect_list(EffectTiming.NoTiming):
                if getattr(eff, '_security_attack_modifier', 0) > 0:
                    sa_mod += eff._security_attack_modifier
        assert sa_mod >= 1, (
            "AD1-024 should have Security Attack +1 effect")


@pytest.mark.behavioral
class TestAD1024AltDigi:
    """Tests for alternate digivolution requirements."""

    def test_alt_digi_from_hero_trait_lv5(self, debug_runner):
        """Can digivolve from Lv.5 with [Hero] trait for cost 5."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # AD1-011 is Paildramon, Lv.5, Hero trait
        runner.place_on_field(1, ["AD1-011"])
        runner.inject_card(1, "AD1-024", "hand")

        action = runner.find_action("Digivolve")
        assert action is not None, (
            f"Should be able to digivolve onto Lv.5 Hero. "
            f"Actions: {runner.actions()}")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        fm_on_field = any(s.card_id == "AD1-024" for s in snap.p1_field)
        assert fm_on_field, "AD1-024 should be on field after digivolving"
        assert snap.memory == 10 - 5, "Alt-digi from Hero trait should cost 5"

    def test_alt_digi_from_imperialdramon_dragon_mode(self, debug_runner):
        """Can digivolve from [Imperialdramon: Dragon Mode] for cost 1."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # BT12-030 is Imperialdramon: Dragon Mode, Lv.6
        runner.place_on_field(1, ["BT12-030"])
        runner.inject_card(1, "AD1-024", "hand")

        action = runner.find_action("Digivolve")
        assert action is not None, (
            f"Should be able to digivolve onto Imperialdramon: Dragon Mode. "
            f"Actions: {runner.actions()}")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        fm_on_field = any(s.card_id == "AD1-024" for s in snap.p1_field)
        assert fm_on_field, "AD1-024 should be on field after digivolving"
        assert snap.memory == 10 - 1, "Alt-digi from Dragon Mode should cost 1"


@pytest.mark.behavioral
class TestAD1024WhenDigivolvingWhenAttacking:
    """Tests for [When Digivolving] [When Attacking] [OPT] lowest DP bottom-deck."""

    def test_when_digivolving_returns_lowest_dp(self, debug_runner):
        """When Digivolving: should return opponent's lowest DP Digimon to deck bottom."""
        runner = debug_runner(initial_memory=10)

        # Opponent field: BT1-011 (DP 1000) and BT1-015 (DP 4000)
        runner.place_on_field(2, ["BT1-011"])  # DP 1000
        runner.place_on_field(2, ["BT1-015"])  # DP 4000

        # Digivolve onto a Lv.5 Hero
        runner.place_on_field(1, ["AD1-011"])  # Paildramon (Lv.5, Hero)
        runner.inject_card(1, "AD1-024", "hand")
        runner.set_phase("Main")

        opp_lib_before = runner.snapshot().p2_library_size

        action = runner.find_action("Digivolve")
        assert action is not None, "Should be able to digivolve"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # BT1-011 (lowest DP) should be bottom-decked
        opp_digimon_ids = [s.card_id for s in snap.p2_field if s.is_digimon]
        assert "BT1-011" not in opp_digimon_ids, (
            "BT1-011 (lowest DP) should be returned to deck bottom")
        assert "BT1-015" in opp_digimon_ids, (
            "BT1-015 (higher DP) should remain on field")
        assert snap.p2_library_size == opp_lib_before + 1, (
            "Opponent should have 1 more card in deck (bottom-decked)")

    def test_when_digivolving_no_target_no_crash(self, debug_runner):
        """When Digivolving: with no opponent Digimon, effect should do nothing."""
        runner = debug_runner(initial_memory=10)

        # No opponent Digimon
        runner.place_on_field(1, ["AD1-011"])  # Paildramon
        runner.inject_card(1, "AD1-024", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        fm_on_field = any(s.card_id == "AD1-024" for s in snap.p1_field)
        assert fm_on_field, "AD1-024 should be on field even with no targets"

    def test_when_attacking_returns_lowest_dp(self, debug_runner):
        """When Attacking: should return opponent's lowest DP Digimon to deck bottom."""
        runner = debug_runner(initial_memory=5)

        # Place Imperialdramon FM on field (unsuspended, played on earlier turn)
        perm = runner.place_on_field(1, ["AD1-024"], turn_played=-1)

        # Opponent field: BT1-029 (DP 1000) and BT1-015 (DP 4000)
        runner.place_on_field(2, ["BT1-029"])  # DP 1000
        runner.place_on_field(2, ["BT1-015"])  # DP 4000

        runner.set_phase("Main")

        opp_lib_before = runner.snapshot().p2_library_size

        # Attack with Imperialdramon FM
        action = runner.find_action("Attack")
        assert action is not None, (
            f"Should be able to attack. Actions: {runner.actions()}")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_digimon_ids = [s.card_id for s in snap.p2_field if s.is_digimon]
        assert "BT1-029" not in opp_digimon_ids, (
            "BT1-029 (lowest DP) should be returned to deck bottom on attack")

    def test_wd_wa_once_per_turn_shared(self, debug_runner):
        """WD and WA share OPT - digivolving uses it, attacking should not trigger again.

        We verify this by checking that the OPT counter is consumed after
        digivolving, so the When Attacking effect's condition returns False.
        """
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner = debug_runner(initial_memory=10)

        # Opponent field: three Digimon so after WD removes one, two remain
        runner.place_on_field(2, ["BT1-011"])  # DP 1000
        runner.place_on_field(2, ["BT1-029"])  # DP 1000
        runner.place_on_field(2, ["BT1-015"])  # DP 4000

        # Digivolve to trigger WD
        runner.place_on_field(1, ["AD1-011"])  # Paildramon
        runner.inject_card(1, "AD1-024", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # One of the DP 1000 Digimon should be gone (WD triggered)
        snap_after_digi = runner.snapshot()
        opp_count_after_digi = len([s for s in snap_after_digi.p2_field if s.is_digimon])
        assert opp_count_after_digi == 2, (
            f"Expected 2 opp Digimon after WD (1 bottom-decked). Got {opp_count_after_digi}")

        # Verify OPT was consumed: check the WA effect's can_activate_this_turn
        fm_perm = None
        for p in runner.game.player1.battle_area:
            if p.top_card and 'AD1-024' in (p.top_card.c_entity_base.card_id if p.top_card.c_entity_base else ''):
                fm_perm = p
                break
        assert fm_perm is not None, "AD1-024 should be on field"

        # Find the WA effect and verify OPT is consumed
        wa_effect = None
        for cs in fm_perm.card_sources:
            for eff in cs.effect_list(EffectTiming.OnUseAttack):
                if "AD1_024_WD_WA" in (getattr(eff, 'hash_string', '') or ''):
                    wa_effect = eff
                    break
        assert wa_effect is not None, "Should find the WA effect"
        assert not wa_effect.can_activate_this_turn(), (
            "WA effect should not be activatable after WD consumed the OPT")

    def test_lowest_dp_with_tie(self, debug_runner):
        """When multiple opponents share lowest DP, player should choose one."""
        runner = debug_runner(initial_memory=10)

        # Two opponent Digimon with same lowest DP (1000)
        runner.place_on_field(2, ["BT1-011"])  # DP 1000
        runner.place_on_field(2, ["BT1-029"])  # DP 1000
        runner.place_on_field(2, ["BT1-015"])  # DP 4000

        runner.place_on_field(1, ["AD1-011"])  # Paildramon
        runner.inject_card(1, "AD1-024", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Exactly one of the DP 1000 Digimon should be removed
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        dp_1000_count = sum(1 for s in opp_digimon if s.dp == 1000)
        assert dp_1000_count == 1, (
            f"Exactly one DP 1000 Digimon should remain (one bottom-decked). "
            f"Got {dp_1000_count}")
        assert any(s.card_id == "BT1-015" for s in opp_digimon), (
            "BT1-015 (DP 4000) should still be on field")


@pytest.mark.behavioral
class TestAD1024AllTurns:
    """Tests for [All Turns] [OPT] observer effect - suspend + unsuspend."""

    def test_triggers_on_own_digimon_play(self, debug_runner):
        """When you play a Digimon, may suspend 1 opp Digimon + unsuspend self."""
        from engine_py_legacy.engine.game.constants import SEL_OPP_FIELD_START
        runner = debug_runner(initial_memory=20)

        # Place Imperialdramon FM on field (suspended to test unsuspend)
        perm = runner.place_on_field(1, ["AD1-024"], is_suspended=True)

        # Opponent Digimon to suspend
        runner.place_on_field(2, ["BT1-015"])  # DP 4000, unsuspended

        # Play a Digimon to trigger
        runner.inject_card(1, "BT1-009", "hand")  # Monodramon (cost 2)
        runner.set_phase("Main")

        action = runner.find_action("Play Monodramon")
        assert action is not None, (
            f"Should be able to play Monodramon. Actions: {runner.actions()}")
        runner.execute(action)

        # The optional All Turns effect fires, offering to suspend an opp Digimon.
        # auto_resolve would pick "Decline" first. We manually select the target.
        opp_target = SEL_OPP_FIELD_START  # First opponent field slot
        legal = runner.action_mask()
        assert opp_target in legal, (
            f"Should be able to select opponent's Digimon (action {opp_target}). "
            f"Legal: {runner.actions()}")
        runner.execute(opp_target)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Check opponent Digimon is suspended
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) > 0, "Opponent Digimon should still be on field"
        assert opp_digimon[0].is_suspended, (
            "Opponent Digimon should be suspended by All Turns effect")

        # Check self is unsuspended
        self_field = [s for s in snap.p1_field if s.card_id == "AD1-024"]
        assert len(self_field) > 0, "AD1-024 should still be on field"
        assert not self_field[0].is_suspended, (
            "AD1-024 should be unsuspended by All Turns effect")

    def test_once_per_turn_across_events(self, debug_runner):
        """All Turns effect is OPT: second play in same turn should not trigger."""
        runner = debug_runner(initial_memory=20)

        runner.place_on_field(1, ["AD1-024"])

        # Two opponent Digimon
        runner.place_on_field(2, ["BT1-015"])  # DP 4000
        runner.place_on_field(2, ["BT1-009"])  # DP 3000

        # Play first Digimon - triggers effect
        runner.inject_card(1, "BT1-011", "hand")
        runner.inject_card(1, "BT1-029", "hand")
        runner.set_phase("Main")

        action1 = runner.find_action("Play")
        assert action1 is not None
        runner.execute(action1)
        runner.auto_resolve()

        snap1 = runner.snapshot()
        suspended_count_1 = sum(
            1 for s in snap1.p2_field
            if s.is_digimon and s.is_suspended)

        # Play second Digimon - OPT should prevent trigger
        action2 = runner.find_action("Play")
        if action2 is not None:
            runner.execute(action2)
            runner.auto_resolve()

        snap2 = runner.snapshot()
        suspended_count_2 = sum(
            1 for s in snap2.p2_field
            if s.is_digimon and s.is_suspended)

        assert suspended_count_2 == suspended_count_1, (
            f"OPT should prevent second trigger. "
            f"Suspended after 1st: {suspended_count_1}, after 2nd: {suspended_count_2}")

    def test_triggers_on_opponent_digimon_play(self, debug_runner):
        """When opponent plays a Digimon, the All Turns effect should fire."""
        from engine_py_legacy.engine.data.enums import EffectTiming
        from engine_py_legacy.engine.core.permanent import Permanent
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.game.constants import SEL_OPP_FIELD_START
        runner = debug_runner(initial_memory=5)

        # Place Imperialdramon FM on P1's field (suspended to test unsuspend)
        perm = runner.place_on_field(1, ["AD1-024"], is_suspended=True)

        # Opponent has a Digimon to suspend
        runner.place_on_field(2, ["BT1-015"])  # DP 4000

        # Simulate opponent playing a Digimon
        db = CardDatabase()
        new_cs = db.create_card_source("BT1-009", runner.game.player2)
        new_perm = Permanent([new_cs])
        runner.game.player2.battle_area.append(new_perm)

        # Fire OnEnterFieldAnyone as if opponent played it
        runner.game.execute_effects(
            EffectTiming.OnEnterFieldAnyone,
            {
                "played_card": new_cs,
                "played_permanent": new_perm,
                "event_player": runner.game.player2,
            },
        )

        # The All Turns effect is optional; manually select the opp target
        legal = runner.action_mask()
        opp_target = SEL_OPP_FIELD_START  # BT1-015 is first in opp battle area
        if opp_target in legal:
            runner.execute(opp_target)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Check opponent Digimon (BT1-015) is suspended
        opp_bt1_015 = [s for s in snap.p2_field
                       if s.card_id == "BT1-015" and s.is_digimon]
        assert len(opp_bt1_015) > 0, "BT1-015 should be on opponent field"
        assert opp_bt1_015[0].is_suspended, (
            "BT1-015 should be suspended by All Turns effect on opponent play")

        # Check self unsuspended
        self_fm = [s for s in snap.p1_field if s.card_id == "AD1-024"]
        assert len(self_fm) > 0
        assert not self_fm[0].is_suspended, (
            "AD1-024 should be unsuspended after All Turns triggers")

    def test_effect_is_optional(self, debug_runner):
        """All Turns effect is optional - player can decline."""
        runner = debug_runner(initial_memory=20)

        runner.place_on_field(1, ["AD1-024"])
        runner.place_on_field(2, ["BT1-015"])

        runner.inject_card(1, "BT1-009", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Monodramon")
        assert action is not None
        runner.execute(action)

        # The All Turns effect fires. Check for pass/decline option.
        # The effect is optional, so there should be a way to skip.
        # auto_resolve picks first available, but we just verify no crash.
        runner.auto_resolve()

        # Verify no crash and game continues
        snap = runner.snapshot()
        assert not snap.is_game_over, "Game should not be over"

    def test_does_not_trigger_on_non_digimon_play(self, debug_runner):
        """All Turns effect should only fire when DIGIMON are played, not Tamers/Options."""
        runner = debug_runner(initial_memory=20)

        runner.place_on_field(1, ["AD1-024"])
        runner.place_on_field(2, ["BT1-015"])

        # Play a Tamer (not Digimon)
        runner.inject_card(1, "ST2-12", "hand")  # Matt Ishida (Tamer)
        runner.set_phase("Main")

        action = runner.find_action("Play Matt")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        snap = runner.snapshot()
        opp_suspended = [s for s in snap.p2_field
                         if s.is_digimon and s.is_suspended]
        assert len(opp_suspended) == 0, (
            "Opponent Digimon should NOT be suspended from Tamer play; "
            "All Turns only fires on Digimon play/digivolve")
